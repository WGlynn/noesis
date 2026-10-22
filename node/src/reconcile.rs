//! Fact reconciliation, first slice: content-addressed contribution facts and an order-free union
//! merge across independently run chains (design note *The node and the network*, 2026-10-22, §2,
//! §5, §9).
//!
//! A cell's `id` is a local counter and its `parent` points at that counter, so no chain can name
//! another chain's contribution. Here every contribution gets a [`ContentId`] derived from what it IS
//! (author, data, the parent's ContentId), never from where it landed (local `id`, `timestamp`,
//! `lock`, height). Facts named that way merge by plain set union keyed by ContentId, which is
//! commutative, associative and idempotent, so two chains' facts reconcile in any order.
//!
//! HONEST SCOPE: CONSENSUS-ISOLATED SHADOW (the `jul`/`wallclock`/`liveness`/`subblock` precedent).
//! Pure and not wired: it reads cells but never touches `Cell`, `Ledger`, validation, finality, or
//! `state_digest`. No adversary model: a record is accepted iff its ContentId re-derives from its
//! fields; there are no signatures, no witness check, no wash or plagiarism defense. Rival claims
//! (the same data under more than one author) are SURFACED by [`FactSet::rival_claims`], not
//! resolved; the resolution rule (earliest commitment, disputes) is a later layer. A record whose
//! parent is absent is a claim awaiting its witness, reported by [`FactSet::dangling`].

use std::collections::{BTreeMap, BTreeSet};

use crate::Cell;

/// A contribution's network-wide name: blake2b-256 over (author, data hash, parent ContentId).
pub type ContentId = [u8; 32];

const CID: &[u8; 16] = b"noesis-cid-v1\0\0\0";
const DATA: &[u8; 16] = b"noesis-data-v1\0\0";
const FSET: &[u8; 16] = b"noesis-fset-v1\0\0";

/// The project's canonical blake2b-256 (the `gossip`/`smt` hasher), domain-separated by `personal`.
fn blake(personal: &[u8; 16], parts: &[&[u8]]) -> [u8; 32] {
    let mut h = blake2b_ref::Blake2bBuilder::new(32).personal(personal).build();
    for p in parts {
        h.update(p);
    }
    let mut out = [0u8; 32];
    h.finalize(&mut out);
    out
}

/// Hash of a contribution's payload alone: the key rival claims group on.
pub fn data_hash(data: &[u8]) -> [u8; 32] {
    blake(DATA, &[data])
}

/// The ContentId of a contribution: commits to the author (`type_script.args`, length-prefixed),
/// the data (through its fixed-size hash) and the parent's ContentId (option-tagged: 0 = root,
/// 1 + cid). Excludes the local `id`, `timestamp` and `lock`, so the same contribution recorded at
/// different heights on two chains gets the same ContentId.
pub fn content_id(author: &[u8], data_hash: &[u8; 32], parent: Option<&ContentId>) -> ContentId {
    let len = (author.len() as u64).to_le_bytes();
    match parent {
        None => blake(CID, &[&len, author, data_hash, &[0]]),
        Some(p) => blake(CID, &[&len, author, data_hash, &[1], p]),
    }
}

/// One contribution fact, named by content. `cid` re-derives from the other three fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactRecord {
    pub cid: ContentId,
    pub author: Vec<u8>,
    pub data_hash: [u8; 32],
    pub parent: Option<ContentId>,
}

impl FactRecord {
    /// Name a contribution by content. `parent` may be a FOREIGN ContentId (another chain's
    /// contribution), which `Cell.parent` cannot express yet: this is how an overlap record is written.
    pub fn new(author: &[u8], data: &[u8], parent: Option<ContentId>) -> Self {
        let data_hash = data_hash(data);
        let cid = content_id(author, &data_hash, parent.as_ref());
        FactRecord { cid, author: author.to_vec(), data_hash, parent }
    }
}

/// A set of contribution facts keyed by ContentId. A BTreeMap, so every output (digest, rival
/// groups, dangling list) comes out in ContentId order on every node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactSet {
    records: BTreeMap<ContentId, FactRecord>,
}

impl FactSet {
    /// Facts from a chain's cells in COMMIT ORDER (pass `&ledger.cells`). Each local `parent` id
    /// resolves to the ContentId of the most recent EARLIER cell with that id. `Err(id)` names the
    /// first cell whose local parent was not committed before it (validation does not check
    /// `parent`): its provenance cannot be named, and dropping or re-rooting it would change the fact.
    pub fn from_cells(cells: &[Cell]) -> Result<Self, u64> {
        let mut set = FactSet::default();
        let mut local: BTreeMap<u64, ContentId> = BTreeMap::new();
        for c in cells {
            let parent = match c.parent {
                None => None,
                Some(p) => Some(*local.get(&p).ok_or(c.id)?),
            };
            let r = FactRecord::new(&c.type_script.args, &c.data, parent);
            local.insert(c.id, r.cid);
            set.records.insert(r.cid, r);
        }
        Ok(set)
    }

    /// Add one record, e.g. an overlap naming a foreign parent. Returns `false` and leaves the set
    /// unchanged if `cid` does not re-derive from the record's fields: a mis-keyed record is what
    /// would let two different records share a key and make the union order-dependent.
    pub fn insert(&mut self, r: FactRecord) -> bool {
        if r.cid != content_id(&r.author, &r.data_hash, r.parent.as_ref()) {
            return false;
        }
        self.records.insert(r.cid, r);
        true
    }

    pub fn get(&self, cid: &ContentId) -> Option<&FactRecord> {
        self.records.get(cid)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// blake2b over the ContentIds in ascending order. Each ContentId commits to its record's full
    /// content, so two sets with equal digests hold the same facts.
    pub fn digest(&self) -> [u8; 32] {
        let cids: Vec<&[u8]> = self.records.keys().map(|k| &k[..]).collect();
        blake(FSET, &cids)
    }

    /// Rival claims, surfaced not resolved: every data hash recorded under MORE than one author,
    /// with those authors (sorted by data hash, authors sorted). One author recording the same data
    /// twice (e.g. under different parents) is not a rival.
    pub fn rival_claims(&self) -> Vec<([u8; 32], Vec<Vec<u8>>)> {
        let mut by_data: BTreeMap<[u8; 32], BTreeSet<&[u8]>> = BTreeMap::new();
        for r in self.records.values() {
            by_data.entry(r.data_hash).or_default().insert(&r.author);
        }
        by_data
            .into_iter()
            .filter(|(_, authors)| authors.len() > 1)
            .map(|(d, authors)| (d, authors.into_iter().map(<[u8]>::to_vec).collect()))
            .collect()
    }

    /// Records whose parent is not in the set: claims awaiting their witness (ascending ContentId).
    pub fn dangling(&self) -> Vec<ContentId> {
        self.records
            .values()
            .filter(|r| r.parent.is_some_and(|p| !self.records.contains_key(&p)))
            .map(|r| r.cid)
            .collect()
    }
}

/// Union of two fact sets keyed by ContentId. Commutative, associative and idempotent: a key present
/// in both sides maps to the same record (the cid re-derives from content), so which side's copy is
/// kept is unobservable.
pub fn merge(a: &FactSet, b: &FactSet) -> FactSet {
    let mut out = a.clone();
    for (k, r) in &b.records {
        out.records.entry(*k).or_insert_with(|| r.clone());
    }
    out
}

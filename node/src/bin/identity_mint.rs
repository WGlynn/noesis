//! identity-mint — the issuer's mint tool for the soulbound identity collection
//! (`noesis::identity_mint`, standard `tokens::soulbound`). Used by the issuer only; the issuer key never
//! goes to the node or the bot.
//!
//! USAGE
//!   identity-mint mint --node http://127.0.0.1:9955 --seed <seed-file> <batch.json>
//!                      [--dry-run] [--confirm <digest prefix>] [--abandon-pending]
//!                      [--daily-cap <n>] [--wait <seconds>] [--state <file>]
//!   identity-mint root --seed <seed-file>          # print the authority's generation-0 lock root (hex)
//!   identity-mint encrypt-seed --seed <seed-file>  # once: seal a plain-text seed file in place
//!
//! THE SEED FILE never enters a repository. Sealed (wallet decision A8): PBKDF2-SHA256 at 600,000 rounds
//! over a random salt, then AES-256-GCM, the web wallet's scheme; `encrypt-seed` converts the old plain
//! form (64 hex characters) in place, verifying the sealed copy before the plain text goes. A plain file
//! is still read, with a warning. The passphrase is typed at a console with no echo, never read from a
//! file or a pipe; only if the issuer opts in is it taken from NOESIS_ISSUER_PASSPHRASE. It is never
//! printed.
//!
//! MINT reads the node's `GET /tokens`, builds ONE soulbound tx for the batch (mints, re-tiers; records
//! already on chain are `unchanged`; records over the per-tx caps or the day's cap are `deferred`), signs
//! it with the key chain and posts it to `POST /token-tx`. The node holds it until the next contribution
//! block carries it; `--wait` polls until then. Output: one JSON line per record on stdout with `status`
//! minted / retiered / unchanged / queued / deferred, `cell_id` and `block_height`.
//!
//! CONFIRMATION. A batch that re-tiers or revokes an identity (or mints one as revoked), any
//! `--abandon-pending`, and a `--daily-cap` above 64 sign only with `--confirm <prefix>`: at least 8 hex
//! characters of the operation digest that `--dry-run` prints for the same batch against the same chain.
//! A dry run signs, writes and sends nothing.
//!
//! DAILY CAP. A state file signs at most 64 issuer operations per UTC day (each mint, re-tier or revoke
//! counts one); over it, records wait as `deferred`, and with none left the run stops (exit 6).
//!
//! ONE-TIME KEYS. The state file (default `<seed-file>.pending.json`) keeps every tx the authority keys
//! signed, by generation, and the tool never deletes it. At a generation whose keys already signed, a run
//! re-sends that exact tx instead of signing another. After a chain reset this replays the issuer's
//! earlier txs in order, one per block, before fresh keys sign anything new. A generation whose keys
//! signed with no copy kept is refused. `--abandon-pending` signs a second digest anyway, knowingly
//! (confirmed). Keep the state file with the seed and back up both.
//!
//! Exit codes: 0 done (or queued, or a dry run) · 2 usage / input / node error · 3 refused by the node ·
//! 4 this generation's keys already signed a different tx (re-sent it; re-run after it lands) · 5 this
//! generation's keys signed a tx the state file has no copy of · 6 the day's cap is used up · 7 needs
//! `--confirm` (missing, too short, or not this operation's digest).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::{Duration, Instant};

use noesis::identity_mint::{self as im, Action, Step};
use serde_json::Value;
use zeroize::Zeroizing;

const PASS_ENV: &str = "NOESIS_ISSUER_PASSPHRASE";

fn die(code: i32, msg: impl std::fmt::Display) -> ! {
    eprintln!("identity-mint: {msg}");
    std::process::exit(code);
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Minimal HTTP/1.1 client over std (the node's server closes each connection after one response).
fn http(method: &str, node: &str, path: &str, body: Option<&[u8]>) -> Result<(u16, Value), String> {
    let hostport = node.strip_prefix("http://").ok_or("--node must look like http://host:port")?.trim_end_matches('/');
    let mut s = TcpStream::connect(hostport).map_err(|e| format!("cannot reach {hostport}: {e}"))?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(60)));
    let _ = s.set_write_timeout(Some(Duration::from_secs(60)));
    let body = body.unwrap_or(b"");
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: {hostport}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes()).and_then(|_| s.write_all(body)).map_err(|e| e.to_string())?;
    let mut raw = Vec::new();
    s.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or("malformed HTTP response")?;
    let status: u16 = std::str::from_utf8(&raw[..split])
        .ok()
        .and_then(|h| h.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .ok_or("malformed HTTP status line")?;
    let v = serde_json::from_slice(&raw[split + 4..]).map_err(|e| format!("response is not JSON: {e}"))?;
    Ok((status, v))
}

fn get_tokens(node: &str) -> Value {
    match http("GET", node, "/tokens", None) {
        Ok((200, v)) => v,
        Ok((s, v)) => die(2, format!("GET /tokens returned {s}: {v}")),
        Err(e) => die(2, e),
    }
}

fn post_envelope(node: &str, envelope: &Value) -> Value {
    match http("POST", node, "/token-tx", Some(envelope.to_string().as_bytes())) {
        Ok((200, v)) => v,
        Ok((_, v)) => die(3, format!("the node refused the tx: {}", v["error"])),
        Err(e) => die(2, e),
    }
}

struct Args {
    cmd: String,
    node: String,
    seed: String,
    batch: Option<String>,
    wait: u64,
    state: Option<String>,
    abandon: bool,
    dry_run: bool,
    confirm: Option<String>,
    daily_cap: Option<usize>,
}

fn parse_args() -> Args {
    let mut it = std::env::args().skip(1);
    let cmd = it.next().unwrap_or_default();
    let mut a = Args {
        cmd,
        node: "http://127.0.0.1:9955".into(),
        seed: String::new(),
        batch: None,
        wait: 0,
        state: None,
        abandon: false,
        dry_run: false,
        confirm: None,
        daily_cap: None,
    };
    while let Some(x) = it.next() {
        match x.as_str() {
            "--node" => a.node = it.next().unwrap_or_else(|| die(2, "--node needs a value")),
            "--seed" => a.seed = it.next().unwrap_or_else(|| die(2, "--seed needs a file")),
            "--wait" => a.wait = it.next().and_then(|w| w.parse().ok()).unwrap_or_else(|| die(2, "--wait needs seconds")),
            "--state" => a.state = Some(it.next().unwrap_or_else(|| die(2, "--state needs a file"))),
            "--abandon-pending" => a.abandon = true,
            "--dry-run" => a.dry_run = true,
            "--confirm" => a.confirm = Some(it.next().unwrap_or_else(|| die(2, "--confirm needs the digest prefix a --dry-run printed"))),
            "--daily-cap" => {
                a.daily_cap = Some(it.next().and_then(|n| n.parse().ok()).unwrap_or_else(|| die(2, "--daily-cap needs a number")))
            }
            other if !other.starts_with("--") && a.batch.is_none() => a.batch = Some(other.to_string()),
            other => die(2, format!("unknown argument {other:?}")),
        }
    }
    if a.seed.is_empty() {
        die(
            2,
            "usage: identity-mint mint --node http://host:port --seed <seed-file> <batch.json> [--dry-run] [--confirm <digest prefix>] \
             [--abandon-pending] [--daily-cap <n>] [--wait <s>] [--state <file>] | identity-mint root --seed <seed-file> \
             | identity-mint encrypt-seed --seed <seed-file>",
        );
    }
    a
}

fn main() {
    let a = parse_args();
    match a.cmd.as_str() {
        "encrypt-seed" => encrypt_seed(&a),
        "root" => println!("{}", hex(&im::authority_root(&load_master(&a), 0))),
        "mint" => mint(&a, &load_master(&a)),
        other => die(2, format!("unknown command {other:?} (mint | root | encrypt-seed)")),
    }
}

// ============ the seed and its passphrase ============

/// The passphrase prompt: a console only, nothing echoed, never a file or a pipe.
mod tty {
    use zeroize::Zeroizing;

    pub const NO_CONSOLE: &str = "the passphrase is typed at a console, with no echo, and never read from a file or a pipe: \
        run this in PowerShell, cmd or Windows Terminal (under Git Bash's mintty, prefix it with `winpty`) or another \
        terminal, or opt in by setting NOESIS_ISSUER_PASSPHRASE";

    #[cfg(windows)]
    pub fn read_hidden(prompt: &str) -> Result<Zeroizing<String>, String> {
        use std::ffi::c_void;
        use std::os::windows::io::AsRawHandle;
        #[link(name = "kernel32")]
        extern "system" {
            fn GetConsoleMode(console: *mut c_void, mode: *mut u32) -> i32;
            fn SetConsoleMode(console: *mut c_void, mode: u32) -> i32;
            fn ReadConsoleW(console: *mut c_void, buf: *mut u16, len: u32, read: *mut u32, control: *mut c_void) -> i32;
        }
        const ENABLE_ECHO_INPUT: u32 = 0x0004;
        struct Restore(*mut c_void, u32);
        impl Drop for Restore {
            fn drop(&mut self) {
                // SAFETY: the console handle and the mode it had before; restoring echo is all this does.
                unsafe { SetConsoleMode(self.0, self.1) };
            }
        }
        let h = std::io::stdin().as_raw_handle();
        let mut mode = 0u32;
        // SAFETY: `h` is this process's standard input; GetConsoleMode fails (returns 0) when it is not a
        // console (a pipe, a file, NUL), which is exactly the refusal wanted.
        if unsafe { GetConsoleMode(h, &mut mode) } == 0 {
            return Err(NO_CONSOLE.into());
        }
        eprint!("{prompt}");
        // SAFETY: the same console handle; clears only the echo bit.
        if unsafe { SetConsoleMode(h, mode & !ENABLE_ECHO_INPUT) } == 0 {
            return Err("could not turn off the console's echo".into());
        }
        let _restore = Restore(h, mode);
        let mut units: Zeroizing<Vec<u16>> = Zeroizing::new(Vec::new());
        let mut buf = Zeroizing::new([0u16; 128]);
        loop {
            let mut n = 0u32;
            // SAFETY: `buf` is a live buffer of `buf.len()` u16s and `n` a live u32.
            if unsafe { ReadConsoleW(h, buf.as_mut_ptr(), buf.len() as u32, &mut n, std::ptr::null_mut()) } == 0 {
                return Err("could not read the console".into());
            }
            units.extend_from_slice(&buf[..n as usize]);
            if n == 0 || units.iter().any(|&u| u == 13 || u == 10) {
                break;
            }
        }
        eprintln!();
        let end = units.iter().position(|&u| u == 13 || u == 10).unwrap_or(units.len());
        String::from_utf16(&units[..end]).map(Zeroizing::new).map_err(|_| "the passphrase is not valid text".into())
    }

    #[cfg(not(windows))]
    pub fn read_hidden(prompt: &str) -> Result<Zeroizing<String>, String> {
        use std::io::{BufRead, IsTerminal, Write};
        if !std::io::stdin().is_terminal() {
            return Err(NO_CONSOLE.into());
        }
        let tty = std::fs::OpenOptions::new().read(true).write(true).open("/dev/tty").map_err(|_| NO_CONSOLE.to_string())?;
        let stty = |arg: &str| {
            tty.try_clone()
                .ok()
                .and_then(|t| std::process::Command::new("stty").arg(arg).stdin(t).status().ok())
                .is_some_and(|s| s.success())
        };
        struct Restore<F: Fn()>(F);
        impl<F: Fn()> Drop for Restore<F> {
            fn drop(&mut self) {
                (self.0)()
            }
        }
        (&tty).write_all(prompt.as_bytes()).map_err(|e| e.to_string())?;
        if !stty("-echo") {
            return Err("could not turn off the terminal's echo (stty)".into());
        }
        let _restore = Restore(|| {
            stty("echo");
        });
        let mut line = Zeroizing::new(String::new());
        std::io::BufReader::new(&tty).read_line(&mut line).map_err(|e| e.to_string())?;
        let _ = (&tty).write_all(b"\n");
        let end = line.trim_end_matches(['\r', '\n']).len();
        line.truncate(end);
        Ok(line)
    }
}

/// The passphrase: from NOESIS_ISSUER_PASSPHRASE if the issuer set it (the opt-in), else typed at the
/// console (twice for a new one). Never echoed, logged or written anywhere.
fn passphrase(prompt: &str, new: bool) -> Zeroizing<String> {
    if let Some(p) = std::env::var_os(PASS_ENV) {
        eprintln!("identity-mint: passphrase taken from {PASS_ENV} (the issuer's opt-in)");
        return Zeroizing::new(p.into_string().unwrap_or_else(|_| die(2, format!("{PASS_ENV} is not valid text"))));
    }
    let first = tty::read_hidden(prompt).unwrap_or_else(|e| die(2, e));
    if new {
        let again = tty::read_hidden("the same passphrase again: ").unwrap_or_else(|e| die(2, e));
        if *first != *again {
            die(2, "the two passphrases differ; nothing changed");
        }
    }
    first
}

/// The issuer's seed from `--seed`: sealed (asks for the passphrase) or, the old form, plain hex.
fn load_master(a: &Args) -> Zeroizing<[u8; 32]> {
    let text = Zeroizing::new(std::fs::read_to_string(&a.seed).unwrap_or_else(|e| die(2, format!("cannot read seed {}: {e}", a.seed))));
    if im::is_sealed(&text) {
        let pass = passphrase(&format!("passphrase for {}: ", a.seed), false);
        return Zeroizing::new(im::open_seed(&text, &pass).unwrap_or_else(|e| die(2, e)));
    }
    eprintln!(
        "identity-mint: WARNING {} is plain text, readable by any program on this machine; seal it once with \
         `identity-mint encrypt-seed --seed {}`",
        a.seed, a.seed
    );
    Zeroizing::new(im::parse_seed(&text).unwrap_or_else(|e| die(2, e)))
}

/// `encrypt-seed`: the one-time, in-place migration of a plain-text seed file (see
/// `identity_mint::encrypt_seed_file` for the order of its checks).
fn encrypt_seed(a: &Args) {
    let path = Path::new(&a.seed);
    let text = Zeroizing::new(std::fs::read_to_string(path).unwrap_or_else(|e| die(2, format!("cannot read seed {}: {e}", a.seed))));
    if im::is_sealed(&text) {
        die(2, format!("{} is already sealed; nothing changed", a.seed));
    }
    im::parse_seed(&text).unwrap_or_else(|e| die(2, format!("{}: {e}; nothing changed", a.seed)));
    let pass = passphrase(
        &format!("a new passphrase for {} ({}+ characters, not shown): ", a.seed, im::MIN_PASSPHRASE_CHARS),
        true,
    );
    im::check_new_passphrase(&pass).unwrap_or_else(|e| die(2, e));
    let root = im::encrypt_seed_file(path, &pass, im::SEED_KDF_ITER).unwrap_or_else(|e| die(2, e));
    println!("{}", hex(&root));
    eprintln!(
        "identity-mint: sealed {} in place (PBKDF2-SHA256, {} rounds, AES-256-GCM); the plain text is gone from it. \
         The line above is its authority root: `identity-mint root --seed {}` asks for the passphrase and must print \
         the same. Without the passphrase the seed is lost: keep the passphrase apart from the file, and replace any \
         plain-text backup of the seed with a copy of this file.",
        a.seed,
        im::SEED_KDF_ITER,
        a.seed
    );
}

// ============ mint ============

fn utc_day() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() / 86_400)
}

/// Write the journal beside its final path, then move it into place, so a crash never leaves half a file.
fn save(path: &str, journal: &im::Journal) {
    let tmp = format!("{path}.tmp");
    im::write_synced(Path::new(&tmp), &journal.to_text())
        .and_then(|_| std::fs::rename(&tmp, path))
        .unwrap_or_else(|e| die(2, format!("cannot write state {path}: {e}")));
}

fn mint(a: &Args, master: &[u8; 32]) {
    let col = im::COLLECTION;
    let batch_path = a.batch.as_deref().unwrap_or_else(|| die(2, "mint needs a batch file"));
    let batch_text = std::fs::read_to_string(batch_path).unwrap_or_else(|e| die(2, format!("cannot read {batch_path}: {e}")));
    let records = im::parse_batch(&batch_text, col).unwrap_or_else(|e| die(2, e));
    let state_path = a.state.clone().unwrap_or_else(|| format!("{}.pending.json", a.seed));
    let mut journal = match std::fs::read_to_string(&state_path) {
        Ok(t) => im::Journal::parse(&t, master).unwrap_or_else(|e| die(2, format!("state file {state_path}: {e}"))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => im::Journal::new(master),
        Err(e) => die(2, format!("cannot read state {state_path}: {e}")),
    };
    let live = im::parse_live(&get_tokens(&a.node), col).unwrap_or_else(|e| die(2, e));
    let g = live.generation;

    let today = utc_day();
    let cap = a.daily_cap.unwrap_or(im::DEFAULT_DAILY_CAP);
    let used = journal.ops_on(today, Some(g));
    // build first: it refuses a seed that does not hold this chain's authority, so a wrong --node never
    // marks this seed's keys as used
    let mut built = im::build_within(&records, &live, master, col, cap.saturating_sub(used)).unwrap_or_else(|e| die(2, e));
    if !a.dry_run && journal.see(g) {
        save(&state_path, &journal);
    }
    let digest = built.digest();
    let digest_hex = digest.map(|d| hex(&d));

    // One-time keys: a generation whose keys already signed re-sends what they signed.
    let reuse = match journal.step(g) {
        Step::Recorded(s) if Some(&s.digest) == digest_hex.as_ref() => {
            if a.dry_run {
                eprintln!("identity-mint: dry run: generation {g}'s tx for this batch is already signed (digest {}); a run re-sends it", s.digest);
                return;
            }
            let (env, d) = (s.envelope.clone(), s.digest.clone());
            return send_and_report(a, &env, &d, g, &built.plan);
        }
        Step::Recorded(s) if !a.abandon => {
            let what = format!(
                "generation {g}'s keys already signed a different tx (digest {}): pending, or signed on a chain since \
                 reset. Signing another would reuse one-time keys",
                s.digest
            );
            if a.dry_run {
                eprintln!("identity-mint: dry run: {what}; a run re-sends that tx instead (exit 4)");
                return;
            }
            let env = s.envelope.clone();
            post_envelope(&a.node, &env);
            die(4, format!("{what}, so it was re-sent. Re-run this batch after it lands (a reset chain replays one tx per block)"));
        }
        Step::Unrecorded if !a.abandon => die(
            5,
            format!(
                "generation {g}'s keys already signed (the tool has seen the authority past it, at generation {}), and \
                 state file {state_path} holds no copy to re-send. Signing again would reuse one-time keys: use the \
                 state file that recorded it, or a fresh issuer seed (a new genesis)",
                journal.seen_generation
            ),
        ),
        Step::Fresh => false,
        _ => true, // --abandon-pending at a generation whose keys already signed
    };

    let Some(digest) = digest else {
        for p in &built.plan {
            println!("{}", im::report(p, p.action.done(), None, col));
        }
        if built.over_budget > 0 {
            die(
                6,
                format!(
                    "the day's cap is used up: {used} of {cap} issuer operations signed today (UTC), so {} record(s) \
                     wait until 00:00 UTC",
                    built.over_budget
                ),
            );
        }
        eprintln!("identity-mint: nothing to send (authority generation {g})");
        return;
    };
    let digest_hex = hex(&digest);
    if built.over_budget > 0 {
        eprintln!(
            "identity-mint: daily cap: {used} of {cap} issuer operations signed today (UTC); {} record(s) deferred to the next UTC day",
            built.over_budget
        );
    }

    let mut why: Vec<&str> = im::needs_confirm(&built.plan).into_iter().collect();
    if a.abandon {
        why.push(if reuse {
            "--abandon-pending signs a second digest with one-time keys that already signed"
        } else {
            "--abandon-pending was given"
        });
    }
    if cap > im::DEFAULT_DAILY_CAP {
        why.push("--daily-cap raises the day's cap above 64");
    }
    if a.dry_run {
        for p in &built.plan {
            println!("{}", im::report(p, p.action.planned(), None, col));
        }
        eprintln!("identity-mint: dry run: nothing signed, written or sent");
        eprintln!(
            "identity-mint: operation digest {digest_hex} (authority generation {g} -> {}), {} issuer operation(s); \
             {used} of {cap} used today (UTC)",
            g + 1,
            built.ops()
        );
        if !why.is_empty() {
            eprintln!("identity-mint: to sign it, run the same command with --confirm {} instead of --dry-run ({})", &digest_hex[..im::MIN_CONFIRM_HEX], why.join("; "));
        }
        return;
    }
    if !why.is_empty() || a.confirm.is_some() {
        if let Err(e) = im::confirm_matches(&digest, a.confirm.as_deref()) {
            let because = if why.is_empty() { String::new() } else { format!(" This run needs the issuer's typed confirmation: {}.", why.join("; ")) };
            die(7, format!("{e}.{because} Run it with --dry-run, check the plan, then run it with --confirm <the first 8+ hex characters of the digest it prints>"));
        }
    }
    if reuse {
        eprintln!("identity-mint: WARNING --abandon-pending: signing a second digest with generation {g}'s one-time keys, which weakens them");
    }

    // Size check before any signature exists (signatures have a fixed length).
    let mut probe = built.tx.clone().expect("a digest implies a tx");
    probe.auths = vec![vec![0u8; 16 * 1024]; probe.inputs.len()];
    if im::envelope(&probe).to_string().len() >= noesis::rpc::MAX_BODY {
        die(2, "the signed tx would exceed the node's body cap; split the batch");
    }

    built.sign();
    let envelope = im::envelope(built.tx.as_ref().expect("signed tx"));
    journal.record(g, digest_hex.clone(), envelope.clone(), today, built.ops());
    save(&state_path, &journal); // recorded before it is sent
    send_and_report(a, &envelope, &digest_hex, g, &built.plan);
}

/// Post a signed tx, wait for its block if asked, and print one line per record.
fn send_and_report(a: &Args, envelope: &Value, digest_hex: &str, g: u64, plan: &[im::Planned]) {
    let col = im::COLLECTION;
    let resp = post_envelope(&a.node, envelope);
    eprintln!("identity-mint: queued tx {digest_hex} (authority generation {g} -> {}), {}", g + 1, resp["note"]);
    let deadline = Instant::now() + Duration::from_secs(a.wait);
    let mut landed: Option<Option<u64>> = None;
    loop {
        let t = get_tokens(&a.node);
        let height = t["included"]
            .as_array()
            .and_then(|v| v.iter().find(|i| i["digest"].as_str() == Some(digest_hex)))
            .and_then(|i| i["height"].as_u64());
        let advanced = im::parse_live(&t, col).map(|l| l.generation > g).unwrap_or(false);
        if height.is_some() || advanced {
            landed = Some(height);
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    for p in plan {
        let (status, h) = match (p.action, landed) {
            (Action::Mint | Action::Retier, Some(h)) => (p.action.done(), h),
            (Action::Mint | Action::Retier, None) => ("queued", None),
            (other, _) => (other.done(), None),
        };
        println!("{}", im::report(p, status, h, col));
    }
    if landed.is_some() {
        eprintln!("identity-mint: landed (the state file keeps the signed tx, for a re-mint after a reset)");
    } else {
        eprintln!("identity-mint: queued; the next contribution block carries it (re-run the same batch to check)");
    }
}

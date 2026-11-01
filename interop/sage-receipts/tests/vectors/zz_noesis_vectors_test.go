package outcomereceipt

// Cross-implementation vectors for the Noesis verifier (interop/sage-receipts in WGlynn/noesis).
// Written in a scratch clone only; not part of SAGE. Each case records the exact input bytes and
// SAGE's own verdict (Decode + VerifyEnvelope), plus the digest and canonical body when accepted,
// so an independent implementation can be checked against the reference byte for byte.
//
//   NOESIS_VECTORS_OUT=path go test ./internal/outcomereceipt -run TestDumpNoesisVectors

import (
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"
)

type noesisVector struct {
	Name      string `json:"name"`
	Input     string `json:"input"`
	Accept    bool   `json:"accept"`
	Digest    string `json:"digest,omitempty"`
	Canonical string `json:"canonical,omitempty"`
	TaskSet   string `json:"task_set_digest,omitempty"`
	GoError   string `json:"go_error,omitempty"`
}

func vectorKey(label string) ed25519.PrivateKey {
	seed := sha256.Sum256([]byte("noesis-vectors/" + label))
	return ed25519.NewKeyFromSeed(seed[:])
}

func signAs(t *testing.T, receipt Receipt, role, signerID string, key ed25519.PrivateKey) Signature {
	t.Helper()
	sig, err := Sign(receipt, role, signerID, key)
	if err != nil {
		t.Fatalf("sign %s: %v", role, err)
	}
	return sig
}

func envelopeJSON(t *testing.T, receipt Receipt, sigs ...Signature) string {
	t.Helper()
	data, err := json.Marshal(SignedReceipt{Receipt: receipt, Signatures: sigs})
	if err != nil {
		t.Fatalf("marshal: %v", err)
	}
	return string(data)
}

func evaluate(name, input string) noesisVector {
	v := noesisVector{Name: name, Input: input}
	env, err := Decode([]byte(input))
	if err == nil {
		err = VerifyEnvelope(env)
	}
	if err != nil {
		v.GoError = err.Error()
		return v
	}
	digest, _ := DigestHex(env.Receipt)
	canonical, _ := CanonicalJSON(env.Receipt)
	taskDigest, _ := TaskSetDigest(env.Receipt.TaskSet.Tasks)
	v.Accept, v.Digest, v.Canonical, v.TaskSet = true, digest, string(canonical), taskDigest
	return v
}

func resign(t *testing.T, r Receipt) string {
	t.Helper()
	return envelopeJSON(t, r, signAs(t, r, RoleEvaluator, "eval-harness-1", vectorKey("evaluator")))
}

func TestDumpNoesisVectors(t *testing.T) {
	out := os.Getenv("NOESIS_VECTORS_OUT")
	if out == "" {
		t.Skip("set NOESIS_VECTORS_OUT to write vectors")
	}
	evalKey := vectorKey("evaluator")
	var vs []noesisVector

	// ---- accepted by the reference ----
	base := validReceipt(t)
	baseJSON := resign(t, base)
	baseSig := signAs(t, base, RoleEvaluator, "eval-harness-1", evalKey)
	invalid := func(r Receipt) string { return envelopeJSON(t, r, baseSig) }
	vs = append(vs, evaluate("base: SAGE's own validReceipt fixture", baseJSON))

	esc := validReceipt(t)
	esc.Caveats = []string{"quote \" backslash \\ angle <tag> & amp", "line sep \u2028 para sep \u2029 end", "accent \u00e9 emoji \U0001F600 cjk \u6f22"}
	vs = append(vs, evaluate("escapes in text fields", resign(t, esc)))

	failed := validReceipt(t)
	failed.Summary = Summary{Status: StatusFailed, Pairs: 4}
	failed.Runs[0].Error = "ctrl \b \f \n \r \t \x01 \x1f \x7f quote \" sep \u2028 <&>"
	failed.Runs[1].Error = "timeout"
	vs = append(vs, evaluate("failed status: run errors carry raw control characters", resign(t, failed)))

	frac := validReceipt(t)
	start := time.Date(2026, 10, 26, 8, 0, 0, 123456789, time.UTC)
	frac.Evaluation.StartedAt = start
	frac.Evaluation.CompletedAt = start.Add(5*time.Minute - 23456789)
	frac.CreatedAt = frac.Evaluation.CompletedAt.Add(20 * time.Millisecond)
	vs = append(vs, evaluate("fractional-second timestamps (trailing zeros trimmed)", resign(t, frac)))

	big := validReceipt(t)
	yes, no := true, false
	for i := range big.Runs {
		if big.Runs[i].TaskMemoryID == "mem-task-a" && big.Runs[i].Trial == 1 {
			big.Runs[i].Seed = ^uint64(0)
		}
		big.Runs[i].LatencyMicros = int64(i) * 1000003
	}
	big.Runs[0].LatencyMicros = 9223372036854775807
	big.Runs[2].Score, big.Runs[2].Passed = "", &yes
	big.Runs[3].Passed = &no
	vs = append(vs, evaluate("max uint64 seed, max int64 latency, passed without score", resign(t, big)))

	sets := validReceipt(t)
	sets.Evidence = append(sets.Evidence,
		EvidenceRef{Kind: "raw-runs", MediaType: "application/x-ndjson", Digest: hashOf("raw"), URI: "https://example.org/raw?a=1&b=<2>"},
		EvidenceRef{Kind: "judge", MediaType: "text/plain", Digest: hashOf("judge")})
	for i := range sets.Runs {
		sets.Runs[i].MaskedMemoryIDs = append([]string{"mem-z-other", "mem-a-other"}, sets.Runs[i].MaskedMemoryIDs...)
		sets.Runs[i].ArtifactHashes = append(sets.Runs[i].ArtifactHashes, hashOf("zzz"), hashOf("aaa"))
	}
	sageKey, noesisKey := vectorKey("sage-agent"), vectorKey("noesis")
	sageID := strings.ToLower(hexOf(sageKey.Public().(ed25519.PublicKey)))
	vs = append(vs, evaluate("unsorted sets, three evidence refs, three signers", envelopeJSON(t, sets,
		signAs(t, sets, RoleNoesis, "noesis-node-1", noesisKey),
		signAs(t, sets, RoleSAGEAgent, sageID, sageKey),
		signAs(t, sets, RoleEvaluator, "eval-harness-1", evalKey))))

	// Valid for SAGE, but the evaluator graded its own memory: a consumer policy must see that.
	self := validReceipt(t)
	self.Candidate.AuthorAgentID = hexOf(evalKey.Public().(ed25519.PublicKey))
	vs = append(vs, evaluate("self-evaluation: the evaluator key is the candidate's author", resign(t, self)))

	// Same receipt as base, reformatted at the JSON level: the digest must not move.
	vs = append(vs, evaluate("base with .000Z timestamps", strings.ReplaceAll(baseJSON, ":00Z\"", ":00.000Z\"")))
	var generic map[string]any
	_ = json.Unmarshal([]byte(baseJSON), &generic)
	pretty, _ := json.MarshalIndent(generic, "", "   ")
	vs = append(vs, evaluate("base pretty-printed with keys in another order", string(pretty)))
	rev := validReceipt(t)
	for i, j := 0, len(rev.Runs)-1; i < j; i, j = i+1, j-1 {
		rev.Runs[i], rev.Runs[j] = rev.Runs[j], rev.Runs[i]
	}
	rev.TaskSet.Tasks[0], rev.TaskSet.Tasks[1] = rev.TaskSet.Tasks[1], rev.TaskSet.Tasks[0]
	vs = append(vs, evaluate("base with runs and tasks reordered", resign(t, rev)))

	// Timestamp spellings whose treatment is decided by the reference, not guessed.
	vs = append(vs, evaluate("lowercase z suffix", strings.Replace(baseJSON, "\"created_at\":\"2026-10-26T08:05:00Z\"", "\"created_at\":\"2026-10-26T08:05:00z\"", 1)))
	vs = append(vs, evaluate("lowercase t separator", strings.Replace(baseJSON, "\"created_at\":\"2026-10-26T08:05:00Z\"", "\"created_at\":\"2026-10-26t08:05:00Z\"", 1)))
	vs = append(vs, evaluate("comma fraction", strings.Replace(baseJSON, "\"created_at\":\"2026-10-26T08:05:00Z\"", "\"created_at\":\"2026-10-26T08:05:00,5Z\"", 1)))
	vs = append(vs, evaluate("ten fraction digits", strings.Replace(baseJSON, "\"created_at\":\"2026-10-26T08:05:00Z\"", "\"created_at\":\"2026-10-26T08:05:00.1234567891Z\"", 1)))
	vs = append(vs, evaluate("+00:00 offset", strings.Replace(baseJSON, "\"created_at\":\"2026-10-26T08:05:00Z\"", "\"created_at\":\"2026-10-26T08:05:00+00:00\"", 1)))
	vs = append(vs, evaluate("leap second", strings.Replace(baseJSON, "\"created_at\":\"2026-10-26T08:05:00Z\"", "\"created_at\":\"2026-10-26T08:05:60Z\"", 1)))
	vs = append(vs, evaluate("key in another case", strings.Replace(baseJSON, "\"version\":\"sage.noesis", "\"Version\":\"sage.noesis", 1)))
	vs = append(vs, evaluate("null for a string field", strings.Replace(baseJSON, "\"name\":\"sage-noesis-pilot\"", "\"name\":null", 1)))

	// ---- rejected by the reference ----
	vs = append(vs, evaluate("tampered delta", strings.Replace(baseJSON, "\"delta\":\"0.25\"", "\"delta\":\"0.26\"", 1)))
	vs = append(vs, evaluate("unknown field", strings.Replace(baseJSON, "\"version\":\"sage.noesis", "\"extra\":true,\"version\":\"sage.noesis", 1)))
	vs = append(vs, evaluate("duplicate key", strings.Replace(baseJSON, "\"metric\":\"task_pass_rate\"", "\"metric\":\"task_pass_rate\",\"metric\":\"task_pass_rate\"", 1)))
	vs = append(vs, evaluate("trailing value", baseJSON+" {}"))
	vs = append(vs, evaluate("deep nesting", strings.Replace(baseJSON, "{\"receipt\":", "{\"x\":"+strings.Repeat("[", 70)+strings.Repeat("]", 70)+",\"receipt\":", 1)))
	nfd := validReceipt(t)
	nfd.Caveats = []string{"accent e\u0301 not composed"}
	vs = append(vs, evaluate("non-NFC text", invalid(nfd)))
	tab := validReceipt(t)
	tab.Caveats = []string{"tab\there"}
	vs = append(vs, evaluate("control character in a text field", invalid(tab)))
	trail := validReceipt(t)
	trail.Summary.ControlMean = "0.50"
	vs = append(vs, evaluate("decimal with trailing zero", invalid(trail)))
	negz := validReceipt(t)
	negz.Summary.Delta = "-0"
	vs = append(vs, evaluate("negative zero", invalid(negz)))
	// Negative values between -1 and 0: the negative-zero rule's "-0." prefix check catches them.
	neg := validReceipt(t)
	neg.Summary = Summary{Status: StatusNegative, Pairs: 4, ControlMean: "0.75", TreatmentMean: "0.5", Delta: "-0.25", CI95Lower: "-0.4", CI95Upper: "-0.1"}
	vs = append(vs, evaluate("negative result: delta -0.25 (reference quirk)", invalid(neg)))
	inc := validReceipt(t)
	inc.Summary.Status, inc.Summary.CI95Lower = StatusInconclusive, "-0.1"
	vs = append(vs, evaluate("inconclusive: CI lower bound -0.1 crosses zero (reference quirk)", invalid(inc)))
	big1 := validReceipt(t)
	big1.Summary = Summary{Status: StatusNegative, Pairs: 4, ControlMean: "3", TreatmentMean: "1.5", Delta: "-1.5", CI95Lower: "-2", CI95Upper: "-1"}
	vs = append(vs, evaluate("negative result at or below -1 passes the decimal rule", resign(t, big1)))
	vs = append(vs, evaluate("no evaluator signature", envelopeJSON(t, base, signAs(t, base, RoleSAGEAgent, sageID, sageKey))))
	ev := signAs(t, base, RoleEvaluator, "eval-harness-1", evalKey)
	vs = append(vs, evaluate("duplicate signature", envelopeJSON(t, base, ev, ev)))
	upper := ev
	upper.PublicKey = strings.ToUpper(upper.PublicKey)
	vs = append(vs, evaluate("uppercase public key", envelopeJSON(t, base, upper)))
	incomplete := validReceipt(t)
	incomplete.Runs = incomplete.Runs[:len(incomplete.Runs)-1]
	vs = append(vs, evaluate("missing treatment arm", invalid(incomplete)))
	extra := validReceipt(t)
	for i := range extra.Runs {
		if extra.Runs[i].Arm == ArmTreatment {
			extra.Runs[i].ContextMemoryIDs = []string{"mem-context-b", "mem-context-a", "mem-added", "mem-candidate"}
			break
		}
	}
	vs = append(vs, evaluate("treatment changes more than the candidate", invalid(extra)))
	early := validReceipt(t)
	early.CreatedAt = early.Evaluation.CompletedAt.Add(-time.Second)
	vs = append(vs, evaluate("created before completion", invalid(early)))
	vs = append(vs, evaluate("non-public candidate", strings.Replace(baseJSON, "\"author_agent_id\":\""+hashOf("author")+"\",\"classification\":0", "\"author_agent_id\":\""+hashOf("author")+"\",\"classification\":1", 1)))
	vs = append(vs, evaluate("float trial", strings.Replace(baseJSON, "\"trial\":1,", "\"trial\":1.0,", 1)))
	vs = append(vs, evaluate("negative seed", strings.Replace(baseJSON, "\"seed\":101", "\"seed\":-101", 1)))
	vs = append(vs, evaluate("seed overflow", strings.Replace(baseJSON, "\"seed\":101", "\"seed\":18446744073709551616", 1)))

	data, err := json.MarshalIndent(vs, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(out, append(data, '\n'), 0o644); err != nil {
		t.Fatal(err)
	}
	t.Logf("wrote %d vectors to %s", len(vs), out)
}

func hexOf(b []byte) string {
	const digits = "0123456789abcdef"
	out := make([]byte, 0, 2*len(b))
	for _, c := range b {
		out = append(out, digits[c>>4], digits[c&15])
	}
	return string(out)
}

---
id: SR-12035
title: "QSL-493 PR #918 spec review (integrity): transition identity receiver"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@7949a3262d295ce69f0bbfcab516318c7f7b6c8f; spec/decisions/ADR-016-state-model-finite-execution-mapping.md, spec/functional/FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md, spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, spec/test-cases/TC-471-model-successors-follow-operations-arguments-and-frames.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-471
    type: reviews
---

## Summary

Ticket: QSL-493. PR quire-spec-language#918, diff `origin/main...HEAD` (4 spec files). Read-only review: no edits, no builds. Checked the new transition identity `{type, operation, receiver, arguments}` against QSpec FR-181 (cited by id only), the merged `quire-semantic-value` `ModelTransition` (PR #41), QSpec FR-151 and the other QSL places that describe a transition's receiver or arguments (ADR-027 TS-2, TC-473, FR-187, FR-122, TC-453/FR-101-AC-1).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-120 says `ModelTransition` has `receiver: Option<ObjectReference>`, but the merged quire-semantic-value type holds `receiver: Option<String>`, one canonical JCS text in QSL's typed canonical form, like `operation` and each argument. The spec names a field type the owning crate does not have. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:306-307 |
| FND-002 | medium | ADR-016 ID-9 has an unescaped `\|` inside the code span `…\|null`. GFM splits table cells on it even inside backticks, so the row has 9 cells against 8 columns: the Identity cell is cut, every later column shifts right and the Oracle cell `TC-453 (FR-101-AC-1)` is dropped. Escape it as `\|`. | spec/decisions/ADR-016-state-model-finite-execution-mapping.md:222 |
| FND-003 | medium | TC-473 hand-writes the replayed traces' transition identities as `isZero[<c1>]`, `increment[<c1>]`, `spawn[<c1>]` and `remove[<c1>]`. All four operations take no parameters, so the bracket is the old receiver-as-argument-0 vector. Under the new identity their arguments are `[]` and the receiver is `<c1>`. A test written from this procedure replays a wrong identity. The PR leaves TC-473 unchanged. | spec/test-cases/TC-473-model-effects-are-trace-data-and-ambient-reads-refuse.md:30-35 |

## Verdict

**FIX.** Three findings, none in the bytes. I recomputed the golden JCS by hand and it is correct. FR-120-AC-1 and TC-471 step 1 read `{"arguments":[],"operation":"{\"digest_domain\":\"sha256-jcs\",\"node\":\"test/counters/Counter/increment\",\"package\":\"test/counters\"}","receiver":<c1>,"type":"transition"}`. The top-level keys are in UTF-16 order `arguments < operation < receiver < type`. The inner keys are in order `digest_domain < node < package`. Inner quotes are escaped `\"`, `/` is not escaped (RFC 8785), and `arguments` is `[]` for a parameterless operation. FR-120-AC-2 and TC-471 step 2 have receiver `<c1>` and arguments `[{"type":"integer","value":"n"}]`, with keys in order `type < value`. The receiver stays out of the arguments vector. FR-101-AC-1's `10`-before-`9` still holds, since `arguments` sorts first. The PR matches QSpec FR-181 (a separate receiver, null when absent, AC-7 two receivers differing only in `receiver`) and ADR-027 TS-2 / FR-187 `ScheduledIdentity` (receiver kept apart). The FR-151 citation now says "the receiver" without "parameter 0". That is acceptable: FR-151's own "parameter 0" is a conformance-axis statement, and FR-120 now says plainly that the receiver is not an argument. FR-122 does not describe the identity.

## Dispositions

Round 1, reviewed at `82b9a1d3608b6ea532027c983030f0a12f73d54b` (delta `7949a32..82b9a1d36`).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 82b9a1d36: FR-120 now types `ModelTransition` as the operation's canonical text, `receiver: Option<String>` and a vector of canonical argument texts, with `None` for no receiver and `Some("null")` refused, matching quire-semantic-value `src/model_transition.rs`; no `Option<ObjectReference>` remains in `spec/` |
| FND-002 | fixed | 82b9a1d36: ADR-016 ID-9 escapes the pipe as `\|null`; the row now has 8 cells like ID-8 and ID-10 |
| FND-003 | fixed | 82b9a1d36: TC-473 steps 1 and 2 write each trace with receiver `<c1>` and arguments `[]` (isZero, increment, spawn, remove); no `op[<c1>]` notation remains in `spec/` |

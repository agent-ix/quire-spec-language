---
id: SR-776
title: "QSL-301 code review of PR 514 (FR-116 FrameCounterexample payload and native frame replay)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-replay/src/execute.rs; qsl-replay/src/execute/frame.rs; qsl-replay/src/lib.rs; qsl-replay/src/spine.rs; qsl-replay/src/spine/clause.rs; qsl-replay/src/witness.rs; qsl-replay/src/witness/frame.rs; qsl-replay/src/result.rs (read, unchanged); qsl-replay/src/request.rs (read, unchanged); qsl-semantics/src/check/lowering.rs; qsl-semantics/src/check/lowering/state.rs; qsl-semantics/src/check/mod.rs; qsl-semantics/src/check/state_clause.rs; qsl-semantics/src/model/observation.rs (read, unchanged); spec/functional/FR-116-replay-a-frame-counterexample.md; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-515
    type: reviews
---
## Summary

Ticket: QSL-301. PR: quire-spec-language#514. This review covers
code and Rust (the rust-review lane is folded in here).

What was checked:

- **Shared frame path.** `run_frame` is now `resolve_frame` followed by
  `check_frame`. `replay_frame` calls the same two `pub(crate)` functions. It
  does not copy them. Only the `FrameRun` inputs differ: replay uses default
  admission and model limits, and says so in its doc comment.
- **Refusal order against FR-116 Behavior.** FR-098 decode and `recompile`
  (which checks the request's `package_id`), then the envelope's
  `package_id`, then operation resolution, then identity checks, then
  admission inside `check_frame`, then the frame check. This is the order in
  FR-116's text. Admission cannot hide an identity refusal, because identities
  are checked first. The one problem is the order *within* the identity
  checks (FND-001).
- **Settlement.** `DisagreementCause::of(proved, replayed, has_value)` gives:
  violation/violation with a value is agreement, so
  `reproduced-with-evaluated-witness`; violation/success with a value is
  `Verdicts`; violation/refusal with no value is `NoValue`. The
  `population_delta_mismatch` refusal reaches the NoValue arm through
  `Evaluate(Refused)`. Each arm is reachable and tested.
- **`ClaimedChange::of_witness`.** One arm per `FrameChange` variant, with no
  `_` arm, so a new change kind will not compile until it is handled here.
  Each arm copies the object key, the field or types, and the witness
  population, and drops the permission (`modifies`/`creates`/`deletes`).
  `Retyped` is a real check-11 outcome (FR-106 `FrameTypeChanged`). The PR
  itself added retyping to FR-116's Inputs, so "FR-116 names Retyped" is true
  only after this PR's own edit. The edit is grounded in FR-106, but it cited
  the wrong FR (FND-003).
- **Unused request fields.** FR-116 Inputs lists only "FR-098's package
  reference, byte provision and limits" from the request. Ignoring
  `selected_function` and `source` follows the spec: the selection is the
  payload and the result arm is the envelope's. It is not an oversight.
- **Rust idioms.** No `unsafe`, `unwrap` in production code, or casts. The
  large error is boxed (`Box<FrameIdentityMismatch>`). Invariant breaks become
  `InternalFault`s, not panics. Disposition matches are total. `charges` maps
  every `LimitKind` with one exhaustive match.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `check_identities` checks the anchor before the frame. But the `operation_anchor` node's content references the frame node (`state.rs` `operation_anchor`: `binding("frame", reference(frame))`), so a changed frame always changes the anchor. A real envelope built from a package whose frame differs is then refused `FrameIdentityMismatch::Anchor`, not the "both frame identities" FR-116-AC-3 requires. The `Frame` variant can only be reached by a spliced payload (current anchor, other frame), which no producer can emit. The AC-3 test builds exactly that splice, so it passes. Confirmed by running the test with all three identities from the other package against the original order: it refuses `Anchor{3cd0…, 7a5f…}`. | qsl-replay/src/execute/frame.rs:362-388; qsl-semantics/src/check/lowering/state.rs:283-311 |
| FND-002 | low | The FR-116 row in `spec/spec.md` still says "not yet implemented -- TC-515 planned". The PR updated FR-116 Status and tests.md, but not this row (the same miss as QSL-300's FND-003). | spec/spec.md:542 |
| FND-003 | low | FR-116 Inputs (PR edit) says "FR-115's four change kinds". FR-115's text lists three (write, create, delete). The fourth, retyping, comes from FR-106's check 11 (`FrameTypeChanged`), which FR-115 runs. | spec/functional/FR-116-replay-a-frame-counterexample.md:49-51 |
| FND-004 | low | The lib.rs comment says the re-exports let CG build the payload "through this crate alone". Building a `FrameCounterexample` also needs `WireNodeId` (qsl_foundation), and `Origin` and `Identifier` (quire_exact). None of these is re-exported. This was already true of `WitnessPacket`, so the fix is the comment, not new exports. | qsl-replay/src/lib.rs:71-73 |
| FND-005 | low | `replay_frame` does not compare the envelope's own `occurrence_key`, `clause_node` or `source_digests` with the payload or the request. The result reports the payload's identities. FR-116 requires only the `package_id` cross-check, so this is not a spec violation. It is a gap to settle when CG defines what it puts in those O-25 members for a frame counterexample. | qsl-replay/src/execute/frame.rs:167-188 |

## Verdict

PASS after fixes. FND-001 was a real AC-3 defect for any real input. It is
fixed in-PR: the check order is now frame, occurrence, anchor, and the AC-3
test now takes all three identities from the other package. The shared
`check_frame` path, the refusal order, the three settlement arms and the
`of_witness` decode are correct.

## Dispositions

| FND | Outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `check_identities` checks frame, occurrence, anchor; AC-3 test takes anchor, frame and occurrence from the parent-modifying package and asserts the anchors differ |
| FND-002 | fixed | spec.md FR-116 row: implemented under QSL-301 (QSL-21f), TC-515 passed locally |
| FND-003 | fixed | FR-116 Inputs: "the four change kinds of FR-106's check 11, which FR-115 runs" |
| FND-004 | fixed | lib.rs comment names the qsl_foundation and quire_exact types CG also needs |
| FND-005 | deferred | FR-116 names only the `package_id` cross-check. What CG puts in the envelope's `occurrence_key`/`clause_node` for a frame counterexample is not defined until quire-contract-codegen#49 lands, so there is nothing to check against yet |

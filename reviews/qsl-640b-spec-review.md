---
id: SR-1358
title: "Spec review of quire-spec-language PR #645: FR-070, FR-098, FR-263, FR-357-AC-16, FR-358 and TC-736, TC-905 to TC-907 (QSL-640)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6ec5705d9187c030367108c3078c3297490b1a96; spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md, spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-263-replay-at-any-depth-under-the-request-limits.md, spec/functional/FR-357-replay-scalar-parity-claims.md (AC-16), spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md, spec/test-cases/TC-736, TC-904 (step 15), TC-905, TC-906, TC-907, spec/spec.md and spec/tests.md rows; context: spec/decisions/ADR-013 O-09 parity preimage (merged in #650)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-358
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-263
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-907
    type: reviews
---
# Spec review of quire-spec-language PR #645

## Summary

Ticket: QSL-640. Reviewed head 6ec5705d9, which includes the spec-only
commit 6ec5705d (FR-358 Status and the composite literal operand rule).
Sub-analyses folded in: integrity (status rows, table structure),
failure-domain (occurrence identity, limit stages) and object review (the
O-09 operand forms). `quire validate` passes on FR-070, FR-098, FR-263,
FR-357, FR-358, TC-736 and TC-905 to TC-907.

Conformance with the decided rulings:

- FR-358 `Refinement` is one closed enum used by both entries; F-1..F-6
  and V-1..V-5 match the ruling, and the common-step refusals (identity
  refusals included) come before F-1.
- Step 6 makes the obligation identity O-09's one parity preimage. The
  Status records that QSL recomputes it through `parity_obligation` with the
  membership checks.
- Step 3 gives enum leaves `Variants` and other non-Boolean leaves a
  never-covered whole-domain position; step 4 takes the request's
  `DeclaredDomain` as the declared bound, keys recursive `Depth` at first
  entry, refuses a wrong-kind `DeclaredDomain` and names
  `ParityBoundRefusal::PositionLimit` (`stage_limit_exceeded`).
- FR-070 decodes integer leaves inside value texts to `ExactInteger`,
  requires strictly ascending entries, finds set duplicates by kernel
  equality (FR-098-AC-8) and refuses float sets.
- FR-357-AC-16 and TC-904 step 15 now say a native-outcome-only difference
  leaves the obligation identity unchanged and is caught by `report.claim()`.

Question (2), the composite literal operand: identifying it as
`graph_child` with its own node id fits O-09's definition ("an operand that
is a graph node, the application's child at that position"), and an empty
`bounds` domain is sound because the content-addressed node id already
binds the value. But O-09 defines `bounds` as "a composite operand's harness
bounds" and states no empty-`bounds` rule for a literal, and FR-358 step 6
itself still says "or a literal's singleton". That is a spec gap, and per
the coordinator it is fixed inside #645 (FND-002).

## Verdict

Request changes, for FND-001 to FND-004. The rulings are otherwise
carried faithfully and testably.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-358 step 6 names refusals that do not exist: `ObligationIdentityMismatch` for a digest mismatch and `ObligationIdentityUnencodable` for an encoder refusal. The ruling, ADR-013 O-09 ("Who computes it") and the code all use `ReplayRefusal::ScalarIdentity` with the cause `Obligation` (naming both digests) or `Encoding` (`stale_dependency`/`revision-mismatch`). Fix: state step 6's refusals as `ScalarIdentity` with causes `Obligation` and `Encoding`, as FR-357 does, and list the membership-check causes (`OperandChild`, `NotInlineLiteral`, `OperandCount`). | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:239-249 |
| FND-002 | medium | The composite literal operand's identity is stated only in FR-358's Status, and contradicts FR-358's normative text. Step 6 says each operand's domain is its harness bounds "or a literal's singleton", and step 3 calls a literal's domain "the singleton holding its value". The code and the Status make a composite literal `graph_child` with `{"tag":"bounds","entries":[]}`. ADR-013 O-09 defines `bounds` as "a composite operand's harness bounds", gives no rule for a literal, and its membership check covers only a `graph_child` that is the node's argument. Fix inside #645: in O-09, add that a composite literal operand is `graph_child` with its own node id and empty `bounds` entries, and that only an integer inline literal is `inline_literal` with the singleton `range`; move the rule from FR-358's Status into step 6; and state what an inline non-integer literal operand does (today `NotInlineLiteral`). | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:239-249, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:290-294 |
| FND-003 | medium | FR-358's `CompositeParityClaim` has no occurrence key, so the claim cannot say which occurrence of its node it covers, and the spec does not say which one QSL uses. One node can occur more than once in one function: `(a = b) and (a = b)` gives one node with two `expression` occurrences (probe at this head, SR-1356 FND-001). ADR-013 O-09 makes those two obligations, so the second can never be claimed. Fix: add `occurrence` (the node's occurrence key) to the claim table, require it to be an occurrence of `node` in the selected function's body (refusing `ScalarIdentity` with cause `Occurrence`), and use it in step 6's preimage; add the case to AC-3. | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:88-99 |
| FND-004 | medium | The falsified rows do not say where an accounting-limit stop during admission goes. F-2 is "an operand fails admission" (refusal) and F-4 is "QSL's exact evaluation reaches a limit" under the claim's `limits`, but admission runs under the request's `accounting_limits` and can stop before any evaluation. The code reports it as `Incomplete { stage: ExactEvaluation }` before F-3, which masks a native fault (SR-1356 FND-002). Fix: state the row and the stage for an admission limit stop, and its order against F-3, and add it to AC-2. | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:255-270 |
| FND-005 | low | Index and tracker rows are stale or regressed. spec.md still says FR-070 and FR-098's QSL-640 parts are "specified, not yet implemented" with TC-905, TC-736 and TC-906 "planned"; FR-263 "not yet implemented, TC-736 planned"; and FR-358 "Not yet implemented, TC-907 planned". tests.md TC-907 still lists "the identity-tie cases" as pending, though the identity tie is implemented and tested. The FR-357 row lost its text on `claim()` and the carry-and-bind observation digest, which #650 had added. TC-905 to TC-907 sit after TC-908 in tests.md. Fix: update the rows to the Status sections, restore the FR-357 sentence, and order the TC rows. | spec/spec.md:990, spec/spec.md:1017, spec/spec.md:1239, spec/spec.md:1242-1243, spec/tests.md:537-540 |
| FND-006 | low | FR-070's Status cites `WitnessEnvelope::reconstruct_within` and FR-098's Status cites `qsl_replay::replay_within`. Neither exists after the rebase onto B5: the raised `replay.input_bytes` goes through `ReplayLimits::with_input_bytes` passed to `WitnessEnvelope::reconstruct` and `replay`. Fix: name the real entries. | spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:278, spec/functional/FR-098-execute-a-replay-request.md:262 |

## Dispositions

Round 1, reviewed at dfc72b6d120b4bc914933300007032a7c6c67537 (fix commits 6ec5705d9..dfc72b6d1). Also checked against the plan lead's round-1 rulings R1 to R5: F-1 to F-7 in ruling order with the new `Admission` stage; `RefusedInput` settles `Inconclusive(ReplayRefused(invalid_runtime_input))`, the ruling's outcome; `ReplayLimits` on every parity entry and `compile_package`; the full claim on both composite reports; typed cause names only; and O-09 states the composite literal (empty `bounds`, justified because O-09 has no composite singleton form and the content-addressed node id binds the value) and the drawn bounds of an unbounded operand.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 70693d605 |
| FND-002 | fixed | 70693d605 |
| FND-003 | fixed | 70693d605 |
| FND-004 | fixed | 70693d605 |
| FND-005 | fixed | 70693d605 |
| FND-006 | fixed | 70693d605 |

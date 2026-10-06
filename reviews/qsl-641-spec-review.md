---
id: SR-1344
title: "Spec review of quire-spec-language PR #644: FR-357, TC-904, FR-098 and FR-100 (QSL-641)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9922c999ae052ae227e16902ba62d8da05c6ae96; spec/functional/FR-357-replay-scalar-parity-claims.md, spec/test-cases/TC-904-replay-scalar-parity-claims.md, spec/functional/FR-098-execute-a-replay-request.md (selection bullet), spec/functional/FR-100-run-a-named-function-through-the-spine.md (NotAPredicate paragraph), spec/spec.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---
# Spec review of quire-spec-language PR #644

## Summary

Ticket: QSL-641. Reviewed head 9922c999.

What conforms:

- FR-357 states the decided contract. It has the two entries; a scalar
  claim is about the lowering, not the spec; never `Refuted`. Its Behavior
  section carries the 20:12 operator inputs and the proof-projection
  comparison, with no cause or charge comparison. It says Incomplete and
  ExecutionFault settle `Failed` without a comparison. It carries R6 (the
  obligation on every settlement) and R7 (recompile, check the package_id,
  node and operator membership, no synthetic predicate, `NotAPredicate`
  unreachable). It has the function-level witness of declared parameters
  only, and the typed `ScalarAgrees` cause counted by FR-069-AC-4.
- The FR-098 and FR-100 edits scope `NotAPredicate` to predicate replay,
  as the ticket's acceptance asks.
- spec.md and tests.md carry the new rows.

## Verdict

Changes requested on two medium honesty points. FR-357 presents integer
div/rem/mod as working operator-arm operators, but none can pass the
package check today (FND-001). It also gives the package mismatch the wrong
sub-cause (FND-002). FND-003 and FND-004 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-357 is not honest about integer division. Its Inputs list `div` and `rem` under a division law and `mod` as operator-arm operators, and AC-7 says "Operator level, for add and divide: ... each agree exactly when ...". QSL lowers no `quire.op.integer.div`/`rem`/`mod` node (`lowering.rs` lowers `/` to `quire.op.rational.div`; golden.rs NOT_LOWERED). R7's operator check therefore refuses every such claim as `stale_dependency`. Nothing in FR-357 says so. State that no QSL package holds an integer div/rem/mod node today, say what such a claim settles, and scope AC-7 to add, subtract, multiply and negate, or to the comparator. | spec/functional/FR-357-replay-scalar-parity-claims.md (Inputs, Behavior, AC-7) |
| FND-002 | medium | FR-357 Behavior says "A mismatch refuses `stale_dependency`/`revision-mismatch` (`ReplayRefusal::ScalarIdentity`, or `PackageIdMismatch` for the package)". `PackageIdMismatch`'s own cause is `stale_dependency/content-mismatch` (qsl-replay/src/execute.rs:138). Only the node and operator mismatches are `revision-mismatch`. The code follows R7's "the same as other replay identity checks". The spec sentence is wrong for the package case. Split it: package_id → `content-mismatch`, node/operator → `revision-mismatch`. | spec/functional/FR-357-replay-scalar-parity-claims.md (Behavior, first bullet) |
| FND-003 | low | TC-904 step 1 replays "`grow(7)` against generated 2". The test unit declares no `grow` (only `inc`, `inv`, `small`), and no test does this. The step that exists is `inc(3)` against a generated out-of-range outcome. Correct the procedure. | spec/test-cases/TC-904-replay-scalar-parity-claims.md (Test Procedure, step 1) |
| FND-004 | low | FR-357 calls the operator arm's obligation "the `ObligationIdentity` CG minted from the scalar's preimage". ADR-013 O-09 (mirrored in qsl-replay/src/identity.rs:30-39) defines that identity's preimage as a clause's: clause node id, occurrence key, obligation kind and arguments. Reusing the opaque type fits R6's intent (no `QSLObligationIdentity` type exists; QSL only carries it). But the spec now gives one type two preimage definitions. Say in FR-357, or widen O-09 to say, that QSL treats the digest as opaque and CG defines a scalar obligation's preimage. | spec/functional/FR-357-replay-scalar-parity-claims.md (Inputs, operator level) |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | TC-904 step 11 lists "a `mul` node under `inc`, a `+` claim on the `mul` node". The test (`tc_904_an_operator_claim_against_another_identity_is_refused`) checks a `mul` claim on `dbl`'s `mul` node under `inc` (the Function cause) and a `mul` claim on `inc`'s `add` node (the Operator cause). It has no `+` claim on the `mul` node. Make the step match the test: "a `mul` claim on the `add` node". | spec/test-cases/TC-904-replay-scalar-parity-claims.md (Test Procedure, step 11) |

## Dispositions

Round 1, reviewed 5398844084ce266a5010f76b21c6ecdf8015b9b0 (tree diff 9922c999..53988440, same base 4403f2f0e).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 5398844084ce266a5010f76b21c6ecdf8015b9b0 |
| FND-002 | fixed | 5398844084ce266a5010f76b21c6ecdf8015b9b0 |
| FND-003 | fixed | 5398844084ce266a5010f76b21c6ecdf8015b9b0 |
| FND-004 | fixed | 5398844084ce266a5010f76b21c6ecdf8015b9b0 |

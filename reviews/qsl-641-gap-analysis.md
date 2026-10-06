---
id: SR-1343
title: "Gap analysis of quire-spec-language PR #644: scalar-parity replay (QSL-641)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9922c999ae052ae227e16902ba62d8da05c6ae96; PR #644 diff against origin/main; FR-357-AC-1..AC-11; trace tags TC-904 (qsl-replay/src/execute/tests.rs, qsl-replay/src/scalar.rs), TC-177/TC-178/TC-179 (qsl-replay/src/proof_result.rs)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: reviews
---
# Gap analysis of quire-spec-language PR #644

## Summary

Ticket: QSL-641. Reviewed head 9922c999. No build was run.

Every FR-357 AC has at least one `#[trace("TC-904", ...)]` test:

- AC-1 to AC-6 are in `execute/tests.rs`, over `inc` (integer), `inv`
  (Rational) and `small` (Boolean).
- AC-7 to AC-10 are in `scalar.rs`.
- AC-10's envelope half is in `proof_result.rs`: TC-177 settlement,
  TC-178 oversized cause, TC-179 round trip.
- AC-11 is in `execute/tests.rs`, through `replay_operator_parity` on
  `inc`'s `x + 1` node.

The bindings are correct, and the oracles are literal values.

The ticket's acceptance is met: diverged, agrees and refused input, each
for an integer function and a non-Boolean (Rational) function.

## Verdict

Coverage is adequate for the add path and the function arm. FND-001
(medium): AC-7's divide rows are proved only on a crate-private helper that
the public entry can never reach for divide. FND-002 and FND-003 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-357-AC-7 says "Operator level, for add and divide". Its divide rows are tested only through `compare` (`scalar.rs:742`, `replayed`). `compare` is crate-private and skips the R7 package/node/operator check. Through `replay_operator_parity`, the public entry the AC describes, a divide claim always refuses `stale_dependency`, because QSL emits no `quire.op.integer.div` node (SR-1342 FND-001). So the test passes for a path no caller can take. Re-scope AC-7 to the comparison it really proves, or add a public-entry test that pins what a divide claim settles today. | qsl-replay/src/scalar.rs:742-744 |
| FND-002 | low | The function arm's `Incomplete` path is untested. FR-357-AC-10 says an exact evaluation that reaches its limits settles `Incomplete`, but only the operator arm tests it (`tc_904_a_starved_exact_evaluation_is_incomplete`). Add a starved `replay_value_parity` (for example `inc` with `work_units: 0`) asserting `ValueParityResult::Incomplete` and `TerminalValue::Incomplete(ResourceExhausted)`. | qsl-replay/src/execute/value_parity.rs:258-261 |
| FND-003 | low | The R7 refusal's cause is not asserted. `tc_904_an_operator_claim_against_another_identity_is_refused` checks only `Code::StaleDependency`, so the node and operator cases (`ScalarIdentity`, `revision-mismatch`) and the package case (`PackageIdMismatch`, `content-mismatch`) are not told apart. The FR-357 text about which cause applies (SR-1344 FND-002) is therefore unchecked. Match on the refusal variant in each case. | qsl-replay/src/execute/tests.rs (tc_904_an_operator_claim_against_another_identity_is_refused) |

## Dispositions

Round 1, reviewed 5398844084ce266a5010f76b21c6ecdf8015b9b0 (tree diff 9922c999..53988440, same base 4403f2f0e).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 5398844084ce266a5010f76b21c6ecdf8015b9b0 |
| FND-002 | fixed | 5398844084ce266a5010f76b21c6ecdf8015b9b0 |
| FND-003 | fixed | 5398844084ce266a5010f76b21c6ecdf8015b9b0 |

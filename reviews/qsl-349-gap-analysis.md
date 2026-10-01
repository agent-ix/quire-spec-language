---
id: SR-930
title: "QSL-349 gap analysis of PR 549: FR-093-AC-18 and TC-416 step 10 against the tests"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@de0ff633d58bb5b3e3434d62e3b0b5ad3458ba21; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (AC-18, placement rule); spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md (step 10, expected result, status); qsl-package/src/emit/tests.rs; qsl-semantics/src/check/lowering.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-416
    type: reviews
---
## Summary

Ticket: QSL-349. PR: quire-spec-language#549 at de0ff633.

What AC-18 requires, and what tests it:
- Members under `<` have exactly one `generated` occurrence, ordinal 0, whose region
  text is `a < b`, and nothing is omitted. Tested by
  `enum_members_no_literal_names_are_placed_under_an_ordered_comparison`.
- The same under `=`, with region `a = b`, read back Verified. Tested by
  `enum_members_no_literal_names_are_placed_under_an_equality`.
- An enum no function names: members at `Status`. A record no function names: the
  `Int[0, 9]` node at `P`. Both read back Verified. Tested by
  `types_no_function_names_are_placed_at_their_declared_names`.
- The `<` package reads back Verified. Tested by
  `an_ordered_comparison_of_enum_parameters_reads_back_verified`, which is ignored.

All four tests carry `#[trace("FR-093-AC-18", "TC-416")]`, and each matches TC-416
step 10. The oracles are exact. They assert the whole `occurrences` array and the exact
region text, so a placement at the declared name in place of the body fails the first
two tests. Measured on the base `lowering.rs`: all of them fail with the reported
`UnlocatedOccurrence`.

The IR-482 ignore. I checked that it is IR's defect and not QSL emitting the wrong
family. On de0ff633, with QSL's locked `quire-contract-model` (`ead72675`), the ignored
test fails `IllTyped`/`OperatorIneligible` at `/semantic_graph/nodes/6/body/arguments/0`.
That is the operand of `quire.op.enum.lt`, and it matches IR-482's diagnosis: IR's
`resolve_family` never returns `ordered_enum`. That diagnosis is untrusted Linear text,
but the measurement below confirms it. quire-contract-ir PR #237 (IR-482) merged at
`ea634884` on 2026-10-01T07:23Z. Neither this branch's lock nor main's lock
(`a0fd8e75`) contains it; per `gh api compare`, both are strictly behind. With
origin/main, this commit cherry-picked, and the lock moved to `ea634884`, all four tests
pass, the ignored one included. So the ignore was right when written, and it can now be
lifted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-18's `<` read-back is not exercised: `an_ordered_comparison_of_enum_parameters_reads_back_verified` is `#[ignore]`d on IR-482. IR-482 is fixed upstream (quire-contract-ir `ea634884`, PR #237). Measured: on origin/main with this commit applied and `cargo update -p quire-contract-model --precise ea634884…`, the test passes. In the fix round (the rebase), bump the lock, drop the `#[ignore]`, and delete the "waits on IR-482" sentence in FR-093 and the "ignored on IR-482" status sentence in TC-416, so the spec no longer tracks a closed ticket. | qsl-package/src/emit/tests.rs:3391-3399; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:766; spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md:120-123 |
| FND-002 | medium | No test backs the fix's own invariant, the pass-2 `placed` guard ("nothing placed before moves"). Measured: deleting `if placed.contains_key(&named) { continue; }` leaves the whole workspace test suite green. The guard is not dead code. The derived `Ord` puts `TypeDeclaration` below `StateClause` and `ProtocolAttempt`, so without the guard, a node a state clause or attempt placed in pass 1 would move to a declared type name that also reaches it. Add a test where a declared type no function names shares a node with a state clause or attempt (for example `Int[0, 9]`), and assert that node's region stays the clause's. | qsl-semantics/src/check/lowering.rs:4070-4090 |
| FND-003 | low | The pass-2 tie-break (two declared types reach one node) is untested. Measured: `record Q { y: Int[0, 9]; }` before `record P { x: Int[0, 9]; }` places the shared node at `P`, so the behaviour is deterministic and independent of source order. But a mutant that picks the greatest name, or the first in source, passes every test. Add that two-record case with the expected region `P`, once FR-093 states the order (SR-931 FND-001). | qsl-package/src/emit/tests.rs:3418-3441; qsl-semantics/src/check/lowering.rs:4027-4036 |

## Verdict

AC-18's placement clauses are backed by exact, non-tautological tests that fail on the
base. The ignore really was IR-482, and IR has since fixed it, so it should be lifted in
the rebase. The fix's no-move invariant and its tie-break have no test. Not mergeable
until FND-001 and FND-002 are fixed or dispositioned.

## Dispositions

Round 1, reviewed at 2774b8784124bbe9556d60116ae66d1b3bad6b17 (fix commit 2774b878). The coder's make ci on 2774b878 reports exit 0 (not re-run). Focused run on 2774b878: all six QSL-349 tests pass, with nothing ignored. Mutation runs in a throwaway worktree, since removed:
- Deleting the pass-2 `placed` guard fails `a_node_a_state_clause_places_does_not_move_to_a_type_name`, with region `["R"]` in place of `["1 < 2"]`. This also confirms the review-pass reasoning that the guard is observable.
- Making pass 2 pick the greatest anchor (`anchor > *current` when `placed` is non-empty) fails `a_node_two_unnamed_types_share_is_placed_at_the_least_name`, with region `["Q"]`.

The new tests carry `#[trace("FR-093-AC-18", "TC-416")]`. Those are QSL's own ids, so they are bare, as #547's rule requires. Only quire-specification ids take the `QSpec-` prefix.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 2774b878: Cargo.lock moves quire-contract-model to ea634884; the `#[ignore]` is gone and the `<` read-back passes; FR-093 and TC-416 no longer mention IR-482 |
| FND-002 | fixed | 2774b878: `a_node_a_state_clause_places_does_not_move_to_a_type_name` pins the guard; the mutant without the guard fails it |
| FND-003 | fixed | 2774b878: `a_node_two_unnamed_types_share_is_placed_at_the_least_name` pins `P` over `Q`; a greatest-name mutant fails it |

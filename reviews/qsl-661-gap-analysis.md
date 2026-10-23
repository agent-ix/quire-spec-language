---
id: SR-12357
title: "gap analysis of QSL-661 encode_smt_query against FR-315-AC-1..AC-5 (QSL PR #1287)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@39e739bf3a4b9cff0ef63ae985d7c09ea933ddff; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md; qsl-eval/src/smt/; qsl-eval/tests/it/smt_query.rs; qsl-analyze/tests/it/smt_query.rs"
review_set: subset
---
# SR-12357: gap analysis of QSL-661 encode_smt_query against FR-315-AC-1..AC-5 (QSL PR #1287)

## Summary

Ticket: QSL-661. This analysis is planless and scoped to the PR diff. It covers FR-315-AC-1 to
AC-5 (TC-900) and the FR-315 Behavior bullets those ACs exercise. FR-181, FR-182 and FR-183 were
read only as context for how query outcomes settle, and the PR does not implement them. AC-6 to
AC-11 are out of scope by design, and their boundary is clean. Plan completion: not assessed.

| AC | Test(s) | Result |
| --- | --- | --- |
| AC-1 | `an_unrolling_declares_the_copies_and_asserts_the_negated_property`; facade test | Met for the stated fixture. |
| AC-2 | `the_step_case_has_no_initial_state_and_the_base_case_has_one`; facade test | Met. |
| AC-3 | `a_field_with_no_smt_sort_is_refused_by_name` | Met. |
| AC-4 | `an_optional_field_is_a_value_and_a_presence_constant` | Not met, see FND-001. |
| AC-5 | `the_step_case_asserts_distinct_copies_and_the_monitor_steps`, `the_base_case_steps_the_initial_monitor_over_position_zero` | Met. The TP-3 item is `always[0,1]` under infinite-trace. |

The reviewer reproduced that S3 refuses AC-4's guard `pre Has { value(self.held) <= 3 }` with
`Unproved(Presence)` at `StateClause Has`, so neither the AC's claim nor its guard can be built.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test verifies AC-4. The test swaps in the total claim `present(s.held) implies value(s.held) <= 3` and a total guard. So no test asserts that `(not s<i>.s.held.present)` makes the negated property hold, or that the `take` guard adds a definedness disjunct. The AC is unrealisable as written under S3. The UE-1 definedness path for atoms and guards (`defined.not()` and `guard_failure` in `negated`) has no test that could fail. | qsl-eval/tests/it/smt_query.rs:446-480; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:308 |
| FND-002 | medium | Every test is a string match on printed assertions. None checks a query's meaning against the explicit-state checker, so the reproduced soundness defects SR-12356 FND-001 and FND-002 (an op with a guard but no successor, and the finite-trace profile) pass every test. Add at least a structural test that the stutter disjunct is present when an op has no precondition. Add one more that a `Counter` whose trace ends before `depth` keeps a model path. | qsl-eval/tests/it/smt_query.rs |
| FND-003 | low | Trace inflation. `a_nonlinear_atom_is_refused_with_its_locus` and `items_without_a_proof_query_are_refused` are tagged FR-315-AC-3, but AC-3 is the field-sort refusal. `a_bounded_eventually_is_violated_where_every_position_is_false` is tagged AC-1, which is `always[0,3]` only. The script unit tests are tagged AC-1 and AC-3 for printer internals. | qsl-eval/tests/it/smt_query.rs:523-612; qsl-eval/src/smt/script.rs:436-468 |

## Verdict

AC-1, AC-2, AC-3 and AC-5 are covered by tests that can fail. AC-4 is not, because its fixture
cannot be built under S3, which needs a spec amendment. Together with SR-12356's two high
soundness findings, the PR is not mergeable. No orphan production code was found, since every new
module serves FR-315. The facade adds nothing and has no compatibility shim, as QSL-661 requires.

## New findings (disposition pass 1)

Disposition pass 1 reviewed head `784202027` against FR-315 as STD-338 (#1289) states it.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | STD-338 added AC-15 to AC-28 to TC-900 for `encode_smt_query` itself, and none has a tagged test. Some behaviours are tested untagged (AC-16 and AC-18 regressions, AC-20 deadlock). Others have no test and do not match the code: AC-17 (the `h<i>` step print and the `query-kind` refusal), AC-24 and AC-27 (results, `field-domain`), and AC-25 (the optional-field equality print). Either implement and tag them in this PR, or defer them with a ticket. | qsl-eval/tests/it/smt_query.rs |
| FND-005 | low | The solver-free `satisfiable()` searches only Int values `0..8`. An `assert!(!sat(..))` therefore passes for any script whose models need values outside that range. That is sound for the `Int[0,3]` fixtures used today, but it is not a general unsat oracle. | qsl-eval/tests/it/smt_query.rs:436-490 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | STD-338 rewrote AC-4 over `Tally` (a range coercion under short-circuit), and no test is tagged FR-315-AC-4. The Slot test was untagged rather than replaced. |
| FND-002 | fixed | 4e7651d6e. `satisfiable()`-backed regression tests for SR-12356 FND-001 and FND-002 now fail on the old encoding. |
| FND-003 | fixed | 4e7651d6e. The misapplied AC-3 and AC-1 tags were removed from the test file. The printer unit tests keep AC-1 and AC-3 tags for canonical printing, which is acceptable. |
| FND-004 | still-open | Found in this pass. |
| FND-005 | still-open | Found in this pass. |

### Disposition pass 2

Disposition pass 2 reviewed head `15aefe760` (fix commit `f3668797c`).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f3668797c. `a_partial_construct_contributes_its_definedness_failure_at_every_position` is tagged FR-315-AC-4 and is built over `Tally`. |
| FND-004 | fixed | f3668797c. AC-15 to AC-25 and AC-27 now have tagged tests. AC-26 and AC-28 are Analysis ACs about how CG's backend reads a model, which CG owns, not this crate. |
| FND-005 | fixed | f3668797c. `assert_unsat` checks that every Int constant is ranged inside the search values before it trusts an unsat. The remaining direct `!sat` uses are over `Int[0,3]` fixtures. |

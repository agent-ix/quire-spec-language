---
id: SR-2698002
title: "gap analysis of PR2698 postcondition range facts from preconditions (FR-577)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-577-AC-1..33, FR-121-AC-37, TC-1416 and qsl-replay/src/spine/clause/tests/postcondition_range.rs; PR2698 b1fa7352c"
review_set: subset
---

## Summary

Ticket: QSL-1901. Plan completion: not assessed. All 33 FR-577 criteria and FR-121-AC-37 are tagged `#[trace("TC-1416", ...)]` in postcondition_range.rs, one test per TC-1416 step. Each test was mutated mentally. Most distinguish the right implementation from a plausible wrong one: AC-20's binder slot coincides across the two clauses, AC-24 shares the inherited operation key, AC-26 and AC-27 each take one disjunct of the family test, and AC-11, AC-12, AC-16, AC-17, AC-19 and AC-22 each catch a specific mutation. Gaps: AC-32 does not observe the refusal it names; AC-9 and AC-10 cannot tell "keep the smaller c" from "last bound wins"; AC-29 cannot fail; and AC-1 and FR-121-AC-37 are only partly asserted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-577-AC-32 test asserts only `Inconclusive`, `NoValue` and no value. Any kernel refusal anywhere in the evaluation gives the same result. The AC requires `IntegerOutOfDomain` at `pre(self.balance) - amount`, and the test checks neither the refusal kind nor its locus. Assert the refusal through `resolved_regions()` or the arm's refusal record. | qsl-replay/src/spine/clause/tests/postcondition_range.rs:779-829 |
| FND-002 | medium | AC-9 and AC-10 always put the stricter bound last: `<=` then `<`, and `CanAfford` then `Strict` in source order. A mutation that keeps the last bound instead of the smaller `c` passes both, in `difference()` and in `conjoin()`. Add the reversed order (`amount < self.balance and amount <= self.balance`; `Strict` before `CanAfford`). The AC text itself has the same weakness. | qsl-replay/src/spine/clause/tests/postcondition_range.rs:442-466; qsl-semantics/src/check/facts.rs:887-890,2129-2135 |
| FND-003 | medium | FR-577-AC-29 cannot fail. The invariant reads at `current`, and transferred precondition facts are keyed at `pre`, so an implementation that wrongly carried precondition facts into invariants still refuses at `self.balance - self.limit`. The criterion needs an invariant that reads at an observation that matches. If no such invariant exists, record the exclusion as holding by construction and drop the AC. | qsl-replay/src/spine/clause/tests/postcondition_range.rs:714-730; spec/functional/FR-577-discharge-a-postcondition-s-range-obligations-from-its-operation-s-preconditions.md:232 |
| FND-004 | low | The FR-577-AC-1 test only checks that the unit compiles. It does not inspect that `pre(self.balance) - amount` is typed `Int[0, 100]`, as AC-1 and TC-1416 step 1 state. AC-3 covers a missing coercion, so the risk is small. | qsl-replay/src/spine/clause/tests/postcondition_range.rs:341-345 |
| FND-005 | low | FR-121-AC-37 requires the region to name the unit's source. The test asserts the byte offsets only, never `region.source()`. | qsl-replay/src/spine/clause/tests/postcondition_range.rs:385-395 |
| FND-006 | low | FR-577's exclusions for "a bound used for a `+`, `*` or unary `-` node, or for `-b + a`" have no test. AC-18's guard is a literal ordering, so no difference bound is ever recorded. Applying `difference_interval` to `Add` in `IntervalWalk` passes every test. | qsl-replay/src/spine/clause/tests/postcondition_range.rs:544-563; qsl-semantics/src/check/facts.rs:1763-1769 |
| FND-007 | low | The FR-577-AC-2 test compiles through `compose`, although TC-1416 says every bank-unit compile goes through `call_site`. It compares sorted clause node keys only, not the requirement record keys that FR-104-AC-6 also requires. | qsl-replay/src/spine/clause/tests/postcondition_range.rs:347-370 |

## Scope and evidence

Planless audit of the PR diff. Tests are linked by `#[trace]` tags; a grep found all 34 criteria tagged. The focused `postcondition_range` run is recorded in the reviewer report. The precondition-shift gap (no result-typed operation) is SR-2698001 FND-003.

- FR-577-AC-1..AC-33 (spec/functional/FR-577-discharge-a-postcondition-s-range-obligations-from-its-operation-s-preconditions.md): each examined against its test.
- FR-121-AC-37 (spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md): region names the unit's source and spans the node. (examined)
- TC-1416 (spec/test-cases/TC-1416-a-postcondition-s-range-is-proved-from-its-operation-s-precondition.md): steps 1-14. (examined)

## Verdict

The tests cover the criteria they are tagged to, apart from FND-001 to FND-003. Fix FND-001 and FND-002 in the tests. FND-003 needs a spec change to AC-29.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | Two new tests are traced to criteria they do not verify. `parameter_facts_follow_the_result_slot_of_a_postcondition` is tagged FR-577-AC-1, but it checks the "parameter of the same name" keying rule over a result-typed operation. `a_bound_refines_neither_a_sum_nor_a_difference_written_backwards` is tagged FR-577-AC-31, a positive "checks" criterion, but it verifies the Behavior exclusions for `+` and `-b + a`. Neither rule has an AC of its own. Add ACs for them, or tag the tests with the FR only. | qsl-replay/src/spine/clause/tests/postcondition_range.rs:1125-1175 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The retired FR-577-AC-29 row keeps the verification "Test (qsl-semantics unit)", but `only_a_postcondition_receives_the_precondition_facts` carries no `#[trace]`, so the computed matrix shows AC-29 with no backing test. Tag the unit test `#[trace("FR-577-AC-29")]`, or make the retired row's verification none. | qsl-semantics/src/check/mod.rs:10163-10180; spec/functional/FR-577-discharge-a-postcondition-s-range-obligations-from-its-operation-s-preconditions.md:234 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | cb8db168f: the AC-32 test runs `run_clause` on the `Debited` invocation and asserts `CallRefusal::Record` with code `integer_out_of_domain`, located exactly at the bytes of `pre(self.balance) - amount`. It then asserts the replay settles `Inconclusive` with category `Refusal` and `NoValue`. |
| FND-002 | fixed | cb8db168f: `the_smaller_bound_is_kept_when_the_stricter_comes_first` (`amount < self.balance and amount <= self.balance`) and `..._across_clauses_when_the_stricter_clause_comes_first` (`Strict` before `CanAfford`) are added. A last-bound-wins mutation of `difference()` or `conjoin()` now fails one of them. |
| FND-003 | still-open | The amended AC-29 (`CanAfford { self.balance >= 50 and amount <= self.balance }`, `invariant Room ... at current { self.limit = self.balance - 50 }`) still cannot fail. `observation_of(Precondition)` is `Pre` and `observation_of(Invariant)` is `Current` (state_clause.rs:57-62). The transferred fact is keyed `self.balance@Pre`, while the invariant reads `self.balance@Current`. An implementation that wrongly passes precondition facts to invariants still refuses at `self.balance - 50`. No invariant can read at `pre`, so a unit-level AC cannot tell the two apart. Either state the invariant and constraint exclusion as holding by construction and remove AC-29, or replace it with a qsl-semantics unit test that the clause declarations for a non-postcondition kind carry an empty `preconditions` slice. |
| FND-004 | fixed | cb8db168f: the AC-1 test walks `Debited`'s body, finds the one `Coerce` over a binary `-`, and asserts its type is `Int[0, 100]`. |
| FND-005 | fixed | cb8db168f: the FR-121-AC-37 assertion checks `region.source()` (authority `agent-ix`, identity `clause-run-fixture`) as well as the byte offsets. |
| FND-006 | fixed | cb8db168f: `a_bound_refines_neither_a_sum_nor_a_difference_written_backwards` records `a <= b` and asserts that `a + b` and `b + a` are refused. Applying `difference_interval` to `Add` would narrow `a + b` to `[0, 0]` and make the test fail. `-b + a` is also refused. Unary `-` alone remains untested, which is acceptable. |
| FND-007 | fixed | cb8db168f: the AC-2 test compiles both units through `call_site` (`bank_site`) and compares the sorted clause sites and the requirement record keys. |
| FND-003 | fixed | 41b7748f9: AC-29 is retired, and FR-577 now states that invariants and state constraints receive no precondition fact by construction (they cannot read at `pre`). The loop gets every clause's facts from the extracted `clause_preconditions`, and `only_a_postcondition_receives_the_precondition_facts` asserts an empty slice for `Invariant`, `Constraint` and `Precondition`, a family member and a missing key. A mutation that hands facts to a non-postcondition kind now fails. |
| FND-008 | fixed | 41b7748f9: new ACs cover the two rules. FR-577-AC-34 (result-slot postcondition, with and without `CanAfford`) is tagged on `parameter_facts_follow_the_result_slot_of_a_postcondition`, and FR-577-AC-35 (`+`, `b + a`, `-b + a` exclusions) on `a_bound_refines_neither_a_sum_nor_a_difference_written_backwards`. FR-577-AC-31 is back on its own test (`a_function_body_guard_discharges_its_difference`). TC-1416 steps 15 and 16 are added. |

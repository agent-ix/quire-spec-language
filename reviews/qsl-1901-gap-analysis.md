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

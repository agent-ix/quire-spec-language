---
id: SR-2341002
title: 'Gap analysis of PR #2341: quantity integer power e ^ n (QSL-1656/QSL-1700), gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@026b62234; git diff origin/main...026b62234 (PR #2341): qsl-eval/tests/it/quantity_power.rs; qsl-forms/tests/it/value_forms.rs:1566-1618; qsl-semantics/src/check/lowering/model/tests.rs:1245-1301; against FR-093-AC-58, TC-1334, FR-091-OQ-15; ticket QSL-1656'
review_set: subset
date: '2026-11-04'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1334
    type: references
---

## Summary

Ticket: QSL-1656. Plan completion: not assessed. The tests tagged FR-093-AC-58 and TC-1334 were checked against each clause of the AC, against TC-1334's procedure and against the QSpec TC-187 rows U12, U13, U21 and QP-01 to QP-10, read by ID. Three mutations were run on the branch:

- Dropping the negative-exponent nonzero obligation in facts.rs was killed by `a_negative_exponent_over_a_base_not_proved_nonzero_refuses`.
- Typing the power with the base's unit instead of the compound result unit was killed by two tests.
- Swapping the r4 and r5 refusal locations (operand versus power) survived.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-093-AC-58 | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:1049 | examined |
| TC-1334 (quantity power) | spec/test-cases/TC-1334-a-quantity-integer-power-lowers-to-quantity-pow.md | examined |
| FR-091-OQ-15 | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1310 | examined |
| quantity_power.rs (6 tests) | qsl-eval/tests/it/quantity_power.rs | examined |
| value_forms power tests | qsl-forms/tests/it/value_forms.rs:1566-1618 | examined |
| lowering row test | qsl-semantics/src/check/lowering/model/tests.rs:1245-1301 | examined |
| QSpec TC-187 U12, U13, U21, QP-01..10 | quire-specification spec/test-cases/TC-187 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No refusal location is asserted. AC-58 places r1 and r2 "at the first `^`", r3 "at the first `^`", r4 "at `i`" and r5 "at the first power", and the code makes exactly that operand/power distinction (`location.child(0)` vs `location`). A mutation swapping the two survived all six tests. Fix: assert `refusal.location` for r1 to r5 (operand child vs power node). | qsl-eval/tests/it/quantity_power.rs:126-276 |
| FND-002 | medium | AC-58 and TC-1334 are tagged as covered, but parts are not exercised. (a) TC-1334 step 1, "IR's v2 reader admits each package", is not run. It is feasible today: a temporary root `it` probe using emit_checked_v2's `producer_bytes` and `read_back` over w1, w2, w3, w5, w6 and w8 was admitted by the strict reader. (b) The lowering row covers only `a ^ 2` over a parameter. w1's literal-base `[ref(2 m), ref(N)]`, w3's dimensionless `result_type` and w5's `s^1` `result_type` (QP-02, QP-03) are asserted only indirectly through equality. (c) w7's value is unmet because quantity negation is QSL-1389; no test claims it, but the AC tag reads as full coverage. Fix: add the reader read-back test and the w1/w3/w5 lowering assertions, and record w7 as deferred to QSL-1389 on the ticket. | qsl-semantics/src/check/lowering/model/tests.rs:1245; qsl-eval/tests/it/quantity_power.rs:51-78 |
| FND-003 | low | TC-1334 is a duplicate id on main. `TC-1334-a-quantity-integer-power-lowers-to-quantity-pow.md` (#2312) and `TC-1334-refuse-named-populations-in-timed-model-predicates.md` (#2294) both carry it, so this PR's `TC-1334` tags are ambiguous with `qsl-semantics/tests/it/timed_model.rs:295`. TC-1334 step 5 also asks the mapping test to carry `FR-091-AC-3`, which it does not. Fix: renumber one TC, retag, and add the FR-091-AC-3 tag. | spec/test-cases/TC-1334-a-quantity-integer-power-lowers-to-quantity-pow.md:2; qsl-forms/tests/it/value_forms.rs:1570 |
| FND-004 | low | The S1 refusal test asserts only `Code::InvalidSyntax`. AC-58 names the cause `unexpected-token`. Fix: assert the diagnostic cause too. | qsl-forms/tests/it/value_forms.rs:1609-1611 |

## Verdict

Conditional. FND-001 and FND-002 are cheap and close real regression holes.

What is sound:

- The r2 Kernel tuples are exactly U21's, not approximations. `L(i,d,e,o,w,r)` is integer_bits, decimal_digits, unit_edges, value_occurrences, work_units and result_units, with every other field 0. `budget(1)` is L(1,0,0,1,1,0) and `budget(0)` is L(1,0,0,1,0,0). The asserted `incomplete { work_units, 0, 0, 1, unit.identity-read }` is U21's record.
- The w8 limits are U13's tuple verbatim, and the asserted record is U13's.
- w1 to w6, w9, the `w1_wrong` negative control, the i128 extremes, the over-i128 refusal, the Linked r1 and r2 refusals, and the affine, non-quantity and carried refusal causes are value-asserting.
- w3 and w5 bind `d` with `let` rather than a parameter. This is equivalent for typing and value, and the check does not see through `let` for the nonzero fact, which r1b confirms.
- No test claims w7 or the reader step.

## Dispositions

| Finding | Disposition |
| --- | --- |
| FND-001 (medium, no refusal locations asserted) | Fixed. `qsl-eval/tests/it/quantity_power.rs` asserts the location of r1, r2, r3 (first `^`), r4 (`i`), r5 (the power), the `let`-bound r1b and the over-i128 refusal. Mutating r4's location to the power's makes the test fail. |
| FND-002 (medium, IR v2 reader step and rows not exercised) | Fixed. `tests/it/quantity_power_v2.rs` reads w1, w2, w3, w5, w6 and w8 back through IR's strict v2 reader and asserts each `quire.op.quantity.pow` node: binary, no law/mode/member/leaves, `[ref(base), ref(N)]` with `N` the exponent's Integer literal (a literal base for w1, w2, w6, w8), and `result_type` the compound unit (`m` squared, dimensionless for w3, `s` to the power 1 for w5, `m` to the power -1 for w6, 2^64 for w8). w7 is deferred to QSL-1389 (quantity negation, FR-093-AC-49); no test claims it. |
| FND-003 (low, TC-1334 defined twice) | Fixed. The power test case is renumbered TC-1336 in the spec, FR-093, the tests and the PR #2312 reviews; the populations TC-1334 is unchanged. |
| FND-004 (low, `unexpected-token` cause not asserted; FR-091-AC-3 tag) | Fixed. The S1 refusal test asserts the cause, and the mapping test carries the FR-091-AC-3 tag. |

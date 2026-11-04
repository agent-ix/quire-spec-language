---
id: SR-2277002
title: 'Gap analysis of PR #2277: decimal(c, s) source constructor, requirements and tests'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@79c8e075d; PR #2277; spec/functional FR-091, FR-093, FR-104; spec/test-cases TC-1232; QSpec FR-140 and TC-185; ticket QSL-1619'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1619. Plan completion: not assessed. The PR's tests prove the D24, D26, D27 and D28 vectors and the checker rules by mutation (a non-member literal let through, a dropped nonzero discharge and a dropped literal hint each fail tests). They do not pin the wire spelling, the zero-coefficient divisor, FR-091-AC-58, or most of FR-104-AC-13 and FR-104-AC-16. Two spec defects: FR-093-AC-39 to AC-42 do not exist although TC-1232 and FR-091 cite them, and the QSpec D24, D26 and D27 work_units figures omit the source call's FunctionCall charge.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-091-AC-57 | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1165 | examined |
| FR-091-AC-58 | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:1166 | examined |
| FR-104-AC-13 | spec/functional/FR-104-check-state-clauses.md:391 | examined |
| FR-104-AC-16 | spec/functional/FR-104-check-state-clauses.md:394 | examined |
| TC-1232 | spec/test-cases/TC-1232-a-decimal-constructor-in-source-checks-lowers-and-evaluates.md | examined |
| FR-093 (AC-39 to AC-42) | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md | examined |
| QSpec FR-140-AC-7, FR-140-AC-8, TC-185 D24 D26 D27 D28 | quire-specification@ad180f162 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test reads the lowered or emitted decimal literal spelling; spelling the text with width scale (".005" for decimal(5,3)) or always as an integer survives every test in qsl-eval, qsl-semantics lib and qsl-package. TC-1232 step 3 values ("1.25", "0.05", "10", "0.00", "-0.5") are asserted nowhere. | qsl-semantics/src/check/node_key/mod.rs:1557-1577 |
| FND-002 | medium | The zero-coefficient and scale cases of the nonzero literal divisor are untested: forcing the discharge for a zero coefficient survives the whole decimal_source_forms suite; no test divides by decimal(0, s) or by a parameter beside a literal. | qsl-semantics/src/check/facts.rs:1140-1165 |
| FND-003 | medium | FR-104-AC-13 and FR-104-AC-16 are mostly untested: only e1 and a variant of e2 appear; e3, e4, e5 and e6 and the `a + decimal(1, 2) = b` example have no test, and no test carries a FR-104-AC-16 or TC-1084 trace tag. | qsl-eval/tests/it/decimal_source_forms.rs |
| FND-004 | medium | FR-091-AC-58 (malformed spellings) is neither implemented for decimal(-0, 2) and decimal(+1, 2) nor tested for any spelling; S1 refuses the others, but no test in the repository names a malformed decimal constructor. | qsl-cst, qsl-forms |
| FND-005 | medium | Spec defect: FR-093 has no AC-39 to AC-42 and no Decimal constructors section, although TC-1232 and FR-091 cite them. The rules that a decimal(c, s) lowers to a decimal literal node spelled with exactly scale fractional digits, and that a nonzero literal divisor discharges the nonzero obligation, are stated only in TC-1232's prose and the FR-322 literal grammar; FR-093 line 196 states the opposite default for quantities and FR-094 line 340 says no guard proves a divisor that is not a literal nonzero. STD-424 (PR #1379) closed unmerged. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md |
| FND-006 | low | QSpec TC-185 D24, D26 and D27 state source vectors under work_units 5, 4 and 5, which the arithmetic charges alone satisfy; the source call also charges FunctionCall first (one work unit), so the vector needs 6, 5 and 6 and its short figure one higher. The PR restates the figures with a note in its test; the QSpec rows should say the figures are arithmetic-only or add the call charge. | quire-specification spec/test-cases/TC-185 |

## Verdict

Not mergeable on gap grounds alone for FND-001 to FND-004 only if the team holds the ACs in scope; FND-005 is a spec fix owned by the spec lane. In the traces of the PR's tests the charge list begins with FunctionCall for D24, D26 and D27 (qsl-eval/tests/it/decimal_source_forms.rs), which confirms FND-006: a limit of the vector's work_units figure stops at the call. This is a vector note, not an evaluator defect.

Stale tests: the qsl-forms value_forms case that expected decimal(1, 2) to be unrepresented and the qsl-package protocol_captures case that expected the same were restated and are correct for FR-091-AC-57; no test was deleted without a replacement. Coverage tags: the three new qsl-eval vectors trace QSpec-TC-185 and QSpec-FR-140 criteria; the qsl-forms test traces FR-091-AC-57 and TC-1232.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | FR-104-AC-13's `function d using v(a: P, b: P): P pure { a / b }` is stated to check and evaluate, but the checker refuses it undefined_expression/unproved-nonzero (probed; P admits zero and nothing proves b nonzero), and no test covers the AC's division, `convert<W>`, `-a = b` or `a / b = a` with `b: R` clauses although FR-104-AC-13 is claimed by tests. Either the AC example lacks a nonzero premise (spec lane) or the checker is wrong. | spec/functional/FR-104-check-state-clauses.md:391 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | The merged FR-093 ACs are only partly asserted. FR-093-AC-39: spellings c1..c5 only (no c6 "1.250"), no shared-node assertion for two functions over decimal(125, 2), no lowered quire.op.decimal.negate or quire.op.decimal.add node check, no IR v2 reader admission. FR-093-AC-40 and FR-093-AC-42 have no trace-tagged test (AC-40 lacks the Boolean result and coefficient 10^41 refusals and the admitted decimal(1200, 3) and decimal(0, 7) cases; AC-42 lacks the convert node and literal "1.5" at R's type node and the Float64 to Decimal mismatch). FR-093-AC-41 asserts checking and the value but not the quire.op.decimal.div node. FR-093-AC-53 is tagged but no test refuses the unit declaration `unit c : Length = decimal(1, 5000) * m;` by s3.decimal_scale. | qsl-eval/tests/it/decimal_source_forms.rs; qsl-semantics/src/check/lowering/tests/rows.rs:819 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | rows.rs asserts the exact spellings "1.25", "0.05", "10", "0.00", "-0.5", "1" and the 65534/65535-digit zero spellings; no spelling mutation was run, the assertions are value-exact. |
| FND-002 | fixed | only_a_nonzero_literal_divisor_discharges_the_nonzero_obligation covers decimal(4, 0), decimal(400, 2), decimal(0, 2), decimal(0, 0), a parameter, p / q and an if of two literals; mutation forcing the discharge for a zero coefficient fails it. |
| FND-003 | fixed | the_six_d25_expressions_check_or_refuse_as_the_criterion_states covers e1 to e6 and a_literal_beside_a_typed_operand_evaluates_at_the_peer_type covers `a + decimal(1, 2) = b` (checked and evaluated); both carry FR-104-AC-16/FR-104-AC-13 and TC-1084 trace tags. Other AC-13 clauses are FND-007. |
| FND-004 | fixed | qsl-cst tests/it/decimal_constructor.rs names the malformed and well-formed constructors and the reserved name, tagged FR-091-AC-58 and TC-1232. |
| FND-005 | deferred | Spec defect owned by the spec lane (titan:QSL:plan): FR-093 still has no AC-38 to AC-42 or Decimal constructors section; the PR changes no spec file, so a code fix round cannot resolve it. The lead must record a spec ticket before TC-1232 is called covered. |
| FND-006 | deferred | QSpec TC-185 vector wording (work_units figures one low for a source call) lives in agent-ix/quire-specification, outside this PR; the PR's test records the call charge in a comment. Low severity, no behavior wrong. |
| FND-005 | fixed | FR-093 now has the Decimal constructors section and AC-38 to AC-42, AC-45, AC-52 to AC-57 (QSL PR #2293). Coverage of those ACs is FND-008. |
| FND-007 | fixed | FR-104-AC-13 now uses `a / decimal(8, 2)` and states the parameter divisor refusal; every clause is asserted: a_quotient_is_typed_by_its_operands... (d, e, d2, f, g, h), the_conversion_negation_and_mismatch_forms_of_the_criterion (convert<W>, convert<W>(a + b), -a = b, a / b with b: R), sums_differences_and_products... and a_literal_beside_a_typed_operand_evaluates_at_the_peer_type, all tagged FR-104-AC-13 and TC-1084. |

## Disposition verdict (pass 1)

Fixed: FND-001 to FND-004. Deferred to the spec lane: FND-005, FND-006. New: FND-007 (spec or checker, AC-13's division example). Gate and mutation evidence is in SR-2277001's Disposition verdict.

## Disposition verdict (pass 2)

Tagged value-asserting coverage per merged AC at head 0b062ccd5: AC-38, AC-45, AC-52, AC-54, AC-55 and AC-56 are asserted as stated; AC-41 (value, not node shape), AC-53 (no unit-declaration case) and AC-57 (m1 to m4 as stated; the Q case asserts (-500, 2), not the AC's (-5, 0), the evaluator follows FR-140 and the AC needs the spec fix, see judgment (c) in SR-2277001's pass 2 verdict) are partial; AC-39 is partial; AC-40 and AC-42 are untagged and partial. FND-008 routes these to the coder. Gate, mutation and judgment evidence is in SR-2277001's pass 2 verdict.

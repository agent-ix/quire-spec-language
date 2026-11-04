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

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | Two assertions are weaker than their AC. The FR-093-AC-53 test checks `format!("{:?}", error.cause)` for "DecimalScaleLimit", "4096", "5000" and "S3DecimalScale" although AssemblyCause::DecimalScaleLimit carries a typed LimitExceeded (setting, bound, actual) that the existing assembly tests read through its accessors. The FR-093-AC-42 test asserts the convert node's result_type differs from R's node (assert_ne) instead of equalling P's type node. | qsl-package/src/emit/tests/decimal_constructors.rs; qsl-eval/tests/it/decimal_source_forms.rs |

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | medium | The D24 and D27 value tests omit the vectors' loss records. QSpec TC-185 D24 expects `{ exact_numerator: 2, exact_denominator: 3, rounded_coefficient: 67, rounded_scale: 2, mode: nearest-even }` and D27 `{ exact_numerator: 6, exact_denominator: 35, rounded_coefficient: 2, rounded_scale: 1, mode: nearest-even }`; D26 expects no loss in both modes. `CheckedPackage::call` returns `evaluation.losses` (read in evaluation_ignores_display_strings.rs), but `call()` in decimal_source_forms.rs drops it and no test reads a loss, so a wrong or missing loss record passes. Fix: keep `evaluation.losses` in `Run` and assert the one decimal loss's fields for D24 and D27 and an empty list for D26. | qsl-eval/tests/it/decimal_source_forms.rs:100-128, 203-259, 290-333, 340-376 |
| FND-011 | medium | FR-093-AC-56 (and QSpec TC-185 D24's last case) requires the `exact` conversion to refuse `inexact_decimal`/`nonzero-discarded-digit`; the test tagged FR-093-AC-56 asserts only `matches!(exact.outcome, Outcome::Refused(_))` plus the charge points, so any refusal code passes. Fix: match the refusal record and assert its code and cause. | qsl-eval/tests/it/decimal_source_forms.rs:240-258 |
| FND-012 | low | Three evaluation refusals are asserted by `format!("{stop:?}").contains("Domain")`: FR-093-AC-57 m3 (`-decimal(-5, 0)` at N, spec `decimal_out_of_domain`/`outside-domain`), FR-104-AC-13 d2 and `convert<W>(a + b)`. The substring also matches any other `*Domain*` refusal (integer, rational, division). Fix: assert the refusal's code and cause. | qsl-eval/tests/it/decimal_source_forms.rs:643, 809, 840 |

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
| FND-008 | fixed | New tests assert every part of the merged ACs: qsl-package emit/tests/decimal_constructors.rs reads the emitted package and IR v2 admission (read_checked_package_v2) for AC-39 (six distinct literals "-0.5", "0.00", "0.05", "1.25", "1.250", "10"; c1 and c1b share one node; every literal typed at Q's node; one quire.op.decimal.negate node over the literal "5"; one quire.op.decimal.add node over [ref(p), ref("1.25")] with the literal's type), AC-40 (literals "1.00", "1.200", "0.0000000"), AC-41 (two quire.op.decimal.div nodes, x1 and x3 sharing, over [ref(a), the literal] typed W) and AC-42 (literal "1.5" at R's node, the sum typed R, one convert node); qsl-eval asserts AC-39 evaluation (125/2 and 225/2), AC-40 a2..a7 refusals and the admitted members, the AC-42 Float64 refusal and the AC-53 unit-declaration refusal. AC-40, AC-41 and AC-42 now carry trace tags. AC-57's Q case (-500, 2) now matches the merged AC text. Mutations: literal spelled with one extra zero fails 2 emitter tests; negate node renamed fails a_negation_and_a_sum...; non-member admitted fails 5 qsl-eval tests. Residual weaknesses are FND-009 (low).
| FND-009 | fixed | Commit 02d74a5bf: the AC-53 test reads the typed `AssemblyCause::DecimalScaleLimit` fields (`configured_bound()` 4096, `actual()` 5000, `setting()` S3DecimalScale), and the AC-42 emitter test pins P's type node through a `decimal(150, 2)` literal at P and asserts `convert["result_type"] == p_type`. |
| FND-010 | fixed | Fix-forward PR (#2277 had merged): `Run` keeps `evaluation.losses`; D24 asserts the one decimal loss `{2/3, 67, 2, nearest-even}`, D27 `{6/35, 2, 1, nearest-even}`, D26 asserts an empty list in both modes, each checked against QSpec TC-185 D24/D26/D27 at origin/main. Mutation: D27 denominator 35 changed to 36 fails d27. |
| FND-011 | fixed | The exact conversion refusal is asserted as `inexact_decimal` / `nonzero-discarded-digit` (`Refusal::code`/`cause`). Mutation: cause changed to another string fails d24. |
| FND-012 | fixed | The three `contains("Domain")` substring checks assert `decimal_out_of_domain` / `outside-domain`. Mutation: cause changed to another string fails the negation, quotient and conversion tests. |

## Disposition verdict (pass 1)

Fixed: FND-001 to FND-004. Deferred to the spec lane: FND-005, FND-006. New: FND-007 (spec or checker, AC-13's division example). Gate and mutation evidence is in SR-2277001's Disposition verdict.

## Disposition verdict (pass 2)

Tagged value-asserting coverage per merged AC at head 0b062ccd5: AC-38, AC-45, AC-52, AC-54, AC-55 and AC-56 are asserted as stated; AC-41 (value, not node shape), AC-53 (no unit-declaration case) and AC-57 (m1 to m4 as stated; the Q case asserts (-500, 2), not the AC's (-5, 0), the evaluator follows FR-140 and the AC needs the spec fix, see judgment (c) in SR-2277001's pass 2 verdict) are partial; AC-39 is partial; AC-40 and AC-42 are untagged and partial. FND-008 routes these to the coder. Gate, mutation and judgment evidence is in SR-2277001's pass 2 verdict.

## Disposition verdict (pass 3)

FND-008 is fixed on head 83d666c66; FND-009 (low) is new and does not block. Spec point: FR-093-AC-42's `convert<Decimal[0, 100; 0, 3]>(a)` (and the same text in FR-397-AC-18) omits the rounding mode, but S1's Decimal type form is `Decimal[lo, hi; smin, smax; mode]` with the mode required (qsl-cst grammar.rs, the Decimal production ends `; RoundingMode ]`), so that spelling refuses `invalid_syntax` at S1 before it can be `ill_typed`/`type-mismatch`. The fix: write `Decimal[0, 100; 0, 3; nearest-even]` in FR-093-AC-42 and FR-397-AC-18, and change FR-140's "`mode` is the optional rounding spelling" to a required one (or make S1 admit the omission). Mergeable. Gate and mutation evidence is in SR-2277001's pass 3 verdict.

## Disposition verdict (pass 4)

Head 02d74a5bf, trial merge with origin/main 623cdcba6; QSpec read at a detached origin/main worktree (860e0890). FND-009 is fixed. FND-005 and FND-007 are fixed (pass 2), FND-006 stays deferred (QSpec vector wording). New: FND-010 and FND-011 (medium, test assertions weaker than the vector and AC) and FND-012 (low). None is a wrong production result; they are coverage gaps on the vectors this ticket exists to prove.

Vectors re-derived by hand from QSpec FR-140 and TC-185. D24: 2/3 into `Decimal[0,100;2,2]` is (2,0)/(3,0) at target scale 2, dividend side shift 2, N = 200, D = 3, `sbits(2,2) = bits(2) + bits(100) = 2 + 7 = 9`, `sdigits = 3`; 200/3 = 66.67 rounds nearest-even to (67,2), retain bits(67) = 7, digits 2; five charges plus the call's FunctionCall; `exact` stops after `decimal.arithmetic`. The test asserts every charge size and the result. D26: (100,2) x (3,0) = 300 at scale 2, retained at scale 0 as (3,0) with discarded `00`, no rounding charge; operands bits(100) = 7, arithmetic 7 + 2 = 9, digits 3 + 1 = 4, retain bits(3) = 2; `work_units` 4 + 1 stops at result-retain, `integer_bits` 8 stops at arithmetic with consumed 7 and next 9. All asserted. D27: (1200,3)/(7,0) into smax 1: divisor shift 3 - 0 - 1 = 2, D = 700, N = 1200, 12/7 = 1.714 rounds to (2,1); exact value 1.2/7 = 6/35; operands bits(1200) = 11, digits 4, scale_expansion 2; `scale_expansion: 1` stops at the expansion needing 2. Result and stop asserted, loss record not (FND-010). D28: 1.50 has membership scale 1 > smax 0 and 1.234 scale 3 > smax 1, both `ill_typed`/`type-mismatch` before any charge; (100,2) x (3,0) admitted. Asserted. The vectors' `work_units` figures are arithmetic-only; the tests add one for the call, as FND-006 records.

Focused tests on the trial merge (QSPEC_DIR set, jobs 4): qsl-eval `--test it` decimal_source_forms, equality_matrix, total_functions 91 passed; qsl-cst decimal_constructor 4 passed; qsl-forms value_forms 32 passed; qsl-foundation lib 60 passed; qsl-semantics lib (rows, refusal, checked_dispatch) 25 passed; qsl-package lib (decimal_constructors, protocol_captures) 17 passed; qsl-eval lib family_contract_tests 6 passed; 0 failed in each. Verdict: CONDITIONAL. Mergeable once FND-010 and FND-011 are fixed in this PR (a few lines of test each); FND-012 may ride along.

## Disposition verdict (pass 5)

FND-010, FND-011 and FND-012 are fixed in a fix-forward PR off origin/main (#2277 merged before the fixes landed); tests only, no production change. Focused: qsl-eval `--test it` decimal_source_forms 21 passed, 0 failed.

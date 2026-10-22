---
id: SR-12300
title: "Gap analysis of PR #1178: FR-397 acceptance criteria and behaviour against TC-1091 tests and the IEEE selection code"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4303b9935336fb691a1f9a0a156f4d8f31515fe1; PR #1178 (git diff origin/main...4303b9935): FR-397-AC-1 to FR-397-AC-9 and FR-397 Behavior against qsl-replay/tests/it/compile_ieee_profile.rs, the TC-917 trace, and qsl-semantics/qsl-package source; ticket QSL-948 acceptance list"
review_set: subset
date: "2026-10-22"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-397
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: references
---

## Summary

Ticket: QSL-948. PR: quire-spec-language#1178, head 4303b9935. Method: planless gap analysis, which reports "Plan completion: not assessed". The analysis covers FR-397's nine ACs and its Behavior clauses. They are checked against the TC-1091 tests this PR adds, the TC-917 trace it extends, and the code. The PR's own gap analysis of the spec, SR-13722 in `reviews/qsl-948-gap-analysis.md` from #1169, stays as it is. This file is a separate artifact for the code PR.

Every AC has a tagged test that asserts it. For AC-1 to AC-4, the test fails with the source change reverted (mutation M1). AC-5 to AC-8 are refusal ACs whose behaviour already held on main, and their tests pin it. AC-9 is TC-917's existing producer test, and this PR adds the trace. Ticket acceptance:

- A float add compiles and strict IR admits it: met (AC-1, AC-4).
- Float-to-rational lowers to `to_rational` and strict IR admits it: met (AC-3, AC-4).
- The comparison question is settled in the spec: met by FR-397 in #1169, which specifies the refusal. AC-6's tests pin it.

## Scope

| Unit | Path | Role | Test |
| --- | --- | --- | --- |
| FR-397-AC-1 | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md | examined | both_float_widths_select_one_profile_and_emit_arithmetic_law_and_rounding |
| FR-397-AC-2 | same | examined | declared_floats_select_the_profile_even_without_an_ieee_application |
| FR-397-AC-3 | same | examined | float_to_rational_is_an_ieee_operation_with_the_selected_law; rational_comparison_evaluates_float_conversion_as_a_separate_operation |
| FR-397-AC-4 | same | examined | strict_ir_reader_admits_the_genuine_float_source_packages |
| FR-397-AC-5 | same | examined | ieee_profile_is_not_header_selectable_with_or_without_float_functions |
| FR-397-AC-6 | same | examined | float_comparisons_and_mixed_width_arithmetic_refuse_at_checking |
| FR-397-AC-7 | same | examined | reserved_ieee_intrinsic_calls_refuse_with_the_named_missing_identity |
| FR-397-AC-8 | same | examined | conversion_into_a_float_is_never_admitted |
| FR-397-AC-9 | same | examined | invariants.rs both_ieee_profile_checks_and_conversion_operand_check_are_real_producers |
| FR-397 Behavior: comparison and mixed-type refusals | same | examined | probes only |
| FR-110-AC-3 | spec/functional/FR-110-resolve-header-profile-selections-at-e3.md | examined | spine::tests::a_header_profile_resolves_by_identity_against_the_root_row |
| TC-1091 | spec/test-cases/TC-1091-a-package-that-declares-or-evaluates-a-float-selects-the-ieee-profile.md | context_only | - |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-397 Behavior says the checker refuses "operands of unlike widths, or a float mixed with an exact type, as `ill_typed`/`type-mismatch`". The comparison bullet says `=`/`<`/... "over `Float32` or `Float64` operands" refuse `operator-ineligible`. For a comparison with mixed operands, both bullets apply. Measured: `a = r` (Float64 against Rational), `r = a`, and `a = b` (Float32 against Float64) all refuse `operator-ineligible`. Arithmetic with mixed operands (`a + r`, `a + i`, Float32 + Float64) refuses `type-mismatch`. The code's choice is coherent, but the spec does not say which code wins for a mixed comparison. Two implementers could read it differently. FR-397's Comparison clause should state that the operator refusal applies when either operand is a float. Spec-side, non-blocking. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md (Behavior, Comparison) |
| FND-002 | low | A float mixed with an exact type in arithmetic is a FR-397 Behavior clause, and no test pins it. AC-6 tests only the Float32 + Float64 width mix. Measured: `a + r` (Float64 + Rational) and `a + i` (Float64 + Int) refuse `type-mismatch`, but nothing would catch a change to that. | qsl-replay/tests/it/compile_ieee_profile.rs:202-206 |

## Verdict

Mergeable. There is no major finding. Every FR-397 AC is covered by a test that asserts it. For AC-1 to AC-4, which this PR's code enables, the oracles fail with the change reverted. The selection rule meets FR-397's trigger and selection clauses:

- The trigger is raised by a declared float (parameter, result, field, alias, nested `Option` or collection) and by every IEEE application and conversion.
- An integer-only package selects no row.
- The law of each IEEE node is the selected row.

The strict V2 reader admits the AC-4 packages. The header still cannot select the row (FR-110-AC-3, AC-5).

Plan completion: not assessed.

## Dispositions

Author fix round; the original review findings and verdict above are preserved.

| Finding | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | Fixed | FR-397 Comparison explicitly applies `operator-ineligible` when either operand is a float, including unlike widths and exact types. A separate arithmetic clause specifies `type-mismatch`. AC-6 and TC-1091 describe both cases; comparison tests assert the refusal and operator region in both operand orders. |
| FND-002 | Fixed | The AC-6 mixed-arithmetic test covers `+`, `-`, `*` and `/` for both float widths paired with `Int` and `Rational` in both operand orders, as well as unlike float widths. Every case must refuse `ill_typed`/`type-mismatch`. |

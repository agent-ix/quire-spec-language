---
id: SR-202611021389
title: 'Gap analysis of QSL PR #2088: FR-397 AC-7/12/13/15/16/24 against TC-193 rows and tests (QSL-1388)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@b3c776659; git diff origin/main...HEAD (PR #2088): FR-397-AC-7, AC-12, AC-13, AC-15, AC-16, AC-24 and the tests under qsl-eval/tests/it/float_callables.rs, qsl-replay/tests/it/compile_ieee_profile.rs, qsl-eval/tests/it/equality_matrix.rs; ticket QSL-1388'
review_set: subset
date: '2026-11-02'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-397
  type: reviews
---

## Summary

Ticket: QSL-1388. PR: quire-spec-language#2088, head b3c776659. Method: gap analysis by hand; the quoin matrix and cargo were not run. Plan completion: not assessed.

Every listed TC-193 row (F01, F02, F02b, F04b, F05, F11, F20, F34) has a test tagged TC-1091 and FR-397-AC-24: float_callables.rs covers F01, F02, F02b, F04b, F05, F11 and F34 (via the AC-13 vector table at both widths) with asserted values and flags; compile_ieee_profile.rs covers F20's seven refusals. AC-7 and AC-12 lowering, AC-13 vectors, AC-15 fma lowering and F11/AC-15 fma values, and the AC-16 arity, type and cbrt refusals and totalOrder/bitIdentical vectors are each asserted, not just run. No test passes vacuously except the call-order check recorded in SR-202611021388 FND-005.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-397-AC-7 | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:256 | examined |
| FR-397-AC-12 | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:261 | examined |
| FR-397-AC-13 | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:262 | examined |
| FR-397-AC-15 | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:264 | examined |
| FR-397-AC-16 | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:265 | examined |
| FR-397-AC-24 | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:273 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-397-AC-16 names two cases with no test in this PR: the zero-argument sqrt() refusal, and a supplied library exporting quire::value::ieee::sqrt making the call refuse invalid_package/conflicting-definition. Only the admission-level reserved-identity check (qsl-semantics/tests/it/ieee_profiles.rs:748-776) exists, so the call-site refusal for fma, numericEqual, totalOrder and bitIdentical is untested. The coder routed the first to the spec lane; the second is not mentioned. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:265; qsl-replay/tests/it/compile_ieee_profile.rs:367-430 |

## Verdict

Not clean, with one low gap. All other listed ACs and TC-193 rows are asserted by tagged tests; no source code was found without an owning requirement (the OrderedKind::Float lowering arm is an unreachable fault and the SMT refusal arms are sound incomplete outcomes).

---
id: SR-1957032
title: "QSL-1373 PR 1972 spec-review-evidence"
type: SpecReview
analysis: evidence
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1373; origin/main...1323427827cfcef6c27e670c953ca2c099a12b8e; four Rust files, FR-314/315, TC-893/900 and spec/reviews/qsl-1373"
---

## Summary

Ticket: QSL-1373. Frozen head: 1323427827cfcef6c27e670c953ca2c099a12b8e. Catalog-backed evidence analysis unavailable: prior quoin advise attempt did not complete; CLI has only repository-wide scope, and dispatch excludes repeated broad hangs. Authored Test methods are consistent with executable acceptance behavior by manual judgement only, not deterministic advisor recommendation. No catalog conformance PASS claimed. Missing test oracles are recorded separately in gap-analysis.

## Verdict

CONDITIONAL. No additional defects found within the stated evidence limits.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Coverage

- FR-314-AC-18 (examined), spec/functional/FR-314-check-an-smt-proof-certificate.md: "On the row-1 refinement proof, command `t28` has no premises and concludes `(= (* 1 (- (+ 1 x) y)) (* 1 (- x (+ (- 1) y))))` with rule `poly_simp`; the checker accepts it by exact Int-polynomial normalization. A separate `poly_simp` step with no premises and conclusion `(= (* x y) (* y x))` is also accepted, exercising a nonlinear integer polynomial. Command `t29` has the single premise `t28` and concludes `(= (= (+ 1 x) y) (= x (+ (- 1) y)))` with rule `poly_simp_rel`; the checker accepts the equality relation at positive exact scale 1. The false equality `(= (* 2 x) (+ (* 2 x) 1))`, a non-eq"
- FR-314-AC-19 (examined), spec/functional/FR-314-check-an-smt-proof-certificate.md: "Over AC-18's polynomial certificate, when `max_certificate_edges` is one less than the number of command, premise/discharge and polynomial-normalization charges required, the checker returns `Stopped(LimitReached{limit: MaxCertificateEdges, setting: certificate_check.max_certificate_edges})`, never `Verified`, and the item settles `incomplete`; with the full required budget it returns `Verified`."
- FR-314-AC-20 (examined), spec/functional/FR-314-check-an-smt-proof-certificate.md: "With otherwise well-formed proof commands, `hole`, `lia_generic` and `rare_rewrite` each return `Unverifiable` at their command, including a `rare_rewrite` whose argument names `bool-double-not-elim`; none returns `Verified`."
- FR-315-AC-57 (examined), spec/functional/FR-315-encode-the-smt-lib-transition-relation.md: "The two Int-sorted linear terms from FR-314-AC-18's `t28`, `(* 1 (- (+ 1 x) y))` and `(* 1 (- x (+ (- 1) y)))`, each normalize to `(+ x (- y) 1)` and therefore print byte-for-byte equally. The normal form distributes only literal scaling over Int addition and subtraction, combines exact coefficients of equal Int atoms, and does not multiply two nonconstant terms; a product of two nonconstant terms is refused `out-of-fragment`."
- FR-314 (examined), spec/functional/FR-314-check-an-smt-proof-certificate.md: "Both polynomial checkers SHALL charge `max_certificate_edges` for each\n  monomial generated and each coefficient combination performed during\n  normalization, before doing that work."
- FR-315 (examined), spec/functional/FR-315-encode-the-smt-lib-transition-relation.md: "The normalizer SHALL distribute literal scaling\n   over addition and subtraction, collect coefficients of equal atoms using\n   exact integer arithmetic, remove zero coefficients, and order atoms by\n   their canonical SMT-LIB term bytes."

TC-893 and TC-900 fixture/procedure/expected-result sections examined. All four changed Rust files and the four existing spec review files examined; context-only: existing FR-314 checked-rule and FR-315 normalization contracts. No applicable AssuranceProfile. No aggregate gate or source/branch edits. Plan completion: not assessed.


## Disposition pass 1 scope and evidence

Frozen head: 1c4c605c8fcf759660667eb918b72d5a3c79d4bd. Static review of fix diff 1323427..1c4c605 and all original findings, plus the new whitelist checker. Changed FR-314/315 and TC-893/900 validate: 4/4 grammar-clean with zero grammar findings. No Cargo or aggregate gate run; prior /tmp target and -j2 claims remain unaccepted as coordinated gate evidence. The existing seven SR-1957026 through SR-1957032 files are absent from the reviewed branch; coder custody and commitment of these latest copies under reviews/ remains required. No proof of a false identity being accepted was found in the examined whitelist implementations. Matrix/advisor limitations from the initial pass remain; no whole-repository assurance PASS is claimed.

Examined scope for this round:

- FR-314-AC-19 (examined), spec/functional/FR-314-check-an-smt-proof-certificate.md: "Over AC-18's polynomial certificate, when `max_certificate_edges` is one less than the number of command, premise/discharge and polynomial-normalization charges required, the checker returns `Stopped(LimitReached{limit: MaxCertificateEdges, setting: certificate_check.max_certificate_edges})`, never `Verified`, and the item settles `incomplete`; with the full required budget it returns `Verified`."
- FR-315-AC-57 (examined), spec/functional/FR-315-encode-the-smt-lib-transition-relation.md: "The two Int-sorted linear terms from FR-314-AC-18's `t28`, `(* 1 (- (+ 1 x) y))` and `(* 1 (- x (+ (- 1) y)))`, each normalize to `(+ x (- y) 1)` and therefore print byte-for-byte equally. The normal form distributes only literal scaling over Int addition and subtraction, combines exact coefficients of equal Int atoms, and does not multiply two nonconstant terms; a product of two nonconstant terms is refused `out-of-fragment`."
- FR-314 (examined), spec/functional/FR-314-check-an-smt-proof-certificate.md: "Both polynomial checkers SHALL charge `max_certificate_edges` for each\n  monomial generated and each coefficient combination performed during\n  normalization, before doing that work."
- FR-315 (examined), spec/functional/FR-315-encode-the-smt-lib-transition-relation.md: "The normalizer SHALL distribute literal scaling\n   over addition and subtraction, collect coefficients of equal atoms using\n   exact integer arithmetic, remove zero coefficients, and order atoms by\n   their canonical SMT-LIB term bytes."
- FR-314-AC-18 (examined), spec/functional/FR-314-check-an-smt-proof-certificate.md: "On the row-1 refinement proof, a premise-free `poly_simp` equality is accepted exactly when both sides have equal normalized Int polynomials, including nonlinear monomials and zero by literal or cancellation. For `poly_simp_rel`, the sole premise must be such a polynomial equality; relation differences are normalized in source operand order, with equality-atom differences identified up to sign because equality is symmetric, and the conclusion must preserve the same relation operator at a positive exact rational scale. Thus the target row-1 equality-relation steps with swapped equality operands"
- FR-314-AC-20 (examined), spec/functional/FR-314-check-an-smt-proof-certificate.md: "With otherwise well-formed proof commands, `hole`, `lia_generic` and `rare_rewrite` with an unknown or unsupported rewrite name return `Unverifiable` at their command; none returns `Verified`."
- FR-314-AC-21 (examined), spec/functional/FR-314-check-an-smt-proof-certificate.md: "The checker locally checks only the named `rare_rewrite` instances `bool-double-not-elim`, `arith-elim-lt`, `arith-elim-leq`, `arith-leq-norm` and `eq-refl`, by exact equality with their respective Boolean or integer relation rewrites; the arithmetic rewrites require Int-sorted operands, and `arith-leq-norm` additionally requires an integer-literal bound. It locally checks `evaluate` only for exact Int-literal expressions built from `+`, `-` and `*`, and `aci_simp` only for Boolean `and`/`or` flattening, commutation, association, idempotence and true/false identities. Each supported instance i"

Current round verdict: no additional finding for this method. Original clean placeholder remains excluded from findings and dispositions. Evidence-advisor unavailable status remains unchanged where applicable.

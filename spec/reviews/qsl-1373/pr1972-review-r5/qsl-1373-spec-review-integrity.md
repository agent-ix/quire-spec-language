---
id: SR-1957029
title: "QSL-1373 PR 1972 spec-review-integrity"
type: SpecReview
analysis: integrity
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1373; origin/main...1323427827cfcef6c27e670c953ca2c099a12b8e; four Rust files, FR-314/315, TC-893/900 and spec/reviews/qsl-1373"
---

## Summary

Ticket: QSL-1373. Frozen head: 1323427827cfcef6c27e670c953ca2c099a12b8e. Checked atomicity, observable outcomes, requirement/test-case consistency and existing normal-form obligations. No applicable AssuranceProfile. See conflict between unchanged rule 7 and new rule 9; this directly explains the old unit test regression.

## Verdict

CONDITIONAL. Resolve findings inside PR 1972 before merge.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Normal-form rule 7 requires the two-term sum t + (-u) to print (- t u), while new rule 9 requires multiple remaining terms to print one + with coefficient -1 as unary negation, yielding (+ t (- u)). No precedence is declared; the existing test and new implementation choose opposite results. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:958 | correct-requirement-no-evidence |

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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | medium | FR-314 Behavior still says every rule outside the listed polynomial/base set is unchecked and explicitly says rare_rewrite remains unchecked even with a named rewrite. New AC-21 requires five named rare_rewrite instances plus evaluate and aci_simp to be accepted. The same valid named command is therefore both required to be Unverifiable and accepted. Update the normative checked-rule contract and its work accounting to match the intended whitelist. | spec/functional/FR-314-check-an-smt-proof-certificate.md:384 | wrong-requirement |
| FND-003 | medium | The polynomial Behavior still requires a semantically valid negative-scale equality-preserving step to return Unverifiable. Revised AC-18 and the new canonical_equality_difference instead accept it after sign canonicalization. For (= (= x y) (= (- x) (- y))), the Behavior and AC-18 mandate opposite outcomes. Reconcile the normative relation rule with the revised acceptance criterion. | spec/functional/FR-314-check-an-smt-proof-certificate.md:404 | wrong-requirement |

## Dispositions

Round 1 at 1c4c605c8fcf759660667eb918b72d5a3c79d4bd. Original finding text is preserved. Fixed means the stated defect was verified corrected by source inspection; focused test execution remains owned by the coordinated origin-coder lane.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | New rule 9 grants subtraction precedence only for atom coefficients +1 and -1. Rule 7 still demands subtraction for any two-term t + (-u), including (-x) + (-y) or (2*x) + (-y). Those cases still have opposite mandated + versus - forms. Reconcile the general rule or explicitly state precedence for every coefficient pattern. |
| FND-002 | still-open | New disposition-pass-1 finding; no subsequent fix is present in frozen head 1c4c605. |
| FND-003 | still-open | New disposition-pass-1 finding; no subsequent fix is present in frozen head 1c4c605. |

Current round verdict: NOT MERGEABLE: still-open findings require another fix round.


### Disposition round 3

Exact reviewed head: 32b95d600f80890be44198501faed7f4837a0c0c. Dispatch explicitly requested round 3; this round rechecks the seven latest open findings from the preserved round-1 records. Source, tests and normative fixes inspected in 1c4c605..32b95d6. No new findings from the scoped Rust, soundness, whitelist, normalization and requirement review. The earlier resolved findings remain fixed. No Cargo or aggregate gate run because the shared host is constrained and PR 1930 has the admission priority. The four changed FR/TC documents validate with 4/4 grammar-clean and zero grammar findings. A static scoped PASS is not a whole-program proof, full-matrix PASS or evidence-advisor completion.

Custody: all seven round-1 scratch artifacts are now committed under spec/reviews/qsl-1373/pr1972-review-r1/. This round's seven latest copies must replace those committed copies before merge, preserving original findings and all disposition history. No reviewed-branch edits were made by the reviewer.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Inspected the actual source/spec fix at 32b95d6. Exact after excerpt: "second is `(- u)` is `(- t u)` when the sum is outside rule 9's linear\n   Int fragment; rule 9 takes precedence for every sum in that fragment." |
| FND-002 | fixed | Inspected the actual source/spec fix at 32b95d6. Exact after excerpt: "The checker locally checks only these additional rule instances:\n\n- `rare_rewrite` with the exact name `bool-double-not-elim` verifies that a\n  Bool-sorted sole argument equals its double negation;" |
| FND-003 | fixed | Inspected the actual source/spec fix at 32b95d6. Exact after excerpt: "operator is checked after sign canonicalization and is accepted when the\n  resulting scale is positive; it SHALL not be rejected merely because the\n  source operand order gives a negative scale." |

Round-3 verdict: all latest open findings for this method are fixed. Scoped review is clear, conditional on coordinated focused validation and latest-artifact custody.


## Disposition pass 4 scope

Exact reviewed head: 568ddbaea129815d04fc0e633eb5d824412c02c8. Static Rust review of 553325c..8a5ca0 and the corrective 8a5ca0..568ddba delta, focusing on one-time alias resolution, immutable expanded clause meaning, relation orientation and the supplied test assertions. The 8a5ca0 review was incomplete and had no completed formal verdict. No new examples, source changes, builds, test execution or aggregate gate were performed in this pass. Existing row-1 Verified responsibility remains with the origin coder; a unit-level ProofOutcome::Refutes assertion would establish rule admission only.

The previous seven SR copies are committed under spec/reviews/qsl-1373/pr1972-review-r3/. These latest pass-4 copies still require coder custody. Previous finding and disposition text is preserved unchanged.

Current pass verdict: No new finding in this method. Existing matrix/advisor limitations remain unchanged.


## Disposition pass 5 scope

Exact reviewed head: 70c5b2cbe0a4daac96a39887e8bbbe532a128a67. Static review of 568ddba..70c5b2c: the only source delta adds explicit ProofOutcome::Refutes assertions to both positive alias sequences. The earlier invalid_step assertions remain. Proof::outcome returns Unverifiable for undecided rules, so these added assertions now distinguish actual rule admission. They do not establish a final refutation or the complete row-1 certificate's Verified result; that responsibility stays with the origin coder. No new finding. SR-1957026 FND-010 remains fixed by the previously reviewed single-pass source logic, unchanged in this delta.

No builds, tests, source edits or aggregate gate were performed by the reviewer. Owner reports focused test 1/1 and Quire 7/7; those are reported producer evidence, not an independently run gate. Previous full-matrix/advisor limitations remain unchanged. Seven prior R4 artifacts are committed under spec/reviews/qsl-1373/pr1972-review-r4/; these latest R5 copies need coder custody. All prior finding and disposition text remains unchanged.

Current pass verdict: scoped review clear; no reviewer finding remains open. Latest artifact custody and separate coordinated validation/final row-1 verification remain with the owning coder and lead. This is not a whole-program assurance or aggregate gate claim.

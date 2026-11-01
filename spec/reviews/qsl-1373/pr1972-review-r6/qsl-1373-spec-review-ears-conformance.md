---
id: SR-1957031
title: "QSL-1373 PR 1972 spec-review-ears-conformance"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1373; origin/main...1323427827cfcef6c27e670c953ca2c099a12b8e; four Rust files, FR-314/315, TC-893/900 and spec/reviews/qsl-1373"
---

## Summary

Ticket: QSL-1373. Frozen head: 1323427827cfcef6c27e670c953ca2c099a12b8e. Engine: quire validate --scope . 'spec/functional/FR-31[45]-*.md' --summary returned 2/2 grammar-clean, zero findings. Reviewed changed SHALL statements for subject, trigger and observable response. Existing enumerated requirement dialect preserved. Module duplicate/advisory diagnostics did not invalidate the documents.

## Verdict

PASS. No additional defects found within the stated evidence limits.

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


### Disposition round 3

Exact reviewed head: 32b95d600f80890be44198501faed7f4837a0c0c. Dispatch explicitly requested round 3; this round rechecks the seven latest open findings from the preserved round-1 records. Source, tests and normative fixes inspected in 1c4c605..32b95d6. No new findings from the scoped Rust, soundness, whitelist, normalization and requirement review. The earlier resolved findings remain fixed. No Cargo or aggregate gate run because the shared host is constrained and PR 1930 has the admission priority. The four changed FR/TC documents validate with 4/4 grammar-clean and zero grammar findings. A static scoped PASS is not a whole-program proof, full-matrix PASS or evidence-advisor completion.

Custody: all seven round-1 scratch artifacts are now committed under spec/reviews/qsl-1373/pr1972-review-r1/. This round's seven latest copies must replace those committed copies before merge, preserving original findings and all disposition history. No reviewed-branch edits were made by the reviewer.

Round-3 verdict: no currently open finding for this method and no additional finding. Existing full-matrix and catalog-advisor evidence limitations remain unchanged. Clean methods retain their prior conclusions within those limits.


## Disposition pass 4 scope

Exact reviewed head: 568ddbaea129815d04fc0e633eb5d824412c02c8. Static Rust review of 553325c..8a5ca0 and the corrective 8a5ca0..568ddba delta, focusing on one-time alias resolution, immutable expanded clause meaning, relation orientation and the supplied test assertions. The 8a5ca0 review was incomplete and had no completed formal verdict. No new examples, source changes, builds, test execution or aggregate gate were performed in this pass. Existing row-1 Verified responsibility remains with the origin coder; a unit-level ProofOutcome::Refutes assertion would establish rule admission only.

The previous seven SR copies are committed under spec/reviews/qsl-1373/pr1972-review-r3/. These latest pass-4 copies still require coder custody. Previous finding and disposition text is preserved unchanged.

Current pass verdict: No new finding in this method. Existing matrix/advisor limitations remain unchanged.


## Disposition pass 5 scope

Exact reviewed head: 70c5b2cbe0a4daac96a39887e8bbbe532a128a67. Static review of 568ddba..70c5b2c: the only source delta adds explicit ProofOutcome::Refutes assertions to both positive alias sequences. The earlier invalid_step assertions remain. Proof::outcome returns Unverifiable for undecided rules, so these added assertions now distinguish actual rule admission. They do not establish a final refutation or the complete row-1 certificate's Verified result; that responsibility stays with the origin coder. No new finding. SR-1957026 FND-010 remains fixed by the previously reviewed single-pass source logic, unchanged in this delta.

No builds, tests, source edits or aggregate gate were performed by the reviewer. Owner reports focused test 1/1 and Quire 7/7; those are reported producer evidence, not an independently run gate. Previous full-matrix/advisor limitations remain unchanged. Seven prior R4 artifacts are committed under spec/reviews/qsl-1373/pr1972-review-r4/; these latest R5 copies need coder custody. All prior finding and disposition text remains unchanged.

Current pass verdict: scoped review clear; no reviewer finding remains open. Latest artifact custody and separate coordinated validation/final row-1 verification remain with the owning coder and lead. This is not a whole-program assurance or aggregate gate claim.


## Disposition pass 6 scope

Exact reviewed head: 513dafdee71381630531aefad9d3e78e27e03801, rebased onto main 6aec0a98. Narrow static compatibility review of commit 513dafd and preservation of reviewed proof behavior. No unrelated main changes were reviewed. Direct git comparison with reviewed 70c5b2c shows no content difference in certificate/alethe.rs, certificate/smt.rs, smt/normal.rs or tests/it/refinement_smt.rs: prior arithmetic fixes, single-pass clause metadata and both explicit ProofOutcome::Refutes alias-admission assertions are retained.

Examined qsl-replay/src/spine/clause/tests.rs fixtures, source receiver expressions, Member::Field's typed-read contract, LowerFrame::Attribute lowering and package member emission. Bare model-object reads carry explicit_deref Some(false), which emits read_form bare; reaches_field's structural member carries None. The updated exact Member assertions preserve the same declaration/name identity and explicitly distinguish the read forms. assert_ne is accompanied by exact expected values for both members, so it does not weaken the field-identity oracle. FR-105-AC-2 and TC-462 bindings remain local QSL identifiers. No production behavior is changed by this fixture-only commit, and no new finding or open reviewer finding remains.

Additional examined unit: FR-105-AC-2, spec/functional/FR-105-emit-state-nodes.md: "Each emitted node's body, `semantic_type`, `dependencies` and occurrences match the Outputs table; `VersionUnchanged`'s condition holds a `quire.op.state.pre` application over the field read of `versionNumber`; `NoCycle`'s holds a `quire.op.model.reaches_field` application whose member is `{kind: \"field\", declaration: <ConfigVersion node>, name: \"parent\"}`, the same member shape `ParentOrder`'s `self.parent` read carries."

Context-only: qsl-semantics/src/value/member.rs, qsl-semantics/src/check/lowering.rs Attribute projection and qsl-package/src/emit.rs typed read_form emission. These are evidence for fixture compatibility, not a review of unrelated main. No new normative interface was authored; no additional spec lens needed.

No builds, test execution, source edits or aggregate gate. Existing full-matrix/advisor and separate row-1 verification limitations remain unchanged. Prior seven R5 copies are committed under spec/reviews/qsl-1373/pr1972-review-r5/. Latest R6 copies need coder custody. All earlier findings and disposition history are preserved.

### Dispositions, round 6

No currently open finding required another outcome row; all prior fixed outcomes remain fixed. No new findings.

Current pass verdict: scoped review clear. Latest artifact custody and coordinated validation/merge remain with the owning coder and lead.

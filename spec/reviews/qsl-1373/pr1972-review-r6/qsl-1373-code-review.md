---
id: SR-1957026
title: "QSL-1373 PR 1972 code-review"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1373; origin/main...1323427827cfcef6c27e670c953ca2c099a12b8e; four Rust files, FR-314/315, TC-893/900 and spec/reviews/qsl-1373"
---

## Summary

Ticket: QSL-1373. Frozen head: 1323427827cfcef6c27e670c953ca2c099a12b8e. Rust lane: rust-review and rust-style API, implementation, safety, wire and testing references applied. Reviewed all four changed Rust files, query sorting, equality canonicalization, relation orientation, exact coefficients, sparse multiplication and both checking passes. No accepted false polynomial identity was found. Wrong sorts remain rejected or undecided. No heavy builds run: origin coder must use locked-build.sh, own worktree target, -j1 and test threads 1. PR body -j2 and /tmp target claims are not accepted shared-host gate evidence.

## Verdict

CONDITIONAL. Resolve findings inside PR 1972 before merge.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Literal zero retains a zero coefficient while cancellation removes it: (step p (cl (= (- x x) 0)) :rule poly_simp) returns ProofStepInvalid although both sides are zero. Return the same empty normal form for every zero polynomial. | qsl-replay/src/certificate/alethe.rs:1683 | correct-requirement-no-evidence |
| FND-002 | medium | linear_form retains first occurrence order instead of canonical SMT-LIB atom order. Add([Const(y),Const(x)]) prints (+ y x), while the reversed operands print (+ x y), violating the declared canonical order. | qsl-eval/src/smt/normal.rs:275 | correct-requirement-no-evidence |
| FND-003 | medium | A well-formed wrong-shape command (step p (cl true) :rule poly_simp) returns Undecided, so invalid_step does not reject it and outcome reports Unverifiable. AC-18 explicitly requires ProofStepInvalid for a non-equality conclusion. | qsl-replay/src/certificate/alethe.rs:1786 | correct-requirement-no-evidence |
| FND-004 | medium | Multiplication reserves one edge per term pair but each pair generates a monomial and performs a coefficient combination, which the spec charges separately. For (* (+ x0 ... x19) (+ y0 ... y19)), the last product performs 400 generations and 400 additions after a 400-edge reservation; all recursive charges for that polynomial total only 503. The declared budget can therefore be exceeded before the checker stops. | qsl-replay/src/certificate/alethe.rs:1736 | correct-requirement-no-evidence |
| FND-005 | medium | The existing arithmetic_over_literals_folds assertion at line 484 expects normal(x + -y) to print (- x y). New linear_form converts that Subtract back into Add and prints (+ x (- y)); the existing test deterministically fails. Reconcile normal-form rules 7 and 9 and run the existing test as well as the new filtered test. | qsl-eval/src/smt/normal.rs:47 | correct-requirement-no-evidence |

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
| FND-006 | medium | canonical_equality_difference generates a new polynomial by cloning every monomial and negating every coefficient with no charge callback. A poly_simp_rel equality reaching this routine performs this work even when the available edge budget is exhausted; the next relation_scale charge happens afterwards. Charge each generated monomial before cloning/storing it, or avoid generating a second map. | qsl-replay/src/certificate/alethe.rs:1909 | implementation-bug-despite-evidence |
| FND-007 | medium | arith-leq-norm accepts an arithmetic expression as its bound and evaluates it before charge_work, although AC-21 requires an integer-literal bound. For :args ("arith-leq-norm" x (+ 1 1)), the matching equality (= (<= x (+ 1 1)) (not (>= x (+ (+ 1 1) 1)))) is accepted. Replacing the bound by a large literal multiplication also performs arbitrary-precision arithmetic before the first charge. Require the declared literal shape before any evaluation. | qsl-replay/src/certificate/alethe.rs:2260 | implementation-bug-despite-evidence |
| FND-008 | medium | The subtraction exception matches any unary-negated term and any other non-Int term, rather than the new rule-9 coefficient pair +1/-1. Thus normal(Add([Negate(x),Negate(y)])) prints (- (- y) x), and normal(Add([Negate(x),Scale(2,y)])) prints (- (* 2 y) x). Neither has the required coefficient pair; both bypass rule 9's general + representation. Restrict the exception after resolving the rule-7 precedence finding. | qsl-eval/src/smt/normal.rs:311 | implementation-bug-despite-evidence |
| FND-009 | medium | The malformed-known-rule guard only checks whether the conclusion has head "=". (step p (cl (= x x x)) :rule poly_simp) is well sorted (application_sort accepts n-ary equality) but polynomial_equality returns None because two() fails, producing Unverifiable instead of the required ProofStepInvalid for a wrong conclusion shape. | qsl-replay/src/certificate/alethe.rs:1803 | implementation-bug-despite-evidence |

## Dispositions

Round 1 at 1c4c605c8fcf759660667eb918b72d5a3c79d4bd. Original finding text is preserved. Fixed means the stated defect was verified corrected by source inspection; focused test execution remains owned by the coordinated origin-coder lane.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Source fix inspected; exact after_excerpt is in the corresponding Linear round-1 disposition comment. |
| FND-002 | fixed | Source fix inspected; exact after_excerpt is in the corresponding Linear round-1 disposition comment. |
| FND-003 | fixed | Source fix inspected; exact after_excerpt is in the corresponding Linear round-1 disposition comment. |
| FND-004 | fixed | Source fix inspected; exact after_excerpt is in the corresponding Linear round-1 disposition comment. |
| FND-005 | fixed | Source fix inspected; exact after_excerpt is in the corresponding Linear round-1 disposition comment. |
| FND-006 | still-open | New disposition-pass-1 finding; no subsequent fix is present in frozen head 1c4c605. |
| FND-007 | still-open | New disposition-pass-1 finding; no subsequent fix is present in frozen head 1c4c605. |
| FND-008 | still-open | New disposition-pass-1 finding; no subsequent fix is present in frozen head 1c4c605. |
| FND-009 | still-open | New disposition-pass-1 finding; no subsequent fix is present in frozen head 1c4c605. |

Current round verdict: NOT MERGEABLE: still-open findings require another fix round.


### Disposition round 3

Exact reviewed head: 32b95d600f80890be44198501faed7f4837a0c0c. Dispatch explicitly requested round 3; this round rechecks the seven latest open findings from the preserved round-1 records. Source, tests and normative fixes inspected in 1c4c605..32b95d6. No new findings from the scoped Rust, soundness, whitelist, normalization and requirement review. The earlier resolved findings remain fixed. No Cargo or aggregate gate run because the shared host is constrained and PR 1930 has the admission priority. The four changed FR/TC documents validate with 4/4 grammar-clean and zero grammar findings. A static scoped PASS is not a whole-program proof, full-matrix PASS or evidence-advisor completion.

Custody: all seven round-1 scratch artifacts are now committed under spec/reviews/qsl-1373/pr1972-review-r1/. This round's seven latest copies must replace those committed copies before merge, preserving original findings and all disposition history. No reviewed-branch edits were made by the reviewer.

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | Inspected the actual source/spec fix at 32b95d6. Exact after excerpt: "for (monomial, coefficient) in polynomial {\n        if !charge(1) {\n            return Err(());\n        }\n        negative.insert(monomial, coefficient.neg());\n    }" |
| FND-007 | fixed | Inspected the actual source/spec fix at 32b95d6. Exact after excerpt: "if name == \"arith-leq-norm\"\n            && matches!(arguments, [_, bound] if !matches!(bound.as_ref(), Term::Number { real: false, .. }))\n        {\n            return Ok(Check::Undecided);\n        }\n        self.charge_work(charge)?;" |
| FND-008 | fixed | Inspected the actual source/spec fix at 32b95d6. Exact after excerpt: "fn is_unit_coefficient(term: &Term) -> bool {\n    !matches!(\n        term,\n        Term::Int(_) \| Term::Negate(_) \| Term::Scale(_, _) \| Term::Add(_) \| Term::Subtract(_, _)\n    )\n}" |
| FND-009 | fixed | Inspected the actual source/spec fix at 32b95d6. Exact after excerpt: "if equality_arguments.len() != 2 {\n            return Ok(Check::Fails);\n        }" |

Round-3 verdict: all latest open findings for this method are fixed. Scoped review is clear, conditional on coordinated focused validation and latest-artifact custody.


## Disposition pass 4 scope

Exact reviewed head: 568ddbaea129815d04fc0e633eb5d824412c02c8. Static Rust review of 553325c..8a5ca0 and the corrective 8a5ca0..568ddba delta, focusing on one-time alias resolution, immutable expanded clause meaning, relation orientation and the supplied test assertions. The 8a5ca0 review was incomplete and had no completed formal verdict. No new examples, source changes, builds, test execution or aggregate gate were performed in this pass. Existing row-1 Verified responsibility remains with the origin coder; a unit-level ProofOutcome::Refutes assertion would establish rule admission only.

The previous seven SR copies are committed under spec/reviews/qsl-1373/pr1972-review-r3/. These latest pass-4 copies still require coder custody. Previous finding and disposition text is preserved unchanged.

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-010 | high | At 8a5ca0, oriented_literal, ordered_poly_relations and read_clause separately expand mutable aliases. The supplied changing_named_alias regression advances the same arithmetic alias through different expressions, so the relation metadata can validate a true relation equivalence while the stored clause equates opposite truth values. | qsl-replay/src/certificate/alethe.rs:1347 | implementation-bug-despite-evidence |

### Dispositions, round 4

| FND | outcome | reason |
| --- | --- | --- |
| FND-010 | fixed | Single-pass ReadClause inspected; exact fixed excerpt: "poly_relations = ordered_poly_relations(&term);\n        parsed.push(term);" |

Current pass verdict: The repeated-resolution defect is fixed. No new arithmetic acceptance defect found in the scoped correction; PR approval remains blocked by the gap-analysis finding.


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

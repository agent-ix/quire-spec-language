---
id: TC-893
title: "An SMT proof certificate is accepted or rejected by the core checker"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: verifies
---
# TC-893: An SMT proof certificate is accepted or rejected by the core checker

## Description

Verify that `check_smt_proof` accepts Alethe refutations of the queries
FR-315 encodes, both for an inductive proof, and rejects a query mismatch,
an invalid proof step, a proof that does not refute and a
certificate of the wrong shape.

Scope: FR-314-AC-1 to FR-314-AC-4, FR-314-AC-9, FR-314-AC-10 to FR-314-AC-21.

## Test Procedure

1. Check FR-314-AC-1's `BoundedComplete{depth: 3}` certificate for
   `always[0,3] holds(c.value <= 3)` and its `Inductive{depth: 1}`
   certificate for `always holds(c.value <= 3)` over `Counter`; settle each.
2. Check the bounded certificate with its query at depth 2; with one
   step's premise replaced by a later step; with its last step removed. Check the inductive certificate
   with an invalid step proof, and a bounded certificate offered for the
   inductive basis. Settle each.
3. Settle the bounded result with no certificate; check and settle the
   bounded certificate with one step's rule replaced by `hole`, then by
   `lia_generic`.
4. Check the bounded certificate with its proof replaced by each proof of
   FR-314-AC-3, the renamed one included, and the inductive certificate
   with an empty base proof and with a malformed step proof. Settle each.
5. Check FR-314-AC-4's `RefinementSmtCertificate` for `CasRefinesCounter` (a
   base part and seven step parts) with `check_refinement_smt_proof`, then with
   six step parts, with eight, with the base part and a step part exchanged,
   with the third step part's query replaced by the fourth's, with the fifth step part's proof (at least four commands) having its
   command 3 not follow from its premises, and with command 3 of the base
   proof using `hole`, and over `CasRefinesCounterAdd` (FR-315-AC-6) with a base part and eight step parts, valid for the seven rows other than `bump`, the part at `bump`'s position being present but neither compared nor checked.
6. Over `CasRefinesCounter` with a mapping row outside `QF_LIA` (FR-314-AC-9), check a certificate with a base part carrying the query of an in-fragment initial state and seven step parts, then the same certificate with an empty base part and with a `hole` proof in a step part; then check the step 5 `CasRefinesCounterAdd` certificate again. Settle each.
7. Assemble a `RefinementSmtCertificate` for `CasRefinesCounter` from a CG
   `Base` record and seven step records (rows 0 to 6) supplied out of row
   order (FR-314-AC-10). Then assemble from the same records with the `Base`
   record missing (FR-314-AC-11), with row 3's record missing (FR-314-AC-12),
   with row 3's record supplied twice (FR-314-AC-13), with a row 7 record added
   (FR-314-AC-14), with row 3's query absent (FR-314-AC-15), and with row 3's
   proof absent (FR-314-AC-16); settle each. Assemble with the `Base` record's
   proof present and of no bytes and check it (FR-314-AC-17).
8. Check the row-1 proof's command `t28` with no premises and conclusion
   `(= (* 1 (- (+ 1 x) y)) (* 1 (- x (+ (- 1) y))))` under `poly_simp`, then
   check a separate `poly_simp` command with no premises and conclusion
   `(= (* x y) (* y x))` to exercise a nonlinear Int polynomial, then
   command `t29` with the single premise `t28` and conclusion
   `(= (= (+ 1 x) y) (= x (+ (- 1) y)))` under `poly_simp_rel`. Repeat with
   the false equality `(= (* 2 x) (+ (* 2 x) 1))`, a non-equality
   `poly_simp` conclusion, `t29`'s right atom changed to `(= x (+ 1 y))`, and
   `poly_simp_rel` with premise
   `(= (* 2 (- x y)) (* (- 2) (- u v)))` and conclusion
   `(= (< x y) (< u v))`, then with the same premise and conclusion
   `(= (< x y) (> u v))`. Repeat with a Real, division and modulo expression.
   Finally check the accepted polynomial certificate with
   `max_certificate_edges` set
   first one charge below the required total and then to the full required
   total.
9. Check `hole`, `lia_generic` and `rare_rewrite` with an unknown name and
   confirm each is `Unverifiable`. Check each of the five FR-314-AC-21 named
   rewrites with a valid instance and a false instance; additionally check
   `arith-leq-norm` with a non-literal bound and an arithmetic rewrite over
   Real, and `bool-double-not-elim` with an Int-sorted argument. Check `evaluate` with exact
   integer-literal `+`, `-` and `*` expressions, a false equality and an
   unsupported non-literal expression. Check `aci_simp` with an accepted
   associative/commutative/idempotent/identity example and a false example.

Tag the tests `#[trace("TC-893", "FR-314-AC-n")]`.

## Expected Results

- Step 1: both accepted; `proved`, `Certified`, success.
- Step 2: `QueryMismatch` at `Unrolling`; `ProofStepInvalid` at that step;
  `NotRefutation`; `ProofStepInvalid` at the
  `Step` step; `ShapeMismatch`; each `inconclusive`,
  `CertificateRejected`.
- Step 3: `proved`, `Uncertified`; `Unverifiable` at that step and
  `proved`, `Uncertified`, twice.
- Step 4: `ProofStepInvalid` at `ProofStep { Unrolling, 3 }`, both before
  and after the renaming; `NotRefutation` at `ProofStep { Unrolling, 4 }`;
  `Unverifiable { Unrolling, 3 }`, `proved`, `Uncertified`; `NotRefutation`
  at `Query { Unrolling }` for the empty proof; `Malformed` at
  `Query { Unrolling }` for each of the four malformed proofs;
  `NotRefutation` at `Query { Base }`; `Malformed` at `Query { Step }`.
  Each rejection is `inconclusive`, `CertificateRejected`.
- Step 5: the complete certificate `Verified`; six parts `ShapeMismatch` at
  `step/6`; eight parts `ShapeMismatch` at `step/7`; the exchanged parts
  `QueryMismatch` at `base`; the swapped query `QueryMismatch` at `step/2`;
  the invalid command `ProofStepInvalid` at `ProofStep { Step { row: 4 }, 3 }`;
  the `hole` `Unverifiable { Base, 3 }`; the `bump` row `Unencodable { Step { row: r } }`, `r` its position in the `steps` order; each rejection `inconclusive`,
  `CertificateRejected`. `Unverifiable` and `Unencodable` each settle `proved`,
  `Uncertified`, never `Verified` or `Certified`.
- Step 6: each of the three certificates is rejected `QueryMismatch` at
  `Base`, never `Unencodable` or `Unverifiable`, and settles `inconclusive`,
  `CertificateRejected`; the `CasRefinesCounterAdd` certificate again returns
  `Unencodable { Step { row: r } }`, `proved`, `Uncertified`.
- Step 7: the assembled certificate holds `format` `Alethe`, `base` and
  `steps[0..7]` with each record's query and proof bytes unchanged, in row
  order; each of the six defective record sets assembles no certificate and
  settles `proved`, `Uncertified`; the empty base proof assembles unchanged
  and is rejected `NotRefutation` at `Query { Base }`, `inconclusive`,
  `CertificateRejected`.
- Step 8: exact linear and nonlinear polynomial equalities and positive-scale
  relation steps are accepted. Equality-relation differences are canonicalized
  up to sign before scale classification, so valid swapped equality operands
  are accepted. The false equality, wrong conclusion shape, changed relation,
  wrong operator and same-operator negative order scaling are `ProofStepInvalid`.
  A negative-scale order relation with its reversed operator is `Unverifiable`.
  Real, division and modulo expressions return `Unverifiable` at their commands.
  The one-charge-short certificate returns
  `Stopped(LimitReached{limit: MaxCertificateEdges})`, never `Verified`, and
  settles `incomplete`; the full-budget certificate returns `Verified`.
  `hole`, `lia_generic` and unknown `rare_rewrite` each return `Unverifiable`
  at the command and settle `proved`, `Uncertified`. The five named rewrites,
  integer-literal `evaluate`, and Boolean `aci_simp` are accepted only for the
  local semantics in AC-21; false instances are `ProofStepInvalid` and
  unsupported instances are `Unverifiable`.

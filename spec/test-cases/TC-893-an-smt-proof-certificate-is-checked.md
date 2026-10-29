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

Scope: FR-314-AC-1 to FR-314-AC-4 and FR-314-AC-9.

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
6. Assemble a `RefinementSmtCertificate` for `CasRefinesCounter` from a CG
   `Base` record and seven `Step{row}` records (FR-314-AC-9), supplied out of
   row order. Then assemble from the same records with the `Base` record
   missing, with the `Step{row: 3}` record missing, with it supplied twice,
   with a `Step{row: 7}` record added, and with the `Step{row: 3}` record
   carrying no proof (FR-314-AC-10); settle each.

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
- Step 6: the assembled certificate holds `format` `Alethe`, `base` and
  `steps[0..7]` with each record's query and proof bytes unchanged, in row
  order; each of the five defective record sets assembles no certificate and
  settles `proved`, `Uncertified`.

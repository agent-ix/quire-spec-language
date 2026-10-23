---
id: TC-1195
title: "An HP-5 item settles through settle_existential with its product-closure check"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-163
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: verifies
---
# TC-1195: An HP-5 item settles through settle_existential with its product-closure check

## Description

Verify that an HP-5 item settles through FR-169's map by
`settle_existential`. Its `Witnessed` certificate is checked by FR-163's
`check_product_closure`, and each result of that check takes the state-graph
row of the same name. Also verify that a product-closure certificate whose
`package_id` is stale returns `PackageMismatch` before any rule, and that
the mismatch settles `StaleDependency`. Scope: FR-169-AC-13, FR-163-AC-8 and
FR-182-AC-13.

## Test Procedure

1. Over ADR-023 §8's secure vault, settle FR-181-AC-1's `Witnessed` outcome
   with `witness_samples` 0 through `settle_existential`, with its lassos
   replayed and its FR-163-AC-1 HP-5 certificate.
2. Repeat step 1 with one certificate member removed, then with the
   certificate's `package_id` replaced by another package's, then with the
   check run under `max_states` 1.
3. Repeat step 1 with the last step of one witness's loop removed.
4. Settle FR-181-AC-2's `Trapped` outcome through `settle_existential`.
5. Check FR-181-AC-1's HP-5 certificate and §8.1's secure `NonInterference`
   certificate with `check_product_closure`, unmodified, then each with its
   `package_id` replaced by another package's, then the `NonInterference`
   one with its keys also unsorted.
6. Settle §8.1's secure `NonInterference` `Holds` outcome whose certificate
   check is step 5's `PackageMismatch` through `settle_hyper_check`.

Tag the tests `#[trace("TC-1195", "FR-169-AC-13")]`,
`#[trace("TC-1195", "FR-163-AC-8")]` and `#[trace("TC-1195", "FR-182-AC-13")]`.

## Expected Results

- Step 1: `proved`, `decisive-witness`, `Proved{basis: Witness{[Explored,
  …]}, certification: Certified}`, success, one source per initial state.
- Step 2: `inconclusive`, `CertificateRejected{SuccessorMissing, at}` at the
  member whose successor is missing; `inconclusive`,
  `StaleDependency{requested, recompiled}` with `requested` the
  certificate's `package_id`; `incomplete`, `LimitReached` naming
  `max_states` and 1. None is `proved`.
- Step 3: the replay refuses as FR-128 refuses a loop that does not close,
  and the item settles `inconclusive`, `ReplayRefused`.
- Step 4: `refuted`, `closed-scope`.
- Step 5: `Accepted` twice; `PackageMismatch{certificate, recompiled}`
  naming both identities, for each; `PackageMismatch`, not `Malformed`.
- Step 6: `inconclusive`, `unsettled`, `StaleDependency{requested,
  recompiled}` with `requested` the certificate's `package_id`; not
  `proved`.

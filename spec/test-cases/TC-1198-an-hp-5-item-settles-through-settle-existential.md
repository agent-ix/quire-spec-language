---
id: TC-1198
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
# TC-1198: An HP-5 item settles through settle_existential with its product-closure check

## Description

Verify that an HP-5 item settles through FR-169's map by
`settle_existential`. Its `Witnessed` certificate is checked by FR-163's
`check_product_closure`, and each result of that check takes the state-graph
row of the same name. Scope: FR-169-AC-13.

## Test Procedure

1. Over ADR-023 §8's secure vault, settle FR-181-AC-1's `Witnessed` outcome
   with `witness_samples` 0 through `settle_existential`, with its lassos
   replayed and its FR-163-AC-1 HP-5 certificate.
2. Repeat step 1 with one certificate member removed, then with the
   check run under `max_states` 1.
3. Repeat step 1 with the last step of one witness's loop removed.
4. Settle FR-181-AC-2's `Trapped` outcome through `settle_existential`.
5. Check FR-181-AC-1's HP-5 certificate and §8.1's secure `NonInterference`
   certificate with `check_product_closure`, unmodified, then the
   `NonInterference` one with its keys unsorted.

Tag the tests `#[trace("TC-1198", "FR-169-AC-13")]`.

## Expected Results

- Step 1: `proved`, `decisive-witness`, `Proved{basis: Witness{[Explored,
  Explored]}, certification: Certified}`, success, one `Explored` for each of
  the vault's two initial snapshots.
- Step 2: `inconclusive`, `CertificateRejected{SuccessorMissing, at}` at the
  member whose successor is missing; `incomplete`, `LimitReached` naming
  `max_states` and 1. None is `proved`.
- Step 3: the replay refuses as FR-128 refuses a loop that does not close,
  and the item settles `inconclusive`, `ReplayRefused`.
- Step 4: `refuted`, `closed-scope`.
- Step 5: `Accepted` twice; the unsorted-keys certificate is `Rejected` with rule `Malformed`.

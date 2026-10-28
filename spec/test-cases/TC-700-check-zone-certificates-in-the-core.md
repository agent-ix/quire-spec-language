---
id: TC-700
title: "The in-core checker accepts valid zone certificates and rejects every tampering"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: verifies
---
# TC-700: The in-core checker accepts valid zone certificates and rejects every tampering

## Description

Verify `check_zone_certificate` on valid certificates, each tampering, identity and package refusals, budgets, and the Kani harnesses of its DBM code.

Scope: FR-245-AC-1 to FR-245-AC-4, FR-245-AC-5, FR-245-AC-6, FR-245-AC-8, FR-245-AC-9.

## Test Procedure

Fixtures: the certificates of TC-699; the tamperings of FR-245-AC-2; the Kani harnesses of the checker's DBM operations; the `Lock` model (FR-237-AC-6) and its variants; the workspace's `cargo metadata`.

1. Check each valid certificate.
2. Check each tampered certificate.
3. Check AC-1's certificate against another item's identity, after a source edit, and with `max_certificate_edges` 1.
4. Run the Kani harnesses for dimension at most 3 with bounds in `[-8, 8]`.
5. Check FR-244-AC-4's certificate, that certificate without the `locked` mark, and a `locked`-marked certificate for the `ping`-resets-`x` variant.
6. Read every core crate's normal, dev and build dependencies from `cargo metadata`.
7. Check the refinement-form certificates of FR-244-AC-5, the `locked`-mark and `positions` tamperings, a forged initial position, a replaced target position, a changed `taken`, the evaluation meter set to 0 and a foreign abstract package (FR-245-AC-8).
8. Check the `Init3` certificate (one failing-but-locked initial state, two passing divergent ones), the certificate with the `locked` mark removed from the initial node of index 1, with a `locked` mark added to the initial node of index 0, and a forged `Holds` certificate for the variant without the time invariant (FR-245-AC-9).

Tag the tests `#[trace("TC-700", "FR-245-AC-n")]`.

## Expected Results

- Step 1: `Accepted`.
- Step 2: `Rejected`, naming the failing node, edge or component.
- Step 3: `stale_dependency`/`content-mismatch` naming both identities; FR-098's refusal; `Stopped` naming the limit and 1.
- Step 4: every harness passes.
- Step 5: `Accepted`; `Rejected` naming the bad node; `Rejected` naming the reachable `ping` cycle.
- Step 6: no core crate names `qsl-analyze`.
- Step 7: `Accepted` for each refinement-form certificate; `Rejected` `StepFails` at the `ZoneEdge`; `Rejected` `InitialMismatch`; `Rejected` `PostDiffers`; `Stopped` naming `evaluation-meter` for the zero meter; `Rejected` `Malformed` at `CertificateMember("positions")`; `stale_dependency`/`content-mismatch` for the foreign package.
- Step 8: `Accepted` for the `Init3` certificate; `Rejected` `InitialMismatch` at the initial `ZoneNode` for each of the two marks; `Rejected` `BadState` at the forged `locked` initial node.

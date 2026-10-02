---
id: TC-583
title: "Reductions route only to advertising candidates, need footprints, and enter the obligation identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-159
    type: verifies
---
# TC-583: Reductions route only to advertising candidates, need footprints, and enter the obligation identity

## Description

Verify the V-8 settlements, negotiation by advertised reductions, and the identity membership of each reduction.

Scope: FR-159-AC-3 to FR-159-AC-4.

## Test Procedure

Fixtures: A test `TransitionSystem` without hooks; a registry with one candidate advertising `symmetry` only; ADR-021 §7.1's subject.

1. Request partial-order reduction over the test system; negotiate a `partial-order` request and a `symmetry` request against the registry.
2. Compute the identities of the §7.1 claim with `[[a, b, c]]`, with no reduction, and with and without constraint `Low`; settle a request with `ReductionNotPreserving`.

Tag the tests `#[trace("TC-583", "FR-159-AC-n")]`.

## Expected Results

- Step 1: V-8 `unsupported-requested-capability`; V-8 for `partial-order`; the `symmetry` request routes to the candidate.
- Step 2: the first two identities differ; the constraint pair are equal; no exploration statistics.

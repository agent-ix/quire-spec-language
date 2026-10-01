---
id: TC-638
title: "Exact runs stop on caller-set budgets and settle with ExactValue or ValueBounds"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: verifies
---
# TC-638: Exact runs stop on caller-set budgets and settle with ExactValue or ValueBounds

## Description

Verify the verdict map rows, the precision and resource budgets with their defaults, inconclusive and unsupported rows, proof accounting, and the obligation identity.

Scope: FR-203-AC-1 to FR-203-AC-4.

## Test Procedure

Fixtures: §15.1, §15.2, §15.3 and §15.4 claims; the FR-197-AC-4, FR-201-AC-1, FR-202-AC-2 and FR-199-AC-3 cases.

1. Settle §15.2 at `5 ms`, §15.1 with `max_rational_bits` 1,024, and §15.4's `Deliver`.
2. Settle FR-197-AC-4's straddle; §15.3's per-window claim with `max_states` 1,000; a request omitting every limit.
3. Settle the rejected certificate, the duplicated path set and the zero-weight variant.
4. Count proofs in a summary; compare obligation identities across confidence and `precision_bits`.

Tag the tests `#[trace("TC-638", "FR-203-AC-n")]`.

## Expected Results

- Step 1: `ExactValue{24233/25000}`; `ValueBounds{BackwardInduction{64}}`; `refuted` with a `Min` entry.
- Step 2: `PrecisionBudget` around `99/100`; `ResourceExhausted` naming `limits.max_states` and 1,000; the published defaults.
- Step 3: `CertificateRejected`; `ReplayParity`; `ZeroWeightComponent`; no `measured` value.
- Step 4: 2 proved; distinct identities for confidence, one identity across `precision_bits`.

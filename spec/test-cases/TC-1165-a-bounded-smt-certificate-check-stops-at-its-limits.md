---
id: TC-1165
title: "A bounded SMT certificate check stops at its limits and on cancellation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: verifies
---
# TC-1165: A bounded SMT certificate check stops at its limits and on cancellation

## Description

Verify that `check_smt_proof` and `check_refinement_smt_proof` run under
FR-245's `CertificateCheckLimits` and FR-276's `Cancel` handle, return
`Stopped` naming the limit, value and setting or the cancellation source,
and that FR-127 settles a stopped check V-7, never `proved`.

Scope: FR-314-AC-5.

## Test Procedure

1. Check TC-893's AC-1 bounded certificate with `max_certificate_bytes` 1;
   with `max_certificate_edges` one below its proof's command count plus
   its premise and discharge entries; with both limits at their defaults;
   and with a `Cancel` handle cancelled before the call. Settle each
   through FR-127's map.
2. Check TC-893's `QueryMismatch` certificate with `max_certificate_bytes`
   equal to its carried bytes plus the printed bytes of its expected query.
3. Check TC-893's `CasRefinesCounter` certificate through
   `check_refinement_smt_proof` with `max_certificate_bytes` 1.
4. Repeat each check under the same limits.

Tag the tests `#[trace("TC-1165", "FR-314-AC-5")]`.

## Expected Results

- Step 1: `Stopped` naming `max_certificate_bytes`, 1 and
  `certificate_check.max_certificate_bytes`, settling `incomplete`,
  `resource-incomplete`, `unavailable`; `Stopped` naming
  `max_certificate_edges`; `Verified`; `Stopped(Cancelled{Requested})`,
  settling `incomplete`, `cancelled{requested}`.
- Step 2: `QueryMismatch`, not `Stopped`.
- Step 3: `Stopped` naming `max_certificate_bytes`.
- Step 4: equal results.

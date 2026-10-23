---
id: TC-1194
title: "An SMT proof settles through the qsl-replay handoff by its checker result"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: verifies
---
# TC-1194: An SMT proof settles through the qsl-replay handoff by its checker result

## Description

Verify that `check_smt_proof` and `check_refinement_smt_proof` return each
`SmtProofCheck` alternative inside `Ok` and each rejection as `Err`, that an
expected query FR-315 refuses rejects `QueryMismatch`, and that
`settle_smt_proof` settles every `SmtProofOutcome` CG hands over by that
result, with or without a certificate.

Scope: FR-314-AC-6 and FR-127-AC-14.

## Test Procedure

1. Check TC-893's AC-1 bounded and inductive certificates, its AC-2
   `QueryMismatch` and `hole` certificates, its AC-4 `CasRefinesCounterAdd`
   certificate and TC-1165's `max_certificate_bytes` 1 check, and record
   each entry's return value.
2. Check TC-893's AC-1 bounded certificate for the TP-2 `on origin` claim
   `always[0,3] holds(c.value * c.value <= 9)` at `BoundedComplete{depth: 3}`.
3. From a crate whose only QSL dependency is `qsl-replay`, call
   `settle_smt_proof` with outcomes built from step 1's certificates, with
   `BoundedComplete{depth: 3}` and `certificate: None`, and with
   FR-315-AC-30's faulting rebuild.

Tag the tests `#[trace("TC-1194", "FR-314-AC-6")]` and
`#[trace("TC-1194", "FR-127-AC-14")]`.

## Expected Results

- Step 1: `Ok(Verified)` twice; `Err(QueryMismatch at Query { part:
  Unrolling })`; `Ok(Unverifiable { part: Unrolling, step })`;
  `Ok(Unencodable { part: Step { row: Some(r) } })`;
  `Ok(Stopped(LimitReached{..}))`.
- Step 2: `Err` with `QueryMismatch` at `Query { part: Unrolling }`.
- Step 3: `Proved{BoundedComplete{depth: 3}, Certified}`,
  `Proved{Inductive{depth: 1}, Certified}`,
  `Inconclusive(CertificateRejected{QueryMismatch, ..})`,
  `Proved{BoundedComplete{depth: 3}, Uncertified}` for the `hole`
  certificate and for `certificate: None` (no check run, meters unchanged),
  `Incomplete(LimitReached{MaxCertificateBytes, 1,
  certificate_check.max_certificate_bytes})` and `Failed`; the crate
  compiles.

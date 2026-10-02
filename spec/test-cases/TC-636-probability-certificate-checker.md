---
id: TC-636
title: "check_probability_certificate accepts sound certificates and rejects flawed ones in exact rationals"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: verifies
---
# TC-636: check_probability_certificate accepts sound certificates and rejects flawed ones in exact rationals

## Description

Verify acceptance and rejection of `Exact`, `LongRun`, `Lower` and fairness certificates, identity refusal, settlement through the checker, and the checker's budget.

Scope: FR-201-AC-1 to FR-201-AC-5.

## Test Procedure

Fixtures: §15.4's `Exact` certificate at threshold 0.95; §15.3's `LongRun` certificate; §15.1's dyadic `Lower` certificate; §15.6's `FairTerminates` certificate; altered copies; the `Opt` model, its even variant and their `Upper` certificates.

1. Check §15.4's certificate, then the copy with `4/5` replaced by `9/10`.
2. Check the `LongRun` certificate and its altered bias; check §15.1's `Lower` certificate.
3. Check a `Lower` certificate with no ranking; one with a foreign identity; `FairTerminates`'s certificate with and without its fairness set.
4. Settle a proof through the checker; run the checker with `max_states` 2.
5. Check the `Opt` certificate for `quantile 1/2 of M >= 3`, then the even variant's certificate.

Tag the tests `#[trace("TC-636", "FR-201-AC-n")]`.

## Expected Results

- Step 1: accepted, side `AtLeast`; rejected naming the first failing state, item `inconclusive`, `CertificateRejected`.
- Step 2: accepted; rejected; accepted with side `AtLeast`.
- Step 3: rejected; refused before re-enumeration; accepted, then rejected.
- Step 4: `proved`; `Incomplete(ResourceExhausted)` naming `max_states`.
- Step 5: accepted with side `AtLeast`, only `skip` tight; rejected, naming the activated state after `go`.

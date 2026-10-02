---
id: TC-690
title: "Timed outcomes settle as FR-331 terminal records with zone-certified, exhaustive and new-cause rows"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: verifies
---
# TC-690: Timed outcomes settle as FR-331 terminal records with zone-certified, exhaustive and new-cause rows

## Description

Verify the timed verdict map, including certificate acceptance and rejection, the time-lock row and the new causes.

Scope: FR-235-AC-1 to FR-235-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; the strict-guard variant; the `Stall` model; FR-234-AC-3's punctual claim.

1. Map one input of each table row.
2. Settle `Settles` (`T = 4 ms`) from the zone search, with its certificate intact and with one node removed; settle `NoLateReply` from the zone search.
3. Settle `NoLateReply` (`T = 3 ms`) after replay, and with the `timeout` delay changed to `5/2`; settle the strict-guard time-lock-freedom item.
4. Settle the `Stall` claim, the punctual claim and a zone search stopped by `max_symbolic_states`.

Tag the tests `#[trace("TC-690", "FR-235-AC-n")]`.

## Expected Results

- Step 1: each row's `TerminalValue`, label, basis and category as FR-235 states.
- Step 2: `Proved{ZoneCertified}`; `inconclusive`, `CertificateRejected`; `Proved{ZoneCertified}`.
- Step 3: `refuted` after replay; `inconclusive`; `refuted`, `closed-scope`, `kind: TimeLock`.
- Step 4: `inconclusive`, `NoAdmittedBehaviour`; `unsupported`, `PunctualInterval`; `incomplete`, `limit-reached`, naming the limit.

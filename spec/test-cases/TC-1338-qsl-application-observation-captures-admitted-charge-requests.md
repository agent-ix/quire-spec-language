---
id: TC-1338
title: "QSL application observation captures admitted charge requests at the call boundary"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-1338: QSL application observation captures admitted charge requests at the call boundary

## Description

Verify that `ApplicationObservation` reports the outcome, losses, meter totals
and exact ordered charge requests admitted during one checked function-body
evaluation. The expected charge trace is fixed by the test fixture and the
evaluator's actual charge calls; it does not read `Meter`'s optional
diagnostic log.

Scope: FR-090-AC-23.

## Test Procedure

1. Evaluate a checked function body that completes after issuing a known ordered
   sequence of meter charge requests. Record the fixture's expected request
   point and every requested counter amount.
2. Evaluate a checked function body that ends in a typed refusal after issuing
   a known ordered sequence of admitted charge requests.
3. Evaluate a checked function body with a meter limit that admits a known
   prefix and refuses the next known request.
4. For each evaluation, read the public `ApplicationObservation`, its returned
   evaluation, and the meter's final counter totals. Do not read or compare
   against the meter's diagnostic or test-support charge log.

Tag the tests `#[trace("TC-1338", "FR-090-AC-23")]`.

## Expected Results

- Steps 1 and 2: the observation's outcome and ordered losses equal the
  returned evaluation's values. Its charge ledger equals the fixture's full
  expected sequence, including each charge point and all requested counter
  amounts, and its ten totals equal the meter's final totals.
- Step 3: the outcome is incomplete at the refused charge. The ledger equals
  exactly the admitted prefix, has no row for the refused request, and its ten
  totals equal the meter's final totals. If the meter admits no request, the
  ledger is empty.
- In every step, the ledger records are captured by QSL at the actual
  `Meter::charge` call boundary; the test-support log's capacity or contents
  do not determine the result.

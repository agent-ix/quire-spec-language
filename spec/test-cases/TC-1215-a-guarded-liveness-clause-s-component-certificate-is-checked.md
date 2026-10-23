---
id: TC-1215
title: "A guarded liveness clause's component certificate is accepted or rejected"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: verifies
---
# TC-1215: A guarded liveness clause's component certificate is accepted or rejected

## Description

Verify that `check_components` accepts the component certificate of a TP-4
clause with a guarded `on each` activation, deciding the activation set from
the phases of the states, and rejects a failing witness, a missing active
successor and an undefined condition.

Scope: FR-339-AC-5.

## Test Procedure

Fixtures: ADR-018 §6's example unit, universe `{a, b}`, both versions 0, and
a test-built checked package carrying the guarded clause's `temporal_clause`
node in QSpec FR-370's shape (`kind` `each`, `trigger` the `over`
parameter's node, `guard` the clause's guard term), used as the engine's
clause source and as the request's recompiled package. Each `when`
condition is read by the test's own letters.

1. Check the certificate of `eventually holds(c.versionNumber = 2)` under
   infinite-trace `on each` when `c.versionNumber = 1`, with a weak `each`
   constraint on `attemptUpdate`.
2. Check the certificate of the same clause when `c.versionNumber = 7`, with
   no fairness constraint.
3. Check step 1's certificate with the waiting component's witness
   `UnfairWeak` naming `upd(a)`; with the active component's witness
   `MissingAcceptance{set: 0}`; with binding `a`'s active `(1,0)` state
   removed from the closure; with the condition undefined at version 1.

Tag the tests `#[trace("TC-1215", "FR-339-AC-5")]`.

## Expected Results

- Step 1: for binding `a`, the active component `(1,0)`, `(1,1)`, `(1,2)`
  carries `UnfairWeak` naming `upd(a)` and the waiting component carries
  `MissingAcceptance{set: 0}` (symmetrically for `b`); accepted, and FR-127
  settles `proved`.
- Step 2: every non-trivial component carries `MissingAcceptance{set: 0}`;
  accepted.
- Step 3: `WitnessFails`; `WitnessFails` at the active component's first
  state; `SuccessorMissing` at the removed state; `BadState` at binding
  `a`'s waiting `(0,0)` state.

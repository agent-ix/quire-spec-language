---
id: TC-662
title: "The replay facade replays a protocol counterexample through ProtocolSystem"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-217
    type: verifies
---
# TC-662: The replay facade replays a protocol counterexample through ProtocolSystem

## Description

Verify replay of protocol counterexamples: digest, enabledness and package checks, the deadlock check with blocked threads, and lasso fairness over scheduler constraints.

Scope: FR-217-AC-1 to FR-217-AC-4.

## Test Procedure

1. Replay `Fill`'s deadlock counterexample; serialize its envelope.
2. Replay with a changed digest, with `join(Both)` as the second step, and
   against the repaired package.
3. Replay with a blocked list missing `right`, and with a prefix that runs
   to s6.
4. Replay FR-212-AC-2's lasso against the adversarial package, and with
   scheduler constraints against the default package.

Tag the tests `#[trace("TC-662", "FR-217-AC-n")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness`; protocol identities and
  the blocked list in the envelope.
- Step 2: `stale_dependency`/`revision-mismatch` naming the step and both
  digests; `invalid_runtime_input`/`invalid-value`; FR-098's package
  refusal.
- Step 3: `inconclusive`, `ReplayParity`, both times.
- Step 4: `reproduced-with-evaluated-witness`; then
  `invalid_runtime_input`/`invalid-value` as unfair.

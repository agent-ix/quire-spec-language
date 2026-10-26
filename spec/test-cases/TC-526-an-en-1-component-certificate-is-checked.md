---
id: TC-526
title: "An EN-1 component certificate is accepted or rejected by the core checker"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: verifies
---
# TC-526: An EN-1 component certificate is accepted or rejected by the core checker

## Description

Verify that `check_components` accepts a weak-fairness liveness proof's
partition and rejects a failing witness, a backward edge and a
non-partition.

Scope: FR-339-AC-1 to FR-339-AC-3, FR-339-AC-6 and FR-339-AC-7.

## Test Procedure

1. Check the certificate of FR-126-AC-1's weak `each` proof.
2. Check it with the witness of `{(*, 0, q1)}` naming `upd(a)`; with the
   two accepting components listed first; with one state in two
   components; with a `Trivial` witness over a state with a self-edge.
3. Check FR-339-AC-3's `UnfairVisibility` certificate for the synthesized SCC with `r` the only accepting state, then with the witness `UnfairWeak`, then with `q → r` not removed.
4. Check step 1's certificate with `max_states`, `max_transitions` and
   `max_automaton_states` equal to its closure's counts, then with
   `max_states` one below, then with a Cancel handle cancelled at its first
   check after the closure check completes.
5. Check the certificate of FR-339-AC-6's `Ring` proof with `max_states` 5,
   `max_transitions` 9 and `max_automaton_states` 2; under the default
   meter budget; under a meter budget whose `work_units` is 0, then check
   its closure alone (FR-338) under that budget.
6. Check, with `check_protocol_components`, the certificate of FR-228-AC-2's `tso` proof under its
   `ProtocolCertificateRequest`; then with the polling-after-store
   component's witness naming the scheduler constraint of branch `w`, then
   `r`'s `flush` class; then with the polling-before-store component's
   witness naming `w`'s `flush` class.

Tag the tests `#[trace("TC-526", "FR-339-AC-n")]`.

## Expected Results

- Step 1: accepted.
- Step 2: `WitnessFails` at `{(*, 0, q1)}`'s first state; `BackwardEdge`;
  `NotPartition`; `WitnessFails`.
- Step 3: accepted although `q → r` runs backward; `WitnessFails`; `BackwardEdge`.
- Step 4: accepted; `Stopped(LimitReached{MaxStates, value, setting})`;
  `Stopped(Cancelled{source: Requested})`; neither stop settles
  `CertificateRejected` or `proved`.
- Step 5: accepted; accepted; `Stopped(LimitReached{EvaluationMeter, value,
  setting})` equal in limit, value and setting to the closure-only check's
  stop, returned before any component rule runs.
- Step 6: accepted, the polling-after-store component naming `w`'s `flush`
  class and the polling-before-store component the scheduler constraint of
  branch `w`; then `WitnessFails` three times.

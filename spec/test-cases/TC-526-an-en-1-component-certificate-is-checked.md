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

Scope: FR-339-AC-1 to FR-339-AC-3 and FR-339-AC-6.

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
   `max_transitions` 9 and `max_automaton_states` 2; with the meter budget
   `M + E`; with `M + E - 1`.

Tag the tests `#[trace("TC-526", "FR-339-AC-n")]`.

## Expected Results

- Step 1: accepted.
- Step 2: `WitnessFails` at `{(*, 0, q1)}`'s first state; `BackwardEdge`;
  `NotPartition`; `WitnessFails`.
- Step 3: accepted although `q → r` runs backward; `WitnessFails`; `BackwardEdge`.
- Step 4: accepted; `Stopped(LimitReached{MaxStates, value, setting})`;
  `Stopped(Cancelled{source: Requested})`; neither stop settles
  `CertificateRejected` or `proved`.
- Step 5: accepted; accepted; `Stopped(LimitReached{EvaluationMeter, M + E - 1,
  setting})` during the third component with the counters at 5, 9 and 2.

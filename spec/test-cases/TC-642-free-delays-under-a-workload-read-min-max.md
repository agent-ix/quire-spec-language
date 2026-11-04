---
id: TC-642
title: "A delay no distribution gives is decided by its minimum or maximum under a workload"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: verifies
---
# TC-642: A delay no distribution gives is decided by its minimum or maximum under a workload

## Description

Verify that under a workload EN-5 leaves the delay of an operation with no
`delay` declaration to the scheduler, assumes no distribution for it, and reads a
`>= θ` bound on the minimum and a `<= θ` bound on the maximum over that
choice.

Scope: FR-204-AC-5 to FR-204-AC-12.

## Test Procedure

Fixture: §15.5's `Retx`, whose `send` has no `delay` declaration, and the
workload `One` with weight 1 on `send`.

1. Decide `probability >= 0.99 [eventually[0 ms, 4 ms] holds(m.delivered)]`
   under `One`.
2. Decide `probability <= 0.999` over the same event under `One`, and read
   the witness scheduler.

3. Over `Race` with `B` guarded by `x >= 1 ms`, decide
   `probability >= 0.5 [eventually[0 ms, 5 ms] holds(r.b)]` and
   `probability <= 0.5` under `Both`, and read each witness scheduler's choice.
4. With `B` guarded by `x >= 2 ms`, decide `probability <= 0.5` and
   `probability <= 0.4` under `Both`, and the same claim under every scheduler.
5. With `A` declaring `delay Race::A ~ discrete { 1 ms: 1, 3 ms: 1 }` and `B` guarded by `x >= 1 ms`, decide `probability <= 0.7` and `probability >= 0.3` under `Both`, list the initial state's choices, and read each witness entry.
6. Over FR-204-AC-10's `Detour` under `All`, decide `probability <= 0.7 [eventually[0 ms, 2 ms] holds(d.b)]` with `max_witness_paths` 2 and `probability >= 0.45` over the same event; read each value, witness entry and path set, and replay the path set.
7. Over FR-204-AC-11's `Race` with the free operations `B` and `C`, list the initial state's choices, decide the maximum and the minimum of `eventually[0 ms, 5 ms] holds(r.b)` under `Both`, and replay a witness whose row names `B` alone.

8. Over FR-204-AC-12's `Loop` under `Both`, list the initial state's choices, decide `probability <= 0.9 [eventually holds(r.b)]`, read the witness entry, replay the refutation, and evaluate the chain induced by the entry `B` delay 1.

Tag the tests `#[trace("TC-642", "<AC id>")]`.

## Expected Results

- Step 1: `proved`, `ExactValue{99/100}`, the entry marked a minimum.
- Step 2: `refuted` on the maximum `9999/10000`, the entry marked a maximum;
  the witness scheduler sends at `x = 1`.
- Step 3: `refuted` on the minimum `ExactValue{0}` with the `FreeDelays` row given `[2]` to `B` delay 3;
  `refuted` on the maximum `ExactValue{1}` with the row given `[2]` to `B` delay 1; three `FreeDelays` choices and no `WorkloadResolve`.
- Step 4: `proved`, `ExactValue{1/2}`, marked a maximum; `refuted` on that
  maximum with the row given `[2]` to `B` delay 2; `unsupported`, `DelayDistribution` naming
  `A` under every scheduler.
- Step 5: `refuted` on the maximum `ExactValue{3/4}` with the rows given `[1]` to `B` delay 1 and given `[3]` to `B` delay 1; `refuted` on the minimum `ExactValue{1/4}` with the rows given `[1]` to `B` delay 2 and given `[3]` to `B` delay 3; nine `FreeDelays` choices and no `WorkloadResolve`.
- Step 6: `refuted` on the maximum `ExactValue{3/4}` with the rows given `[1]` to `B` delay 2 and given `[3]` to `B` delay 1, the path set `A` at 1 ms (`1/2`) and `B` at 1 ms then `G` at 2 ms (`1/4`) replaying `refuted`; `refuted` on the minimum `ExactValue{3/8}` with the rows given `[1]` to `B` delay 1 and given `[3]` to `B` delay 2. Neither value is the before-draw `5/8` or `1/2`.
- Step 7: nine `FreeDelays` choices, each one row given `[2]` naming `B` and `C`; maximum `ExactValue{1}` with `B` delay 1, `C` delay 2; minimum `ExactValue{0}` with `B` delay 2, `C` delay 1; the `B`-only row refuses `invalid_runtime_input`/`invalid-value`.
- Step 8: two `FreeDelays` choices; `refuted` on the maximum `ExactValue{1}` with the row given `[2]` to `B` delay 2; replay `refuted`; the `B` delay 1 chain has value `0`.

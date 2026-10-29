---
id: TC-841
title: "The replay facade replays a temporal counterexample over an observed trace"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-331
    type: verifies
---
# TC-841: The replay facade replays a temporal counterexample over an observed trace

## Description

Verify replay of a `WitnessEnvelope<TemporalCounterexample>` with observed
steps: agreement, parity disagreement, identity refusals and malformed
lassos.

Scope: FR-331-AC-1 to FR-331-AC-3 and FR-331-AC-6.

## Test Procedure

Build envelopes by hand over the `Counter` unit of TC-840.

1. Replay a `Reaches` packet with loop 0, 1 at position `0`, and the same
   packet with loop 0, 1, 2.
2. Replay a `Bounded` packet over 0, 1, 2 with `interval`
   `{lower: "0", upper: "1"}` at position `0` and at position `1`.
3. Replay step 1's first packet with an empty loop; at position `7`; with
   `trace: "model"`.
4. Replay step 1's first packet for the clause `ReachesFair`
   (`eventually holds(c.value = 2)` under `fair weak inc`).
5. Decode and re-encode a temporal replay request whose `over` reference is
   `Counters` / `c1`, then resolve that object in the first observed document.

Tag the tests `#[trace("TC-841", "FR-331-AC-n")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness`; `inconclusive`,
  `ReplayParity`.
- Step 2: `reproduced-with-evaluated-witness`; `inconclusive`,
  `ReplayParity`.
- Step 3: `invalid_runtime_input`/`invalid-value` three times.
- Step 4: `ReplayRefusal::MissingFairnessPremise` naming `fair weak whole
  inc`, catalog code `unsupported_projection`/`missing-fairness-premise`,
  O-16 unsupported, no result.
- Step 5: the request round-trips the `Counters` population and `c1` key, and
  replay binds that object in the first observed document.

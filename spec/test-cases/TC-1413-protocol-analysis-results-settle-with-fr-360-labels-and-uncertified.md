---
id: TC-1413
title: "Protocol analysis results settle with the FR-360 labels and Uncertified proofs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-573
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-360
    type: references
---
# TC-1413: Protocol analysis results settle with the FR-360 labels and Uncertified proofs

## Description

Verify the settlement map of FR-573 for both kinds, including the replay
arm and the certification. Steps 1, 2, 4 and 6, and step 5's `max_states`
and `max_transitions` stops, settle their items in one `analyze` run over
the linked engines; step 5's meter stop is settled on the engine's own stop
mapping (FR-573 "Serving"); step 3 edits a
counterexample of each kind and replays it through its arm.

Scope: FR-573-AC-1 to FR-573-AC-7.

## Test Procedure

1. Settle a sound projection and a realizable protocol; read label, axes and
   certification.
2. Settle a projection counterexample and each realizability refutation.
3. Settle a counterexample whose last step is removed, and one whose walk
   ends elsewhere than its stated divergence. For realizability also replay
   one with a step at another successor index, an absent initial state,
   equal actions, another obligation, key or record, a truncated defeating
   play, and a pair of histories whose two actions are both enabled at both
   ends with no evidence that either loses at one end.
4. Settle a run that reaches `max_depth`, one with an absent realizability
   premise, and one decided on a cut count.
5. Settle runs stopped by `max_states`, by `max_transitions` and by the
   meter.
6. Settle a protocol using `repeat`; settle a sound result and read its
   certification for `Certified` or `Trusted`.

Tag the tests `#[trace("TC-1413", "FR-573-AC-n")]`.

## Expected Results

- Step 1: `proved`, `closed-scope`, completed, `satisfied`, `Uncertified`.
- Step 2: `refuted`, `decisive-counterexample`, completed, `violated`, each
  after its arm passes: the `ReplaySource::ProtocolAnalysis` walk for the
  projection, the arena walk for each realizability form.
- Step 3: `inconclusive`.
- Step 4: `inconclusive`, completed, `pending`, `unsettled`.
- Step 5: `failed`, `resource-incomplete`, `unavailable`, no artifact, the
  limit and value named.
- Step 6: `unsupported`, `unavailable` axes.

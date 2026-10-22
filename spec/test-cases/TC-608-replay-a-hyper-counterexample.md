---
id: TC-608
title: "The replay facade replays hyper counterexamples of every kind through ModelSystem"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: verifies
---
# TC-608: The replay facade replays hyper counterexamples of every kind through ModelSystem

## Description

Verify `replay_model_trace_tuple` on `Lockstep`, `WitnessExhausted`, `Projected` and `StepTuple` counterexamples: reproduction, disagreement, refusal and determinism, and the refusal of an `Undefined` counterexample of an `align skip` clause.

Scope: FR-183-AC-1 to FR-183-AC-4, FR-183-AC-7, FR-183-AC-8.

## Test Procedure

Fixtures: the counterexamples of TC-601, TC-602, TC-603 and TC-604, and the edited and hand-built envelopes FR-183-AC-3 and AC-4 name.

1. Replay §8.1's leaky `Lockstep` and §8.2's leaky `WitnessExhausted`.
2. Replay the `Projected` and `StepTuple` counterexamples.
3. Replay each FR-183-AC-3 edited envelope.
4. Replay FR-183-AC-4's hand-built envelopes; §8.2's leaky one with `max_witness_set` 0; one envelope twice.
5. Replay FR-178-AC-9's `Undefined` counterexample.
6. Check FR-183-AC-8's matched and unmatched `MatchWitness`es, each twice, and the witness whose instance key is outside the universe.

Tag the tests `#[trace("TC-608", "FR-183-AC-n")]`.

## Expected Results

- Step 1: reproduced at `trace_position` 1; reproduced with `X_0 = {((1, 0), g)}` and `X_1` empty.
- Step 2: both reproduce.
- Step 3: each refuses with no result.
- Step 4: `inconclusive`, `Verdicts` for each hand-built envelope; the replay result stopped with `max_witness_set` and value 0; equal results.
- Step 5: refused `unsupported_construct`/`not-yet-implemented`, no result.
- Step 6: `Accepted`, the body not evaluated; `Refused` with `invalid_runtime_input`/`invalid-value` at joint step 1; equal results; `Refused` with `invalid_runtime_input`/`invalid-value` for the outside key before any re-execution.

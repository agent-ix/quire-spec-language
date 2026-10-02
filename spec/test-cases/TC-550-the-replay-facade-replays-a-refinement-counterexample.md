---
id: TC-550
title: "The replay facade replays a refinement counterexample and recomputes what the abstract model saw"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-145
    type: verifies
---
# TC-550: The replay facade replays a refinement counterexample and recomputes what the abstract model saw

## Description

Verify replay of refinement prefixes and lassos: recompiling both packages,
recomputing history, mapped states and candidate sets, rerunning
`check_step`, the parity rule and the refusals.

Scope: FR-145-AC-1 to FR-145-AC-4.

## Test Procedure

Fixtures: the counterexamples of FR-142-AC-2 to FR-142-AC-5 and FR-143-AC-1
to FR-143-AC-3; `RegisterHistory` with `ensure fair weak
Spec::Register::write`.

1. Replay the lost-update prefix, the `InitialNotAbstract`,
   `StutterChanged`, broken-history and `Coin` counterexamples.
2. Replay the divergence, terminal-stutter and `Pair` lassos.
3. Replay the three parity and unfairness variants of FR-145-AC-3.
4. Replay each refusal variant of FR-145-AC-4; replay one envelope twice.

Tag the tests `#[trace("TC-550", "FR-145-AC-n")]`.

## Expected Results

- Step 1: mapped values 0, 0, 0, 1, 1 and `reproduced-with-evaluated-witness` naming `AbstractStepRejected{position: 4, inc(c),
  Postcondition}`; each other counterexample reproduces.
- Step 2: `Divergence{inc}`; `Divergence`; `AbstractUnfair{flipB}`.
- Step 3: `inconclusive`, `ReplayParity`; `inconclusive`, `ReplayParity`;
  refused as unfair.
- Step 4: each refuses with FR-145-AC-4's code and subcode and no result;
  equal results.

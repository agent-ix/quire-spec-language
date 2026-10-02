---
id: TC-593
title: "Backward reachability and path counting decide state-graph claims with canonical evidence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: verifies
---
# TC-593: Backward reachability and path counting decide state-graph claims with canonical evidence

## Description

Verify witnesses, traps, path pairs and proofs on ADR-022 §7's worked
examples and their variants, decisive evidence on a partial run, and that
phase 0 changes only a witness's source.

Scope: FR-168-AC-1 to FR-168-AC-4 and FR-168-AC-6.

## Test Procedure

Fixtures: ADR-022 §7.1's subject; §7.2's game, its `restart` variant and
its `celebrate` variant (FR-168-AC-4); §7.3's job, its sequenced variant,
and the sequenced variant with `reset` (FR-168-AC-3).

1. §7.1 with `witness_samples` 0: `ReachesTwo` and `ReachesThree`.
2. §7.2: `CanStillWin`, its `from` variant, and the `restart` variant.
3. §7.3: `InOneWay` over the job, the sequenced variant and the `reset`
   variant.
4. The `celebrate` variant under `max_depth` 3: `CanStillWin` and `possible
   x.phase = Won and x.n = 50`.
5. `ReachesTwo` with `witness_samples` 64 and 0; step 2's requests twice.

Tag the tests `#[trace("TC-593", "FR-168-AC-n")]`.

## Expected Results

- Step 1: `Witnessed` with `upd(a), upd(a)` for `c = a` and `upd(b),
  upd(b)` for `c = b`; `Trapped` at `(0, 0)` with an empty stem for each
  instance.
- Step 2: `Trapped` at `Lost`, stem `play, lose`; `Holds{Exhaustive}`;
  `Holds{Exhaustive}`.
- Step 3: `PathPair`, empty stem, `stepA, stepB, finish` and `stepB, stepA,
  finish`; `Holds{Exhaustive}`; `PathPair` with `stepA, stepB, finish` and
  `stepA, reset, stepA, stepB, finish`.
- Step 4: `Trapped` at `Lost` with the node `Won`, `n = 1` open;
  `NoDecision`, end `Completed`, open cause `MaxDepth`.
- Step 5: `Witnessed` with sources `Sampled` and `Explored`; byte-equal
  outcomes.

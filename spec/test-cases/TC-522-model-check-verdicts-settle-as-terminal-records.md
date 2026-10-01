---
id: TC-522
title: "Model-check outcomes settle as FR-331 terminal records with their strength"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: verifies
---
# TC-522: Model-check outcomes settle as FR-331 terminal records with their strength

## Description

Verify the map from EN-1's outcomes and their replay results to
`TerminalValue`, QSpec FR-341 labels, FR-243 bases and O-16 categories; the
category of every `ProofBasis` and new inconclusive cause; each replay
refusal path; a faulting replay; and the named limit of a stopped run.

Scope: FR-127-AC-1 to FR-127-AC-5.

## Test Procedure

1. Settle one input of each row of FR-127-AC-1.
2. Read `TerminalValue::category` for each value of FR-127-AC-2.
3. Settle FR-126-AC-1's outcomes: the weak `each` proof; the counterexample
   under the constraint with no granularity, with its replay; that
   counterexample with one post-state digest altered; in an envelope for
   the weak `each` clause; with its last step removed; one whose formula
   evaluates `true` on replay; one whose replay returns `InternalFault`
   through a replay stand-in.
4. Settle FR-126-AC-6's `max_automaton_states` run and FR-126-AC-5's
   `max_depth` 2 and evaluation-meter runs.
5. Settle FR-126-AC-3's deadlock-freedom violation with its replay.

Tag the tests `#[trace("TC-522", "FR-127-AC-n")]`.

## Expected Results

- Step 1: each row exactly as FR-127's table; depth 2 and method
  `explicit-state` on V-5; the stopped record names `max_states`, value 2.
- Step 2: inconclusive `KaniVacuousProof`; success four times; inconclusive
  four times.
- Step 3: `proved`, `closed-scope`, `Proved{Exhaustive}`, success;
  `refuted`, `decisive-counterexample`, violation; `inconclusive`,
  `ReplayRefused` three times; `inconclusive`, `ReplayParity`; `failed`,
  category failed.
- Step 4: `failed`, `resource-incomplete`, `unavailable`,
  `Incomplete(ResourceExhausted)` naming `max_automaton_states`, value 50;
  `inconclusive`, `BoundReached{depth: 2}`, execution `completed`, truth
  `pending`; a stopped record naming `EvaluationMeter`, value 0.
- Step 5: `refuted` with a counterexample of `kind: Deadlock`, and an
  obligation identity distinct from the authored claims'.

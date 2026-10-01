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

Verify the map from model-check outcomes and replay results to
`TerminalValue`, QSpec FR-341 labels, FR-243 bases and O-16 categories,
including `ProofBasis`, the new inconclusive causes and the
`max_automaton_states` settlement.

Scope: FR-127-AC-1 to FR-127-AC-4.

## Test Procedure

1. Map one input of each FR-127 row (V-1 to V-8), with V-2 at depth 5, V-3
   at depth 2 and V-5 at depth 1; map `Proved{Checks{0}}` and
   `Proved{Checks{1}}`.
2. Settle FR-126-AC-1's outcomes: the `fair weak each` proof; the `fair
   weak` counterexample with its replay; the same counterexample with one
   post-state digest altered; one whose formula evaluates `true` on replay.
3. Settle FR-126-AC-5's `max_automaton_states` and `max_depth` runs.
4. Settle FR-126-AC-3's deadlock-freedom violation with its replay; a claim
   under the fixed-sample profile over a model subject.

Tag the tests `#[trace("TC-522", "FR-127-AC-n")]`.

## Expected Results

- Step 1: each row exactly as FR-127's table, depths and method carried;
  inconclusive `KaniVacuousProof`; success.
- Step 2: `proved`, `closed-scope`, `Proved{Exhaustive}`, success;
  `refuted`, `decisive-counterexample`, violation; `inconclusive`,
  `ReplayRefused`; `inconclusive`, `ReplayParity`.
- Step 3: `failed`, `resource-incomplete`, `unavailable`,
  `Incomplete(ResourceExhausted)` naming `max_automaton_states`;
  `inconclusive`, `BoundReached{depth: 1}`, execution `completed`, truth
  `pending`.
- Step 4: `refuted` with a counterexample of `kind: Deadlock`;
  `unsupported`, `unsupported-requested-capability`.

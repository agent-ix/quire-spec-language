---
id: TC-643
title: "State-graph certificates of true claims are accepted and certify the proof"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: verifies
---
# TC-643: State-graph certificates of true claims are accepted and certify the proof

## Description

Verify that the engine returns a state-graph certificate with every
unreduced `proved`, that the core checker accepts it for `possible`,
`always possible` and `unique path`, and that a reduced proof settles
`Uncertified` with no certificate.

Scope: FR-169-AC-9.

## Test Procedure

Fixtures: ADR-022 §7.1's subject with `ReachesTwo`; §7.2's game with
`CanStillWin` `from (x.phase != Lost)`; §7.3's sequenced job with
`InOneWay`; the `possible` item of FR-167-AC-4 with partial-order reduction.

1. Run each unreduced item, then run `check_state_graph` on its
   certificate and settle it.
2. Run and settle the reduced `possible` item.

Tag the tests `#[trace("TC-643", "FR-169-AC-9")]`.

## Expected Results

- Step 1: `ReachesTwo` for `c = a`: closure of 9 states, ranks 0 at
  `va = 2`, 1 at `va = 1`, 2 at `va = 0`; accepted;
  `Proved{Witness{…}, Certified}`. `CanStillWin` variant: ranks `Won` 0,
  `Mid` 1, `Start` 2, `Lost` unranked; accepted; `Proved{Exhaustive,
  Certified}`. `InOneWay` sequenced: every count 1 with a reverse
  topological order; accepted; `Proved{Exhaustive, Certified}`.
- Step 2: `Proved{Reduced{…}, Uncertified}`; no certificate.

---
id: TC-520
title: "A model subject's behaviours read as temporal traces, with terminal stutter and interval wrap"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: verifies
---
# TC-520: A model subject's behaviours read as temporal traces, with terminal stutter and interval wrap

## Description

Verify positions and anchors of a model behaviour, the closed reading under
a bounded profile and the terminal stutter under infinite-trace, interval
operators over a lasso, the `over` binding, and the empty-initial subject.

Scope: FR-125-AC-1 to FR-125-AC-4.

## Test Procedure

1. Build the `Counter` subject (FR-124-AC-1, no `terminal` member); read its
   behaviour's positions and anchors, under event-position false-extension
   and under infinite-trace.
2. Evaluate FR-125-AC-2's three formulas over that behaviour.
3. Evaluate FR-125-AC-3's two formulas over ADR-018 §6's lasso, reading the
   meter's charges.
4. Instantiate a clause with `over (c: Config::ConfigVersion)` over ADR-018
   §6's subject; build a subject with an empty `initial` list.

Tag the tests `#[trace("TC-520", "FR-125-AC-n")]`.

## Expected Results

- Step 1: positions `0, 1, 2, 3` with anchors `initialization`, `inc`,
  `inc`, `inc`; under infinite-trace, then the stutter step repeating
  position 3, with a transition identity distinct from `inc`'s.
- Step 2: `false` under false-extension and `true` under infinite-trace for
  `always[0,5]`; `true`; `false`.
- Step 3: `true` and `false`, one work unit per (node, position) visit.
- Step 4: two instances, `a` and `b`; `NoInitialState` before exploration.

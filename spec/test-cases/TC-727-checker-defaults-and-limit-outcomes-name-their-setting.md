---
id: TC-727
title: "Checker defaults admit any depth that fits, and its limit outcomes name the setting"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-258
    type: verifies
---
# TC-727: Checker defaults admit any depth that fits, and its limit outcomes name the setting

## Description

Verify that the checker's defaults hold no depth, that a deep input within
them checks, and that a node or work stop names its setting and clears
when that setting is raised at each entry point.

Scope: FR-258-AC-3, FR-258-AC-4.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. At the default limits, check, lower and emit a function whose body is the
   longest `a and (…)` chain the default S1 limits admit. Read
   `CheckingLimits::default()`, and build `CheckingLimits` with node counts
   0, 1 and `u64::MAX`.
2. Check a function whose body is a 1,000-term sum with `s3.nodes` at 1,500.
   Check it again with `s3.nodes` at 5,000 through the builder, through a
   replay request's `stage_limits` entry and through
   FR-255's settings operation given `s3.nodes=5000`.
3. Measure the same function's declaration work `w`; check it with
   `s3.work_units` at `w - 1`, then at `w` through each entry point.

Tag the tests `#[trace("TC-727", "FR-258-AC-3")]`, `#[trace("TC-727", "FR-258-AC-4")]`.

## Expected Results

- Step 1: the chain checks with no outcome naming a depth; the defaults are
  100000 nodes, 16777216 preimage bytes and 16777216 work units; every
  construction succeeds.
- Step 2: `stage_limit_exceeded`/`node-count-exceeded`, bound 1,500, count
  1,501, setting `s3.nodes`, at the node whose entry failed; each raised
  run checks.
- Step 3: `stage_limit_exceeded`/`work-budget-exceeded`, bound `w - 1`,
  count `w`, setting `s3.work_units`, at the declaration's span; at `w` the
  declaration's own charge is admitted and a later lowering charge stops it.


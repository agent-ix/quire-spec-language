---
id: TC-626
title: "Statistical runs stop on caller-set budgets and reproduce from their provenance"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-191
    type: verifies
---
# TC-626: Statistical runs stop on caller-set budgets and reproduce from their provenance

## Description

Verify each `StatisticalLimits` member, its default, the stop record, reproduction from the seed, and that method and limits stay out of the obligation identity.

Scope: FR-191-AC-1 to FR-191-AC-5.

## Test Procedure

Fixtures: `NoFault`, `P95`, FR-189-AC-4's activation variant, FR-190-AC-2's transient variant, a two-test variant of `Service`, FR-189-AC-7's `Wide` model for the `Unsupported` outcome.

1. `NoFault` with Okamoto at `max_samples` 1,000,000 and at 9,210,341; a request omitting `max_samples`.
2. The activation variant at `max_draws` 1,000; the transient variant; a cancelled `Cancel` handle.
3. `P95` with SPRT and seed 7 twice; with `max_samples` 100 and then the default; with seed 8.
4. The two-test variant; `P95` with Okamoto and with SPRT.
5. `NoFault` at `max_samples` 9,210,340 and 9,210,341; read the provenance of the stopped outcome and of the `Wide` model's `Unsupported` outcome (FR-189-AC-7), and the stop of a run cancelled by its handle.

Tag the tests `#[trace("TC-626", "FR-191-AC-n")]`.

## Expected Results

- Step 1: `Stopped` naming `statistical.max_samples` and 1,000,000 with no decision; decided; default 16,777,216.
- Step 2: `Stopped` naming `statistical.max_draws` and `Cancelled`, neither with a decision; the transient variant `Stopped` with `Limit{MaxCycleSteps, value}`, no decision, `limit-reached` naming `statistical.max_cycle_steps`.
- Step 3: byte-equal results; equal samples at trace indices 0 to 99; seed 8 recorded, equal obligation identity, a differing sample.
- Step 4: per-test provenance with test 1 starting at trace 1 in steps of 2; the method changes and the obligation identity does not.
- Step 5: complete at exactly 9,210,341, `Limit{MaxSamples, 9,210,340}` after 9,210,340 counted; provenance on both outcomes; `Cancelled` for the cancelled run.

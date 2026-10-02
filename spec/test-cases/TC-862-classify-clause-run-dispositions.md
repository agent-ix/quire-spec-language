---
id: TC-862
title: "Clause-run dispositions classify into one refinement class with their stage"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-342
    type: verifies
---
# TC-862: Clause-run dispositions classify into one refinement class with their stage

## Description

Verify FR-342's run classification over `run_clause` reports from FR-109's
and FR-115's fixtures, and over dispositions built in the test for the
variants the gate never produces.

Scope: FR-342-AC-1 to FR-342-AC-5.

## Test Procedure

1. Run healthy-parent, violating-parent and forbidden-parent-change
   (`Frame`).
2. Run a `Clause` selection naming `Absent`; a `Function` selection of a
   function returning `Integer`; dangling-parent; a `Function` selection
   of `sameIdentity` with an argument named `c`.
3. Run incomplete-population; run exhausted-work (work budget zero).
4. Build `StalePackage`, `UnknownLanguage`, `Admit(Fault)` and
   `EvaluateFault` dispositions.
5. Run missing-model (no package supplied).

Every expected class and stage below is a literal in the test. Tag the
tests `#[trace("TC-862", "FR-342-AC-n")]`.

## Expected Results

- Step 1: `admitted`/`evaluate`; `refused`/`evaluate`; `refused`/`evaluate`.
- Step 2: `absent`/`select`; `refused`/`select`; `refused`/`admit`;
  `refused`/`admit`.
- Step 3: `incomplete`/`admit` naming its limit; `incomplete`/`evaluate`
  naming `work_units` and value zero.
- Step 4: `tool failure`, four times.
- Step 5: `tool failure`/`compile`, equal to FR-341's class of its
  `CompileRefusal`.

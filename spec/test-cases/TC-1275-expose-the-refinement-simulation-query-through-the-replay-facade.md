---
id: TC-1275
title: "The replay facade encodes refinement queries and maps step rows to source rows"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-492
    type: verifies
---
# TC-1275: The replay facade encodes refinement queries and maps step rows to source rows

## Description

Verify the replay-owned `encode_refinement_query` and
`refinement_step_sources` wrappers over `CasRefinesCounter`. Scope: FR-492-AC-1 to
FR-492-AC-3 and FR-492-AC-5 to FR-492-AC-7; AC-4 is the `arch-lint-api-surface-qsl` make target.

## Test Procedure

Test layer: `qsl-replay/tests/it/refinement_query_facade.rs`, calling the
entries as the root exports them, over a `RefinementQueryRequest` built from root items
alone (`acquire_checked_closure`, `refinement_node_key`, `RefinementUniverse::for_population`,
`RefinementQuerySubject::new`); the fixture supplies document values and the oracle only.

1. Encode `Initial` and the `Step` queries of `peek` and `commitA`.
2. Read `refinement_step_sources`; compare with the checker's emitted order
   and map named operations to their source rows.
3. Write `commitB` before `commitA`; compare mappings and the `commitA` row's
   query.
4. Resolve `CasRefinesCounter`, `Missing` and the state clause `CanInc` with
   `refinement_node_key`; build a universe with `for_population`.

Tag the tests `#[trace("TC-1275", "FR-492-AC-n")]`.

## Expected Results

- Step 1: a one-copy and two-copy script, distinct per row, deterministic.
- Step 2: a permutation equal to the emitted order.
- Step 3: different mappings, equal `commitA` queries.
- Step 4: the refinement's node key, equal on repeat; `UnknownName` and
  `NotARefinement` respectively; a universe equal to the one built from the
  `DeclarationKey`.

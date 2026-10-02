---
id: TC-598
title: "Hyper and relation clauses classify into HP-1 to HP-6 and HP-4 settles unsupported"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-173
    type: verifies
---
# TC-598: Hyper and relation clauses classify into HP-1 to HP-6 and HP-4 settles unsupported

## Description

Verify form classification, the requirement records, V-8 for HP-4 and non-infinite-trace profiles, the HP-5 `match` refusal, and the request writer's subjects and deadlock-freedom items.

Scope: FR-173-AC-1 to FR-173-AC-4.

## Test Procedure

Fixtures: the clauses FR-173-AC-1 to AC-4 name.

1. Classify `NonInterference`, `Opaque`, `Det`, §13's `align skip` clause and the single-`exists` clause.
2. Classify and negotiate each FR-173-AC-2 clause.
3. Check the single-`exists` clause with a `match` block; negotiate `NonInterference` under the event-position profile.
4. Write requests for `SavesPower`, and for `NonInterference` with `Opaque`.

Tag the tests `#[trace("TC-598", "FR-173-AC-n")]`.

## Expected Results

- Step 1: `Universal`, `ForallExistsSafety`, `StepRelation`, `ProjectionAligned`, `SingleExistential`, each with (`temporal-satisfaction`, `Unbounded`).
- Step 2: `Other` naming its rule; V-8 `unsupported-requested-capability` with no engine run.
- Step 3: refusal at the block; no form and V-8.
- Step 4: two subjects and two deadlock-freedom items; one deadlock-freedom item.

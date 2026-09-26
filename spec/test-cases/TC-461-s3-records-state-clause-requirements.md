---
id: TC-461
title: "S3 records one operation-contract requirement per state clause and frame"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: verifies
---
# TC-461: S3 records one operation-contract requirement per state clause and frame

## Description

Verify the state clause `requirements` records and the stability of clause
identity.

Scope: FR-104-AC-5, FR-104-AC-6.

## Test Procedure

1. Check FR-108's unit without `sameIdentity` and read the requirement
   records of the `CheckedGraph`.
2. Remove `VersionUnchanged` and check again.
3. Check the step 1 unit twice, and once with its three clauses in reverse
   order.
4. Add `invariant ParentOrder2` with `ParentOrder`'s body.

Tag the tests `#[trace("TC-461", "FR-104-AC-n")]`.

## Expected Results

- Step 1: four records, each capability kind `operation-contract` and extent
  `Unbounded` with one domain, the `config_history` population (kind
  population, boundable by `Cardinality`): one per clause keyed by that clause's `claim` occurrence, and one
  keyed by `attemptUpdate`'s frame node occurrence.
- Step 2: two records, both clauses; no frame record.
- Step 3: equal clause node identities and equal record keys in all three.
- Step 4: `ParentOrder2` has `ParentOrder`'s node identity; the node carries
  two `claim` occurrences, ordinals 0 (`ParentOrder`) and 1 (`ParentOrder2`);
  the unit has five records, keyed by those two occurrences among others.

## Status

Planned (QSL-273).

---
id: TC-777
title: "QSL records tell the cache what it never stores, and hold no wall time"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-293
    type: verifies
---
# TC-777: QSL records tell the cache what it never stores, and hold no wall time

## Description

Verify that QSL terminal records carry the category, typed cause and producer `BackendId` the cache reads to apply QSpec FR-306's never-stored rule, and hold no wall time.

Scope: FR-293-AC-1, FR-293-AC-2.

## Test Procedure

1. Run TC-762 step 1's `analyze` request with a `Cancel` cancelled before it ends, with a `Deadline` cancel that passes, and with the state budget set to 1; settle a test plugin's `proved` result (FR-290).
2. Serialize FR-281-AC-1's and FR-281-AC-6's records, read each back with the typed reader, and run each request a second time.

Tag the tests `#[trace("TC-777", "<AC id>")]`.

## Expected Results

- Step 1: the three `incomplete` causes are pairwise distinct, and the budget cause names the state budget; the plugin record carries the plugin's `BackendId`.
- Step 2: no record holds a member that records a time, and each pair of runs gives equal bytes.

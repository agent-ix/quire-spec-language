---
id: TC-878
title: "Capability outcomes come only from vectors executed in the run, over the scope the caller selects"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-351
    type: verifies
---
# TC-878: Capability outcomes come only from vectors executed in the run, over the scope the caller selects

## Description

Verify that a corpus's recorded results do not decide an outcome, and that a caller scope selects exactly its capabilities or refuses an unknown id.

Scope: FR-351-AC-3, FR-351-AC-4.

## Test Procedure

1. Take TC-877's corpus and set its `results` member to record `passed` for `CAP-B`'s failing vector; run.
2. Run TC-877's corpus scoped to `CAP-A` and `CAP-E`.
3. Run it scoped to `CAP-Z`, which the corpus does not list.

Tag the tests `#[trace("TC-878", "FR-351-AC-n")]`.

## Expected Results

- Step 1: `CAP-B` `failed`.
- Step 2: exactly `CAP-A` `passed` and `CAP-E` `uncovered`.
- Step 3: the run refuses `unknown_required_feature`/`unknown-feature` naming `CAP-Z`, and no vector runs.

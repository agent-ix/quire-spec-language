---
id: TC-775
title: "Plugin-run failures settle as typed records of the plugin's own items"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-291
    type: verifies
---
# TC-775: Plugin-run failures settle as typed records of the plugin's own items

## Description

Verify the terminal records, categories and exit codes QSL gives each QSpec FR-462 plugin-run settlement, and that one plugin's failure leaves other providers' records unchanged.

Scope: FR-291-AC-1 to FR-291-AC-3.

## Test Procedure

1. For each FR-462 tool-failure event (crash, non-zero exit, malformed or non-canonical frame, foreign protocol identity, frame limit with limit 1024, unrouted item, unadvertised kind, missing record), build a `failed` record with that event's `ToolFailure` cause, serialize it (FR-286) and map its category through FR-285.
2. Build an `incomplete` record with cause `TimedOut` and one with cause cancelled; serialize each and map its category.
3. Build an outcome of two items, a `failed` record from step 1 and a compile-time provider's `proved` record, and an outcome holding the `proved` record alone; serialize both and map the first outcome's category.

Tag the tests `#[trace("TC-775", "<AC id>")]`.

## Expected Results

- Step 1: each document has category internal failure and names its event; the frame-limit document also names the frame limit and the value 1024; each exit code is 30.
- Step 2: each document has category incomplete; each exit code is 22.
- Step 3: the `proved` record's bytes are equal in both outcomes; the two-item outcome's exit code is 30.

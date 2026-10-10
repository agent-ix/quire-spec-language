---
id: TC-720
title: "A reached limit names its kind, bound, count and setting"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: verifies
---
# TC-720: A reached limit names its kind, bound, count and setting

## Description

Verify that every configurable limit in FR-255's setting table reports
the limit, its configured value, the count reached and the setting that
raises it, that the rendered diagnostic says how to raise it, and that each
limits type maps its fields to the table.

Scope: FR-255-AC-1, FR-255-AC-2, FR-255-AC-3.

## Test Procedure

1. For each row of FR-255's setting table, build the smallest input that
   reaches that limit at a configured bound `B` (for the I2 rows, a v2
   artifact; for the observation rows, a snapshot; for the library rows, a
   catalog), and run the row's stage.
2. Check `f`, whose body is `1 + 1 + 1`, with `s3.nodes` at 4, and render the
   diagnostic.
3. For each stage's limits type, read the setting name of each field. Set
   each field through its builder method to a value distinct from its
   default.

Tag the tests `#[trace("TC-720", "FR-255-AC-1")]`, `#[trace("TC-720", "FR-255-AC-2")]`, `#[trace("TC-720", "FR-255-AC-3")]`.

## Expected Results

- Step 1: each run returns its stage's limit outcome with the row's kind,
  bound `B`, the count the refused charge would have reached and the row's
  setting name; its catalog record carries `kind`, `bound`, `actual` and
  `setting`.
- Step 2: "S3 node limit 4 reached (5) at <locus>; raise it with
  `--limit s3.nodes=<n>` or the request's `stage_limits` entry `s3.nodes`",
  with FR-096's locus of the node whose entry failed.
- Step 3: each field maps to one name; the names are distinct; their union
  equals the table; each builder call changes only its own field.

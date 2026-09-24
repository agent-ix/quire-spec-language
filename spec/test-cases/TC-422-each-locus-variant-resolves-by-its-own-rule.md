---
id: TC-422
title: "Each Locus variant resolves to regions by its own rule, and the artifact pointer is RFC 6901"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-095
    type: verifies
---
# TC-422: Each Locus variant resolves to regions by its own rule, and the artifact pointer is RFC 6901

## Description

Verify ADR-013 T-5's `Locus`: a region locus is its own region, an
occurrence locus resolves through the package source map, and an artifact
locus names no region; its pointer is an RFC 6901 JSON pointer. This
catches an occurrence locus that bypasses the map and a pointer parser that
admits a bare `~`.

Scope: FR-095-AC-5, FR-095-AC-6.

## Test Procedure

1. Over a map holding one key with two regions, resolve a region locus, an
   occurrence locus at that key, an occurrence locus at the next ordinal and
   an artifact locus.
2. Parse `""`, `/`, `/source_map/0/regions`, `/a~1b/c~0d`, `source_map`,
   `/a~2b` and `/a~`.

Tag the tests `#[trace("TC-422", "FR-095-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: the region itself; the two regions; the unknown-occurrence cause;
  the artifact refusal.
- Step 2: the first four parse and keep their text; `source_map` refuses for
  a missing leading `/`; the last two refuse for the `~` at byte 2.

## Status

Passed locally. `qsl-foundation/src/diagnostic/locus.rs` backs both steps.

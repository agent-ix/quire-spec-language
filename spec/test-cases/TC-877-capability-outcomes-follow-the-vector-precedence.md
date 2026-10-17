---
id: TC-877
title: "Each capability settles from its vectors in the stated precedence, and an empty capability is uncovered"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-351
    type: verifies
---
# TC-877: Each capability settles from its vectors in the stated precedence, and an empty capability is uncovered

## Description

Verify the per-capability settlement order and the `uncovered` outcome.

Scope: FR-351-AC-1, FR-351-AC-2.

## Test Procedure

Build a fixture corpus with capabilities `CAP-A` (three passing vectors), `CAP-B` (one passing, one failing), `CAP-C` (one `unsupported`, one `incomplete` under a zero budget), `CAP-D` (one `tool-failure`, one failing), `CAP-E` (applicability: language, no vector) and `CAP-F` (applicability: a monitor subject only, one vector). Run over the default scope.

Give `CAP-A` through `CAP-E` a profile held by QSL's definition catalog.
Add `CAP-G`, whose capability applicability names `language` but only a
profile absent from that catalog. Include a vector for `CAP-G` whose source
does not exist, so accidental execution is observable as `tool-failure`.

Tag the tests `#[trace("TC-877", "FR-351-AC-n")]`.

## Expected Results

- `CAP-A` `passed`; `CAP-B` `failed` holding the failing vector's id; `CAP-C` `unsupported`; `CAP-D` `failed`; `CAP-E` `uncovered`.
- `CAP-F` appears in no outcome and its vector does not run.
- `CAP-G` appears in no outcome and its vector does not run.

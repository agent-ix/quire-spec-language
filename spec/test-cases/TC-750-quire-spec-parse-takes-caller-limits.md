---
id: TC-750
title: "quire-spec parse takes caller limits and parses a 100,000-deep source"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: verifies
---
# TC-750: quire-spec parse takes caller limits and parses a 100,000-deep source

## Description

Verify that the `quire-spec parse` command takes the root parser's three
limits from the command line and uses them as given.

Scope: FR-256-AC-5.

## Test Procedure

1. Write a source whose invariant body is 100,000 nested parentheses around
   `1`. Run `quire-spec parse` on it with no limit flags.
2. Run it again with `--source-bytes`, `--tokens` and `--nodes` raised to fit.
3. Run it with `--tokens` given no value, and with a non-numeric value.

Tag the test `#[trace("TC-750", "FR-256-AC-5")]`.

## Expected Results

- Step 1: exits 22 with `stage_limit_exceeded` and the exhausted ceiling in its message.
- Step 2: exits 0 and reports `"status":"parsed"`.
- Step 3: exits with the usage error.

## Status

Passed locally: `tests/it/deep_sources.rs`.

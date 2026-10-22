---
id: TC-750
title: "quire-spec parse takes caller limits and parses a 100,000-deep source"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: verifies
---
# TC-750: quire-spec parse takes caller limits and parses a 100,000-deep source

## Description

Verify that `quire-spec parse` takes S1 limits only as `--limit s1.<name>=<n>`
operands, uses each as given, and refuses every other limit operand as a
usage error.

Scope: FR-256-AC-5, FR-002-AC-8.

## Test Procedure

1. Write a complete-V1 source whose one function body is 100,000 nested
   parentheses around `1`. Run `quire-spec parse` on it with no `--limit`
   operand.
2. Run it again with `--limit s1.input_bytes=<n>`, `--limit s1.tokens=<n>`,
   `--limit s1.nodes=<n>` and `--limit s1.work_units=<n>`, each raised to fit.
3. Run `quire-spec parse` over `tests/fixtures/spine-compile.native` with
   `--limit s1.tokens=` set to the fixture's token count minus one, then to
   its token count.
4. Run it over the same fixture with, in turn, `--limit s3.nodes=1`,
   `--limit s1.tokens=ten`, `--source-bytes 10`, `--tokens 10` and
   `--nodes 10`.

Tag the tests `#[trace("TC-750", "FR-256-AC-5")]` (steps 1 and 2) and
`#[trace("TC-750", "FR-002-AC-8")]` (steps 3 and 4).

## Expected Results

- Step 1: exits 22; the outcome document's one diagnostic is
  `stage_limit_exceeded` naming the reached `s1.*` limit, its configured
  value, the count reached and its setting.
- Step 2: exits 0; the outcome document has category `success` and
  `last_stage` `S2`.
- Step 3: the first run exits 22 naming `s1.tokens` at the configured value;
  the second exits 0.
- Step 4: each run exits 20 with a usage refusal naming the operand, and
  writes no outcome document.

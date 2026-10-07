---
id: TC-749
title: "The root native parser parses 100,000-deep sources on a small stack"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: verifies
---
# TC-749: The root native parser parses 100,000-deep sources on a small stack

## Description

Verify that the root native parser, both editions, parses 100,000-deep inputs
under raised limits, without native recursion that grows with depth and
without a ceiling the caller cannot raise.

Scope: FR-256-AC-4.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack.

1. Through `parse`, with `source_bytes`, `tokens` and `nodes` raised to fit
   each input, parse an invariant body of: 100,000 nested parentheses around
   `1`; `not` applied 100,000 times to `true`; a `+` chain of 100,000 terms;
   an `else if` chain of 100,000 branches; and 100,000 nested `let … in`.
2. Through `parse_native_source`, parse the same five bodies in a composed
   invariant.
3. Through `admit_namespace`, admit a composed source with a 100,000-term
   `+` chain under raised parser limits, and under the defaults.

Tag the tests `#[trace("TC-749", "FR-256-AC-4")]`.

## Expected Results

- Step 3: the defaults refuse it, the raised limits admit it, and the report's
  `parser_limits()` equal the raised limits.
- Steps 1 and 2: each input parses, the thread completes, and the parsed unit has the shape the input has (one invariant clause, with at least as many expressions as the input has operators or brackets).

## Status

Passed locally: `tests/it/deep_sources.rs`.

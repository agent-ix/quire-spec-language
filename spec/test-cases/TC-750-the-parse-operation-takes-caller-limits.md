---
id: TC-750
title: "The parse operation takes caller limits and parses a 100,000-deep source"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: verifies
---
# TC-750: The parse operation takes caller limits and parses a 100,000-deep source

## Description

Verify that the `parse` operation runs S1 under exactly the `qsl_cst::Limits`
its caller gives it, that FR-255's settings operation builds those limits
from `s1.*` operands, and that each outcome's document carries FR-285's exit
code for its category.

Scope: FR-256-AC-5, FR-002-AC-8.

## Test Procedure

1. Build a complete-V1 source whose one function body is 100,000 nested
   parentheses around `1`. Call the `parse` operation on it under the limits
   FR-255's settings operation returns for no operand, and serialize the
   result through `OutcomeDocument::from_parse`.
2. Call it again under the limits the settings operation returns for
   `s1.input_bytes=<n>`, `s1.tokens=<n>`, `s1.nodes=<n>` and
   `s1.work_units=<n>`, each raised to fit.
3. Call the `parse` operation over `tests/fixtures/spine-compile.native`
   under the limits the settings operation returns for `s1.tokens=<n>`, with
   `n` the fixture's token count minus one, then with `n` its token count.
   Read the limits' fields.

Tag the tests `#[trace("TC-750", "FR-256-AC-5")]` (steps 1 and 2) and
`#[trace("TC-750", "FR-002-AC-8")]` (step 3).

## Expected Results

- Step 1: the settings operation returns the FR-255 defaults; the operation
  is refused with `stage_limit_exceeded` naming the reached `s1.*` limit, its
  configured value, the count reached and its setting; the document's exit
  code is 22.
- Step 2: the operation succeeds; the document has category `success`,
  `last_stage` `S2` and exit code 0.
- Step 3: the limits hold `s1.tokens` at `n` and every other S1 field at its
  default; the first call is refused naming `s1.tokens`, bound `n` and the
  count reached, exit code 22; the second succeeds.

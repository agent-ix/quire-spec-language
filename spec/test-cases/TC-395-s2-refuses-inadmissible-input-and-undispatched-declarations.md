---
id: TC-395
title: "S2 refuses an inadmissible source and a unit holding a declaration with no dispatch entry"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-395: S2 refuses an inadmissible source and a unit holding a declaration with no dispatch entry

## Description

Verify S2's two whole-unit refusals ahead of expression mapping:
`RecoveringCst` for an inadmissible source (ADR-011 §2.3 E2) and
`NoDispatchEntry` for a declaration whose leading token has no entry.

This catches a production that builds forms for the dispatchable
declarations and skips the rest, and one that admits a source carrying a
diagnostic but no recovery.

Steps 4 and 5 depend on FR-091-OQ-1.

Scope: FR-091-AC-4, FR-091-AC-5, FR-091-AC-6.

## Test Procedure

1. Parse a unit with a syntax error so the CST carries a recovery. Run S2.
2. Parse an admissible unit, then call `ParsedSource::prepend_diagnostic`
   so it carries a diagnostic and no recovery. Run S2.
3. Parse an admissible unit holding a valid `function` declaration followed
   by an `invariant ... using v on T::m at current { true }` declaration.
   Run S2.
4. For each of `enum`, `predicate`, `dimension` and `unit`, parse an
   admissible unit whose only declaration uses that keyword. Run S2.
5. Record the refusal's token spelling and span for steps 3 and 4.

Tag the test `#[trace("FR-091-AC-4", "FR-091-AC-5", "FR-091-AC-6", "TC-395")]`.

## Expected Results

- Steps 1 and 2 each refuse with cause `RecoveringCst` and return no
  parsed unit.
- Step 3 refuses with cause `NoDispatchEntry`, spelling `invariant` and the
  `invariant` declaration's span. No form is returned for the `function`.
- Step 4 refuses each with `NoDispatchEntry`, naming that keyword and the
  declaration's span.

## Status

Planned; no test backs this case.

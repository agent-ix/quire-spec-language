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

Verify S2's whole-unit refusals that come before expression mapping:
`RecoveringCst`, the diagnosed-source cause, and `NoDispatchEntry`.

This catches a production that builds forms for the dispatchable
declarations and skips the rest. It also catches one that admits a source
that carries a diagnostic but no recovery, or reports that source as
`RecoveringCst`.

Step 4 depends on FR-091-OQ-1.

Scope: FR-091-AC-4, FR-091-AC-5, FR-091-AC-6.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Parse a unit with a syntax error, so the CST carries a recovery. Run S2.
2. Parse an admissible unit and add one diagnostic with
   `ParsedSource::prepend_diagnostic`, so it carries a diagnostic and no
   recovery. Run S2.
3. Parse an admissible unit holding
   `function t using v(): Boolean pure { true }` followed by
   `invariant Positive using v on M::T at current { true }`. Run S2.
4. For each of `enum`, `predicate`, `dimension` and `unit`, parse an
   admissible unit whose only declaration uses that keyword, and run S2.

Tag the test `#[trace("FR-091-AC-4", "FR-091-AC-5", "FR-091-AC-6", "TC-395")]`.

## Expected Results

- Step 1 refuses with cause `RecoveringCst`.
- Step 2 refuses with the diagnosed-source cause, which is not
  `RecoveringCst` and holds the prepended diagnostic's code.
- Step 3 refuses with cause `NoDispatchEntry`, the spelling `invariant` and
  the `invariant` declaration's span. No form is returned for `t`.
- Step 4 refuses each unit with `NoDispatchEntry`, naming that keyword and
  the declaration's span.
- No step returns a parsed unit.

## Status

Planned; no test backs this case.

---
id: TC-392
title: "S2 returns one Value form per declaration, in source order, with its span and the unit edition"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-392: S2 returns one Value form per declaration, in source order, with its span and the unit edition

## Description

Verify that the S2 entry walks a unit's declarations in source order and
returns one `Value` parsed form per declaration, each carrying the span of
its `Declaration` CST node and the unit's edition, and that the unit carries
its profile selection as spelled.

This catches three faults: a production that dispatches on the unit root's
leading token (`language`) instead of each declaration's, one that reorders
or drops declarations, and one that gives every form the root span.

Scope: FR-091-AC-1.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Parse with `qsl_cst::parse` a unit holding, in order:
   `type Digit = Int[0, 9];`,
   `function inc using v(x: Digit): Int[0, 10] pure { x + 1 }`,
   `record Point { x: Int[0, 9]; }` and `tuple Pair(Int[0, 9], Int[0, 9]);`.
   Confirm `is_admissible()`.
2. Run the S2 entry on the result.
3. For each returned form, read its kind and span. Read the unit's edition
   and its selections.
4. Collect the spans of the four `Declaration` CST nodes and of the
   `Profile` CST node from the CST.

Tag the test `#[trace("FR-091-AC-1", "TC-392")]`.

## Expected Results

- Step 2 returns a parsed unit, not a refusal.
- Step 3 reads exactly four forms, in the order alias, function, record,
  tuple.
- Each form's span equals the matching span from step 4.
- The unit's edition reads `1-draft`.
- The unit's selections hold one profile selection, with alias `v`, its
  definition reference as written and the `Profile` span from step 4, and
  no import or model selection.

## Status

Backed by `qsl-forms/tests/it/value_forms.rs`: `a_unit_builds_one_form_per_declaration_in_source_order` (QSL-141).

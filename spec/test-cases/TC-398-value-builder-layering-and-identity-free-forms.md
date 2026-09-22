---
id: TC-398
title: "The Value form builder depends only on layer 2, layer 1 and F, and its forms hold no ValueType or NodeKey"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-398: The Value form builder depends only on layer 2, layer 1 and F, and its forms hold no ValueType or NodeKey

## Description

Verify the layering of the `Value` family form builder against ADR-011
§6.1's layer-2 allow-list ("1, F"), and that `Value` parsed forms hold no
typed or identity-bearing field (ADR-011 §2.2 E2 row). Also verify
FR-091-CON-2: there is no `_` arm at the three matches.

Use the resolved-import and definition-scan approach of TC-256, TC-170 and
TC-390. A `quire_exact` edge reached only through the `Integer`, `Rational`
or `Collection` payloads is reported separately under FR-091-OQ-8 and does
not fail this case.

Scope: FR-091-AC-11.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Resolve every `use` edge and inline path in the `Value` builder module
   under `src/forms/`.
2. Scan the field types of every `Value` parsed form type, of the type form,
   and of every `Expression` variant in FR-091's mapping table.
3. With `syn`, parse the dispatch function, the `Value` expression-mapping
   function and the assembler's type-form resolution function, and list
   each `match`'s arm patterns.

Tag the test `#[trace("FR-091-AC-11", "TC-398")]`.

## Expected Results

- Step 1 resolves only to the `forms` core, `qsl_cst` and `qsl_foundation`,
  apart from the FR-091-OQ-8 edge, and to nothing under another family
  module, `crate::check`, `crate::value` or `crate::model`.
- Step 2 finds no field of type `ValueType` or `NodeKey`, including
  `Expression::Convert`'s target and `FunctionDeclaration`'s parameters and
  result.
- Step 3 finds no `_` or catch-all arm.

## Status

Planned; no test backs this case.

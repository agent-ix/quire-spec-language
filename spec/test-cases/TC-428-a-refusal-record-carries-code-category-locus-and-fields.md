---
id: TC-428
title: "A refusal record carries its code, category, locus and the catalog's fields"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-428: A refusal record carries its code, category, locus and the catalog's fields

## Description

Verify FR-096's `RefusalRecord` and `catalog_fields`: a family refusal and a
kernel refusal at S6a each build a record with its code, category refusal,
the evaluation's resolved locus and the catalog payload, and every cause's
fields match the keys fixed for it. This catches a record that drops the
payload, reads it from a message, or invents a locus.

Scope: FR-096-AC-6, FR-096-AC-7, FR-096-AC-8.

## Test Procedure

1. Evaluate `lookup<T>(p, r) absent refused` with no member for `r`, and
   build the record from the `FamilyResult::Refused` cause.
2. Build a record from an `Evaluation` whose `location` is `None`.
3. For each cause in FR-096's key table, read `catalog_fields()`.
4. Build a record from an `Evaluation` whose outcome is a kernel `Refused`.

Tag the tests `#[trace("TC-428", "FR-096-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: code `invalid_runtime_input`/`absent-key`, category refusal,
  fields `binding` and `key` naming the population binding and the requested key, and the
  region of the `lookup` expression.
- Step 2: no locus.
- Step 3: each cause's fields hold exactly the keys the table lists for it.
- Step 4: the code QSL's kernel map gives the cause, category refusal and
  the evaluation's resolved locus.

## Status

Partial (QSL-245). Steps 1 to 3 pass locally (FR-096-AC-6, FR-096-AC-7).
Step 4 (FR-096-AC-8) is partly backed: `CardinalityOutOfBound` builds its
record, and `CheckedInvariant` is an internal fault, never a record. It is not
backed for `InexactDecimal`, `DecimalOutOfDomain`, `DivisionPairOutOfDomain`,
`ModuloOutOfDomain`, `TextLengthOutOfDomain`, `IntegerOutOfDomain`,
`RationalOutOfDomain` and `IeeeNotExact`, which have no catalog code yet.

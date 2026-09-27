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

Scope: FR-096-AC-6, FR-096-AC-7, FR-096-AC-8, FR-096-AC-13.

## Test Procedure

1. Evaluate `lookup<T>(p, r) absent refused` with no member for `r`, and
   build the record from the `FamilyResult::Refused` cause.
2. Build a record from an `Evaluation` whose `location` is `None`.
3. For each cause in FR-096's key table, read `catalog_fields()`.
4. For each of the twelve kernel causes in FR-096's key table, build a
   record from an `Evaluation` whose outcome is that kernel `Refused`, and
   read the cause's `Refusal::code()` and `Refusal::cause()`. Use the targets
   FR-096-AC-8 names: `Int[-5, 9]`, `Decimal[-100, 100; 0, 2]`,
   `Rational[-9, 9; 1, 9]`, `Text[1, 8; nfc]`, an integer target
   `Int[0, 9]` for `InexactDecimal`, a `binary32` `IeeeNotExact` whose
   `nearest-even` flags are inexact and overflow, and a `binary64` to
   `binary32` NaN conversion. Build one from a kernel `CheckedInvariant`.
5. Build a `DivisionPairOutOfDomain` record for consumer domain `Int[0, 9]`
   with only the quotient outside, only the remainder outside, and both
   outside.

Tag the tests `#[trace("TC-428", "FR-096-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: code `invalid_runtime_input`/`absent-key`, category refusal,
  fields `binding` and `key` naming the population binding and the requested key, and the
  region of the `lookup` expression.
- Step 2: no locus.
- Step 3: each cause's fields hold exactly the keys the table lists for it.
- Step 4: each record carries the key table's code and cause, category
  refusal, exactly the table's field keys and the evaluation's resolved
  locus; `code()` and `cause()` return the same code and cause. The fields
  read exactly `expected` `Int[-5, 9]`, `Decimal[-100, 100; 0, 2]`,
  `Rational[-9, 9; 1, 9]`, `Text[1, 8; nfc]` and `Int[0, 9]`; `expected`
  `binary32` with `flags` `overflow,inexact`; and `expected` `binary32` with
  `actual` `binary64`. `CheckedInvariant` builds no record.
- Step 5: causes `quotient-outside-domain`, `remainder-outside-domain` and
  `both-outside-domain`, each with `expected` `Int[0, 9]`.

## Status

Partial (QSL-245). Steps 1 to 3 pass locally (FR-096-AC-6, FR-096-AC-7).
Step 4 (FR-096-AC-8) passes for `CardinalityOutOfBound` and
`ForeignReference` (QSL-281); `CheckedInvariant` builds no record, and its
conversion to an internal fault is not built. Step 4 for the ten kernel
value refusals that catalog revision `1-draft.8` (QSpec STD-110) codes, and
step 5 (FR-096-AC-13), are planned: the variants do not yet carry their
target domain or width, and `kernel_refusal_record` builds no record for
them.

---
id: TC-482
title: "S2 builds dimension and unit forms"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-482: S2 builds dimension and unit forms

## Description

Verify the `dimension` and `unit` dispatch entries and their forms: a
dimension's terms with their operators and exponents, and a unit's
dimension name, scale, target and offset, each exact number carried as its
spelling kind and two kernel integers.

This catches an S2 production that reduces or normalizes (the assembler
does both), drops a term's operator, reads an absent exponent as a written
one, or loses a span.

Scope: FR-091-AC-31.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Run S1 and S2 on a unit declaring `dimension Length;`.
2. Run S1 and S2 on a unit declaring
   `dimension Accel = Length * Time^-2 / Mass;`.
3. Run S1 and S2 on a unit declaring
   `unit C : Temperature = rational(1, 1) * K + decimal(27315, 2);`.
4. Run S1 and S2 on a unit declaring `unit m : Length = rational(1, 1);`.

Tag the test `#[trace("FR-091-AC-31", "TC-482")]`.

## Expected Results

- Step 1: a dimension form named `Length` with no terms.
- Step 2: terms `Length` (no operator, no exponent), `Time` (`*`, exponent
  `-2`) and `Mass` (`/`, no exponent), each with the span of its name, and
  `Time`'s exponent with its span.
- Step 3: a unit form named `C`, dimension name `Temperature`, scale
  `rational` with parts `1` and `1`, target `K`, offset `decimal` with parts
  `27315` and `2`, each with its span.
- Step 4: a unit form with scale `rational` `1`, `1`, no target and no
  offset.

## Status

Not implemented. QSL-275 adds the entries and forms; today S2 refuses
`dimension` and `unit` with `NoDispatchEntry`.

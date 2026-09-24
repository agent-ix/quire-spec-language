---
id: TC-424
title: "parse and format take the four source labels and report the source reference"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: verifies
---
# TC-424: parse and format take the four source labels and report the source reference

## Description

Verify FR-010's `parse` and `format` grammar: the authority, identity,
revision namespace and revision value precede the file, the parse result
reports the source reference, and a missing, extra or empty operand exits
20.

Scope: FR-010-AC-11.

## Test Procedure

1. Run `parse agent-ix specs/a.quire git 3f2a <file>` over an admissible
   file.
2. Run `parse agent-ix specs/a.quire git 3f2a` with no file, and
   `parse agent-ix specs/a.quire git 3f2a <file> extra`.
3. Run `parse agent-ix specs/a.quire "" 3f2a <missing-file>`.
4. Run `format agent-ix specs/a.quire git 3f2a <file>` over admissible
   complete-V1 source, then again with one extra operand.

Tag the tests `#[trace("TC-424", "FR-010-AC-11")]`.

## Expected Results

- Step 1: exit 0; the result's source reads authority `agent-ix`, identity
  `specs/a.quire`, revision namespace `git`, value `3f2a` and the file's
  `quire.source.bytes/v1` digest.
- Step 2: both exit 20.
- Step 3: exit 20 with `invalid_source_identity`, not a file error.
- Step 4: exit 0, then exit 20.

## Status

Planned. ADR-013 §7 slice S-4b (QSL-233).

---
id: TC-425
title: "parse and format take the two source labels and report the source reference"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: verifies
---
# TC-425: parse and format take the two source labels and report the source reference

## Description

Verify FR-010's `parse` and `format` grammar: the authority and identity
precede the file, the parse result
reports the source reference, and a missing, extra or empty operand exits
20.

Scope: FR-010-AC-11.

## Test Procedure

1. Run `parse agent-ix specs/a.quire <file>` over an admissible file.
2. Run `parse agent-ix specs/a.quire` with no file, and
   `parse agent-ix specs/a.quire <file> extra`.
3. Run `parse agent-ix "" <missing-file>`.
4. Run `format agent-ix specs/a.quire <file>` over admissible
   complete-V1 source, then again with one extra operand.

Tag the tests `#[trace("TC-425", "FR-010-AC-11")]`.

## Expected Results

- Step 1: exit 0; the result's source reads authority `agent-ix`, identity
  `specs/a.quire` and the file's `quire.source.bytes/v1` digest, and no
  other member.
- Step 2: both exit 20.
- Step 3: exit 20 with `invalid_source_identity`, cause `blank-label`,
  `label` `identity`, not a file error.
- Step 4: exit 0, then exit 20.

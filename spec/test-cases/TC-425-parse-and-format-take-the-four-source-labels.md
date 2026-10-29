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

Call each operation in process, capturing the process streams.

1. Call the `parse` operation with authority `agent-ix` and identity
   `specs/a.quire` over an admissible file.
2. Call source intake with authority `agent-ix`, an empty identity and a
   missing path.
3. Call `format` with authority `agent-ix` and identity `specs/a.quire`
   over admissible complete-V1 source.

Tag the tests `#[trace("TC-425", "FR-010-AC-11")]`.

## Expected Results

- Step 1: exit code 0; the result's source reads authority `agent-ix`, identity
  `specs/a.quire`, and no other member.
- Step 2: exit code 20 with `invalid_source_identity`, cause `blank-label`,
  `label` `identity`, not a file error.
- Step 3: the formatted source, exit code 0.
- No step writes a byte to stdout or stderr.

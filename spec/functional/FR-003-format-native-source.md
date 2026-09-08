---
id: FR-003
title: "Format validated source without changing token meaning"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-001"
    type: implements
---
# FR-003: Format validated source without changing token meaning

## Description

When validated source is formatted, the formatter shall preserve every token spelling and comment.

## Inputs

Parsed unit and output budget.

## Outputs

Formatted UTF-8 source or output-budget incompleteness.

## Behavior

Formatting changes whitespace and normalizes layout to LF while retaining grouping and original token order. It writes stdout and leaves files untouched. The caller assigns a new source revision before using changed bytes in evidence. Output has a 1 MiB ceiling.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-003-AC-1 | Formatting preserves the ordered token spellings. | Test |
| FR-003-AC-2 | Formatting preserves comments. | Test |
| FR-003-AC-3 | A second formatting pass produces identical bytes. | Test |
| FR-003-AC-4 | Output beyond its selected ceiling receives resource_exhausted. | Test |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [Detailed contract or implementation evidence](../../src/format.rs) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.

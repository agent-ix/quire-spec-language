---
id: FR-003
title: "Format validated source without changing token meaning"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-001"
    type: traces_to
---
# FR-003: Format validated source without changing token meaning

## Description

When validated source is formatted, the formatter shall preserve every token spelling and comment.

## Inputs

Parsed unit and an optional selected output-byte ceiling.

## Outputs

Formatted UTF-8 source or output-budget incompleteness.

## Behavior

Formatting changes whitespace and normalizes layout to LF while retaining grouping and original token order. The library returns the complete string; the CLI writes it to stdout and leaves files untouched. The caller assigns a new source revision before using changed bytes in evidence.

The existing format(unit) API retains the default 1 MiB ceiling. An additive
format_with_limit(unit, output_bytes) API accepts lower limits, including zero;
larger values are clamped to the implementation ceiling. The byte limit is
inclusive and counts spaces, indentation, comments and the final newline.
Every append is checked before its growth; a refused operation returns no
partial string. Allocation capacity is an implementation detail and is not a
separate heap-accounting promise. Reuse the existing declarative token vocabulary.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-003-AC-1 | Formatting preserves the ordered token spellings. | Test |
| FR-003-AC-2 | Formatting preserves comments. | Test |
| FR-003-AC-3 | A second formatting pass produces identical bytes. | Test |
| FR-003-AC-4 | Output beyond its selected ceiling receives resource_exhausted. | Test |
| FR-003-AC-5 | Exactly-at-ceiling output succeeds, including the final newline; zero or one-byte-short ceilings refuse without returning a partial string. | Test |
| FR-003-AC-6 | format(unit) and format_with_limit(unit, 1048576) return identical bytes, and a larger selected ceiling cannot bypass the 1 MiB maximum. | Test |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [Detailed contract or implementation evidence](../../src/format.rs) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.

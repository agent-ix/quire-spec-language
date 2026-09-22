---
id: FR-003
title: "Format validated source without changing token meaning"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-001"
    type: traces_to
  - target: "ix://agent-ix/quire-spec-language/ADR-011"
    type: depends_on
---
# FR-003: Format validated source without changing token meaning

## Description

When validated source is formatted, the formatter shall preserve every token spelling and comment.

## Inputs

The S1 output `qsl_cst::ParsedSource` (its lossless CST and exact source)
and an optional selected output-byte ceiling. ADR-011 §6.1 places `format` in
the tool layer over S1, depending on layer 1 and F, and ADR-011 §6.2 and §7.3
(M-6a row) retarget it from the arena parse to the CST.

## Outputs

Formatted UTF-8 source or output-budget incompleteness.

## Behavior

Formatting changes whitespace and normalizes layout to LF while retaining grouping and original token order. The library returns the complete string; the CLI writes it to stdout and leaves files untouched. The caller assigns a new source revision before using changed bytes in evidence.

The format(source) API applies the default 1 MiB ceiling. The
format_with_limit(source, output_bytes) API accepts lower limits, including zero;
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
| FR-003-AC-6 | format(source) and format_with_limit(source, 1048576) return identical bytes, and a larger selected ceiling cannot bypass the 1 MiB maximum. | Test |
| FR-003-AC-7 | format and format_with_limit take a `qsl_cst::ParsedSource`. `src/format.rs` names no type from the arena `syntax` or native `parser` modules, and formatting complete-V1 source that the arena parser does not accept succeeds under AC-1 to AC-3. | Test (TC-404) |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 (tool layer), §6.2 (`format` row) and §7.3 (M-6a row) set the input
  to the CST. §2.3's E1 row lets the formatter consume a recovering CST.
- [Detailed contract or implementation evidence](../../src/format.rs) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.

The input is the CST (ADR-011 §7.3 M-6a). `src/format.rs` takes the arena
`ParsedUnit` today; AC-7 is unimplemented.

## Open Questions

- **FR-003-OQ-1: Does `format` accept a CST that carries a recovery, and
  what does it emit at a recovery site?** ADR-011 §2.3's E1 row lets the
  formatter consume a recovering CST ("only to tooling"), but does not
  require it. `complete::format_document` refuses any non-admissible CST.
  `qsl-cst` records recoveries as a side list, not as nodes in the tree.
  AC-1 to AC-7 are stated over admissible input.

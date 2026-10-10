---
id: FR-003
title: "Format validated source without changing token meaning"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-001"
    type: traces_to
  - target: "ix://agent-ix/quire-spec-language/ADR-011"
    type: depends_on
  - target: "ix://agent-ix/quire-spec-language/US-035"
    type: traces_to
  - target: "ix://agent-ix/quire-spec-language/FR-255"
    type: depends_on
---
# FR-003: Format validated source without changing token meaning

## Description

When validated source is formatted, the formatter shall preserve every token spelling and comment.

## Inputs

The S1 output `qsl_cst::ParsedSource` (its lossless CST and exact source)
and an optional selected output-byte ceiling. `format` is the layer-1
library operation `qsl_cst::format`, beside `cst` in crate `qsl-cst`,
depending on F only (ADR-011 §6.1 layer 1 and X-12; ADR-029 OP-1). ADR-011
§6.2 and §7.3 (M-6a row) retarget it from the arena parse to the CST.

## Outputs

Formatted UTF-8 source or output-budget incompleteness.

## Behavior

`format` admits a `ParsedSource` only when `ParsedSource::is_admissible()`
holds. It refuses any other input with a typed cause that holds the input's
first diagnostic code, and returns no output. ADR-011 §2.3's E1 row permits
a formatter to consume a recovering CST without requiring it, and this
requirement formats validated source only.

`format` formats complete-V1 source. A native-edition source (for example
edition `0-draft`) parses as a recovering CST that carries `unknown_edition`,
so `format` refuses it with that code and the CLI exits 20. Native-edition
`format` retires in M-6a with no replacement (ADR-011 §7.3 M-6a row;
ADR-011 Rulings 2026-09-22, native-edition `format`).

Formatting changes whitespace and normalizes layout to LF while retaining grouping and original token order. The library returns the complete string; the CLI writes it to stdout and leaves files untouched.

The format(source) API applies the published default output limit of 1 MiB. The
typed `FormatLimits.output_bytes` field is the `format.output_bytes` setting in
FR-255, and `format_with_limits(source, limits)` consumes that value.
`format_with_limit(source, output_bytes)` is the convenience form that builds
the same limits value. Both forms accept any limit, including zero and values
above the default, and use it as given. A refusal at the limit names
`format.output_bytes`, its value and `format_with_limit` as the way to raise it.
The byte limit is inclusive and counts spaces, indentation, comments and the
final newline.
Every append is checked before its growth; a refused operation returns no
partial string. Allocation capacity is an implementation detail and is not a
separate heap-accounting promise. Reuse the existing declarative token vocabulary.

Formatting keeps meaning: checking the formatted source yields the same
checked package identity as checking the original source.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-003-AC-1 | Formatting preserves the ordered token spellings. | Test |
| FR-003-AC-2 | Formatting preserves comments. | Test |
| FR-003-AC-3 | A second formatting pass produces identical bytes. | Test |
| FR-003-AC-4 | Output beyond its selected ceiling receives resource_exhausted. | Test |
| FR-003-AC-5 | Exactly-at-ceiling output succeeds, including the final newline; zero or one-byte-short ceilings refuse without returning a partial string. | Test |
| FR-003-AC-6 | format(source) and format_with_limit(source, 1048576) return identical bytes. Source whose formatted output exceeds 1 MiB refuses under format(source) naming the limit, its value and format_with_limit, and formats in full under format_with_limit with a limit raised to fit it. | Test |
| FR-003-AC-7 | format and format_with_limit take a `qsl_cst::ParsedSource`. `src/format.rs` names no type from the arena `syntax` or native `parser` modules, and formatting complete-V1 source that the arena parser does not accept succeeds under AC-1 to AC-3. | Test (TC-404) |
| FR-003-AC-8 | format and format_with_limit refuse a `ParsedSource` whose CST carries a recovery, and one that carries a diagnostic and no recovery, each with a typed cause and no output string; neither call panics. | Test (TC-404) |
| FR-003-AC-9 | For each complete-V1 fixture unit in the repository's tests that checks, checking its formatted source through the S1 to S4 spine yields a checked package identity equal to the original's, and formatting the formatted source again yields identical bytes. | Test (TC-885) |
| FR-003-AC-10 | `FormatLimits` maps its only field, `output_bytes`, to FR-255's `format.output_bytes` through `SettingLimits`; `with_output_bytes` changes only that field, and the settings operation's `format.output_bytes=<n>` operand supplies the resulting value to `format_with_limits`. | Test (TC-931) |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 (layer 1), §6.2 (`format` row) and §7.3 (M-6a row) set the input
  to the CST.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) owns the
  setting row and the shared limits/settings seam.
- [Detailed contract or implementation evidence](../../src/format.rs) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.

The input is the CST (ADR-011 §7.3 M-6a). `src/format.rs` takes
`qsl_cst::ParsedSource` and owns the one CST token walk, which
`complete::editor::format_document` also calls. AC-1 to AC-3 are backed by
TC-013, AC-4 to AC-6 by TC-016, AC-7 and AC-8 by TC-404, and AC-9 by
TC-885. AC-6 states a caller-raisable ceiling; `src/format.rs` clamps
every selected ceiling to 1 MiB today, so AC-6 is not met until that
clamp goes.

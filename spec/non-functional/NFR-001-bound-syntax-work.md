---
id: NFR-001
title: "Bound syntax work and storage"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-001"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-002"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-003"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-004"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/ADR-030"
    type: depends_on
---
# NFR-001: Bound syntax work and storage

## Statement

When the next syntax-processing operation would exceed a selected resource ceiling, the compiler shall return a `stage_limit_exceeded` diagnostic naming the ceiling's kind (`quire.native.diagnostics/v1`) without performing that operation.

## Scope

Native and complete-V1 source intake, tokenization and parsing, S2 form building (`qsl_forms::build_unit`), and native formatting and source-map validation.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Default source/output ceiling | 1048576 bytes | 1048576 bytes | negative-abuse-testing |
| Default token ceiling | 100000 tokens | 100000 tokens | negative-abuse-testing |
| Default syntax-node ceiling | 50000 nodes | 50000 nodes | negative-abuse-testing |
| Source-map segment ceiling | 50000 segments | 50000 segments | negative-abuse-testing |
| Parse time (S1 and S2 together) | Linear in source bytes and tokens, measured at 2,000, 8,000, 50,000 and 200,000 declarations with the S1 ceilings raised to fit | Time per declaration at 200,000 declarations within 2 times the time per declaration at 2,000 | benchmark |

## Nesting

This rule binds every S1 parser: the native parser
([FR-002](../functional/FR-002-parse-native-units.md)) and the complete-V1
lossless CST parser.

Bracket nesting, binary-operator chains, prefix-operator chains and `let … in`
and `if … else` chains are bounded only by the byte, token, syntax-node and
work ceilings. Each S1 parser keeps its open delimiters and productions on
explicit heap stacks, so every source within the selected ceilings returns a
parse result or a diagnostic, whatever its nesting
([FR-256](../functional/FR-256-parse-source-at-any-nesting-depth.md),
ADR-030 D-4.1). Each ceiling is named by its setting (`s1.input_bytes`,
`s1.tokens`, `s1.nodes`, `s1.work_units`;
[FR-255](../functional/FR-255-name-the-setting-that-raises-a-reached-limit.md)).

## Syntax node

The native parser counts one syntax node per expression or declaration node it
builds. The complete-V1 parser counts one syntax node per node of the lossless
CST it retains, including the nodes of every production the parse passes
through.

## Verification

Run the native boundary and malformed-input tests. On a 512 KiB stack, through
each S1 parser, check these chains, each built to the longest length the
default token and syntax-node ceilings admit: a flat `+` chain, a
right-associative `implies` chain, a prefix `not` chain, a `let … in` chain
and an `if … else` chain, and through the complete-V1 parser a temporal
`always` chain. Each parses. The same chain one element longer returns `stage_limit_exceeded`
naming the token or syntax-node ceiling. The native parser also completes a
20000-operator flat chain. Check that brackets nested 100,000 deep, including
by `Option<…>` type nesting, parse through the complete-V1 parser on the same
stack under ceilings raised to fit them (FR-256). Check that a work-budget
refusal names the work limit. A caller-supplied ceiling is used as given,
above or below the default; an implementation ceiling is not a domain
bound.

Formatter appends check prospective length, including its final newline, against
the inclusive selected ceiling. Byte limits bound output content, not allocator
capacity. Existing source-file intake may read one sentinel byte to detect
overflow; it never admits that oversized source. Native tests use the shared
canonical trace attribute and resolving TC/AC identifiers. The selected local
gate requires no hosted run or external producer qualification.

## Dependencies

- [FR-001](../functional/FR-001-read-exact-source.md)
- [FR-002](../functional/FR-002-parse-native-units.md)
- [FR-003](../functional/FR-003-format-native-source.md)
- [FR-004](../functional/FR-004-verify-source-maps.md)

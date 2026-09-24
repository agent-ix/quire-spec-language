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
---
# NFR-001: Bound syntax work and storage

## Statement

When the next syntax-processing operation would exceed a selected resource ceiling, the compiler shall return a resource_exhausted diagnostic without performing that operation.

## Scope

Native and complete-V1 source intake, tokenization and parsing, and native formatting and source-map validation.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Default source/output ceiling | 1048576 bytes | 1048576 bytes | negative-abuse-testing |
| Default token ceiling | 100000 tokens | 100000 tokens | negative-abuse-testing |
| Default syntax-node ceiling | 50000 nodes | 50000 nodes | negative-abuse-testing |
| Default nesting ceiling | 64 levels | 64 levels | negative-abuse-testing |
| Source-map segment ceiling | 50000 segments | 50000 segments | negative-abuse-testing |

## Nesting level

This definition binds every S1 parser: the native parser
([FR-002](../functional/FR-002-parse-native-units.md)) and the complete-V1
lossless CST parser.

One nesting level is one bracket pair: a delimiter pair `(` … `)`,
`[` … `]` or `{` … `}`, or the `<` … `>` of a type-argument list such as
`Option<T>` or `Set<T>[0, 1]`. A token's nesting depth is the number of
bracket pairs that enclose it. The nesting ceiling bounds that depth: a unit
whose deepest token sits inside `N` pairs is within a ceiling of `N`. The
opening bracket of pair `N + 1` returns resource_exhausted naming nesting
depth, the ceiling and that bracket's span, whether the lexer or the parser
detects it.

Nothing else counts as a level. Grammar productions, binary-operator chains
(`a + b + c`, `a implies b implies c`), prefix-operator chains (`not not x`,
`- - x`, temporal `always always p`) and `let … in` and `if … else` chains add
no nesting depth; their length is bounded by the token and syntax-node
ceilings. So `(((((x + x) + x) + x) + x) + x)` has nesting depth 5, and a flat
chain of any length has nesting depth 0.

Every source within the selected ceilings returns a parse result or a
resource_exhausted diagnostic. No source within them overflows the thread's
stack.

This is a different quantity from S2's expression depth
([FR-091](../functional/FR-091-produce-value-forms-and-assemble-package-declarations.md)
"S2 depth limit"), which counts `Expression` nodes in the parsed form: `((x))`
has S1 nesting depth 2 and S2 depth 1, and `not not x` has S1 nesting depth 0
and S2 depth 3. Each stage enforces its own bound.

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
`always` chain. Each has nesting depth 0
and parses. The same chain one element longer returns resource_exhausted
naming the token or syntax-node ceiling. The native parser also completes a
20000-operator flat chain. Check that a unit nested to exactly the ceiling
parses, including by `Option<…>` type nesting in the complete-V1
parser, and that one pair deeper is
refused as above. Check that a work-budget refusal names the work limit
rather than nesting depth. A caller-supplied ceiling is used as given, above
or below the default; an implementation ceiling is not a domain bound.

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

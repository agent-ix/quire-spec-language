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

Native source intake, tokenization, parsing, formatting and source-map validation.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Default source/output ceiling | 1048576 bytes | 1048576 bytes | negative-abuse-testing |
| Default token ceiling | 100000 tokens | 100000 tokens | negative-abuse-testing |
| Default syntax-node ceiling | 50000 nodes | 50000 nodes | negative-abuse-testing |
| Default delimiter-nesting ceiling | 64 levels | 64 levels | negative-abuse-testing |
| Source-map segment ceiling | 50000 segments | 50000 segments | negative-abuse-testing |

## Nesting level

One nesting level is one delimiter pair: `(` … `)`, `[` … `]` or `{` … `}`.
A token's nesting depth is the number of delimiter pairs that enclose it. The
delimiter-nesting ceiling bounds that depth: a unit whose deepest token sits
inside `N` pairs is within a ceiling of `N`, and the opening delimiter of pair
`N + 1` returns resource_exhausted naming nesting depth, the ceiling and that
delimiter's span.

Nothing else counts as a level. Grammar productions, binary-operator chains
(`a + b + c`, `a implies b implies c`) and prefix-operator chains
(`not not x`, `- - x`) add no nesting depth; their length is bounded by the
token and syntax-node ceilings. So `(((((x + x) + x) + x) + x) + x)` has
nesting depth 5, and a 20000-operator flat chain has nesting depth 0.

This is a different quantity from S2's expression depth
([FR-091](../functional/FR-091-produce-value-forms-and-assemble-package-declarations.md)
"S2 depth limit"), which counts `Expression` nodes in the parsed form: `((x))`
has S1 nesting depth 2 and S2 depth 1, and `not not x` has S1 nesting depth 0
and S2 depth 3. Each stage enforces its own bound.

## Verification

Run the native boundary and malformed-input tests. Check a 20000-operator flat chain, a 5000-operator right-associative `implies` chain and a 5000-operator prefix `not` chain on a 512 KiB stack; each has nesting depth 0 and parses at default ceilings. Check a unit nested to exactly the ceiling parses, one pair deeper is refused, and a work-budget refusal names the work limit rather than nesting depth. Caller-supplied ceilings may be lower; an implementation ceiling is not a domain bound.

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

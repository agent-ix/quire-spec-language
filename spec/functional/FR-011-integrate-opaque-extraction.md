---
id: FR-011
title: "Consume opaque extracted native clauses"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-004"
    type: traces_to
---
# FR-011: Consume opaque extracted native clauses

## Description

When an existing Quire adapter supplies a native clause, the compiler shall consume its exact source bindings without requiring a second expression compiler in extraction.

## Inputs

Existing extractor output, native language tag, authored obligation/source mapping and verified body map.

## Outputs

Source-addressable native clause outcome with extraction availability retained.

## Behavior

C owns changes to existing-repository adapters and wire adoption. A's optional
[Quire consumer](FR-030-consume-quire-extraction.md) can call the available Rust
extractor directly and verify its exact body against original bytes before
parse/link/lower. A namespaced unchecked language tag is not successful parsing.
Missing original source correspondence refuses the join. Existing consumer wire
versions and Quire availability remain unchanged.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-011-AC-1 | A verified extracted body reaches the native parser with its authored source mapping. | Test |
| FR-011-AC-2 | An unchecked native tag alone grants no parsed result. | Test |
| FR-011-AC-3 | A malformed original/body mapping refuses the adapter join. | Test |
| FR-011-AC-4 | An unsupported native clause retains its authored obligation identity. | Test |

## Dependencies

- [US-004](../usecase/US-004-reuse-existing-toolchain.md) supplies the user need.
- [Detailed contract or implementation evidence](../../docs/source-correspondence.md) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.

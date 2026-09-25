---
id: FR-002
title: "Parse the admitted native source grammar"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-001"
    type: traces_to
---
# FR-002: Parse the admitted native source grammar

## Description

When a source unit is parsed, the compiler shall construct located syntax for every admitted declaration and expression.

## Inputs

Validated source and token/node/depth budgets.

## Outputs

Immutable ParsedUnit or a located invalid/unsupported/incomplete diagnostic.

## Behavior

Token recognition is declarative. Expression precedence uses the approved grammar, with right-associative implication and nonchained comparisons. Grouping and each name/import literal retain exact spans. Parsing never establishes model linkage. Recovery does not create a partial successful unit.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-002-AC-1 | The parent example produces status parsed. | Test |
| FR-002-AC-2 | A chained comparison receives invalid_syntax. | Test |
| FR-002-AC-3 | A balanced reserved collect form receives unsupported_construct. | Test |
| FR-002-AC-4 | An exhausted syntax-node budget receives `stage_limit_exceeded`. | Test |
| FR-002-AC-5 | A grouped expression retains its delimiter-inclusive source span. | Test |
| FR-002-AC-6 | The bounded malformed/truncation corpus returns syntax or a located diagnostic without panic; any admitted variant retains its exact submitted bytes. | Test |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [Detailed contract or implementation evidence](../../src/parser.rs) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.

---
id: FR-040
title: "Admit state extensions through exact profile dependencies"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When admitting a state declaration or imported predicate, the frontend SHALL enforce the exact selected definition closure without combining permissions from unrelated profiles or installed backends.

## Inputs

Exact profile and dependency definitions, parsed state/query/graph constructs, checked predicate imports and authoritative model declarations.

## Outputs

A declaration admitted under one compatible definition closure, or a located profile/dependency refusal.

## Behavior

The state contract explicitly proposes reusable-predicate/query and finite-graph extensions beyond the retained first profile. They require distinct accepted definition identities; adding one does not change old definitions. A callee retains its own checked semantics across family imports. An incompatible duplicate definition, absent required dependency or unsupported construct refuses even in an unreachable branch. Runtime budgets and backend capabilities do not add source permissions.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-040-AC-1 | The first no-call/no-reference profile still refuses a helper call or graph navigation; the explicitly selected reviewed extension admits its supported typed form. | Test (TC-040) |
| FR-040-AC-2 | A caller with a wider local profile cannot grant an imported predicate operations absent from that predicate's selected definition. | Test (TC-040) |
| FR-040-AC-3 | Unknown, conflicting or missing required definitions refuse before any dependent checked clause is published. | Test (TC-040) |
| FR-040-AC-4 | An unreachable prohibited construct still refuses, while old accepted profile bytes and fixture identities remain unchanged. | Test (TC-040) |

## Dependencies

- [Detailed draft contract](../../proposals/quire-v1/state-contract.md).
- [Shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed composed-v1 obligation. Historical definitions and their acceptance
remain separate; this artifact does not establish implementation coverage.

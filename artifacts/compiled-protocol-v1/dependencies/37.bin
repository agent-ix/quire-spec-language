---
id: FR-045
title: "Preserve conditional evaluation and exact optional-value facts"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When checking and evaluating a state control expression, the implementation SHALL preserve the selected branch and exact observation-qualified guard facts defined by the native expression.

## Inputs

A typed control expression, exact optional-value identities, immutable lexical bindings and execution/probe capability information.

## Outputs

The specified result and available source activation facts, or a typed definedness/refusal/incomplete outcome.

## Behavior

And/or/implies evaluate their left operand once and skip right for false/true/false respectively. If evaluates only the selected branch; let initializes once. Static name/type/profile admission still covers unselected syntax. Optional access is justified only by a sound guard for the same immutable optional value/field/observation; lexical aliases preserve that identity when they preserve the value. A Boolean result does not establish that an implication consequent ran, and missing probes cannot be reported as observed inactivity.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-045-AC-1 | False-and, true-or and false-implies skip right and return false, true and true respectively; reached right operands retain their actual evaluation facts. | Test (TC-045) |
| FR-045-AC-2 | If selects one branch and let initializes once even when its binding is read repeatedly; resource exhaustion never creates a completed result. | Test (TC-045) |
| FR-045-AC-3 | A guard can justify an alias of the same immutable optional value, but not a same-named field on another object or observation. | Test (TC-045) |
| FR-045-AC-4 | An unknown name or prohibited source construct in an unselected branch still refuses static admission. | Test (TC-045) |
| FR-045-AC-5 | A completed vacuous implication does not claim consequent participation; an unavailable activation probe remains unavailable. | Test (TC-045) |

## Dependencies

- [Detailed draft contract](../../proposals/quire-v1/state-contract.md).
- [Shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed composed-v1 obligation. Historical definitions and their acceptance
remain separate; this artifact does not establish implementation coverage.

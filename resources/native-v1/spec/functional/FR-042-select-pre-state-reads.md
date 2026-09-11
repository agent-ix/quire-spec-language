---
id: FR-042
title: "Select pre-state reads without retagging captured values"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When binding a postcondition pre(expression), the checker SHALL select that invocation's pre observation for eligible state reads inside the expression while preserving lexical and invocation-value provenance.

## Inputs

An exact postcondition invocation environment, located expression, lexical captures, declared operation parameters/result and pre/post snapshot identities.

## Outputs

A source-bound pre-selected expression or a typed anchor/provenance refusal.

## Behavior

The state contract's anchor table and correspondence examples define the selection. Pre is refused in invariants/preconditions, on bare parameters/constants/captures, and for post-only result access. Pure immutable parameters may accompany a permitted pre state read in a composite expression. A let initialized outside pre is not re-executed, and a post-qualified captured value/reference cannot be retagged. A let inside pre reads that selected observation. Nested eligible pre is idempotent. Presence facts never cross observation identity.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-042-AC-1 | pre(self.version + delta) and pre(P(self.version, delta)) bind the field to pre and delta to the same immutable invocation parameter. | Test (TC-042) |
| FR-042-AC-2 | pre(let v = self.version in v + delta) reads pre; pre(pre(self.version)) retains the same observation and original selector loci. | Test (TC-042) |
| FR-042-AC-3 | let v = self.version in pre(v) and let s = self in pre(s.version) refuse instead of replaying or retagging post captures. | Test (TC-042) |
| FR-042-AC-4 | pre(delta), pre(result), invariant/precondition historical access and a pre guard used for a post optional value refuse with their exact cause and source. | Test (TC-042) |
| FR-042-AC-5 | A pre-qualified reference keeps pre navigation after deletion in post; a post reference cannot become pre merely because its object ID matches. | Test (TC-042) |

## Dependencies

- [Detailed draft contract](../../proposals/quire-v1/state-contract.md).
- [Shared grammar](../../proposals/quire-v1/shared-grammar.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed composed-v1 obligation. Historical definitions and their acceptance
remain separate; this artifact does not establish implementation coverage.

---
id: FR-034
title: "Bind cross-family predicates to explicit evaluation environments"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-005
    type: implements
---
## Description

When a temporal or protocol clause references a checked state predicate, the linker SHALL preserve the predicate's exact model, argument, anchor and immutable capture bindings at that reference.

## Inputs

Checked predicate identities, family references, argument declarations, operation/snapshot anchors and declared capture environments.

## Outputs

A typed family reference carrying the resolved environment, or a located incompatible-binding refusal.

## Behavior

A state predicate remains a Boolean expression evaluated in its declared environment. Temporal/protocol status cannot become one of its Boolean arguments. The same textual name in another model or invocation does not satisfy identity. Capture values are immutable for the lifetime of their obligation; later snapshots cannot replace them. Caller and callee must agree on current/pre/post and invocation identity where applicable. Family-specific capture timing is specified by its owner and cannot be guessed by an adapter.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-034-AC-1 | State, temporal and protocol references preserve their shared predicate declaration and each exact argument/anchor/capture environment. | Test (TC-034) |
| FR-034-AC-2 | A same-named declaration from another model or a pre-state from another operation invocation refuses binding. | Test (TC-034) |
| FR-034-AC-3 | Changing later observed state does not replace an obligation's captured value. | Test (TC-034) |
| FR-034-AC-4 | A pending temporal or protocol result used as an ordinary Boolean argument refuses type checking. | Test (TC-034) |

## Dependencies

- [Shared drafting foundation](../../proposals/quire-v1/shared-foundation.md).
- [Composed architecture](../assurance/AD-001-composed-native-language.md).
- [Planned acceptance matrix](../composed-foundation/tests.md).

Proposed requirement for the composed profile; it does not amend historical
accepted definitions or establish an implemented capability.

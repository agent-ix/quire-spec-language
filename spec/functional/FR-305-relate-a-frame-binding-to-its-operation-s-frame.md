---
id: FR-305
title: "Relate a frame binding to its operation's frame and anchor"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-031
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-012
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-353
    type: depends_on
---
# FR-305: Relate a frame binding to its operation's frame and anchor

## Description

When the S3 checker checks a `FrameBinding` keyed by an `OperationKey`, the
checker SHALL accept the binding whether or not a clause or attempt names the
operation. When a clause or attempt names the operation, the checker SHALL
relate the binding to that operation's one `state`/`frame` node and one
`state`/`operation_anchor` node.

## Inputs

- A `FrameBinding` of a `CheckedAbstractionRelation`, keyed by `OperationKey
  { declaring: DeclarationKey, operation: Identifier }`
  ([FR-304](FR-304-check-an-authored-abstraction-relation.md)).
- The unit's `CheckedOperationFrame`s, one per (declaring type, operation
  name) that a clause or attempt names ([FR-104](FR-104-check-state-clauses.md),
  [FR-114](FR-114-bind-a-protocol-attempt-to-its-operation-frame.md)).

## Outputs

The binding, related to the operation's frame node and anchor node when the
unit has them.

## Behavior

### The operation key determines the frame identities

An operation has one anchor and one frame
([FR-105](FR-105-emit-state-nodes.md)). Its QSpec FR-013 frame identity and
its QSpec FR-012 pre, post and result anchor identities are functions of its
`OperationKey` (ADR-017 AR-2), so the binding's key carries no anchor member.
The bound function's entry is the pre anchor, its exit the post anchor, and
its return value the result. The operation's parameters are read at entry
through the binding's `parameters`, the framed state at entry and exit
through its `receiver`, a framed field through its object type's
`ObjectBinding.fields`, and a created or deleted object through its
population's `PopulationBinding`. The key derivation is the one STD-121 (Q-7)
records for QSpec FR-353-AC-1.

### Inherited operations bind at their declaring type

An inherited operation anchors at its declaring type (FR-105), so its
`OperationKey` names the declaring type. A binding whose `OperationKey`
names a subtype that inherits the operation without declaring it is keyed by
a type that declares no such operation, and refuses as FR-304 states
(`missing_declaration`/`missing-name`).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-305-AC-1 | A unit that binds the frame of an operation no clause or attempt names checks, and its checked relation holds that `FrameBinding` under the operation's `OperationKey`. | Test (TC-802) |
| FR-305-AC-2 | A unit that binds `attemptUpdate`'s frame and has a clause naming `attemptUpdate` relates the binding to exactly the `state`/`frame` node and the `state`/`operation_anchor` node of `attemptUpdate`'s `CheckedOperationFrame`. | Test (TC-802) |
| FR-305-AC-3 | A frame binding keyed by `OperationKey { Sub, op }`, where `Sub` inherits `op` from `Base` without redeclaring it, refuses `missing_declaration`/`missing-name` naming the key; the same binding keyed by `OperationKey { Base, op }` checks. | Test (TC-803) |

## Dependencies

- **Upstream:** QSpec FR-353, FR-012 and FR-013 own the frame and anchor
  identities; STD-121 (Q-7) records the derived key for QSpec FR-353-AC-1;
  ADR-017 AR-2 fixes the key; [FR-105](FR-105-emit-state-nodes.md) gives one
  anchor and one frame per operation.
- **Downstream:** [FR-307](FR-307-export-the-bindings-each-item-references.md)
  returns the frame binding for a frame, precondition or postcondition item.

## References

- ADR-017 §3 AR-2 ("Anchors", "Frame identity").
- QSpec STD-121 (Q-7).
- Linear QSL-388 (specification), QSL-36 (implementation).

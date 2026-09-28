---
id: FR-114
title: "Bind a protocol attempt to its operation's anchor and frame"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-006
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-340
    type: depends_on
---
# FR-114: Bind a protocol attempt to its operation's anchor and frame

## Description

A protocol operation is an `attempt N by R on M::T::op contracts [...]`
event node. Its frame is the frame of the model operation it attempts: the
`modifies`, `creates` and `deletes` that FR-103 admits from the domain
package into the operation's `OperationEffect`. QSpec gives an attempt no
frame of its own: an attempt runs "without bypassing the operation's
model/frame contract" (`choreography-surface.md`), and "a caller cannot
substitute a more permissive frame" (`state-contract.md`, FR-013).

When S3 checks an attempt, the `ProtocolClause` family SHALL resolve its
operation, bind it to the operation's one anchor and one frame, and check
each `contracts` entry against that anchor. S4 SHALL name the frame by
FR-105's `state`/`frame` node. There is one frame node per (declaring object
type, operation name), however many clauses and attempts name the operation
(FR-105); an attempt adds no second frame node and no second frame
requirement record.

## Inputs

- The attempt's form: its name, role, operation name `M::T::op` and
  `contracts` entries, each with its span.
- The assembled `PackageDeclarations` with the operations FR-103 admits, and
  the unit's checked state clauses (FR-104).

## Outputs

- A checked attempt holding the identity of its operation's
  `state`/`operation_anchor` node and `state`/`frame` node (FR-105), and the
  identity of each state clause its `contracts` list names, in list order.
- Or the refusals below.

## Behavior

- The checker SHALL resolve `M::T::op` exactly as FR-104's Resolution
  resolves a `pre` or `post` clause's operation: `missing_declaration`/
  `missing-name` at the missing name, and `ambiguous_declaration`/
  `ambiguous-name` at `op` when two ancestors declare it and neither
  declaring type is more derived. An operation inherited by a subtype binds
  the anchor and frame of its declaring type (FR-105).
- The checker SHALL record the attempt's frame only by the frame node's
  identity, and SHALL NOT copy the frame's entries into the attempt.
- If a `contracts` entry names no state clause of the unit, then the
  checker SHALL refuse `missing_declaration`/`missing-name` at the entry.
- If a `contracts` entry names an invariant, or a `pre` or `post` clause
  anchored at another operation anchor, then the checker SHALL refuse
  `wrong_snapshot`/`wrong-anchor` at the entry, naming the attempt's anchor
  and the clause's anchor (QSpec: "Listed state pre/postcondition
  declarations must match this operation and its invocation anchors").
- The checker SHALL admit an empty `contracts` list, and the attempt still
  binds its operation's anchor and frame.
- S4 and the `requirements` hook SHALL treat an operation that a protocol
  attempt names as they treat one a `pre` or `post` clause names (FR-104,
  FR-105). An operation named only by an attempt still gets its one anchor,
  one frame and one `operation-contract` frame record.
- S4 SHALL give the operation's anchor node one `anchor` occurrence per
  clause or attempt that names the operation, ordinals in source order
  (FR-105's Outputs table).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-114-AC-1 | Over the ConfigVersion package (TC-458 fixture), a protocol with `attempt Update by R on Config::ConfigVersion::attemptUpdate contracts [VersionUnchanged]` checks. Its checked attempt holds the identity of `attemptUpdate`'s operation anchor node, of its frame node (whose `modifies` is exactly the `versionNumber` field) and of `VersionUnchanged`'s clause node. The compiled package holds exactly one anchor node and one frame node for `attemptUpdate`, and exactly one `operation-contract` record for the frame. | Test (TC-513) |
| FR-114-AC-2 | The same unit with `VersionUnchanged` removed, so only the attempt names `attemptUpdate` (with `contracts []`), still gets exactly one anchor node, one frame node and one frame record for `attemptUpdate`. | Test (TC-513) |
| FR-114-AC-3 | `on Config::ConfigVersion::missing` refuses `missing_declaration`/`missing-name` at `missing`; `contracts [Absent]` refuses `missing_declaration`/`missing-name` at the entry; `contracts [ParentOrder]` (an invariant) and `contracts [ProbePre]` (a `pre` clause of `probe`) each refuse `wrong_snapshot`/`wrong-anchor` at the entry, naming both anchors. | Test (TC-513) |
| FR-114-AC-4 | Over a package where `Sub` specializes `ConfigVersion`, an attempt on `Config::Sub::attemptUpdate` and a `post` clause on `Config::ConfigVersion::attemptUpdate` share one anchor node and one frame node, whose context is `ConfigVersion`. | Test (TC-513) |

## Dependencies

- FR-103 (operations and their frames at I1 and E3), FR-104 (operation
  resolution, frame requirement records), FR-105 (the one anchor and frame
  node per operation), FR-113 (the protocol's scope check).
- QSpec FR-013 (a caller cannot enlarge a frame), FR-340 (frame body),
  `choreography-surface.md` (the `attempt` row), `state-contract.md`.
- STD-111 (QSpec) for the emitted spellings, as FR-105 states.

## Status

Specified under QSL-296 (QSL-21a). Implemented under QSL-309 (QSL-21c): the
assembler resolves an attempt's operation as FR-104 resolves a clause's.
S3 checks the `contracts` list, and refuses a list entry that names two
state clauses of one name. S4 binds the operation's one anchor and frame
node, and gives each clause or attempt naming the operation an `anchor`
occurrence, with ordinals in source order. The attempt's frame record is
computed by the same function a clause's frame record uses. An operation
named only by an attempt emits its anchor, frame and frame record. AC-1 to
AC-4 are tested (TC-513), including through `emit_checked`.

A protocol holding the attempt compiles only when every other part of it is
checked (FR-113 Status lists what is and what still refuses
`unsupported_construct`/`not-yet-implemented`). So TC-513's own fixture
body `{ updated }`, which reads the binder, still refuses until protocol
bodies are checked. The tests use `{ true }`. The emitted spellings follow
FR-105's and are pending STD-111, as FR-105's are.

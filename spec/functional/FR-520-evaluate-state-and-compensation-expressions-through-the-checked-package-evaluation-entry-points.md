---
id: FR-520
title: "Evaluate state and compensation expressions through the checked-package evaluation entry points"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-500
    type: references
---
# FR-520: Evaluate state and compensation expressions through the checked-package evaluation entry points

## Description

quire-protocol's compensation engine (`src/compensation/graph.rs`, tested by
`tests/tc_003_graph_compensation.rs`) and its assessment (`src/assessment.rs`)
evaluate authored expressions through `quire_spec_language::state::{StateView,
EvaluationReport, evaluate_v2, ACCOUNTING_VERSION}`: the guard, retry relation and
recovery predicate of each compensation, captures, and the activation guard. QSL-834
deleted that module. The replacement is the qsl-eval entry points over a strictly
read checked package (`docs/compiled-protocol-v2.md`): `CheckedPackageEvaluation::call`,
`evaluate` and `evaluate_protocol_slot` (FR-390). They are not a drop-in: there is
no caller-supplied artifact graph, no `StateView`, and no `EvaluationReport` with
per-binder accounting; the artifact-graph path
`state::Refusal::UnpublishedArtifact` is retired by design (FR-049).

This requirement gives every behavior quire-protocol relies on a successor
criterion, states the mapping from the old outcomes to the new ones, and divides
the tc_003 behaviors between the QSL evaluation they depend on and the
quire-protocol engine that owns the rest.

## Semantic authority and boundary

QSL owns expression evaluation (FR-390, FR-107). quire-protocol owns the
compensation state machine (registration, ordering, attempts, retry bound as a
count, recovery decision) and builds the supply from its observation views. QSL
never coerces an undefined, refused, incomplete or unavailable result to a
Boolean, and the engine must not either (FR-024-AC-6 of quire-protocol).

## Inputs

A `CheckedPackageEvaluation` over the package read with
`read_checked_package` (the retained `V2Read` view, FR-390), a `ProtocolSlot`, a
`ProtocolSupply` and a `Meter`; for functions and predicates of the model,
`call(name, args)` and `evaluate`.

## Outputs

`Result<ProtocolSlotEvaluation, CallFailure>` (FR-390).

## Behavior

### Outcome map

| Retired outcome (`graph.rs`) | Successor |
| --- | --- |
| guard, retry relation or recovery predicate evaluated true | `Evaluated(Completed(true))` |
| `GuardFalse(report)`, `RetryNotAuthorized(report)` | `Evaluated(Completed(false))` of `trigger.guard`, `retry.relation` |
| `Unavailable(MissingEvidence)`, `UnavailableAfterEvaluation` | `ProtocolSlotEvaluation::Unavailable` (the binder, its cause and the read location) or `Evaluated(Undefined)` with its cause |
| `Source(report)` | `CallFailure::Admission` with the original admission record (nothing is charged) or `Evaluated(Refused)` |
| `Unsupported(report)` | `Evaluated` with the family's unsupported result (exit 21) |
| resource-incomplete | `Evaluated(Incomplete)` naming the counter |
| `Refusal::UnpublishedArtifact`, `state::Refusal` artifact-graph causes | retired (FR-049): no successor, and no call can name an artifact |
| `EvaluationReport` per-binder accounting, `ACCOUNTING_VERSION` | the `Meter` charge log of the ten counters FR-255 names; no per-binder split |
| `StateView` (binders, populations, objects, authority adapter) | `ProtocolSupply { binders, observation }`; references and populations through `ObservationAdmission` and `ProtocolReferenceOccurrence` (FR-390) |
| the registration and activation captures | `registration_captures` and `activation_captures` slots, one call each |

### tc_003 behaviors

| quire-protocol criteria (tc_003) | Owner | QSL criterion |
| --- | --- | --- |
| FR-020-AC-1 to AC-5 (one registration per paired effect, replay returns it, no invocation) | engine | the `registration_captures` slot is a pure function of its supply: equal supply gives equal value and equal charges (AC-3); the engine's no-op replay needs nothing else |
| FR-021-AC-1 to AC-4 (ordering violations from QObs positions) | engine | none: no expression is evaluated |
| FR-022-AC-1 to AC-5 (finite retry bound, attempts maximum counts the initial attempt) | engine for the count, QSL for the relation | the bound is the node's `attempts maximum` argument, read through FR-519's `protocol_view`; the `retry.relation` slot over two attempts (AC-4) |
| FR-023-AC-1 to AC-5 (attempt and effect distinctness) | engine | none |
| FR-024-AC-1 to AC-6 (recovery only by the authored relation) | QSL for each relation | AC-5 and AC-6 below |
| FR-003-AC-6, AC-7 (missing authority leaves no success; no I/O) | engine plus QSL | AC-6 and AC-8 below |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-520-AC-1 | For every outcome row of the map, a test over **R** (FR-517) produces the successor outcome: true and false `trigger.guard`, `retry.relation` and `recovery.predicate`; a binder supplied `Unavailable`; a supplied value of the wrong type; a construct the family reports as unsupported; a meter one below measured usage. | Test (TC-1315) |
| FR-520-AC-2 | The registration capture of **R**'s `Full` template evaluated from a payment record gives the captured total; the same call with an equal supply gives an equal value and an equal charge log; a retry or recovery call whose supply holds that captured value returns the same result for a payment record of total 12 as for one of total 10. | Test (TC-1315) |
| FR-520-AC-3 | With a captured target of 10 and receipts `[3, 4]`, `Partial`'s `recovery.predicate` is true and `Full`'s false; with `[4, 6]`, `Full` true and `Partial` false; with `[]`, both false; with `[5, 6]`, both false; neither relation ranks above the other, and the arithmetic is exact. | Test (TC-1315) |
| FR-520-AC-4 | `retry.relation` over a first and a second attempt record is a Boolean from the authored relation alone; the node's `attempts maximum` read through `protocol_view` equals the authored count, and a bound of `n` admits `n` attempts at the engine (FR-022-AC-2). | Test (TC-1315) |
| FR-520-AC-5 | A recovery success requires the exact relation, the exact captured target and the exact achieved state: varying any one of the three (the other relation's slot, a target of 11, receipts `[3, 5]` against `[3, 4]`) changes the result from true, and no call returns true from a successful compensating-operation record or an attempt outcome `successful` alone. | Test (TC-1315) |
| FR-520-AC-6 | A recovery supply with a reference to a population that is incomplete, a missing target state, a missing achieved state or a binder supplied `Unavailable` returns `CallFailure::Admission`, `Unavailable` or `Evaluated(Incomplete)` and never `Completed(true)`; `ProtocolSlotEvaluation` and `Evaluation` expose no `as_bool`, `Into<bool>` or `unwrap_or(false)` convenience (a compile-fail test). | Test (TC-1315) |
| FR-520-AC-7 | Three slot calls that share one `Meter` leave a charge log equal to the sum of the same three calls on fresh meters, over the ten counters of FR-255; the exact-usage and one-below vectors of FR-390-AC-9 and AC-10 hold for **R**'s `retry.relation`. | Test (TC-1315) |
| FR-520-AC-8 | `evaluate_protocol_slot` and `call` perform no network, storage or business-operation I/O (FR-025 of quire-protocol): the evaluation crates have no I/O dependency (`cargo tree`) and a run under a denied-syscall harness completes. | Test (TC-1315) |
| FR-520-AC-9 | The result for the in-process package equals the result for the package read back with the reader, charge log included, for each slot of **R**'s two compensations (FR-390's equality rule). | Test (TC-1315) |
| FR-520-AC-10 | After QSL-834's deletion of `state::*`, no QSL export named `StateView`, `EvaluationReport`, `evaluate_v2` or `ACCOUNTING_VERSION` exists, and the outcome map above is the only place the loss of per-binder accounting is stated. | Inspection (TC-1315) |

## Dependencies

- **Upstream:** FR-390 (and QSL-852's open gaps AC-13, AC-15, AC-16, AC-18 to
  AC-22, AC-24), FR-255, FR-383-AC-8, FR-519, FR-517.
- **Downstream:** quire-protocol `src/compensation/graph.rs`,
  `tests/tc_003_graph_compensation.rs`, `tests/support/v2_state.rs` (QSL-1549),
  which keeps the same assertions on the engine behaviors it owns and moves the
  report-based assertions onto the outcome map.

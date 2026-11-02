---
id: FR-524
title: "Create one temporal instance per semantic trigger or execution origin with immutable captures, and name every activation fault"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-053
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
  - target: ix://agent-ix/quire-specification/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-094
    type: references
---
# FR-524: Create one temporal instance per semantic trigger or execution origin with immutable captures, and name every activation fault

## Description

The spine temporal evaluator SHALL create one instance per distinct semantic
trigger identity or admitted execution-origin identity. The evaluator evaluates
the clause's captures once at the activation anchor and keeps them unchanged. For
an activation that cannot be established, it reports the fault and the input the
fault names. For a trigger scope that creates no instance it reports one of four
dispositions, and none of the four carries a truth value.

This is the spine successor of FR-044-AC-1 to AC-9 and the activation half of
FR-052-AC-10 and FR-053-AC-7 to AC-9. It adds the instance model that
`qsl-eval`'s `on each` and `on origin` activation (FR-327) does not have: the
instance is keyed by the opaque trigger identity, not by a position.

## Use case

A payment monitor receives the same refund trigger twice, once through each of
two relays, then a second refund for the same amount. The evaluator keeps one
instance for the first refund with both relay receipts as provenance, and a
second instance for the second refund even though its amount is equal. When one
relay then delivers a different amount under the first refund's identity, the
evaluator refuses and names that identity, and the retained amount does not
change.

## Semantic authority and boundary

QSpec FR-093 and FR-094 own activation, guard and capture meaning. FR-053 owns
the opaque trigger identity, and FR-516 carries one delivery per call.
FR-516 cannot express a repeated delivery in one call, so the multi-delivery
rules live in the activation function below; the evidence call applies it to its
one delivery, and a caller that holds several deliveries makes one call per
instance (an instance is identified by trigger identity, FR-053). This reading of
FR-516-AC-4's redelivery vectors is an inference that FR-524-AC-2 and AC-3 make
testable.

## Inputs

`qsl_eval::temporal::activation::activate`, over the clause's `Activation`
(FR-515: `Origin`, or `Each { trigger, guard }`) and its ordered captures, with an
`ActivationInput`:

- `scope`: the trigger scope's closure (`Closed` or `Open`) and its trigger
  evidence (`Admitted`, `Missing` or `Refused`), independent of the decision
  scope;
- `deliveries`: for `Each`, a `Vec<TriggerDelivery>`, each `{ identity:
  SemanticTriggerIdentity, receipt, anchor, payload, guard: Option<bool>,
  captures: Vec<CaptureFact> }`;
- `origins`: for `Origin`, a `Vec<ExecutionOrigin>`, each `{ identity, payload }`;
- `anchor`: the evaluation anchor;
- `evicted`: the retained records the caller reports as no longer available, each
  an instance and a capture (or a valuation, below).

A `CaptureFact` is `Value { anchor, value }`, `Missing`, `Null`, `WrongType` or
`Stale`.

## Outputs

An `Activation`: a `disposition` and the `instances`.

- `disposition` is `Inactive`, `Unknown` or `Active`. `Unknown` carries its
  completeness and its assessment-execution disposition, which distinguish
  `Unknown(open)`, `Unknown(missing)` and `Unknown(refused)`.
- An `Instance` holds its identity, its receipts in delivery order (the
  provenance) and its `CaptureEnvironment`: for each capture in authored order
  its declared name, type, value, source binding and anchor.
- An activation fault: an `ActivationFault` naming the capture or the guard and
  its kind, with `non_value.kind` `Missing` where the input is incomplete and
  `Refused` where it is refused (FR-516). The activation of a faulted instance
  is `Unavailable` (FR-516).
- A contradiction refusal: `invalid_runtime_input`/`invalid-value` carrying
  `contradicted_identity` (the trigger or origin identity bytes). The evidence
  call reports it as `Err(CallFailure::Admission)` with an empty charge log.

No output holds a truth value.

## Behavior

### Instance identity

- The evaluator SHALL identify an event-triggered instance by the clause and the
  semantic trigger identity, byte for byte, and SHALL NOT identify it by a
  timestamp, display name, receipt or equal captured payload.
- The evaluator SHALL create a distinct instance for each distinct trigger
  identity, even where the captured values are equal.
- When a second delivery carries an identity of an existing instance and an equal
  payload, the evaluator SHALL add its receipt to that instance's provenance and
  SHALL NOT create a second instance.
- When a second delivery carries an identity of an existing instance and a
  different payload, the evaluator SHALL return the contradiction refusal naming
  that identity, and SHALL NOT alias the deliveries or replace the retained
  capture environment.
- The evaluator SHALL identify a whole-execution-origin instance by the clause and
  the admitted execution-origin identity, and SHALL create exactly one such
  instance per origin identity.
- When two origins carry one identity and different payloads, the evaluator SHALL
  return the contradiction refusal naming that identity.

### Guard

- When a delivery's `guard` is `Some(true)`, the evaluator SHALL create the
  instance, and when it is `Some(false)` the evaluator SHALL create none and SHALL
  NOT report a refusal.
- When the clause has a guard and a delivery's `guard` is `None`, the evaluator
  SHALL report an `ActivationFault` naming the guard with `non_value.kind`
  `Missing`, and SHALL NOT read the guard as false.

### Captures

- The evaluator SHALL evaluate the captures of an instance in authored order,
  once, at the delivery's anchor, and SHALL retain for each its declared name,
  type, value, source binding and anchor.
- When a capture is `Missing` or `Null`, when its record is in `evicted`, or when
  the delivery supplies fewer captures than the clause declares, the evaluator
  SHALL report an `ActivationFault` naming the capture with `non_value.kind`
  `Missing`.
- When a capture is `WrongType` or `Stale`, or when the delivery's anchor or the
  capture's anchor differs from the evaluation anchor, the evaluator SHALL report
  an `ActivationFault` naming the capture with `non_value.kind` `Refused`.
- The evaluator SHALL NOT begin temporal evaluation for an instance with a fault,
  and an unrelated instance of the same call SHALL keep its activation.
- The evaluator SHALL NOT re-evaluate an initializer during an incremental
  re-evaluation, a restoration or a replay, and an evicted capture SHALL make its
  instance `Missing` rather than be recomputed at a later anchor.
- A later observation SHALL NOT change a retained capture, and one instance's
  captures SHALL NOT satisfy another instance's proposition.
- The checker SHALL refuse a capture initializer that reads a capture written
  after it, because only earlier captures are in a capture's scope (FR-337
  "Captures at S3").

### Retained valuations

- When a valuation an unsettled instance still requires is in `evicted`, the
  evaluator SHALL report `Incomplete` naming the evicted subject, and SHALL NOT
  narrow the evaluated interval to the valuations that remain.

### Dispositions of a scope with no instance

- The evaluator SHALL report `Inactive` for a trigger scope that is `Closed`, with
  `Admitted` evidence, a complete trigger record and no instance-creating delivery.
- The evaluator SHALL report `Unknown(open)` for an `Open` scope with no instance.
- The evaluator SHALL report `Unknown(missing)` for `Missing` trigger evidence,
  and `Unknown(refused)` for `Refused` trigger evidence, whatever the closure.
- The evaluator SHALL report `Unknown(open)` with incomplete completeness for a
  `Closed` scope whose trigger record is incomplete.
- A scope whose every admitted delivery has a false guard SHALL report as a scope
  with no instance.
- No disposition SHALL be a truth value, and the evaluator SHALL evaluate no
  position for `Inactive` or `Unknown`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-524-AC-1 | Two deliveries with identities `t1` and `t2`, equal payloads and equal captured amounts create two instances; a proposition over `t1`'s capture is not satisfied by `t2`'s equal amount, and the reported mismatch names `t1` and `t2`. | Test (TC-1319) |
| FR-524-AC-2 | A second delivery of `t1` under another receipt with an equal payload creates no second instance, adds its receipt after the first in `t1`'s provenance and leaves the capture environment byte-equal. | Test (TC-1319) |
| FR-524-AC-3 | Two deliveries of `t1` with different payloads return the contradiction refusal with `contradicted_identity` `t1`, the retained environment unchanged, and through `evaluate_temporal_evidence` an `Err(CallFailure::Admission)` with an empty charge log. | Test (TC-1319) |
| FR-524-AC-4 | The instance identity is the trigger identity byte for byte: two deliveries equal in timestamp, receipt, payload and captures but different in one identity byte give two instances. | Test (TC-1319) |
| FR-524-AC-5 | Over a clause `on origin`, two admitted origins `o1` and `o2` give two instances that do not alias, re-evaluating `o1` does not split it, and two origins `o1` with different payloads return the contradiction refusal naming `o1`. | Test (TC-1319) |
| FR-524-AC-6 | A guard evaluating false creates no instance and no refusal, and a closed scope whose every delivery has a false guard reports `Inactive`. | Test (TC-1319) |
| FR-524-AC-7 | A guarded clause with a delivery whose `guard` is `None` reports an `ActivationFault` naming the guard, `non_value.kind` `Missing`, and no instance. | Test (TC-1319) |
| FR-524-AC-8 | A `Missing` capture, a `Null` capture, an evicted capture and a delivery with one capture too few each report an `ActivationFault` naming that capture with `Missing`, and the instance evaluates no position. | Test (TC-1319) |
| FR-524-AC-9 | A `WrongType` capture, a `Stale` capture, a delivery anchor that differs from the evaluation anchor and a capture anchor that differs from it each report an `ActivationFault` with `Refused`, naming the capture (or the anchor), and the instance evaluates no position. | Test (TC-1319) |
| FR-524-AC-10 | In one call with a healthy instance and a faulted instance, the healthy instance keeps its activation and its capture environment. | Test (TC-1319) |
| FR-524-AC-11 | The captures of a three-capture clause are evaluated in authored order, once per instance, and the evaluation count per instance stays one across an incremental re-evaluation and an admitted restoration. | Test (TC-1319) |
| FR-524-AC-12 | A later delivery or observation that carries a changed amount or reference leaves the retained capture environment byte-identical, and the type exposes no mutator of it. | Test (TC-1319) |
| FR-524-AC-13 | A capture initializer that reads a capture written after it is refused at S3 and no clause node is emitted. | Test (TC-1319) |
| FR-524-AC-14 | An evicted required valuation returns `Incomplete` naming the evicted subject with no truth, and the same trace with the valuation present evaluates the whole interval. | Test (TC-1319) |
| FR-524-AC-15 | A closed scope with admitted evidence and no instance-creating delivery reports `Inactive`; an open scope reports `Unknown(open)`; `Missing` evidence reports `Unknown(missing)`; `Refused` evidence reports `Unknown(refused)`; a closed scope with an incomplete trigger record reports `Unknown(open)` with incomplete completeness; none carries a truth value or evaluates a position. | Test (TC-1319) |
| FR-524-AC-16 | Through `evaluate_temporal_evidence` the activation states of FR-516-AC-2 equal the dispositions of AC-15 and the fault kinds of AC-8 and AC-9: `Inactive`, `Unknown`, `Active`, and `Unavailable` for a faulted instance. | Test (TC-1319) |

## Dependencies

- FR-044-AC-1 to AC-9 (the legacy criteria this requirement succeeds), FR-053
  (the opaque identity), FR-327 (position activation), FR-337 (capture scope),
  FR-516 (the evidence call that applies this function), FR-523 (the counters
  `captures` and `support` and the stop scope).
- QSpec FR-093 and FR-094.
- Feeds QSL-1545 (FR-516-AC-2 to AC-4).

## References

- Linear QSL-1503; deletion-table rows B17, B18, B20, B21, B22.

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
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-522
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
the clause's captures once at the activation anchor and keeps them unchanged,
including across calls through the retained instance table. For an activation
that cannot be established, it keeps the instance in the call's `instances` as
`Unavailable`, with the fault and the input the fault names. An activation is
never left unset: every resolved instance is `Active` or `Unavailable`, and every
call result holds one of the four activation states (FR-516).
For a trigger scope that creates no instance it reports one of four
dispositions, and none of the four carries a truth value.

This is the spine successor of FR-044-AC-1 to AC-9 and the activation half of
FR-052-AC-10 and FR-053-AC-7 to AC-9. It adds the instance model that
`qsl-eval`'s `on each` and `on origin` position activation (FR-327) does not
have: the instance is keyed by the opaque trigger identity, not by a position.

## Use case

A payment monitor receives the same refund trigger twice, once through each of
two relays, then a second refund for the same amount. The evaluator keeps one
instance for the first refund with both relay receipts as provenance, and a
second instance for the second refund even though its amount is equal. When one
relay then delivers a different amount under the first refund's identity, the
evaluator refuses and names that identity, and the retained amount does not
change. When the monitor comes back the next day and the first refund arrives a
third time, the retained amount is still the one it kept.

## Semantic authority and boundary

QSpec FR-093 and FR-094 own activation, guard and capture meaning. FR-053 owns
the opaque trigger identity. `evaluate_temporal_evidence` (FR-516) carries an
ordered `deliveries` list for a clause that activates `on each` and an `origins`
list for a clause that activates `on origin`, and calls `activate` once with them.
`trigger` of the request, when present, selects the instance the call assesses; a request that carries no delivery and no origin has no `trigger` (FR-516). The other
instances the lists resolve are reported in `instances` and are not assessed in
that call.

## Inputs

`qsl_eval::temporal::activation::activate`, over the clause's `Activation`
(FR-515: `Origin`, or `Each { trigger, guard }`) and its ordered captures, with an
`ActivationInput`:

- `scope`: the trigger scope's closure (`Closed` or `Open`), its trigger
  evidence (`Admitted`, `Missing` or `Refused`) and the completeness of its
  trigger record (`Complete` or `Incomplete`), independent of the decision
  scope;
- `execution`: the surrounding execution's disposition (`Completed` or `Failed`),
  which an `Unknown` disposition carries;
- `deliveries`: for `Each`, an ordered `Vec<TriggerDelivery>`, each `{ identity:
  SemanticTriggerIdentity, receipt, anchor, payload, guard: Option<bool>,
  captures: Vec<CaptureFact> }`;
- `origins`: for `Origin`, an ordered `Vec<ExecutionOrigin>`, each `{ identity,
  payload, captures: Vec<CaptureFact> }`, so that a clause `on origin` with
  captures establishes them from the origin's facts exactly as a delivery does;
- `anchor`: the evaluation anchor;
- `evicted`: the retained records the caller reports as no longer available, each
  an instance and a capture, or a valuation;
- `ledger`: an optional `EvidenceLedger` (below).

A `CaptureFact` is `Value { anchor, value }`, `Missing`, `Null`, `WrongType` or
`Stale`.

### Instance-bound valuations

Each observed position of the evidence request (FR-516) carries, besides its
shared valuations, `instance_valuations`: a map from a trigger identity to the
valuations of that instance at that position. An atom is **instance-bound** when
the binders its Boolean operand reads (FR-515 `binders()`) include the trigger
binder or a capture. For the assessed instance an instance-bound atom reads only
that instance's entry.

### The evidence ledger

`EvidenceLedger` is the caller-held value that FR-502's `ProgressLedger` is the
pattern of: the request carries it and the result returns the updated one. Its
`instances` table is keyed by the obligation (the package identity and the
clause) and the trigger identity. Each entry holds the retained payload, the
receipts in arrival order and the `CaptureEnvironment`. FR-526 extends the same
value with the table of latest results. The ledger holds no result bytes, only
flat entries, and the caller owns its size.

## Outputs

An `Activation`: a `disposition`, the `instances` and the updated `ledger`.

- `disposition` is `Inactive`, `Unknown` or `Active`. `Unknown` carries its
  completeness and its assessment-execution disposition, which distinguish
  `Unknown(open)`, `Unknown(missing)` and `Unknown(refused)`.
- An `Instance` holds its identity, its receipts in arrival order (the
  provenance) and its `CaptureEnvironment`: for each capture in authored order
  its declared name, type, value, source binding and anchor.
- An activation fault: an `ActivationFault` naming the capture or the guard and
  its kind, with `non_value.kind` `Missing` where the input is incomplete and
  `Refused` where it is refused (FR-516). The activation of a faulted instance
  is `Unavailable` (FR-516).
- A contradiction refusal: `invalid_runtime_input`/`invalid-value` carrying
  `contradicted_identity` (the trigger or origin identity bytes). The evidence
  call reports it as `Err(CallFailure::Admission)` with an empty charge log.
- A surplus-capture refusal: `invalid_runtime_input`/`invalid-value` carrying
  `surplus_capture_identity` (the delivery or origin identity bytes) and
  `surplus_capture_index` (the index of the first capture fact beyond the clause's
  declared captures). It is malformed
  input, not a fault of an activation, and the evidence call reports it as
  `Err(CallFailure::Admission)` with an empty charge log.
- A counter or meter stop while establishing instances is not an error and not a
  fault: the evidence call reports it as an `Incomplete` result (FR-516, FR-523).

No output holds a truth value.

## Behavior

### Instance identity

- The evaluator SHALL identify an event-triggered instance by the clause and the
  semantic trigger identity, byte for byte.
- The evaluator SHALL NOT identify an instance by a timestamp, display name,
  receipt or equal captured payload.
- The evaluator SHALL create a distinct instance for each distinct trigger
  identity, even where the captured values are equal.
- When a delivery carries the identity of an instance created earlier in the same
  call or retained in the ledger, with an equal payload, the evaluator SHALL add
  its receipt to that instance's provenance.
- In that case the evaluator SHALL NOT create a second instance.
- When such a delivery carries a different payload, the evaluator SHALL return the
  contradiction refusal naming that identity.
- In that case the evaluator SHALL NOT replace the retained capture environment.
- The evaluator SHALL identify a whole-execution-origin instance by the clause and
  the admitted execution-origin identity.
- The evaluator SHALL create exactly one instance per origin identity.
- The evaluator SHALL establish an origin's captures from the origin's own
  `captures`, in authored order, once, as it does for a delivery.
- When two origins carry one identity and different payloads, the evaluator SHALL
  return the contradiction refusal naming that identity.

### Guard

- When a delivery's `guard` is `Some(true)`, the evaluator SHALL create the
  instance.
- When a delivery's `guard` is `Some(false)`, the evaluator SHALL create no
  instance.
- The evaluator SHALL NOT report a refusal for a false guard.
- When the clause has a guard and a delivery's `guard` is `None`, the evaluator
  SHALL keep the delivery's instance in `instances` as `Unavailable` with an
  `ActivationFault` naming the guard with `non_value.kind` `Missing`, and the
  delivery has no activation.
- The evaluator SHALL record a faulted instance in the updated `ledger` as an
  `Unavailable` entry holding its identity, payload, receipts and fault and no
  capture environment, and SHALL NOT count it in the `instances` counter, which
  counts instances created in the call. A later delivery of that identity with an
  equal payload adds its receipt and reports the retained fault, a different
  payload is the contradiction refusal, and no initializer runs again.
- A delivery with a false guard SHALL NOT appear in `instances`.
- The evaluator SHALL NOT read an unestablished guard as false.

### Captures

- The evaluator SHALL evaluate the captures of an instance in authored order,
  once, at the delivery's anchor.
- The evaluator SHALL retain for each capture its declared name, type, value,
  source binding and anchor.
- The evaluator SHALL refuse a delivery or an origin that carries more capture
  facts than the clause declares captures, with the surplus-capture refusal, and
  SHALL NOT ignore the surplus facts.
- The evaluator SHALL examine the refusals of the whole call before it charges
  anything: first the contradiction check over every delivery or origin in list
  order, and only when none contradicts, the surplus-capture check over every
  delivery or origin in list order. A call that carries both refusals returns the
  contradiction refusal.
- The evaluator SHALL examine a delivery's faults in this order: the guard, the
  number of captures, the delivery anchor, then each capture in authored order, an
  evicted record before the capture's fact.
- The evaluator SHALL report the first fault found in that order.
- When an origin supplies fewer captures than the clause declares, the evaluator
  SHALL report an `ActivationFault` naming the first capture it does not supply,
  with `non_value.kind` `Missing`.
- When the delivery supplies fewer captures than the clause declares, the
  evaluator SHALL report an `ActivationFault` naming the first capture the
  delivery does not supply, with `non_value.kind` `Missing`.
- When a capture is `Missing` or `Null`, or its record is in `evicted`, the
  evaluator SHALL report an `ActivationFault` naming that capture with
  `non_value.kind` `Missing`.
- When a capture is `WrongType` or `Stale`, the evaluator SHALL report an
  `ActivationFault` naming that capture with `non_value.kind` `Refused`.
- When the delivery's anchor, or a capture's anchor, differs from the evaluation
  anchor, the evaluator SHALL report an `ActivationFault` with `non_value.kind`
  `Refused`.
- The evaluator SHALL NOT begin temporal evaluation for an instance with a fault.
- An unrelated instance of the same call SHALL keep its activation.
- The evaluator SHALL NOT re-evaluate an initializer during an incremental
  re-evaluation, a restoration or a replay.
- When a record of a retained instance's capture is in `evicted`, the evaluator
  SHALL report an `ActivationFault` naming the capture with `Missing`.
- The evaluator SHALL NOT recompute an evicted capture at a later anchor.
- A later observation SHALL NOT change a retained capture.
- One instance's captures SHALL NOT satisfy another instance's proposition.
- The checker SHALL refuse a capture initializer that reads a capture written
  after it, because only earlier captures are in a capture's scope (FR-337
  "Captures at S3").

### Instance-bound atoms

- When an instance-bound atom has no entry for the assessed instance at an eligible
  position, the evaluator SHALL treat the valuation as missing (FR-501).
- The evaluator SHALL NOT read another instance's entry for that atom.
- The missing valuation SHALL name the assessed instance and the identities of the
  other instances that hold an entry for the same atom and position, in the
  `instance` and `also_held_by` members of FR-501's `MissingValuation` reason,
  which FR-504's `Indeterminate` carries identically.

### Retained valuations

- When a valuation an unsettled activation still requires is in `evicted`, the
  evaluator SHALL report an `Incomplete` result (FR-516) naming the evicted
  subject.
- The evaluator SHALL NOT narrow the evaluated interval to the valuations that
  remain.

### Charges

- The evaluator SHALL charge one `temporal.visit` (`evaluation_steps`) for each
  delivery or origin it resolves.
- The evaluator SHALL charge one `temporal.visit` for each capture it establishes.
- The evaluator SHALL charge one `evicted` visit for each hit: each evicted record
  it finds to match a capture or a valuation it consults. A consultation that finds
  no matching record charges nothing, and a fault or stop at the first hit ends the
  consultation, so an instance or an evaluation charges at most one.
- The evaluator SHALL charge one `instances` unit for each instance it creates in
  the call and one `captures` unit for each capture it retains (FR-523). An
  instance that the ledger already retains, a delivery with a false guard and an
  instance whose activation faulted are not created in the call and charge no
  `instances` unit, although the call's `instances` output lists the retained and
  the faulted ones.

### Dispositions of a scope with no instance

- The evaluator SHALL report `Inactive` for a trigger scope that is `Closed`, with
  `Admitted` evidence, a complete trigger record and no instance-creating delivery.
- The evaluator SHALL report `Unknown(open)` for an `Open` scope with no instance.
- The evaluator SHALL report `Unknown(missing)` for `Missing` trigger evidence,
  whatever the closure.
- The evaluator SHALL report `Unknown(refused)` for `Refused` trigger evidence,
  whatever the closure.
- The evaluator SHALL report `Unknown(open)` with incomplete completeness for a
  `Closed` scope whose trigger record is incomplete.
- The evaluator SHALL carry the input `execution` disposition in each `Unknown`
  disposition and SHALL NOT let it change which disposition is reported.
- A scope whose every admitted delivery has a false guard SHALL report as a scope
  with no instance.
- No disposition SHALL be a truth value.
- The evaluator SHALL evaluate no position for `Inactive` or `Unknown`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-524-AC-1 | Two deliveries with identities `t1` and `t2` and equal payloads and captured amounts create two instances with two distinct capture environments keyed by `t1` and `t2`. | Test (TC-1319) |
| FR-524-AC-2 | An instance-bound atom with an entry only for `t2` at a position leaves `t1`'s valuation missing: where the verdict depends on the atom, `t1`'s `Indeterminate` carries the reason `MissingValuation` with `instance` `t1` and `also_held_by` holding `t2`, and `t2` is assessed with its entry. | Test (TC-1319) |
| FR-524-AC-3 | A second delivery of `t1` under another receipt with an equal payload creates no second instance and adds its receipt after the first in `t1`'s provenance. | Test (TC-1319) |
| FR-524-AC-4 | The capture environment of `t1` is byte-equal before and after the AC-3 delivery. | Test (TC-1319) |
| FR-524-AC-5 | Two deliveries of `t1` with different payloads, passed to `activate`, return the contradiction refusal with `contradicted_identity` `t1`, and the retained environment is unchanged. | Test (TC-1319) |
| FR-524-AC-6 | Through `evaluate_temporal_evidence`, a request whose `deliveries` hold `t1` twice with different payloads returns `Err(CallFailure::Admission)` with an empty charge log. | Test (TC-1319) |
| FR-524-AC-7 | Two deliveries equal in timestamp, receipt, payload and captures and different in one identity byte create two instances. | Test (TC-1319) |
| FR-524-AC-8 | Over a clause `on origin`, two admitted origins `o1` and `o2` create two instances that do not alias, and re-evaluating `o1` does not split it. | Test (TC-1319) |
| FR-524-AC-9 | Two origins `o1` with different payloads return the contradiction refusal naming `o1`. | Test (TC-1319) |
| FR-524-AC-10 | A delivery whose `guard` is `Some(false)` creates no instance and no refusal, and a closed scope whose every delivery has a false guard reports `Inactive`. | Test (TC-1319) |
| FR-524-AC-11 | A guarded clause with a delivery whose `guard` is `None` reports an `ActivationFault` naming the guard, `non_value.kind` `Missing`, and no activation for that delivery: its instance is listed in `instances` as `Unavailable`. | Test (TC-1319) |
| FR-524-AC-12 | A `Missing` capture, a `Null` capture and an evicted capture each report an `ActivationFault` naming that capture with `Missing`, and the instance evaluates no position. | Test (TC-1319) |
| FR-524-AC-13 | A delivery with one capture too few reports an `ActivationFault` naming the first capture it does not supply, with `Missing`. | Test (TC-1319) |
| FR-524-AC-14 | A `WrongType` capture and a `Stale` capture each report an `ActivationFault` with `Refused` naming that capture. | Test (TC-1319) |
| FR-524-AC-15 | A delivery anchor that differs from the evaluation anchor and a capture anchor that differs from it each report an `ActivationFault` with `Refused`. | Test (TC-1319) |
| FR-524-AC-16 | A delivery with a wrong anchor and a missing capture reports the anchor fault, and a delivery with an evicted capture and a wrong-type later capture reports the evicted capture. | Test (TC-1319) |
| FR-524-AC-17 | In one call with a healthy instance and a faulted instance, the healthy instance keeps its activation and its capture environment. | Test (TC-1319) |
| FR-524-AC-18 | The captures of a three-capture clause are evaluated in authored order, once per instance. | Test (TC-1319) |
| FR-524-AC-19 | With a ledger holding `t1` from an earlier call, a later call that delivers `t1` again with an equal payload adds its receipt to the retained provenance and evaluates no initializer, and the evaluation count of `t1` stays one. | Test (TC-1319) |
| FR-524-AC-20 | With a ledger holding `t1`, a later call that delivers `t1` with a different payload returns the contradiction refusal, and the ledger is unchanged. | Test (TC-1319) |
| FR-524-AC-21 | With a ledger holding `t1` and a retained capture of `t1` in `evicted`, the call reports an `ActivationFault` naming the capture with `Missing` and recomputes nothing. | Test (TC-1319) |
| FR-524-AC-22 | A later delivery or observation that carries a changed amount or reference leaves the retained capture environment byte-identical, and the type exposes no mutator of it. | Test (TC-1319) |
| FR-524-AC-23 | A capture initializer that reads a capture written after it is refused at S3 and no clause node is emitted; FR-337-AC-8 owns this refusal and its test. | Test (TC-1267) |
| FR-524-AC-24 | An evicted required valuation returns `Incomplete` naming the evicted subject with no truth. | Test (TC-1319) |
| FR-524-AC-25 | The same trace as AC-24 with the valuation present evaluates the whole interval. | Test (TC-1319) |
| FR-524-AC-26 | A call with two deliveries and one capture each charges `evaluation_steps` 2 for the deliveries, 2 for the captures, `instances` 2 and `captures` 2 for the activation. | Test (TC-1319) |
| FR-524-AC-27 | A closed scope with admitted evidence and no instance-creating delivery reports `Inactive`, and an open scope reports `Unknown(open)`. | Test (TC-1319) |
| FR-524-AC-28 | `Missing` trigger evidence reports `Unknown(missing)` and `Refused` evidence reports `Unknown(refused)`, whatever the closure. | Test (TC-1319) |
| FR-524-AC-29 | A closed scope with an incomplete trigger record and no instance reports `Unknown(open)` with incomplete completeness. | Test (TC-1319) |
| FR-524-AC-30 | No disposition of AC-27 to AC-29 carries a truth value or evaluates a position. | Test (TC-1319) |
| FR-524-AC-31 | Through `evaluate_temporal_evidence` with no delivery, no origin and `trigger` `None`, the activation states of FR-516-AC-2 equal the dispositions of AC-27 and AC-28, and a faulted assessed instance reports `Unavailable`. | Test (TC-1319) |
| FR-524-AC-32 | Over a clause `on origin` with one capture, an origin that supplies one `Value` capture creates one instance with that capture in its environment, an origin that supplies none reports an `ActivationFault` naming the capture with `Missing`, and an origin whose capture is `WrongType` reports `Refused`. | Test (TC-1319) |
| FR-524-AC-33 | A delivery or an origin that supplies one capture fact more than the clause declares returns the surplus-capture refusal naming its identity and the index of the surplus fact, through `activate` and through `evaluate_temporal_evidence` as `Err(CallFailure::Admission)` with an empty charge log, and no instance of the call is created. | Test (TC-1319) |
| FR-524-AC-34 | A call with a guard-`None` delivery `t1`, a guard-`Some(false)` delivery `t2` and a healthy delivery `t3` lists `t1` as `Unavailable` and `t3` as `Active` in `instances`, and does not list `t2`. | Test (TC-1319) |
| FR-524-AC-35 | A call with a ledger holding `t1`, a redelivery of `t1`, one healthy new delivery `t2` and one faulted new delivery `t3` charges `instances` 1, lists three instances, and returns a ledger holding `t1`, `t2` and `t3`, with `t3` an `Unavailable` entry without a capture environment. | Test (TC-1319) |
| FR-524-AC-36 | An instance whose first capture and second capture are both in `evicted` charges one `evicted` visit, and a retained-valuation lookup that matches one evicted record charges one `evicted` visit, and one that matches none charges none. | Test (TC-1319) |
| FR-524-AC-37 | A later call with the ledger of AC-35 that delivers `t3` again with an equal payload and complete captures adds the receipt, reports the retained fault, evaluates no initializer and keeps `t3` `Unavailable`; with a different payload it returns the contradiction refusal. | Test (TC-1319) |
| FR-524-AC-38 | A call that carries a contradiction on `t1` and a surplus capture on `t2` returns the contradiction refusal naming `t1`; the same call without the contradiction returns the surplus-capture refusal with `surplus_capture_identity` `t2` and `surplus_capture_index` equal to the declared capture count. | Test (TC-1319) |
| FR-524-AC-39 | An open scope with no instance returns `Unknown(open)` carrying `Failed` under `execution` `Failed` and `Completed` under `execution` `Completed`; `Unknown(missing)` and `Unknown(refused)` carry it likewise; `Inactive` carries neither, and the reported disposition kind is the same for both values. | Test (TC-1319) |

## Dependencies

- FR-044-AC-1 to AC-9 (the legacy criteria this requirement succeeds), FR-053
  (the opaque identity), FR-327 (position activation), FR-337 (capture scope),
  FR-515 (the clause view), FR-522 (the identity that carries the ledger entries),
  FR-523 (the counters `captures`, `instances`, `evaluation_steps`).
- FR-516 carries the lists and the entry that calls `activate` and is a reference,
  not a dependency, of this requirement.
- QSpec FR-093 and FR-094.

## References

- Linear QSL-1503; deletion-table rows B17, B18, B20, B21, B22.

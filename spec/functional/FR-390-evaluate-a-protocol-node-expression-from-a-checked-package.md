---
id: FR-390
title: "Evaluate a protocol node's expression from a checked package at S6a with shared accounting"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-464
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-465
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-466
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-058
    type: depends_on
---
# FR-390: Evaluate a protocol node's expression from a checked package at S6a with shared accounting

## Description

S6a SHALL evaluate one expression of one protocol node of a `CheckedPackage`:
an activation guard, a capture initializer, a retry relation, a recovery
predicate, a trigger guard, a control's constraint, guard, visible entry or
`related` end, a finish or terminal condition, a role's lifetime or a channel's
key. The expression is the term the package holds in the node's body (QSpec
FR-464, FR-465, FR-466). The caller supplies the value of every binder and
every captured value the expression reads, and an observation when a supplied
value names a model object. Evaluation reads the package and the supplied
values and nothing else.

A capture is a frozen value. The protocol evaluates a registration capture when
the obligation registers and an activation capture once when it activates
(QSpec FR-466), and a protocol capture once when the protocol starts; each
result is fixed from then on (FR-218). The evaluator models this one way: a
capture slot is a call that evaluates the capture's initializer from the
supply of its own stage and returns the capture's value; every later slot (a
trigger guard, a retry relation, a recovery predicate, a constraint) takes the
captured values it reads as part of its supply, and the evaluator never
recomputes a capture it is given.

A consumer evaluates one expression per call: it evaluates a compensation's
registration capture at registration, its activation capture at activation, and
then its trigger guard, retry relation and recovery predicate as the protocol
reaches them, each as its own call. The calls that the caller gives one
`Meter` accumulate their charges in it, so a compensation's work is accounted
once across them (FR-255's ten `quire.value.accounting/v1` counters). A call
that passes a fresh meter is accounted alone.

## Use case

A protocol runtime holds the checked package of a refund protocol. When a
payment effect occurs it evaluates the refund's registration capture from the
payment record and keeps the captured total. When a shipment fails it
evaluates the activation capture from the failed shipment. Later it asks
whether the trigger guard holds, whether the retry relation admits a second
attempt after a first, and whether the refunded amounts satisfy the recovery
predicate, each time supplying the two attempts or the refunded record and the
captured values it kept. The retry and recovery calls see the total as it
was registered, whatever the payment record holds by then, and receive a
Boolean, an undefined result with its cause, or an incomplete result that
names the counter or the value that stopped the evaluation.

## Inputs

- A `CheckedPackage` that carries the protocol as the v2 protocol nodes FR-383
  emits (QSpec FR-464).
- A `ProtocolSlot { node, member, index }`:
  - `node` is the node key of a `protocol` node;
  - `member` is the name of the node's body binding that holds the expression,
    as the slot table below lists it;
  - `index` is the position within a list member, and is present exactly for
    such a member.
- A `ProtocolSupply { binders, observation }`:
  - `binders`: the supplied sources, each a binder node key paired with a
    `SuppliedValue`, which is either a value of FR-106's closed value grammar
    (a Boolean, an integer, a record, a reference to an object of the supplied
    observation, an option or a sequence of those) or `Unavailable(cause)`. A
    captured value is supplied under its capture's binder key;
  - `observation`: none, or one current snapshot admitted as FR-106 admits the
    snapshot of a `ValueInvariants` selection, with no clause, anchor
    comparison or invocation. It is needed exactly when a supplied value is a
    reference.
- A `Meter` (FR-107).

The slots of a protocol node:

| Node | `member` | Stage (QSpec FR-464) | `index` |
| --- | --- | --- | --- |
| clause | `activation.guard` | `protocol-activation` | none |
| clause | `captures` | `protocol-capture` | the capture's position |
| role | `replication.until` | `role-lifetime` | none |
| channel | `ordering.key` | `channel-key` | none |
| compensation | `registration_captures` | `registration-capture` | the capture's position |
| compensation | `trigger.guard` | `trigger-guard` | none |
| compensation | `activation_captures` | `activation-capture` | the capture's position |
| compensation | `retry.relation` | `retry` | none |
| compensation | `recovery.predicate` | `recovery` | none |
| `choice` | `cases.guard` | `run` | the case's position in label order |
| `choice`, `repeat` | `visible` | `run` | the entry's position |
| `repeat` | `guard` | `run` | none |
| `check` | `condition` | `run` | none |
| `send`, `receive`, `attempt`, `effect`, `event` | `related.from`, `related.to` | `run` | the `related` record's position |
| `send`, `receive`, `attempt`, `effect`, `event`, `commit` | `constraint` | `run` | none |
| `finish` obligation | `constraint` | `finish` | none |
| `terminal` obligation | `condition` | `terminal` | none |

A `repeat`'s `proof.invariant` and `proof.variant` are not slots: S3 discharges
the proof before emission and a termination claim is their only reader
(FR-218), so nothing at run time evaluates them.

## Outputs

`Result<ProtocolSlotEvaluation, CallFailure>`:

- `ProtocolSlotEvaluation::Evaluated`, holding FR-090's `Evaluation`. The
  `Evaluation` is `Completed` with a Boolean for a guard, relation, predicate,
  condition or constraint; with the term's value for a capture (the capture's
  frozen value, of its declared type), a `visible` entry, a `related` end or a
  key; or `Undefined`, `Refused` or `Incomplete`, or a family result, as
  FR-107 and FR-100 map them.
- `ProtocolSlotEvaluation::Unavailable`, holding the binder supplied as
  `Unavailable`, its cause and the location of the read that met it.
  Category incomplete.
- `CallFailure::Input(InputRefusal::Supply(AdmissionFailure))` for a selection
  or supply refused before S6a, as FR-106's `AdmissionFailure`: `Refused` or
  `Incomplete`, one record holding the catalog code, cause and the locus it
  names (the node, member, index or binder). Nothing is charged.
- `CallFailure::Fault(InternalFault)` for an S6a invariant break, as FR-107.

Exit statuses are FR-285's `Category::exit_code` of the outcome's category and
FR-100's outcome mapping, unchanged: a completed value 0, an undefined result
10, a refusal 20, an unsupported construct 21, an incomplete result 22.

## Behavior

### The arm

- The `ProtocolClause` arm of the S6a seam SHALL serve this evaluation with the
  one call to its `evaluate` hook that FR-107 gives it. `S6aFamilyKind` gains
  no variant, and the seam matches the arm with no `_` arm.
- `CheckedPackageEvaluation` SHALL gain `evaluate_protocol_slot(&self, slot:
  &ProtocolSlot, supply: &ProtocolSupply, meter: &mut Meter) ->
  Result<ProtocolSlotEvaluation, CallFailure>`.
- The evaluator SHALL be the one S6a evaluator over the package's terms. It
  holds no second interpreter and reads no serialized form: a package the
  contract reader returns for FR-383's emitted nodes gives the same
  `ProtocolSlotEvaluation` and the same charge log as the checked package that
  was emitted.

### API contract

The result of `evaluate_protocol_slot` is `Result<ProtocolSlotEvaluation,
CallFailure>` with exactly these shapes: `ProtocolSlotEvaluation::Evaluated`
and `ProtocolSlotEvaluation::Unavailable`; a selection or supply refused before
S6a is `CallFailure::Input(InputRefusal::Supply(AdmissionFailure))`, and an S6a
invariant break is `CallFailure::Fault(InternalFault)`. The protocol slot is
selected only by this call: `run_clause` and FR-312's request carry no protocol
slot.

### Charge points and their order

Within one call the evaluator charges, in this order: the nodes of the slot's
expression in evaluation order (left to right, short-circuited operands
skipped), a read of a supplied source being one node; `model.navigate` for each
field read of an object and `model.deref` for each dereference, as FR-107
states, which occur only in a slot that may read model state; and
`result_units` for the completed value. Admission charges nothing. A capture
slot charges its initializer's nodes in its own call and no other call charges
them. Across calls on one meter the charges accumulate in call order, each
call's charges depending on its own package, slot and supply only. Tests assert
these relations and the boundaries measured by a call's usage `U` (exact `U`
completes, `U` less one stops naming the counter, calls on one meter add), and
never an absolute count.

### Selection and supply admission

Admission runs before S6a, charges nothing, and stops at the first failing
check with its one record:

1. If `node` names no node of the package, then admission SHALL refuse
   `missing_declaration`/`missing-name` at the node. If it names a node whose
   tag is not `protocol`, then admission SHALL refuse `invalid_model_binding`/
   `malformed-declaration` at the node.
2. If the node's operator has no `member` in the slot table, or the member is
   optional and absent in the node (a `terminal` obligation of mode `any` holds
   no `condition`), or the member is a list member and `index` is absent or out
   of range, or the member is not one and `index` is present, then admission
   SHALL refuse `missing_declaration`/`missing-name`, naming the member and, for
   an index defect, the index.
3. The slot's **sources** are the binders its expression reads: the
   `value`/`parameter` nodes of its dependency closure (QSpec FR-464's closure
   through `dependencies`, stopping at a binder node and at a function node). A
   protocol, registration or activation capture the expression reads is such a
   binder, so its captured value is a source. A capture slot's own binder is not
   a source of that slot. The closure is static, so a read that a short-circuit
   would skip still makes its binder a source.
4. If one binder key is supplied twice, then admission SHALL refuse
   `invalid_runtime_input`/`duplicate-member` naming the binder. If a supplied
   key is not a source of the slot, which includes a binder the expression does
   not read, a binder of another template or of another protocol, and a key that
   names no binder, then admission SHALL refuse `invalid_runtime_input`/
   `unknown-member` naming the first such key in ascending (level, node key)
   order. If a source is not supplied, then admission SHALL refuse
   `invalid_runtime_input`/`missing-member` naming the first missing source in
   ascending (level, node key) order. The three checks run in that order.
5. For each supplied binder with a value, in ascending (level, node key) order,
   admission SHALL check the value against the binder's `semantic_type` in this
   order: a value whose shape does not fit refuses `invalid_runtime_input`/
   `wrong-value-kind`; a record or reference of a type that is in the model but
   does not conform refuses `invalid_runtime_input`/`wrong-value-kind`, and a
   reference to a population of another model or universe refuses
   `foreign_reference`/`foreign-universe`, as FR-100 states for a call's
   argument; a payload that is not in the type's domain refuses
   `invalid_runtime_input`/`invalid-value`. Each refusal names the binder. A
   captured value is checked against its capture's declared type.
6. If a supplied value is a reference and `observation` is none, then admission
   SHALL return `Incomplete` with `unavailable_observation`/
   `missing-required-artifact`, naming the binder. Otherwise it SHALL resolve
   each reference in the observation: an absent target in a complete population
   refuses `dangling_reference`/`absent-target-in-complete-population`; a
   required population the observation does not mark complete returns
   `Incomplete` with `incomplete_population`/`incomplete-scope`, as FR-106
   check 7 and 8 state. An observation that FR-106 refuses is refused with
   FR-106's record (for the model the package selects, `invalid_model_binding`/
   `wrong-model-selection`).

A source supplied as `Unavailable` is admitted by checks 4 and 5 without a
value; check 6 does not apply to it.

### Evaluation

- The evaluator SHALL evaluate the slot's expression term over the supplied
  values, with the charges FR-107 and FR-100 state for the same expression
  nodes under the ten accounting counters of FR-255, and SHALL add no counter
  and no charge point. A completed value is retained and charged as a
  completed S6a value is.
- A capture slot SHALL evaluate its own initializer from its stage's supply and
  return the value as the capture's frozen value. The evaluator SHALL NOT
  retain it, and no other slot recomputes a capture: a slot that reads a capture
  reads the value the caller supplied for it. So two calls of a later slot that
  differ only in the object the registered record now names give the same
  result.
- A capture initializer is total in FR-104's sense: it carries no undischarged
  range, nonzero or presence obligation (FR-381). A dispatched call whose
  selected precondition is false is not such an obligation (FR-301), so an
  initializer that makes one evaluates to `FamilyResult::Undefined` with reason
  `precondition-false`, and the slot's result is that undefined result.
- Evaluation order is the term's: left to right, with `and`, `or` and `implies`
  skipping the right operand when the left decides (FR-107).
- If evaluation reads a binder supplied as `Unavailable`, then the evaluator
  SHALL stop and return `Unavailable` with that binder and the location of the
  read, keeping the charges already made. An `Unavailable` binder that
  evaluation does not read has no effect on the result.
- If the meter denies a charge, then the evaluator SHALL return
  `Outcome::Incomplete` naming the counter, `resource_exhausted`/
  `insufficient-next-charge`, with no value and no partial value, and the meter
  keeps what the evaluation consumed before the denial.
- A family result is mapped as FR-090 and FR-100 map it.
- A package holds no capture that reads itself, a capture written later or
  another template's capture: S3 refuses each (FR-381) and the contract reader
  refuses a package whose nodes hold one (QSpec FR-464), so the evaluator is
  never given one.

### Shared accounting and determinism

- The evaluator SHALL retain no state between calls except the caller's meter.
  Calls that pass the same `&mut Meter` accumulate their charges in it; calls
  that pass fresh meters are accounted separately. The charge a call makes
  depends on the package, the slot and the supply alone, never on an earlier
  call, so the total of several calls is the sum of their charges whatever
  order they run in.
- The evaluator SHALL be deterministic: the same package, slot, supply and
  meter budget give the same `ProtocolSlotEvaluation` and the same charge log.
- The evaluator SHALL read no path, environment variable, clock or search
  location.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-390-AC-1 | Over the refund protocol of TC-1070, each slot of the table evaluates to the value TC-1070 gives: the activation guard, the three capture stages, the trigger guard, the retry relation and the recovery predicate of two templates (each with the trigger ready, expecting false), the constraints of an `effect`, a `commit` and the `finish`, and the `related` ends, `visible` entry and case guards of a `choice`. | Test (TC-1070) |
| FR-390-AC-2 | A capture is a frozen value: a capture slot returns the value its initializer gives from its stage's supply, and a later slot reads only the value the caller supplies for it. A retry relation or recovery predicate called with the total registered as 5 sees 5 when the record the registration read now holds 9, and with 9 supplied sees 9. | Test (TC-1070) |
| FR-390-AC-3 | An empty supply refuses `invalid_runtime_input`/`missing-member` naming the lowest source of the slot; a supplied binder the expression does not read, including the forward binder at a retry, refuses `unknown-member`. | Test (TC-1070, TC-1071) |
| FR-390-AC-4 | A node that does not exist, a node of another tag, a member the operator lacks, an index out of range, a missing index and a surplus index each refuse before any charge with the code, cause and locus TC-1071 lists. | Test (TC-1071) |
| FR-390-AC-5 | A duplicate, an unknown and a missing source refuse in that order, each naming the binder; a binder of another template is unknown, and the order among several is ascending level. | Test (TC-1071) |
| FR-390-AC-6 | A value of the wrong shape, a record of another type, a value outside its type's domain, a reference to another model's population and a dangling reference each refuse with the code and cause TC-1071 lists; no observation with a reference, and an incomplete population, return `Incomplete`; an observation FR-106 refuses is refused with its record. | Test (TC-1071) |
| FR-390-AC-7 | A source supplied `Unavailable` and read returns `Unavailable` naming the binder and the read, with the charges already made; the same source in a call whose short-circuit never reads it completes. | Test (TC-1072) |
| FR-390-AC-8 | A capture initializer whose dispatched call has a false precondition gives the capture slot an `Undefined` result with reason `precondition-false`, category undefined, exit 10; the same slot with the precondition true completes. | Test (TC-1072) |
| FR-390-AC-9 | With usage `U` from a completed call, a meter with every charged counter at exactly its measured usage completes; with each charged counter in turn one below its usage and the others at theirs, the call returns `Incomplete` naming that counter with no value; a zero `work_units` returns `Incomplete`; a fresh meter completes the same call with the same value. | Test (TC-1073) |
| FR-390-AC-10 | One meter passed to a compensation's registration capture, activation capture, trigger guard, retry relation and recovery predicate in turn accumulates exactly the sum of the calls' charges; a `work_units` budget of that sum completes all of them and one below it returns `Incomplete` `work_units` at the last call; the calls in another order give the same total. | Test (TC-1073) |
| FR-390-AC-11 | Running one call twice gives equal results and charge logs; a call over the package the contract reader returns for FR-383's emitted nodes gives the same result and charge log as over the checked package that was emitted; a refused call leaves the meter as it was. | Test (TC-1073) |
| FR-390-AC-12 | A package whose capture initializer reads itself, a later capture or another template's capture is refused at S3 (FR-381) and no package is built; a package mutated in its bytes so an activation capture's initializer reads its own binder, or another template's capture binder, is refused by the contract reader with `missing_declaration`/`missing-name` (`read` `forward` and `foreign`), and no evaluation call is possible. | Test (TC-1074) |
| FR-390-AC-13 | With the S6a probe variant of FR-107-AC-6 added, the build fails with E0004 at exactly the checked-in S6a seam list: the arm serving state clauses and protocol slots is the one `ProtocolClause` arm. | Test (TC-1074) |

## Dependencies

- [FR-107](FR-107-evaluate-state-clauses-at-s6a.md): the `ProtocolClause` arm,
  the `Meter`, the charges of an expression's nodes and the clause evaluator
  this requirement extends to a protocol slot;
  [FR-090](FR-090-return-a-family-outcome-or-a-typed-family-refusal.md):
  the `Evaluation` shape.
- [FR-106](FR-106-admit-snapshots-and-invocations.md): the value grammar and the
  snapshot admission the supplied observation uses;
  [FR-100](FR-100-run-a-named-function-through-the-spine.md): the argument
  admission, the outcome mapping and the exit statuses.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md): the ten
  accounting counters and their setting names.
- [FR-381](FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md):
  the names each stage may read and the capture order that S3 checks;
  [FR-218](FR-218-check-every-protocol-control-construct-at-s3.md): the checked
  protocol and its frozen captures.
- [FR-383](FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md): the emitted
  protocol nodes and their read-back.
- QSpec FR-464, FR-465, FR-466 (the protocol node bodies, the stages and the
  slots that bar model state), FR-322 (the checked package), FR-056, FR-057,
  FR-058 (compensation registration, commit, retry and recovery).

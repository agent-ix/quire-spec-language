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
predicate, a trigger guard, a control's constraint or guard, a finish or
terminal condition, a role's lifetime or a channel's key. The expression is the
term the package holds in the node's body (QSpec FR-464, FR-465, FR-466). The
caller supplies the values of the binders the expression reads from outside the
protocol, and an observation when those values name model objects.
Evaluation reads the package and the supplied values and nothing else.

A consumer that runs a protocol evaluates one expression at a time: it
evaluates a compensation's trigger guard, then its retry relation, then its
recovery predicate, each as its own call. The calls draw on the one `Meter` the
caller passes, so the compensation's work is accounted once across them
(FR-255's ten `quire.value.accounting/v1` counters). A call that passes a fresh
meter is accounted alone.

## Use case

A protocol runtime holds the checked package of a refund protocol. When a
shipment fails it asks whether the refund's trigger guard holds for the failed
shipment, whether the retry relation admits a second attempt after a first, and
whether the refunded amounts satisfy the recovery predicate. It supplies the
shipment, the two attempts and the refunded records, and nothing it could
compute from them, and it receives a Boolean, an undefined result with its
cause, or an incomplete result that names the counter or the value that
stopped the evaluation. It never supplies a captured total: the evaluator
computes that from the capture's own initializer.

## Inputs

- A `CheckedPackage` that carries the protocol as the v2 protocol nodes FR-383
  emits (QSpec FR-464).
- A `ProtocolSlot { node, member, index }`:
  - `node` is the node key of a `protocol` node;
  - `member` is the name of the node's body binding that holds the expression,
    as the slot table below lists it;
  - `index` is the position within an ordered-records member, and is present
    exactly for such a member.
- A `ProtocolSupply { binders, observation, prepared_for }`:
  - `binders`: the supplied binders, each a binder node key paired with a
    `SuppliedValue`, which is either a value of FR-106's closed value grammar
    (a Boolean, an integer, a record, a reference to an object of the supplied
    observation, an option or a sequence of those) or `Unavailable(cause)`;
  - `observation`: none, or one current snapshot admitted as FR-106 admits the
    snapshot of a `ValueInvariants` selection, with no clause, anchor
    comparison or invocation;
  - `prepared_for`: none, or the `package_id` the supplier prepared the values
    against.
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
| `choice` | `cases` | `run` | the case's position in label order |
| `repeat` | `guard` | `run` | none |
| `check` | `condition` | `run` | none |
| `send`, `receive`, `attempt`, `effect`, `event`, `commit` | `constraint` | `run` | none |
| `finish` obligation | `constraint` | `finish` | none |
| `terminal` obligation | `condition` | `terminal` | none |

## Outputs

`Result<ProtocolSlotEvaluation, CallFailure>`:

- `ProtocolSlotEvaluation::Evaluated`, holding FR-090's `Evaluation` and
  `captures()`, the capture binders the call evaluated to completion, in the
  order it evaluated them. The `Evaluation` is `Completed` with a Boolean for a
  guard, relation, predicate, condition or constraint and with a value of the
  capture's declared type for a capture; or `Undefined`, `Refused` or
  `Incomplete`, or a family result, as FR-107 and FR-100 map them.
- `ProtocolSlotEvaluation::Unavailable`, holding the binder supplied as
  `Unavailable`, its cause, the location of the read that met it and
  `captures()`. Category incomplete.
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
  holds no second interpreter and reads no serialized form: a package read
  back from emitted bytes (the I2 reader) and the checked package that was
  emitted give the same `ProtocolSlotEvaluation` and the same charge log.

- The evaluator runs against the in-process checked package. Evaluation from
  emitted bytes is the same evaluation over the package the contract reader
  returns for FR-383's nodes; once the reader's protocol step exists, its
  result and charge log equal the in-process evaluation's (FR-390-AC-12).

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
skipped); at the first read of a capture, the nodes of that capture's
initializer, charged at the point of the read and before the read's operator
continues, each capture once; `model.navigate` for each field read and
`model.deref` for each dereference, as FR-107 states; and `result_units` for the
completed value. Admission charges nothing. Across calls on one meter the
charges accumulate in call order, each call's charges depending on its own
package, slot and supply only. Tests assert these relations and the
boundaries measured by a call's usage `U` (exact `U` completes, `U` less one
stops naming the counter, calls on one meter add), and never an absolute count.

### Selection and supply admission

Admission runs before S6a, charges nothing, and stops at the first failing
check with its one record:

1. If `supply.prepared_for` is present and differs from the package's
   `package_id`, then admission SHALL refuse `stale_dependency`/
   `content-mismatch`, naming both identities.
2. If `node` names no node of the package, then admission SHALL refuse
   `missing_declaration`/`missing-name` at the node. If it names a node whose
   tag is not `protocol`, then admission SHALL refuse `invalid_model_binding`/
   `malformed-declaration` at the node.
3. If the node's operator has no `member` in the slot table, or the member is
   optional and absent in the node (a `terminal` obligation of mode `any` holds
   no `condition`), or the member is an ordered-records member and `index` is
   absent or out of range, or the member is not one and `index` is present, then
   admission SHALL refuse
   `missing_declaration`/`missing-name`, naming the member and, for an index
   defect, the index.
4. The slot's **sources** are the binders its expression reads from outside the
   protocol: the binders of its dependency closure that no capture initializer
   in that closure defines, found by FR-464's closure rule (the dependency
   closure through `dependencies`, which continues through the initializer of
   every capture the expression reads and stops at a binder node and at a
   function node). A capture binder is never a source. The closure is static, so
   a read that a short-circuit would skip still makes its binder a source.
5. If one binder key is supplied twice, then admission SHALL refuse
   `invalid_runtime_input`/`duplicate-member` naming the binder. If a supplied
   key is not a source of the slot, which includes a capture binder, a binder of
   another template or of another protocol, and a key that names no binder,
   then admission SHALL refuse `invalid_runtime_input`/`unknown-member` naming
   the first such key in ascending (level, node key) order. If a source is not
   supplied, then admission SHALL refuse `invalid_runtime_input`/
   `missing-member` naming the first missing source in ascending (level, node
   key) order. The three checks run in that order.
6. For each supplied binder with a value, in ascending (level, node key) order,
   admission SHALL check the value against the binder's `semantic_type` in this
   order: a value whose shape does not fit refuses `invalid_runtime_input`/
   `wrong-value-kind`; a reference of a type that is in the model but does not
   conform refuses `invalid_runtime_input`/`wrong-value-kind`, and a reference
   to a population of another model or universe refuses `foreign_reference`/
   `foreign-universe`, as FR-100 states for a call's argument; a payload that is
   not in the type's domain refuses `invalid_runtime_input`/`invalid-value`.
   Each refusal names the binder.
7. If a supplied value is a reference and `observation` is none, then admission
   SHALL return `Incomplete` with `unavailable_observation`/
   `missing-required-artifact`, naming the binder. Otherwise it SHALL resolve
   each reference in the observation: an absent target in a complete population
   refuses `dangling_reference`/`absent-target-in-complete-population`; a
   required population the observation does not mark complete returns
   `Incomplete` with `incomplete_population`/`incomplete-scope`, as FR-106
   check 7 and 8 state. An observation that FR-106 refuses is refused with
   FR-106's record (for the model the package selects, `invalid_model_binding`/
   `wrong-model-selection`).

A source supplied as `Unavailable` is admitted by checks 5 and 6 without a
value; check 7 does not apply to it.

### Evaluation

- The evaluator SHALL evaluate the slot's expression term over the supplied
  values, with the charges FR-107 and FR-100 state for the same expression
  nodes under the ten accounting counters of FR-255, and SHALL add no counter
  and no charge point. A completed value is retained and charged as a
  completed S6a value is.
- A capture the expression reads is evaluated by the evaluator. It evaluates
  the capture's initializer, at the capture's own stage, over the supplied
  values and the captures written before it, at its first read in the call,
  once, and charges it to the same meter; a second read in the call reuses the
  value without a charge. A capture the evaluation does not read, including one
  a short-circuit skips, is not evaluated and is not charged.
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
- A family result is mapped as FR-090 and FR-100 map it. A capture initializer
  that dispatches a call whose selected precondition is false evaluates to
  `FamilyResult::Undefined` with reason `precondition-false`, and the slot's
  result is that undefined result.
- A package holds no capture cycle and no capture that reads another
  template's capture or a capture written later: S3 refuses each (FR-381) and
  the reader refuses a package whose nodes hold one (QSpec FR-464 and FR-322),
  so the evaluator is never given one.

### Shared accounting and determinism

- The evaluator SHALL retain no state between calls except the caller's meter.
  Calls that pass the same `&mut Meter` accumulate their charges in it; calls
  that pass fresh meters are accounted separately. The charge a call makes
  depends on the package, the slot and the supply alone, never on an earlier
  call, so the total of several calls is the sum of their charges whatever
  order they run in.
- The evaluator SHALL be deterministic: the same package, slot, supply and
  meter budget give the same `ProtocolSlotEvaluation`, the same `captures()`
  and the same charge log.
- The evaluator SHALL read no path, environment variable, clock or search
  location.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-390-AC-1 | Over the refund protocol of TC-1070, each slot of the table evaluates to the value the table of TC-1070 gives: the activation guard, the protocol capture, the registration and activation captures, the trigger guard, the retry relation and the recovery predicate of two templates, and the constraints of an `effect`, a `commit` and the `finish`. | Test (TC-1070) |
| FR-390-AC-2 | The caller supplies only sources: `captures()` lists the captures the evaluation read, in read order, each once, and none when a short-circuit skips them; the retry relation with a first operand that decides it evaluates no capture. | Test (TC-1070) |
| FR-390-AC-3 | An empty supply refuses `invalid_runtime_input`/`missing-member` naming a source, never a capture, for every slot that has one; a supplied capture binder refuses `unknown-member`. | Test (TC-1070, TC-1071) |
| FR-390-AC-4 | A node that does not exist, a node of another tag, a member the operator lacks, an index out of range, a missing index and a surplus index each refuse before any charge with the code, cause and locus TC-1071 lists. | Test (TC-1071) |
| FR-390-AC-5 | A duplicate, an unknown and a missing source refuse in that order, each naming the binder; a binder of another template is unknown, and the order among several is ascending level. | Test (TC-1071) |
| FR-390-AC-6 | A value of the wrong shape, a reference of another type, a reference to another model's population and a dangling reference each refuse with the code and cause TC-1071 lists; no observation with a reference, and an incomplete population, return `Incomplete`; an observation FR-106 refuses is refused with its record. | Test (TC-1071) |
| FR-390-AC-7 | A `prepared_for` that differs from the package's `package_id` refuses `stale_dependency`/`content-mismatch` naming both; the matching value evaluates. | Test (TC-1071) |
| FR-390-AC-8 | A source supplied `Unavailable` and read returns `Unavailable` naming the binder and the read, with the charges already made and the captures completed before it; the same source in a call whose short-circuit never reads it completes. | Test (TC-1072) |
| FR-390-AC-9 | A capture initializer whose dispatched call has a false precondition gives the slot an `Undefined` result with reason `precondition-false`, category undefined, exit 10; the same slot with the precondition true completes. | Test (TC-1072) |
| FR-390-AC-10 | With usage `U` from a completed call, a meter at exactly `U` on each charged counter completes, one below `U` on `work_units` and on `result_units` returns `Incomplete` naming that counter with no value, a zero `work_units` returns `Incomplete`, and a fresh meter completes the same call with the same value. | Test (TC-1073) |
| FR-390-AC-11 | One meter passed to the trigger guard, the retry relation and the recovery predicate of one compensation in turn accumulates exactly the sum of the three calls' charges; a `work_units` budget of that sum completes all three and one below it returns `Incomplete` `work_units` at the last call; the three calls in another order give the same total. | Test (TC-1073) |
| FR-390-AC-12 | Running one call twice gives equal results, `captures()` and charge logs; a call over the package read back from its emitted bytes gives the same result and charge log as over the checked package that was emitted; a refused call leaves the meter as it was. | Test (TC-1073) |
| FR-390-AC-13 | A package whose capture initializer reads itself, a later capture or another template's capture is refused at S3 (FR-381) and no package is built; a package mutated in its bytes so an initializer reads the retry relation, or another template's capture binder, is refused by the reader with `invalid_package`/`definition-cycle` and `missing_declaration`/`missing-name`, and no evaluation call is possible. | Test (TC-1074) |
| FR-390-AC-14 | With the S6a probe variant of FR-107-AC-6 added, the build fails with E0004 at exactly the checked-in S6a seam list: the arm serving state clauses and protocol slots is the one `ProtocolClause` arm. | Test (TC-1074) |

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
  protocol.
- QSpec FR-464, FR-465, FR-466 (the protocol node bodies and the stages), FR-322
  (the checked package), FR-056, FR-057, FR-058 (compensation registration,
  commit, retry and recovery).

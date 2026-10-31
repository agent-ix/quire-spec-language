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
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
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
  - target: ix://agent-ix/quire-semantic-value/FR-130
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

- One admitted evaluation view of either an in-process `CheckedPackage` or
  the complete package admitted by the contract reader from the v2 protocol
  nodes FR-383 emits (QSpec FR-464), as the bridge below defines. An
  `ImportView`, a free-standing term and a decoded wire graph are not inputs.
- A `ProtocolSlot { node, member, index }`:
  - `node` is the node key of a `protocol` node;
  - `member` is the name of the node's body binding that holds the expression,
    as the slot table below lists it;
  - `index` is the position within a list member, and is present exactly for
    such a member.
- A `ProtocolSupply<'a> { binders, observation }`:
  - `binders`: the supplied sources, each a binder node key paired with a
    `ProtocolSuppliedValue`. It contains the existing `SuppliedValue` (a value
    of FR-106's closed value grammar or `Unavailable(cause)`) and a sequence of
    `ProtocolReferenceOccurrence` entries. Each occurrence pairs a path rooted
    at that binder, expressed with
    `quire_semantic_value::admission::PathStep`, with the exact checked
    population declaration `NodeKey` that the caller used to construct that
    reference. The path identifies one reference occurrence in the supplied
    value: the root reference has an empty relative path; a reference inside a
    record, a present option or a sequence has the corresponding field,
    option-value or index steps. This is provenance for the value already
    supplied, not a second value, object identity or population-membership
    claim. A captured value is supplied under its capture's binder key;
  - `observation`: `Option<&'a quire_semantic_value::admission::ObservationAdmission<qsl_package::observation::AdmittedProtocolObservation<'a>>>`,
    using QSV's existing generic directly as FR-106 defines. Its `Admitted`
    arm carries a current-snapshot authority for the evaluated view and the
    protocol's selected model; its other arms retain the original admission
    result. It is required exactly when a supplied source value contains a
    reference, including a nested reference. It carries no raw snapshot,
    provision or caller-constructed `Observation`.
  `ProtocolSuppliedValue` and `ProtocolReferenceOccurrence` are QSL
  layer-5 supply wrappers around the existing supplied value and shared path
  vocabulary. They SHALL NOT change `ObjectReference` identity, create a
  second admission result/carrier, contain raw snapshot or call inputs, or
  infer a population from object presence or from a model/type match. The
  checked `NodeKey` in each occurrence selects the intended population even
  when several declarations have the same normalized universe and effective
  member type. A caller SHALL be able to construct a supplied value with the
  existing typed `SuppliedValue` and its occurrence entries; the wrapper
  exposes no mutable admitted authority. Admission rechecks every supplied
  node against the selected view, so constructing an occurrence does not
  grant it authority.

The public QSL layer-5 input types SHALL be:

```rust
pub struct ProtocolReferenceOccurrence {
    pub path: Vec<quire_semantic_value::admission::PathStep>,
    pub population: quire_exact::NodeKey,
}

pub struct ProtocolSuppliedValue {
    pub value: qsl_eval::value::SuppliedValue,
    pub references: Vec<ProtocolReferenceOccurrence>,
}

pub struct ProtocolSupply<'view> {
    pub binders: Vec<(quire_exact::NodeKey, ProtocolSuppliedValue)>,
    pub observation: Option<&'view quire_semantic_value::admission::ObservationAdmission<
        qsl_package::observation::AdmittedProtocolObservation<'view>,
    >>,
}
```

`path` is relative to the source value root and uses the shared typed field,
option-value and index steps; `population` is the checked declaration node
key in the selected model. An empty path therefore describes a root
reference. `value` is the existing QSL supplied-value type; these wrappers add
only the occurrence-to-declaration association and do not define a new value
grammar, result type or checker. `ProtocolSupply` remains the sole supply
accepted by `evaluate_protocol_slot`.

The public entry remains the S6a protocol-slot method, with the existing
failure and meter types:

```rust
pub fn evaluate_protocol_slot(
    &self,
    slot: &qsl_eval::value::ProtocolSlot,
    supply: &qsl_eval::value::ProtocolSupply<'_>,
    meter: &mut quire_exact::Meter,
) -> Result<qsl_eval::value::ProtocolSlotEvaluation, quire_eval::CallFailure>;
```

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
- `CallFailure::Admission(Box<CallAdmissionFailure>)` for a selection or
  supply refused or left incomplete before S6a. `CallAdmissionFailure` is the
  one closed `Refused`/`Incomplete` result owned by quire-eval; each variant
  carries the original shared QSV `AdmissionRecord` and the checked-package
  locus, including a population node where completeness fails, as shared
  FR-130 requires. No QSL-owned mirror call-admission record or fault type is
  introduced, and nothing is charged.
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
CallFailure>` with exactly these success shapes: `ProtocolSlotEvaluation::Evaluated`
and `ProtocolSlotEvaluation::Unavailable`; a selection or supply refused or
left incomplete before S6a is `CallFailure::Admission(Box<CallAdmissionFailure>)`,
and an S6a invariant break is `CallFailure::Fault(InternalFault)`. The protocol
slot is selected only by this call: `run_clause` and FR-312's request carry no
protocol slot.

### One admitted checked-term view

QSL SHALL define one sealed `qsl_eval::value::CheckedTermView` contract in
layer 5. Its concrete layer-4 input carrier SHALL be
`qsl_package::evaluation_view::AdmittedEvaluationView<'a>`, with private variants
that borrow either `&'a CheckedPackage` or the successful I2 read's retained
admitted package, source map, effective limits and admitted dependency closure.
The reader variant MAY retain plain kernel values/types and read descriptors
projected from that witness; it SHALL retain no newly constructed checked term.
The in-process entry SHALL be `CheckedPackage::evaluation_view(&self) ->
AdmittedEvaluationView<'_>`; the successful I2 read SHALL expose
`evaluation_view(&self) -> Result<AdmittedEvaluationView<'_>, ViewFailure>` over its retained
`Arc<quire_contract_model::CheckedPackageV2>`. The reader-result entry and its
IR variant SHALL exist only with `std`. The view contract and in-process
variant SHALL introduce no IR or `std` dependency into the no_std path.

The reader's retained admitted package is the witness: a caller SHALL NOT
construct the reader variant from `CheckedSemanticGraphV2`, `CheckedTerm`,
`ImportView`, bytes, or a deserialized value. A refused, limited, cancelled or
faulted I2 read SHALL yield no view. The view SHALL retain the reader's exact
identity, source-map and dependency associations for its entire lifetime.
Exposing the successful reader result does not change I2 admission or its
existing staged refusal vocabulary.

Under `std`, `qsl-package` SHALL expose its existing
`read_checked_package_v2(bytes: &[u8], identity: LibraryName,
limits: V2ReadLimits, evidence: &CheckedPackageEvidence) -> V2ReadOutcome`
and the successful `V2Read` carrier, with its fields private. `V2ReadOutcome`
SHALL remain `Result<Staged<V2Read>, StageFailure<V2ReadRefusal>>`.
`V2Read::evaluation_view(&self)` SHALL be the reader-result entry above.
This exposes the existing admission result; it does not add a second reader.

The layer-5 `CheckedTermView` SHALL select a slot with
`select_protocol_slot(&self, slot: &ProtocolSlot) ->
Result<SelectedProtocolTerm<'_>, CallFailure>`. `SelectedProtocolTerm` SHALL
have private fields and read accessors for its expression handle and ordered
source descriptors. Each descriptor SHALL expose the admitted binder's
`NodeKey`, level, `ValueType` and read location. The expression handle
`TermHandle<'a>` SHALL be a layer-4 opaque value, tied to the exact borrowed view and owning
body, and unavailable from an integer, a node key, a free-standing term or
another view. Operand handles SHALL retain that same owner; dependency calls
SHALL resolve within the witness's exact admitted closure. Slot selection and
source ordering SHALL obey the selection and supply admission rules below.
`ProtocolSlot`, `SelectedProtocolTerm`, source descriptors and `CallFailure`
SHALL remain layer-5 inputs and results; layer 4 SHALL import none of them.

The layer-4 view SHALL expose
`read_node<'v>(&'v self, handle: &TermHandle<'v>) ->
Result<quire_checked::NodeRead<'v, TermHandle<'v>>, InternalFault>`.
The canonical result and its operation vocabulary SHALL be exactly
[FR-400](FR-400-separate-checked-terms-from-the-checker.md)'s `NodeRead` and
generic `NodeKind`, including their borrowed literal, type and location.
Every child position of the returned `NodeKind` SHALL contain a handle of
that same view and owning expression body; call metadata SHALL retain the
exact admitted callee identity and closure association. `SelectedProtocolTerm`
SHALL expose `root(&self) -> &TermHandle<'_>` and
`sources(&self) -> &[ProtocolSource]`; `ProtocolSource` SHALL expose
`binder(&self) -> &NodeKey`, `level(&self) -> usize`,
`value_type(&self) -> &ValueType` and `location(&self) -> &Location`.
An unavailable literal or type projection SHALL prevent creation of the view,
never become a late partial `NodeRead` or a successful default value.
This is a read projection of admitted data; it SHALL neither build a
`CheckedBody`, `Node` or another checked proof nor call `construct`. The
in-process and reader projections SHALL be consumed by the existing S6a
`Machine`, using the same operation execution, function and dispatch lookup,
model reads and family-result mapping. The bridge SHALL add no interpreter,
family-dispatch table, checking pass, source recompilation or serialized-term
execution path. The sealed `CheckedPackageEvaluation` seam SHALL evaluate
`AdmittedEvaluationView` as well as `CheckedPackage`; the latter SHALL
delegate slot evaluation through its view to the same `ProtocolClause` hook.

View creation, term projection and slot/supply admission SHALL charge no
evaluation counter. Once evaluation starts, the same caller meter SHALL reach
the existing machine unchanged. Admission failures SHALL retain their code,
cause and node/member/binder locus with no charge; an impossible foreign
handle or broken admitted projection SHALL be an `InternalFault`, never a
successful value or a new admission authority.

The bridge SHALL preserve ADR-011 §6.1's layer edges and ADR-016's execution
arrows: byte admission and typed projection live in layer 4, execution lives
in layer 5, and the shared checked-term vocabulary stays below both. It SHALL
NOT enable `quire-checked/construct` outside FR-400's existing allowlist or
place IR types in `quire-checked`, `quire-eval` or the no_std evaluation closure.

### Typed projection admission and upstream dependencies

Reader view creation SHALL resolve all expression, callable and binder types,
literal payloads, operation members and location associations in the admitted
closure before yielding a view. Resolution SHALL be fallible: IR's general
value-type vocabulary SHALL NOT be presumed to represent every kernel
`ValueType`. The view SHALL use the existing kernel `ValueType` as its only
evaluation type vocabulary, with no QSL-owned type mirror, guessed identity
or fallback to an unchecked or serialized interpretation.

`ViewFailure` SHALL be the layer-4 closed enum
`Unsupported(UnsupportedView)`, `Limit(LimitExceeded)` or `Fault(InternalFault)`.
`UnsupportedView` SHALL carry the catalog pair
`unknown_required_feature`/`unsupported-feature`, the owning package identity,
the affected node's `NodeKey`, its `Location`, and one typed reason:
`MissingTypeRepresentation`, `MissingLiteralProjection`,
`MissingOperationProjection`, `MissingBindingProjection` or
`MissingChargeSemantics`.
If more than one projection is unsupported, view creation SHALL report the
first by ascending package identity, node key and the reason order above.
This is evaluation-input admission before S6a, category unsupported, exit 21;
it SHALL yield no partial view and SHALL leave the successful I2 read intact.
View creation SHALL accept no `Meter`; it SHALL neither create, replace nor
charge the caller's evaluation meter. Once a view exists, `read_node` SHALL
return a complete canonical read or an invariant `InternalFault`, never a
late unsupported projection. Projection storage and traversal SHALL remain
within the retained I2 resource ceilings; exhaustion SHALL yield no view and
return `ViewFailure::Limit` with the existing I2 limit vocabulary, never
`Unsupported`. A failed I2 read itself retains its existing staged result.

The upstream dependencies are tracked as IR-1193 (public typed literal,
binding-name and operation/member reads), IR-969 (checked-term placement),
IR-1194 (semantic-type and exact identity joins), and STD-611 (lossless
bare/explicit-dereference semantics). FR-400 owns the canonical checked-term
read vocabulary. The projections require protocol slot and binder
identities/levels/types, exact source occurrences and callable/dispatch closure
associations. IR's `expression::ValueType` is separate from the canonical
evaluation target `quire_exact::ValueType`; neither a type-name match nor a
structural `CheckedNodeId` is an admitted identity conversion. IR-1194 must
supply exact `NodeKey`/`EffectiveId` associations for Composite and Reference
and unit identities for Quantity. General coverage of String, Uuid, Timestamp,
Dimension, Alias and Union requires explicit shared-type extensions wherever
the canonical target cannot represent the admitted type; nested Option and
Collection types require the same joins recursively. These tracker references
record dependencies, not new spec-artifact edges or upstream API signatures.

The reader projection SHALL preserve every evaluation- and charge-relevant
operation flag, including `NodeKind::Attribute.derefed`. A bare object-field
read and an explicit `deref` of the same reference SHALL retain FR-107's
distinct charge logs through emission, admission and `NodeRead`, even when
their completed values are equal. If the admitted representation has erased
that distinction, then view creation SHALL return `MissingChargeSemantics`
with no partial view; the required AC-11 comparison remains unmet. The bridge
SHALL NOT infer explicitness from source spans, source text, a guessed flag,
recompilation or an otherwise identical semantic graph. STD-611 and the owning
IR contract determine the lossless encoding; this requirement prescribes no
new wire field, operation or mode.

For AC-11 and TC-1073, the upstream extensions SHALL at least cover Boolean,
bounded Integer, the Receipt/Attempt/Flag/Tick record types with exact member
and declaration identities, Node's object type, and Reference<Node> with its
exact model/object identity, as TC-1070 fixture E defines. A type that QSL emits and evaluates
remains subject to complete AC-11 parity: an unsupported projection of such a
type SHALL count as an unmet dependency, not a passing parity case or a scope
reduction. Unrelated IR types that QSL cannot evaluate MAY return the typed
unsupported admission above. A source-backed call or JSON inspection SHALL
NOT stand in for reader-backed parity. Projection MAY traverse already
admitted typed data, but SHALL NOT repeat admission or give a free-standing
decoded term checked authority.

### Charge points and their order

Within one call the evaluator charges, in this order: the nodes of the slot's
expression in evaluation order (left to right, short-circuited operands
skipped), a read of a supplied source being one node; `model.navigate` for each
field read of an object and `model.deref` for each dereference, as FR-107
states, which occur only in a slot that may read model state; and the existing
S6a result-retain charges when the expression reaches their charge points.
Returning a forwarded scalar or a literal constant adds no result-retain
point: its `result_units` usage is zero by S6a when no existing retain point
was reached. Protocol slots add no counter or charge point. Admission charges
nothing. A capture slot charges its initializer's nodes in its own call and no other call charges
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
   Admission SHALL then match the supplied value's reference occurrences to
   that binder's `ProtocolReferenceOccurrence` entries in FR-106 value-walk
   order. Every occurrence SHALL have exactly one entry at its exact nested
   path. An omitted path SHALL refuse `invalid_runtime_input`/`missing-member`;
   a repeated path SHALL refuse `invalid_runtime_input`/`duplicate-member`;
   an entry whose path is extra or does not end at a reference SHALL refuse
   `invalid_runtime_input`/`unknown-member`; and an entry whose declaration
   does not name a population in the exact selected model, whose effective
   member type does not conform to the reference target, or whose normalized
   universe/effective target type disagrees with the declaration SHALL refuse
   with the existing `invalid_model_binding`/`wrong-model-selection` or
   `invalid_runtime_input`/`wrong-value-kind` respectively. A genuinely
   foreign reference model retains `foreign_reference`/`foreign-universe`.
   The declaration identity is checked even for an empty population, an
   unlisted population, or a key absent from the snapshot; object presence
   SHALL NOT supply or repair provenance. A source that is `Unavailable` has
   no reference occurrences and SHALL have no occurrence entries. These
   checks introduce no catalog code or cause.
6. If a supplied value contains a reference and `observation` is none, then admission
   SHALL return `Incomplete` with `unavailable_observation`/
   `missing-required-artifact`, naming the first such binder in the source
   order above. When observation admission is required, the call SHALL inspect
   the supplied typed result after checks 1 through 5: `Refused` and
   `Incomplete` retain their respective failure kinds and the entire original
   QSV admission record; `Fault` returns the original `InternalFault` through
   `CallFailure::Fault`; no non-admitted arm yields an authority. For
   `Admitted`, admission SHALL verify the authority's association with this
   evaluated view and the protocol's selected model using FR-106's retained
   provenance, never infer it from the objects that happen to be present.
   A wrong model retains `invalid_model_binding`/`wrong-model-selection`.
   Admission SHALL use each occurrence's exact checked population declaration
   as the initial population provenance for that supplied reference. It SHALL
   combine those supplied-value facts with the authority's parsed object
   fields, relationship extents, population manifest and normalized model
   associations, and reuse FR-106 checks 7 and 8 for exactly those initial
   populations and their recursive required closure. It SHALL NOT derive an
   initial population from a normalized universe/effective-type pair or a
   matching object key. A
   required incomplete population returns `Incomplete` with
   `incomplete_population`/`incomplete-scope` naming its population node and
   retaining the full FR-106 record, even if the referenced object is present.
   An unrelated incomplete population has no effect. Only after completeness
   succeeds SHALL FR-106 check 8 decide absent targets in complete populations
   as `dangling_reference`/`absent-target-in-complete-population`. These checks
   operate on the authority's retained admitted facts, read no bytes, perform
   no expression evaluation and leave the caller's meter unchanged.

A source supplied as `Unavailable` is admitted by checks 4 and 5 without a
value; check 6 does not apply to it.

### Evaluation

- The evaluator SHALL evaluate the slot's expression term over the supplied
  values, with the charges FR-107 and FR-100 state for the same expression
  nodes under the ten accounting counters of FR-255, and SHALL add no counter
  and no charge point. A completed value is retained and charged only at the
  existing S6a retain points of its expression. A forwarded scalar or literal
  constant that reaches none has zero `result_units` by S6a; returning it from
  a protocol slot does not add a retain charge.
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
| FR-390-AC-9 | With usage `U` from a completed call, a meter with every charged counter at exactly its measured usage completes; with each charged counter in turn one below its usage and the others at theirs, the call returns `Incomplete` naming that counter with no value; a zero `work_units` returns `Incomplete`; a fresh meter completes the same call with the same value. Forwarded scalars and literal constants with no existing retain point have zero `result_units` by S6a; tests assert those zero usages independently of the charged-counter boundary loop. | Test (TC-1073) |
| FR-390-AC-10 | One meter passed to a compensation's registration capture, activation capture, trigger guard, retry relation and recovery predicate in turn accumulates exactly the sum of the calls' charges; a `work_units` budget of that sum completes all of them and one below it returns `Incomplete` `work_units` at the last call; the calls in another order give the same total. | Test (TC-1073) |
| FR-390-AC-11 | Running one call twice gives equal results and charge logs; a call over the package the contract reader returns for FR-383's emitted nodes gives the same result and charge log as over the checked package that was emitted; a refused call leaves the meter as it was. | Test (TC-1073) |
| FR-390-AC-12 | A package whose capture initializer reads itself, a later capture or another template's capture is refused at S3 (FR-381) and no package is built; a package mutated in its bytes so an activation capture's initializer reads its own binder, or another template's capture binder, is refused by the contract reader with `missing_declaration`/`missing-name` (`read` `forward` and `foreign`), and no evaluation call is possible. | Test (TC-1074) |
| FR-390-AC-13 | With the S6a probe variant of FR-107-AC-6 added, the build fails with E0004 at exactly the checked-in S6a seam list: the arm serving state clauses and protocol slots is the one `ProtocolClause` arm. | Test (TC-1074) |
| FR-390-AC-14 | A consumer can obtain a view from an in-process package and from a successful I2 reader result. Construction of a reader view or expression handle from a decoded graph, free-standing term, `ImportView`, raw bytes, integer or node key fails to compile. A refused, limited, cancelled or faulted read exposes no evaluation view. | Test |
| FR-390-AC-15 | For every TC-1070 slot, AC-11 compares the in-process view with a view obtained by emitting and actually admitting that package's bytes: the complete result (including value, unavailable binder, catalog code/cause, location and losses) and every ordered charge-log entry are equal. Repeat for unavailable and short-circuited supplies, each AC-9 counter boundary, and AC-10's shared-meter sequence; each selection/supply refusal has the same locus and leaves its meter unchanged. No fixture recompiles source to produce the reader-side evaluation. | Test |
| FR-390-AC-16 | Inspection finds one slot-selection view contract and one existing machine/ProtocolClause execution path for both input variants, using FR-400's single generic `NodeKind` read vocabulary, no checked-term reconstruction, no new `construct` caller and no second family dispatch. With `std` off the in-process view compiles without IR; the reader-result entry is absent, and the existing no_std closure acquires no dependency on IR or `std`. | Inspection |
| FR-390-AC-17 | An admitted IR type without a kernel representation yields `ViewFailure::Unsupported` before any view exists, with `unknown_required_feature`/`unsupported-feature`, the affected package/node/location and `MissingTypeRepresentation`; no meter is accepted or charged. A missing literal, operation or binding projection yields its corresponding typed reason. A projection-resource limit yields `Limit`, no partial view. A handle from another owner returns an invariant fault rather than a node. | Test |
| FR-390-AC-18 | Reader views resolve Boolean, bounded Integer, record/composite and Reference<Node> identities of TC-1070 fixture E to the exact kernel values/types of the in-process view, including recursively nested Option/Collection values used by a slot. AC-11/15 fail if a required QSL-emitted type is reported unsupported; unsupported cases cannot replace any required result/log comparison. | Test |
| FR-390-AC-19 | A bare `view.total` read and an explicit `deref(view).total` read over the same observation retain their respective in-process charge logs through emitted-package admission and `NodeRead`, including FR-107's explicit-dereference charge distinction. If the representation erases that distinction, view creation returns `MissingChargeSemantics` before a view or meter charge exists, and AC-11 remains unmet; equal completed values or an unsupported result cannot pass the required parity comparison. | Test |
| FR-390-AC-20 | TC-1071 row 20 uses a successful immutable observation authority in the typed result; although `n5` exists, `nodes` incomplete returns `CallFailure::Admission` with category incomplete, the population locus and the unchanged QSV FR-106 record. An unrelated incomplete population does not alter the successful call. The meter is unchanged on either admission failure. | Test |
| FR-390-AC-21 | TC-1071 row 21 supplies the `Refused` result of FR-106 current-snapshot admission for another model and returns the original `invalid_model_binding`/`wrong-model-selection` record without a foreign-reference replacement. `Incomplete` preserves its kind/record and `Fault` preserves its `InternalFault`; none supplies successful authority or charges the meter. | Test |
| FR-390-AC-22 | A malformed slot, duplicate source, unknown source, missing source and wrong-shaped value combined with a refused observation result return the corresponding check-1-through-5 failure before the observation result is inspected. With those defects absent, a reference-valued source and no observation returns the existing unavailable-observation record; a present failed result returns its original arm. All failures leave the caller's existing meter unchanged, and TC-1071 row 22 charges only its successful evaluation. | Test |
| FR-390-AC-23 | TC-1070's E-P procedure carries one exact population declaration for every root and nested reference occurrence in a supplied source, including record, present-option and sequence paths. Its `Receipt.peer: Reference<Other>` is present in the supplied forward `Receipt` despite the selected initializer reading only `forward.total`; `Other` is not a direct protocol context, while `otherNodes` and `otherSpare` both declare `Other`. A missing, duplicate, extra or non-reference path refuses with the existing cause and typed path before observation inspection; a declaration from another model refuses `invalid_model_binding`/`wrong-model-selection`; wrong normalized identity/type refuses with its existing cause. A ghost key explicitly attributed to an empty `otherSpare` marked incomplete, or to an unlisted `otherSpare`, returns `incomplete_population`/`incomplete-scope` at that declaration node even if a matching object is present in `otherNodes`; the same ghost against a complete empty `otherSpare` returns `dangling_reference`/`absent-target-in-complete-population` for `otherSpare`. An unrelated incomplete population leaves the call admissible. Completing the seeded population then exposes any dangling reference in the recursive reference-field or relationship-end closure in FR-106 walk order. Every refusal/incomplete result retains the original shared FR-106 `AdmissionRecord`, and the meter is unchanged. | Test (TC-1070) |

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

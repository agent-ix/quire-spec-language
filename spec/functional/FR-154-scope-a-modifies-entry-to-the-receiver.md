---
id: FR-154
title: "Scope a modifies entry to the receiver and enforce it in the one frame decision"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
---
# FR-154: Scope a modifies entry to the receiver and enforce it in the one frame decision

## Description

A `modifies` entry has a scope: `receiver`, written `modifies self.f`, grants
writes to field `f` on the operation's receiver only; `every-object`,
written `modifies f`, grants them on every object whose effective type has
`f`, QSpec FR-013's meaning (ADR-021 POR-3, RU-1). QSL SHALL read the scope
at intake, carry it through the checked package and the S4 frame node, and
enforce it wherever a frame is decided: `population::decide_frame`, the one
frame decision that FR-106 check 11 for an invocation (FR-115) and the
invocation admission `enforce_frame` call, and that FR-120 calls over its
in-memory states; and in FR-120's candidate generation. A change to `f`
on any object other than the receiver under a `receiver` entry is
`frame_violation`. A `deletes` entry has the matching scope: type-wide
`deletes T`, receiver `deletes self`, or argument `deletes p` for a reference
parameter `p`.

## Use case

A verification operator scopes `bump()`'s frame to its own counter. The
model checker then knows two bumps on different counters cannot interfere,
which partial-order reduction needs, and any step that writes another
counter's `v` is reported as a frame violation (US-019).

## Inputs

- A domain package whose operation frames carry a scope on each `modifies`
  and `deletes` entry (QSpec FR-013 with the receiver-scope amendment).
- The model-document form of that scope, which QSL intake reads. The `frame`
  of an operation is a closed object with exactly these optional members; an
  absent member is empty or false:
  - `modifies`, `creates`, `deletes`: required arrays of node identity
    strings, as before, each present and empty when empty. They mean `every-object` field entries (or relationship
    entries), `creates` entries and type-wide `deletes` entries.
  - `modifies_receiver`: an array of field node identity strings, the
    receiver-scoped `modifies self.f` entries.
  - `deletes_receiver`: a boolean, `true` for `deletes self`.
  - `deletes_parameters`: an array of parameter-name strings, the
    `deletes p` entries.

  A frame that omits `modifies`, `creates` or `deletes` is malformed.

  The model contract's Frames row spells the three scoped members. Intake
  validates a frame against the semantic-IR frame schema of
  filament-core-data before it reads the frame, so that schema SHALL admit
  the three members as optional members with the types above; intake reads
  them only with that schema in place.

  A member the list does not name, a member of the wrong JSON type, or an
  array element that is not a string refuses `invalid_model_binding`/
  `malformed-declaration` at the member.

## Outputs

```rust
// qsl_semantics::model::domain_package
pub struct OperationEffect {
    /// `modifies f`: field members written on every object (every-object).
    pub modifies: Vec<DeclarationKey>,
    pub creates: Vec<DeclarationKey>,
    /// `deletes T`: object types deleted type-wide.
    pub deletes: Vec<DeclarationKey>,
    /// `modifies self.f`: field members written on the receiver only.
    pub receiver_modifies: Vec<DeclarationKey>,
    /// `deletes self`.
    pub deletes_receiver: bool,
    /// `deletes p`: names of reference parameters whose bound argument is
    /// deleted.
    pub deletes_arguments: Vec<String>,
}

// qsl_semantics::model::population (crate-internal)
pub(crate) struct FrameDecision<'e> {
    pub(crate) index: &'e ModelIndex,
    pub(crate) ancestor_steps: u64,
    pub(crate) effect: &'e OperationEffect,
    pub(crate) declared_created: &'e [String],
    pub(crate) declared_deleted: &'e [String],
    /// The receiver's identity in the population being decided, when the
    /// receiver lies in it.
    pub(crate) receiver: Option<&'e str>,
    /// `(parameter name, object identity)` for each reference argument
    /// bound to an object of the population being decided.
    pub(crate) arguments: &'e [(String, String)],
}

pub(crate) struct FrameObject<'a, V> {
    pub(crate) object: &'a str,
    pub(crate) type_identity: DeclarationKey,
    pub(crate) fields: BTreeMap<DeclarationKey, V>,
}

pub(crate) struct FrameDelta {
    /// Identities in post and not in pre.
    pub(crate) created: BTreeSet<String>,
    /// Identities in pre and not in post.
    pub(crate) deleted: BTreeSet<String>,
}

pub(crate) fn decide_frame<V, E>(
    frame: &FrameDecision<'_>,
    pre: &[FrameObject<'_, V>],
    post: &[FrameObject<'_, V>],
    values_equal: impl Fn(&DeclarationKey, Option<&V>, Option<&V>) -> bool,
    charge: &mut impl FnMut(FrameCharge) -> Result<(), E>,
) -> Result<FrameDelta, FrameStop<E>>;

// FrameStop::Refused(ModelRefusal) or FrameStop::Denied(E), the first
// charge the caller's meter denied.
```

The scope of a `modifies` entry is the `OperationEffect` field that holds it:
an entry in `modifies` is every-object and an entry in `receiver_modifies` is
receiver-scoped. A relationship `modifies` entry has no scope and is held in
`modifies`.

## Behavior

- **Intake.** Intake SHALL read each `modifies` entry's scope, `receiver`
  or `every-object`, by the `OperationEffect` field it lands in
  (`receiver_modifies` or `modifies`), and each `deletes` entry's scope
  (type-wide in `deletes`, receiver in `deletes_receiver`, argument in
  `deletes_arguments`), as QSpec FR-013 defines them. It SHALL resolve a `modifies` entry's field as FR-103 does.
  It SHALL check the frame's member shapes first, then the receiver rule
  below, then the entries in this order: `modifies`, `modifies_receiver`,
  `creates`, `deletes`, `deletes_parameters`, each in document order,
  reporting the first that fails, with these refusals:
  - An operation declared by a process or by a persistence interface has no
    object receiver: the receiver of a process operation is a run, and a
    persistence interface is no object type. Such an operation refuses a
    non-empty `modifies_receiver` and `deletes_receiver: true` with
    `invalid_model_binding`/`malformed-declaration` at that member
    (`modifies_receiver` first when both are present), before any entry is
    read, so no entry of the member is resolved (QSpec FR-013). A clause that
    names a process operation binds its `self` to the run (FR-104), and the
    frame it takes holds no receiver-scoped entry because intake refused one
    here. An
    empty `modifies_receiver`, `deletes_receiver: false` and
    `deletes_parameters` read as for an object type. QSpec's Frames row refuses
    the same forms on a state machine's operation, and that row is its
    contract.
  - A `modifies_receiver` entry SHALL name a field among the fields, own and inherited, of the
    operation's declaring object type. An entry that names
    no declaration of the package refuses `missing_declaration`/`missing-name`
    at the entry. An entry that names a declaration of any other meaning, a
    relationship, an object type, or a field of a type the declaring object
    type does not conform to, refuses `invalid_model_binding`/
    `malformed-declaration` at the entry. A relationship `modifies` entry has
    no scope.
  - A `deletes_parameters` entry SHALL name a parameter of the operation whose type is
    `Reference<T>`. A name that is no parameter, or a parameter of any other
    type, refuses `missing_declaration`/`missing-name` at the entry (QSpec
    FR-340 states the same refusal for the wire entry). `deletes_receiver: true` is
    always admitted for an operation of an object type.
  - After every entry of the frame resolves, a `modifies` entry and a
    `modifies_receiver` entry for one field SHALL refuse `invalid_model_binding`/
    `conflicting-binding`, naming both entries and the operation, as QSpec
    FR-013 states, located at the later of the two in document order; intake
    never merges them. A field written twice with one scope, and a scoped
    `deletes` written twice, are one entry.
- **Emission.** S4 SHALL emit the `state`/`frame` node's body (FR-105) as
  QSpec FR-340 states it. Each field entry is `{kind: "field", declaration,
  name, scope}`, `declaration` being the node of the object type that
  declares or inherits the field (the receiver-scoped entry names the same
  node an every-object entry for that field names) and `scope` being
  `receiver` or `every-object`. A type-wide `deletes T` entry stays in
  `deletes`, as a node reference. `deletes self` is the `deletes_scoped`
  entry `{kind: "receiver"}` and `deletes p` is `{kind: "parameter", name:
  "p"}`. `deletes_scoped` is always present and is the empty array when the
  frame has no scoped delete, and it is sorted with the `receiver` entry
  first and then the `parameter` entries by name as UTF-8 bytes. `modifies`
  keeps FR-105's order, (declaring node digest, field name, scope with
  `every-object` first), as QSpec FR-340 states; intake admits no two
  entries for one field, so the scope never decides the position of an
  emitted entry. A `deletes_scoped` entry names no node, so it
  adds no `dependencies` entry. Two frames that differ only by an entry's
  scope, or only by a scoped `deletes`, SHALL have different node content and
  so different node ids.
- **Decision.** `decide_frame` decides one population's `pre` and `post`
  objects, both in document order, under `FrameDecision.effect`. The caller
  passes the receiver's identity in that population as
  `FrameDecision.receiver` (none when the receiver lies in another
  population) and the reference arguments bound to objects of that
  population as `FrameDecision.arguments`. It returns the first of these
  findings, in this order, as `FrameStop::Refused`:
  1. an object in `post` and not in `pre` whose type conforms to no `creates`
     entry, or a surviving object whose most-specific type differs;
  2. an object in `pre` and not in `post` that no `deletes` entry grants;
  3. a field of a surviving object whose value changed and that no `modifies`
     entry grants.

  The findings of steps 1 to 3 are `frame_violation`/`unauthorized-change`,
  each naming the object, with the field for a write and the type for a
  creation or deletion. Failing all three, `decide_frame` compares the
  caller's declared created and deleted identities with the computed sets and
  refuses `population_delta_mismatch`/`delta-disagreement` on a difference;
  otherwise it returns the computed `FrameDelta`.

  For a surviving object `o` and field `f`, a change of `f` from pre to post
  is granted when `f` is a member of `effect.modifies`, or is reached from
  such a member through `redefines` (FR-013), or when `o` is the receiver and
  `f` is a member of `effect.receiver_modifies` or reached from one through
  `redefines`. Every other change is the step 3 finding. `creates` keeps its
  meaning. A deletion is granted when any one entry grants it: a type-wide
  `deletes T` grants any subset of the objects whose type conforms to `T`;
  `deletes_receiver` grants the receiver only; a `deletes_arguments` name `p`
  grants the object `p` is bound to in `FrameDecision.arguments` and no
  object when `p` has no entry there. Entries of the three scopes in one
  frame add up. A `Receiver` entry and `deletes_receiver` grant nothing when
  `FrameDecision.receiver` is none. The meter charge a write makes
  (`frame.write`, then `frame.redefines-link` per link followed) is
  `FrameStop::Denied` with no verdict when the caller's meter denies it.
- **Arguments.** `FrameDecision.arguments` maps each reference parameter of
  the application to the object it is bound to, within the population being
  decided.
- **One decision.** `enforce_frame` (reached from `admit_invocation`) and
  FR-106's `observation::frame::enforce` (check 11, and through it FR-115's
  `Frame` run) each call `decide_frame` once per population, so the two give
  the same verdict for the same effect, receiver, arguments, pre and post.
  FR-106 check 11 reads the receiver from the invocation's `self` binding and
  the arguments from the invocation's reference parameters.
  FR-120's frame decision calls it the same way over its in-memory states.
- **Candidates.** FR-120's post-state construction SHALL range a field that
  a `Receiver` entry grants over its domain on the receiver only, and keep
  its value on every other object.
- **Domains.** A field that only `Receiver` entries grant SHALL still be a
  root of `ModelSystem::domains()`, as FR-120 lists granted fields.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-154-AC-1 | The `test/counters` package of FR-120 with `increment()` framed `modifies self.value`, state `w` holding `c1` and `c2` at `value` 0, and no clause: `increment` on receiver `c1` has three successors, each changing only `c1.value`, and `c2.value` is 0 in every one. With the frame `modifies value` (every-object), the same application has nine successors, ranging both objects' `value`. | Test (TC-572) |
| FR-154-AC-2 | `decide_frame` with effect `modifies self.value`, receiver `c1`, no arguments, pre `w` and a post with `c2.value` 1 is refused `frame_violation`/`unauthorized-change` naming `c2` and `value`; the same post with receiver `c2` returns a `FrameDelta` with empty `created` and `deleted`; with effect `modifies value` both return a `FrameDelta`. | Test (TC-572) |
| FR-154-AC-3 | FR-115's `Frame` run of an invocation of `increment` on `self = c1` whose post snapshot changes `c2.value` settles `violation` with the frame witness naming `c2.value` under `modifies self.value`, and `success` under `modifies value`. The FR-106 clause-run admission of the same invocation refuses `frame_violation` under `modifies self.value`. | Test (TC-573) |
| FR-154-AC-4 | The S4 frame node of `increment` under `modifies self.value` carries the scope `receiver`, and under `modifies value` the scope `every-object`; the two nodes' contents differ. A frame whose `modifies_receiver` and `modifies` both name `value` is refused at intake with `invalid_model_binding`/`conflicting-binding` naming both entries and `increment`, and no frame node is emitted for it. | Test (TC-573) |
| FR-154-AC-5 | An operation `drop()` framed `deletes self` on receiver `c1`: `decide_frame` returns a `FrameDelta` for a post without `c1` (declared deleted `c1`) and refuses a post without `c2` as `frame_violation`/`unauthorized-change` naming `c2`; framed `deletes Counter`, both posts and a post without either return a `FrameDelta`; the S4 frame nodes of the two frames differ. | Test (TC-572) |
| FR-154-AC-6 | The S4 frame body of `bump()` framed `modifies self.step`, `modifies value` and `deletes self` over `test/counters` is exactly TC-1043 vector W1; the body of `merge(a, b)` framed `deletes b`, `deletes a`, `deletes self` and `deletes Counter` is exactly vector W2, where `deletes_scoped` lists the receiver first and then `a` before `b` whatever the authored order, and `deletes` holds the `Counter` node reference; a frame with no entry has all four members empty (vector W3). | Test (TC-1043) |
| FR-154-AC-7 | Frames that differ only by one entry's scope (`modifies self.value` against `modifies value`, vectors W4 and W5), or only by `deletes self` against no entry (vectors W6 and W3), have different frame node ids, and two packages differing only by that one scope have different `package_id`s; the frame node of a frame with only type-wide entries carries `deletes_scoped: []` and every field entry `every-object`. | Test (TC-1043) |
| FR-154-AC-8 | Over TC-1043's fixture, intake reads the frame members of rows I1 to I22: a `modifies_receiver` entry naming no declaration refuses `missing_declaration`/`missing-name` (I1); one naming a field of `Gauge`, a type `Counter` does not conform to, or a relationship refuses `invalid_model_binding`/`malformed-declaration` (I2, I3); a `deletes_parameters` name that is no parameter, or the name of an `Integer` parameter, refuses `missing_declaration`/`missing-name` (I4, I5); `deletes_parameters` naming a `Reference<Counter>` parameter with `deletes_receiver` true admits (I6), and so does `modifies_receiver` naming `value` on an operation of `Sub`, which inherits it (I7); a frame whose `modifies_receiver` holds a missing field and `value` while `modifies` holds `value` reports only the `missing-name` refusal (I8); `modifies` and `modifies_receiver` both naming `value` refuse `conflicting-binding` (I9); a non-boolean `deletes_receiver`, an unnamed member and a frame omitting `deletes` refuse `malformed-declaration` (I10, I11, I12); over the process `Flow`, a non-empty `modifies_receiver` or `deletes_receiver: true` refuses `invalid_model_binding`/`malformed-declaration` at the member, `modifies_receiver` first when both are present (I13, I14, I17), and an empty `modifies_receiver` with `deletes_receiver: false`, or `deletes_parameters` naming a `Reference<Counter>` parameter, admits (I15, I16); the same holds over the persistence interface `Store`, whose operation has no object receiver: a non-empty `modifies_receiver` or `deletes_receiver: true` refuses `invalid_model_binding`/`malformed-declaration` at the member, `modifies_receiver` first when both are present, before any entry is read (I18, I19, I22), and an empty `modifies_receiver` with `deletes_receiver: false`, or `deletes_parameters` naming a `Reference<Counter>` parameter, admits (I20, I21). | Test (TC-1043) |
| FR-154-AC-9 | `decide_frame` over a population holding `c1`, `c2` and `c3` of `Counter`, all `value` 0 and `step` 0, with effect `receiver_modifies` `value` and `modifies` `step`, receiver `c1`, returns a `FrameDelta` with empty `created` and `deleted` for a post that changes `c1.value` and `c2.step`, and is refused `frame_violation`/`unauthorized-change` naming `c2` and `value` for a post that changes `c2.value`. With `deletes_receiver` true, `deletes_arguments` `a`, receiver `c1` and `a` bound to `c2`, a post without `c1` and `c2` (declared deleted `c1`, `c2`) returns a `FrameDelta` whose `deleted` is {`c1`, `c2`}, and a post without `c3` is `frame_violation`/`unauthorized-change` naming `c3`; with `deletes_arguments` `a` alone a post without `c1` is `frame_violation`/`unauthorized-change` naming `c1`. | Test (TC-1043) |
| FR-154-AC-10 | The I2 reader's frame step refuses, as QSpec FR-340 states each: a field entry with no `scope` or a `scope` other than `receiver` or `every-object`, and a frame body with no `deletes_scoped` (schema failures of the frame body, each read after the edited node and its dependents are re-keyed, as TC-1043 step 4 states); a `deletes_scoped` array with the `parameter` entry for `a` before the `receiver` entry, or with `b` before `a` (`invalid_semantic_graph`); and, in the state step rather than the frame step, a `parameter` entry naming a parameter that is not a reference parameter of every operation whose anchor references the frame (`missing_declaration`/`missing-name`, located at the frame node, QSpec FR-342 check 5). A `modifies` array holding both scopes of one field, in either listed order, refuses `invalid_model_binding`/`conflicting-binding` and not `invalid_semantic_graph` (X8, X9). The body of vector W1 is admitted. | Test (TC-1043) |

## Dependencies

- ADR-021 POR-2 (writes), POR-3, RU-1, TX-1 (1) and (2).
- [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
  (intake of frames), [FR-105](FR-105-emit-state-nodes.md)
  (the S4 frame node), [FR-106](FR-106-admit-snapshots-and-invocations.md)
  (check 11), [FR-115](FR-115-run-an-operation-frame-over-an-invocation.md)
  (the `Frame` run), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (the frame decision, post-states and domains).
- QSpec FR-013 owns frame semantics; the receiver-scope amendment, its
  surface syntax and the scope member of the checked-package v2 frame entry
  are QSpec's (ADR-021 QS-4). The domain package's operation frame shape is
  the model contract's.

## References

- QSpec half: QSpec FR-013 and FR-340 (Linear STD-134; ADR-021 §9 QS-4). Owning ticket: Linear
  QSL-368.

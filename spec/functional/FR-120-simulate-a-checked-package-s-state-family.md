---
id: FR-120
title: "Simulate a checked package's state family over its operation frames"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
---
# FR-120: Simulate a checked package's state family over its operation frames

## Description

QSpec FR-181 is the normative source for finite simulation, and QSpec FR-013
for operation frames. FR-101 is QSL's engine for FR-181: canonical order,
the state key, the fixed-algorithm sampler, replay, findings and the stopped
outcomes, over any `TransitionSystem` (FR-101's trait, including its
`Expansion`, `ExpansionStop` and `Outcome::Stopped`). This requirement
supplies `ModelSystem`, the `TransitionSystem` for a checked package's state
family, which provides the three FR-181 parts that are model-level:

- the successor relation of FR-181's exploration contract: an operation
  applied to an argument vector from its finite parameter domains, enabled
  when its effective precondition is true, times every post-state within its
  frame (FR-013-AC-1) for which its postcondition holds;
- recording a successor that violates an invariant rather than pruning it
  (FR-181-AC-4, last clause), as FR-101 findings;
- performing none of the effects a model describes: effects appear only as
  typed trace data, and a request for evaluator I/O, mutation or an ambient
  store refuses with its source location (FR-181-AC-6).

The simulator runs over the S4 in-memory `CheckedPackage`
(`CheckedPackage::link`) and the domain package bytes FR-106 admission
reads. A unit whose S5 emission refuses today, such as one with `pre` or
`post` clauses (`Emit(UnlocatedOccurrence { role: Generated })`, pending
STD-111), is simulated through its S4 package.

Where FR-181 leaves a choice open, this requirement names QSL's choice and
says so.

## Use case

A verification operator has a domain package with operations and frames and
a `1-draft` unit with invariants and operation contracts (US-003). They
supply a few initial snapshots and a finite key set for each population,
then explore every reachable state or sample a seeded trace. The result
names each reachable state where an invariant is false or could not be
decided, keeps exploring from it, and shows each step's creations, deletions,
field changes and result as data. A run that could not decide a successor
set is incomplete, never exhaustive.

## Inputs

A `ModelSimulationRequest` (`qsl_eval::simulation::model`):

- `package: &CheckedPackage`, the S4 package;
- `packages`, FR-056's package input (domain packages by identity and version), and the `ModelNormalizationLimits` FR-106 admission takes;
- `snapshots`, an FR-106 snapshot provision, and `initial:
  Vec<DocumentRef>`, the initial states;
- `universes: Vec<PopulationUniverse>`: for each simulated population, its
  declaration key and its finite key set `keys: Vec<String>`;
- `ObservationLimits` (FR-106);
- `ExpansionLimits { max_candidates: u64 }`;
- the evaluation meter budget. Each clause evaluation gets a fresh `Meter`
  with that budget (FR-107).

## Outputs

```rust
impl ModelSystem<'_> {
    pub fn new(request: ModelSimulationRequest<'_>) -> Result<Self, AdmissionFailure>;
    pub fn domains(&self) -> Vec<(NodeKey, ValueType)>;
}

pub fn explore_model(
    system: &ModelSystem<'_>,
    position_limit: u64,
    limits: Limits,
    poll: impl FnMut() -> bool,
) -> Result<Exploration<ModelFinding>, NotSimulated>;

pub fn sample_model(
    system: &ModelSystem<'_>,
    position_limit: u64,
    sampler: &DefinitionRef,
    seed: u64,
    trace: u64,
    max_steps: usize,
) -> Result<ModelTrace, ModelSampleError>;

pub enum ModelSampleError {
    NotSimulated(NotSimulated),
    Replay(ReplayError<ModelTransition>),
}

pub struct ModelTrace {
    pub trace: Trace<ModelTransition, ModelFinding>,
    pub effects: Vec<StepEffect>,
}
```

- `ModelSystem::new` returns the system or FR-106's `AdmissionFailure`
  (`Refused` or `Incomplete`, one record).
- `ModelSystem::over_model` builds the system of one model of a refinement
  unit (ADR-020 RM-1) and SHALL refuse a model that the unit does not select
  with `invalid_runtime_input`/`wrong-role-mapping`. A `ModelSystem` reports
  the model it is over, so a caller can check that it holds the system it
  meant to (FR-140).
- `explore_model` is FR-101's `explore_request` over the system, its
  `domains()` and the package's `TypeEnvironment`. `sample_model` is FR-101's
  `sample_request` over the same inputs, followed by `replay` of the sampled
  trace against the system, which yields one `StepEffect` per step.
  `ModelTrace.trace` holds FR-181's transition identities, state keys
  and findings; `ModelTrace.effects` holds the effects that replay derives.
  `sample_model` returns FR-101's `NotSimulated` as
  `ModelSampleError::NotSimulated`, and a trace that does not replay against
  the system that sampled it as `ModelSampleError::Replay`.

## Behavior

### Identities

The **declaration identity text** of a `DeclarationKey` is the RFC 8785 JCS
text of its existing `$defs.DeclarationKey` preimage (`qsl-semantics/src/
model/key.rs:84-124`, QSpec `model-complete.md` "Identity domains"):
`{"node":"<node>","package":"<package>"}`. It
names the package, so two domain packages with equal node identities give
different texts. It is the text form for every declaration below: the
`semantic` map's population keys, a record's `name`, a transition's
operation and an anchor's handler name. This is QSL's choice; FR-181's
`<qualified-name>` leaves the text form to the implementation.

### Initial states

`ModelSystem::new` checks `universes`, then admits each `initial` document
in the given order, and returns the first failure:

1. A universe population that is no population of the package refuses
   `invalid_runtime_input`/`wrong-role-mapping`. An empty key, which names
   no object (FR-035), refuses `invalid_runtime_input`/`invalid-value`. A
   key repeated within one universe refuses
   `invalid_runtime_input`/`conflicting-identity`.
2. Each initial document is a `quire.state.snapshot/v1` snapshot (FR-106
   "Document forms"), admitted by `StateModel::admit_initial` (see "Frame
   and observation seam"), which runs FR-106 check 1 (read), check 3, check
   4 (model), check 6 (populations and values), check 7 and check 8
   (closure), with these readings:
   - check 3: `observation` is `current` and `anchor.kind` is
     `initialization`. Another `observation` refuses `wrong_snapshot`/
     `wrong-observation`, and another anchor kind refuses `wrong_snapshot`/
     `wrong-anchor`. Treating an initial state as an initialization
     observation is QSL's choice: QSpec's state contract names
     initialization and handler anchors for invariants and leaves a
     simulator's initial state to the implementation.
   - check 7 applies to every population of the document. Each universe
     population is also required: a universe population the document omits
     returns `Incomplete` with `incomplete_population`/`incomplete-scope`,
     naming it.
3. An object whose key is not in its population's universe refuses
   `invalid_runtime_input`/`invalid-value`, naming the population and key.
   A population the document holds with no universe is an unbounded
   `Population(None)` domain in `domains()` (see "Bounds").

Equal initial states coalesce by key, as FR-101 admits them.

### State, state key and anchor

A model state is one complete, closed population state: for each universe
population, its objects, each with its key, its most-specific type and a
value for each declared field, where every reference value names an object
of the state. It also carries its anchor, which its key omits.

The state key is FR-181's
`{"type":"simulation-state","semantic":…,"control":…,"queues":…,"roles":…,"observations":…,"bounds":…,"memory":…}`.
`control`, `queues`, `roles`, `observations`, `bounds` and `memory` are empty maps
(`{"type":"map","entries":[]}`), matching a state family whose only state is
its populations. FR-181 leaves a population state's typed canonical form to the
implementation, and QSL's `semantic` member is a map in FR-181's map form:

- key: each universe population's declaration identity text, as `text`;
- value: a map from each object's reference triple to a `record` whose
  `name` is the declaration identity text of the object's most-specific type
  and whose `fields` are its declared fields in declaration order, each
  encoded by its declared value type (FR-181 "Typed canonical form").

A field declared `Option<T>` holds an option value, `{"type":"option","value":
<encoded>|null}`. A presence-optional (`?`) field that an object leaves absent
is the plain `null` of its `{name, value}` entry (QSpec FR-181 "Typed canonical
form"); a present one is its bare encoded value.

Every other value encodes in FR-181's typed canonical form, read from its
declared type:

- a decimal as `{"type":"decimal","coefficient":…,"scale":…}` with its
  retained coefficient and scale (QSpec FR-140);
- a quantity as the `rational` form of its exact magnitude with an added
  `"unit"` member, since a QSL quantity's magnitude is an exact rational
  (`quire-exact` `Quantity`);
- a UUID as `{"type":"uuid","value":…}` holding its K2 canonical payload,
  and a Timestamp as `{"type":"timestamp","nanoseconds":…}` holding its
  signed nanosecond count (FR-056, ADR-034);
- an enum value as `{"type":"enum","name":…,"member":…}`;
- a record value as `{"type":"record","name":…,"fields":[…]}`, one
  `{name, value}` entry per declared field in declaration order, with an
  absent `?` field `null`;
- a tuple value as `{"type":"tuple","name":…,"components":[…]}` in
  declaration order;
- a union value as `{"type":"union","name":…,"member":…,"components":[…]}`,
  an empty array for a nullary member.

In these forms `name` is the enum, record, tuple or union declaration's
declaration identity text (above) and `unit` is the declared quantity type's
`UnitId` text (FR-088), and `member` and a
field's `name` are the declared identifiers: the spellings FR-070 gives the
same values in a counterexample witness, so a value has one FR-181 text in
QSL wherever it appears. An object's `record` name stays the declaration
identity text of its most-specific type, as above.

Every element of a set or bag is written in FR-181's canonical element
encoding before the elements are ordered by their JCS bytes, recursively
through nested options, collections, records, tuples and unions: a decimal
takes its normalized coefficient and scale (QSpec FR-140), and every other
family, a quantity's exact rational included, keeps the form above. A value outside a set
or bag element keeps its retained form.

A state member that holds a population's objects is typed
`Set<Reference<T>>` and encodes as FR-181's `set` of reference triples. A
value of a population type itself, a population handle rather than its
objects, has no FR-181 form (QSpec FR-181: a type that is a population has
none). Encoding one, wherever it appears in a state key, a transition
identity or a step effect, SHALL refuse with the typed `EncodingRefusal`
naming the population-typed field, receiver, argument or result, never a
panic; FR-101 returns it as `NotSimulated::KeyEncoding` (`replay` as
`ReplayError::KeyEncoding`), `invalid_runtime_input`/`invalid-value`.

A reference encodes as FR-181's triple: `universe` is the population's
`UniverseId` as 64 lowercase hex digits, `object_type` the most-specific
type's `EffectiveId` as 64 lowercase hex digits, and `identity` the
lowercase hex of the key's UTF-8 bytes (`c1` is `6331`).

A state's anchor is QSL's choice: QSpec requires an exact named
initialization or handler observation for an invariant and leaves a
simulated state's anchor to the implementation. An initial state's anchor is its snapshot's
`{initialization, <name>}`. A reached state's anchor is
`{handler, <operation's declaration identity text>}` of a transition that
reached it. In exploration it is the transition that first reached the
state: FR-101's canonical breadth-first order decides which transition is
first, and the engine keeps the first-reached state for a key. In a sampled
trace, each step's state is anchored to that step's own transition. The anchor appears in each invariant
finding.

### Frame and observation seam

`qsl_semantics::model::state` is the public layer-3 seam `ModelSystem`
calls. It reuses FR-106's model views, value reader and frame decision:

```rust
pub struct StateModel { /* FR-106's re-derived model views and the checked TypeEnvironment */ }

pub struct ModelState {
    pub populations: BTreeMap<DeclarationKey, BTreeMap<String, StateObject>>,
}

pub struct StateObject {
    pub type_identity: DeclarationKey,
    pub fields: BTreeMap<DeclarationKey, Value>,
}

impl StateModel {
    pub fn new(
        model_selections: &[DomainPackageRef],
        types: &TypeEnvironment,
        packages: &BTreeMap<DomainPackageIdentity, Vec<u8>>, // keyed by package identity (FR-056); code: CODE-LANE QSL-1260
        limits: ModelNormalizationLimits,
    ) -> Result<Self, AdmissionFailure>;
    pub fn admit_initial(
        &self,
        provision: &BTreeMap<DomainPackageIdentity, Vec<u8>>, // keyed by package identity (FR-056)
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> Result<(ModelState, SelectedAnchor), AdmissionFailure>;
    pub fn observation(
        &self,
        state: &ModelState,
        identity: DocumentRef,
    ) -> Result<Observation, AdmissionFailure>;

    // Reads of the re-derived views `ModelSystem` needs; none decides a frame.
    pub fn populations(&self) -> Vec<StatePopulation>;
    pub fn type_identity(&self, key: &DeclarationKey) -> Option<EffectiveId>;
    pub fn type_key(&self, identity: EffectiveId) -> Option<&DeclarationKey>;
    pub fn is_abstract(&self, key: &DeclarationKey) -> bool;
    pub fn fields(&self, type_key: &DeclarationKey)
        -> Result<Option<Vec<StateField>>, AdmissionFailure>;
    pub fn object_reference(&self, type_key: &DeclarationKey, object: &str)
        -> Result<ObjectReference, AdmissionFailure>;
    pub fn population_index(&self, population: &DeclarationKey) -> Option<&ModelIndex>;
    pub fn ancestor_steps(&self) -> u64;
}

pub struct StatePopulation { pub key: DeclarationKey, pub member_types: Vec<DeclarationKey> }
pub struct StateField {
    pub key: DeclarationKey,       // the key a `StateObject` holds the value under
    pub name: String,
    pub value_type: ValueType,
    pub optional: bool,            // declared `?`: the object may leave it absent
}
```

`StateModel` decides no frame grant: `decide_frame` is the only decision
(ADR-016 FE-1), and `ModelSystem` learns which fields an application may write
by asking it about a state that changes that one field.

- **The frame decision.** `ModelSystem` decides whether a candidate stays
  inside its operation's frame by FR-106 check 11 over two in-memory states,
  with no `StateModel` method between: per population, in ascending
  population `DeclarationKey` order, it computes the created keys (the post
  state's keys absent from the pre state) and the deleted keys (the pre
  state's keys absent from the post state), builds the population's pre and
  post `FrameObject`s, and calls `population::decide_frame` (FR-154 "One
  decision"), the one frame decision that `enforce_frame` and FR-106's
  `observation::frame::enforce` also call. It passes those created and deleted
  keys as `FrameDecision.declared_created` and `declared_deleted`, the
  application's `OperationEffect`, the application's receiver as
  `FrameDecision.receiver` when the receiver lies in that population, and
  each reference argument of the application as a `(parameter name, object)`
  pair in `FrameDecision.arguments` when the object lies in it, so that a
  scoped `modifies` or `deletes` entry is decided as FR-154 states. The
  step's frame delta is the union over populations of the returned
  `FrameDelta`s (`created` and `deleted` identity sets). A refusal from
  `decide_frame` is check 11's first finding
  (`frame_violation`/`unauthorized-change`, or
  `population_delta_mismatch`/`delta-disagreement`), and its verdicts are the
  ones FR-115 reports.
- `observation` builds FR-106's `Observation` for a state: `identity` is the
  given `DocumentRef`; `environment` is the `ObjectEnvironment` of every
  object with every field value, built by FR-106's builder; `populations`
  maps each population to `true` (complete).
- A synthesized state's `DocumentRef` has labels `authority` `quire`,
  `identity` `quire.simulation.state/<n>`, `n` the decimal position of the
  state in the order the exploration first reached it; the state is addressed by that
  identity alone.
- `ModelSystem` builds each clause evaluation's `AdmittedObservations`
  directly, since its fields are public (`qsl-semantics/src/model/
  observation.rs:309-335`): `usage` is `AdmissionUsage::default()`; `clause`
  is the clause's node key; for an invariant, `current` is the state's
  observation; for a precondition, `pre` is the state's observation and
  `post` is none (FR-107 reads `pre` only); for a postcondition, `pre` is
  the state's and `post` the candidate's observation, `created` and
  `deleted` are the frame delta; `self_object` is the receiver, or
  the invariant's object; `parameters` are the arguments by declared name;
  `result` is the candidate result or none.

### Applications and transition identities

For each object `r` of a state and each operation name of `r`'s effective
view (FR-103, FR-081), the application's operation is the effective member
that name selects for `r`'s most-specific type. Its receiver is `r`, and its
arguments are one value per declared parameter, in declaration order, from the
parameter's domain. The receiver is not an argument.

The transition identity is FR-181's
`{"type":"transition","operation":"<qualified-name>","receiver":<encoded>,"arguments":[<encoded>, ...]}`,
and `<qualified-name>` is the operation's declaration identity text. Written
as bytes its keys are in JCS order: `arguments`, `operation`, `receiver`,
`type`. `ModelTransition` is the typed form of that identity: the `operation` as its
canonical text, a `receiver: Option<String>` and the `arguments` as a vector of
canonical text. Each text is the value's canonical JCS text, as `operation` and
each argument are. The receiver, each argument, each `StepEffect` value and
each result encode by the same forms as a state field ("State, state key and
anchor"), so a transition over a decimal, quantity, UUID, Timestamp, enum,
record, tuple or union argument has an identity (ADR-016 ID-9). A model application always has a receiver; `receiver` is
`None` only for an operation FR-181 admits with none, and `Some("null")` is
refused.

A value domain is: `Boolean`, `false` then `true`; `Int[lower, upper]`,
every integer in the range; `Option<T>`, absent then each value of `T`;
`Reference<T>`, each object of the state, or of the post-state for a field
or result, whose most-specific type conforms to `T`; a sequence with a
declared maximum `N`, every sequence of length 0 to `N` over its element
domain. The rules below add the enum, model-variant union, record, tuple,
set, bag and sequence of a finite or clock element to that list;
FR-120-AC-22 to FR-120-AC-26 specify them.

- An enum SHALL be a finite domain, each member in declaration order, as a
  bare operation parameter, result or granted field and as a component.
- A union derived from a model variant type (FR-056) SHALL be a finite
  domain when each case's payload positions have finite domains by these
  rules. Each case contributes the product of its payload-position domains,
  with a nullary case contributing one value; cases are ordered by ascending
  UTF-8 bytes of case name, and the first payload position is most
  significant. A payload position whose type has no finite domain makes the
  union unbounded.
- A record or tuple whose non-clock components all have a finite value domain
  by these rules SHALL be a finite value domain: every value that gives each
  non-clock component a value of its domain, with the first declared component
  most significant and each component in its own domain order (ADR-027 ST-11's
  "finite record domain"; FR-215 states the same order for a record of finite
  fields). Its size SHALL be the
  product of its non-clock components' domain sizes.
- A record field declared presence-optional (`name?: T`) SHALL range over
  absent, then each value of `T`, as `Option<T>` does.
- A record or tuple with a clock-bearing record or tuple component SHALL take
  that component's non-clock domain by these rules.
- A component of type `Clock`, of an alias of `Clock` (FR-230 follows the alias
  chain), or of `Option<Clock>` or a `?` field of `Clock`, is a clock place
  (FR-230 "Clocks and resets"): it SHALL contribute no factor to the
  enumeration and SHALL NOT make the record or tuple unbounded. A record or
  tuple with no non-clock component SHALL have exactly one value.
- The explorer SHALL NOT enumerate or write a clock place as a value. A
  candidate keeps each clock place at its pre-state value unless the
  postcondition resets it as FR-230 "Records, tuples and unions" states
  (a clock the operation does not reset keeps its value; the snapshot carries
  no clock value).
- A record or tuple with any non-clock component whose type has no finite domain
  by these rules SHALL be an unbounded domain in `domains()`, and so SHALL a
  union derived from a model variant type with any payload position lacking a
  finite domain, a `Reference<Clock>` (data that names a clock place, not a clock
  place itself, so it is not covered by the clock-place rule), and every other
  type that this section's list does not name: `Integer` with no range, a
  collection with no declared maximum other than `Set<Clock>`, a `Map` (it
  needs its own ruling for its key domain), an ordered set, a decimal, a
  quantity, a UUID or a Timestamp, for example. A general `Map` and an
  ordered set remain unbounded because their candidate order and/or key-domain
  behavior is not defined here.
- A `Sequence<E>`, `Set<E>` or `Bag<E>` with a declared maximum `N` SHALL be a
  finite value domain when `E` is a finite value domain by these rules or is a
  clock place (`Clock`, an alias of `Clock`, or `Option<Clock>`). A record or
  tuple `E` with no non-clock component is finite by the record rule. Let `p`
  be the size of `E`'s non-clock domain: the product of the non-clock
  components' sizes for a record or tuple (1 when it has none), `E`'s own size
  otherwise, and 1 for a clock place, which adds no factor (`Option<Clock>`
  included: its presence adds none). The size SHALL be, over lengths `k` from 0
  to `N`:
  - `Sequence<E>`: the sum of `p^k`, which is `N + 1` when `p` is 1 and
    `(p^(N+1) - 1) / (p - 1)` otherwise;
  - `Set<E>`: the sum of C(`p`, `k`), the `k`-element subsets of the `p`
    values (so `Set<Clock>[0, N]`, `N` at least 1, has 2 values: empty and one
    element, as FR-230 records);
  - `Bag<E>`: the sum of C(`p + k - 1`, `k`), the multisets of size `k`
    (`N + 1` when `p` is 1).
- The values of a `Sequence<E>` SHALL be ordered by length first and, within a
  length, lexicographically with the first element most significant, each
  element in `E`'s own non-clock domain order. The values of a `Set<E>` or
  `Bag<E>` SHALL be ordered by element count first and, within a count,
  lexicographically over the elements in ascending `E` domain order, the
  smallest element most significant.
- The explorer SHALL NOT enumerate the clock slots of an element as values.
  For a `Sequence<E>`, a slot present in both the pre-state and the candidate
  (the same position) SHALL hold the pre-state value, or the reset FR-230's
  slot provenance (`Carried` or `Fresh`, FR-230 "Collection-valued writes")
  gives when the postcondition writes the collection. For a `Set<E>` or
  `Bag<E>`, a candidate element SHALL be matched to a pre-state element with
  equal non-clock components, one to one (a bag's equal elements are matched in
  ascending order); a matched element SHALL keep that pre-state element's clock
  values, or the provenance's reset when the postcondition writes the
  collection. An element or slot that the candidate adds, with no pre-state
  match (a sequence longer than the pre-state, an unmatched set or bag element),
  SHALL hold the `Fresh` reset value, each of its clock places at 0 in its
  declared unit. This creation value is this section's own rule: FR-230
  states provenance for a written collection and gives no value to a created
  clock.
- A `Set<Clock>` SHALL be a finite value domain of 2 values (empty, then the
  one element) with or without a declared maximum, as FR-230 records it as
  holding at most one element.
- Any other `Sequence<E>`, `Set<E>` or `Bag<E>` with no declared maximum, or
  whose element has a non-clock component with no finite domain, SHALL be an
  unbounded domain, and `explore_model` and `sample_model` SHALL return
  `NotSimulated::RequiresBound` naming its node key.
- The tuple, presence-optional field, clock-place, enum-as-parameter and union
  rules above are this section's own; FR-215 states only the record of finite
  fields and its order.

Rationale. `Sequence<Timer>` and `Sequence<Clock>` with a declared maximum were
unbounded in the earlier reading of this section; the planner ruling on STD-508
withdraws that. The length-first order keeps the empty value first, as
`Option<T>` puts absent first, and reads the record rule over positions; FR-215
states an order only for a record of finite fields and takes a field's domain
from this section. `Reference<Clock>` is data and not a clock place, so it stays
unbounded. A map, whose size depends on a key domain, and an ordered set need
their own ruling.

### Contract clauses

A `pre` or `post` clause applies to an application when it resolves to the
application's operation (FR-104) and `r`'s most-specific type conforms to
the clause's context type.

The effective precondition is `true` when no `pre` clause applies, and
otherwise is formed as QSpec FR-151's rule `quire.model.conformance.refinement/v1`
defines it (QSpec FR-151-AC-17 to AC-19, TC-196 D09 to D11): `own(m)` and
`eff(m)` are QSpec's, and a member's own clauses are conjoined in
ascending clause name (UTF-8 bytes) under FR-006's short-circuit, so an
undefined clause that sorts before a false one makes the precondition
undefined and the reverse order makes it `false`. The order does not depend on
source position, source unit or link order: a member's own clauses are
gathered from every source unit of the package (FR-480) and conjoined in
name order alone (FR-120-AC-15 for one unit, FR-120-AC-19 for D10's clauses
split across two units in both link orders, FR-120-AC-20 for D10's
`absent-key` form). The dispatch path conjoins the own
clauses through `view_clauses` (`dispatch_link.rs`) and disjoins the result
with the redefined member's through `effective_terms`
(`checked_dispatch.rs`). On the spine every operation record's `redefines` is
`None` (`qsl-semantics/src/model/intake.rs:1444`), so the applicable clauses
are the selected member's own.
The effective postcondition is the conjunction of the applicable `post`
clauses, and `true` when none applies.

A conjunction is evaluated through FR-107's `evaluate_clause_set`, over the
applicable clauses of one kind, with a fresh meter for each clause and each
clause in ascending name order (UTF-8 bytes):

- An `Incomplete` result stops the expansion with `resource_exhausted`/
  `insufficient-next-charge` at once: the successor set is not known. The stop
  names the limit `EvaluationMeter` whichever counter of the evaluation ran out,
  the counter limits of the exact kernel included (FR-149 relies on this
  label), and carries that counter: the `quire.value.accounting/v1` counter
  the `Incomplete` names as exhausted
  (`ExpansionLimit::EvaluationMeter { counter }`). A settlement reads it to
  name the counter's FR-255 accounting setting and the counter's configured
  bound in the meter budget (FR-192, FR-203); the stop carries no bound.
- A `CallFailure` stops the expansion with `runtime_invariant`/
  `established-invariant-broken` at once: the simulator built the
  observations itself, so a refused selection is a broken internal
  invariant.
- Otherwise, for a postcondition, the conjunction is `true` when every clause is
  `Completed(true)` and `false` when any clause is `Completed(false)`. When
  no clause is `Completed(false)` and one or more are `Undefined` or
  `Refused`, the conjunction is undecided, and the expanded state gains one
  `ModelFinding::ContractUndetermined { transition, candidate, clause,
  evaluation }` per such clause, in name order; `candidate` is the
  candidate's state key. The result depends only on the set of clause
  outcomes.

For the effective precondition the conjunction over one member's own
clauses is QSpec FR-151's: clauses in ascending name order, each evaluated
under FR-006's short-circuit, and evaluation stops at the first clause that is
not `Completed(true)`; the clauses after it are not evaluated or charged. A
`Completed(false)` clause makes the precondition `false`: the application is
not enabled and no finding is recorded. A clause that is `Undefined` or
`Refused` makes the precondition undecided: the application is not enabled
and the expanded state gains one `ModelFinding::ContractUndetermined
{ transition, candidate, clause, evaluation }` for that stopping clause, with
`candidate` none. An undefined clause that sorts before a false one therefore
gives the undecided outcome, and the reverse order gives `false`. For the
effective postcondition the conjunction keeps the outcome rule above and
depends only on the set of clause outcomes.

An application is enabled when its effective precondition is `true`.

### Post-states

The application's frame is its operation's frame as FR-114 binds it: the
operation's one `OperationEffect` (FR-103), the declaring type's frame for
an inherited operation. The request's only frame source is the checked
package (FR-115, QSpec FR-013-AC-3). Simulation reads every operation's
`OperationEffect` from the package, including one that no clause or attempt
names.

For an enabled application, the candidate post-states are every complete,
closed population state built from the state by:

- giving each field that `modifies` grants any value in its domain, on every
  object whose effective type has the field or reaches it through
  `redefines` (FR-013 Behavior), and keeping every other field of a
  surviving object;
- deleting any subset of the objects whose most-specific type conforms to a
  `deletes` entry;
- creating an object under any subset of each population's unused universe
  keys. Each created object has a non-abstract type that conforms to a
  `creates` entry and to a member type of the population, and any value in
  each field's domain.

Every reference of a candidate names an object of the candidate.

Each candidate goes through the frame decision ("The frame decision" above)
with the state as `pre` and the candidate as `post`. A candidate is within the
frame exactly when `decide_frame` returns a delta for every population, the
case FR-115 reports as `success`, and that delta is the step's `created` and `deleted`. The construction yields
only such candidates, so a finding from the check stops the expansion with
`runtime_invariant`/`established-invariant-broken`.

For an operation that declares a result, each candidate pairs with each
value of the result type's domain over the post-state, in ascending order of
the value's typed canonical JCS bytes; an operation with no result pairs
each candidate with none. A candidate is a successor when some pair's
effective postcondition is `true`, and the first such result is the
transition's result.

Candidates are generated lazily, one at a time in canonical order; no
expansion materializes its candidate set. Each expansion counts every
argument vector it forms, whether or not its precondition holds, every
candidate it generates, including one the frame check or closure rejects,
and every (candidate, result) pair it evaluates, over all its applications
(ADR-016 G-4). A pair is evaluated only when the pairs before it in canonical
order were not accepted, so an operation whose first pair is accepted counts
one pair however large its result domain is. A result domain is put in
canonical order before its pairs are read, which holds one position per
value; a domain of more than 65,536 values cannot be ordered and stops the
expansion the same way. When the count would exceed `max_candidates`, the expansion
stops with `resource_exhausted`/`insufficient-next-charge`, naming the limit
`MaxCandidates` and no counter.

### Invariants

Every expansion evaluates each invariant clause of the package, in
ascending name order, for each object of the state whose most-specific type
conforms to the clause's context type, in ascending reference-key order.
Each evaluation goes through `evaluate_clause`, over the state's `current`
observation, with a fresh meter. The expanded state gains:

- `ModelFinding::InvariantViolated { clause, self_object, anchor }` for
  `Completed(false)`;
- `ModelFinding::InvariantUndetermined { clause, self_object, anchor,
  evaluation }` for `Undefined`, `Refused` or `Incomplete`.

A `CallFailure` stops the expansion with `runtime_invariant`/
`established-invariant-broken`. An invariant finding keeps the state and its
successors: the state stays in the explored set, is expanded, and its
successors are explored (FR-181-AC-4). A state's invariant findings come
before its contract findings; contract findings are in ascending
transition-identity bytes, then candidate state-key bytes (none first), then
clause name. FR-101 records findings for expanded states, and a stopped
expansion's `ExpansionStop` carries its cause alone (FR-101 "Findings and stopped
expansions").

### Bounds

`ModelSystem::domains()` lists, for FR-101's requires-bound check, one root
per:

- operation parameter, typed by its declared type;
- operation result, typed by its declared type;
- field that a `modifies` entry grants, and field of every creatable type,
  typed by its declared type;
- population the initial states hold, as `ValueType::Population(Some(n))`
  with `n` its universe's length (FR-104's `Cardinality` bound), or
  `Population(None)` when it has no universe.

Each root is named by a structural checked-package node key `{package, path,
ordinal}`, as QSpec FR-331 "Model member domain keys" defines: a field, an
operation parameter or an operation result is keyed under the model
declaration node of the type that declares it, with path `[0, f, ...]` for a
field, `[1, o, p, ...]` for a parameter and `[2, o, ...]` for a result; a
population is named by its `relation`/`population` node (QSpec FR-201
`PopulationId`). A root that `classify_extent` finds unbounded makes
`explore_model` and `sample_model` return `NotSimulated::RequiresBound`
naming it, before any state is expanded.

### Effects as data

The simulator's inputs are its request's in-memory values and its outputs
are its return values; `successors` takes the state by shared reference and
returns new states, so every input keeps its value.
The effects a model describes are its operations' frames, and each appears
only as data: a `StepEffect` holds the step's `created` and `deleted` object
references and its `changed` entries `{object, field, pre, post}`, objects
in ascending reference-key order and fields in declaration order, each value
in FR-181's typed canonical form, and the transition's result or none.

Every simulator input is data. The complete-V1 grammar QSL parses has
declaration and expression forms only, and FR-107's evaluator takes the
package, the observations and the meter as its whole input.
The forms that read ambient state outside their anchor refuse at S3, each at
its own source location, before a `CheckedPackage` exists (QSpec
`shared-grammar.md`: "`self`, `result` and `pre(...)` are caller-side anchor
operations, unavailable as implicit ambient state"):

- `self` outside a state clause: `missing_declaration`/`missing-name` at the
  `self` (FR-104; `self_reference`, `qsl-semantics/src/check/
  check.rs:1667-1673`, refuses at the `self` node's location);
- `result` outside a `post` clause of an operation with a result:
  `wrong_snapshot`/`wrong-anchor` at the `result` (FR-104);
- `pre(e)` outside a `post` clause: `wrong_snapshot`/`forbidden-pre-read` at
  the `pre` (FR-104);
- `reaches` outside a state clause: `ill_typed`/`operator-ineligible` at the
  `reaches` (FR-104).

Every code and cause above is in QSpec `native-diagnostics.md`.

## Acceptance Criteria

The fixture domain package `test/counters` declares object type
`test/counters/Counter` with fields `value: Int[0, 2]`, `label: Int[0, 1]`,
`next: Option<Reference<Counter>>` and `tag: Integer?` and `id: Integer` (unbounded
fields no frame grants; `tag` absent and `id` 0 in every state), and population
`test/counters/counters` of `Counter`. Its operations are listed per AC,
each with node `test/counters/Counter/<name>`. One further check adds `tag: Integer?` and `id: Integer` to `Counter`, fields no
frame grants and no finite domain covers. The initial state `s0` holds
`c1` (`value` 0, `label` 0, `next` absent), anchored `{initialization,
start}`, and the universe of `counters` is `[c1, c2]`, unless an AC says
otherwise. `v<n>` is `s0` with `c1.value` = n. `<op>` is the declaration
identity text of operation `op`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-120-AC-1 | With operation `increment()` (frame `modifies value`) and no clause, `s0` has three successors, `v0`, `v1` and `v2`, all with transition identity, as JCS bytes, `{"arguments":[],"operation":"{\"node\":\"test/counters/Counter/increment\",\"package\":\"test/counters\"}","receiver":<c1's reference>,"type":"transition"}`; `label` and `next` keep their values in each. Adding `post Inc on Counters::Counter::increment { self.value = pre(self.value) + 1 }` leaves one successor, `v1`. With `pre CanInc … { self.value < 2 }` and no `post` clause, `v0` and `v1` each have the three successors and `v2` has no `increment` successor. | Test (TC-471) |
| FR-120-AC-2 | With `setTo(n: Int[0, 2])` (frame `modifies value`, post `self.value = n`), `s0` has one successor per `n`, with `receiver` `<c1>` and arguments `[{"type":"integer","value":"0"}]` to `[{"type":"integer","value":"2"}]`. With `c2` added to `s0`, each receiver gives its own identity. With `pre A { self.value >= 1 }` and `pre B { self.value <= 1 }` on `setTo`, both must hold: it is enabled at `v1` and not at `v0` or `v2`. | Test (TC-471) |
| FR-120-AC-3 | With `spawn()` (frame `creates Counter`) and no clause, `s0` has 1 + 18 successors: `s0` itself, and `c2` created with each (`value`, `label`, `next`) in 3 × 2 × 3 (absent, `c1`, `c2`). With `remove()` (frame `deletes Counter`) over `c1` and `c2`, where `c1.next` names `c2`, each receiver's successors are the deletion sets {}, {`c1`} and {`c1`, `c2`}; deleting `c2` alone leaves `c1.next` dangling and is not a candidate. Each successor's delta is the one the frame decision returns. | Test (TC-471) |
| FR-120-AC-4 | `s0`'s state key has `semantic` equal to `{"type":"map","entries":[{"key":{"type":"text","value":"{\"node\":\"test/counters/counters\",\"package\":\"test/counters\"}"},"value":{"type":"map","entries":[{"key":{"type":"reference","universe":"<U>","object_type":"<E>","identity":"6331"},"value":{"type":"record","name":"{\"node\":\"test/counters/Counter\",\"package\":\"test/counters\"}","fields":[{"name":"value","value":{"type":"integer","value":"0"}},{"name":"label","value":{"type":"integer","value":"0"}},{"name":"next","value":null}]}}]}}]}`, where `<U>` and `<E>` are the universe and effective type of the `ObjectReference` that FR-106 admission gives `c1` in `s0`'s admitted `Observation` (`StateModel::admit_initial`, independent of the simulator's key encoding), each as 64 lowercase hex digits; `control`, `queues`, `roles`, `observations`, `bounds` and `memory` are `{"type":"map","entries":[]}`. `s0` with `c1.label` 1 has a different key. | Test (TC-471) |
| FR-120-AC-5 | With `increment`, `CanInc`, `Inc` and `invariant Small … at current { self.value < 2 }`, exploration is `Exhaustive` with 3 states, 2 transitions and depth 2, and `findings` holds exactly one entry: `v2`'s state key, depth 2, `InvariantViolated { Small, c1, {handler, <increment>} }`. Adding `reset()` (frame `modifies value`, post `self.value = 0`) makes the run `Exhaustive` with 3 states and 5 transitions, including `v2 → v0`: the violating state is expanded. `s0`'s findings, when any, carry anchor `{initialization, start}`. | Test (TC-472) |
| FR-120-AC-6 | With `increment`, `Small`, no `pre` or `post` clause and a meter budget of zero, exploration is `Exhaustive` with 3 states, and each state has `InvariantUndetermined` for `Small` and `c1`, its evaluation `Incomplete` with `resource_exhausted`/`insufficient-next-charge`. With budget zero and `CanInc`, the run is `Outcome::Stopped` with cause `resource_exhausted`/`insufficient-next-charge`, frontier `[<s0>]`, category incomplete. With `max_candidates` 2 and `increment` with no clause, the run is `Stopped` with the same cause. | Test (TC-472) |
| FR-120-AC-7 | `ModelSystem::new` refuses an initial snapshot with `observation` `pre` (`wrong_snapshot`/`wrong-observation`), one anchored `{handler, validate}` (`wrong_snapshot`/`wrong-anchor`), one whose anchor kind is `other` (`invalid_runtime_input`/`wrong-value-kind` with field `anchor`, refused by FR-106's document reader, `qsl-semantics/src/model/observation/document.rs:525-528`, before check 3), and one holding `c3` (`invalid_runtime_input`/`invalid-value`); a universe listing `c1` twice refuses `invalid_runtime_input`/`conflicting-identity`; a universe for a population the package does not declare refuses `invalid_runtime_input`/`wrong-role-mapping`; a universe population absent from the snapshot returns `Incomplete` with `incomplete_population`/`incomplete-scope`. With no universe for `counters`, or with an operation parameter typed `Integer`, `explore_model` returns `NotSimulated::RequiresBound`, and `sample_model` `ModelSampleError::NotSimulated(RequiresBound)`, naming that root's node key and expand no state. | Test (TC-472) |
| FR-120-AC-8 | A `sample_model` trace (seed `424242`, trace `0`, `max_steps` 4) over the AC-5 package ends `NoSuccessors` at `v2`, records `Small`'s finding at step 2, and replays; the same trace with that finding removed refuses `ReplayError::FindingMismatch` at step 2. With budget zero and `CanInc`, the trace ends `StopReason::Stopped(resource_exhausted/insufficient-next-charge)` at step 0 and replays; replayed against the same package with the default budget it refuses `ReplayError::Stopped { step: 0, recorded: Some(…), replayed: None }`. Two runs with equal inputs give equal `Exploration`s and equal `ModelTrace`s. | Test (TC-472) |
| FR-120-AC-9 | Over the `test/orders` dispatch fixture (object type `A`, population `pop`, a query operation `ready(): Boolean` of `A` whose effective precondition is `false`, `guard` false): from `t1`, `pre Low { self.ready() }` on `ready` makes `ready` not enabled and records `ContractUndetermined { <ready identity>, none, Low, the family result Undefined with reason precondition-false }`; adding `pre Aaa { false }`, which sorts before `Low`, records no finding; `post LowPost { self.ready() }` on `ready` records the finding with `candidate` = `t1`'s state key and gives no successor. Not covered: the `test/tallies` package with `Tally`, its `Sequence<Int[0, 2]>[1, 1]` field and an empty-frame `touch()` (the `touch` operation and the post-`LowPost` case on it). | Test (TC-472) |
| FR-120-AC-10 | With `isZero(): Boolean` (empty frame, `post Z { result = (self.value = 0) }`), `v0` has one `isZero` successor, `v0`, whose step effect has result `{"type":"boolean","value":true}`, and `v1` has one, `v1`, with result `false`; the `isZero` result root is in `domains()`. | Test (TC-473) |
| FR-120-AC-11 | For `increment` (post `Inc`), `spawn` and `remove` over `s0` and state `w` = {`c1` (`value` 0), `c2` (`value` 0)}: the step `increment` from `v0` has effect `created: []`, `deleted: []`, `changed: [{object: c1, field: value, pre: {"type":"integer","value":"0"}, post: {"type":"integer","value":"1"}}]`, result none; `spawn` from `s0` to `s0` plus `c2` (`value` 1, `label` 0, `next` absent) has `created: [<c2>]` and `changed: []`; `remove` with receiver `c1` from `w` to {`c2`} has `deleted: [<c1>]`. Exploring the AC-5 package twice, the second time in a child process started in an empty temporary directory, gives equal `Exploration`s, and `s0`'s key bytes are equal before and after exploring. | Test (TC-473) |
| FR-120-AC-12 | A unit whose function body reads `self`, whose invariant reads `result`, whose invariant reads `pre(self.value)`, or whose function calls `reaches`, refuses at S3 with `missing_declaration`/`missing-name`, `wrong_snapshot`/`wrong-anchor`, `wrong_snapshot`/`forbidden-pre-read` or `ill_typed`/`operator-ineligible`, each at the source span of that form, and yields no `CheckedPackage` to simulate. | Test (TC-473) |
| FR-120-AC-13 | With `spawn()` over a population whose universe has 30 unused keys and each created object's fields ranging over `Int[0, 2]`, `Int[0, 1]` and `Option<Reference<Counter>>`, so the candidate product exceeds 10^9, and `max_candidates` 10, exploration returns `Outcome::Stopped` with `resource_exhausted`/`insufficient-next-charge` after generating at most 11 candidates, and does not allocate the product. | Test (TC-472) |
| FR-120-AC-14 | `ModelSystem::over_model` over a model identity that the refinement unit does not select (`example/not-selected`) refuses `invalid_runtime_input`/`wrong-role-mapping`; over each of the unit's two selected models it returns a system that reports that model. | Test (TC-1086) |
| FR-120-AC-15 | Over the `test/orders` fixture of AC-9 with two `pre` clauses on `ready`, one `false` and one undefined (`self.ready()`), the clauses are conjoined in ascending name under short-circuit whatever their source order: with the undefined clause sorting first (`Aaa` undefined, `Zzz` false, in either source order) the application is not enabled and exactly one `ContractUndetermined` is recorded for `Aaa`, the false clause not being reached; with the false clause sorting first (`Aaa` false, `Zzz` undefined) none is recorded. On the dispatch path (`view_clauses`), over `ConfigVersion::size` with the same two clauses, a call of `size` gives the result of the clause that sorts first (undefined for `other`'s precondition when the undefined clause sorts first, undefined for `size`'s own precondition when the false clause does), with equal outcome and charges in both source orders. | Test (TC-472) |
| FR-120-AC-16 | Over a model whose object type `Item` (population `items`, universe `{i1}`) declares the fields `id: UUID`, `at: Timestamp`, `price: Decimal[0, 1000; 0, 2]`, `length: Quantity<metre>`, `color: Color` (enum `red`, `blue`), `tag: Tag` (record `Tag { sku: Integer, note?: Text[0, 8] }`), `pair: Pair` (tuple `Pair(Boolean, Integer)`) and `shape: Shape` (union with nullary member `none`), an initial state whose `i1` holds `00112233-4455-6677-8899-aabbccddeeff`, Timestamp `-1`, `1.50`, `1.50` metre, `red`, `Tag { sku: 7 }`, `Pair(true, -3)` and `none` is admitted, and its state key writes each field as the QSpec FR-181-AC-19 bytes of that family: `{"type":"uuid","value":"00112233-4455-6677-8899-aabbccddeeff"}`, `{"nanoseconds":"-1","type":"timestamp"}`, `{"coefficient":"150","scale":"2","type":"decimal"}`, and for `color`, `tag`, `pair` and `shape` FR-181-AC-19's enum, record, tuple and nullary-union bytes with each `name` the declaration's identity text; `length` writes `{"denominator":"2","numerator":"3","type":"rational","unit":"<u>"}`, `<u>` the `UnitId` text of `metre`. Exploration of an operation `setPrice(p: Decimal[0, 1000; 0, 2])` from that state gives the transition argument for `1.50` as `{"coefficient":"150","scale":"2","type":"decimal"}` and a step effect whose `price` post value has the same bytes. No refusal is returned. | Test (TC-1195) |
| FR-120-AC-17 | Over AC-16's model with the universe `{i1, i2}` and the fields `prices: Set<Decimal[0, 1000; 0, 2]>` and `times: Set<Timestamp>` added, a state holding `prices = {1.50}` and `times = {2, 10}` writes `prices` as `{"elements":[{"coefficient":"15","scale":"1","type":"decimal"}],"type":"set"}` while `price` keeps coefficient `150`, scale `2`, and writes `times` as `{"elements":[{"nanoseconds":"10","type":"timestamp"},{"nanoseconds":"2","type":"timestamp"}],"type":"set"}` (QSpec FR-181-AC-20). A field `members: Set<Reference<Item>>` holding the objects `i1` and `i2`, built in the order `i1, i2` and in the order `i2, i1`, gives two states with equal state keys, the set's reference triples ordered by their JCS bytes (QSpec FR-181-AC-21). | Test (TC-1195) |
| FR-120-AC-18 | A state key, transition identity or step effect that would hold a value of a population type refuses `EncodingRefusal` naming the population-typed field, receiver, argument or result: `explore_model` returns `NotSimulated::KeyEncoding`, and `replay` of a trace through it returns `ReplayError::KeyEncoding`, each with `invalid_runtime_input`/`invalid-value`, and neither panics nor records a state for that value. | Test (TC-1195) |
| FR-120-AC-19 | QSpec TC-196 D10's `B/size` clauses split across two linked source units (FR-480-AC-1), in both link orders and with either clause in either unit, give D10's one-unit results, with equal outcome and charges: `view_clauses` and the expansion read the member's own clauses from both units in ascending clause name. | Test (TC-472) |
| FR-120-AC-20 | D10's `absent-key` form: with `pre PB1 using V on M::B::size { (lookup<M::A>(M::PX, self) absent undefined) = self }` and `pre PB2 using V on M::B::size { false }` over a closed population `PX` with no runtime members, a call of `size` for `d1` is `undefined` with reason `absent-key` after `dispatch.select` and `PB1`'s charges, with no `function.call`, and `PB2` and `PA` are not evaluated. With the two bodies' names swapped it is `undefined` with reason `precondition-false` after the false clause's and then `PA`'s charges, the lookup clause neither evaluated nor charged. | Test (TC-472) |
| FR-120-AC-21 | FR-120-AC-6's runs with `CanInc`: under a meter budget whose `work_units` is 0 and every other counter at its default, the `Stopped` outcome's `ExpansionStop` names `EvaluationMeter` with counter `work_units`, the counter whose next charge was denied; with `max_candidates` 2 and `increment` with no clause, it names `MaxCandidates` and carries no counter. | Test (TC-472) |
| FR-120-AC-22 | Over a model whose object type `Cell` (population `cells`, universe `{k1}`) declares `cfg: Cfg` (record `Cfg { on: Boolean, n: Int[0, 2] }`), `pr: Pr` (tuple `Pr(Boolean, Int[0, 1])`) and `w: Wrap` (record `Wrap { c: Cfg, o: Option<Boolean> }`), with an operation `put()` whose frame is `modifies cfg, pr, w` and no clause, each field's domain is finite and `domains()` lists no unbounded root for them. `cfg` has 6 values, ordered `on` first: (`false`, 0), (`false`, 1), (`false`, 2), (`true`, 0), (`true`, 1), (`true`, 2). `pr` has 4: (`false`, 0), (`false`, 1), (`true`, 0), (`true`, 1). `w` has 6 × 3 = 18 values, `c` first in `Cfg` order, `o` absent, then `false`, then `true`. An application of `put` from a state holding `k1` has 6 × 4 × 18 = 432 candidates, each counted against `max_candidates`. In a separate model, a granted field `oc: Option<Cfg>` has 7 values, and so has `sq: Sequence<Cfg>[0, 1]` (the empty sequence, then the 6 one-element sequences); a two-component record `R1 { c: Cfg, a: Boolean }` has 6 × 2 = 12. | Test (TC-1270) |
| FR-120-AC-23 | Over an enum `Color` (`red`, `blue`, `green`): an operation parameter `c: Color`, an operation result of type `Color` and a granted field `col: Color` are each a finite domain of 3 values in declaration order, and `domains()` lists no unbounded root for them. A record `Pick { col: Color, on: Boolean }` has 6 values, `col` most significant. A presence-optional field `note?: Boolean` of a record `Note { note?: Boolean, n: Int[0, 1] }` gives 3 × 2 = 6 values, `note` absent first. | Test (TC-1270) |
| FR-120-AC-24 | Over a model declared `time dense unit ms` (FR-230; a `Clock` field is refused `invalid_timed_model`/`clock-without-dense-time` otherwise), with `ck: Timer` (record `Timer { x: Clock, n: Int[0, 3] }`) `sw: Stopwatch` (record `Stopwatch { x: Clock, y: Clock }`) and `mx: Mix` (record `Mix { t: Timer, b: Boolean }`) on `Cell`, and an operation `retime()` with frame `modifies ck, sw, mx` and postcondition `self.ck = Timer { x: 0 ms, n: 3 }` (FR-230-AC-11): `domains()` lists a root for each of `ck`, `sw` and `mx` and none is unbounded. `Timer`'s domain is the 4 values of `n` ascending, `x` contributing no factor; `Stopwatch` has exactly one value; `mx` has 4 × 2 = 8 values, `t` most significant. A successor of `retime` holds `ck.n` 3 and `x` reset, and `sw` keeps its pre-state clocks. With no postcondition, a candidate keeps each clock at its pre-state value. No candidate ranges over a value of `x`. A granted field of `Sequence<Timer>[0, 1]` or `Sequence<Clock>[0, 1]` is bounded (FR-120-AC-26). | Test (TC-1270) |
| FR-120-AC-25 | Over AC-22's model, `op: Open` (record `Open { n: Integer, b: Boolean }`), `tm: Tm` (record `Tm { x: Clock, k: Integer }`) and `wr: Wrap2` (record `Wrap2 { o: Open }`) added as fields, one at a time in a frame that grants it, are unbounded roots: `explore_model` and `sample_model` return `NotSimulated::RequiresBound` naming each field's node key, before any state is expanded. A field `choice: Choice`, where the admitted model variant type `Choice` has nullary case `Empty` and `Enabled` with payload type `Boolean`, is finite with three values in order: `Empty`, `Enabled(false)`, `Enabled(true)`; it adds no unbounded root. A variant case with payload type `Integer` is unbounded. A clock component does not make `tm` bounded, and `Wrap2` is unbounded by its nested component. | Test (TC-1270) |
| FR-120-AC-26 | In a model with `time dense unit ms`, `Cfg` and `Open` as in AC-22 and AC-25 and `Tick { x: Clock }`, a frame that grants the field, `sc: Sequence<Cfg>[0, 2]` is a finite value domain of 1 + 6 + 36 = 43 values: the empty sequence, then the 6 one-element sequences in `Cfg` order, then the 36 two-element sequences with the first element most significant (`[c0, c0]`, `[c0, c1]`, ..., `[c5, c5]`). `st: Sequence<Tick>[0, 3]` and `sk: Sequence<Clock>[0, 3]` have 4 values each, one for each length, and so does `so: Sequence<Option<Clock>>[0, 3]`. With AC-24's `Timer { x: Clock, n: Int[0, 3] }` (`p` = 4), `sr21: Sequence<Timer>[0, 2]` has 1 + 4 + 16 = 21 values and `sr85: Sequence<Timer>[0, 3]` has 1 + 4 + 16 + 64 = 85, ordered length first and then by `n` with the first element most significant (`[]`, `[n0]`, ..., `[n3]`, `[n0, n0]`, `[n0, n1]`, ...); the `x` adds no factor. `Set<Cfg>[0, 1]` has 1 + 6 = 7 values and `Set<Cfg>[0, 2]` has 1 + 6 + 15 = 22, ordered by count and then by ascending `Cfg`; `Bag<Cfg>[0, 2]` has 1 + 6 + 21 = 28; with `Timer` as above `Set<Timer>[0, 2]` has 1 + 4 + C(4, 2) = 11 values and `Bag<Timer>[0, 2]` has 1 + 4 + C(5, 2) = 15, ordered by count and then by `n`; from a pre-state set `{ {x: 5 ms, n: 1} }`, the candidate `{n 1, n 2}` holds `x` 5 ms for `n` 1 (matched by equal `n`) and 0 ms for `n` 2, and the candidate `{n 2}` holds `x` 0 ms; `Set<Clock>[0, 2]` has 2, and so does `Set<Clock>` with no declared maximum (FR-230 caps it at one element). No candidate ranges over a clock slot's value. From a pre-state in which `sr21` is `[ {x: 5 ms, n: 1} ]`, the candidate of length 2 with `n` values 1, 2 holds the pre-state `x` 5 ms in slot 0 and a fresh `x` 0 ms in slot 1; the candidate of length 0 holds no slot; from a pre-state of length 0 every slot of a length-2 candidate is fresh at 0 ms. A `Reference<Clock>` field, `Sequence<Reference<Clock>>[0, 1]`, `Sequence<Open>[0, 1]`, `Sequence<Cfg>` and `Set<Cfg>` with no declared maximum, and a `Map<Boolean, Boolean>[0, 1]` are unbounded: `explore_model` and `sample_model` return `NotSimulated::RequiresBound` naming the field's node key, before any state is expanded. | Test (TC-1270) |
## Dependencies

- [FR-215](FR-215-implement-protocolsystem-as-a-transitionsystem.md): the
  record value domain and its order for protocol binders (ADR-027 ST-2,
  ST-11), whose order FR-120's record domain follows; FR-230 (clock
  places, record-literal reset).
- QSpec FR-181, its exploration contract, typed canonical form and AC-4 to
  AC-6; QSpec FR-013 (frames, including its Behavior's `modifies`, `creates`
  and `deletes` rules); QSpec FR-151 (the receiver, effective
  pre- and postconditions, conformance); QSpec `state-contract.md` and
  `shared-grammar.md` (anchor operations are caller-side);
  `native-diagnostics.md`.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md):
  the engine, its `TransitionSystem` trait, findings, `Outcome::Stopped`,
  `StopReason::Stopped` and replay.
- [FR-056](FR-056-admit-domain-package-model-declarations.md): the spine
  reads bound scalar field, parameter and result types such as `Int[0, 2]`
  through FR-056's `value-type/v1` scalar reader, which is on main. Every AC's fixture uses bound scalars.
- [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
  (operations and effects), [FR-104](FR-104-check-state-clauses.md) (checked
  clauses and their S3 refusals),
  [FR-106](FR-106-admit-snapshots-and-invocations.md) (snapshot admission,
  its model views and value reader),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (`evaluate_clause`).
- [FR-114](FR-114-bind-a-protocol-attempt-to-its-operation-frame.md) (an
  operation's one frame, including an inherited operation's) and
  [FR-115](FR-115-run-an-operation-frame-over-an-invocation.md) (the frame
  check over a pre and post observation, and its verdicts): the frame
  semantics FR-120 uses, through `population::decide_frame`.
- The S4 `CheckedPackage` and FR-056's package input, which are the
  simulator's whole input.

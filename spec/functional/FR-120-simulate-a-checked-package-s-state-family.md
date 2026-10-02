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
the state key, the pinned sampler, replay, findings and the stopped
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
- `packages`, FR-056's package input (domain package bytes by `sha256-jcs`
  digest), and the `ModelNormalizationLimits` FR-106 admission takes;
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
    pub fn domains(&self) -> Vec<(WireNodeId, ValueType)>;
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
- `explore_model` is FR-101's `explore_request` over the system, its
  `domains()` and the package's `TypeEnvironment`. `sample_model` is FR-101's
  `sample_request` over the same inputs, followed by `replay` of the sampled
  trace against the system, which yields one `StepEffect` per step.
  `ModelTrace.trace` holds FR-181's transition identities, state-key digests
  and findings; `ModelTrace.effects` holds the effects that replay derives.
  `sample_model` returns FR-101's `NotSimulated` as
  `ModelSampleError::NotSimulated`, and a trace that does not replay against
  the system that sampled it as `ModelSampleError::Replay`.

## Behavior

### Identities

The **declaration identity text** of a `DeclarationKey` is the RFC 8785 JCS
text of its existing `$defs.DeclarationKey` preimage (`qsl-semantics/src/
model/key.rs:84-124`, QSpec `model-complete.md` "Identity domains"):
`{"digest_domain":"sha256-jcs","node":"<node>","package":"<package>"}`. It
names the package, so two domain packages with equal node identities give
different texts. It is the text form for every declaration below: the
`semantic` map's population keys, a record's `name`, a transition's
operation and an anchor's handler name. This is QSL's choice; FR-181's
`<qualified-name>` leaves the text form to the implementation.

### Initial states

`ModelSystem::new` checks `universes`, then admits each `initial` document
in the given order, and returns the first failure:

1. A universe population that is no population of the package refuses
   `invalid_runtime_input`/`wrong-role-mapping`. A key repeated within one
   universe refuses `invalid_runtime_input`/`conflicting-identity`.
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
`{"type":"simulation-state","semantic":…,"control":…,"queues":…,"roles":…,"observations":…,"bounds":…}`.
`control`, `queues`, `roles`, `observations` and `bounds` are empty maps
(`{"type":"map","entries":[]}`), matching a state family whose only state is
its populations. FR-181 leaves a population state's typed canonical form to the
implementation, and QSL's `semantic` member is a map in FR-181's map form:

- key: each universe population's declaration identity text, as `text`;
- value: a map from each object's reference triple to a `record` whose
  `name` is the declaration identity text of the object's most-specific type
  and whose `fields` are its declared fields in declaration order, each
  encoded by its declared value type (FR-181 "Typed canonical form").

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

pub struct StateDelta {
    pub created: Vec<ObjectReference>,
    pub deleted: Vec<ObjectReference>,
}

impl StateModel {
    pub fn new(
        model_selections: &[DomainPackageRef],
        types: &TypeEnvironment,
        packages: &BTreeMap<[u8; 32], Vec<u8>>,
        limits: ModelNormalizationLimits,
    ) -> Result<Self, AdmissionFailure>;
    pub fn admit_initial(
        &self,
        provision: &BTreeMap<[u8; 32], Vec<u8>>,
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> Result<(ModelState, SelectedAnchor), AdmissionFailure>;
    pub fn check_frame(
        &self,
        effect: &OperationEffect,
        pre: &ModelState,
        post: &ModelState,
    ) -> Result<StateDelta, AdmissionFailure>;
    pub fn observation(
        &self,
        state: &ModelState,
        identity: DocumentRef,
    ) -> Result<Observation, AdmissionFailure>;
}
```

- `check_frame` is FR-106 check 11 over two in-memory states. Per
  population, in ascending population `DeclarationKey` order, it first
  computes the delta from the two states: the created keys are the post
  state's keys absent from the pre state, and the deleted keys the pre
  state's keys absent from the post state. It then builds the population's
  `FrameObject`s and calls `population::decide_frame`, the one frame
  decision `enforce_frame` and FR-106's `observation::frame::enforce` also
  call, passing those created and deleted keys as
  `FrameDecision.declared_created` and `declared_deleted`. It returns the
  computed delta, or check 11's first finding as `AdmissionFailure::Refused`
  (`frame_violation`/`unauthorized-change` or `population_delta_mismatch`/
  `delta-disagreement`). Its verdicts are the ones FR-115 reports.
- `observation` builds FR-106's `Observation` for a state: `identity` is the
  given `DocumentRef`; `environment` is the `ObjectEnvironment` of every
  object with every field value, built by FR-106's builder; `populations`
  maps each population to `true` (complete).
- A synthesized state's `DocumentRef` has labels `authority` `quire`,
  `identity` `quire.simulation.state/<state-key digest, 64 lowercase hex>`,
  and its digest is the state-key digest (QSpec STD-150).
- `ModelSystem` builds each clause evaluation's `AdmittedObservations`
  directly, since its fields are public (`qsl-semantics/src/model/
  observation.rs:309-335`): `usage` is `AdmissionUsage::default()`; `clause`
  is the clause's node key; for an invariant, `current` is the state's
  observation; for a precondition, `pre` is the state's observation and
  `post` is none (FR-107 reads `pre` only); for a postcondition, `pre` is
  the state's and `post` the candidate's observation, `created` and
  `deleted` are `check_frame`'s delta; `self_object` is the receiver, or
  the invariant's object; `parameters` are the arguments by declared name;
  `result` is the candidate result or none.

### Applications and transition identities

For each object `r` of a state and each operation name of `r`'s effective
view (FR-103, FR-081), the application's operation is the effective member
that name selects for `r`'s most-specific type. Its arguments are `r`, then
one value per declared parameter from the parameter's domain. The receiver
is argument 0, as QSpec FR-151 makes it parameter 0.

The transition identity is FR-181's
`{"type":"transition","operation":"<qualified-name>","arguments":[<encoded>, ...]}`,
and `<qualified-name>` is the operation's declaration identity text.

A value domain is: `Boolean`, `false` then `true`; `Int[lower, upper]`,
every integer in the range; `Option<T>`, absent then each value of `T`;
`Reference<T>`, each object of the state, or of the post-state for a field
or result, whose most-specific type conforms to `T`; a sequence with a
declared maximum `N`, every sequence of length 0 to `N` over its element
domain. Every other type is an unbounded domain in `domains()`.

### Contract clauses

A `pre` or `post` clause applies to an application when it resolves to the
application's operation (FR-104) and `r`'s most-specific type conforms to
the clause's context type.

The effective precondition is `true` when no `pre` clause applies, and
otherwise is the conjunction of the applicable clauses. QSpec FR-151:88-90
defines the effective precondition as "the disjunction of its own
precondition clauses with the effective preconditions of the members it
redefines", which joins a member's own precondition to the inherited ones,
and makes an absent precondition `true`. How several own clauses of one
member combine is QSL's inference where FR-151 is silent: they are
conjoined, as own postcondition clauses are, which is the reading TC-196
D06 ("its absent (true) clause disjoined with PA") and `effective_terms`
(`qsl-semantics/src/check/checked_dispatch.rs:477`) take. On the spine every
operation record's `redefines` is `None` (`qsl-semantics/src/model/
intake.rs:1444`), so the applicable clauses are the selected member's own.
The effective postcondition is the conjunction of the applicable `post`
clauses, and `true` when none applies.

A conjunction is evaluated through FR-107's `evaluate_clause`, each call
with a fresh meter, over every applicable clause in ascending name order
(UTF-8 bytes):

- An `Incomplete` result stops the expansion with `resource_exhausted`/
  `insufficient-next-charge` at once: the successor set is not known.
- A `CallFailure` stops the expansion with `runtime_invariant`/
  `established-invariant-broken` at once: the simulator built the
  observations itself, so a refused selection is a broken internal
  invariant.
- Otherwise, the conjunction is `true` when every clause is
  `Completed(true)` and `false` when any clause is `Completed(false)`. When
  no clause is `Completed(false)` and one or more are `Undefined` or
  `Refused`, the conjunction is undecided, and the expanded state gains one
  `ModelFinding::ContractUndetermined { transition, candidate, clause,
  evaluation }` per such clause, in name order; `candidate` is the
  candidate's state-key digest for a postcondition and none for a
  precondition. The result depends only on the set of clause outcomes.

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

Each candidate goes through `StateModel::check_frame` with the state as
`pre` and the candidate as `post`. A candidate is within the frame exactly
when the check returns a delta, the case FR-115 reports as `success`, and
that delta is the step's `created` and `deleted`. The construction yields
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
candidate it generates, including one the frame check or closure rejects,
and every (candidate, result) pair it evaluates, over all its applications
(ADR-016 G-4). When the count would exceed `max_candidates`, the expansion
stops with `resource_exhausted`/`insufficient-next-charge`.

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

Each root's `WireNodeId` is `WireNodeId::from_digest` of the SHA-256 of the
JCS bytes of a naming object: for a population or a field, its
`$defs.DeclarationKey` preimage; for a parameter,
`{"operation":<the operation's preimage>,"parameter":"<name>"}`; for a
result, `{"operation":<the operation's preimage>,"result":"result"}`. This
naming is QSL's choice; FR-101 takes the ids from its caller. A root that
`classify_extent` finds unbounded makes `explore_model` and `sample_model`
return `NotSimulated::RequiresBound` naming it, before any state is
expanded.

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
`test/counters/Counter` with fields `value: Int[0, 2]`, `label: Int[0, 1]`
and `next: Option<Reference<Counter>>`, and population
`test/counters/counters` of `Counter`. Its operations are listed per AC,
each with node `test/counters/Counter/<name>`. The initial state `s0` holds
`c1` (`value` 0, `label` 0, `next` absent), anchored `{initialization,
start}`, and the universe of `counters` is `[c1, c2]`, unless an AC says
otherwise. `v<n>` is `s0` with `c1.value` = n. `<op>` is the declaration
identity text of operation `op`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-120-AC-1 | With operation `increment()` (frame `modifies value`) and no clause, `s0` has three successors, `v0`, `v1` and `v2`, all with transition identity `{"type":"transition","operation":"{\"digest_domain\":\"sha256-jcs\",\"node\":\"test/counters/Counter/increment\",\"package\":\"test/counters\"}","arguments":[<c1's reference>]}`; `label` and `next` keep their values in each. Adding `post Inc on Counters::Counter::increment { self.value = pre(self.value) + 1 }` leaves one successor, `v1`. With `pre CanInc … { self.value < 2 }` and no `post` clause, `v0` and `v1` each have the three successors and `v2` has no `increment` successor. | Test (TC-471) |
| FR-120-AC-2 | With `setTo(n: Int[0, 2])` (frame `modifies value`, post `self.value = n`), `s0` has one successor per `n`, with arguments `[<c1>, {"type":"integer","value":"0"}]` to `…"2"}]`. With `c2` added to `s0`, each receiver gives its own identity. With `pre A { self.value >= 1 }` and `pre B { self.value <= 1 }` on `setTo`, both must hold: it is enabled at `v1` and not at `v0` or `v2`. | Test (TC-471) |
| FR-120-AC-3 | With `spawn()` (frame `creates Counter`) and no clause, `s0` has 1 + 18 successors: `s0` itself, and `c2` created with each (`value`, `label`, `next`) in 3 × 2 × 3 (absent, `c1`, `c2`). With `remove()` (frame `deletes Counter`) over `c1` and `c2`, where `c1.next` names `c2`, each receiver's successors are the deletion sets {}, {`c1`} and {`c1`, `c2`}; deleting `c2` alone leaves `c1.next` dangling and is not a candidate. Each successor's delta is the one `StateModel::check_frame` returns. | Test (TC-471) |
| FR-120-AC-4 | `s0`'s state key has `semantic` equal to `{"type":"map","entries":[{"key":{"type":"text","value":"{\"digest_domain\":\"sha256-jcs\",\"node\":\"test/counters/counters\",\"package\":\"test/counters\"}"},"value":{"type":"map","entries":[{"key":{"type":"reference","universe":"<U>","object_type":"<E>","identity":"6331"},"value":{"type":"record","name":"{\"digest_domain\":\"sha256-jcs\",\"node\":\"test/counters/Counter\",\"package\":\"test/counters\"}","fields":[{"name":"value","value":{"type":"integer","value":"0"}},{"name":"label","value":{"type":"integer","value":"0"}},{"name":"next","value":{"type":"option","value":null}}]}}]}}]}`, where `<U>` and `<E>` are the universe and effective type of the `ObjectReference` that FR-106 admission gives `c1` in `s0`'s admitted `Observation` (`StateModel::admit_initial`, independent of the simulator's key encoding), each as 64 lowercase hex digits; `control`, `queues`, `roles`, `observations` and `bounds` are `{"type":"map","entries":[]}`. `s0` with `c1.label` 1 has a different key. | Test (TC-471) |
| FR-120-AC-5 | With `increment`, `CanInc`, `Inc` and `invariant Small … at current { self.value < 2 }`, exploration is `Exhaustive` with 3 states, 2 transitions and depth 2, and `findings` holds exactly one entry: `v2`'s digest, depth 2, `InvariantViolated { Small, c1, {handler, <increment>} }`. Adding `reset()` (frame `modifies value`, post `self.value = 0`) makes the run `Exhaustive` with 3 states and 5 transitions, including `v2 → v0`: the violating state is expanded. `s0`'s findings, when any, carry anchor `{initialization, start}`. | Test (TC-472) |
| FR-120-AC-6 | With `increment`, `Small`, no `pre` or `post` clause and a meter budget of zero, exploration is `Exhaustive` with 3 states, and each state has `InvariantUndetermined` for `Small` and `c1`, its evaluation `Incomplete` with `resource_exhausted`/`insufficient-next-charge`. With budget zero and `CanInc`, the run is `Outcome::Stopped` with cause `resource_exhausted`/`insufficient-next-charge`, frontier `[<s0>]`, category incomplete. With `max_candidates` 2 and `increment` with no clause, the run is `Stopped` with the same cause. | Test (TC-472) |
| FR-120-AC-7 | `ModelSystem::new` refuses an initial snapshot with `observation` `pre` (`wrong_snapshot`/`wrong-observation`), one anchored `{handler, validate}` (`wrong_snapshot`/`wrong-anchor`), one whose anchor kind is `other` (`invalid_runtime_input`/`wrong-value-kind` with field `anchor`, refused by FR-106's document reader, `qsl-semantics/src/model/observation/document.rs:525-528`, before check 3), and one holding `c3` (`invalid_runtime_input`/`invalid-value`); a universe listing `c1` twice refuses `invalid_runtime_input`/`conflicting-identity`; a universe for a population the package does not declare refuses `invalid_runtime_input`/`wrong-role-mapping`; a universe population absent from the snapshot returns `Incomplete` with `incomplete_population`/`incomplete-scope`. With no universe for `counters`, or with an operation parameter typed `Integer`, `explore_model` returns `NotSimulated::RequiresBound`, and `sample_model` `ModelSampleError::NotSimulated(RequiresBound)`, naming that root's `WireNodeId` and expand no state. | Test (TC-472) |
| FR-120-AC-8 | A `sample_model` trace (seed `424242`, trace `0`, `max_steps` 4) over the AC-5 package ends `NoSuccessors` at `v2`, records `Small`'s finding at step 2, and replays; the same trace with that finding removed refuses `ReplayError::FindingMismatch` at step 2. With budget zero and `CanInc`, the trace ends `StopReason::Stopped(resource_exhausted/insufficient-next-charge)` at step 0 and replays; replayed against the same package with the default budget it refuses `ReplayError::Stopped { step: 0, recorded: Some(…), replayed: None }`. Two runs with equal inputs give equal `Exploration`s and equal `ModelTrace`s. | Test (TC-472) |
| FR-120-AC-9 | Over package `test/tallies` (object type `Tally` with `items: Sequence<Int[0, 2]>[1, 1]`, population `tallies`, operation `touch()` with an empty frame) and unit type `Tiny = Int[0, 1]`: from `t1` with `items` `[2]`, `pre Low { sum<Tiny>(x in self.items: x) = 0 }` makes `touch` not enabled and records `ContractUndetermined { <touch identity>, none, Low, Undefined(SumOutOfDomain) }`; adding `pre Never { false }` records no finding, whatever the two names; `post LowPost` with `Low`'s body records the finding with `candidate` = `t1`'s state-key digest and gives no successor. | Test (TC-472) |
| FR-120-AC-10 | With `isZero(): Boolean` (empty frame, `post Z { result = (self.value = 0) }`), `v0` has one `isZero` successor, `v0`, whose step effect has result `{"type":"boolean","value":true}`, and `v1` has one, `v1`, with result `false`; the `isZero` result root is in `domains()`. | Test (TC-473) |
| FR-120-AC-11 | For `increment` (post `Inc`), `spawn` and `remove` over `s0` and state `w` = {`c1` (`value` 0), `c2` (`value` 0)}: the step `increment` from `v0` has effect `created: []`, `deleted: []`, `changed: [{object: c1, field: value, pre: {"type":"integer","value":"0"}, post: {"type":"integer","value":"1"}}]`, result none; `spawn` from `s0` to `s0` plus `c2` (`value` 1, `label` 0, `next` absent) has `created: [<c2>]` and `changed: []`; `remove` with receiver `c1` from `w` to {`c2`} has `deleted: [<c1>]`. Exploring the AC-5 package twice, the second time in a child process started in an empty temporary directory, gives equal `Exploration`s, and `s0`'s key bytes are equal before and after exploring. | Test (TC-473) |
| FR-120-AC-12 | A unit whose function body reads `self`, whose invariant reads `result`, whose invariant reads `pre(self.value)`, or whose function calls `reaches`, refuses at S3 with `missing_declaration`/`missing-name`, `wrong_snapshot`/`wrong-anchor`, `wrong_snapshot`/`forbidden-pre-read` or `ill_typed`/`operator-ineligible`, each at the source span of that form, and yields no `CheckedPackage` to simulate. | Test (TC-473) |
| FR-120-AC-13 | With `spawn()` over a population whose universe has 30 unused keys and each created object's fields ranging over `Int[0, 2]`, `Int[0, 1]` and `Option<Reference<Counter>>`, so the candidate product exceeds 10^9, and `max_candidates` 10, exploration returns `Outcome::Stopped` with `resource_exhausted`/`insufficient-next-charge` after generating at most 11 candidates, and does not allocate the product. | Test (TC-472) |

## Dependencies

- QSpec FR-181, its exploration contract, typed canonical form and AC-4 to
  AC-6; QSpec FR-013 (frames, including its Behavior's `modifies`, `creates`
  and `deletes` rules); QSpec FR-151 (the receiver as parameter 0, effective
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

## Status

Specified; not yet implemented. TC-471 to TC-473
planned. FR-056's bound scalar reader, which every AC's fixture uses, is
on main.

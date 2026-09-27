---
id: FR-120
title: "Simulate a checked package's state family over its operation frames"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-101
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
for operation frames. FR-101 implements FR-181's engine: canonical order, the
state key, the pinned sampler, replay and the stopped outcomes, over any
`TransitionSystem`. This requirement supplies the model-level
`TransitionSystem` for a checked package's state family, and the three
FR-181 parts FR-101 leaves to it:

- the successor relation of FR-181's exploration contract: an operation
  applied to an argument vector from its finite parameter domains, enabled
  when its effective precondition is true, times every post-state within its
  frame (FR-013-AC-1) for which its postcondition holds;
- recording a successor that violates an invariant rather than pruning it
  (FR-181-AC-4, last clause);
- performing none of the effects a model describes: effects appear only as
  typed trace data, and a request for evaluator I/O, mutation or an ambient
  store refuses with its source location (FR-181-AC-6).

The simulator runs over the S4 in-memory `CheckedPackage`
(`CheckedPackage::link`). It reads no emitted bytes and needs no S5
emission. A unit whose S5 emission refuses today, such as one with `pre` or
`post` clauses (`Emit(UnlocatedOccurrence { role: Generated })`, pending
STD-111 and QSL-279), is simulated through its S4 package.

Where FR-181 leaves a choice open, this requirement names QSL's choice and
says so.

## Use case

A verification operator has a domain package with operations and frames and
a `1-draft` unit with invariants and operation contracts (US-003). They
supply a few initial snapshots and a finite key set for each population,
then explore every reachable state or sample a seeded trace. The result
names each reachable state where an invariant is false or could not be
decided, keeps exploring from it, and shows each step's creations, deletions
and field changes as data. A run that could not decide a successor set is
incomplete, never exhaustive.

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

FR-101's `explore_request` and `sample_request` take the resulting
`ModelSystem` with its `domains()`, the package's `TypeEnvironment`, the
extent walk's `position_limit`, and FR-101's own limits, poll, seed and
sampler inputs.

## Outputs

- `ModelSystem::new(request)`: a `ModelSystem`, or an FR-106
  `AdmissionFailure` (`Refused` or `Incomplete`, one `RefusalRecord`).
- Exploration: `Exploration<ModelFinding> { outcome, findings }` (FR-101's
  `Outcome`, with `Stopped` below), or FR-101's `NotSimulated`.
- Sampling: `ModelTrace`, FR-101's `Trace` with its findings and, per step,
  the transition's `StepEffect`.

## Behavior

### Engine amendment (FR-101)

FR-101's `TransitionSystem` is amended so that an expansion can report
findings and can stop:

```rust
pub trait TransitionSystem {
    type State;
    type TransitionId: Clone + Eq + std::fmt::Debug + Serialize;
    type Key: Serialize;
    type Finding: Clone + Eq + std::fmt::Debug;

    fn initial(&self) -> Vec<Self::State>;
    fn key(&self, state: &Self::State) -> Self::Key;
    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop>;
}

pub struct Expansion<T, S, F> {
    pub successors: Vec<(T, S)>,
    pub findings: Vec<F>,
}

pub struct ExpansionStop {
    pub cause: CatalogCode,
}

pub struct StateFindings<F> {
    pub state: DigestRecord,
    pub depth: usize,
    pub findings: Vec<F>,
}
```

- `explore_request` returns `Exploration<S::Finding> { outcome: Outcome,
  findings: Vec<StateFindings<S::Finding>> }`. Each expanded state with at
  least one finding contributes one entry, in expansion order, with its
  findings in the order the system returned them.
- An `Err(ExpansionStop)` ends exploration with `Outcome::Stopped { stats,
  frontier, cause }`. Its frontier is the state whose expansion stopped,
  then the queue in next-expansion order. `Outcome::category()` gives
  `Stopped` the O-16 category of `cause`'s code: incomplete for
  `resource_exhausted`, internal failure for `runtime_invariant`.
- Sampling expands every state on the trace, the last included, and records
  its findings in `Trace.findings` (depth is the step index, 0 for the
  initial state). The last state's successors are not drawn from. An
  `ExpansionStop` ends the trace with `StopReason::Stopped(cause)`.
- Replay expands each state again. A recomputed finding list that differs
  from the recorded one refuses `ReplayError::FindingMismatch { step }`; an
  `ExpansionStop` refuses `ReplayError::Stopped { step, cause }`.

FR-101's order, keys, digests, limits, cancellation and requires-bound are
unchanged.

### Initial states

`ModelSystem::new` checks `universes`, then admits each `initial` document
in the given order, and returns the first failure:

1. A universe population that is no population of the package refuses
   `invalid_runtime_input`/`wrong-role-mapping`. A key repeated within one
   universe refuses `invalid_runtime_input`/`conflicting-identity`.
2. Each initial document is a `quire.state.snapshot/v1` snapshot (FR-106
   "Document forms"). Admission runs FR-106 check 1 (read), check 3, check 4
   (model), check 6 (populations and values), check 7 and check 8
   (closure), with these readings:
   - check 3: `observation` is `current` and `anchor.kind` is
     `initialization`. Another `observation` refuses `wrong_snapshot`/
     `wrong-observation`, and another anchor kind refuses `wrong_snapshot`/
     `wrong-anchor`. An initial state is an initialization observation
     (QSpec `state-contract.md`, "Operation anchors, aliases and captures").
   - check 7 applies to every population of the document. Each universe
     population is also required: a universe population the document omits
     returns `Incomplete` with `incomplete_population`/`incomplete-scope`,
     naming it.
3. An object whose key is not in its population's universe refuses
   `invalid_runtime_input`/`invalid-value`, naming the population and key.
   A population the document holds without a universe is unbounded, and
   `domains()` names it (see "Bounds").

Equal initial states coalesce by key, as FR-101 admits them.

### State and state key

A model state is one complete, closed population state: for each universe
population, its objects, each with its key, its most-specific type and a
value for each declared field. Every reference value names an object of the
state.

The state key is FR-181's
`{"type":"simulation-state","semantic":…,"control":…,"queues":…,"roles":…,"observations":…,"bounds":…}`.
`control`, `queues`, `roles`, `observations` and `bounds` are empty maps:
the state family has no control positions, queues, roles, observation
progress or remaining bounds. FR-181 does not give a population state's
typed canonical form. QSL's `semantic` member is a map in FR-181's map form:

- key: each universe population's declaration identity, as `text`;
- value: a map from each object's reference triple to a `record` whose
  `name` is the object's most-specific type's declaration identity and whose
  `fields` are its declared fields in declaration order, each encoded by its
  declared value type (FR-181 "Typed canonical form").

A reference encodes as FR-181's triple: `universe` is the population's
`UniverseId` as 64 lowercase hex digits, `object_type` the most-specific
type's `EffectiveId` as 64 lowercase hex digits, and `identity` the
lowercase hex of the key's UTF-8 bytes (`c1` is `6331`).

### Applications and transition identities

For each object `r` of a state and each operation name of `r`'s effective
view (FR-103, FR-081), the application's operation is the effective member
that name selects for `r`'s most-specific type. Its arguments are `r`, then
one value per declared parameter from the parameter's domain. The receiver
is argument 0, as QSpec FR-151 makes it parameter 0.

The transition identity is FR-181's
`{"type":"transition","operation":"<qualified-name>","arguments":[<encoded>, ...]}`.
QSL's `<qualified-name>` is the operation's `DeclarationKey` node identity
(FR-103), not the unit's alias spelling, so it is the same in every unit
that selects the package.

A value domain is: `Boolean`, `false` then `true`; `Int[lower, upper]`,
every integer in the range; `Option<T>`, absent then each value of `T`;
`Reference<T>`, each object of the state, or of the post-state for a field
or result, whose most-specific type conforms to `T`; a sequence with a
declared maximum `N`, every sequence of length 0 to `N` over its element
domain. Any other type is unbounded, and `domains()` names it.

### Enabled applications

A `pre` clause applies to an application when it resolves to the
application's operation (FR-104) and `r`'s most-specific type conforms to
the clause's context type. The spine's operation records carry no
`redefines` link (FR-103 intake sets none), so the applicable clauses are
the selected member's own. QSpec FR-151 makes the effective precondition
"the disjunction of its own precondition clauses with the effective
preconditions of the members it redefines": the disjunction joins a member's
own precondition to the inherited ones, and a member's own clauses are
conjoined, as its postcondition clauses are. An absent precondition is
`true`. So the effective precondition is `true` when no clause applies, and
otherwise is true when every applicable clause is `Completed(true)`.

The simulator evaluates the applicable clauses in ascending name order
(UTF-8 bytes) through FR-107's `evaluate_clause`, over the state as the
`pre` observation, `self` = `r` and the argument values, each call with a
fresh meter. Per clause:

- `Completed(true)`: the next clause is evaluated. When every applicable
  clause is `Completed(true)`, the application is enabled.
- `Completed(false)`: the application is not enabled, and no further clause
  is evaluated.
- `Undefined` or `Refused`: the parent state gains a
  `ModelFinding::ContractUndetermined` naming the transition identity, the
  clause and the `Evaluation`; the application is not enabled, and no
  further clause is evaluated.
- `Incomplete`: the expansion stops with `resource_exhausted`/
  `insufficient-next-charge`. The successor set is not known, so the run
  cannot be exhaustive.
- `CallFailure`: the expansion stops with `runtime_invariant`/
  `established-invariant-broken`. The simulator builds the observations
  itself, so a refused selection is a broken internal invariant.

### Post-states

The application's frame is its operation's frame as FR-114 binds it: the
operation's one `OperationEffect` (FR-103), the declaring type's frame for
an inherited operation, and never a frame of the caller's. The request
carries no frame and no permission (FR-115, QSpec FR-013-AC-3). Simulation
reads the `OperationEffect` from the package, so it also applies an
operation that no clause or attempt names; FR-115's `Frame` selection needs
that operation's frame node, and simulation does not.

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

A candidate in which a reference names no object of the candidate is not a
state and is not a successor.

Each candidate's frame check is FR-115's: FR-106 check 11, in its order,
through the one frame decision in code
(`qsl_semantics::model::population::decide_frame`), with the state as the
pre observation, the candidate as the post observation, and the computed
delta as the declared delta. A candidate is within the frame exactly when
that check finds nothing, the case FR-115 reports as `success`, and the
decision's computed delta is the step's `created` and `deleted`. The
construction above yields only such candidates, so a finding from the check
(an FR-115 `violation`, or a `population_delta_mismatch`) stops the
expansion with `runtime_invariant`/`established-invariant-broken`.

For an operation that declares a result, each candidate pairs with each
value of the result type's domain over the post-state, in ascending order of
the value's typed canonical JCS bytes. The effective postcondition is the
conjunction of the applicable `post` clauses (applicability as for `pre`),
and `true` when none applies. It is evaluated through `evaluate_clause` over
the state as `pre`, the candidate as `post`, `self` = `r`, the arguments and
the result, in ascending clause name order, stopping at the first
`Completed(false)`. A candidate is a successor when some result makes every
applicable clause `Completed(true)`. The first such result is the
transition's result. An `Undefined` or `Refused` clause adds a
`ContractUndetermined` finding that also names the candidate's state-key
digest, and that (candidate, result) pair fails. `Incomplete` and
`CallFailure` stop the expansion as for a precondition.

Each expansion counts the (candidate, result) pairs it evaluates, over all
its applications. When the count would exceed `max_candidates`, the
expansion stops with `resource_exhausted`/`insufficient-next-charge`.

### Invariants

Every expansion evaluates each invariant clause of the package, in
ascending name order, for each object of the state whose most-specific type
conforms to the clause's context type, in ascending reference-key order.
Each evaluation goes through `evaluate_clause`, over the state as the
`current` observation, with a fresh meter. The expanded state gains:

- `ModelFinding::InvariantViolated { clause, self_object }` for
  `Completed(false)`;
- `ModelFinding::InvariantUndetermined { clause, self_object, evaluation }`
  for `Undefined`, `Refused` or `Incomplete`.

A `CallFailure` stops the expansion with `runtime_invariant`/
`established-invariant-broken`. An invariant finding never removes the
state or its successors: the state stays in the explored set, is expanded,
and its successors are explored (FR-181-AC-4). A state's invariant findings
come before its contract findings. Contract findings are in ascending
transition-identity bytes, then candidate state-key bytes (none first),
then clause name.

### Bounds

`ModelSystem::domains()` lists, for FR-101's requires-bound check: each
operation parameter type and result type; each field type of every field a
`modifies` entry grants and of every creatable type; and each population the
initial states hold, as a `Population` domain bounded by its universe's
length (FR-104's `Cardinality` bound). A population with no universe, or a
type with no finite domain above, is unbounded, and `explore_request` and
`sample_request` return `NotSimulated::RequiresBound` naming it before any
state is expanded.

### Effects as data

The simulator reads only its request. It reads no path, environment
variable, clock or search location, writes nothing, and changes no input:
`successors` takes the state by shared reference and returns new states.
The effects a model describes are its operations' frames, and each appears
only as data.

`sample_model` returns a `ModelTrace`: FR-101's `Trace` and one `StepEffect`
per step. A `StepEffect` holds the step's `created` and `deleted` object
references and its `changed` entries `{object, field, pre, post}`, objects
in ascending reference-key order and fields in declaration order, each value
in FR-181's typed canonical form. It also holds the transition's result, or
none. `StepEffect` is recomputed from the trace by replay; the trace keeps
FR-181's transition identities only.

No simulator input can ask the evaluator for I/O, mutation or an ambient
store. The complete-V1 grammar QSL parses has no I/O or assignment form, and
FR-107's evaluator takes only the package, the observations and the meter.
The forms that read ambient state outside their anchor refuse at S3, with
their source span, before a `CheckedPackage` exists (QSpec
`shared-grammar.md`: "`self`, `result` and `pre(...)` are caller-side anchor
operations, unavailable as implicit ambient state"):

- `self` outside a state clause: `missing_declaration`/`missing-name`
  (FR-104);
- `result` outside a `post` clause of an operation with a result:
  `wrong_snapshot`/`wrong-anchor` (FR-104);
- `pre(e)` outside a `post` clause: `wrong_snapshot`/`forbidden-pre-read`
  (FR-104);
- `reaches` outside a state clause: `ill_typed`/`operator-ineligible`
  (FR-104).

Every code and cause above is in QSpec `native-diagnostics.md` revision
`1-draft.8`.

## Acceptance Criteria

The fixture package `Counters` declares object type `Counter` with fields
`value: Int[0, 2]`, `label: Int[0, 1]` and `next: Option<Reference<Counter>>`,
and population `counters` of `Counter`. Its operations are listed per AC.
The initial state `s0` holds `c1` (`value` 0, `label` 0, `next` absent), and
the universe of `counters` is `[c1, c2]`, unless an AC says otherwise.
`v<n>` is the state `s0` with `c1.value` = n.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-120-AC-1 | With operation `increment()` (frame `modifies value`) and no clause, `s0` has three successors, `v0`, `v1` and `v2`, all with transition identity `{"type":"transition","operation":"<increment's node identity>","arguments":[<c1's reference>]}`; `label` and `next` keep their values in each. Adding `post Inc on Counters::Counter::increment { self.value = pre(self.value) + 1 }` leaves one successor, `v1`. With `pre CanInc … { self.value < 2 }` and no `post` clause, `v0` and `v1` each have the three successors and `v2` has no `increment` successor. | Test (TC-471) |
| FR-120-AC-2 | With `setTo(n: Int[0, 2])` (frame `modifies value`, post `self.value = n`), `s0` has one successor per `n`, with arguments `[<c1>, {"type":"integer","value":"0"}]` to `…"2"}]`. With `c2` added to `s0`, each receiver gives its own identity. With `pre A { self.value >= 1 }` and `pre B { self.value <= 1 }` on `setTo`, both must hold: it is enabled at `v1` and not at `v0` or `v2`. | Test (TC-471) |
| FR-120-AC-3 | With `spawn()` (frame `creates Counter`) and no clause, `s0` has 1 + 18 successors: `s0` itself, and `c2` created with each (`value`, `label`, `next`) in 3 × 2 × 3 (absent, `c1`, `c2`). With `remove()` (frame `deletes Counter`) over `c1` and `c2`, where `c1.next` names `c2`, each receiver's successors are the deletion sets {}, {`c1`} and {`c1`, `c2`}; deleting `c2` alone leaves `c1.next` dangling and is not a successor. Each successor's delta is the one `decide_frame` computes. | Test (TC-471) |
| FR-120-AC-4 | A state key's `semantic` member is the map of `counters` to its objects, each keyed by its reference triple, whose `identity` is `6331` for `c1`, and valued by a `record` named `Counter`'s declaration identity with fields `value`, `label`, `next` in that order. Two states differing only in `label` have different keys and are two states; `control`, `queues`, `roles`, `observations` and `bounds` are empty maps. | Test (TC-471) |
| FR-120-AC-5 | With `increment` and `CanInc`, `Inc` and `invariant Small … at current { self.value < 2 }`, exploration is `Exhaustive` with 3 states, 2 transitions and depth 2, and `findings` holds exactly one entry: `v2`'s digest, depth 2, `InvariantViolated { Small, c1 }`. Adding `reset()` (frame `modifies value`, post `self.value = 0`) makes the run `Exhaustive` with 3 states and 5 transitions, including `v2 → v0`: the violating state is expanded, not pruned. | Test (TC-472) |
| FR-120-AC-6 | With `increment`, `Small`, no `pre` or `post` clause and a meter budget of zero, exploration is `Exhaustive` with 3 states, and each of `v0`, `v1` and `v2` has `InvariantUndetermined` for `Small` and `c1`, its evaluation `Incomplete` with `resource_exhausted`/`insufficient-next-charge`. With budget zero and `CanInc`, the run is `Outcome::Stopped` with cause `resource_exhausted`/`insufficient-next-charge`, frontier headed by `s0`'s digest, category incomplete. With `max_candidates` 2 and `increment` with no clause, the run is `Stopped` with the same cause. | Test (TC-472) |
| FR-120-AC-7 | `ModelSystem::new` refuses an initial snapshot with `observation` `pre` (`wrong_snapshot`/`wrong-observation`), one anchored `handler validate` (`wrong_snapshot`/`wrong-anchor`), and one holding `c3` (`invalid_runtime_input`/`invalid-value`); a universe listing `c1` twice refuses `invalid_runtime_input`/`conflicting-identity`; a universe population absent from the snapshot returns `Incomplete` with `incomplete_population`/`incomplete-scope`. With no universe for `counters`, or with an operation parameter typed `Integer`, `explore_request` and `sample_request` return `NotSimulated::RequiresBound` naming that domain and expand no state. | Test (TC-472) |
| FR-120-AC-8 | A seeded `sample_model` trace over the AC-5 package records `Small`'s finding at the step whose state is `v2` and replays without the simulator; the same trace with that finding removed refuses `ReplayError::FindingMismatch` at that step. Two runs with equal inputs give equal `Exploration`s and equal `ModelTrace`s. | Test (TC-472) |
| FR-120-AC-9 | A `ModelTrace` step for `increment` from `v0` has effect `changed: [{object: c1, field: value, pre: {"type":"integer","value":"0"}, post: {"type":"integer","value":"1"}}]`, no creations, deletions or result; a `spawn` step creating `c2` has `created: [c2]`; a `remove` step has `deleted` listing the removed objects. Exploring and sampling in a process whose working directory is an empty temporary directory give the same results, and `s0`'s key is equal before and after exploration. | Test (TC-473) |
| FR-120-AC-10 | A unit whose function body reads `self`, whose invariant reads `result`, whose invariant reads `pre(self.value)`, or whose function calls `reaches`, refuses at S3 with `missing_declaration`/`missing-name`, `wrong_snapshot`/`wrong-anchor`, `wrong_snapshot`/`forbidden-pre-read` or `ill_typed`/`operator-ineligible`, each at the source span of that form, and yields no `CheckedPackage` to simulate. | Test (TC-473) |

## Dependencies

- QSpec FR-181, its exploration contract, typed canonical form and AC-4 and
  AC-6; QSpec FR-013 (frames, including its Behavior's `modifies`, `creates`
  and `deletes` rules); QSpec FR-151 (the receiver as parameter 0, effective
  pre- and postconditions, conformance); QSpec `state-contract.md`
  ("Operation anchors, aliases and captures") and `shared-grammar.md` (anchor
  operations are not ambient state); `native-diagnostics.md` revision
  `1-draft.8`.
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md):
  the engine, amended above.
- [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
  (operations and effects), [FR-104](FR-104-check-state-clauses.md) (checked
  clauses and their S3 refusals),
  [FR-106](FR-106-admit-snapshots-and-invocations.md) (snapshot admission
  and the frame decision),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (`evaluate_clause`).
- [FR-114](FR-114-bind-a-protocol-attempt-to-its-operation-frame.md) (an
  operation's one frame, including an inherited operation's) and
  [FR-115](FR-115-run-an-operation-frame-over-an-invocation.md) (the frame
  check over a pre and post observation, and its verdicts). FR-120 defines
  no frame semantics of its own.
- The S4 `CheckedPackage` only. Simulation needs no S5 emission, so FR-105
  and QSL-279 do not gate it.

## Status

Specified under QSL-274 (A05-4); not yet implemented. TC-471 to TC-473
planned.

---
id: ADR-016
title: "State, model and finite execution on the shared foundation (ARCH-40)"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-150
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
---
# ADR-016: State, model and finite execution on the shared foundation (ARCH-40)

## Status

Proposed, 2026-09-29. Owning ticket: Linear QSL-19 (GitHub #220, ARCH-40),
epic QSL-34 (#205), Layer 4 architecture and conformance mapping. It builds
on ADR-011 to ADR-015 and reopens none of their cells. It owns no feature
implementation. The implementation owners are QSL-68 (#120, V1-A04, model
graph and binding), QSL-67 (#121, V1-A05, native execution and finite
simulation) and QSL-57 (#164, the static type/model conformance boundary,
Done). Its `/spec-review` is SR-786 to SR-789 in
[`spec/reviews/state-model-mapping/`](../reviews/state-model-mapping/integrity.md).

ADR-012 §15 (QSL-273) is the state-clause share of this mapping: families,
stages, identity, wire, observations, outcomes, requirements and deletion
order for `invariant`, `pre` and `post`. This record does not restate it. It
adds the model graph, the static/runtime boundary, identity across all four
phases (check, execute, proof handoff, replay), finite exploration, and the
owner and oracle of every remaining piece.

Item ids `SC-`, `ID-`, `EX-`, `FE-`, `ND-`, `FP-`, `G-`, `OR-` and `PI-` are
local to this record. Other artifacts cite them as `ADR-016 G-3`.

## Context

Measured on QSL `main` at `99e9b6c7`.

**Implemented and on the spine.**

- Model intake and normalization (I1, S3 `model`). `model::intake::admit`
  (`qsl-semantics/src/model/intake.rs:564`) and `admit_unit`
  (`model/intake/unit.rs:98`) admit a domain package, and `normalize`
  (`model/normalize.rs:2780`) derives the effective view. `EffectiveId` is
  minted only by `EffectiveDeclarationPreimage::identity_and_canonical_len`
  (`model/key.rs:432-434`). Conformance, subsetting and redefinition are in
  `model/conformance.rs` (FR-082). Dispatch is linked at check time by
  `model::dispatch::link_dispatch` (`model/dispatch.rs:229`, FR-083). Systems
  classification is in `model/systems.rs` (FR-086).
- The model correspondence. `check` records `DeclarationKey` → `NodeKey` while
  lowering (`check/lowering/model.rs:242-265`), stores it as
  `ModelCorrespondence` (`check/identity.rs:257`) in `CheckedGraph`
  (`check/mod.rs:334`), and resolves it through
  `CheckedGraph::resolve_declaration` (`check/mod.rs:1656`).
- Static model typing at S3. Field reads resolve through
  `TypeEnvironment::attribute` (`value/declaration.rs:884`, from
  `check/check.rs:1659`). `allInstances` is typed `Set<T>[0,N]` from
  `ValueType::Population(N)`, "no runtime value is consulted at check time"
  (`check/check.rs:2164`). `lookup` checks `S` against `T` with
  `TypeEnvironment::conforms` (`check/check.rs:2214`,
  `value/declaration.rs:966`). QSL-57's three items are delivered: an inherited
  field read through `deref` (`qsl-eval/tests/it/inherited_attributes.rs:152`),
  upcast reference equality (TC-198 L08,
  `qsl-eval/tests/it/equality_matrix.rs:1028`), and the check-time `lookup`
  conformance refusal.
- Population data at S6a. `PopulationBinding` (`model/population.rs:551`),
  `admit_binding` (`:806`), `admit_invocation` (`:1259`), `all_instances`
  (`:1718`) and `lookup` (`:1888`). `mint_population_id` (`:749`) mints the
  only `PopulationId`, over `{version, domain_package_selection,
  population_key, admission_role}` (`:714`). `ObjectEnvironment`
  (`model/object_environment.rs:88`) records the `PopulationId` →
  `PopulationBinding` correspondence and refuses a second, unequal record
  (`:156`). The evaluator's `AllInstances` and `Lookup` arms
  (`qsl-eval/src/value/expression/evaluate.rs:1377`, `:1396`) call
  `value::model_query` (`qsl-semantics/src/value/model_query.rs:183`, `:211`).
- State clauses (ADR-012 §15, FR-102 to FR-109). Forms
  (`qsl-forms/src/protocol_clause.rs:61`), the checker
  `check::state_clause` with `ProtocolClauseFamily`
  (`check/state_clause.rs:275`, `:331`), observation-keyed facts
  (`check/observation.rs:27`, `check/facts.rs:36`), lowering to
  `state`/`state_clause`, `operation_anchor` and `frame`
  (`check/lowering/state.rs:154`, `:283`, `:318`), the S6a arm
  (`qsl-eval/src/value/expression/s6a/protocol_clause.rs:94`), and the run
  entry `qsl_replay::spine::run_clause` (`qsl-replay/src/spine/clause.rs:552`).
- Frames (FR-114 to FR-116). `admit_frame_invocation`
  (`model/observation.rs:1245`), `CheckedPackageEvaluation::evaluate_frame`
  (`qsl-eval/src/value/expression/mod.rs:408`) and `replay_frame`
  (`qsl-replay/src/execute/frame.rs:167`) over `FrameCounterexample`
  (`qsl-replay/src/witness/frame.rs:136`).
- The finite exploration engine (FR-101 AC-1 to AC-11). `explore_request`
  (`qsl-eval/src/simulation/explore.rs:269`), `sample_request`
  (`simulation/sample.rs:270`), `replay` (`simulation/trace.rs:117`),
  `Outcome{Exhaustive, Bounded, Cancelled}` with `Outcome::category()`
  (`explore.rs:75`, `:111`), `Frontier` of `quire.simulation.state-key/v1`
  digests (`frontier.rs:10`, `key.rs:63-71`), and `SampleProvenance{seed,
  trace, sampler, stopped}` (`trace.rs:35`).

**Not implemented.**

- FR-101 AC-12 to AC-14. `TransitionSystem::successors` returns
  `Vec<(TransitionId, State)>` (`explore.rs:46`), not `Expansion` or
  `ExpansionStop`. There are no findings and no `Outcome::Stopped`.
- FR-120. `ModelSystem`, `qsl_semantics::model::state` and
  `explore_model`/`sample_model` do not exist. The only `TransitionSystem`
  implementers are four test fixtures in
  `qsl-eval/tests/it/finite_simulation.rs` (`:106`, `:148`, `:189`, `:216`).
  Linear QSL-274 is Done because it was the specification ticket. The code is
  not written.
- A `StateModel` family implementation. `FamilyKind::StateModel`
  (`qsl-semantics/src/family/mod.rs:90`) has no `FamilyContract`
  implementer, and `S6aFamilyKind` has exactly `Value` and `ProtocolClause`
  (`qsl-eval/src/value/expression/s6a/mod.rs:98-104`).
- Population reads inside a state clause. The `AllInstances` and `Lookup`
  arms refuse under `ProtocolClause` evaluation with
  `ProtocolClauseUnsupported`, `unknown_required_feature`/
  `unsupported-feature` (`evaluate.rs:1378`, `:1401`; `causes.rs:140-170`).
  No admitted clause reaches them (FE-4).
- A relationship-end resolver (FR-085). Only the data type
  (`model/domain_package.rs:393`) and its intake reader (`intake.rs:2089`)
  exist.
- The native state lane still exists: `src/state`, `src/native_model`,
  `src/protocol_artifact`, and the FR-108 parity test
  `tc_469_step_2_and_3_native_and_spine_agree_and_name_the_boundary_locus`
  (`tests/it/config_version_spine.rs:324`).

**Pinned blockers.** FR-105-AC-3 and FR-108-AC-6's I04 `read` half wait on
IR-370: the pinned `quire-contract-model` (`quire-contract-ir` `2a28643`)
refuses every `quire.op.model.reaches_field` application. IR `lower` returns
no form for a `state` node (ADR-012 §15.7). Gate QSL-20 (#219, ARCH-G3) is
Backlog.

## Decision

### 1. Static conformance versus runtime population data

One rule: **the checked package is a function of source, domain packages and
dependencies only. No population, snapshot, invocation or universe enters
it** (ADR-011 E6, ADR-012 §15.4 "Not emitted"). S3 decides every question
about static types. S6a decides every question about population data.
Neither decides the other's questions.

| ID | Question | Decided at | Owner, function | Refusal or result |
| --- | --- | --- | --- | --- |
| SC-1 | Does a field, operation or relationship end exist on a static object type, including inherited members? | S3 | `model` effective view through `TypeEnvironment::attribute`/`operation` | `missing_declaration`; ambiguous operation `ambiguous_declaration` |
| SC-2 | Does static type `S` conform to `T` (upcast equality, `lookup` key type, argument passing, clause context type)? | S3 | `TypeEnvironment::conforms`, total over a closure computed once under the check stage's `ancestor_steps` (B-3, ADR-014 §1) | `ill_typed`/`type-mismatch`; a deep chain refuses as `stage_limit_exceeded` |
| SC-3 | Which operation a dispatched call selects (unique most-specific) | S3, link time | `link_dispatch`, `check::checked_dispatch_operation` | ambiguity or ineligibility refuses at S3 with no partial table (FR-083) |
| SC-4 | The declared maximum of a population and the element type of `allInstances` | S3 | `ValueType::Population(max)` from the domain package | none; a type fact |
| SC-5 | Which objects exist, their most-specific type, field values, closure and dangling references | S6a admission | `admit_binding`, `admit_invocation`, FR-106 `admit_observations` | `dangling_reference`, `incomplete_population` (incomplete), `invalid_runtime_input` |
| SC-6 | Whether a runtime object's dynamic type conforms to `T` (`lookup` narrowing) | S6a | `model::population::lookup` through the metered `ModelIndex::conforms` walk | `lookup … absent undefined` → `FamilyResult::Undefined` `absent-key`; `absent refused` → `ModelRefusal`; a walk past the caller's `ancestor_steps` → `resource_exhausted` (B-2) |
| SC-7 | Whether a dispatched call's precondition holds for its receiver | S6a | `StateModel` undefined cause `PreconditionFalse` (ADR-013 O-16) | `FamilyResult::Undefined` `precondition-false` |
| SC-8 | Whether a change stays inside an operation's frame | S6a | `population::decide_frame` (§4) | `frame_violation`, `population_delta_mismatch` |

**Agreement rule.** SC-2 and SC-6 answer different questions (static versus
dynamic type) over the same generalization edges. When both complete, they
agree on every pair of static types: for static types `A` and `B`,
`TypeEnvironment::conforms(A, B)` equals the runtime walk over the same
edges. The runtime walk may stop early only on its B-2 meter
(`resource_exhausted`), never with a different truth value. So "check accepts
what evaluation refuses" is permitted only as a resource outcome. The
disagreeing cases QSL-57 measured (an unknown supertype, a cycle) cannot
reach S6a, because S3 refuses the type environment first and no
`CheckedPackage` exists (ADR-011 §2.3 E3). Oracle: OR-2.

**No population data at check time.** A universe, a snapshot or an initial
state is an S6a or simulation input. The simulator's `domains()` come from
the checked types plus the request's universes (FR-120 "Bounds"). They never
feed back into S3.

### 2. Identity and provenance across check, execute, proof handoff and replay

Each row names the one minter and what every later phase does with the value.
"Carried" means passed unchanged and never re-derived (ADR-011 §2.2).

| ID | Identity | Minted by (phase) | Execute (S6a, simulation) | Proof handoff (E5 to E8) | Replay (E9) | Equality |
| --- | --- | --- | --- | --- | --- | --- |
| ID-1 | Original declaration: `DeclarationKey{package, node}` | the domain package; admitted at I1 (ADR-013 O-03) | Carried in `StateObject.type_identity`, the `semantic` map keys and FR-120's declaration identity text | Reached from a `NodeKey` only through the model correspondence (ID-3); not on the wire as such | Re-admitted from the QC-1 byte provision; must be equal | declared |
| ID-2 | Effective declaration: `EffectiveId` | `model` normalization only (ADR-013 O-05) | The type component of every `ObjectReference`; FR-120 encodes it as `object_type` | Absent: no v2 member carries it (ADR-013 O-05) | Recomputed by re-normalization; a reference whose type is not an `EffectiveId` of the bound universe refuses at value admission | normalized |
| ID-3 | Checked node of a model declaration: `NodeKey` over `ModelOwner` | `check` only (O-04, C-02), recorded in `ModelCorrespondence` | S6a selects clauses, anchors and frames by `NodeKey` | Written as `WireNodeId` in v2 | `WireNodeId` → `NodeKey` by lookup in the recompiled package (O-04) | normalized |
| ID-4 | Reference: `ObjectReference{UniverseId, EffectiveId, ObjectId}` | `model` at admission (O-05 reference components) | Keys `ObjectEnvironment`; FR-181 triple in state keys | Only as a witness value decoded by `Witness::decode` (C-11) | Admitted again from the replay input; the universe must equal the recomputed one | declared over the triple |
| ID-5 | Population: `PopulationId` | `model` at admission (O-13, QC-21) | Resolves its `PopulationBinding` through `ObjectEnvironment` | Absent | Re-minted by re-admission | normalized |
| ID-6 | Clause, anchor and frame: the node ids of `state`/`state_clause`, `operation_anchor` and `frame` | `check` (ADR-012 §15.4) | Selected by the name → (node id, occurrence) table in the in-process `CheckedPackage` (FR-109) | Clause node id and occurrence key enter the obligation identity (O-09) | Resolved in the recompiled package; a frame whose recomputed identity differs refuses `FrameIdentityMismatch` (FR-116) | normalized |
| ID-7 | Observation: snapshot or invocation `DocumentRef` with `sha256-jcs` digest | the document's producer; admitted by FR-106 | Anchors each clause evaluation; carried in the run report | Not on the v2 wire (ADR-012 §15.4) | Supplied by digest; bytes must match | normalized (digest) |
| ID-8 | Simulated state: FR-181 state key and its `quire.simulation.state-key/v1` digest | `qsl_eval::simulation` from the `TransitionSystem` key (FR-101) | Coalescing compares full key bytes; traces and frontiers hold digests | Absent | FR-101 `replay` recomputes it; never E9 | lexical over key bytes |
| ID-9 | Transition: FR-181 `{"type":"transition","operation":…,"arguments":…}` | the `TransitionSystem` (FR-120: operation = declaration identity text of ID-1) | Orders successors (FR-101) | Absent | FR-101 `replay` | lexical over JCS bytes |
| ID-10 | Synthesized state document: `DocumentRef{quire, quire.simulation.state/<digest>, quire.simulation.state-key/v1, 1}` | FR-120 `StateModel::observation` | Lets a clause evaluation name a simulated state as an ID-7 observation | Absent | Only through ID-8 | normalized (its digest is the ID-8 digest) |
| ID-11 | Counterexample: `WitnessEnvelope<P>` | CG from a backend run (O-25) | not applicable | Carries obligation identity, occurrence key, `package_id`, `run_limits`, declared domains, `trace_position` | The only E9 input (FR-098) | lexical over the admitted transcript |

**Original and effective provenance.** The original `DeclarationKey` is the
only declaration identity that crosses a phase boundary. It enters node
identity (ID-3), state keys and transition identities (ID-8, ID-9). The
`EffectiveId` appears only inside values (ID-4) and is recomputed from the
original at every admission. No `EffectiveId` becomes a `NodeKey`, and none
is carried on the wire or in a replay request (ADR-013 O-05 "Conversions").
A derived effective member is always reported with the original it derives
from, through the FR-081 derivation trail.

**ID-5 decision: population identity binds its content.** FR-089's open
question (Linear QSL-131) is settled here. The `PopulationId` preimage gains
one member, `members`: the RFC 8785 JCS digest, taken through
`quire-canonical`, of the admitted binding's member set. That set is the
`(ReferenceKey, DeclarationKey)` pairs in ascending `ReferenceKey` order. So
two admissions that agree on package, key, role and members share one
`PopulationId`, and two that differ in any of them never collide. The
"declared maximum" half of the question needs no member. The maximum is read
from the declaration that `(domain_package_selection, population_key)`
already names, so it cannot differ between two admissions that agree on
those. After this change, `ObjectEnvironment::with_population`'s refusal of
an unequal second record can be reached only through a broken invariant. It
becomes an `InternalFault`, not a guard. Reason: identity follows content
(ADR-013 §2 normalized kind), and no production path admits two bindings into
one evaluation today (`admit_binding` is called only from tests, and
`admit_invocation` from tests and `qsl-bench`), so the change has
no migration. Implementation: G-3.

**Two keyings of a population domain, kept apart.** A clause's requirement
record keys a population domain as `DomainKey{node: object_type node, path:
[population ordinal]}` (ADR-012 §15.7). That key crosses to `route`, CG and
FR-331. FR-120 names a simulation root by `WireNodeId::from_digest` over the
population's `$defs.DeclarationKey` preimage, and FR-101's `RequiresBound`
reports it. That key is local to one `ModelSimulationRequest`. It never
crosses to `route`, CG, FR-331 or replay, and no conversion between the two
exists. A caller answers a simulation `RequiresBound` with universes, not
with `ProofBound`s.

### 3. Population, state keys, seeds, frontiers, trace positions, cancellation, exhaustion and incomplete outcomes

ADR-014 TR-1, TR-6 and TR-7 and FR-101 define the engine's types. This table
fixes how the state family uses them.

| ID | Concept | Decision |
| --- | --- | --- |
| EX-1 | Model population in a simulated state | A complete, closed population state per universe population (FR-120). A universe is caller-chosen: in kind it is an ADR-014 B-4 proof bound (a finite domain substituted for a `Population(None)` domain), carried as FR-120's `PopulationUniverse`, not converted into `ProofBound`. Every exploration result is evidence only for the request that holds it (EX-9). |
| EX-2 | State key | FR-120's typed canonical form over FR-101's encoder. The anchor is not in the key. |
| EX-3 | Seed | `SampleProvenance.seed` (TR-6). Exploration has no seed. The sampler is `quire.simulation.sampler/v1` `1-draft.1` only; any other refuses `GeneratorMismatch`. |
| EX-4 | Frontier | FR-101 `Frontier`, in next-expansion order, on `Bounded`, `Cancelled` and (after G-1) `Stopped`. |
| EX-5 | Trace positions | Three distinct concepts, never converted: (a) FR-101's step index inside a simulation `Trace`; (b) ADR-014 TR-2 `TemporalPosition`, carried as `qsl_replay::TracePosition`, owned by `TemporalTrace`; (c) the FR-351 separating-witness trace position. A state-family `WitnessEnvelope` carries `trace_position: None`. A simulation step index never becomes a `TracePosition`. |
| EX-6 | Cancellation | Only exploration is cancellable: the `poll` callback gives `Cancelled{cause: cancelled/caller-cancelled}`. S6a clause evaluation and admission are not cancellable (ADR-014 §7). Sampling is bounded by `max_steps` and is not cancellable. |
| EX-7 | Exhaustion | Reaching an exploration `Limits` field gives `Bounded{limit}`. A clause meter that runs out inside an expansion is not exhaustion of the exploration. It gives an `InvariantUndetermined` finding (invariant) or an `ExpansionStop` with `resource_exhausted`/`insufficient-next-charge` (contract clause, candidate cap), as FR-120 states. |
| EX-8 | Incomplete outcomes | `Bounded`, `Cancelled` and `Stopped` with `resource_exhausted` are O-16 incomplete. `Stopped` with `runtime_invariant` is internal failure. `NotSimulated::RequiresBound` is a disposition, not an outcome. FR-106 `incomplete_population` is incomplete at admission. `Exhaustive` is success of the exploration only (§6). |
| EX-9 | Evidence scope | The identity of an exploration result is its whole request: the `package_id`, the domain-package digests, the initial `DocumentRef`s, the universes, `Limits`, `ExpansionLimits` and the meter budget. Two results compare only when these are equal. |

### 4. Frame effects and model-bound evaluation through the Layer 3 clause/frame path

| ID | Decision |
| --- | --- |
| FE-1 | **One frame decision.** `model::population::decide_frame` is the only function that decides whether a change stays inside a frame. `admit_invocation`, FR-106 `observation::frame::enforce`, FR-115 `evaluate_frame` and FR-120 `StateModel::check_frame` all call it. No second frame implementation is added. |
| FE-2 | **One frame source.** An operation's frame is its one `OperationEffect` admitted at I1 (FR-103) and bound in the checked package (FR-114). A request, snapshot or universe never supplies a frame (QSpec FR-013-AC-3). |
| FE-3 | **One clause evaluator.** A state clause is evaluated only through `CheckedPackageEvaluation::evaluate_clause` and the `ProtocolClause` S6a arm (FR-107), with a fresh `Meter` per evaluation. `run_clause`, `ModelSystem` and replay all use it. |
| FE-4 | **Model reads inside clause bodies.** Field reads, `deref` and `reaches` evaluate over the clause's observation (ADR-012 §15.5). `allInstances<T>(p)` and `lookup<T>(p, r)` take a `Population<T>[N]` value, and no state-clause source can produce one: a clause's `self`, `result` and parameters draw from `ValueTypeRef`, which has no `Population` variant, and the grammar has no population-valued expression. The S6a refusal `ProtocolClauseUnsupported` (`unknown_required_feature`/`unsupported-feature`, `evaluate.rs:1378`, `:1401`) is therefore a defensive guard that no admitted clause reaches (TC-467). This is not a gap: an invariant already ranges over its context population (ADR-012 §15.7). Reopen condition: QSpec admits a population-valued expression in a state clause. That is new model behavior, a ticket non-goal here. |
| FE-5 | **Effects as data.** A simulator or evaluator performs no effect. Effects are `StepEffect` data (FR-120 "Effects as data"). Forms that read ambient state refuse at S3 (FR-120-AC-12). |

### 5. Authored nondeterminism versus reproducible exploration bookkeeping

| ID | Kind | Source | Rule |
| --- | --- | --- | --- |
| ND-1 | Authored | Operation choice, argument values from finite parameter domains, frame post-states (`modifies`, `creates`, `deletes`) and operation results | Part of the successor relation. Exploration enumerates every choice; nothing prunes one except a false effective pre- or postcondition. |
| ND-2 | Bookkeeping | FR-101 canonical order (transition bytes, then post-state key bytes), FIFO parent order, initial-state order | Fixed by the engine. It never changes which states are reachable, only the order they are visited and the state recorded first for an anchor (FR-120 "State, state key and anchor"). |
| ND-3 | Bookkeeping | Seeded sampling | Selects one path among the ND-1 choices. The same seed, trace index and `DefinitionRef` give the same trace. A sampled trace is never a claim about the unsampled choices. |
| ND-4 | Forbidden | Hash-map iteration, registration order, wall clock, ambient randomness | Never a source of order or choice (ADR-013 R-05). |

### 6. Finite exhaustion is not proof

`Outcome::Exhaustive` states that every state reachable under the request's
universes, initial states and parameter domains was expanded. It is not a
proof of any clause for an unbounded population, for other universes or for
other initial states. It is never `proved`. Its findings are evidence for the
states they name. A clean `Exhaustive` run means "no violation found within
this request" (EX-9). Three consumers enforce this:

- the O-16 map: `Exhaustive` is exploration success, and no clause outcome
  derived from it is promoted beyond `tested` (ADR-013 O-16 success row);
- the requires-bound pre-check: an unbounded domain in `domains()` returns
  `NotSimulated::RequiresBound` before any state is expanded, and no `Limits`
  value answers it (FR-101-AC-8);
- replay: a simulation trace replays only through FR-101 `replay`, not E9. A
  finding carried as a counterexample envelope is `ReplaySource::Input`, and
  its agreement settles `reproduced-without-witness`, never backend evidence
  (ADR-013 O-25).

Proof of a state clause goes only through negotiation. Each clause and frame
records one `operation-contract` requirement (ADR-012 §15.7). While IR
admits no `state` node, no backend proves one, and no state-clause proof
evidence is claimed toward gate QSL-20 (#219).

### 7. Family placement of the model

| ID | Decision |
| --- | --- |
| FP-1 | `StateModel` owns model declarations, population extent, redefinition, dispatch and the model expression forms (`deref`, `allInstances`, `lookup`, dispatched calls, `reaches` inside a clause), as ADR-012 §3 and §15.2 state. Its checked outputs are `check`-core types (ADR-012 §1): `NodeKind::Attribute`, `AllInstances`, `Lookup`, `Reaches` and the dispatch table. |
| FP-2 | Its `check` hook is the S3 work for those forms. Today that work sits inside the `Value` typer's `infer_form` arms (`check/check.rs:1659`, `:2164`, `:2214`). Moving it behind a `StateModel` `FamilyContract` implementation with thin seam arms (ADR-012 §4.3) is G-2. |
| FP-3 | No `StateModel` variant is added to `S6aFamilyKind`. No S6a input is a `StateModel` item: a model form is always nested in a `Value` function body or a `ProtocolClause` body, and it evaluates inside that family's `evaluate` hook through the `value::model_query` functions, raising `StateModel`-owned causes (ADR-013 O-16). This applies ADR-012 §2's #214 rule: a contract part nothing can construct is left out. Reopen condition: an S6a entry whose selected item is a `StateModel` declaration. |
| FP-4 | Simulation is a layer-5 consumer of checked packages (ADR-011 §6.1 `simulation`), not a family. `ModelSystem` is its only production `TransitionSystem`. With it, lane D has converged (ADR-011 §8), and `qsl_eval::simulation::Outcome` is a canonical S6a-layer outcome whose O-16 map is `Outcome::category()` (ADR-014 §7). |

### 8. Conformance summary

The implemented state/model behavior conforms to ADR-011 to ADR-015 on stage
placement (S3 typing, S6a population data, layer-5 simulation, layer-6 run
and replay), identity minting (one minter each for `NodeKey`, `EffectiveId`
and `PopulationId`), typestate (every run starts from a `CheckedPackage`
compiled from source), outcomes (every result maps to one O-16 category),
bounds (the requires-bound pre-check, B-2 versus B-3) and replay (frames
through `replay_frame` over FR-098). It does not yet conform in the seven
places G-1 to G-7 name. None of them needs a new architectural decision.

### 9. Responsibilities returned to the implementation tickets

Each gap has one owner, a bounded acceptance criterion and a test. None is
completed by this record. QSL-57 (#164) is Done and its three items are
delivered, so the static-conformance leftovers go to QSL-68 (#120).

| ID | Gap | Owner | Interface or conversion | Bounded acceptance criterion | Test |
| --- | --- | --- | --- | --- | --- |
| G-1 | FR-101 findings, `Expansion`, `ExpansionStop`, `Outcome::Stopped`, `StopReason::Stopped`, replay `FindingMismatch`/`Stopped` | QSL-67 (#121) | `TransitionSystem::successors` → `Result<Expansion<…>, ExpansionStop>`; `explore_request` → `Exploration<F>` | FR-101-AC-12 to AC-14 | TC-474 |
| G-2 | `StateModel` check hook: a `FamilyContract` implementation for the model expression forms, with thin `infer_form` arms | QSL-68 (#120) | `check::state_model` family module; ADR-012 S1 to S3 seams gain the `StateModel` arms | The model forms are typed only through `StateModel`'s `check`; the `Value` typer's arms each make one call (ADR-012 §4.3); seam probe lists them | FR-063 seam probe; the existing FR-082 to FR-084 TCs unchanged |
| G-3 | ID-5 population identity binds its member set | QSL-68 (#120) | `population_id_preimage` gains `members`; `with_population`'s unequal re-record becomes an `InternalFault` | Two admissions differing only in members get different ids; equal content gets equal ids; FR-089 amended to match | FR-089 TC-291 extended |
| G-4 | FR-120 `ModelSystem`, `qsl_semantics::model::state`, `explore_model`, `sample_model` | QSL-67 (#121) | as FR-120 "Outputs" and "Frame and observation seam"; `check_frame` calls `decide_frame` (FE-1) | FR-120-AC-1 to AC-12 | TC-471 to TC-473 |
| G-5 | FR-085 relationship-end resolution | QSL-68 (#120) | a `model` resolver over `RelationshipEnd`, producing the `relationship_end` member (ADR-013 O-06) | FR-085's ACs | FR-085's TCs |
| G-6 | M-6c state lane deletion | QSL-67 (#121) with QSL-5 | none | ADR-012 §15.8 step 2: `src/state`, native `run`, `runtime`, `mapped`, `model_source`, `NativePackage`, the native-linked-package/1 reader and the FR-108 parity test are deleted in one PR; FR-108's independent table stays | TC-469 step 1 stays green; the parity test is gone |
| G-7 | M-6e composed deletion and `native_model` | QSL-68 (#120) and QSL-67 (#121), each in the PR that lands its S3 checker | none | ADR-012 §15.8 step 3: `native_model` goes in the PR that removes its last importer (SEAM-2 `checking/composed/solver.rs`, SEAM-3 `protocol_artifact/mod.rs`) | build with the module removed |

Migration: none (ADR-011 Decision 9). Each replaced path is deleted in the
PR that lands its successor, and nothing runs side by side except FR-108's
parity test, whose end is fixed by G-6.

The QSL-67 and QSL-68 ticket bodies still say "extend `runtime::execute`" and
name the native lane. This record and ADR-011 govern: the work lands on the
spine, and `runtime::execute` is deleted in M-6c.

### 10. Owners and oracles for the named behaviors

| ID | Behavior | Owner | Oracle |
| --- | --- | --- | --- |
| OR-1 | Ambiguous dispatch | S3 `link_dispatch`, QSL-68 | FR-083 tests `qsl-semantics/tests/it/model_dispatch.rs` (TC-222 to TC-225): ambiguity refuses with no partial table |
| OR-2 | Static conformance (SC-2) and its runtime agreement | S3 `TypeEnvironment::conforms`; S6a `lookup`; QSL-68 | FR-082 tests (`model_conformance.rs`, `type_environment_model.rs`); TC-198 L08 (`equality_matrix.rs:1028`); a new agreement test in G-2 over every pair of an FR-082 fixture's types |
| OR-3 | Closure (dangling references, complete populations) | S6a admission, QSL-68 and QSL-67 | FR-084 tests (`model_population.rs`); FR-106 `dangling_reference` and `incomplete_population` rows (ADR-012 §15.6) |
| OR-4 | Cancellation | `qsl_eval::simulation`, QSL-67 | FR-101-AC-6, TC-455 `cancellation_stops_the_run_and_returns_the_frontier` (`finite_simulation.rs:728`) |
| OR-5 | Exhaustion | `qsl_eval::simulation`, QSL-67 | FR-101-AC-7, TC-455 limit tests (`:773`, `:826`, `:880`); TC-439 category map (`:1113`) |
| OR-6 | Unsupported | S6a `ProtocolClauseUnsupported` (FE-4 guard); CG negotiation for proof | TC-467 `all_instances_under_a_protocol_clause_refuses_unsupported_construct` (`evaluate.rs:2169`); FR-057 backend-absence case for `operation-contract` |
| OR-7 | Incomplete | S6a meter (`Incomplete`), exploration (`Bounded`, `Cancelled`, `Stopped`), admission (`incomplete_population`) | FR-100 outcome table; FR-101-AC-12 (G-1); FR-120-AC-6 (G-4) |
| OR-8 | Requires-bound before exploration | `explore_request`, `sample_request` | FR-101-AC-8, TC-455 `requires_bound_refuses_before_any_transition_system_call` (`:975`) |

### 11. Pinned and current-head integration

| ID | Requirement |
| --- | --- |
| PI-1 | QSL pins `quire-contract-ir` at `2a28643` in `Cargo.lock`. FR-105-AC-3 and FR-108-AC-6's I04 `read` half stay pending until IR-370 lands and the pin moves. No state-node emission test is ignored or weakened in the meantime. |
| PI-2 | FCD `agent-ix-extraction-frontend` and `agent-ix-semantic-ir` are consumed at their exact git revisions through `model::intake` only (ADR-011 §7.1). A change to intake shapes is a filament-core-data ticket. |
| PI-3 | QSpec contracts selected by this family: diagnostics catalog `1-draft.8`, sampler `quire.simulation.sampler/v1` `1-draft.1`, FR-181's typed canonical form, STD-111's state node rules. Each is selected by exact revision and refused otherwise (ADR-013 R-08). |
| PI-4 | The current-head lane (#215, QI `heads/`) is a drift check. A green heads run precedes each pin bump that G-1 to G-7 need. It never substitutes for a pin (ADR-013 O-23). |
| PI-5 | No state-clause proof evidence is claimed until IR `lower` admits `state` nodes and CG has an `operation-contract` arm for them. Gate QSL-20 (#219) counts none from this family before then. |

### 12. Evidence from implemented behavior

These run today and exercise the shared foundation. They validate the
mechanism, not the unfinished A04 and A05 features.

- **ConfigVersion through the spine** (FR-108, TC-469,
  `tests/it/config_version_spine.rs:181`): source → S1 to S4 with I1 → FR-106
  admission → FR-107 S6a → `run_clause`, matching an independent expected
  table. It exercises ID-1, ID-3, ID-6, ID-7, FE-3 and SC-5.
- **Frame run and frame replay** (FR-115 TC-514, FR-116 TC-515): FE-1, FE-2,
  and ID-6 through E9 recompilation.
- **Finite exploration engine** (FR-101 TC-453 to TC-455, over test systems):
  EX-2 to EX-8 and ND-2, ND-3. It validates the engine only, since no
  production `TransitionSystem` exists (G-4).
- **Population identity** (FR-089 TC-291 to TC-297): ID-5 as it stands
  before G-3.
- **Static conformance** (QSL-57's three items, SC-1, SC-2).

## Consequences

- QSL-67 and QSL-68 can close their state, model and simulation work against
  G-1 to G-7 without another ownership, identity or outcome question.
- The checked package stays independent of every population, snapshot and
  universe. The same package is reproducible whatever is simulated.
- Finite exploration never reads as proof. `Exhaustive` is scoped by EX-9,
  and state-clause proof waits for IR and CG.
- `PopulationId` becomes content-bound, and `ObjectEnvironment`'s guard
  becomes an invariant.
- Lane D converges when G-4 lands. ADR-013 §6's "simulation `Outcome`" row
  names the old lane-D type, not `qsl_eval::simulation::Outcome` (see
  Amendments made with this record).

## Amendments made with this record

- ADR-012 Status: §15 is the state-clause share; the rest of the #220
  mapping is this record.
- ADR-013 §6 Outcomes row: "simulation `Outcome`" is the lane-D type before
  convergence; `qsl_eval::simulation::Outcome` is canonical, with its O-16
  map `Outcome::category()` (ADR-014 §7, FP-4).
- FR-089 Status: the admission-preimage question is decided by ID-5 and
  implemented under G-3.
- `spec/spec.md`: index row.

## Open dependencies

1. IR-370 (PI-1): the pinned IR reader refuses `reaches_field`.
2. IR admission of `state` nodes and a CG `operation-contract` arm (PI-5).
   Neither blocks G-1 to G-7.

## Alternatives Considered

- **Give `StateModel` its own S6a family kind.** Rejected (FP-3). No S6a
  input selects a `StateModel` item, so the variant would have no producer.
- **Key a `PopulationId` by document bytes.** Rejected. The same member set
  in two documents with different layout would get two identities. The
  member set is the content that matters.
- **Convert simulation universes into `ProofBound`s.** Rejected. They key
  domains by simulation roots that never cross to CG (§2), and ADR-014 §1
  forbids converting between bound values. Their kind is recorded (EX-1)
  and their scope enforced (EX-9).
- **Treat a clean `Exhaustive` run as a proof over the universe.** Rejected
  (§6). A universe is a caller choice, and QSpec FR-181 and ADR-014 §8 make
  finite results evidence for what was evaluated only.
- **Admit population data into the checked package so the checker can
  decide closure.** Rejected (§1). The package's identity would depend on
  its observations.

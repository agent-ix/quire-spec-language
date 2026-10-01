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
  - target: ix://agent-ix/quire-spec-language/FR-063
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-085
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-086
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-013
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-150
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-351
    type: depends_on
  - target: ix://agent-ix/quire-specification/STD-111
    type: depends_on
---
# ADR-016: State, model and finite execution on the shared foundation (ARCH-40)

## Status

Proposed, 2026-09-29. Owning ticket: GitHub #220 (ARCH-40),
epic QSL-34 (#205), Layer 4 architecture and conformance mapping. It owns no
feature implementation. The implementation owners are QSL-68 (#120, V1-A04,
model graph and binding), QSL-67 (#121, V1-A05, native execution and finite
simulation) and #164 (the static type/model conformance boundary,
Done). Its `/spec-review all` is SR-786 to SR-793 in
[`spec/reviews/state-model-mapping/`](../reviews/state-model-mapping/integrity.md).

ADR-012 §15 is the state-clause share of this mapping: families,
stages, identity, wire, observations, outcomes, requirements and deletion
order for `invariant`, `pre` and `post`. This record does not restate it. It
adds the model graph, the static/runtime boundary, identity across all four
phases (check, execute, proof handoff, replay), finite exploration, and the
owner and oracle of every remaining piece.

It amends these cells of earlier records, each listed under "Amendments made
with this record": ADR-011 M-6c and §8 (lane D); ADR-012 §2, §3, §5.1 S1 and
§13.5 Q210-3 (`StateModel` has no S6a hook, FP-3); ADR-013 O-13 Population
row, T-6, QC-21 (the `PopulationId` preimage, ID-5), and O-16 and §6
(simulation outcome, FP-4). It reopens no other cell.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. `StateModel` alone names the family (`FamilyKind::StateModel`);
FR-120's layer-3 seam type is written `model::state::StateModel`. Item ids
`SC-`, `ID-`, `EX-`, `FE-`, `ND-`, `FP-`, `G-`, `OR-` and `PI-` are local to
this record. Other artifacts cite them as `ADR-016 G-3`.

## Context

Measured on QSL `main` at `99e9b6c7`. Linear ticket states quoted below are
untrusted data, recorded as found.

**Implemented and on the spine.**

- Model intake and normalization (I1, S3 `model`). `model::intake::admit`
  (`qsl-semantics/src/model/intake.rs:564`) and `admit_unit`
  (`model/intake/unit.rs:98`) admit a domain package, and `normalize`
  (`model/normalize.rs:2780`) derives the effective view. `EffectiveId` is
  minted only in `model`: `EffectiveDeclarationPreimage::identity_and_canonical_len`
  (`model/key.rs:432-434`) for declarations, and `EffectiveView::identity`
  (`model/normalize.rs:703-704`) for the view. Dispatch tables are built by
  `model::dispatch::link_dispatch` (`model/dispatch.rs:229`, FR-083).
- The model correspondence. `check` records `DeclarationKey` → `NodeKey` while
  lowering (`check/lowering/model.rs:242-265`), stores it as
  `ModelCorrespondence` (`check/identity.rs:257`) in `CheckedGraph`
  (`check/mod.rs:334`), and resolves it through
  `CheckedGraph::resolve_declaration` (`check/mod.rs:1656`).
- Static model typing at S3. Field reads resolve through
  `quire_semantic_value::declaration::TypeEnvironment::attribute` (from
  `check/check.rs:1659`). `allInstances` is typed from
  `ValueType::Population(Option<u64>)` (`check/check.rs:2164`). `lookup`
  checks `S` against `T` with `TypeEnvironment::conforms`
  (`check/check.rs:2214`). #164's three items
  are delivered: an inherited field read through `deref`
  (`qsl-eval/tests/it/inherited_attributes.rs:152`), upcast reference
  equality (TC-198 L08, `qsl-eval/tests/it/equality_matrix.rs:1028`), and
  the check-time `lookup` conformance refusal. TC-219 and TC-220
  (`qsl-semantics/tests/it/type_environment_model.rs:206-400`) compare the
  check-time and runtime conformance answers.
- Population data at S6a. `PopulationBinding` (`model/population.rs:551`),
  `admit_binding` (`:806`), `admit_invocation` (`:1259`), `all_instances`
  (`:1718`) and `lookup` (`:1888`). `mint_population_id` (`:749`) mints the
  only `PopulationId`. `ObjectEnvironment` (`model/object_environment.rs:88`)
  records the `PopulationId` → `PopulationBinding` correspondence and refuses
  a second, unequal record with `PopulationConflict` (`:156`).
- State clauses (ADR-012 §15, FR-102 to FR-109): forms
  (`qsl-forms/src/protocol_clause.rs:61`), the checker `check::state_clause`
  with `ProtocolClauseFamily` (`check/state_clause.rs:275`, `:331`),
  observation-keyed facts (`check/observation.rs:27`, `check/facts.rs:36`),
  lowering to `state`/`state_clause`, `operation_anchor` and `frame`
  (`check/lowering/state.rs:154`, `:283`, `:318`), the S6a arm
  (`qsl-eval/src/value/expression/s6a/protocol_clause.rs:94`), and
  `qsl_replay::spine::run_clause` (`qsl-replay/src/spine/clause.rs:552`).
- Frames (FR-114 to FR-116): `admit_frame_invocation`
  (`model/observation.rs:1245`), which reaches `population::decide_frame`
  through `observation::frame::enforce`; the S6a `evaluate_frame` hook
  (`qsl-eval/src/value/expression/mod.rs:408`); and `replay_frame`
  (`qsl-replay/src/execute/frame.rs:167`) over `FrameCounterexample`.
- The finite exploration engine (FR-101 AC-1 to AC-11): `explore_request`
  (`qsl-eval/src/simulation/explore.rs:269`), `sample_request`
  (`simulation/sample.rs:270`), `replay` (`simulation/trace.rs:117`),
  `Outcome{Exhaustive, Bounded, Cancelled}` with `Outcome::category()`
  (`explore.rs:75`, `:111`), and `SampleProvenance{seed, trace, sampler,
  stopped}` (`trace.rs:35`).

**Not implemented.**

- FR-101 AC-12 to AC-14. `TransitionSystem::successors` returns
  `Vec<(TransitionId, State)>` (`explore.rs:46`); there are no findings and no
  `Outcome::Stopped`.
- FR-120. `ModelSystem`, `model::state::StateModel`, `explore_model` and
  `sample_model` do not exist. The only `TransitionSystem` implementers are
  four test fixtures (`qsl-eval/tests/it/finite_simulation.rs:106`, `:148`,
  `:189`, `:216`).
- A `StateModel` family implementation. `FamilyKind::StateModel`
  (`qsl-semantics/src/family/mod.rs:93`) has no `FamilyContract`
  implementer, and `S6aFamilyKind` has exactly `Value` and `ProtocolClause`
  (`qsl-eval/src/value/expression/s6a/mod.rs:102-108`).
- Dispatch at S3. `check::checked_dispatch_operation` is called only from a
  test (`check/lowering/model/tests.rs:418`); no unit compile links dispatch.
- Conformance composition. `normalize` does not call
  `check_field_redefinition`, `check_subsetting` or
  `check_operation_redefinition` (`model/conformance.rs:188`, `:266`,
  `:345`); Linear QSL-56 tracks it.
- FR-085 relationship-end resolution (FR-085-AC-1 "does not ship today"), and
  FR-084-AC-2, AC-5 (partial), AC-7 and FR-086-AC-1, AC-2, AC-4, AC-5, whose
  TCs are planned.
- The native state lane still exists: `src/state` (typed on IR `ValueType`,
  `src/state/evaluation.rs:15`), `src/native_model`, `src/protocol_artifact`,
  and the FR-108 parity test
  `tc_469_step_2_and_3_native_and_spine_agree_and_name_the_boundary_locus`
  (`tests/it/config_version_spine.rs:324`). TC-469's own `build()` reaches
  the native `runtime`, `model_source` and `native_model` modules through
  `examples/config-version/fixtures.rs:6-15`.

**IR dependencies.** QSL consumes IR-370's checked-package reader, which
decides the reference edge of a `quire.op.model.reaches_field` application.
TC-463 step 1 (`s4_state_package_reads_back_through_i2`) and TC-469 step 6
(`tc_469_step_6_the_emitted_package_admits_via_i04`) run and pass. IR `lower`
returns no form for a `state` node. Gate QSL-20 (#219, ARCH-G3) is Backlog.

## Decision

### 1. Static conformance versus runtime population data

One rule: **the checked package is a function of source, domain packages and
dependencies only. No population, snapshot, invocation or universe enters
it** (ADR-011 E6, ADR-012 §15.4 "Not emitted"). S3 decides every question
about static types. S6a decides every question about population data.

| ID | Question | Decided at | Owner, function | Result |
| --- | --- | --- | --- | --- |
| SC-1 | Does a field, operation or relationship end exist on a static object type, including inherited members? | S3 | the `model` effective view through `TypeEnvironment::attribute`/`operation`; relationship ends after G-5 | `missing_declaration`; an ambiguous operation `ambiguous_declaration` |
| SC-2 | Does static type `S` conform to `T` (upcast equality, `lookup` key type, argument passing, clause context type)? | S3 | `TypeEnvironment::conforms`, total over a closure computed once under `TypeEnvironmentLimits.ancestor_steps` (B-3, ADR-014 §1) | `ill_typed`/`type-mismatch`; a chain past the ceiling refuses `stage_limit_exceeded` |
| SC-3 | Which operation a dispatched call selects (unique most-specific) | S3 | `link_dispatch` through `check::checked_dispatch_operation`, once G-7 wires it | ambiguity or ineligibility refuses at S3 with no partial table (FR-083) |
| SC-4 | The declared maximum of a population and the element type of `allInstances` | S3 | `ValueType::Population(Some(n))` gives `Set<T>[0,n]`; `Population(None)` gives an unbounded set (ADR-014 §2) | a type fact |
| SC-5 | Which objects exist, their most-specific type, field values, closure and dangling references | S6a admission | `admit_binding`, `admit_invocation`, FR-106 `admit_observations` | `dangling_reference`, `incomplete_population` (incomplete), `invalid_runtime_input` |
| SC-6 | Does a runtime object's dynamic type conform to `T` (`lookup` narrowing)? | S6a | `model::population::lookup` through `ModelIndex::conforms` under the binding's read-only `ancestor_steps` ceiling | `absent undefined` → `FamilyResult::Undefined` `absent-key`; `absent refused` → `ModelRefusal`; the ceiling reached → `Refused`, `resource_exhausted`/`ancestor-steps` (ADR-014 §7) |
| SC-7 | Does a dispatched call's precondition hold for its receiver? | S6a | `StateModel` undefined cause `PreconditionFalse` (ADR-013 O-16) | `FamilyResult::Undefined` `precondition-false` |
| SC-8 | Does a change stay inside an operation's frame? | S6a | `population::decide_frame` (FE-1) | `frame_violation`, `population_delta_mismatch` |

**Agreement rule.** This is FR-082-AC-6's rule, and TC-219 and TC-220 are
its oracle: when the type environment and population admission are
configured from the same `ancestor_steps` value, every conformance question
S3 admits, S6a completes with the same truth value. With different
configured values, S6a may stop at its own ceiling, and that stop is a
resource refusal, never a different truth value. A generalization cycle or
an unknown supertype never reaches S6a: S6a re-normalizes each domain
package from provision bytes whose digest equals the `model_selections`
digest the package was checked against (FR-106), and normalization refuses
both.

**No population data at check time.** A universe, a snapshot or an initial
state is an S6a or simulation input. The simulator's `domains()` come from
the checked types plus the request's universes (FR-120 "Bounds"), and never
feed back into S3.

### 2. Identity and provenance across check, execute, proof handoff and replay

Each row names the one minter and what every later phase does. "Carried"
means passed unchanged and never re-derived (ADR-011 §2.2). A replay
refusal cites the row of ADR-012 §15.6 or FR-098 that fixes its code.

| ID | Identity | Minted by | Execute (S6a, simulation) | Proof handoff (E5 to E8) | Replay (E9) | Equality | Oracle |
| --- | --- | --- | --- | --- | --- | --- | --- |
| ID-1 | Original declaration: `DeclarationKey{package, node}` | the domain package; admitted at I1 (ADR-013 O-03) | Carried in `StateObject.type_identity`, the `semantic` map keys and FR-120's declaration identity text | Reached from a `NodeKey` only through the model correspondence (ID-3) | Re-admitted from the QC-1 byte provision; a digest mismatch refuses `stale_dependency`/`byte-digest-mismatch` (ADR-013 O-01) | declared | FR-081 tests; TC-469 package-id pin (`config_version_spine.rs:854`) |
| ID-2 | Effective declaration: `EffectiveId` | `model` normalization only (O-05) | The type component of every `ObjectReference`; FR-120 encodes it as `object_type` | Absent from v2 (O-05) | Recomputed by re-normalization; a reference whose type is no `EffectiveId` of the bound universe refuses at FR-106 admission, `invalid_runtime_input` (ADR-012 §15.6) | normalized | FR-081 tests; FR-120-AC-4 (G-4) |
| ID-3 | Checked node of a model declaration: `NodeKey` over `ModelOwner` | `check` only (O-04, C-02), recorded in `ModelCorrespondence` | S6a selects clauses, anchors and frames by `NodeKey` | Written as `WireNodeId` | `WireNodeId` → `NodeKey` by lookup in the recompiled package (O-04) | normalized | TC-417 to TC-419 (FR-094) |
| ID-4 | Reference: `ObjectReference{UniverseId, EffectiveId, ObjectId}` | `model` at admission (O-05) | Keys `ObjectEnvironment`; QSpec FR-181 triple in state keys | Only as a witness value decoded by `Witness::decode` (C-11) | Admitted again from the replay input; a foreign universe refuses `foreign_reference` (FR-096) | declared over the triple | FR-084 tests (`model_population.rs`) |
| ID-5 | Population: `PopulationId` | `model` at admission (O-13, QC-21) | Resolves its `PopulationBinding` through `ObjectEnvironment` | Absent | Re-minted by re-admission | normalized | TC-291 to TC-297; FR-089-AC-7 (G-3) |
| ID-6 | Clause, anchor and frame: the node ids of `state`/`state_clause`, `operation_anchor` and `frame` | `check` (ADR-012 §15.4) | Selected by the name → (node id, occurrence) table in the in-process `CheckedPackage` (FR-109) | Clause node id and occurrence key enter the obligation identity (O-09) | Resolved in the recompiled package; a differing frame refuses `FrameIdentityMismatch` (FR-116) | normalized | TC-515 (FR-116) |
| ID-7 | Observation: snapshot or invocation `DocumentRef` with `sha256-jcs` digest | the document's producer; admitted by FR-106 | Anchors each clause evaluation; carried in the run report | Not on the v2 wire (ADR-012 §15.4) | Supplied by digest; a mismatch refuses `stale_dependency` (ADR-012 §15.6) | normalized | FR-106 admission tests |
| ID-8 | Simulated state: QSpec FR-181 state key and its `quire.simulation.state-key/v1` digest | `qsl_eval::simulation` from the `TransitionSystem` key (FR-101) | Coalescing compares full key bytes; traces and frontiers hold digests | Absent | FR-101 `replay` recomputes it; never E9 | lexical over key bytes | TC-453 (FR-101-AC-2) |
| ID-9 | Transition: QSpec FR-181 `{"type":"transition","operation":…,"arguments":…}` | the `TransitionSystem`; FR-120 names the operation by ID-1's declaration identity text | Orders successors (FR-101) | Absent | FR-101 `replay` | lexical over JCS bytes | TC-453 (FR-101-AC-1) |
| ID-10 | Synthesized state observation: the pair (`DocumentRef{quire, quire.simulation.state/<digest>, quire.simulation.state-key/v1, 1}`, anchor) | FR-120 `model::state::StateModel::observation` | Lets a clause evaluation name a simulated state as an observation. Its identity is the pair: one `DocumentRef` names every anchoring of one state, because the key omits the anchor | Absent | Never resolved through a byte provision; only through ID-8 | the pair | FR-120-AC-5 (G-4) |
| ID-11 | Counterexample: `WitnessEnvelope<P>` | CG from a backend run (O-25) | not applicable | Carries obligation identity, occurrence key, `package_id`, `run_limits`, declared domains, `trace_position` | The only E9 input (FR-098) | lexical over the admitted transcript | TC-515 |

**Original and effective provenance.** The original `DeclarationKey` is the
only declaration identity that crosses a phase boundary. It enters node
identity (ID-3), state keys and transition identities (ID-8, ID-9). The
`EffectiveId` appears only inside values (ID-4) and is recomputed from the
original at every admission. No `EffectiveId` becomes a `NodeKey`, and none
is carried on the wire or in a replay request (O-05). A derived effective
member is reported with the original it derives from, through the FR-081
derivation trail. `ModelCorrespondence` is one-to-one in both directions: a
second, different entry for one `NodeKey` or one `DeclarationKey` is an
internal fault (G-8).

**ID-5: population identity binds membership, maximum and pre binding.**
FR-089's open preimage question is settled here, and FR-089's Behavior is
amended to state the result. The `PopulationId` preimage is exactly
`{version, domain_package_selection, population_key, admission_role,
declared_maximum, members, pre}`, with the encodings FR-089 gives. The
version label `POPULATION_ID_DOMAIN` is unchanged: no `PopulationId` is
persisted or sent on any wire, so the golden vector
(`model/population.rs:2160`) is regenerated in the implementing PR. So:

- two admissions that agree on every member share one id;
- two that differ in members, in declared maximum (a caller argument of
  `admit_binding`, `population.rs:811`), or, for `Post` bindings, in their
  pre binding, never collide.

A `PopulationBinding` carries membership only; field values belong to the
`ObjectEnvironment` entries its references key. `with_population`'s typed
`PopulationConflict` refusal stays. With this preimage, it is reached only by
two bindings that differ in their admission limit `ancestor_steps`, which is
not identity. Reason: identity follows content (ADR-013 §2 normalized kind),
and no production path admits two bindings into one evaluation today
(`admit_binding` is called from tests and `qsl-bench/src/model.rs:309`), so
the change needs no migration.

**Keyings of simulation domains, kept apart.** A clause's requirement record
keys a population domain as `DomainKey{node: object_type node, path:
[population ordinal]}` (ADR-012 §15.7). That key crosses to `route`, CG and
QSpec FR-331. FR-120 names every simulation root (population, parameter,
result, granted field) by a `WireNodeId::from_digest` over a naming object,
and FR-101's `RequiresBound` reports it. That key is local to one
`ModelSimulationRequest`. It never crosses to `route`, CG, QSpec FR-331 or
replay, and no conversion between the two exists. A caller answers a
simulation `RequiresBound` on a population root with a universe. On a
parameter, result or field root it can only change the model, for example by
declaring a `bounded_domain`.

### 3. Population, state keys, seeds, frontiers, trace positions, cancellation, exhaustion and incomplete outcomes

ADR-014 TR-1, TR-6 and TR-7 and FR-101 define the engine's types. Two terms:
**bound exhaustion** is reaching an exploration `Limits` field (QSpec
FR-181's sense), and an **exhaustive run** is one whose frontier emptied
(`Outcome::Exhaustive`).

| ID | Concept | Decision |
| --- | --- | --- |
| EX-1 | Model population in a simulated state | A complete, closed population state per universe population (FR-120). A universe is a caller-chosen simulation input. It bounds no claim, so it is not an ADR-014 bound value, is not a `ProofBound` and is never converted into one. Its scope is EX-9's. |
| EX-2 | State key | FR-120's typed canonical form over FR-101's encoder. The anchor is not in the key (ID-10). |
| EX-3 | Seed | `SampleProvenance.seed` (TR-6). Exploration has no seed. Only `quire.simulation.sampler/v1` `1-draft.1` runs; any other refuses `GeneratorMismatch`. |
| EX-4 | Frontier | FR-101 `Frontier`, in next-expansion order, on `Bounded`, `Cancelled` and (after G-1) `Stopped`. |
| EX-5 | Trace positions | Three distinct concepts, never converted: (a) FR-101's step index inside a simulation `Trace`; (b) ADR-014 TR-2 `TemporalPosition`, carried as `qsl_replay::TracePosition`, owned by `TemporalTrace`; (c) the QSpec FR-351 separating-witness trace position. A state-family `WitnessEnvelope` carries `trace_position: None`. A simulation step index never becomes a `TracePosition`. |
| EX-6 | Cancellation | Only exploration is cancellable. `poll` is called once before each state's expansion, so a cancellation takes effect within one expansion; the returned outcome is `Cancelled{cause: cancelled/caller-cancelled}`. S6a clause evaluation and admission are not cancellable (ADR-014 §7). Sampling is bounded by `max_steps` and is not cancellable. |
| EX-7 | Bound exhaustion versus a clause meter | If the run reaches an exploration `Limits` field, it returns `Bounded{limit}`. If a clause meter runs out inside an expansion, the run is not bound-exhausted: an invariant evaluation records `InvariantUndetermined`, and a contract-clause evaluation or the `max_candidates` cap stops the expansion with `ExpansionStop` `resource_exhausted`/`insufficient-next-charge` (FR-120). |
| EX-8 | Incomplete outcomes | `Bounded`, `Cancelled` and `Stopped` with `resource_exhausted` are ADR-013 O-16 incomplete. `Stopped` with `runtime_invariant` is internal failure. `NotSimulated::RequiresBound` is a disposition, not an outcome. FR-106 `incomplete_population` is incomplete at admission. `Exhaustive` is success of the exploration only (§6). |
| EX-9 | Evidence scope | An exploration result is evidence only for its request: the `package_id`, the domain-package digests, the initial `DocumentRef`s, the universes, `Limits`, `ExpansionLimits` and the meter budget (ADR-014 §8). No type carries this set and no function compares results; this is a scope statement, verified by analysis. |
| EX-10 | Clean run | A run is clean exactly when it is `Exhaustive`, expanded at least one state, and has no finding. An `Exhaustive` run over no initial state (FR-101-AC-9) is not clean. |

### 4. Frame effects and model-bound evaluation through the Layer 3 clause/frame path

| ID | Decision |
| --- | --- |
| FE-1 | **One frame decision.** `population::decide_frame` (`pub(crate)` in `qsl-semantics`) is the only function that decides whether a change stays inside a frame. `admit_invocation`, FR-106 `observation::frame::enforce` (reached from `admit_frame_invocation` for FR-115) and FR-120 `model::state::StateModel::check_frame` all call it. It walks redefinition through `redefinition_reaches`, which charges no meter today (QSL-48); metering it is a prerequisite of G-4. A redefinition cycle cannot reach it, because normalization refuses one. Verified by inspection. |
| FE-2 | **One frame source.** An operation's frame is its one `OperationEffect` admitted at I1 (FR-103) and bound in the checked package (FR-114). A request, snapshot or universe never supplies a frame (QSpec FR-013-AC-3). |
| FE-3 | **One clause evaluator.** A state clause is evaluated only through `CheckedPackageEvaluation::evaluate_clause` and the `ProtocolClause` S6a arm (FR-107), with a fresh `Meter` per evaluation. `run_clause`, `ModelSystem` and replay all use it. |
| FE-4 | **Model reads inside clause bodies.** Field reads, `deref` and `reaches` evaluate over the clause's observation (ADR-012 §15.5). `allInstances<T>(p)` and `lookup<T>(p, r)` take a `Population<T>[N]` value, and no state-clause source can produce one: a clause's `self`, `result` and parameters draw from `ValueTypeRef`, which has only `Native` and `Package`, and the grammar has no population-valued expression. The S6a refusal `ProtocolClauseUnsupported` (`unknown_required_feature`/`unsupported-feature`, `evaluate.rs:1378`, `:1401`) is a defensive guard no admitted clause reaches. TC-467 exercises the guard by bypassing admission, so unreachability is verified by analysis. This is not a gap: an invariant already ranges over its context population (ADR-012 §15.7). Reopen condition: QSpec admits a population-valued expression in a state clause, which is new model behavior and a non-goal here. |
| FE-5 | **Effects as data.** A simulator or evaluator performs no effect. Effects are `StepEffect` data (FR-120 "Effects as data"). Forms that read ambient state refuse at S3 (FR-120-AC-12). |

### 5. Authored nondeterminism versus reproducible exploration bookkeeping

| ID | Kind | Source | Rule |
| --- | --- | --- | --- |
| ND-1 | Authored | Operation choice, argument values from finite parameter domains, frame post-states (`modifies`, `creates`, `deletes`) and operation results | Part of the successor relation. Exploration enumerates every choice. A choice leaves the successor set only for one of these reasons, each as FR-120 states: the effective precondition is false; no (candidate, result) pair makes the effective postcondition true; a contract conjunction is undecided, which disables the application and records `ContractUndetermined`; the candidate is not a closed state, so it is never generated. An `Incomplete` clause result or the `max_candidates` cap stops the whole expansion (EX-7) instead of removing one choice. |
| ND-2 | Bookkeeping | FR-101 canonical order (transition bytes, then post-state key bytes), FIFO parent order, initial-state order | Fixed by the engine. It never changes which states are reachable, only the visit order and the transition that first reaches a state, which fixes its anchor (FR-120). |
| ND-3 | Bookkeeping | Seeded sampling | Selects one path among the ND-1 choices. The same seed, trace index and `DefinitionRef` give the same trace. A sampled trace is never a claim about the unsampled choices. |
| ND-4 | Forbidden | Hash-map iteration, registration order, wall clock, ambient randomness | Never a source of order or choice (ADR-013 R-05). |

### 6. Exhaustive exploration is not proof

`Outcome::Exhaustive` states that every state reachable under the request's
universes, initial states and parameter domains was expanded. It is not a
proof of any clause for an unbounded population, for other universes or for
other initial states, and it is never `proved`. A clean run (EX-10) means "no
violation found within this request" (EX-9). Three rules hold it there:

- exploration results never enter a QSpec FR-331 result, an FR-100 clause
  outcome or any proof accounting. No map from an exploration to a clause
  outcome exists, and none is added;
- the requires-bound pre-check returns `NotSimulated::RequiresBound` for an
  unbounded domain in `domains()` before any state is expanded, and no
  `Limits` value answers it (FR-101-AC-8);
- a simulation finding is reproduced only by FR-101 `replay` of its trace. It
  never becomes a `WitnessEnvelope` and never enters E9.

Proof of a state clause goes only through negotiation. Each clause and frame
records one `operation-contract` requirement (ADR-012 §15.7). While IR admits
no `state` node, no backend proves one, and no state-clause proof evidence is
claimed toward gate QSL-20 (#219). This record claims no proof evidence, so
its dependency on QSL-20 in the ticket ("where proof evidence is claimed")
does not apply.

### 7. Family placement of the model

| ID | Decision |
| --- | --- |
| FP-1 | `StateModel` owns model declarations, population extent, redefinition, dispatch and the model expression forms (`deref`, `allInstances`, `lookup`, dispatched calls, `reaches` inside a clause), as ADR-012 §3 and §15.2 state. Its checked outputs are `check`-core types (ADR-012 §1): `NodeKind::Attribute`, `AllInstances`, `Lookup`, `Reaches` and the dispatch table. |
| FP-2 | `StateModel`'s `check` hook takes one model expression form, the way `Value`'s function-application checker (`check::family::Application`, ADR-012 §14.1 FR-065-AC-4 row) takes one nested call: the enclosing family's typer calls it from a thin arm with the shared `CheckContext`. Today that work sits inside the `Value` typer (`check/check.rs:1659`, `:2164`, `:2214`). Moving it is G-2. |
| FP-3 | `StateModel` implements `FamilyContract` and not `ReferenceEvaluation`, and no `StateModel` variant is added to `S6aFamilyKind`. No S6a input is a `StateModel` item: a model form is always nested in a `Value` function body or a `ProtocolClause` body, and it evaluates inside that family's `evaluate` hook through `value::model_query`, raising `StateModel`-owned causes (ADR-013 O-16). This applies ADR-012 §2's #214 rule (a contract part nothing can construct is left out). ADR-012 §2, §3, §5.1 S1 and §13.5 are amended to say so. Reopen condition: an S6a entry whose selected item is a `StateModel` declaration. |
| FP-4 | Simulation is a layer-5 consumer of checked packages (ADR-011 §6.1 `simulation`), not a family. `ModelSystem` is its only production `TransitionSystem`, and it evaluates through the `ProtocolClause` evaluator (FE-3), which is ADR-011 §8's "through a family evaluator". With it, lane D has converged, and `qsl_eval::simulation::Outcome` is a canonical S6a-layer outcome whose O-16 map is `Outcome::category()` (ADR-014 §7). |

### 8. Conformance summary

The implemented state/model behavior conforms to ADR-011 to ADR-015 on stage
placement (S3 typing, S6a population data, layer-5 simulation, layer-6 run
and replay), identity minting (one minting module each for `NodeKey`,
`EffectiveId` and `PopulationId`), typestate (every run starts from a
`CheckedPackage` compiled from source), outcomes (every result maps to one
O-16 category), bounds (the requires-bound pre-check, B-2 versus B-3) and
replay (frames through `replay_frame` over FR-098). It does not yet conform
where G-1 to G-10 say. None of them needs a further architectural decision.

### 9. Responsibilities returned to the implementation tickets

Each gap has one owner, a bounded acceptance criterion and a test. None is
completed by this record. #164 is Done and its three items are
delivered, so static-conformance work goes to QSL-68 (#120). Migration: none
(ADR-011 Decision 9); each replaced path is deleted in the PR that lands its
successor.

| ID | Gap | Owner | Interface or conversion | Bounded acceptance criterion | Test | Order |
| --- | --- | --- | --- | --- | --- | --- |
| G-1 | FR-101 findings and stopped expansions | QSL-67 (#121) | `TransitionSystem::successors` → `Result<Expansion<…>, ExpansionStop>`; `explore_request` → `Exploration<F>`; the four test systems adapt | FR-101-AC-12 to AC-14 | TC-474 | first, in its own PR |
| G-2a | `StateModel` check hook for each model form | QSL-68 (#120) | a `check::state_model` family module implementing `FamilyContract` per FP-2 | The `Value` typer's `Attribute`, `AllInstances`, `Lookup`, `Reaches` and dispatched-call arms each make exactly one call into `check::state_model` | Inspection, at the G-2 PR review | after G-3 to G-7; blocks no feature work |
| G-2b | Seam coverage | QSL-68 | the FR-063 checked-in seam list | The list gains the `StateModel` arms at S1 (`catalog_code()` only), S2 and S3 | TC-161 (`xtask seam-probe`) | with G-2a |
| G-2c | Cause prefix | QSL-68 | none | A model refusal raised under `Value` evaluation renders with the `state-model` catalog prefix | new unit test beside `family/mod.rs` | with G-2a |
| G-3 | ID-5 population identity | QSL-68 (#120) | `population_id_preimage` builds FR-089's seven-member preimage; `with_population` keeps `PopulationConflict` | FR-089-AC-1 (amended) and FR-089-AC-7 | TC-291 (the 3-and-7 conflict test at `qsl-eval/tests/it/model_reference_queries.rs:2834` changes to expect two ids) | before G-4 |
| G-4 | FR-120 `ModelSystem`, `model::state::StateModel`, `explore_model`, `sample_model` | QSL-67 (#121) | as FR-120 "Outputs" and "Frame and observation seam"; `check_frame` calls `decide_frame` (FE-1); candidates generated lazily in canonical order, each generated candidate charged to `max_candidates` | FR-120-AC-1 to AC-13 | TC-471 to TC-473 | after G-1, G-3 and QSL-48 |
| G-5 | FR-085 relationship-end resolution | QSL-68 (#120) | FR-085's resolution step over `RelationshipEnd` | FR-085-AC-1 to AC-3 | TC-230 to TC-232 | independent |
| G-6 | M-6c state lane deletion | QSL-67 (#121), with QSL-68 for the model half (ADR-011 M-6c row) | first split the spine generator in `examples/config-version/` from `fixtures.rs`'s native imports, so TC-469's `build()` needs no native module | ADR-012 §15.8 step 2 and ADR-011 M-6c exactly, including the SEAM-3 reads and the IR import that feed `state` (`src/state/evaluation.rs:15`), `lowering`, IT-010 and the dev dependencies, in one PR; FR-108's independent table stays | TC-469 step 1 passes with the native modules deleted; the parity test is gone | independent of G-4 |
| G-7 | Dispatch at S3 | QSL-68 (#120) | spine `compile` links dispatch through `check::checked_dispatch_operation` | A unit whose dispatched call has two most-specific candidates refuses at S3 with FR-083's ambiguity code and no checked package | new unit-level test beside TC-222 to TC-225 | independent |
| G-8 | One-to-one model correspondence | QSL-68 (#120) | `ModelCorrespondence::record` returns an internal fault for a second, different entry for one key in either direction | Recording `(n, d1)` then `(n, d2)` faults; recording `(n1, d)` then `(n2, d)` faults | new unit test in `check/identity.rs` | independent |
| G-9 | Planned model criteria | QSL-68 (#120) | as each FR states | FR-084-AC-2, AC-5, AC-7; FR-086-AC-1, AC-2, AC-4, AC-5 | TC-227, TC-240, TC-410; TC-233, TC-234, TC-236, TC-241 | independent |
| G-10 | Open QSL-68 follow-ups, each owned by its own ticket and acceptance criteria | QSL-56 (conformance checks in normalize, needed for SC-1), QSL-48 (meter the redefinition walk, needed for G-4), QSL-49, QSL-50, QSL-52, QSL-55, QSL-59 | as each ticket states | each ticket's own criteria; QSL-55 is checked against FR-104 before it starts, since state clauses now carry `result` and `self` | each ticket's own | QSL-48 before G-4 |

**Lane deletion ownership.** `native_model` is not deleted by G-6. It is
imported by SEAM-2 (`src/checking/composed/`), SEAM-3 (`src/protocol_artifact/`),
`src/temporal` and the ConfigVersion example fixtures. It goes in the PR that
removes its last importer, whichever of #218 (M-6d), the temporal M-6c PR
(#188, #189) and the last M-6e family PR lands last (ADR-011 M-6e "the last
one deletes the remainder", ADR-012 §15.8 step 3). QSL-67 and QSL-68 delete
only the composed `StateModel` family, in the PR that lands G-2a (M-6e).

**Ticket text.** The QSL-67 and QSL-68 bodies still say "extend
`runtime::execute`" and name the native lane. This record and ADR-011
govern: the work lands on the spine, and `runtime::execute` is deleted in
M-6c. On acceptance, the QSL-67 body gains G-1, G-4 and G-6, and the QSL-68
body gains G-2a to G-3 and G-5 to G-10, each as the row above states. Linear
gets a `blocks` edge from QSL-68 to QSL-67 for G-3 before G-4.

### 10. Owners and oracles for the named behaviors

| ID | Behavior | Owner | Oracle |
| --- | --- | --- | --- |
| OR-1 | Ambiguous dispatch | `model::dispatch::link_dispatch`; QSL-68 | Model layer: TC-222 to TC-225 (`qsl-semantics/tests/it/model_dispatch.rs`). Unit level: G-7's test |
| OR-2 | Static conformance (SC-2) and its runtime agreement | `TypeEnvironment::conforms`, `lookup`; QSL-68 | TC-219 and TC-220 (`type_environment_model.rs:206-400`); FR-082 tests (`model_conformance.rs`); TC-198 L08 (`equality_matrix.rs:1028`) |
| OR-3 | Closure (dangling references, complete populations) | S6a admission; QSL-68 and QSL-67 | FR-084 tests (`model_population.rs`), with TC-227 pending (G-9); FR-106 `dangling_reference` and `incomplete_population` rows (ADR-012 §15.6) |
| OR-4 | Cancellation | `qsl_eval::simulation`; QSL-67 | FR-101-AC-6, TC-455 `cancellation_stops_the_run_and_returns_the_frontier` (`finite_simulation.rs:728`) |
| OR-5 | Bound exhaustion | `qsl_eval::simulation`; QSL-67 | FR-101-AC-7, TC-455 (`:773`, `:826`, `:880`); TC-439 category map (`:1113`) |
| OR-6 | Unsupported | S6a `ProtocolClauseUnsupported` (FE-4); CG negotiation and `route` for proof; QSL-67 | TC-467 `all_instances_under_a_protocol_clause_refuses_unsupported_construct` (`evaluate.rs:2169`); TC-155 `only_the_supported_item_is_routed` (`qsl-route/tests/it/routing.rs:225`) |
| OR-7 | Incomplete | S6a meter, exploration and admission; QSL-67 and QSL-68 | TC-469's incomplete cases through `run_clause`; TC-226 `l02_…` (`model_population.rs:525`); `tc465_row23_…` (`state_clauses.rs:3349`); TC-455; FR-101-AC-12 (G-1) |
| OR-8 | Requires-bound before exploration | `explore_request`, `sample_request`; QSL-67 | FR-101-AC-8, TC-455 `requires_bound_refuses_before_any_transition_system_call` (`finite_simulation.rs:975`) |

FE-1, FE-4's unreachability, §6 and EX-9 are verified by inspection or
analysis, not by a test.

### 11. Pinned integration

| ID | Requirement |
| --- | --- |
| PI-1 | `quire-contract-model`'s checked-package reader decides the reference edge of `quire.op.model.reaches_field` (IR-370). TC-463 step 1 and TC-469 step 6 run and pass, backing FR-105-AC-3 and FR-108-AC-6's I04 `read` half; no state-node emission test is ignored or weakened. |
| PI-2 | FCD `agent-ix-extraction-frontend` and `agent-ix-semantic-ir` are consumed at their exact git revisions through `model::intake` only (ADR-011 §7.1). A change to intake shapes is a filament-core-data ticket. |
| PI-3 | QSpec contracts selected by this family: diagnostics catalog `1-draft.8`, sampler `quire.simulation.sampler/v1` `1-draft.1`, QSpec FR-181's typed canonical form, STD-111's state node rules. Each is selected by exact revision and refused otherwise (ADR-013 R-08). |
| PI-4 | The gate for a pin-bump PR is `make ci`. |
| PI-5 | No state-clause proof evidence is claimed until IR `lower` admits `state` nodes and CG has an `operation-contract` arm for them. Neither has a ticket yet (Open dependencies). Gate QSL-20 (#219) counts no evidence from this family before then. |

### 12. Evidence from implemented behavior

These run today and exercise the shared foundation. They validate the
mechanism, not the unfinished A04 and A05 features. Tests are cited by
function, not by the matrix status in `spec/tests.md`.

- **ConfigVersion through the spine**:
  `tc_469_step_1_every_case_matches_the_independent_expected_table`
  (`tests/it/config_version_spine.rs:181`). Source goes through S1 to S4
  with I1, then FR-106 admission, FR-107 S6a and `run_clause`, matching an
  independent table on stage, category, truth, code and exit code. This is
  end-to-end disposition evidence for SC-5 and FE-3. It asserts no identity
  directly; identities are pinned by `tc_469_step_6_package_id_is_pinned_across_every_case`
  (`:854`) and the ID-row oracles.
- **Frame run and frame replay**: FR-115 TC-514
  (`qsl-replay/src/spine/clause/tests/frame.rs`) and FR-116 TC-515
  (`tests/frame_replay.rs`) exercise FE-1, FE-2 and ID-6 through E9
  recompilation.
- **Finite exploration engine** (FR-101 TC-453 to TC-455, over test
  systems): EX-3, EX-4, EX-5(a), EX-6, the FR-101 encoder, and the `Bounded`
  and `Cancelled` halves of EX-7 and EX-8. The FR-120 state form,
  `InvariantUndetermined`, `ExpansionStop` and `Stopped` wait for G-1 and
  G-4.
- **Population identity** (FR-089 TC-291 to TC-297): ID-5 as it stands
  before G-3.
- **Static conformance**: #164's three items and TC-219/TC-220 (SC-1,
  SC-2, the agreement rule).

## Consequences

- QSL-67 and QSL-68 can close their state, model and simulation work against
  G-1 to G-10 without another ownership, identity or outcome question.
- The checked package stays independent of every population, snapshot and
  universe.
- Exhaustive exploration never reads as proof. Its scope is EX-9, and
  state-clause proof waits for IR and CG.
- `PopulationId` binds membership, declared maximum and pre binding, and
  `ObjectEnvironment`'s conflict refusal narrows to limit differences.
- Lane D converges when G-4 lands, and the simulation outcome is canonical.

## Amendments made with this record

- ADR-011 §7.3 M-6c row: the state half's trigger is ADR-012 §15.8 step 2,
  and `native_model` moves to the last-importer PR (§9). ADR-011 §8 lane D:
  its implementer is `ModelSystem`, through the `ProtocolClause` evaluator.
- ADR-012 Status, §2 (Stage hooks row and the `ReferenceEvaluation`
  paragraph), §3 `StateModel` row, §5.1 S1 row and §13.5 Q210-3 row:
  `StateModel` has no `evaluate` hook (FP-3).
- ADR-013 O-13 Population row, T-6 and QC-21: the `PopulationId` preimage is
  ID-5's. O-16 (both simulation sentences) and §6: the converged
  `qsl_eval::simulation::Outcome` is canonical (FP-4).
- FR-089 Behavior, AC-1, new AC-7 and Status: ID-5.
- FR-120 "Post-states" and new AC-13: every generated candidate counts
  toward `max_candidates`, and candidates are generated lazily (G-4).
- `spec/spec.md` and `spec/tests.md`: index rows.

## Open dependencies

1. IR admission of `state` nodes and a CG `operation-contract` arm (PI-5).
   Neither has a ticket; the team lead files them. Neither blocks G-1 to
   G-10.

## Alternatives Considered

- **Give `StateModel` its own S6a family kind.** Rejected (FP-3). No S6a
  input selects a `StateModel` item, so the variant would have no producer.
- **Key a `PopulationId` by document bytes.** Rejected. The same member set
  in two documents with different layout would get two identities.
- **Read the declared maximum from the population record and drop the
  `admit_binding` parameter.** Rejected here. It changes FR-084's admission
  inputs, which is feature work. Putting the maximum in the preimage keeps
  identity exact without it.
- **Convert simulation universes into `ProofBound`s.** Rejected. A universe
  bounds no claim, and its roots never cross to CG (§2).
- **Treat a clean run as a proof over the universe.** Rejected (§6). A
  universe is a caller choice, and ADR-014 §8 makes finite results evidence
  only for what was evaluated.
- **Admit population data into the checked package so the checker can decide
  closure.** Rejected (§1). The package's identity would depend on its
  observations.

---
id: ADR-014
title: "Temporal, trace and boundedness architecture (ARCH-42)"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-013
    type: depends_on
  - target: ix://agent-ix/quire-specification/AD-016
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-144
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-346
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-250
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-341
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: relates_to
---
# ADR-014: Temporal, trace and boundedness architecture (ARCH-42)

## Status

Proposed, 2026-09-24. Owning ticket: Linear QSL-17 (GitHub #222, ARCH-42),
epic QSL-34 (#205), Layer 2. It answers ADR-013 Q222-1, Q222-2 and Q222-3,
the ADR-013 O-20 owner row, and the "available finite bound" predicate that
ADR-012 §1.1 defers. It is the prerequisite of QSL-140 (ADR-013 §7 S-6),
QSL-42 (#189, V1-A12), QSL-43 (#188, V1-A11) and gate QSL-23. Supersedes
nothing. Its `/spec-review all` is SR-624 to SR-630 in
[`spec/reviews/boundedness/`](../reviews/boundedness/integrity.md).

QSpec ids below are written "QSpec FR-nnn". A bare FR id is a QSL
requirement.

## Context

Measured on QSpec `main` at `eb4234f`, QSL `main` at `fc27aacc` and
quire-contract-codegen (CG) `main` at `e2a5671`.

- **QSpec, unbounded declarations** (QSpec#113, merged as `5413ba6`). QSpec
  FR-144: the collection cardinality bound is optional, an absent bound means
  unbounded, and bound presence is part of collection type identity (QSpec
  FR-144-AC-9, AC-12, AC-13). QSpec AD-013 (accepted) states one rule with no
  compatibility layer. QSpec FR-153-AC-9: a population with no declared
  maximum is unbounded. QSpec FR-228-AC-5: a loop with a proved invariant and
  a well-founded variant is admitted with no finite maximum. QSpec FR-346,
  FR-347 and FR-348 (V1-EXPR-027 to 029): unbounded quantification, induction
  and composition are negotiated per item and settle `unsupported` with a
  warning when no capable backend is registered. The root definitions
  `quire.value.complete/v1` and `quire.model.complete/v1` are at `1-draft.2`,
  because the collection-type identity preimage changed.
- **QSpec, infinite traces** (QSpec#112, on `main`). QSpec FR-090-AC-7:
  selecting `quire.temporal.infinite-trace/v1` admits the unbounded temporal
  operator forms, and no other profile does. QSpec FR-090, FR-250-AC-6 and
  FR-255: that profile admits no interval bound, and an operator admitted
  under it carries no interval object. QSpec FR-161: fairness restricts the
  admitted traces before evaluation; finite prefixes, absence of a
  counterexample and resource exhaustion never prove infinite satisfaction; a
  counterexample is a finite prefix plus a loop (a lasso); with no liveness
  backend an unbounded formula settles `unsupported` with a warning. QSpec
  FR-341 maps `proved`, `refuted`, `inconclusive`, `unsupported` and `failed`
  onto the QSpec FR-241 to FR-243 axes.
- **QSpec, capabilities.** QSpec FR-290 has ten kinds and two modes. It has no
  liveness kind and no quantifier kind: a temporal claim under either profile
  is `temporal-satisfaction`, and quantification, induction and composition
  are obligation forms inside a claim that the selected backend's arm
  settles. Its advertised-mode table settles `requires-bound` for an
  unbounded extent on a bounded-only candidate when a finite bound is
  available, and `unsupported`/`unbounded-extent` when none is. CG reads that
  availability as `ExtentClassification.finite_bound_available` and computes
  none of it (CG `src/capability.rs:150-163`): it belongs to #222.
- **QSpec, diagnostics** (`quire.native.diagnostics/v1` `1-draft.7`).
  `stage_limit_exceeded` covers the limits of S1 to S4, the I2 reader, a
  family `check`, `replay` and `route`. `resource_exhausted` is the caller's
  work-budget meter, and "a semantic maximum is not a caller work budget".
  There is no timeout, solver-absent or bound-exhausted code: those are QSpec
  FR-331 results.
- **QSpec, kernel.** AD-016's Shared-type row lists the kernel types exactly,
  including `CardinalityBound`.
- **QSL code.** Bounds: kernel `CardinalityBound{minimum, maximum}`
  (`quire-exact/src/collection.rs:75`), held by `CollectionType.bound`
  (`:126`), and `ValueType::Population(u64)`. The spine grammar requires the
  `[uint, uint]` collection bound (`qsl-cst/src/grammar.rs:419-429`) and an
  interval on every temporal operator (`:1000-1017`). Limits: F
  `diagnostic::LimitExceeded`/`LimitKind` (T-4), kernel
  `Meter`/`Incomplete`/`quire_exact::LimitKind`. Capability: `Capability`,
  `qsl_route::Mode`, `BackendDescriptor`, `Registry::candidates`,
  `routing::Disposition`. Outcomes: `qsl_replay::proof_result::{ProofCategory,
  IncompleteCause, UnavailabilityCause, InconclusiveCause}`. Replay:
  `WitnessEnvelope<P: FamilyPayload>` with an opaque `TracePosition`, a
  `proof_bounds: ScalarLimits` member and `DeclaredDomain{parameter, domain:
  String}`. Simulation: `explore::Outcome{Exhaustive, Bounded{frontier,
  limit}, Cancelled}` and `SampleProvenance{seed, sampler_version, stopped}`.
  The bounded temporal evaluator (`src/temporal`) and the native-v1 ceilings
  are SEAM code that retires in ADR-011 M-6c. No infinite-trace code exists.

## Decision

Item ids `B-`, `A-`, `TR-` and `N-` are local to this record. Other artifacts
cite them as `ADR-014 B-n`.

### 1. Bound taxonomy (Q222-1)

Six kinds exist. Each is its own type with one owner. No value of one kind
converts into another kind.

| ID | Kind | Meaning | Type and owner | Supplied by | When reached |
| --- | --- | --- | --- | --- | --- |
| B-1 | Authored semantic bound | Part of the meaning of a declaration: which values the type admits | Kernel `CollectionType.bound: Option<CardinalityBound>` and `ValueType::Population(Option<u64>)`, where `None` is unbounded; the `BoundedInteger` range of a `bounded_domain`; the TemporalTrace `TemporalInterval` (§3). The S3 checker builds them from forms | the source author | A value outside the bound refuses `cardinality_out_of_bound` or `BoundViolation` (QSpec FR-144). Never a resource outcome |
| B-2 | Execution resource bound | How much work one evaluation, normalization or admission may spend | A ceiling of an accounting-contract limits type: kernel `ScalarLimits`/`Meter` (`quire.value.accounting/v1`, QSpec FR-323 `limits`), `ModelNormalizationLimitsV1`, `PopulationAdmissionLimitsV1` | the caller, per run | A denied charge yields `Incomplete` with its charge point. A read-only ceiling (FR-082 and NFR-012 `ancestor_steps`, `family_steps`) refuses. Both carry `resource_exhausted` (or `cancelled`) |
| B-3 | Stage limit | How much input and work a compiler stage or reader may take | A ceiling of a stage's own limits type (`CheckingLimits`, `TypeEnvironmentLimits`, the reader limits), reported as F `diagnostic::LimitExceeded` (ADR-013 T-4, FR-096) | the caller of the stage, or that type's published default | `StageFailure::Limit(LimitExceeded)`, `stage_limit_exceeded`. A stage failure, not an outcome category |
| B-4 | Proof bound | A finite domain the caller substitutes for one unbounded domain of an item, to ask a bounded-mode backend | F `bound::ProofBound{domain: DomainKey, bound: FiniteBound}` (§4), built by QSL-140. On the wire: QSpec FR-331 request `domains`. In replay: `DeclaredDomain.domain` becomes `FiniteBound` | the caller of the request, explicitly | Not reached. It is part of the obligation identity (O-09), and a result qualifies only over it |
| B-5 | Backend tool budget | How long and how deep a backend tool runs (Kani unwind, solver time) | IR `ResourceBounds` and the AD-016 Kani tool pin. Not a QSL type | the backend provider | QSpec FR-331 `incomplete` with `IncompleteCause::{TimedOut, ResourceExhausted}` |
| B-6 | Profile ceiling | A maximum a selected profile fixes for every package | None on the spine. The native-v1 ceilings (`MAX_SEQUENCE_ITEMS`, `src/native_model/admission.rs:18`; `checked_handoff::MAX_POPULATION`; the `native_temporal` `MAX_*`; `NativeModelProfile`) are lane-private (ADR-013 §6) and retire with SEAM-1 and M-6c | none | Native-v1 admission keeps refusing until it retires. No spine stage reads a profile ceiling |

**Classification rule.** A limit is classified by the type that carries it,
not by the stage that reads it. A ceiling of an accounting-contract limits
type is B-2 wherever it is read, including `ModelNormalizationLimitsV1`,
which S3 `model` reads (QSpec FR-150, NFR-012, ADR-013 O-21). A ceiling of a
stage's own limits type is B-3. On this rule the only reclassification is the
expression checker's `TypeEnvironmentLimits{ancestor_steps, work_units}`: it
is a check-stage limits type, so it is B-3. §12 amends FR-082, FR-096 and ADR-013 T-4 to say this rule.

A caller may set a B-2 ceiling and a B-3 ceiling from one configured number,
for example the same `ancestor_steps` value for population admission and for
the type environment (FR-082-AC-6). That is configuration. No value converts.

**Relations between kinds.** A bounded authored domain needs no B-4 value: it
is already its own proof domain (AD-016 harness-domain rule). A B-4 value is
used only to build a bounded request (§4). No B-2, B-3, B-5 or B-6 value
ever becomes a B-1 or B-4 value.

The replay envelope's `proof_bounds: ScalarLimits`
(`qsl-replay/src/witness.rs:346`) holds the proving run's accounting limits,
a B-2 value. QSL-140 renames it `run_limits` and types
`DeclaredDomain.domain` as `FiniteBound`.

### 2. Absent bounds (Q222-2, first part)

Every absent bound means exactly one of three things: unbounded, inherited or
invalid. No bound in QSL is "unspecified".

| Where the bound is absent | Meaning | Rule |
| --- | --- | --- |
| Collection cardinality `K<T>` | unbounded | QSpec FR-144. `CollectionType.bound == None`, part of type identity. `K<T>` and `K<T>[a,b]` are different types |
| Population maximum | unbounded | QSpec FR-153-AC-9. `ValueType::Population(None)` |
| Integer range (`Int` with no `[a,b]`) | unbounded | The kernel `Integer` is unbounded. Already the rule |
| Recursive value type depth | unbounded | QSpec FR-143 recursive types have no depth bound |
| Loop maximum | invalid, unless the loop carries an invariant and a well-founded variant | QSpec FR-228-AC-5, FR-171. S3 refuses a loop with neither a maximum nor a variant. A declared variant that is not proved settles `inconclusive`. A loop is never silently bounded |
| Temporal interval under a bounded profile | invalid | The S3 TemporalTrace `check` refuses it (QSpec FR-091, FR-092) |
| Temporal interval under `quire.temporal.infinite-trace/v1` | not applicable | QSpec FR-090, FR-255: the operator has no interval |
| B-2 or B-3 limit | inherited | The published default of that limits type (for example NFR-011, NFR-012). A limit is never unbounded at its point of use |
| B-4 for an unbounded domain | none supplied | §4 |
| Extent classification of a requested item | invalid | `invalid-request`, `invalid_capability`/`absent-extent` (QSpec FR-290, FR-057) |

### 3. Trace concepts (Q222-2, second part)

| ID | Concept | Owner and type | Rule |
| --- | --- | --- | --- |
| TR-1 | Trace identity | No new identity or digest domain. A counterexample trace is identified by its packet (obligation identity O-09, occurrence key O-07) and compared lexically over its canonical encoding, as O-25 compares a transcript. A sampled trace is identified by `SampleProvenance{seed, sampler_version}` plus its index in the run's output. An observation trace handed to S6a is an input value, not an identity | A trace never takes its identity from arrival order or storage position (ADR-013 R-05). QSpec FR-181 names the sampler by `DefinitionRef`; replacing `sampler_version: String` with it is the simulation lane's work |
| TR-2 | Trace position | Layer-5 TemporalTrace evaluator: `TemporalPosition(u64)`, the zero-based index into the represented trace, prefix first, then loop. It crosses replay in `qsl_replay::TracePosition` as decimal ASCII with no leading zero (`0` is position zero). Replay stores it and does not read it (O-25) | Only the TemporalTrace evaluate hook decodes it. A position outside the represented trace refuses at reconstruction, `invalid_runtime_input`/`invalid-value` |
| TR-3 | Interval | Layer-3 TemporalTrace `check`: `TemporalInterval{lower: u64, upper: u64}`, one validated constructor. It exists only under a bounded profile. Its key is QSpec FR-255's (`lower`, `upper`, profile identity, clock binding), and it is identified by the checked node id of the operator that carries it (O-04). A checked operator holds `Option<TemporalInterval>`: `Some` under a bounded profile, `None` under infinite-trace | `lower > upper`, a missing interval under a bounded profile, any interval (including `[a,*]`) under infinite-trace, and `[a,*]` under a bounded profile each refuse as QSpec FR-091, FR-092 and FR-090 state. Bounded and infinite-trace operators never share a representation |
| TR-4 | Horizon | Derived, not stored. The TemporalTrace `check` computes the greatest reach of a bounded-profile formula's intervals with checked arithmetic. Overflow past `u64` refuses as a TemporalTrace `check` stage limit, `LimitExceeded` with limit kind work budget (`stage_limit_exceeded`), at the operator. An infinite-trace formula has no horizon | A horizon is a B-1 consequence, never a budget |
| TR-5 | Evaluation budget | The kernel `Meter` (B-2). The S6a TemporalTrace evaluator charges `quire_exact::LimitKind::WorkUnits` once per (temporal node, position) visit | Running out yields `Incomplete`, never a truth value. The native `quire.native.temporal-work/1` counters retire with M-6c |
| TR-6 | Seed | `SampleProvenance.seed` in `qsl_eval::simulation`. Exhaustive exploration (`explore`) has no seed: it is deterministic given its `Limits` and the model | The same seed and sampler reproduce the same traces |
| TR-7 | Frontier | `qsl_eval::simulation::frontier::Frontier` in `explore::Outcome::{Bounded{frontier, limit}, Cancelled{frontier}}` | An exploration that stops early reports its frontier and never claims exhaustive success (QSpec FR-181) |
| TR-8 | Liveness capability | No new type. It is the QSpec FR-290 pair (`temporal-satisfaction`, `unbounded`) in `BackendDescriptor.advertises`, plus a CG `negotiate_*` arm that discharges the infinite-trace IR form | §6 |

### 4. Extent, domains and the available finite bound (Q222-3)

Three concepts, three types:

- **Finite** is a property of a concrete value or trace. It is never an
  extent.
- **Extent** is a property of a requested item: `ClaimExtent{Bounded,
  Unbounded{domains: Vec<DomainKey>}}`, in the layer-3 family core
  (`qsl-semantics::family`), beside `Requirements`. It is not named `Extent`,
  because `model::Extent{Closed, Open}` is QSpec FR-153 population closure.
  Wire spelling: QSpec FR-290 `bounded`/`unbounded`.
- **Mode** is a property of a backend advertisement: `qsl_route::Mode{Bounded,
  Unbounded}`, which exists.

**Domain key.** F `bound::DomainKey{node: WireNodeId, path: Vec<u32>}` names
one unbounded domain: the checked node that carries it (a parameter, a bound
variable, a loop, or a temporal clause), then, for a type position, the
child-index path into that node's type (element, field, variant payload). A
loop or a clause has an empty path. It is a
wire-level key because it crosses to CG and to replay; layer 3 converts its
`NodeKey`s to `WireNodeId`s when it builds the request (ADR-013 O-04). F
`bound` is a new foundation module that imports only K and F `digest`, so
`qsl-replay`, layer 3 and the request writer can all name it.

**Extent rule.** The owning family computes `ClaimExtent` in
`requirements()` (ADR-012 §2) over the transitive closure of the claim's
argument and bound-variable types. Each of these positions is an unbounded
domain:

| Unbounded domain | Boundable by a `FiniteBound` |
| --- | --- |
| collection with `bound: None` | yes: `Cardinality{maximum}` |
| population with `Population(None)` | yes: `Cardinality{maximum}` |
| `Integer` with no range | yes: `IntegerRange{lower, upper}` |
| recursive value type (QSpec FR-143) | yes: `Depth{maximum}` |
| loop admitted under QSpec FR-228-AC-5 | no |
| temporal formula under `quire.temporal.infinite-trace/v1` | no. A finite prefix never proves infinite satisfaction (QSpec FR-161) |

No unbounded domain gives `Bounded`. Otherwise the extent is `Bounded`.

**Available finite bound.** For an item with extent `Unbounded`, a finite
bound is available exactly when every domain in `domains` is boundable (the
table's right column). QSL-140's O-20 request writer computes it and writes it
into the QSpec FR-331 item's extent classification. CG reads it
(`ExtentClassification.finite_bound_available`) and computes nothing. With it
true, a bounded-only candidate settles `requires-bound`. With it false, the
candidate settles `unsupported`, warned, `unbounded-extent`. No B-2, B-3, B-5
or B-6 value, default, profile or registered backend makes a bound available.

**Bounded request.** The caller answers `requires-bound` by submitting a new
request that carries one `ProofBound` for each domain in `domains`. The
request writer substitutes each `FiniteBound` for its domain, so the new
item's extent is `Bounded` (QSpec FR-290: a finite domain classifies as
bounded). Its obligation identity includes the substituted domains (O-09,
AD-016 arrow 5), and it has its own `request_index`. The unbounded item keeps
its terminal `requires-bound`. That closes the loop: a request with bounds is
bounded, a request without them is not.

The request writer refuses, before any request is emitted, a `ProofBound`
whose `DomainKey` names no unbounded domain of the item, a `ProofBound` on a
domain that is not boundable, a `FiniteBound` whose variant does not match the
domain's kind (for example `IntegerRange` on a collection), a set that misses
a domain, and a `FiniteBound` whose range is empty or inverted (its
constructor refuses those). The code is `invalid_runtime_input`/`invalid-value`. Two
bounds for one domain cannot be built: the writer takes a map keyed by
`DomainKey`.

**IR's predicate.** AD-016's IR `requires-bound` is the same classification
over the lowered form: which IR forms carry an unbounded domain. IR reads the
v2 wire QSL emits (ADR-011 FB-05), and a QSL-140 and IR conformance test over
the §10 scenarios pins that the two agree.

### 5. The `quire.temporal.infinite-trace/v1` facet (ticket decision 4)

| ID | Rule |
| --- | --- |
| A-1 | **Parse.** S1 and S2 parse the bare unbounded operators (no interval) under every profile. Parsing selects no meaning. The spine grammar makes the interval optional on the unary and binary temporal operators (QSL-43). |
| A-2 | **Admit.** The S3 TemporalTrace `check` admits a bare operator only when the unit's temporal profile selection is `quire.temporal.infinite-trace/v1` (QSpec FR-090-AC-7). Otherwise it refuses `unsupported_construct`/`expression-form`, located at the operator (ADR-012 §3). Under that profile it refuses any interval (TR-3). A unit selects exactly one temporal profile. The profile uses the event-position sequence authority without false-extension closure (QSpec FR-250-AC-6). The clause's authored fairness constraints are checked with it (QSpec FR-161). |
| A-3 | **Record.** An admitted infinite-trace clause yields `Requirements{kind: temporal-satisfaction, extent: Unbounded{domains}}`, where `domains` names the formula. |
| A-4 | **Execute.** S6a never settles an infinite-trace claim `proved`. Over a finite trace it uses three-valued finite-prefix evaluation: it returns violation only when the formula is in the safety fragment (negation normal form over atoms, `and`, `or`, `always`, `release` and past operators) and the prefix makes it false for every extension; it returns pending otherwise, O-16 `inconclusive` (QSpec FR-161-AC-2, FR-324-AC-2). This rule is sound and incomplete: it never reports a false violation. Over a lasso (a non-empty prefix or none, then a non-empty loop that re-enters at its first position) it first checks the lasso against the clause's fairness constraints; a lasso that violates them is not an admitted trace and is refused with `invalid_runtime_input`/`invalid-value`, and a clause whose fairness premise is missing yields no verdict: its S6a result is O-16 category unsupported, as QSpec FR-341 settles the same case for a backend. A fair lasso is evaluated exactly, with work charged per visit under TR-5. A false result is a violation for that lasso. A true result is `tested`, evidence for that lasso only. This is replay of a given trace, not a liveness solver. |
| A-5 | **Discharge.** Proof of an infinite-trace claim goes only through negotiation (§6). QSpec FR-341 results map to O-16: `proved` → success, `refuted` → violation, `inconclusive` → inconclusive, `unsupported` → unsupported, `failed` → incomplete when resource-incomplete, otherwise internal failure. |

### 6. Capability negotiation for liveness and quantifier-capable backends (ticket decision 5)

No new capability kind, flag or mode is added. QSL records; CG settles.

1. QSL S3 records `Requirements{kind, extent, bounds}` per item (§4).
2. `qsl_route::Registry::candidates` matches on kind alone (FR-075).
3. QSL-140's request writer writes the item's extent classification,
   including `finite_bound_available` (§4).
4. CG `negotiate_*` applies the QSpec FR-290 advertised-mode table, then the
   candidate's arm decides whether it discharges the IR obligation form
   (quantifier, induction, composition, infinite-trace formula). An arm that
   does not discharge the form settles `unsupported`, warned,
   `unsupported_projection`/`unsupported-requested-capability`, naming the
   required capability. CG's `negotiate_kani` settles on modes alone today; the
   form check is CG follow-up work, which the QSL-42 and QSL-43 exit cases do
   not need, since no liveness or quantifier backend is registered for them.
5. **Kani.** Kani advertises its kinds with mode `bounded` only. The Kani
   provider manifest that states this is CG's (QSpec FR-331); until CG
   publishes it, the QSL-42 exit tests register a test descriptor with the
   same advertisement. For an unbounded item on a Kani-only registry, Kani is
   never `supported` (QSpec FR-290-AC-6). It settles `requires-bound` when a
   finite bound is available, and `unsupported`, warned, `unbounded-extent`
   when not. A Kani result on the resulting bounded request carries that
   request's obligation identity, which includes the substituted domains
   (O-09). It is joined on the bounded request's own `request_index` in the
   QSpec FR-331 accounting record and never on the unbounded item's. QSL
   computes the extent from the checked types and never takes it from the
   caller, so no caller input can make an unbounded item classify `Bounded`
   without its bounds entering its identity.

### 7. Structured outcomes (ticket decision 6)

| Case | Where it is settled | Type | Catalog and O-16 category |
| --- | --- | --- | --- |
| Unsupported backend: empty candidate set, an arm that does not discharge the form, or an unbounded extent with no finite bound available | CG negotiation | `qsl_route::routing::Disposition::Unsupported` (from QSpec FR-331 `dispositions`) | `unsupported_projection` with `unsupported-requested-capability` or `unbounded-extent`; unsupported |
| `requires-bound` | CG negotiation | `Disposition::RequiresBound` | QSpec FR-331 disposition; unsupported |
| Solver or tool absence at the probe | the routed backend's probe (ADR-012 §7.4) | QSpec FR-331 result `unsupported`, `UnavailabilityCause::SolverAbsent` | `unsupported_projection`/`tool-unavailable`; unsupported |
| Tool changed after a passing probe | the run | QSpec FR-331 result `failed` | `unsupported_projection`/`tool-unavailable`; internal failure |
| Bound exhaustion, S6a charge | S6a meter | `Outcome::Incomplete` | `resource_exhausted`/`insufficient-next-charge`; incomplete |
| Bound exhaustion, read-only B-2 ceiling | S3 model normalization and S6a population admission | `Refused` with the `ancestor-steps` cause (FR-082) | `resource_exhausted`; refusal |
| Bound exhaustion, finite exploration | `qsl_eval::simulation::explore` | `Outcome::Bounded{stats, frontier, limit}` | incomplete; never success, never violation |
| Bound exhaustion, backend | backend | `IncompleteCause::ResourceExhausted` | QSpec FR-331 `incomplete`; incomplete |
| Stage ceiling | S1 to S4, I2, `check`, `replay`, `route` | `StageFailure::Limit(LimitExceeded)` | `stage_limit_exceeded`; a stage failure |
| Timeout | backend | `IncompleteCause::TimedOut` | QSpec FR-331 `incomplete`; incomplete |
| Cancellation | backend or exploration | `IncompleteCause::Cancelled`; `explore::Outcome::Cancelled{frontier}` | `cancelled`/`caller-cancelled`; incomplete. The kernel `Outcome` has no cancellation variant, and S6a is not cancellable |
| Semantic violation | S6a or backend | `Completed(false)` of a claim; QSpec FR-331 `refuted` with its counterexample | violation |
| Unresolved liveness on a finite prefix | S6a | pending truth | inconclusive |

One mapping is new: `explore::Outcome` to O-16. `Exhaustive` is success;
`Bounded` and `Cancelled` are incomplete, each keeping its frontier. It lives
in layer 5 as `qsl_eval::simulation::explore::Outcome::category()`, which
returns F's O-16 category type (layer 5 may depend on F; F may not depend on
layer 5).

### 8. Bounded reference runtime (ticket decision 7)

- Bounded evaluation is unchanged: QSpec FR-145 occurrence visiting, the
  bounded temporal profiles, and finite exploration. Every existing bounded
  corpus keeps its results.
- A concrete value of an unbounded type is finite, and S6a construction and
  traversal over it are exact. They never refuse for cardinality (QSpec
  FR-144-AC-12) and stop only on B-2.
- S6a never decides a claim whose extent is `Unbounded`. For `forall` and
  `exists` over an unbounded collection type, QSpec FR-346 makes the claim's
  decision a negotiated obligation (QSpec FR-346-AC-2). S6a evaluating the
  quantifier over one concrete input is a test of that input: a false result
  is a counterexample for that input, and a true result is `tested`. Neither
  is `proved` or a decision of the claim.
- The same holds for finite exploration and for a bounded backend request: a
  result is evidence only for what it evaluated (one input, one explored
  bound, one set of `ProofBound`s), and its identity carries that scope. Three
  consumers enforce this. The QSpec FR-331 accounting record joins a result
  only to its own `request_index`, so a bounded result cannot settle the
  unbounded item (§6 step 5). The O-16 map keeps `tested` distinct from
  `proved` and never promotes it. The replay settlement
  `reproduced-without-witness` never counts as backend evidence (O-25).
- The native-v1 ceilings (B-6) are not lifted. Native-v1 admission keeps
  refusing unbounded declarations (`UnrepresentableConstraint`) until SEAM-1
  retires. ADR-011 §10 scenario 3 forbids extending `NativeModelProfile`.

### 9. Version and package effects; compatibility (ticket decision 8)

| ID | Effect |
| --- | --- |
| N-1 | The collection-type identity preimage is QSpec's root definitions at `1-draft.2`. QSL selects exactly that revision and refuses any other (ADR-013 R-08, O-22). QSL-42 (V1-A12) is the ticket in which QSL adopts the `1-draft.2` collection identity preimage. QSL references the definitions by revision and digest and copies none of them. |
| N-2 | Every package with a collection type gets new node ids and a new `package_id` under `1-draft.2`, because the preimage changed. Bounded corpora keep their source spelling and meaning. Their expected identities are regenerated, not migrated. |
| N-3 | The kernel change is a field shape, not a new type: `CollectionType.bound` becomes `Option<CardinalityBound>` and `ValueType::Population` carries `Option<u64>`. AD-016's kernel row keeps its type list, and ADR-011 §6.1's "`ValueType::Population` carries its count only" still holds. Under the bounded profiles the interval stays mandatory, as today. The v2 contract stays `quire.checked-package/v2`; while it is prerelease, QSpec revises its node and operation set in place (ADR-013 QC-19). |
| N-4 | Compatibility: none is needed. This is pre-release with no users. There is one rule for absent bounds (§2) and no second spelling. There is no reader for `1-draft.1`, no bounded-by-default reading of `K<T>`, and no adapter. |

A claim over an `Int` argument with no range has extent `Unbounded` (§4).
That is the existing AD-016 rule (a value outside a declared domain is
`requires-bound`, never narrowed; ADR-013 C-22), made explicit. An author who
wants a bounded claim declares a `bounded_domain` such as `Int[0, 9]`.

### 10. Change scenarios

1. **Unbounded `eventually`, with and without a liveness backend.** S2 parses
   the bare `eventually p` (A-1). The S3 TemporalTrace `check` admits it
   because the unit selects `quire.temporal.infinite-trace/v1` (A-2); under a
   bounded profile it refuses `unsupported_construct`. `requirements()` returns
   (`temporal-satisfaction`, `Unbounded{[formula]}`) (A-3). The formula is not
   boundable, so `finite_bound_available` is false (§4). With no backend
   registered, `Registry::candidates` is empty and `negotiate_*` settles
   `unsupported` with the warning naming `temporal-satisfaction` (QSpec
   FR-161-AC-7). With a backend advertising (`temporal-satisfaction`,
   `unbounded`) whose arm takes the infinite-trace form, it settles
   `supported`, `route` routes it, and the QSpec FR-341 result maps through
   A-5. A bounded-only temporal backend settles `unsupported`,
   `unbounded-extent`.
2. **Unbounded collection declaration with bounded native evaluation.**
   `Set<Account>` parses (QSL-42's grammar change). S3 builds
   `CollectionType{bound: None}`. S4 emits it under the `1-draft.2` preimage
   (N-1). S6a evaluates a concrete set of any size: construction never refuses
   for cardinality, it charges `collection.bound`, and it stops only with
   `Incomplete`/`resource_exhausted` when the caller's meter runs out (B-2). A
   claim over the declared type has extent `Unbounded`, and evaluating it on
   one input is a test of that input (§8). Native-v1 still refuses the
   declaration (B-6).
3. **Claim over an unbounded collection with Kani.** `requirements()`
   returns (`value-validity`, `Unbounded{[the set's DomainKey]}`). The domain
   is boundable, so `finite_bound_available` is true. The candidate is Kani,
   bounded-only, so `negotiate_*` settles `requires-bound` and emits no
   harness. The caller submits a new request with `ProofBound{domain,
   Cardinality{maximum: 8}}`. The writer substitutes it, the new item's
   extent is `Bounded`, and Kani settles its own disposition. A Kani `proved`
   on it is qualified by `maximum: 8` in its obligation identity and joined on
   its own `request_index`. The unbounded claim stays `requires-bound`. It is
   never narrowed. Had the claim also ranged over an FR-228-AC-5 loop, the
   bound would not be available, and the item would settle `unsupported`,
   warned, `unbounded-extent`, naming the kind, the candidate and its modes.
4. **Finite exploration that exhausts its bound.**
   `qsl_eval::simulation::explore` stops at `Limits{max_states, max_depth,
   max_transitions}` and returns `Outcome::Bounded{stats, frontier, limit}`.
   `Outcome::category()` maps it to incomplete (§7). The frontier and the
   reached limit are kept. No exhaustive success and no verdict is reported.
   The run is deterministic given its `Limits` and the model, so it
   reproduces.
5. **Replay of a temporal counterexample.** A backend returns QSpec FR-331
   `refuted` with a lasso. The #231 envelope `WitnessEnvelope<P>` carries the
   obligation identity, occurrence key, `package_id`, profile selections
   (including the temporal profile), `trace_position` (TR-2) and the
   TemporalTrace `FamilyPayload`, `TemporalCounterexample{prefix, loop,
   fairness, interval: Option<IntervalKey>}`, defined in `qsl-replay` beside
   the other family payloads. `IntervalKey{lower, upper, profile:
   DefinitionRef, clock_binding}` is the wire-level QSpec FR-255 key, defined
   in F `bound` so `qsl-replay` can name it; the TemporalTrace `check`
   converts a `TemporalInterval` to it. `interval` is the failing operator's
   key under a bounded profile and `None` under infinite-trace.
   `fairness` names the clause's fairness constraint nodes. At ADR-011 E9 the
   layer-6 `replay` facade recompiles from digest-addressed source and
   resolves the occurrence key to the operator node. It refuses, with
   `stale_dependency`/`revision-mismatch`, when the recompiled operator's
   interval key, the profile selection or the fairness set differs from the
   packet's. It refuses a malformed lasso (empty loop, position out of range)
   with `invalid_runtime_input`/`invalid-value`. The TemporalTrace evaluate
   hook then re-evaluates the formula over the lasso at the decoded
   `TemporalPosition` (A-4). Agreement settles
   `reproduced-with-evaluated-witness`. Disagreement settles `inconclusive`
   with a typed cause (O-27).

### 11. Named interfaces for the dependent tickets

**QSL-140 (ADR-013 §7 S-6)** builds:

- the kernel field shapes `CollectionType.bound: Option<CardinalityBound>` and
  `ValueType::Population(Option<u64>)` (N-3);
- F `bound`: `DomainKey`, `FiniteBound{Cardinality, IntegerRange, Depth}`,
  `ProofBound` and `IntervalKey`;
- in `qsl-semantics::family`: `ClaimExtent`, the `Requirements` type and the
  `FamilyContract::requirements()` method (this moves FR-062-AC-4 from QSL-152
  to QSL-140);
- the O-20 request writer: extent classification with
  `finite_bound_available`, `ProofBound` substitution and its refusals (§4);
- in `qsl-replay`: `DeclaredDomain.domain: FiniteBound` and the rename of
  `proof_bounds` to `run_limits`;
- `qsl_eval::simulation::explore::Outcome::category()` (§7);
- the QSL-140 and IR agreement test for the extent classification (§4);
- O-19 `Capability` and the O-20 request representation, as ADR-013 lists.

**QSL-42 (V1-A12)** builds against them:

- the optional collection bound at the grammar
  (`qsl-cst/src/grammar.rs:419-429`), in `qsl-forms/src/value.rs` and in
  `check/type_form.rs`, and `Population(None)`;
- the `1-draft.2` preimage (N-1);
- `requirements()` for each claim form it touches, with the §4 extent rule;
- its exit tests on a bounded-only test descriptor (§6 step 5).

It does not touch `NativeModelProfile` or the native-v1 ceilings (B-6).

**QSL-43 (V1-A11)** builds:

- the TemporalTrace migration ADR-011 already gives #188 and #189: the M-3b
  temporal S2 forms, the bounded-profile S3 `check` and the layer-5
  evaluator, and the M-6c deletion of `src/temporal`;
- the optional interval on the unary and binary temporal operators in the
  spine grammar (A-1);
- `TemporalInterval` and the infinite-trace admission (A-2, TR-3) in the S3
  TemporalTrace `check`, and `requirements()` for temporal clauses (A-3);
- `TemporalPosition` and the finite-prefix and lasso evaluation (A-4) in the
  layer-5 TemporalTrace evaluator;
- `TemporalCounterexample: FamilyPayload` in `qsl-replay`, and the scenario 5
  replay refusals and settlement.

Temporal atoms over state and protocol operations need the checked types of
QSL-68 (#120) and QSL-21 (#218); atoms over values alone do not.

Both tickets settle their exit cases through `Registry::candidates`,
CG `negotiate_*` (agent-ix/quire-contract-codegen#86, which reads the extent
classification) and `routing::route`, which is unchanged.

### 12. Amendments made with this record

- ADR-013: Q222-1 to Q222-3 answered in its §8 question table; O-20's owner
  row names §4 and IR's form predicate separately; O-21 and the S-6 row point
  here.
- ADR-012 §1.1: negotiation settles from QSL's extent classification, and the
  "available finite bound" predicate is §4 of this record. ADR-012 §2's
  `requirements()` deferral and its §14 row move to QSL-140.
- ADR-013 T-4 and FR-096: "every stage ceiling" means a ceiling of a stage's
  own limits type (§1); FR-096 gains a third no-region case for the
  type environment.
- FR-057: the `requires-bound` row cites §4.
- FR-082: the expression checker's type-environment ceilings are stage limits
  (B-3), reported `stage_limit_exceeded` with limit kinds node count
  (`ancestor_steps`, a count of expanded types) and work budget
  (`work_units`). The model's conformance walk and S6a population admission
  keep `resource_exhausted` (B-2). The code change belongs to QSL-160 (S-5b,
  `LimitExceeded`), and TC-220's type-environment rows are pending until it
  lands.
- `spec/spec.md` and `spec/tests.md`: index rows.

### 13. Open dependencies

1. **QSpec v2 temporal operation identities.** The QSpec operation catalog has
   `quire.op.temporal.clause` only, and the v2 `TemporalNode` body is
   unconstrained. S4 emission of any temporal formula, bounded or
   infinite-trace, waits for that spelling, and so does IR v2 temporal intake
   (Linear IR-7). No QSpec ticket tracks it yet. QSL-43's S3 admission, S6a
   evaluation and replay do not wait for it.
2. **A QSpec wording conflict on `[a,*]`.** The shared grammar and TC-238
   mention `[a,*]` under the infinite-trace profile, while QSpec FR-090,
   FR-250-AC-6 and FR-255 say that profile admits no interval. This record
   follows the requirements (TR-3), so nothing waits on it. QSpec should
   reconcile the grammar note.

There is no open STD-12 dependency: QSpec#113 is fully on QSpec `main`.

## Consequences

- QSL-140 can build its bound types, and QSL-42 and QSL-43 can build against
  them, with no further ownership question.
- A claim is never narrowed silently. QSL computes the extent from checked
  types, CG settles against Kani's bounded-only advertisement, and a bounded
  result joins only its own request.
- `resource_exhausted` and `stage_limit_exceeded` separate by one rule: the
  limits type. Only the type-environment ceilings change code.
- Bounded corpora keep their meaning. Their expected identities change once,
  under `1-draft.2`. Claims over unranged `Int` settle `requires-bound` on
  Kani, as C-22 already requires.
- The QSL-42 and QSL-43 ticket bodies still describe the native-v1 seam
  (`NativeModelProfile`, `src/temporal`, "re-vendor") and name IR-89
  (contract-ir#109) as the temporal IR form's source. ADR-011 and this record
  govern: the work lands on the spine, definitions are referenced and never
  vendored, and the temporal IR intake is IR-7.

## Alternatives Considered

- **New kernel `CollectionBound` and `PopulationBound` enums.** Rejected. They
  would add types to AD-016's closed kernel row, a QSpec amendment, for a
  distinction `Option` carries with one stated rule (`None` is unbounded).
- **A liveness or quantifier capability kind.** Rejected. QSpec FR-290 has
  none, and QSpec places these as obligation forms inside a claim. The (kind,
  mode) pair plus the arm's form check express them.
- **A default or a profile ceiling as the available finite bound.** Rejected.
  `requires-bound` would then imply a bound nobody chose.
- **`requires-bound` only when the caller already supplied bounds.**
  Rejected. The caller would never learn that a bound would help, and a
  resubmitted request would classify `Unbounded` again.
- **A `route` guard that re-checks a `supported` settlement against the
  extent.** Rejected. QSpec FR-290-AC-6 makes it `negotiate_*`'s obligation,
  `route` reads dispositions only (FR-075), and the §6 identity rule already
  keeps a bounded result off the unbounded item.
- **Lift the native-v1 ceilings through `NativeModelProfile`, as the QSL-42
  body says.** Rejected. ADR-011 §10 scenario 3 forbids extending
  `NativeModelProfile`, and that lane retires with SEAM-1.
- **Classify limits by the stage that reads them.** Rejected. It would
  reclassify QSpec accounting-contract meters that S3 charges (QSpec FR-150),
  which is QSpec's decision, not QSL's.

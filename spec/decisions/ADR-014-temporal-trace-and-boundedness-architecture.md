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
  - target: ix://agent-ix/quire-specification/FR-144
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-090
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
nothing.

## Context

Measured on QSpec `main` at `eb4234f` and QSL `main` at `fc27aacc`.

- **QSpec, unbounded declarations** (QSpec#113, merged as `5413ba6`). FR-144:
  the collection cardinality bound is optional, an absent bound means
  unbounded, and bound presence is part of collection type identity
  (FR-144-AC-9, AC-12, AC-13). AD-013 (accepted) states one rule with no
  compatibility layer. FR-153-AC-9: a population with no declared maximum is
  unbounded. FR-228-AC-5: a loop with a proved invariant and a well-founded
  variant is admitted with no finite maximum. FR-346, FR-347 and FR-348
  (V1-EXPR-027 to 029): unbounded quantification, induction and composition
  are negotiated per item, and settle `unsupported` with a warning when no
  capable backend is registered. The value and model root definitions
  `quire.value.complete/v1` and `quire.model.complete/v1` are at `1-draft.2`,
  because the collection-type identity preimage changed.
- **QSpec, infinite traces** (QSpec#112, on `main`). FR-090-AC-7: selecting
  `quire.temporal.infinite-trace/v1` admits the unbounded temporal operator
  forms, and no other profile does. FR-161: finite prefixes, absence of a
  counterexample and resource exhaustion never prove infinite satisfaction; a
  counterexample is a finite prefix plus a loop (a lasso) or another complete
  witness the provider declares; with no liveness backend an unbounded
  formula settles `unsupported` with a warning. FR-341 maps the infinite-trace
  results `proved`, `refuted`, `inconclusive`, `unsupported` and `failed`
  onto the FR-241 to FR-243 axes. FR-255 keys an interval by (`a`, `b`,
  selected profile identity, clock binding); an unbounded interval is not an
  absent bound. The shared grammar admits `[a,*]` only under the
  infinite-trace profile.
- **QSpec, capabilities.** FR-290 has ten kinds and two modes, `bounded` and
  `unbounded`. It has no liveness kind and no quantifier kind: a temporal
  claim under either profile is `temporal-satisfaction`, and quantification,
  induction and composition are obligation forms inside a claim that the
  selected backend's negotiation arm settles. The advertised-mode table:
  `requires-bound` for an unbounded extent on a bounded-only candidate when a
  finite bound is available, and `unsupported` with
  `unsupported_projection`/`unbounded-extent` when none is.
- **QSpec, diagnostics** (`quire.native.diagnostics/v1` `1-draft.7`).
  `stage_limit_exceeded` covers the limits of S1 to S4, the I2 reader, a
  family `check`, `replay` and `route`. `resource_exhausted` is the caller's
  work-budget meter, and "a semantic maximum is not a caller work budget".
  There is no timeout, solver-absent or bound-exhausted code: those are FR-331
  results.
- **QSL code.** Bounds today: kernel `CardinalityBound{minimum, maximum}`
  (both mandatory, `quire-exact/src/collection.rs:75`),
  `ValueType::Population(u64)` (`quire-exact/src/value.rs:205`), and the
  mandatory `[uint, uint]` collection bound in the grammar
  (`qsl-cst/src/grammar.rs:419-429`), refused when missing at
  `qsl-semantics/src/check/type_form.rs:149-166`. Limits: F
  `diagnostic::LimitExceeded`/`LimitKind` (T-4), kernel
  `Meter`/`Incomplete`/`quire_exact::LimitKind`. Capability: `Capability`
  (`qsl-semantics/src/check/capability.rs`), `qsl_route::Mode`,
  `BackendDescriptor`, `Registry::candidates` and `routing::Disposition`.
  Outcomes: `qsl_replay::proof_result::{ProofCategory, IncompleteCause,
  UnavailabilityCause, InconclusiveCause}`. Replay:
  `WitnessEnvelope<P: FamilyPayload>` with an opaque `TracePosition`.
  Finite exploration: `qsl_eval::simulation::explore::Outcome{Exhaustive,
  Bounded{frontier, limit}, Cancelled}` and `SampleProvenance{seed}`. The
  bounded temporal evaluator (`src/temporal`) and the native-v1 ceilings are
  SEAM code that retires in ADR-011 M-6c. No infinite-trace code exists.

## Decision

Item ids `B-`, `A-`, `TR-` and `N-` are local to this record. Other artifacts
cite them as `ADR-014 B-n`.

### 1. Bound taxonomy (Q222-1)

Six kinds exist. Each is its own type with one owner. No value of one kind
converts into another kind, except by the two derivations listed after the
table.

| ID | Kind | Meaning | Type and owner | Supplied by | When reached |
| --- | --- | --- | --- | --- | --- |
| B-1 | Authored semantic bound | Part of the meaning of a declaration: which values the type admits | Kernel (`quire-exact`): `CollectionBound{Bounded(CardinalityBound), Unbounded}` for collections, `PopulationBound{AtMost(u64), Unbounded}` for `ValueType::Population`, and the `BoundedInteger` range of a `bounded_domain`. TemporalTrace family (layer 3): `TemporalInterval` (§3). The S3 checker builds them from forms | the source author | A value outside the bound is refused `cardinality_out_of_bound` or `BoundViolation` (FR-144). It is never a resource outcome |
| B-2 | Execution resource bound | How much work one evaluation may spend | Kernel `ScalarLimits`, `Meter`, `quire_exact::LimitKind`, `Incomplete` under `quire.value.accounting/v1`; FR-323 `limits` | the caller, per run | `Outcome::Incomplete` with its charge point, catalog `resource_exhausted` (or `cancelled`). O-16 category `incomplete` |
| B-3 | Stage limit | How much input and work a compiler stage or reader may take | F `diagnostic::LimitExceeded` and F `LimitKind` (ADR-013 T-4, FR-096) | the caller of the stage, or the published default of that stage's limits type | `StageFailure::Limit(LimitExceeded)`, catalog `stage_limit_exceeded`. It is a stage failure, not an outcome category |
| B-4 | Proof bound | The finite domain over which a bounded-mode backend is asked to prove an item | QSL: `ProofBound`, one per unbounded domain of the item, in the request representation (ADR-013 O-20, built in QSL-140). On the wire: FR-331 request `domains`. In replay: `DeclaredDomain`, whose `domain` becomes the typed `ProofBound` | the caller of the request, explicitly | Not reached. It is part of the obligation identity (O-09), and a result qualifies only over it |
| B-5 | Backend tool budget | How long and how deep a backend tool runs (Kani unwind, solver time) | IR `ResourceBounds` and the AD-016 Kani tool pin. Not a QSL type | the backend provider | FR-331 result `incomplete` with `IncompleteCause::{TimedOut, ResourceExhausted}` |
| B-6 | Profile ceiling | A maximum that a selected profile fixes for every package | None on the spine. The native-v1 ceilings (`MAX_SEQUENCE_ITEMS` at `src/native_model/admission.rs:18`, `checked_handoff::MAX_POPULATION`, `native_temporal` `MAX_*`, `NativeModelProfile`) are lane-private (ADR-013 §6) and retire with SEAM-1 and M-6c | none | Native-v1 admission keeps refusing until it retires. No spine stage reads a profile ceiling |

Derivations. There are exactly two:

1. **B-1 → B-4.** A bounded authored domain is its own proof domain. A
   bounded item needs no `ProofBound` (ADR-013 C-25).
2. **B-4 → a new bounded request.** A `ProofBound` makes a new bounded
   request with its own obligation identity (ADR-012 §1.1, AD-016 arrow 5).

No B-2, B-3, B-5 or B-6 value ever becomes a B-1 or B-4 value. A meter that
runs out does not bound a type. A stage ceiling does not narrow a declaration.
A tool budget does not narrow a domain. A default never stands in for a proof
bound.

Classification rule for a limit that is not obviously in one row: a ceiling
that a compiler stage, reader, `replay` or `route` reads is B-3. A budget that
S6a evaluation charges is B-2. On this rule, the FR-082 `ancestor_steps` and
`work_units` ceilings that the check-stage type environment reads are B-3, and
the same ceilings in S6a population admission are B-2. §12 amends FR-082 to
match.

The replay envelope's `proof_bounds: ScalarLimits` member
(`qsl-replay/src/witness.rs:346`) holds the proving run's accounting limits.
That is a B-2 value, not a proof bound. QSL-140 renames it `run_limits`, and
types `DeclaredDomain.domain` (today a `String`) as `ProofBound`. The proof
bound of a packet is its declared domains.

### 2. Absent bounds (Q222-2, first part)

Every absent bound means exactly one of three things: unbounded, inherited or
invalid. No bound in QSL is "unspecified".

| Where the bound is absent | Meaning | Rule |
| --- | --- | --- |
| Collection cardinality `K<T>` | unbounded | FR-144. `CollectionBound::Unbounded`. It is part of type identity. `K<T>` and `K<T>[a,b]` are different types |
| Population maximum | unbounded | FR-153-AC-9. `PopulationBound::Unbounded` |
| Integer range (`Int` with no `[a,b]`) | unbounded | The kernel `Integer` is unbounded. Already the rule |
| Loop maximum | invalid, unless the loop carries a proved invariant and a well-founded variant | FR-228-AC-5, FR-171. An unproved termination claim is refused or inconclusive, and never silently bounded |
| Temporal interval under a bounded profile | invalid | S3 TemporalTrace `check` refuses it (FR-091, FR-092) |
| Temporal interval under `quire.temporal.infinite-trace/v1` | unbounded | A bare unbounded operator has no interval. `[a,*]` has an interval with an unbounded upper end. FR-255: neither is an absent bound that defaults |
| Execution resource limit (B-2) or stage limit (B-3) | inherited | The caller gets the published default of that limits type (for example NFR-011, NFR-012). A limit is never unbounded at its point of use |
| Proof bound (B-4) for an unbounded domain | none available | The predicate in §4 returns false. The item settles `unsupported`, warned, `unsupported_projection`/`unbounded-extent`, on a bounded-only candidate |
| Extent classification of a requested item | invalid | `invalid-request`, `invalid_capability`/`absent-extent` (FR-290, FR-057) |

### 3. Trace concepts (Q222-2, second part)

| ID | Concept | Owner and type | Rule |
| --- | --- | --- | --- |
| TR-1 | Trace identity | No new identity or digest domain. A counterexample trace is identified by its packet (obligation identity O-09, occurrence key O-07) and compared lexically over its canonical encoding, as O-25 compares a transcript. A sampled trace is (seed, sampler `DefinitionRef`, trace index) under `quire.simulation.sampler/v1`, carried by `qsl_eval::simulation::trace::SampleProvenance`. An observation trace handed to S6a is an input value, not an identity | A trace never takes its identity from arrival order or storage position (ADR-013 R-05) |
| TR-2 | Trace position | TemporalTrace family (layer 5 evaluator, layer 3 checked form): `TemporalPosition{index: u64}`, the zero-based index into the trace as represented, prefix first and then loop. It is carried in `qsl_replay::TracePosition` as the decimal ASCII of `index` with no leading zero. Replay stores it and does not read it (O-25) | Only the TemporalTrace family decodes it. A position past the end of the represented trace refuses at reconstruction |
| TR-3 | Interval | TemporalTrace family (layer 3 check): `TemporalInterval{lower: u64, upper: IntervalUpper}`, `IntervalUpper{Finite(u64), Unbounded}`. It has one validated constructor, which takes the selected temporal profile. It refuses `lower > upper`, and it refuses `Unbounded` unless the profile is `quire.temporal.infinite-trace/v1`. An operator holds `Option<TemporalInterval>`, and `None` is admitted only for a bare operator under the infinite-trace profile. The interval key is FR-255's (`lower`, `upper`, profile identity, clock binding), and the interval is identified by the checked node id of the operator that carries it (O-04) | Bounded, unbounded-upper and absent intervals are three distinct values. None defaults to another |
| TR-4 | Horizon | Derived, not stored. The horizon of a bounded-profile formula is the greatest reach of its finite intervals, computed by the TemporalTrace `check`. Overflow of that sum refuses at check. An infinite-trace formula has no horizon | A horizon is a B-1 consequence, never a budget |
| TR-5 | Evaluation budget | The kernel `Meter` (B-2). The S6a TemporalTrace evaluator charges `quire_exact::LimitKind::WorkUnits`, one unit per (temporal node, position) visit | Running out yields `Incomplete`, never a truth value. The native `quire.native.temporal-work/1` counters retire with M-6c |
| TR-6 | Seed | `SampleProvenance.seed` in `qsl_eval::simulation` | Same seed and same sampler `DefinitionRef` reproduce the same traces |
| TR-7 | Frontier | `qsl_eval::simulation::frontier::Frontier` and `explore::Outcome::{Bounded{frontier, limit}, Cancelled{frontier}}` | An exploration that stops early reports its frontier and never claims exhaustive success (QSpec FR-181) |
| TR-8 | Liveness capability | No new type. It is the FR-290 pair (`temporal-satisfaction`, `unbounded`) in `BackendDescriptor.advertises`, plus the backend's CG `negotiate_*` arm accepting the infinite-trace IR form | §6 |

### 4. Extent, mode and the available finite bound (Q222-3)

Three concepts are three types:

- **Finite** is a property of a concrete value or a concrete trace: a kernel
  `Value`, or a trace with a known number of positions. It is never an extent.
- **Extent** is a property of a requested item. `ClaimExtent{Bounded,
  Unbounded}` is built by QSL-140 in the layer-3 family core
  (`qsl-semantics::family`), beside `Requirements`. It is named `ClaimExtent`
  because `model::Extent{Closed, Open}` already names FR-153 population
  closure, which is a different concept. The wire spelling is FR-290's
  `bounded`/`unbounded`.
- **Mode** is a property of a backend advertisement: `qsl_route::Mode{Bounded,
  Unbounded}`, which already exists.

**Extent rule.** An item's extent is `Unbounded` when any domain it ranges over
is unbounded. The domains are: a collection with `CollectionBound::Unbounded`,
a population with `PopulationBound::Unbounded`, an unbounded integer argument,
a loop admitted under FR-228-AC-5, and a temporal formula under the
infinite-trace profile. Otherwise the extent is `Bounded`. The family that
owns the claim form computes it in `requirements()` (ADR-012 §2). `Requirements`
holds the item's one `Capability`, its `ClaimExtent` and its authored bounds
(B-1).

**Available finite bound predicate.** A finite bound is available for an item
exactly when the request carries a `ProofBound` for every unbounded domain of
that item. The only source of a `ProofBound` is the caller's request. A B-2,
B-3, B-5 or B-6 value, a default, a profile or a registered backend never
makes one available. With the predicate true, a bounded-only candidate settles
`requires-bound`. The caller then submits the bounded request that the
`ProofBound` makes (derivation 2), under its own obligation identity. With the
predicate false, it settles `unsupported`, warned, `unbounded-extent`. The
ADR-012 §1.1 mechanics are otherwise unchanged.

### 5. The `quire.temporal.infinite-trace/v1` facet (ticket decision 4)

| ID | Rule |
| --- | --- |
| A-1 | **Parse.** S1 and S2 parse the bare unbounded operators and the `[a,*]` interval under every profile. Parsing selects no meaning. |
| A-2 | **Admit.** The S3 TemporalTrace `check` admits an unbounded form only when the unit's temporal profile selection is `quire.temporal.infinite-trace/v1` (FR-090-AC-7). Otherwise it refuses `unsupported_construct`/`expression-form`, located at the operator (ADR-012 §3 "unbounded without facet"). A unit selects exactly one temporal profile (FR-090). The infinite-trace profile uses the event-position sequence authority without false-extension closure (FR-250-AC-6). |
| A-3 | **Record.** An admitted infinite-trace clause yields `Requirements{kind: temporal-satisfaction, extent: Unbounded}`. |
| A-4 | **Execute.** S6a never produces `proved` or a true settlement for an infinite-trace formula. Over a finite trace it returns violation only for a safety formula whose bad prefix is complete. Otherwise it returns pending, which is O-16 `inconclusive` (FR-161, FR-161-AC-2, FR-324-AC-2). Over a lasso (a finite prefix plus a loop) it evaluates exactly, since the trace is ultimately periodic. The work is bounded by (prefix + loop length) × formula size under TR-5. That is replay of a given trace, not a liveness solver. |
| A-5 | **Discharge.** Proof of an infinite-trace claim goes only through negotiation (§6). The results map through FR-341 to O-16: `proved` → success, `refuted` → violation, `inconclusive` → inconclusive, `unsupported` → unsupported, `failed` → incomplete when resource-incomplete, otherwise internal failure. |

### 6. Capability negotiation for liveness and quantifier-capable backends (ticket decision 5)

No new capability kind, flag or mode is added. QSL records; CG settles.

1. QSL S3 records `Requirements{kind, extent, bounds}` per item (§4).
2. `qsl_route::Registry::candidates` matches on kind alone (FR-075).
3. CG `negotiate_*` applies the FR-290 advertised-mode table to the extent,
   then the candidate's arm decides whether it discharges the IR obligation
   form (quantifier, induction, composition, infinite-trace formula). If a
   candidate advertises (kind, `unbounded`) but its arm does not discharge the
   form, the item settles `unsupported`, warned,
   `unsupported_projection`/`unsupported-requested-capability`, naming the
   required capability.
4. **QSL guard.** `qsl_route::routing::route` refuses to route an item with
   `ClaimExtent::Unbounded` settled `supported` on a candidate whose
   descriptor does not advertise (kind, `Unbounded`). The refusal is
   `invalid_capability`/`inconsistent-candidates`. QSL therefore cannot
   forward a narrowed claim even if a settlement arrives malformed.
5. **Kani.** Kani's manifest advertises its kinds with mode `bounded` only.
   For an unbounded item on a Kani-only registry, Kani is therefore never
   `supported`. It settles `requires-bound` when the predicate in §4 holds,
   and `unsupported`, warned, `unbounded-extent` when it does not. A Kani
   `proved` on the resulting bounded request carries that request's
   obligation identity, which includes the `ProofBound` (O-09). It never
   joins the unbounded item's `request_index`, so it cannot be read as an
   answer to the unbounded claim.

### 7. Structured outcomes (ticket decision 6)

Every case already has a type. This table fixes which one.

| Case | Where it is settled | Type | Catalog and O-16 category |
| --- | --- | --- | --- |
| Unsupported backend: empty candidate set, an arm that does not discharge the form, or an unbounded extent with no proof bound | CG negotiation | `qsl_route::routing::Disposition::Unsupported` (from FR-331 `dispositions`) | `unsupported_projection` with `unsupported-requested-capability` or `unbounded-extent`; unsupported |
| `requires-bound` | CG negotiation | `Disposition::RequiresBound` | FR-331 disposition; unsupported |
| Solver or tool absence | the probe of a routed backend (ADR-012 §7.4) | FR-331 result `unsupported`, `UnavailabilityCause::SolverAbsent` | `unsupported_projection`/`tool-unavailable`; unsupported |
| Bound exhaustion, S6a | S6a meter | `Outcome::Incomplete` | `resource_exhausted`/`insufficient-next-charge`; incomplete |
| Bound exhaustion, finite exploration | `qsl_eval::simulation::explore` | `Outcome::Bounded{stats, frontier, limit}` | incomplete; never success and never violation |
| Bound exhaustion, backend | backend | `IncompleteCause::ResourceExhausted` | FR-331 `incomplete`; incomplete |
| Stage ceiling | S1 to S4, I2, `check`, `replay`, `route` | `StageFailure::Limit(LimitExceeded)` | `stage_limit_exceeded`; a stage failure, not an outcome |
| Timeout | backend | `IncompleteCause::TimedOut` | FR-331 `incomplete`; incomplete |
| Cancellation | backend, S6a or exploration | `IncompleteCause::Cancelled`, `explore::Outcome::Cancelled{frontier}` | `cancelled`/`caller-cancelled`; incomplete |
| Semantic violation | S6a or backend | `Completed(false)` of a claim; FR-331 `refuted` with its counterexample | violation |
| Unresolved liveness on a finite prefix | S6a | pending truth | inconclusive |

The table adds one mapping that no record states: `explore::Outcome` to O-16.
`Exhaustive` is success. `Bounded` and `Cancelled` are incomplete, and each
keeps its frontier. QSL-140 adds that map to F `diagnostic` beside the other
O-16 maps.

### 8. Bounded reference runtime (ticket decision 7)

- Bounded evaluation is unchanged: FR-145 occurrence visiting, the bounded
  temporal profiles, and finite exploration. Every existing bounded corpus
  keeps its results.
- S6a evaluating a concrete value of an unbounded type is exact for that value.
  It never refuses for cardinality (FR-144-AC-12). It stops only on B-2.
- A result of S6a, of finite exploration or of a bounded backend request is
  evidence only for what it evaluated: one input, one explored bound, or one
  `ProofBound`. Its identity carries that scope. It is reported `tested`
  (O-16), or `proved` qualified by its obligation identity's domains, and it
  never answers an unbounded claim. No consumer reads an `Exhaustive`
  exploration, a `tested` result or a bounded `proved` as proof of an item
  whose extent is `Unbounded`.
- The native-v1 ceilings (B-6) are not lifted. Native-v1 admission keeps
  refusing unbounded declarations (`UnrepresentableConstraint`) until SEAM-1
  retires. ADR-011 §10 scenario 3 already forbids extending
  `NativeModelProfile`.

### 9. Version and package effects; compatibility (ticket decision 8)

| ID | Effect |
| --- | --- |
| N-1 | The collection-type identity preimage is QSpec's root definitions at `1-draft.2`. QSL selects exactly that revision and refuses any other (ADR-013 R-08, O-22). QSL-42 (V1-A12) is the ticket in which QSL adopts the `1-draft.2` collection identity preimage. QSL references the definitions by revision and digest and copies none of them. |
| N-2 | Every package with a collection type gets new node ids and a new `package_id` under `1-draft.2`. That is correct: the preimage changed. Bounded corpora keep their source spelling and meaning, and their expected identities are regenerated, not migrated. |
| N-3 | The mandatory interval under the bounded profiles is the existing rule. The infinite-trace forms add `Option<TemporalInterval>` and `IntervalUpper::Unbounded` inside the checked temporal form. The v2 contract stays `quire.checked-package/v2`. While v2 is prerelease, QSpec revises its node and operation set in place (ADR-013 QC-19 precedent). |
| N-4 | Compatibility: none is needed. This is pre-release with no users. There is one rule for absent bounds (§2) and no second spelling. No reader for `1-draft.1`, no bounded-by-default reading of `K<T>`, and no adapter exist. |

### 10. Change scenarios

1. **Unbounded `eventually`, with and without a liveness backend.** S2 parses
   `eventually p`. The S3 TemporalTrace `check` admits it because the unit
   selects `quire.temporal.infinite-trace/v1` (A-2). Under a bounded profile
   it refuses `unsupported_construct` instead. `requirements()` returns
   (`temporal-satisfaction`, `Unbounded`) (A-3). With no backend registered,
   `Registry::candidates` is empty and `negotiate_*` settles `unsupported`
   with the warning naming `temporal-satisfaction` (FR-161-AC-7). With a
   backend advertising (`temporal-satisfaction`, `unbounded`) whose arm takes
   the infinite-trace form, it settles `supported`, `route` routes it, and
   the FR-341 result maps through A-5. A registered bounded-only temporal
   backend settles `unsupported`, `unbounded-extent`: a temporal formula has
   no `ProofBound` (§4).
2. **Unbounded collection declaration with bounded native evaluation.**
   `Set<Account>` parses (the QSL-42 grammar change). S3 builds
   `CollectionBound::Unbounded`. S4 emits it under the `1-draft.2` preimage
   (N-1). S6a evaluates a concrete set of any size: construction never
   refuses for cardinality, it charges `collection.bound`, and it stops only
   with `Incomplete`/`resource_exhausted` when the caller's meter runs out
   (B-2). A claim over the declared type has extent `Unbounded`. Evaluating it
   on the concrete input is `tested` and never answers the claim (§8).
   Native-v1 still refuses the declaration (B-6).
3. **Claim over an unbounded collection with Kani.** `requirements()` returns
   (`value-validity`, `Unbounded`). The candidate is Kani, which advertises
   `bounded` only. With no `ProofBound` in the request, `negotiate_*` settles
   `unsupported`, warned, `unbounded-extent`, naming the kind, the candidate
   and its modes, and emits no harness. With a `ProofBound`, it settles
   `requires-bound`. The caller's bounded re-request is a new item with its
   own identity, and its Kani result never joins the unbounded item. If a
   malformed settlement said `supported`, `route` refuses it with
   `inconsistent-candidates` (§6 step 4). The claim is never narrowed.
4. **Finite exploration that exhausts its bound.**
   `qsl_eval::simulation::explore` stops at `Limits{max_states, max_depth,
   max_transitions}` and returns `Outcome::Bounded{stats, frontier, limit}`.
   O-16 maps it to incomplete (§7). The frontier and the reached limit are
   kept. No exhaustive success and no verdict is reported. The run's seed is
   in `SampleProvenance` (TR-6), so the same run reproduces.
5. **Replay of a temporal counterexample.** A backend returns FR-331
   `refuted` with a lasso. The #231 envelope `WitnessEnvelope<P>` carries the
   obligation identity, occurrence key, `package_id`, profile selections
   (including the temporal profile), `trace_position` (TR-2) and a
   TemporalTrace `FamilyPayload`, `TemporalCounterexample{prefix, loop,
   interval}`. `interval` is the failing operator's `TemporalInterval` and its
   FR-255 key. At ADR-011 E9 the layer-6 `replay` facade recompiles from
   digest-addressed source and resolves the occurrence key to the operator
   node. It refuses if the recompiled operator's interval key or the profile
   selection differs from the packet's. The TemporalTrace evaluate hook then
   re-evaluates the formula over the lasso at `TemporalPosition` (A-4).
   Agreement settles `reproduced-with-evaluated-witness`. Disagreement settles
   `inconclusive` with a typed cause (O-27).

### 11. Named interfaces for the dependent tickets

**QSL-140 (ADR-013 §7 S-6)** builds:

- kernel `CollectionBound{Bounded(CardinalityBound), Unbounded}` in
  `quire-exact/src/collection.rs`;
- kernel `PopulationBound{AtMost(u64), Unbounded}`, which replaces the `u64`
  in `ValueType::Population`;
- `ClaimExtent{Bounded, Unbounded}` and `ProofBound` in
  `qsl-semantics::family`;
- the typed `DeclaredDomain.domain: ProofBound`, and the rename of
  `WitnessEnvelope.proof_bounds` to `run_limits` in `qsl-replay`;
- the `explore::Outcome` → O-16 map in F `diagnostic`;
- the FR-082 check-time limit reclassification to `LimitExceeded` (§12);
- O-19 `Capability` and the O-20 request representation, as ADR-013 already
  lists.

**QSL-42 (V1-A12)** builds against those types:

- makes the collection bound optional at the grammar
  (`qsl-cst/src/grammar.rs:419-429`), in `qsl-forms/src/value.rs` and in
  `check/type_form.rs`;
- admits `PopulationBound::Unbounded`;
- adopts the `1-draft.2` preimage (N-1);
- computes `ClaimExtent` in each claim form's `FamilyContract::requirements()`
  (the method QSL-152 adds);
- adds the `route` guard (§6 step 4).

It does not touch `NativeModelProfile` or the native-v1 ceilings (B-6).

**QSL-43 (V1-A11)** builds:

- `TemporalInterval`, `IntervalUpper` and the infinite-trace admission (A-2)
  in the S3 TemporalTrace family `check`;
- `TemporalPosition`, `TemporalCounterexample: FamilyPayload` and the lasso
  evaluation (A-4) in the layer-5 TemporalTrace evaluator, which ADR-011 §6.2
  places under `value::expression` and which replaces `src/temporal` in M-6c.

Both tickets settle their exit cases through `Registry::candidates`,
CG `negotiate_*` (agent-ix/quire-contract-codegen#86) and `routing::route`.
Neither needs a new ownership decision.

### 12. Amendments made with this record

- ADR-013: Q222-1 to Q222-3 answered in its §8 question table, O-20's owner row and O-21 point
  here, and the S-6 row names its bound types.
- ADR-012 §1.1: the "available finite bound" predicate is §4 of this record.
- FR-057: the `requires-bound` row cites §4.
- FR-082: the check-stage `ancestor_steps` and `work_units` ceilings are stage
  limits (B-3), reported `stage_limit_exceeded` with limit kinds nesting depth
  and work budget. This is the rule FR-096 already states for every S3
  ceiling. The S6a population-admission walk keeps `resource_exhausted`
  (B-2).
- `spec/spec.md`: index row and relationship.

### 13. Open dependency

One. QSpec has no v2 operation identities for temporal operators or
intervals: the operation catalog has `quire.op.temporal.clause` only, and the
v2 `TemporalNode` body is unconstrained. S4 emission of any temporal formula,
bounded or infinite-trace, waits for that spelling, and so does IR v2
temporal intake (Linear IR-7). QSL-43's S3 admission and its S6a evaluation
and replay do not wait for it. There is no open STD-12 dependency: QSpec#113
is fully on QSpec `main`.

## Consequences

- QSL-140 can build its bound types, and QSL-42 and QSL-43 can build against
  them, with no further ownership question.
- A claim is never narrowed silently. Kani's bounded-only advertisement is
  explicit in its manifest, is enforced by `negotiate_*`, and is checked again
  by `route`.
- `resource_exhausted` and `stage_limit_exceeded` separate by one rule (stage
  versus S6a). FR-082 changes code at check time.
- Bounded corpora keep their meaning. Their expected identities change once,
  under `1-draft.2`.
- The QSL-42 and QSL-43 ticket bodies still describe the native-v1 seam
  (`NativeModelProfile`, `src/temporal`, "re-vendor"). ADR-011 and this
  record govern: the work lands on the spine, and definitions are referenced,
  never vendored.

## Alternatives Considered

- **`Option<CardinalityBound>` in the kernel.** Rejected in favour of a named
  enum. Absence carries a meaning (unbounded) that is part of type identity,
  and a named variant keeps a missing value from being read as "not yet
  known".
- **A liveness or quantifier capability kind.** Rejected. FR-290 has none, and
  QSpec places these as obligation forms inside a claim. The (kind, mode)
  pair plus the arm's form check express them.
- **Use a default or a profile ceiling as the available finite bound.**
  Rejected. It would make `requires-bound` imply a bound nobody chose, which
  is silent narrowing by another route.
- **Lift the native-v1 ceilings through `NativeModelProfile`, as the QSL-42
  body says.** Rejected. ADR-011 §10 scenario 3 forbids extending `NativeModelProfile`, and
  that lane retires with SEAM-1.
- **Keep the check-stage ancestor ceiling as `resource_exhausted`.** Rejected.
  It contradicts FR-096 and ADR-013 T-4, and it would give one ceiling two
  codes depending on the caller.

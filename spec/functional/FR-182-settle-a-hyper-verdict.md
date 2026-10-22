---
id: FR-182
title: "Settle a hyper or step-relation verdict and its causes"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-179
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: depends_on
---
# FR-182: Settle a hyper or step-relation verdict and its causes

## Description

QSL SHALL settle every HP-1, HP-2, HP-3 and HP-6 item, and every HP-4 item,
as exactly one FR-331 terminal record through FR-127's map, with ADR-018's
verdict kinds V-1 to V-8 and no new label, basis, category or `ProofBasis`
member (ADR-023 HV-1 to HV-8). `InconclusiveCause` gains `MatchUndetermined`
and `VacuousMatch`. An item whose claim evaluates undefined on a tuple
settles `refuted` with cause `UndefinedEvaluation{where, cause}` naming the
tuple (ADR-018 UE-1, UE-2). HP-5 items settle by the possible family's map
(FR-181). Settlement is `qsl-replay`'s (ADR-029 CB-2): an EN-1 `proved`
settles only after the product-closure certificate checker (FR-163)
accepts its certificate, and a proof whose method has no certificate
checker settles `proved` labelled `Uncertified` (ADR-023 HV-1, HX-7).

## Use case

A verification operator reads the result of a noninterference check. A
proof says it covered every matched pair exhaustively, or under copy-swap.
A refutation counts only after its tuple of traces replays. A clause whose
match no pair of runs satisfies, or whose induction proof came with no
replayed pair of runs that the match accepts, reads `inconclusive`,
`VacuousMatch`, never `proved`, and a run that hit `max_witness_set` names that limit.

## Inputs

- A `ModelCheckOutcome` from FR-176, FR-177, FR-178 or FR-179, which
  `settle_hyper_check` settles, or an SMT outcome for HP-2 from CG's EN-2 or
  EN-3 (ADR-023 HC-6, HC-7), which CG hands over as a `HyperSmtOutcome` to
  `settle_hyper_smt` (Outputs). Every SMT outcome for an HP-2 item goes
  through `settle_hyper_smt`, and none through `settle_hyper_check`, so
  each one passes the identity check below.
- For `Holds` from EN-1: the `ProductClosureCheck` of its certificate
  (FR-163).
- For `Violated`: the FR-072 replay result of its `HyperCounterexample`
  (FR-183), or the `ReplayRefusal` that stopped it.
- For an SMT `Inductive{depth}` or `InductionNotClosed{depth}` outcome for
  HP-2: the item's `HyperQueryRequest` (FR-315), the outcome's
  `HyperProductIdentity` (FR-315 "Product identity"), whose `instance()` is
  the outcome's instance, and an optional `MatchWitness` (FR-183), present
  when a non-vacuity witness came with the outcome. The caller passes the
  witness, never a `MatchWitnessCheck`: QSL checks the witness itself.
- For an item negotiation did not route: its negotiation disposition.

## Outputs

```rust
// InconclusiveCause gains:
//   MatchUndetermined,   // μ refused or incomplete at a reachable joint step
//   VacuousMatch,        // non-empty μ_U with no matched fair universal tuple
//                        // established: EN-1 found none (ADR-023 HM-7), or an
//                        // SMT induction proof came with no accepted match
//                        // witness for its instance (FR-183)

// HyperSettleRefusal gains:
//   ProductMismatch { member: ProductMember }, // an HP-2 SMT outcome whose
//                        // HyperProductIdentity is not the item's
//   ItemIdentity(SmtEncodingRefusal),          // the item's own identity
//                        // refused (FR-315 "Refusals and their order")
pub enum ProductMember { Clause, Subjects, Instance }

//   LimitNotApplicable,                        // a `LimitStopped` naming a limit
//                        // that does not bound the SMT run
//   QueryKind,                                 // an `Unrolling` query was
//                        // refused: an HP-2 item has none (FR-315)

pub enum HyperSmtResult {
    Inductive { depth: u64 },          // EN-3: base and step both unsatisfiable
    InductionNotClosed { depth: u64 }, // EN-3: base unsatisfiable, step satisfiable
    Violated {                         // a counterexample, for QSL's own FR-183 replay
        counterexample: Box<HyperCounterexample>,
        method: SmtMethod,             // the query that produced it
    },
    BoundReached { depth: u64 },       // EN-2: searched to `depth`, no replayable lasso
    Unknown { depth: u64, method: SmtMethod }, // the solver did not decide a query at `depth`,
                                       // or stopped for a resource no request limit names
    TimedOut { method: SmtMethod },    // the backend's own wall-clock limit expired
    LimitStopped { limit: ReachedLimit, method: SmtMethod }, // a stop under a request limit that bounds the SMT run
    BackendFault { method: SmtMethod }, // the solver crashed or its output did not parse
    Cancelled { source: CancelCause, method: SmtMethod }, // the run's cancellation handle tripped
    EncodingRefused { refusal: SmtEncodingRefusal }, // `encode_hyper_query` refused: always EN-3
}

pub enum SmtMethod {
    KInduction,   // an EN-3 base or step query, method `k-induction`
    SmtUnrolling, // an EN-2 query, method `smt-unrolling`
}

pub struct HyperSmtOutcome {
    pub result: HyperSmtResult,
    pub product: HyperProductIdentity, // the `product` of the queries it answers
}

pub fn settle_hyper_smt(
    item: &HyperQueryRequest<'_>,      // the item's clause, subjects and instance
    outcome: &HyperSmtOutcome,
    witness: Option<&MatchWitness>,
    replay_limits: ReplayLimits,       // FR-183's limits for the witness check and the replay
    limits: &ModelCheckLimits,
    requested: &[ModelCheckLimit],
) -> Result<HyperRecord, HyperSettleRefusal>;
```

`HyperSmtResult`, `HyperSmtOutcome` and `settle_hyper_smt` SHALL be types
and an entry of the `qsl-replay` facade, beside `settle_hyper_check`, and
CG calls `settle_hyper_smt` from its `replay` adapter module (FR-315
"Product identity"). A satisfiable base query is no `HyperSmtResult` by
itself: FR-315 "Reading the results" sends CG on to EN-2's search at that
depth, whose result is `Violated` or `BoundReached`. One exception: a
satisfiable base whose failure is an undefined letter, guard or `μ_U` is
HV-8 refuting evidence with no lasso (FR-315). CG concretises it into an
`Undefined` `HyperCounterexample` and returns it as `Violated` with method
`KInduction`. A `Lockstep` counterexample comes from EN-2's lasso search
with method `SmtUnrolling`.

and the FR-331 terminal record with its QSpec FR-360 label,
FR-243 basis, O-16 category and method.

## Behavior

- The map SHALL be FR-127's, exhaustive with no `_` arm, with these rows
  for hyper items:

| Verdict | Input | QSpec FR-360 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-1 | `Holds{Exhaustive}` or `Holds{Reduced{…}}` (copy-swap named, FR-174), proof basis `exhaustive` or `reduced` (QSpec FR-399), whose certificate check is `Accepted` | `proved` | `closed-scope` | `Proved{basis: Exhaustive, certification: Certified}` or `Proved{basis: Reduced{…}, certification: Certified}` | success |
| V-3 | SMT `k`-inductive for HP-2 with a safety body, every subject with an initial state, and an empty `μ_U` or an `Accepted` `MatchWitnessCheck` for the outcome's instance | `proved` | `decisive-witness` | `Proved{basis: Inductive{depth: k}, certification: Uncertified}` | success |
| V-4 | `Violated`, or SMT `Violated` with a `Lockstep` counterexample, whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-4 | `Violated`, or SMT `Violated` (method `k-induction` from a satisfiable base, or `smt-unrolling`), with an `Undefined` counterexample (QSpec FR-400) whose replay reproduces the undefined value at `trace_position` (FR-183) | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| V-5 | `BoundReached{depth}`; SMT `BoundReached{depth}` | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth})` | inconclusive |
| V-6 | `Undecided(MatchUndetermined)`, `Undecided(VacuousMatch)`, `UndecidedSuccessor`, `NoInitialState`, `NoInstance` (an item with zero instances, FR-125), `ReductionNotPreserving` (wire cause `reduction-not-preserving`); `Holds` whose certificate check is `Rejected` (`CertificateRejected{rule, at}`, ADR-018 PC-2); SMT `InductionNotClosed{depth}`; SMT `Unknown{depth}` (`InductionNotClosed{depth}`); SMT `EncodingRefused` with `match-conjunct` (`MatchUndetermined`); SMT `k`-inductive for HP-2 with a non-empty `μ_U` and no `Accepted` `MatchWitnessCheck` for the outcome's instance (`VacuousMatch`); `Violated` or SMT `Violated` whose replay settles `inconclusive` (`ReplayParity`, wire cause `replay-parity`) or refuses (`ReplayRefused`) | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | `Stopped(ResourceExhausted, limit)`, including `MaxWitnessSet` and `MaxRelationTuples` (FR-184); SMT `LimitStopped{limit}` | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7, FR-127) | incomplete |
| V-7 | `Stopped(Cancelled, None)`; SMT `Cancelled{source}` | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(Cancelled{source})`, written `cancelled{source}` (FR-127) | incomplete |
| — | SMT `BackendFault` | `failed` | `unavailable` | `Failed` | failed (ADR-013 O-16 internal failure) |
| V-7 | SMT `TimedOut` | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(TimedOut)`, written `timed-out` (FR-293, QSpec FR-331) | incomplete |
| V-8 | HP-4; a `behaviours` clause under a profile other than infinite-trace; SMT `EncodingRefused` with `hyper-body`, a field sort or an out-of-fragment construct | `unsupported` | `unavailable` | `Unsupported(unsupported-requested-capability)` | unsupported |

- A `Violated` outcome SHALL settle `refuted` only through its replay.
- An item whose body or `μ` evaluates undefined on a tuple SHALL settle
  `refuted`, through its replay, with cause `UndefinedEvaluation{where,
  cause}`, `where` naming the tuple (ADR-023 HV-8). The item SHALL settle on
  the first refuting evidence in canonical order, a false tuple or an
  undefined one.
- An HP-3 refutation SHALL carry basis `decisive-counterexample`: its
  evidence is the universal tuple, and replay recomputes the existential
  half over it (ADR-023 HX-4).
- A V-5 record SHALL state what the bound covers: for HP-2 and HP-3, no
  counterexample whose universal lasso has at most `k` joint steps; for
  HP-6, no counterexample of at most `k` moves; for HP-1, no counterexample
  whose executions all leave states at depth below `k`.
- A V-7 record SHALL name its limit, its value and the request member that
  raises it.
- Every record, whatever its verdict, SHALL state the value of each
  model-check limit the run used, FR-126's and FR-184's, with whether the
  request set it or the published default applied.
- The record SHALL carry the method (`explicit-state`, `smt-unrolling` or
  `k-induction`).
- A `proved` record SHALL carry ADR-018 PC-1's certification label: `Certified` when the
  product-closure certificate checker accepted its certificate, and
  `Uncertified` for a proof whose method has no certificate checker, which
  is V-3 from `k-induction`. An `Uncertified` proof SHALL keep the label
  `proved`.
- A certificate check that a limit stopped SHALL settle V-7 naming the
  limit.
- A verdict SHALL hold for exactly its subjects; the obligation identity
  binds them as FR-172 states.
- When `settle_hyper_smt` reads an outcome, it SHALL first, ahead of
  choosing any row, compute the item's identity as
  `HyperProductIdentity::of(item)` and compare it with `outcome.product`.
  When `of` refuses, it SHALL write no record and refuse
  `HyperSettleRefusal::ItemIdentity` carrying that refusal. When the two
  identities differ, the outcome is no evidence for the item: it SHALL
  write no record and refuse `HyperSettleRefusal::ProductMismatch`, naming
  the first member that differs in the order `clause`, `subjects`,
  `instance`.
- `settle_hyper_smt` SHALL check a supplied `witness` itself, by FR-183's
  "Match witness" rules, over the item's own clause and the `ModelSystem`
  of each of the item's subjects, under `outcome.product.instance()`, and
  bounded by `replay_limits`. The resulting `MatchWitnessCheck` is the one
  the rules below read. A witness checked over any other clause, subjects
  or instance is never an input, so the check is bound to the identity
  the outcome names.
- `settle_hyper_smt` SHALL settle each `HyperSmtResult`, after the identity
  check, as follows. The record's method is `k-induction` for `Inductive`
  and `InductionNotClosed`, `smt-unrolling` for `BoundReached`, the
  result's `method` for `Violated`, `Unknown`, `TimedOut`, `LimitStopped`,
  `BackendFault` and `Cancelled`, and
  `k-induction` for `EncodingRefused`, because `encode_hyper_query` builds
  only EN-3's base and step queries:
  - `Inductive{depth}` by the rules below;
  - `InductionNotClosed{depth}` as V-6 `InductionNotClosed{depth}`;
  - `Violated` only through QSL's own replay. When `counterexample.instance`
    differs from `outcome.product.instance()`, `settle_hyper_smt` SHALL
    write no record and refuse `ProductMismatch` naming `instance`.
    Otherwise it SHALL replay the counterexample itself, by FR-183's
    `Lockstep` or `Undefined` rules for its kind, over the item's own clause
    and the `ModelSystem` of each of the item's subjects, under that
    instance and bounded by `replay_limits`, with no caller-supplied replay
    and no recompile from a replay request. Reproduced is V-4 `refuted`
    (with `UndefinedEvaluation` for an `Undefined` counterexample), a
    disagreement is V-6 `ReplayParity`, a refusal is V-6 `ReplayRefused`,
    and a replay a limit stopped is V-7;
  - `BoundReached{depth}` as V-5 `BoundReached{depth}`, stating the HP-2
    reading of the bound;
  - `Unknown{depth}` as V-6 `InductionNotClosed{depth}`, the cause ADR-018
    V-6 defines as the induction not closed at `k`, whether a step
    counterexample exists or the solver did not decide a query; QSpec's
    `induction-not-closed` carries the same meaning. The cause means only
    that the induction is not closed at `k`;
  - a solver stop: CG SHALL report a stop as exactly one result, the first
    of these that applies, so no stop has two causes:
    1. `Cancelled{source}`, when the run's cancellation handle tripped
       (FR-276), a caller's deadline included;
    2. `TimedOut`, when the backend's own wall-clock limit expired. CG's
       HP-2 SMT arm runs under the `qsl-replay` facade's cancellation handle
       and carries no QSpec FR-331 `deadline_ms` from QSL, so a caller's
       deadline reaches the run only through that handle, which is rule 1.
       A host that enforces a `deadline_ms` overrun on the backend enforces
       the backend's own wall clock, which is this rule, as QSpec FR-363
       settles that overrun `timed-out`. It settles V-7
       `Incomplete(TimedOut)` (FR-293), QSpec FR-331's `timed-out`, never
       `LimitReached` or `InductionNotClosed`;
    3. `LimitStopped{limit}`, when a `ModelCheckLimit` the request names,
       and that bounds the SMT run, stopped the solver. It settles V-7
       `Incomplete(LimitReached{limit, value, setting})` naming that limit,
       never `InductionNotClosed`. A limit bounds the SMT run only when its
       definition names an EN-2 or EN-3 query; every other
       `ModelCheckLimit` bounds none. For example, `MaxStates`,
       `MaxTransitions` and `MaxAutomatonStates` bound EN-1's exploration,
       `MaxWitnessSet` and `MaxRelationTuples` bound FR-184's searches,
       `MaxCertificateBytes` and `MaxCertificateEdges` bound a certificate
       check, `MaxWalkSteps`, `WitnessSamples` and the `Statistical` limits
       bound sampling runs, `MaxCandidates` bounds expansion, and
       `SolverSteps` is FR-250's hybrid solver budget. `EvaluationMeter` bounds `μ_U` while the query is
       encoded, and its stop is the `match-conjunct` encoding refusal. A
       `LimitStopped` naming a limit that does not bound the SMT run SHALL
       write no record and refuse `HyperSettleRefusal::LimitNotApplicable`.
       Until a `ModelCheckLimit` bounds the SMT run, every solver stop that
       is neither cancellation nor wall clock falls under rule 4;
    4. `Unknown{depth}`, for the solver's own undecided answer (`unknown`),
       including an internal resource stop that no request limit names. It
       settles V-6 `InductionNotClosed{depth}`, as QSpec FR-363 and FR-399
       define `induction-not-closed`.

    A solver crash, or solver output that does not parse as an SMT-LIB
    response to the query, is no stop and SHALL NOT be reported as
    `Unknown`. CG SHALL report it as `BackendFault`, which settles `failed`,
    `Failed`, the ADR-013 O-16 internal-failure category, as FR-127 settles
    an internal fault;
  - `Cancelled{source}` as V-7 `Incomplete(Cancelled{source})`;
  - `EncodingRefused`: `match-conjunct` as V-6 `MatchUndetermined`;
    `hyper-body`, a field sort or an out-of-fragment construct as V-8
    `Unsupported(unsupported-requested-capability)`; `query-kind` writes no
    record and refuses `HyperSettleRefusal::QueryKind`. The other refusal
    codes cannot reach this row, because `HyperProductIdentity::of(item)`
    refuses them first (`ItemIdentity`).
- An SMT `Inductive{depth: k}` outcome for HP-2 SHALL settle V-3 only when
  every subject has an initial state and either `μ_U` is empty or its
  `MatchWitnessCheck` is `Accepted` for a witness whose `instance` equals
  the outcome's instance. FR-315's induction queries are also
  unsatisfiable when no fair tuple matches `μ_U`, and ADR-023 HM-7 settles
  such a clause `VacuousMatch`, never `proved`. The checks run in this
  order:
  - First, over a subject with no initial state, the outcome SHALL settle
    V-6 `NoInitialState` (FR-125), whatever witness comes with it.
  - Otherwise, with a non-empty `μ_U` and no `Accepted` check for the
    outcome's instance, the outcome SHALL settle V-6 `VacuousMatch`: no
    matched fair tuple has been established. A `Refused` check, or an
    `Accepted` check of a witness for another instance, counts as none.
  - An empty `μ_U` needs no witness. A finite subject with an initial state
    has a fair behaviour under any fairness set (ADR-018 §4), and with no
    match constraint every tuple of fair behaviours is matched, so the
    universal domain is not empty (HM-7).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-182-AC-1 | Each input row maps to its `TerminalValue`, FR-360 label, FR-243 basis and O-16 category exactly as the table states, `MatchUndetermined` and `VacuousMatch` each to inconclusive. | Test (TC-607) |
| FR-182-AC-2 | ADR-023 §8.1's leaky refutation settles `refuted`, `decisive-counterexample`, only after FR-183 replay reproduces it; the secure proof settles `proved`, `closed-scope`, `Exhaustive`, `Certified`, and under copy-swap `Reduced` naming `CopySwap`, `Certified`. §8.2's leaky `Opaque` refutation settles `refuted`, `decisive-counterexample`. | Test (TC-607) |
| FR-182-AC-3 | FR-176-AC-2's vacuous match settles `inconclusive`, `VacuousMatch`; FR-177-AC-4's run settles `incomplete`, `LimitReached`, naming `max_witness_set` and its value; FR-179-AC-3's tuple-limited run names `max_relation_tuples`; FR-173-AC-2's HP-4 clauses settle `unsupported`, `unsupported-requested-capability`. | Test (TC-607) |
| FR-182-AC-4 | FR-179-AC-3's `max_depth` run settles `inconclusive`, `BoundReached{depth: 1}`, execution `completed`, truth `pending`, with a record stating the HP-1 reading of the bound. | Test (TC-607) |
| FR-182-AC-5 | FR-179-AC-4's outcome settles `refuted`, `decisive-counterexample`, category violation, after FR-183 replay reproduces the undefined value at its tuple, with a record carrying `UndefinedEvaluation` naming that tuple, its two executions and the division-by-zero cause; it never settles `proved`. | Test (TC-610) |
| FR-182-AC-6 | FR-179-AC-5's `Violated` outcome settles `refuted`, `decisive-counterexample`, after its `StepTuple` replays, with no `UndefinedEvaluation`: the false tuple is the first refuting evidence, ahead of the undefined tuples. | Test (TC-611) |
| FR-182-AC-7 | §8.1's secure proof settles `proved`, certification `Certified`, after FR-163-AC-1's certificate is accepted; with FR-163-AC-2's member-removed certificate the item settles `inconclusive`, `CertificateRejected{Closure, …}`, never `proved`. An HP-2 `Inductive{depth: 1}` outcome from `k-induction`, with an empty `μ_U` or an `Accepted` `MatchWitnessCheck` (FR-182-AC-10), settles `proved`, `decisive-witness`, certification `Uncertified`. | Test (TC-607) |
| FR-182-AC-10 | Over ADR-023 §8's secure vault, `NonInterference`'s `Inductive{depth: 1}` outcome with no `MatchWitnessCheck` settles `inconclusive`, `VacuousMatch`, never `proved`. With FR-183-AC-8's matched witness, whose check is `Accepted`, it settles `proved`, `decisive-witness`, `Uncertified`. With FR-183-AC-8's unmatched witness, whose check is `Refused`, it settles `inconclusive`, `VacuousMatch`. Over a vault whose `vaults` universe is `{v, w}`, the outcome for instance `w` with the matched witness for instance `v`, whose check is `Accepted`, settles `inconclusive`, `VacuousMatch`. The one-variable clause `forall trace a of V { always holds(v.l @ a <= 1) }`, whose `μ_U` is empty, settles its `Inductive{depth: 1}` outcome `proved`, `Uncertified`, with no witness. `NonInterference` over the secure vault with an empty initial list settles `inconclusive`, `NoInitialState`, never `proved`, with or without a witness. | Test (TC-607) |
| FR-182-AC-11 | Over ADR-023 §8's secure vault with the `vaults` universe `{v, w}`, `settle_hyper_smt` for the `NonInterference` item at instance `v`, with an `Inductive{depth: 1}` outcome carrying FR-315's identity for that item, settles `proved`, `decisive-witness`, `Uncertified`, method `k-induction`, with FR-183-AC-8's matched witness for `v`, and `inconclusive`, `VacuousMatch` with its unmatched witness; an `InductionNotClosed{depth: 1}` outcome carrying the same identity settles `inconclusive`, `InductionNotClosed{depth: 1}`. Each of these writes no record and refuses `ProductMismatch` for the item at `v`: the outcome carrying the identity for instance `w`, naming `instance`; the outcome carrying the identity for a subject that differs only in a proof bound, naming `subjects`; the `Inductive` and the `InductionNotClosed` outcomes carrying the identity of the clause with `fair { weak V::Vault::step }` on `a`, naming `clause`; and the outcome carrying that fair clause's identity for instance `w`, which differs in both `clause` and `instance`, naming `clause`. The item with an empty `instance` refuses `ItemIdentity` carrying `instance-binding`. | Test (TC-1190) |
| FR-182-AC-12 | Over ADR-023 §8's secure vault, `settle_hyper_smt` for the `NonInterference` item at instance `v`, each outcome carrying that item's identity, settles: `BoundReached{depth: 2}` as `inconclusive`, `BoundReached{depth: 2}`, method `smt-unrolling`, with a record stating the HP-2 bound reading; the leaky vault's `Violated` `Lockstep` lasso, method `smt-unrolling`, as `refuted`, `decisive-counterexample` once QSL's own replay over the item reproduces it, and the same lasso with one post-state digest altered as `inconclusive`, `ReplayRefused`; the same lasso with its `instance` binding `w`, inside the outcome carrying `v`'s identity, writes no record and refuses `ProductMismatch` naming `instance`, never `refuted`; FR-315-AC-9's undefined-match base model, concretised as an `Undefined` counterexample with method `k-induction`, as `refuted`, `UndefinedEvaluation`, method `k-induction`; `Unknown{depth: 1, method: KInduction}` as `inconclusive`, `InductionNotClosed{depth: 1}`, method `k-induction`, never `proved`; `Cancelled{source: Requested, method: KInduction}` as `failed`, `Incomplete(Cancelled{Requested})`; a solver stop at depth 1 under the backend's own wall-clock limit, reported `TimedOut{method: KInduction}`, as `failed`, `Incomplete(TimedOut)`, never `InductionNotClosed`; `LimitStopped` naming `MaxStates`, which bounds no SMT run, writes no record and refuses `LimitNotApplicable`, never `InductionNotClosed`; a solver crash and a solver response that does not parse, each reported `BackendFault{method: KInduction}`, as `failed`, `Failed`, never `InductionNotClosed`; an internal solver resource stop that no request limit names, reported `Unknown{depth: 1, method: KInduction}`, as `inconclusive`, `InductionNotClosed{depth: 1}`; `EncodingRefused` with `match-conjunct` as `inconclusive`, `MatchUndetermined`, and with `hyper-body` as `unsupported`, `unsupported-requested-capability`. `EncodingRefused` with `query-kind` writes no record and refuses `QueryKind`. Each of these outcomes carrying the identity for instance `w` refuses `ProductMismatch` naming `instance`. | Test (TC-1190) |
| FR-182-AC-8 | FR-177-AC-5's outcome settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation` naming `a` and `b`, after its replay; FR-177-AC-6's and FR-178-AC-6's outcomes settle `inconclusive`, `MatchUndetermined`. | Test (TC-607) |
| FR-182-AC-9 | ADR-023 §1's `NonInterference` over an empty `Vault` universe settles `inconclusive`, `NoInstance`, basis `unsettled`, before any state is explored, never `Proved` or `Refuted`; ADR-023 §1's `SavesPower` `(d: S::Device, e: N::Device)` with one empty universe and the other non-empty settles the same way, so the product is read over both parameters; the same clause over a population with no universe is `RequiresBound`, not `NoInstance`. | Test (TC-607) |

## Dependencies

- ADR-023 §6 HV-1 to HV-8, §7 HX-4 and HX-7; ADR-029 CB-2 and RU-2; ADR-018 §1 (V-1 to V-8); ADR-013 O-16
  and O-24 as amended by ADR-018.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the map), [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md),
  [FR-177](FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md),
  [FR-178](FR-178-check-a-projection-aligned-hyperproperty.md),
  [FR-179](FR-179-check-a-step-relation-over-reachable-transitions.md),
  [FR-183](FR-183-replay-a-hyper-counterexample.md),
  [FR-315](FR-315-encode-the-smt-lib-transition-relation.md) (the HP-2
  `HyperProductIdentity` an SMT outcome carries).
- QSpec owns the verdict table onto FR-360 and FR-243 and the new
  inconclusive causes on the wire (ADR-023 QS-5).

## References

- ADR-023. QSpec half: QSpec FR-399 (Linear STD-136; ADR-023 QS-5).

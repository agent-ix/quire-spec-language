---
id: FR-127
title: "Settle a model-check verdict as a QSpec FR-331 terminal record"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: depends_on
---
# FR-127: Settle a model-check verdict as a QSpec FR-331 terminal record

## Description

QSL SHALL settle every temporal item over a model subject that EN-1 decides
as exactly one QSpec FR-331 terminal record whose value states the verdict's
strength (ADR-018 V-1, V-4 to V-7). `qsl-replay` owns the terminal record,
its category map and the `ProofBasis` and `InconclusiveCause` additions
(ADR-013 O-24, as amended by ADR-018), whose category map covers every
value ADR-018 V-1 to V-8 names. `qsl-replay`, in the qualified core,
also owns the one settlement map from `ModelCheckOutcome` (FR-126) and the
replay result of its counterexample (FR-128) to `TerminalValue`, and the
certificate checkers that a proof passes before it settles `proved`
(ADR-018 PC-1 to PC-5, LA-3). The driver calls the map with the outcome
`qsl-analyze` returns.

## Use case

A verification operator reads a run's results. A `proved` item says how it
was proved: exhaustive exploration, a bounded-complete unrolling, or
induction. A depth-limited search that found nothing says `inconclusive`
with its depth, never `proved`. A run that hit a limit says which limit, its
value and how to raise it. A counterexample counts as `refuted` only after
it replays.

## Semantic authority and boundary

QSpec owns the verdict table onto QSpec FR-360 and FR-243,
the method and depth members of the QSpec FR-331 terminal record, and the new
inconclusive causes on the wire (ADR-018 QS-6; References). This
requirement specifies QSL's types and its map from EN-1's outcomes. The SMT
backend's outcomes (V-2, V-3, `InductionNotClosed`) reach `TerminalValue`
through CG's map (ADR-018 DS-2), and a V-8 settlement through negotiation;
QSL supplies their types and categories.

## Inputs

- A `ModelCheckOutcome` (FR-126), with its `ProofCertificate` for `Holds`.
- For a certificate check: FR-098's replay request for the subject's
  package (package reference and byte provision), the subject's universes
  and initial-state snapshots, and the item.
- For `Violated`: the FR-072 result of replaying its counterexample
  (FR-128), or the `ReplayRefusal` that stopped it.
- For a cancelled run: the `CancelCause` its `Cancel` handle carries
  (FR-276).
- For an SMT proof of a temporal item: the `SmtProofOutcome` CG's SMT map
  hands over, the item's FR-338 `CertificateRequest`, and the request's
  FR-245 `CertificateCheckLimits` and FR-276 `Cancel` handle.

## Outputs

`TerminalValue` (`qsl_replay::proof_result`) with these additions:

```rust
pub enum ProofBasis {
    Checks { success_checks: u32 },   // Kani; zero is the vacuous proof
    Exhaustive,
    BoundedComplete { depth: u64 },
    Inductive { depth: u64 },
}

pub enum Certification { Certified, Uncertified, Trusted }   // ADR-018 PC-1

// TerminalValue::Proved { basis: ProofBasis, certification: Certification }
// InconclusiveCause gains:
//   BoundReached { depth: u64 }, InductionNotClosed { depth: u64 },
//   UndecidedSuccessor, NoInitialState, NoInstance,
//   CertificateRejected { rule: CertificateRule, at: CertificateLocus },
//   written certificate-rejected{rule, at} (QSpec FR-331)
//   StaleDependency { requested: StaleIdentity, recompiled: StaleIdentity },
//   where StaleIdentity is Package(PackageId) or Refinement { name },
//   written stale-dependency{requested, recompiled} (QSpec FR-331)
//   RefutedIgnoringConstraints, written as the QSpec FR-331 inconclusive
//   cause refuted-ignoring-constraints; its lasso violates an explicitly
//   ignored memory constraint and is not an ordinary counterexample.

// ProductStateRef, ClosureCertificate, CertificateRule: FR-338.
// ComponentCertificate, ProofCertificate: FR-339.
// IncompleteCause gains LimitReached { limit: ModelCheckLimit, value: u64, setting: RequestSetting },
//   QSpec FR-331's limit-reached; setting is the FR-255 setting name that raises it.
//   A monitor run (FR-251) writes the same record with limit `event_buffer` and
//   setting `monitor.max_event_rate`.
//   Cancelled { source: CancelCause }, written cancelled{source} (QSpec FR-331),
//   source the CancelCause wire name: requested or deadline (FR-276).
```

An SMT proof of a temporal item reaches the settlement map through this
handoff, a type and an entry of the `qsl-replay` facade:

```rust
/// The proof bases an SMT backend reports (ADR-018 V-2, V-3).
pub enum SmtProofBasis {
    BoundedComplete { depth: u64 },
    Inductive { depth: u64 },
}

/// What CG's SMT map hands QSL for an SMT proof of a temporal item: the
/// proof's basis and the solver's certificate when one came with it.
pub struct SmtProofOutcome {
    pub basis: SmtProofBasis,
    pub certificate: Option<SmtProofCertificate>,   // FR-314
}

pub fn settle_smt_proof(
    request: &CertificateRequest<'_>,   // FR-338: the item, its recompiled package and re-admitted subject
    outcome: &SmtProofOutcome,
    limits: &CertificateCheckLimits,     // FR-245: the request's actual limits
    cancel: &Cancel,                     // FR-276: the request's actual handle
) -> TerminalValue;
```

CG calls `settle_smt_proof` from its `replay` adapter module, as it calls
FR-182's `settle_hyper_smt` for an HP-2 item. An SMT proof of a temporal
item settles through `settle_smt_proof` and through no other entry, so no
SMT proof value is built outside QSL's certificate check. CG's SMT map
constructs every other SMT outcome of a temporal item (V-5,
`InductionNotClosed`, V-7 and replayed counterexamples, ADR-018 DS-2).
`SmtProofBasis` names only the two SMT bases, so a `Checks` or `Exhaustive`
basis cannot reach FR-314's checker. A refinement item's SMT records settle
through FR-146.

The certificate types and checkers are FR-338's and FR-339's. Their closed
`TemporalCertificateCheck` result has four alternatives: `Accepted`,
`Rejected(CertificateRejection)`, `Stopped(IncompleteCause)` and
`PackageMismatch{certificate, recompiled}` (FR-282). `Stopped` carries only
the existing `LimitReached{limit,value,setting}` or `Cancelled{source}`; it
adds no kernel or wire cause.

`StaleIdentity` preserves the identity domain in the one stale-dependency
cause:

```rust
pub enum StaleIdentity {
    Package(PackageId),
    Refinement { name: String },
}

pub struct StaleDependency {
    pub requested: StaleIdentity,
    pub recompiled: StaleIdentity,
}
```

The FR-331 wire writes a package identity as
`{"kind":"package","package_id":<PackageId>}` and a refinement identity as
`{"kind":"refinement","name":<name>}`. `requested` and `recompiled` each
carry the tagged identity they represent; a refinement name is never encoded
as a package ID or digest.

and the QSpec FR-331 terminal record carrying the value, its QSpec FR-360 label, its QSpec FR-243 basis and its O-16 category.

## Behavior

### The map from EN-1's outcomes

- The settlement map SHALL map each input to exactly one row of this table:

| Verdict | Input | QSpec FR-360 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-1 | `Holds{Exhaustive}` whose certificate the core checker accepts | `proved` | `closed-scope` | `Proved{basis: Exhaustive, certification: Certified}` | success |
| V-6 | `Holds{Exhaustive}` whose certificate the core checker rejects | `inconclusive` | `unsettled` | `Inconclusive(CertificateRejected{rule, at})` | inconclusive |
| V-6 | `Holds{Exhaustive}` whose certificate the core checker returns `PackageMismatch{certificate, recompiled}` (FR-338, FR-339) | `inconclusive` | `unsettled` | `Inconclusive(StaleDependency{requested, recompiled})`, `requested` the certificate's `package_id`, written `stale-dependency{requested, recompiled}` | inconclusive |
| V-4 | `Violated` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-4 | `Violated` with `kind: UndefinedEvaluation{where, cause}` whose replay reproduces the undefined value at `where` (FR-128) | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| V-5 | `BoundReached{depth}` | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth})` | inconclusive |
| V-6 | `Undecided(UndecidedSuccessor)` (a refused contract conjunction), `Undecided(NoInitialState)` or `Undecided(NoInstance)` (an item with zero instances, FR-125) | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-6 | `Violated` whose replay settles `inconclusive` (`Verdicts` or `NoValue`) | `inconclusive` | `unsettled` | `Inconclusive(ReplayParity)` | inconclusive |
| V-6 | `Violated` whose replay refuses with `ReplayRefusal::UnfairLasso` (FR-131), an unfair lasso, which EN-1 never builds (ADR-019 SV-4) | `inconclusive` | `unsettled` | `Inconclusive(ReplayParity)` | inconclusive |
| V-6 | The protocol liveness engine returns `RefutedIgnoringConstraints` (FR-228), because its lasso violates an ignored memory fairness constraint | `inconclusive` | `unsettled` | `Inconclusive(RefutedIgnoringConstraints)` | inconclusive |
| V-6 | `Violated` whose replay refuses with `ReplayRefusal::PackageIdMismatch{requested, recompiled}` (ADR-013 O-26), the request's `package_id` and the recompiled one; or whose FR-149 simulation certificate names a different refinement than its request | `inconclusive` | `unsettled` | `Inconclusive(StaleDependency{requested, recompiled})`, written `stale-dependency{requested, recompiled}` with package or refinement identities preserved and tagged | inconclusive |
| V-6 | `Violated` whose replay refuses with any other `ReplayRefusal` variant except an internal fault | `inconclusive` | `unsettled` | `Inconclusive(ReplayRefused)` | inconclusive |
| — | `Violated` whose replay refuses with `InternalFault` | `failed` | `unavailable` | `Failed` | failed (ADR-013 O-16 internal failure) |
| — | An SMT result whose `check_smt_proof` or `check_refinement_smt_proof` returns `SmtProofCheck::Fault` (FR-314, FR-315 "Internal faults") | `failed` | `unavailable` | `Failed` | failed (ADR-013 O-16 internal failure) |
| V-7 | `Stopped{cause: ResourceExhausted, limit: Some(ReachedLimit{limit, value})}`, a limit reached | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` | incomplete |
| V-7 | `Stopped{cause: Cancelled, limit: None}` | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(Cancelled{source})`, written `cancelled{source}` | incomplete |

- When an outcome is `Violated`, the settlement map SHALL settle it `refuted`
  only through its replay.
- When the protocol liveness engine returns `RefutedIgnoringConstraints`,
  the settlement map SHALL write the typed QSpec FR-331 cause
  `refuted-ignoring-constraints` as `inconclusive`. It SHALL retain neither a
  `refuted` label nor a proof basis, and it SHALL NOT send the lasso to the
  ordinary counterexample replay path or write it as a standard
  counterexample.
- When a claim, a contract conjunction or a `terminal when` predicate
  evaluates undefined on an admitted behaviour (ADR-018 UE-1, UE-6), the
  settlement map SHALL settle the item `refuted` with cause `UndefinedEvaluation{where,
  cause}`, carried as its counterexample's `kind` (ADR-018 UE-1, UE-2); it
  SHALL add no label, basis, `TerminalValue` variant or category for it.
- The settlement map SHALL write in a limit-reached record the limit, its
  value and the FR-255 setting that raises it (FR-126 `ReachedLimit`,
  FR-255 Behaviors 9 and 11), `max_automaton_states` included:

  | `ReachedLimit` | `limit` | `value` | `setting` |
  | --- | --- | --- | --- |
  | `{MaxStates, v}` | `max_states` | `v` | `model_check.max_states` |
  | `{MaxTransitions, v}` | `max_transitions` | `v` | `model_check.max_transitions` |
  | `{MaxAutomatonStates, v}` | `max_automaton_states` | `v` | `model_check.max_automaton_states` |
  | `{EvaluationMeter { counter }, v}` | the counter's `quire.value.accounting/v1` member name | `v`, that counter's configured bound in the subject's meter budget | `accounting.<counter>` |
  | `{MaxCandidates, v}` | `max_candidates` | `v` | `expansion.max_candidates` |

- When a V-7 stop is an evaluation-meter stop, the settlement map SHALL
  write the record FR-203 writes for the same stop (FR-203-AC-7, FR-192-AC-1):
  `limit` the exhausted counter's name, `value` its configured bound and
  `setting` `accounting.<counter>`, so a caller raises it with
  `--limit accounting.<counter>=<n>` or a replay request's `stage_limits`
  entry of that name. The same holds for a meter stop that a certificate
  check (FR-338, FR-339) returns as `Stopped(LimitReached{..})`. No record
  carries a setting `meter`, a bare counter name or a bare
  `max_candidates`.
- The settlement map SHALL write in every terminal record of an EN-1 item,
  V-1 included, the values of `max_states`, `max_transitions` and
  `max_automaton_states` the run used and whether any was reached (FR-126
  defaults).
- The settlement map SHALL give a run that completes every depth up to `k`
  execution `completed` and truth `pending` (V-5), and a run that reaches a
  budget before completing `k` V-7 (ADR-014 B-5 as amended by ADR-018).
- The settlement map SHALL write in the record the method `explicit-state`, and
  for V-5 the depth.
- The settlement map SHALL settle a `DeadlockFreedom` item through the same map;
  its V-4 record carries the counterexample whose `kind` is `Deadlock`.
- The settlement map SHALL bind the subject in the record's obligation identity
  (ADR-013 O-09), so a verdict holds for exactly its subject.

- FR-072's replay causes `Verdicts` and `NoValue` SHALL settle the item
  `inconclusive`, cause `ReplayParity`, spelled `replay-parity` on the wire
  (QSpec FR-364); `Verdicts` names FR-072's replay result and
  `replay-parity` the item's cause, and no other name is used for either.

### Proof certificates

- Before settling `Holds{Exhaustive}` the settlement map SHALL run the
  core checker for its certificate, `check_closure` or `check_components`,
  and settle `Proved{Exhaustive, Certified}` when it accepts and
  `Inconclusive(CertificateRejected{rule, at})` when it returns `Rejected`.
  A `PackageMismatch{certificate, recompiled}` SHALL settle
  `Inconclusive(StaleDependency{requested, recompiled})` with `requested`
  the certificate's `package_id` (QSpec FR-331), never `CertificateRejected`.
  A `Stopped(cause)` SHALL settle V-7, execution `resource-incomplete`,
  basis `unavailable`, `TerminalValue::Incomplete(cause)`, category
  incomplete. The map SHALL preserve the exact checker limit, consumed
  value and request setting, or FR-276 cancellation source. The limits and
  Cancel handle supplied to both checkers are the request's actual ones;
  no certificate failure or unproved result substitutes for a stop.
- The checkers SHALL be FR-338's `check_closure` and FR-339's
  `check_components`; a rejection names the rule and the product state at
  which it failed.
- For an `SmtProofOutcome` that CG's SMT map hands it, `settle_smt_proof`
  SHALL run FR-314's `check_smt_proof` on the request, the outcome's
  `basis` and its `SmtProofCertificate`, and settle `Proved{basis, ..}` with
  `basis` the outcome's `BoundedComplete{depth}` or `Inductive{depth}`, and
  with
  `certification: Certified` when it returns `Verified`,
  `Inconclusive(CertificateRejected{rule, at})` when it rejects, and
  `proved` with `Uncertified` when it returns `Unverifiable` or the
  outcome's `certificate` is `None` (ADR-018 PC-6). With no certificate it
  SHALL run no check and charge nothing. When `check_smt_proof` returns
  `Fault`, it SHALL settle `failed`, `TerminalValue::Failed` (the table's
  row). The map SHALL supply the
  request's actual `CertificateCheckLimits` and `Cancel` handle to
  `check_smt_proof` and `check_refinement_smt_proof`; when either returns
  `Stopped(cause)`, the map SHALL settle V-7 as for `check_closure` above:
  execution `resource-incomplete`, basis `unavailable`,
  `TerminalValue::Incomplete(cause)` with the exact limit, value and
  setting or cancellation source, category incomplete, never `proved`. A proof from any other
  native engine with no core certificate checker SHALL
  carry `Uncertified`; one from a third-party plugin SHALL
  carry `Trusted`; a Kani `Checks` proof, from the qualified core's
  prove path, SHALL carry `Certified`. Every `Proved` carries exactly one
  certification; each settles `proved` and maps to success (ADR-018 PC-1). CG's C-09 map (ADR-013 C-09, ADR-011 T-13) and its SMT
  map (ADR-018 DS-2) construct the values of their backends; this
  settlement map covers QSL's native engine, EN-1, and the SMT certificate
  check.
- **Bounded-complete basis rule.** Before it settles an SMT
  `BoundedComplete{depth}` result, with a certificate or without one, and
  before every other rule for that result above, the settlement map SHALL
  check the basis against the item: when the item is not a TP-2 `on origin`
  item, or `depth` is below the item's ADR-014 TR-4 horizon `h` (ADR-018
  V-2), it SHALL settle `inconclusive`, `unsettled`,
  `Inconclusive(CertificateRejected{ShapeMismatch, Query { part: Unrolling
  }})`, run no certificate check and charge nothing, never `proved`
  `Certified` or `Uncertified`. Below `h` the claim can fail at a position
  the query does not reach (FR-315 "Monitors"). This rule is the
  settlement's first step whichever entry settles the result; under the
  `qsl-replay` handoff entry for an SMT proof of a temporal item it is that
  entry's first step, ahead of its rule that a result with no certificate
  runs no check and settles `Uncertified`, which applies only to a basis
  this rule admits. FR-314's checker applies the same rule to a certificate
  it is given.
- Downstream need (IR/CG overlap): QSL needs CG's SMT backend to build its
  queries with FR-315's encoding and to hand each SMT `BoundedComplete` or
  `Inductive` result over with the solver's proof certificate, each refuted
  query and its proof in a format FR-314 names, so that the SMT label can
  be checked away (ADR-018 DS-2).

### Categories of every proof value

- `TerminalValue::category` SHALL map `Proved{basis: Checks{success_checks:
  0}, certification: Certified}` to inconclusive with `KaniVacuousProof`, and every other `Proved`
  basis, `Exhaustive`, `BoundedComplete{depth}` and `Inductive{depth}`, to
  success.
- `TerminalValue::category` SHALL map `Proved` with any certification
  as its basis maps, and `Inconclusive` with each of the new causes,
  `CertificateRejected` included, to inconclusive, and `Unsupported(cause)` to unsupported.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-127-AC-1 | Each row of the map settles its `TerminalValue`, FR-360 label, FR-243 basis and O-16 category exactly as the table states: `Holds{Exhaustive}` with an accepted certificate, settling `Proved{Exhaustive, Certified}`; a reproduced `Violated`; `BoundReached{depth: 2}` carrying depth 2 and method `explicit-state`; `Undecided(UndecidedSuccessor)`; `Undecided(NoInitialState)`; `Stopped{ResourceExhausted, {MaxStates, 2}}` settling `Incomplete(LimitReached{limit: max_states, value: 2, setting: model_check.max_states})`. | Test (TC-522) |
| FR-127-AC-2 | `TerminalValue::category` maps `Proved{Checks{0}, Certified}` to inconclusive `KaniVacuousProof`; `Proved{Checks{1}, Certified}`, `Proved{Exhaustive, Certified}`, `Proved{BoundedComplete{depth: 5}, Certified}`, `Proved{BoundedComplete{depth: 5}, Uncertified}` and `Proved{Inductive{depth: 2}, Uncertified}` to success; `Inconclusive` with `BoundReached{1}`, `InductionNotClosed{2}`, `UndecidedSuccessor` and `NoInitialState` to inconclusive. | Test (TC-522) |
| FR-127-AC-3 | FR-126-AC-1's weak `each` outcome settles `proved`, `closed-scope`, `Proved{Exhaustive, Certified}`, success. Its counterexample under the constraint with no granularity settles `refuted` only after FR-128 replay reproduces it. The same counterexample with one post-state digest altered settles `inconclusive`, `ReplayRefused`; replayed in an envelope for the weak `each` clause, whose fairness it fails, it settles `inconclusive`, `ReplayParity`, since EN-1 builds only fair lassos; with its last step removed, so the loop does not close, it settles `inconclusive`, `ReplayRefused`; one whose formula evaluates `true` on replay settles `inconclusive`, `ReplayParity`. A replay that returns `InternalFault` settles `failed`, category failed. | Test (TC-522) |
| FR-127-AC-4 | FR-126-AC-6's `max_automaton_states` run settles FR-360 `failed`, execution `resource-incomplete`, result `incomplete`, `unavailable`, `Incomplete(LimitReached{limit: max_automaton_states, value: 50, setting: model_check.max_automaton_states})`, category incomplete; a cancelled run settles `incomplete`, `Incomplete(Cancelled{source})`, written `cancelled{source}`, `source` `requested` for a caller's cancellation and `deadline` for a deadline; every record, AC-3's proof included, states the three limits used and whether one was reached; FR-126-AC-5's `max_depth` 2 run settles `inconclusive`, `BoundReached{depth: 2}`, execution `completed`, truth `pending`; its evaluation-meter run settles `Incomplete(LimitReached{limit: work_units, value: 0, setting: accounting.work_units})`. | Test (TC-522) |
| FR-127-AC-5 | FR-126-AC-3's deadlock-freedom violation settles `refuted` after replay, with a record whose counterexample `kind` is `Deadlock`, and the record's obligation identity differs from the authored claims' over the same subject. | Test (TC-522) |
| FR-127-AC-6 | FR-126-AC-9's undefined-evaluation counterexample settles `refuted`, `decisive-counterexample`, `Refuted`, category violation, after FR-128 replay reproduces it, and the record's counterexample carries `kind: UndefinedEvaluation{where: position 2, cause: division-by-zero}`. The same payload with its cause changed to another undefined reason settles `inconclusive`, `ReplayParity`. | Test (TC-538) |
| FR-127-AC-7 | FR-338-AC-1's TP-1 proof with its accepted certificate settles `proved`, `closed-scope`, `Proved{Exhaustive, Certified}`; with FR-338-AC-2's `(1, 0)`-removed certificate it settles `inconclusive`, `unsettled`, `Inconclusive(CertificateRejected{SuccessorMissing, (1, 0)})`. Exhausting the auxiliary checker graph budget returns `Stopped(LimitReached{max_states, value, model_check.max_states})` and settles `Incomplete` with those exact members; a certificate check whose atom reading is denied a `work_units` charge under a meter budget whose `work_units` is 0 returns `Stopped(LimitReached{work_units, 0, accounting.work_units})` and settles `Incomplete` with those members, and caller/deadline cancellation retains `requested`/`deadline`. Neither settles `CertificateRejected` or `WitnessFails`. | Test (TC-522) |
| FR-127-AC-8 | FR-339-AC-1's weak `each` proof with its accepted certificate settles `Proved{Exhaustive, Certified}`; with FR-339-AC-2's `upd(a)` witness it settles `Inconclusive(CertificateRejected{WitnessFails, ...})` naming that component's first state. | Test (TC-522) |
| FR-127-AC-9 | The `TerminalValue`s `Proved{Checks{3}, Certified}`, `Proved{BoundedComplete{depth: 5}, Certified}`, `Proved{BoundedComplete{depth: 5}, Uncertified}`, `Proved{Inductive{depth: 2}, Uncertified}` and a plugin's `Proved{Exhaustive, Trusted}`, as CG's maps and the settlement map construct them, each read `proved` with category success, never `inconclusive`, and each keeps its one certification. `Inconclusive(CertificateRejected)` maps to category inconclusive. | Test (TC-522) |
| FR-127-AC-10 | A replay result with cause `Verdicts` settles the item `inconclusive`, cause `ReplayParity`, written `replay-parity` in the QSpec FR-331 record. | Test (TC-522) |
| FR-127-AC-12 | FR-127-AC-3's counterexample replayed with a request whose `package_id` is another package's refuses `PackageIdMismatch` and settles `inconclusive`, `Inconclusive(StaleDependency{requested, recompiled})` naming the request's and the recompiled `package_id`, written `stale-dependency{requested, recompiled}` on the FR-331 record, not `ReplayRefused`. | Test (TC-1141) |
| FR-127-AC-13 | FR-338-AC-11's stale closure certificate and FR-339-AC-4's stale component certificate each settle `inconclusive`, `unsettled`, `Inconclusive(StaleDependency{requested, recompiled})` with `requested` the certificate's `package_id` and `recompiled` the recompiled one, written `stale-dependency{requested, recompiled}` on the FR-331 record, never `certificate-rejected` and never `proved`. | Test (TC-1141) |
| FR-127-AC-14 | `settle_smt_proof`, a `qsl-replay` entry, settles each `SmtProofOutcome` for FR-314-AC-1's `Counter` items: `BoundedComplete{depth: 3}` with FR-314-AC-1's accepted certificate as `Proved{BoundedComplete{depth: 3}, Certified}`, `proved`, success; `Inductive{depth: 1}` with its accepted certificate as `Proved{Inductive{depth: 1}, Certified}`; `BoundedComplete{depth: 3}` with `certificate: None` as `Proved{BoundedComplete{depth: 3}, Uncertified}`, with no check run and the request's meters unchanged; FR-314-AC-2's `QueryMismatch` certificate as `Inconclusive(CertificateRejected{QueryMismatch, Query { part: Unrolling }})`; FR-314-AC-2's `hole` certificate as `Proved{BoundedComplete{depth: 3}, Uncertified}`; FR-314-AC-5's limit-stopped certificate as `Incomplete(LimitReached{MaxCertificateBytes, 1, certificate_check.max_certificate_bytes})`; FR-315-AC-30's faulting rebuild as `Failed`. A crate whose only QSL dependency is `qsl-replay` names `SmtProofOutcome`, `SmtProofBasis`, `SmtProofCertificate` and `settle_smt_proof` and compiles. | Test (TC-1199) |
| FR-127-AC-15 | Over FR-314-AC-1's `Counter` under event-position false-extension, an SMT `BoundedComplete{depth: 0}` result for `always[0,3] eventually[0,1] holds(c.value = 3)` (horizon 4) with no certificate settles `inconclusive`, `Inconclusive(CertificateRejected{ShapeMismatch, Query { part: Unrolling }})`, with no check run and the request's certificate meters unchanged, never `Proved{.., Uncertified}`; the same result with a certificate settles the same before any query is encoded; `BoundedComplete{depth: 3}` with no certificate for the TP-1 item `always holds(c.value <= 3)` settles the same; `BoundedComplete{depth: 3}` with no certificate for `always[0,3] holds(c.value <= 3)` (horizon 3) still settles `Proved{BoundedComplete{depth: 3}, Uncertified}`. | Test (TC-1201) |
| FR-127-AC-16 | The settlement map settles `Stopped{ResourceExhausted, {EvaluationMeter{work_units}, 0}}` (FR-126-AC-19's run) as FR-360 `failed`, execution `resource-incomplete`, `unavailable`, `Incomplete(LimitReached{limit: work_units, value: 0, setting: accounting.work_units})`, category incomplete; `{EvaluationMeter{value_occurrences}, 7}` as `Incomplete(LimitReached{limit: value_occurrences, value: 7, setting: accounting.value_occurrences})`; `{EvaluationMeter{integer_bits}, 18446744073709551615}` as `Incomplete(LimitReached{limit: integer_bits, value: 18446744073709551615, setting: accounting.integer_bits})`; and `{MaxCandidates, 1}` as `Incomplete(LimitReached{limit: max_candidates, value: 1, setting: expansion.max_candidates})`. Each record equals the one FR-203 writes for the same stop, each `setting` is a name the FR-255 settings operation accepts (`accounting.work_units=1` returns accounting limits with `work_units` 1), and no record writes the setting `meter`, a bare counter name or a bare `max_candidates`. | Test (TC-522) |
| FR-127-AC-17 | FR-228-AC-10's `RefutedIgnoringConstraints` engine result settles as `inconclusive`, `unsettled`, `Inconclusive(RefutedIgnoringConstraints)`, with no basis and no standard counterexample record; it is never readable as `refuted`, `Refuted` or any proof. | Test (TC-673) |
| FR-127-AC-18 | FR-149-AC-4's refinement-name mismatch and FR-127-AC-12's package-ID mismatch each settle as the same `StaleDependency` cause; the former writes both identities as `{"kind":"refinement","name":...}` and the latter as `{"kind":"package","package_id":...}`. A package ID remains scoped to package identity; neither identity kind is substituted for the other. | Test (TC-556) |

## Dependencies

- ADR-018 §1 (V-1 to V-8, `ProofBasis`, the new causes, length, depth,
  UE-1 and UE-2);
  ADR-013 O-16 and O-24 as amended by ADR-018; ADR-014 B-5 as amended.
- [FR-069](FR-069-implement-typed-proof-result-envelope.md) (terminal record
  and category map), [FR-072](FR-072-implement-typed-replay-result.md)
  (replay result), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-128](FR-128-replay-a-model-counterexample.md),
  [FR-338](FR-338-check-an-en-1-closure-certificate.md) and
  [FR-339](FR-339-check-an-en-1-component-certificate.md) and
  [FR-314](FR-314-check-an-smt-proof-certificate.md) (the checkers),
  [FR-315](FR-315-encode-the-smt-lib-transition-relation.md) (the SMT encoding).

## References

- QSpec FR-364 (the `replay-parity` cause).
- QSpec FR-363 (every-behaviour temporal verdicts) and FR-360 (the
  infinite-trace result disposition): the QSpec half of ADR-018 QS-6 (Linear
  STD-131).

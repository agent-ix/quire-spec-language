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

## Outputs

`TerminalValue` (`qsl_replay::proof_result`) with these additions:

```rust
pub enum ProofBasis {
    Checks { success_checks: u32 },   // Kani; zero is the vacuous proof
    Exhaustive,
    BoundedComplete { depth: u64 },
    Inductive { depth: u64 },
}

pub enum Certification { Certified, Uncertified, Trusted }

// TerminalValue::Proved { basis: ProofBasis, certification: Option<Certification> }
// InconclusiveCause gains:
//   BoundReached { depth: u64 }, InductionNotClosed { depth: u64 },
//   UndecidedSuccessor, NoInitialState,
//   CertificateRejected { rule: CertificateRule, state: CertificateLocus }

// ProductStateRef, ClosureCertificate, CertificateRule: FR-338.
// ComponentCertificate, ProofCertificate: FR-339.
// IncompleteCause gains LimitReached { limit: ModelCheckLimit, value: u64, setting: RequestSetting },
//   QSpec FR-331's limit-reached; setting names the request member that raises it.
//   Cancelled, written cancelled (QSpec FR-408-AC-8).
```

The certificate types and checkers are FR-338's and FR-339's.

and the QSpec FR-331 terminal record carrying the value, its QSpec FR-360 label, its QSpec FR-243 basis and its O-16 category.

## Behavior

### The map from EN-1's outcomes

- The settlement map SHALL map each input to exactly one row of this table:

| Verdict | Input | QSpec FR-360 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-1 | `Holds{Exhaustive}` whose certificate the core checker accepts | `proved` | `closed-scope` | `Proved{basis: Exhaustive, certification: Certified}` | success |
| V-6 | `Holds{Exhaustive}` whose certificate the core checker rejects | `inconclusive` | `unsettled` | `Inconclusive(CertificateRejected{rule, state})` | inconclusive |
| V-4 | `Violated` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-4 | `Violated` with `kind: UndefinedEvaluation{where, cause}` whose replay reproduces the undefined value at `where` (FR-128) | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| V-5 | `BoundReached{depth}` | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth})` | inconclusive |
| V-6 | `Undecided(UndecidedSuccessor)` (a refused contract conjunction) or `Undecided(NoInitialState)` | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-6 | `Violated` whose replay settles `inconclusive` (`Verdicts` or `NoValue`) | `inconclusive` | `unsettled` | `Inconclusive(ReplayParity)` | inconclusive |
| V-6 | `Violated` whose replay refuses with `ReplayRefusal::UnfairLasso` (FR-131), an unfair lasso, which EN-1 never builds (ADR-019 SV-4) | `inconclusive` | `unsettled` | `Inconclusive(ReplayParity)` | inconclusive |
| V-6 | `Violated` whose replay refuses with any other `ReplayRefusal` variant except an internal fault | `inconclusive` | `unsettled` | `Inconclusive(ReplayRefused)` | inconclusive |
| — | `Violated` whose replay refuses with `InternalFault` | `failed` | `unavailable` | `Failed` | failed (ADR-013 O-16 internal failure) |
| V-7 | `Stopped{cause: ResourceExhausted, limit: Some(ReachedLimit{limit, value})}`, a limit reached | `incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` | incomplete |
| V-7 | `Stopped{cause: Cancelled, limit: None}` | `incomplete` | `unavailable` | `Incomplete(Cancelled)`, written `cancelled` | incomplete |

- When an outcome is `Violated`, the settlement map SHALL settle it `refuted`
  only through its replay.
- When a claim, a contract conjunction or a `terminal when` predicate
  evaluates undefined on an admitted behaviour (ADR-018 UE-1, UE-6), the
  settlement map SHALL settle the item `refuted` with cause `UndefinedEvaluation{where,
  cause}`, carried as its counterexample's `kind` (ADR-018 UE-1, UE-2); it
  SHALL add no label, basis, `TerminalValue` variant or category for it.
- The settlement map SHALL write in a limit-reached record the limit, its
  value and the request member that raises it (FR-126 `ReachedLimit`),
  `max_automaton_states` included.
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
  `Inconclusive(CertificateRejected{rule, state})` when it rejects.
- The checkers SHALL be FR-338's `check_closure` and FR-339's
  `check_components`; a rejection names the rule and the product state at
  which it failed.
- For an SMT `BoundedComplete` or `Inductive` result that CG's SMT map
  hands it, the settlement map SHALL run FR-314's `check_smt_proof` on the
  result's `SmtProofCertificate` and settle `proved` with
  `certification: None` when it returns `Verified`,
  `Inconclusive(CertificateRejected{rule, state})` when it rejects, and
  `proved` with `Some(Uncertified)` when it returns `Unverifiable` or the
  result carries no certificate (ADR-018 PC-6). A proof from any other
  native engine with no core certificate checker SHALL
  carry `Some(Uncertified)`; one from a third-party plugin SHALL
  carry `Some(Trusted)`; a Kani `Checks` proof, from the qualified core's
  prove path, SHALL carry `None`. Each settles `proved` and maps to success
  (ADR-018 PC-1). CG's C-09 map (ADR-013 C-09, ADR-011 T-13) and its SMT
  map (ADR-018 DS-2) construct the values of their backends; this
  settlement map covers QSL's native engine, EN-1, and the SMT certificate
  check.
- Downstream need (IR/CG overlap): QSL needs CG's SMT backend to build its
  queries with FR-315's encoding and to hand each SMT `BoundedComplete` or
  `Inductive` result over with the solver's proof certificate, each refuted
  query and its proof in a format FR-314 names, so that the SMT label can
  be checked away (ADR-018 DS-2).

### Categories of every proof value

- `TerminalValue::category` SHALL map `Proved{basis: Checks{success_checks:
  0}}` to inconclusive with `KaniVacuousProof`, and every other `Proved`
  basis, `Exhaustive`, `BoundedComplete{depth}` and `Inductive{depth}`, to
  success.
- `TerminalValue::category` SHALL map `Proved` with either certification
  as its basis maps, and `Inconclusive` with each of the new causes,
  `CertificateRejected` included, to inconclusive, and `Unsupported(cause)` to unsupported.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-127-AC-1 | Each row of the map settles its `TerminalValue`, FR-360 label, FR-243 basis and O-16 category exactly as the table states: `Holds{Exhaustive}` with an accepted certificate, settling `Proved{Exhaustive, Certified}`; a reproduced `Violated`; `BoundReached{depth: 2}` carrying depth 2 and method `explicit-state`; `Undecided(UndecidedSuccessor)`; `Undecided(NoInitialState)`; `Stopped{ResourceExhausted, {MaxStates, 2}}` settling `Incomplete(LimitReached{MaxStates, 2, max_states})` naming `max_states`, value 2. | Test (TC-522) |
| FR-127-AC-2 | `TerminalValue::category` maps `Proved{Checks{0}}` to inconclusive `KaniVacuousProof`; `Proved{Checks{1}}`, `Proved{Exhaustive, Certified}`, `Proved{BoundedComplete{depth: 5}}` and `Proved{Inductive{depth: 2}}` to success; `Inconclusive` with `BoundReached{1}`, `InductionNotClosed{2}`, `UndecidedSuccessor` and `NoInitialState` to inconclusive. | Test (TC-522) |
| FR-127-AC-3 | FR-126-AC-1's weak `each` outcome settles `proved`, `closed-scope`, `Proved{Exhaustive, Certified}`, success. Its counterexample under the constraint with no granularity settles `refuted` only after FR-128 replay reproduces it. The same counterexample with one post-state digest altered settles `inconclusive`, `ReplayRefused`; replayed in an envelope for the weak `each` clause, whose fairness it fails, it settles `inconclusive`, `ReplayParity`, since EN-1 builds only fair lassos; with its last step removed, so the loop does not close, it settles `inconclusive`, `ReplayRefused`; one whose formula evaluates `true` on replay settles `inconclusive`, `ReplayParity`. A replay that returns `InternalFault` settles `failed`, category failed. | Test (TC-522) |
| FR-127-AC-4 | FR-126-AC-6's `max_automaton_states` run settles `incomplete`, `unavailable`, `Incomplete(LimitReached{MaxAutomatonStates, 50, max_automaton_states})`, category incomplete, its record naming `max_automaton_states` with value 50 as the member that raises it; a cancelled run settles `incomplete`, `Incomplete(Cancelled)`, written `cancelled`; every record, AC-3's proof included, states the three limits used and whether one was reached; FR-126-AC-5's `max_depth` 2 run settles `inconclusive`, `BoundReached{depth: 2}`, execution `completed`, truth `pending`; its evaluation-meter run names `EvaluationMeter` with value 0. | Test (TC-522) |
| FR-127-AC-5 | FR-126-AC-3's deadlock-freedom violation settles `refuted` after replay, with a record whose counterexample `kind` is `Deadlock`, and the record's obligation identity differs from the authored claims' over the same subject. | Test (TC-522) |
| FR-127-AC-6 | FR-126-AC-9's undefined-evaluation counterexample settles `refuted`, `decisive-counterexample`, `Refuted`, category violation, after FR-128 replay reproduces it, and the record's counterexample carries `kind: UndefinedEvaluation{where: position 2, cause: division-by-zero}`. The same payload with its cause changed to another undefined reason settles `inconclusive`, `ReplayParity`. | Test (TC-538) |

| FR-127-AC-7 | FR-338-AC-1's TP-1 proof with its accepted certificate settles `proved`, `closed-scope`, `Proved{Exhaustive, Certified}`; with FR-338-AC-2's `(1, 0)`-removed certificate it settles `inconclusive`, `unsettled`, `Inconclusive(CertificateRejected{SuccessorMissing, (1, 0)})`. | Test (TC-522) |
| FR-127-AC-8 | FR-339-AC-1's weak `each` proof with its accepted certificate settles `Proved{Exhaustive, Certified}`; with FR-339-AC-2's `upd(a)` witness it settles `Inconclusive(CertificateRejected{WitnessFails, ...})` naming that component's first state. | Test (TC-522) |
| FR-127-AC-9 | The `TerminalValue`s `Proved{Checks{3}, None}`, `Proved{BoundedComplete{depth: 5}, Some(Uncertified)}`, `Proved{Inductive{depth: 2}, Some(Uncertified)}` and a plugin's `Proved{Exhaustive, Some(Trusted)}`, as CG's maps construct them, each read `proved` with category success, never `inconclusive`; the Kani one carries no label. `Inconclusive(CertificateRejected)` maps to category inconclusive. | Test (TC-522) |
| FR-127-AC-10 | A replay result with cause `Verdicts` settles the item `inconclusive`, cause `ReplayParity`, written `replay-parity` in the QSpec FR-331 record. | Test (TC-522) |

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

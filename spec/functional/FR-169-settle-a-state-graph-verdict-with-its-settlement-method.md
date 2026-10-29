---
id: FR-169
title: "Settle a state-graph verdict with its settlement method"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-020
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-170
    type: depends_on
---
# FR-169: Settle a state-graph verdict with its settlement method

## Description

QSL SHALL settle every state-graph item instance as exactly one FR-331
terminal record, through the same map FR-127 owns, extended by two verdict
kinds (ADR-022 GV-1 to GV-6): V-9 `Witnessed` and V-10 refuted by trap. The
record SHALL state how the item was settled (ADR-022 §4 "Settlement
method"): a sampled witness with its seed and trace index, an explored or
unrolled witness, exhaustive exploration, or exploration with evidence that
replays. The verdict does not depend on the method. A witness settles
`proved` only from a run that ruled out a reachable undefined evaluation;
witnesses from any other run settle `inconclusive`,
`WellDefinednessUnchecked` (ADR-022 GV-1, RU-5). A `proved` from an
unreduced exploration settles only after the core checker accepts its
state-graph certificate, or for an HP-5 item (FR-181) its product-closure
certificate (FR-163), and carries ADR-018 PC-1's `Certified` label; a
`proved` from a reduced exploration has no core checker and carries
`Uncertified` (ADR-022 GC-1 to GC-4). `qsl-replay` owns
`ProofBasis::Witness`, its category and this verdict map, beside the other
verdicts and settlement (ADR-029 CB-2).

An item whose instancing produces no instances (FR-125) SHALL still produce
one terminal record for the item, settled as `NoInstance` by FR-127's V-6
row. It produces no per-instance state-graph outcome, explores no state,
and is never `proved`. EN-1 SHALL represent this item-level result with the
`StateGraphOutcome::NoInstance` marker in that item's run result; the marker
is not a per-instance decision and carries no evidence. The FR-169
settlement map SHALL map the marker directly to that one item terminal
record without checking a certificate or invoking replay. When a request
contains other items that do have instances (or requests deadlock freedom),
their shared exploration proceeds independently; the no-instance item
itself contributes no predicate labels or state exploration.

## Use case

A verification operator reads a `proved` `possible` claim and sees that a
sampled walk with seed 7 and trace index 3 settled it, and that the walk
replayed. A second run with sampling off reads `proved` too, settled by
exploration. A refuted `always possible` claim names its trap, and a
depth-limited search that found neither a witness nor a trap reads
`inconclusive` with its depth. A run that a state budget stopped after
sampling found every witness reads `inconclusive`: witness found,
well-definedness unchecked.

## Inputs

- A `StateGraphOutcome` (FR-168), or an SMT `possible` path an engine
  returned through CG (ADR-022 GE-3), mapped into the same values.
- For `Witnessed`, `WitnessedUnchecked`, `Trapped` and `PathPair`: the FR-072 replay result of its
  evidence (FR-170), or the `ReplayRefusal` that stopped it.
- The run's seed and limits (FR-166).
- For `Witnessed` and `Holds`: the
  `StateGraphCertificate` the engine returns with it (FR-168), and FR-098's
  replay request for the subject, which the checker recompiles.
- For an HP-5 item: FR-181's outcome for one instance in place of a
  `StateGraphOutcome`, and for its `Witnessed` the
  `ProductClosureCertificate` it carries (FR-163) in place of a
  `StateGraphCertificate`, with FR-098's replay request for the alias's
  subject.

## Outputs

```rust
pub enum ProofBasis {
    // FR-127's members, plus:
    Witness { sources: Vec<WitnessSource> },   // one per initial state, in order
}

pub enum WitnessSource {
    Sampled(SampleProvenance),     // FR-101: seed, trace index, sampler
    Explored,
    Unrolled { depth: u64 },       // only in WellDefinednessUnchecked
}

// InconclusiveCause gains:
//   WellDefinednessUnchecked { sources: Vec<WitnessSource> }
// TerminalValue::Proved carries FR-127's `certification` beside its basis.

pub struct StateGraphCertificate {
    pub package_id: PackageId,           // the package the engine explored
    pub closure: Vec<DigestRecord>,      // explored model states' state-key digests, ascending
    pub ranks: Vec<(u64, u32)>,          // `possible`, `always possible`: (closure index, d)
    pub counts: Vec<(u64, u8, u64)>,     // `unique path`: (closure index, count, order)
}

pub enum StateGraphCheck {
    Accepted,
    Rejected(CertificateRejection),          // FR-338's: rule and locus
    Stopped(ReachedLimit),                   // FR-126: the limit and its value
    PackageMismatch { certificate: PackageId, recompiled: PackageId },  // FR-282
}

pub fn check_state_graph(request: &CertificateRequest<'_>, claim: &StateGraphItem,
    certificate: &StateGraphCertificate) -> StateGraphCheck;
```

The checker uses FR-338's `CertificateRejection{rule, at}`. FR-338's
`CertificateRule` gains `PredicateUndefined`, `RankMissing`, `RankBroken`,
`OrderBroken` and `CountBroken` (`InitialMissing`, `SuccessorMissing`
and `BadState` are FR-338's own), and its `CertificateLocus` gains
`ModelState(DigestRecord)`, a closure entry's digest record, since a
state-graph certificate holds model states, not product states, and
`Entry { index: u64 }`, a `ranks` or `counts` entry's closure index that
names no closure state. The checker also rejects with FR-163's `Malformed`.

The `u64` that opens a `ranks` or `counts` entry is the zero-based index of
a state in `closure`, and the entry concerns that state. A closure entry's
digest record is in the `quire.simulation.state-key/v1` domain; a record in
any other domain is a malformed entry, and only a `Malformed` rejection
names a `ModelState` outside that domain (QSpec FR-331's State locus).

and the FR-331 terminal record carrying the value, its QSpec FR-360 label,
its QSpec FR-243 basis, its O-16 category and its settlement method.

`qsl-replay` SHALL hold two settlement entries over the one map below,
beside `check_state_graph`: `settle_state_graph`, which settles one state-
graph item result (an instance outcome or its item-level `NoInstance`
marker), and `settle_existential`, which settles one instance of an HP-5
item from FR-181's outcome. Each entry takes the item's outcome, the run's
method, seed and limits (FR-166), the replay request,
FR-276's `Cancel` handle and FR-170's replay of the instance's evidence, and
returns the instance's terminal record. `settle_state_graph` checks a
certificate with `check_state_graph`; `settle_existential` checks the
`ProductClosureCertificate` of a `Witnessed` with FR-163's
`check_product_closure`. FR-182's `settle_hyper_check` settles no HP-5
item.

## Behavior

- When it settles an HP-5 item, `settle_existential` SHALL read "the
  checker" in each row below as FR-163's `check_product_closure` and "the
  certificate" as the `ProductClosureCertificate` FR-181's `Witnessed`
  carries. An HP-5 item has no `Holds` and no `PathPair`.
- When it settles an HP-5 item, `settle_existential` SHALL settle each
  `ProductClosureCheck` alternative, `Accepted`, `Rejected`,
  `PackageMismatch` or `Stopped`, by the row that `StateGraphCheck`'s
  alternative of the same name takes, with the same cause.

- The map SHALL be exhaustive, with no `_` arm, and give each input exactly
  one row:

| Verdict | Input | QSpec FR-360 label | QSpec FR-243 basis | `TerminalValue` | O-16 category |
| --- | --- | --- | --- | --- | --- |
| V-9 | `Witnessed` whose replay settles `reproduced-with-evaluated-witness` and whose certificate the checker accepts | `proved` | `decisive-witness` | `Proved{basis: Witness{sources}, certification: Certified}` | success |
| V-10 | `Trapped` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `closed-scope` | `Refuted` | violation |
| V-4 | `PathPair` whose replay settles `reproduced-with-evaluated-witness` | `refuted` | `decisive-counterexample` | `Refuted` | violation |
| V-4 | `Undefined` whose replay reproduces the undefined value at `where` (FR-170) | `refuted`, cause `UndefinedEvaluation{where, cause}` | `decisive-counterexample` | `Refuted` | violation |
| V-1 | `Holds` whose certificate the checker accepts | `proved` | `closed-scope` | `Proved{basis: Exhaustive, certification: Certified}` | success |
| V-1 | `HoldsReduced{reductions}` | `proved` | `closed-scope` | `Proved{basis: Reduced{reductions}, certification: Uncertified}` | success |
| V-6 | `Witnessed` or `Holds` whose certificate the checker rejects | `inconclusive` | `unsettled` | `Inconclusive(CertificateRejected{rule, at})` (ADR-018 PC-2) | inconclusive |
| V-6 | `Witnessed` or `Holds` whose certificate the checker returns `PackageMismatch{certificate, recompiled}`; evidence whose replay refuses with `PackageIdMismatch{requested, recompiled}` (FR-170) | `inconclusive` | `unsettled` | `Inconclusive(StaleDependency{requested, recompiled})`, `requested` the certificate's or the request's `package_id` (FR-127) | inconclusive |
| V-5 | `NoDecision` with `end` `Completed`, open causes only `MaxDepth`, and no partial-order reduction | `inconclusive` | `unsettled` | `Inconclusive(BoundReached{depth: max_depth})` | inconclusive |
| V-6 | `WitnessedUnchecked` whose witnesses each replay to `reproduced-with-evaluated-witness` | `inconclusive` | `unsettled` | `Inconclusive(WellDefinednessUnchecked{sources})` | inconclusive |
| V-6 | `NoDecision` with `end` `Completed` and an `UndecidedSuccessor` open cause (`UndecidedSuccessor`), else a `ConstraintBoundary` one (`ConstraintReached`); `NoInitialState`; `NoInstance` (FR-125); `ReductionNotPreserving` (FR-167); evidence whose replay settles `inconclusive` (`ReplayParity`, wire cause `replay-parity`) or refuses with a `ReplayRefusal` other than `PackageIdMismatch` (`ReplayRefused`), a trap replay a limit stopped included (FR-170) | `inconclusive` | `unsettled` | `Inconclusive(cause)` | inconclusive |
| V-7 | `NoDecision` with `end` `Stopped(cause, limit)`; `NoDecision` with `end` `Completed` and open causes only `MaxDepth` under partial-order reduction (ADR-021 RV-5); `Stopped(ResourceExhausted, WitnessSamples)` (FR-166); a certificate check a limit stopped; a trap replay a limit stopped (FR-170) | `failed`, execution `resource-incomplete` | `unavailable` | `Incomplete(LimitReached{limit, value, setting})` (ADR-018 V-7, FR-127) | incomplete |
| V-8 | No candidate discharges the form | `unsupported` | `unavailable` | `Unsupported(cause)` | unsupported |

- `TerminalValue::category` SHALL map `Proved{basis: Witness{…}, certification}` to
  success, with either certification.

### State-graph certificate (ADR-022 GC-1 to GC-4)

- **Production.** With `Witnessed` or `Holds` from an unreduced
  exploration, the engine SHALL return a `StateGraphCertificate`: the
  `package_id` of the package it explored; every explored model state's `quire.simulation.state-key/v1` digest, sorted;
  for `possible` and `always possible`, each state's distance `d` to the
  nearest target node, for every state from which a target is reachable;
  for `unique path`, each node of `H`'s path count and its position in a
  reverse topological order of `H`.
- **Check.** `check_state_graph`, a layer-6 entry of `qsl-replay` beside
  FR-338's `check_closure`, SHALL recompile the package, re-admit the
  subject and recompute with its own code, reading nothing from the engine
  but the certificate. It SHALL return `Rejected` with FR-338's
  `CertificateRejection{rule, at}`, `at` the `ModelState` locus of
  the first failing state in certificate order, which settles
  `CertificateRejected{rule, at}`. Before any rule, the checker SHALL
  compare the recompiled package's `package_id` with the certificate's
  `package_id`; when they differ it SHALL return
  `PackageMismatch{certificate, recompiled}` naming both and evaluate no
  rule (FR-282), which settles `Inconclusive(StaleDependency{requested,
  recompiled})` with `requested` the certificate's `package_id`. The
  checker SHALL evaluate the rules in
  the order listed here, each over every reached state, and reject with the
  first rule that fails (for `unique path`, `OrderBroken` before
  `CountBroken`):
  - `Malformed`, before any recomputation, reading the certificate once in
    its stored order: at the `ModelState` of the first `closure` entry
    whose digest record is outside the `quire.simulation.state-key/v1`
    domain or is not strictly greater than the entry before it; then, over
    `ranks` and then over `counts`, at the first entry whose closure index
    is not strictly greater than the index before it, names no closure
    state, or, for `counts`, carries a count above 2. A rank or count entry
    rejects at the `ModelState` of the closure state its index names, or at
    `Entry { index }` when the index names none. A certificate carries only
    the list of its claim's form: a `counts` entry in a certificate for
    `possible` or `always possible`, or a `ranks` entry in one for
    `unique path`, rejects `Malformed` by the same locus rule, at the
    `ModelState` of the closure state its index names, or at
    `Entry { index }` when the index names none, read in the same pass after
    the entries before it;
    `Entry { index }` when the index names none;
  - `InitialMissing`, when an initial state is not in `closure`;
  - `SuccessorMissing`, when a successor that `ModelSystem` computes for a
    reached `closure` state is not in `closure`, as FR-338's `check_closure` does;
  - `BadState`, when the expansion of a reached `closure` state records
    `ContractUndetermined` (FR-120): a contract conjunction of the state is
    undefined or refused, so its successors are not known, as FR-338's
    `BadState` states;
  - `PredicateUndefined`, when a claim predicate evaluates undefined at a
    reached `closure` state;
  - for `possible`: `RankMissing`, when an initial state has no rank;
    for `always possible`: `RankMissing`, when a state where `from` holds
    has no rank; and for both, `RankBroken`, when a ranked state with
    `d = 0` fails the target or one with `d > 0` has no successor ranked
    `d - 1`;
  - for `unique path`: `OrderBroken`, when an edge of `H` does not go to a
    node earlier in the order, and `CountBroken`, when a node's count is not
    1 for a `to` node, otherwise the sum of its `H` successors' counts
    saturated at 2, or when a node where `from` holds counts other than 1.
- The checker SHALL NOT read an undefined or refused contract conjunction
  as false or as true. A closure entry no initial state reaches is never
  expanded and so never decides `BadState`.
- An accepted certificate establishes the claim over every reachable
  state, well-definedness included: the closure holds every reachable
  state, no predicate is undefined there, and the ranks or counts give the
  claim's truth (ADR-022 GM-4, GM-5).
- The checker SHALL count states against `max_states` and successors
  against `max_transitions`, with checked arithmetic, and return
  `Stopped` naming the reached limit and its value, which settles V-7,
  `incomplete`, `LimitReached{limit, value, setting}`.
- A proof under a reduction SHALL carry `certification: Uncertified`, with
  no certificate (ADR-022 GC-4).
- No item SHALL settle `proved` from a witness unless FR-168, or FR-181 for
  an HP-5 item, returned `Witnessed`, which it returns only from a run that completed with no open
  node and no undefined evaluation of the target.
- No item SHALL settle `proved` from a witness, or `refuted` from a trap,
  path pair or undefined evaluation, before its evidence replays (FR-170).
- An `Undefined` item SHALL settle `refuted` with cause
  `UndefinedEvaluation{where, cause}`, adding no label, basis or category
  (ADR-022 GV-7).
- **Settlement method.** The record SHALL carry the method
  `explicit-state` (or the SMT method for an unrolled witness) and:
  - for V-9, each initial state's `WitnessSource`; a `Sampled` source SHALL
    name its seed and trace index;
  - for V-6 `WellDefinednessUnchecked`, each witness's source, the run's
    end and the limits the run used;
  - for V-1, `Exhaustive`, or `Reduced` with its reductions;
  - for V-10 and V-4, that exploration found the evidence it carries;
  - for V-5 to V-7, the limits the run used.
- Every record SHALL state the search horizon `max_depth` the run used as a
  method parameter beside the limits, never as a limit (QSpec FR-392).
- Every record SHALL carry the run's seed and `witness_samples`, whether
  the request named them or the defaults applied, and the count of walks
  `max_walk_steps` stopped (FR-166), as QSpec FR-392 records them.
- `Witness{sources}` SHALL NOT carry an `Unrolled` source; an EN-2 witness
  settles only `WellDefinednessUnchecked` (ADR-022 GE-3).
- A V-5 record SHALL have execution `completed` and truth `pending`, as
  FR-127 states.
- A V-7 record SHALL name its limit, its value and the request setting that
  raises it.
- A verdict SHALL hold for exactly its subject; the obligation identity
  binds the subject, the claim and the instance (ADR-013 O-09).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-169-AC-1 | Each input row maps to its `TerminalValue`, FR-360 label, FR-243 basis and O-16 category exactly as the table states; `Proved{basis: Witness{sources}, certification: Certified}` maps to success. | Test (TC-594) |
| FR-169-AC-2 | ADR-022 §7.1's `ReachesTwo` under default limits settles `proved`, `decisive-witness`, `Proved{basis: Witness{[Sampled(…)]}, certification: Certified}`, success, and its record names the seed and trace index of each walk; with `witness_samples` 0 it settles the same label and basis with sources `[Explored]` and a record naming exploration. `ReachesThree` settles `refuted`, `closed-scope`, after its trap replays. | Test (TC-594) |
| FR-169-AC-3 | §7.2's `CanStillWin` settles `refuted`, `closed-scope` (V-10); its `from` variant `proved`, `closed-scope`, `Proved{Exhaustive, Certified}` (V-1). §7.3's `InOneWay` settles `refuted`, `decisive-counterexample` (V-4). | Test (TC-594) |
| FR-169-AC-4 | FR-168-AC-4's trap settles `refuted` (V-10) and its `NoDecision` settles `inconclusive`, `BoundReached{depth: 3}`, execution `completed`, truth `pending`, and its record states horizon 3 as a method parameter, not among the limits. A run stopped by `max_states` settles `incomplete`, `LimitReached`, naming `max_states` and its value; an undecided node with no decisive evidence settles `inconclusive`, `UndecidedSuccessor`; a subject with no initial state settles `inconclusive`, `NoInitialState`. | Test (TC-594) |
| FR-169-AC-5 | A `Witnessed` outcome whose replay settles `inconclusive` settles `inconclusive`, `ReplayParity`, never `proved`; a `Trapped` outcome whose replay a limit stops settles `incomplete`, `LimitReached`, naming the limit. | Test (TC-594) |
| FR-169-AC-6 | FR-168-AC-5's `Undefined` outcome settles `refuted`, `decisive-counterexample`, category violation, after FR-170 replay reproduces it, with cause `UndefinedEvaluation` naming the node `(2, 0)` and `division-by-zero`. | Test (TC-612) |
| FR-169-AC-7 | FR-168-AC-7's `Undefined` outcome, found although a sampled witness existed, settles `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`; `ReachesTwo` with default limits settles `proved`, `decisive-witness`, with `Sampled` sources, and its record names a completed exploration. | Test (TC-613) |
| FR-169-AC-8 | FR-168-AC-8's `WitnessedUnchecked` outcome settles `inconclusive`, `unsettled`, `Inconclusive(WellDefinednessUnchecked{sources})` with `Sampled` sources, naming `max_states` and its value, never `proved`; its `NoDecision` variant with `witness_samples` 0 settles `incomplete`, `LimitReached`. | Test (TC-614) |
| FR-169-AC-9 | Accepted certificates. `ReachesTwo` for `c = a` returns a certificate whose closure is §7.1's 9 states and whose ranks are `d = 0` at `va = 2`, 1 at `va = 1` and 2 at `va = 0`; `check_state_graph` accepts it and the item settles `Proved{Witness{…}, Certified}`. §7.2's `CanStillWin` `from (x.phase != Lost)` returns ranks `Won` 0, `Mid` 1, `Start` 2 and no rank for `Lost`, accepted, `Proved{Exhaustive, Certified}`. §7.3's sequenced `InOneWay` returns counts 1 at every node of `H` with a reverse topological order, accepted. | Test (TC-643) |
| FR-169-AC-10 | Rejected certificates, each settling `inconclusive`, `CertificateRejected{rule, at}` and never `proved`: AC-9's `ReachesTwo` certificate with `(2, 2)` removed (`SuccessorMissing` at the first, in certificate order, of the two closure states that reach it, `(1, 2)` and `(2, 1)`; the engine gives `(1, 2)`); with the rank of `(1, 0)` changed to 2 (`RankBroken` at `(0, 0)`, the only ranked state left with no successor one rank lower, because `(1, 0)` keeps its successor `(1, 1)` at rank 1); with `(0, 0)` unranked (`RankMissing` at `(0, 0)`); a hand-built certificate for FR-168-AC-5's item that lists all 9 states (`PredicateUndefined` at the first, in certificate order, of the three states where `va = 2`, because the target divides by zero at each; the engine gives `(2, 0)`); the `InOneWay` certificate over §7.3's unsequenced job with every count 1 and a valid reverse topological order (`CountBroken` at the node with two paths); the sequenced certificate with its order reversed (`OrderBroken` at the first, in certificate order, of its nodes other than the `to` node; the engine gives `tff`). The check with `max_states` 2 stops and settles `failed`, naming `max_states`. | Test (TC-644) |
| FR-169-AC-11 | A certificate holding a state whose expansion evaluates a contract conjunction undefined is rejected `BadState` at that state and settles `inconclusive`, `CertificateRejected{BadState, at}`, never `proved`: §7.1's subject with universe `{a}`, the one initial state `va = 0` and the guard `pre Guard using v on Config::ConfigVersion::attemptUpdate { 2 div (2 - self.versionNumber) >= 1 }` (the guard of TC-592's FR-167-AC-2 fixture), checked with a hand-built certificate of its 3 states ranked 2, 1 and 0 for `ReachesTwo`, is rejected `BadState` at the state where `va = 2`, whose guard has no value; the same ranks over a subject without the guard are accepted. | Test (TC-644) |
| FR-169-AC-12 | Package identity. AC-9's `ReachesTwo` certificate carries the `package_id` of the package the engine explored, equal to the recompiled one, and is accepted. The same certificate with its `package_id` replaced by another package's returns `PackageMismatch{certificate, recompiled}` naming both identities before any rule is evaluated, and the item settles `inconclusive`, `unsettled`, `Inconclusive(StaleDependency{requested, recompiled})` with `requested` the certificate's `package_id`, never `proved`. AC-9's `ReachesTwo` witness replayed with a request whose `package_id` is stale refuses `PackageIdMismatch` and the item settles `Inconclusive(StaleDependency{requested, recompiled})` with `requested` the request's `package_id`, not `ReplayRefused`. | Test (TC-1141) |
| FR-169-AC-13 | HP-5 settlement through `settle_existential`. FR-181-AC-1's `Witnessed` outcome with `witness_samples` 0, whose lassos replay and whose FR-163-AC-1 HP-5 certificate `check_product_closure` accepts, settles `proved`, `decisive-witness`, `Proved{basis: Witness{[Explored, Explored]}, certification: Certified}`, success, one `Explored` for each of the vault's two initial snapshots `(h, l) = (0, 0)` and `(1, 0)`. The same outcome settles `inconclusive`, `CertificateRejected{SuccessorMissing, at}` with one member of that certificate removed, `at` the member whose successor is missing; `inconclusive`, `StaleDependency{requested, recompiled}` with its `package_id` replaced by another package's, `requested` the certificate's; and `incomplete`, `LimitReached`, naming `max_states` and its value, when checked with `max_states` 1. None of these settles `proved`. With the last step of one witness's loop removed, its replay refuses as FR-128 refuses a loop that does not close and the item settles `inconclusive`, `ReplayRefused`. FR-181-AC-2's `Trapped` outcome settles `refuted`, `closed-scope`, after its product trap replays. | Test (TC-1198) |
| FR-169-AC-14 | Malformed certificates, each rejected `Malformed` before any successor is recomputed and settling `inconclusive`, `CertificateRejected{Malformed, at}`, never `proved`: AC-9's `ReachesTwo` certificate with its first two closure entries swapped (at the `ModelState` of the entry that is not greater than the one before it); with one closure entry duplicated (at the duplicate); with one closure entry's domain replaced by `package-fingerprint` (at that `ModelState`, in the `package-fingerprint` domain); with a rank entry whose index is the closure's length (at `Entry { index }`, that length); with two rank entries in descending index order (at the `ModelState` of the second entry's closure state); and the sequenced `InOneWay` certificate with one count set to 3 (at the `ModelState` of that entry's closure state). Each rejection's `at` is written in QSpec FR-331's State or State-graph entry form, the entry index as a `U64String`. | Test (TC-1197) |
| FR-169-AC-15 | A certificate holding the other form's list is rejected `Malformed` before any successor is recomputed and settles `inconclusive`, `CertificateRejected{Malformed, at}`, never `proved`: AC-9's `ReachesTwo` certificate with the `counts` entry `(0, 1, 0)` added, at the `ModelState` of closure state 0; the sequenced `InOneWay` certificate of AC-9 with the `ranks` entry `(0, 0)` added, at the `ModelState` of closure state 0; `ReachesTwo`'s certificate with the `counts` entry `(9, 1, 0)` added, its index naming no state of the 9-state closure, at `Entry { index: 9 }`. AC-9's `ReachesTwo` and sequenced `InOneWay` certificates as the engine returns them, with an empty list of the other form, are accepted. | Test (TC-644) |
| FR-169-AC-16 | A state-graph claim with no instances under FR-125 produces one item-level `NoInstance` marker (not a per-instance outcome), contributes no predicate labels, and settles directly as one terminal record `inconclusive`, `Inconclusive(NoInstance)`, never `proved`, without certificate checking or replay; when it is the only requested work, no state is explored. A claim with one or more instances proceeds through the state-graph outcome map. | Test (TC-594) |
| FR-169-AC-17 | The engine-produced `HoldsReduced` from FR-168-AC-14 for `always possible` under symmetry settles as `proved`, `closed-scope`, `Proved{basis: Reduced{[Symmetry{config_history, [[a, b, c]]}]}, certification: Uncertified}`, success, without calling `check_state_graph`; the same claim's unreduced `Holds` is checked and settles `Proved{basis: Exhaustive, certification: Certified}`. `ReachesTwo` under POR returns `HoldsReduced{[PartialOrder{BreadthFirstRevisit}]}` and settles `Proved{basis: Reduced{[PartialOrder{BreadthFirstRevisit}]}, certification: Uncertified}`, success, without a certificate check or replay. | Test (TC-594) |

## Dependencies

- ADR-022 §4 GV-1 to GV-6 and "Settlement method"; §10 RU-4 and RU-5; ADR-018 §1
  (V-1 to V-8); ADR-013 O-16 and O-24 as amended by ADR-018.
- [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the map this extends), [FR-069](FR-069-implement-typed-proof-result-envelope.md),
  [FR-072](FR-072-implement-typed-replay-result.md),
  [FR-168](FR-168-decide-state-graph-claims-over-the-explored-graph.md),
  [FR-170](FR-170-replay-state-graph-evidence.md).
- QSpec FR-391 owns the verdict table onto FR-360 and FR-243 and the
  widening of FR-360's scope to state-graph results; QSpec FR-392 owns the
  settlement method and seed in the terminal record (ADR-022 QS-4, QS-7).

## References

- ADR-022. QSpec half: QSpec FR-391 and FR-392 (Linear STD-135; ADR-022
  QS-4, QS-7).

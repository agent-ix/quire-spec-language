---
id: FR-281
title: "Analyze claims over every behaviour with QSL's in-process engines"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-507
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
  - target: "ix://agent-ix/quire-specification/FR-331"
    type: depends_on
---
# FR-281: Analyze claims over every behaviour with QSL's in-process engines

## Description

`analyze` decides claims over every behaviour of a model with QSL's
in-process engines (ADR-029 OP-2). The engines are the ones in QSL layer A,
crate `qsl-analyze`: ADR-018's EN-1 model checker, ADR-024's EN-4
statistical engine, ADR-028's EN-5 exact probabilistic engine and ADR-026's
zone search, and each engine later added to layer A. Those records own each
engine's semantics; this requirement owns the operation that runs them and
settles their results.

A `proved` result is `certified` once the engine's certificate
checker in the qualified core accepts its certificate (FR-282). A proof from
a native engine with no core checker is `proved`, labelled `uncertified` (FR-290). A counterexample counts only
after S6a replay reproduces it.

An `analyze` item is a proof item. An evaluation of the claim that is
undefined at some state or step, in any engine, settles the item `refuted`
with cause `UndefinedEvaluation{where, cause}`: the claim did not hold there.
The item's category is violation, it exits 10 (FR-285), and it never carries
the `undefined` label (FR-286).

### Negotiated items

`analyze` does not negotiate and never chooses among engines. Negotiation is
not a QSL stage: CG's `negotiate_*` is the single negotiation point (QSpec
AD-016, FR-057), it settles each item from its `candidates` and extent under
QSpec FR-290's candidate table, it never selects among several candidates,
and QSL depends on no CG crate (FR-280). So the caller negotiates, as it does
for `prove`: it builds a registry from the manifests of the engines it
passes (FR-288), computes each item's `candidates` by FR-075, settles each
item with `negotiate_*`, and hands `analyze` each item with its settlement.
Routing then follows FR-057: an item settled `supported` runs on its one
candidate.

- An item settled `Supported(candidate)` runs on the engine in `engines`
  whose `BackendId` is that candidate, and on no other engine.
- An item settled `requires-bound`, `unsupported` or `invalid-request` runs
  on no engine. Its terminal record is settled by that disposition (QSpec
  FR-331 `settled_by` `disposition`) and keeps the disposition's FR-272
  diagnostic. An item with more than one candidate and no named backend
  therefore reaches `analyze` settled `invalid-request` with
  `invalid_capability`/`ambiguous-backend`, and an item no engine advertises
  settled `unsupported` with `unsupported_projection`/
  `unsupported-requested-capability` naming its kind, exactly as through
  `prove`. That record carries the diagnostic and no `cause`: QSpec FR-331's
  closed `unsupported` causes hold no claim-kind cause, so there is no
  `unsupported-claim-kind` cause, and the claim kind travels in the
  diagnostic's FR-272 payload (the item's FR-290 kind, and its
  `requirements` when it carries them).
- `analyze` reads no manifest to route an item: it computes no candidate set,
  and the only dispositions it settles are FR-507's `inapplicable-kind` and
  the `unknown-backend` and `inconsistent-candidates` checks below. An
  item's engine is its `Supported` candidate
  and no other, so an item two engines could decide reaches `analyze` only as
  the `invalid-request`/`ambiguous-backend` disposition above.
- A `Supported` candidate that `engines` does not hold settles the item
  `invalid-request` with `invalid_capability`/`unknown-backend` naming it,
  FR-290's first row, and no engine runs.
- A `Supported` candidate that `engines` holds, but whose manifest does not
  advertise the item's (kind, mode) pair, is a disposition negotiation could
  not have given (QSpec FR-290's candidate table admits only an advertising
  backend). It settles the item `invalid-request` with
  `invalid_capability`/`inconsistent-candidates` naming the candidate and the
  item's kind and mode, and no engine runs.

`analyze` judges each item in this order, and the first rule that applies
settles it:

1. FR-507's declaration-family check. An item whose kind does not apply to
   its claim's declaration family settles `invalid-request` with
   `invalid_capability`/`inapplicable-kind`, whatever disposition it was
   handed: `Supported` on any candidate (held or not, advertising its
   (kind, mode) or not), `RequiresBound`, `Unsupported`, or `InvalidRequest`
   with any other cause, `ambiguous-backend` included. This is QSpec FR-290's
   order, in which the check precedes every candidate-table cause, so the
   record equals the one a conforming `negotiate_*` gives.
2. A handed `RequiresBound`, `Unsupported` or `InvalidRequest`: a record
   settled by that disposition with its diagnostic.
3. `Supported(candidate)` with no engine in `engines` under that `BackendId`:
   `unknown-backend`.
4. `Supported(candidate)` on an engine that does not advertise the item's
   (kind, mode): `inconsistent-candidates`.
5. Otherwise the item runs on its candidate.

`Engines` is the set of in-process engines `analyze` and the provider entry
(FR-280) may run, each by its `BackendId` with its FR-331 manifest.
`Engines::qsl()` holds every layer-A engine, each under its own
`BackendId`. `Engines::with` adds an implementation of `qsl_analyze::Engine`,
the trait every layer-A engine implements: its manifest, and its run over one
item, which returns the engine's own result (a proof with its certificate or
none, a counterexample, an undefined evaluation, a budget stop, or the
engine's refusal) for `analyze` to settle. `Engines` holds at most one engine
per `BackendId`, so a candidate names at most one engine: `Engines::with`
given an engine whose manifest's `BackendId` the set already holds returns
`Err(DuplicateEngine)`, `invalid_capability`/`duplicate-backend` naming that
`BackendId` (the cause QSpec FR-290 gives a registry holding one backend
twice), and the set it was called on is unchanged. Tests use
it to run test engines. SMT, Kani and plugin providers are not `Engine`s:
they reach a claim through the driver's `prove`, never through `analyze`
(ADR-029 OP-2).

### External provider arm

`analyze` also runs a registered external provider: a decision procedure
outside QSL, such as an SMT solver, that decides a `value-validity` item over
unbounded mathematical `Integer` operands, which no in-process engine decides
(FR-495). The provider is a QSL-owned interface. It receives the item's claim
target and reports one of three verdicts, `proved`, `refuted` with a
counterexample assignment, or `unknown`; its types are QSL's own, so QSL
depends on no CG crate for it (FR-280).

- An engine is built from a provider only with a descriptor that advertises
  `value-validity` in the `unbounded` mode. A descriptor that does not is
  refused, naming its `BackendId`, and no engine is made. The provider is
  registered beside the in-process engines under its own `BackendId`; it never
  replaces one, and `Engines::with` keeps FR-281's one-engine-per-`BackendId`
  rule for it.
- A provider item runs on its provider only when it reaches `analyze` settled
  `Supported` on that provider's `BackendId`. Registering a provider does not
  change any other item: an item settled for a bounded engine runs on that
  engine and no other, and a bounded engine's item whose operands are
  unbounded settles as it would with no provider registered.
- The verdict is the provider's; QSL adds no verdict route of its own. A
  `proved` verdict settles the item `proved`, proof basis `inductive` at depth
  0, naming the provider, with certification `uncertified` for a provider
  registered as first-party (descriptor origin `Linked`) and `trusted` for a
  plugin's (origin `Process`), as QSpec FR-331 and FR-354 give each; no core
  checker covers either proof. An `unknown` verdict settles the item
  `inconclusive` with cause `solver-inconclusive`. A `refuted` verdict is a candidate
  violation, not a result: QSL replays the counterexample through the native
  evaluator at the assignment's own integers, never narrowed to any `Int[..]`
  (the replay of "A counterexample counts only after S6a replay reproduces
  it"). The item settles `refuted` and carries the counterexample only when
  the replay reproduces the violation; a counterexample that is not an
  assignment of the claim's parameters, or that the replay does not reproduce,
  settles the item `inconclusive` with cause `replay_parity`, and a
  counterexample that exhausts the case budget in replay settles `incomplete`
  with the case-budget limit cause. Cancellation ends a provider item as it
  ends any engine's: a cancelled run settles the item `incomplete` with the
  cancellation cause, and a cancellation that fires during the provider's
  check or the replay stops the replay the same way. A provider that panics is
  a caller defect, handled as a panic in any other engine is: `analyze` does
  not catch it.

### Engine refusals

An engine's settlement map (FR-127 for EN-1, FR-192 for EN-4, FR-203 for
EN-5) gives some engine refusals no terminal record and leaves them to the
caller. `analyze` is that caller, and it returns exactly one record per item,
so it SHALL settle each such refusal of a `Supported` item by this table,
with the record naming the engine:

| Engine refusal | Record |
| --- | --- |
| `Admission(failure)` (FR-126 `ModelCheckRefusal`, FR-192, FR-203) | `declined`, cause `invalid-input` for a refused admission and `incomplete-input` for an incomplete one, with the failure's FR-272 diagnostic, as FR-280's provider entry settles a refused binding |
| `RequiresBound` (FR-126 `ModelCheckRefusal::RequiresBound`, FR-192, FR-203 `NotSimulated(RequiresBound)`) | `unsupported` with the diagnostic `unsupported_projection`/`unbounded-extent`, whose payload is QSpec FR-272's for that cause: the item's FR-290 kind, the candidate (the engine's `BackendId`) and the modes its manifest advertises for that kind (QSpec FR-272-AC-9). FR-272 closes that payload, so the unbounded root FR-101's `RequiresBound` names is not written in the record |
| EN-4 `Symmetry` (a malformed symmetry declaration, FR-151) | `declined`, cause `invalid-input`, diagnostic `invalid_runtime_input`/`invalid-value` |
| EN-4 `NotStatistical` | `invalid-request` with `invalid_capability`/`not-statistical`, settled by that disposition and carrying no result (QSpec FR-331) |
| Any other engine refusal with no settlement-map row, whose FR-272 diagnostic code is `unsupported_construct` or `unsupported_projection` | `unsupported` with that diagnostic |
| Any other engine refusal with no settlement-map row | `failed` with its FR-272 diagnostic |

An engine refusal never settles `proved`, `refuted` or `measured`, and no item
is left without a record.

## Inputs

```rust
pub fn analyze(
    package: &CheckedPackage,
    request: &AnalyzeRequest,
    engines: &Engines,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
) -> Result<Staged<AnalyzeOutcome>, StageFailure<AnalyzeRefusal>>;

pub struct AnalyzeRequest {
    pub binding: SubjectBinding,
    pub items: Vec<NegotiatedItem>,
}

pub struct NegotiatedItem {
    pub item: ClaimItem,
    pub disposition: Disposition,
    pub diagnostic: Option<DispositionDiagnostic>,
}

pub struct AnalyzeLimits {
    pub accounting: ScalarLimits,
    pub observation: ObservationLimits,
    pub normalization: ModelNormalizationLimits,
    pub deadline: DeadlineSupply,
}

pub enum DeadlineSupply {
    Unbounded,
    Milliseconds(u64),
}
```

- `&CheckedPackage`.
- `SubjectBinding`: the subject the claims are decided over, FR-120's
  `ModelSimulationRequest` members other than the package: `packages`
  (FR-056's domain package input), `snapshots` (the FR-106 snapshot
  provision), `initial: Vec<DocumentRef>` and `universes:
  Vec<PopulationUniverse>`. `analyze` admits it as `ModelSystem::new`
  admits a request (FR-120) before any item runs.
- `ClaimItem`: one requested item, the typed value of a QSpec FR-331
  request item in its in-process form ("Items"): `request_index`, `claim`
  (`{node, occurrence}`, the item's occurrence key), `kind`, `extent` with
  its proof bounds, the named `backend` when the caller names one,
  `candidates`, `options`, and the kind's own member (`temporal`,
  `probabilistic` or `hybrid`) with its settings and limits, for example
  ADR-018's model-check limits. `request_index` values are `0..n` in
  `items` order.
- `NegotiatedItem`: a `ClaimItem` with the settlement `negotiate_*` gave it:
  `disposition`, FR-057's `qsl_route::Disposition` (`Supported(candidate)`,
  `RequiresBound`, `Unsupported` or `InvalidRequest`), and `diagnostic`, the
  FR-272 code, cause and payload of an `unsupported` or `invalid-request`
  disposition (QSpec FR-331), present exactly for those two.
- `Engines`: the engines "Negotiated items" describes.
- `AnalyzeLimits`: the accounting limits, the observation and
  model-normalization limits the binding is admitted under, each with its
  published default (FR-277), and the request's deadline `deadline`.
  `DeadlineSupply` has no `Default`, so `AnalyzeLimits` has none and the
  caller supplies `deadline` explicitly, as FR-277 has callers supply
  `ScalarLimits` and `supplied`: `Unbounded`, or `Milliseconds(n)`, the
  QSpec FR-331 `deadline_ms` of the request. `analyze` reads no clock and
  never measures the deadline (FR-276). The frontend that owns the timer
  arms it from `deadline` at the call and, when it expires, cancels the
  request's `Cancel` handle with `CancelCause::Deadline`; `analyze` sees the
  expiry only through that handle. `analyze` passes `deadline` to no engine:
  it is part of the request so that the request states its deadline and the
  timer is armed from that one value.
- `&Cancel`.

## Outputs

```rust
pub struct AnalyzeOutcome {
    pub records: Vec<TerminalRecord>,
}

pub enum AnalyzeRefusal {
    Binding(AdmissionFailure),
}
```

`Staged<AnalyzeOutcome>`: exactly one terminal record per requested item,
in `request_index` order, with the FR-331 result vocabulary, its O-16
category, its typed cause, its basis, its counterexample or certificate
where it has one, and its accounting. A record of an item an engine ran
names that engine's `BackendId` and is settled by its result; a record of an
item settled `requires-bound`, `unsupported` or `invalid-request`, or refused
`inapplicable-kind` by FR-507, is settled by that disposition, names no engine
and carries its diagnostic. An undefined
claim evaluation gives a `refuted` record, category violation, with cause
`UndefinedEvaluation{where, cause}` and the point where the evaluation was
undefined as its counterexample. A binding that FR-120 admission does not admit
fails the whole request with `StageFailure::Refused(AnalyzeRefusal::Binding)`
holding the `AdmissionFailure`, and no record.

## Behavior

- The `analyze` operation shall return exactly one terminal record for each
  requested item.
- When an item's disposition is `Supported(candidate)` and `engines` holds
  that candidate, the `analyze` operation shall run the item on that engine
  only.
- If an item's kind does not apply to its claim's declaration family
  (FR-507), then `analyze` shall settle the item, without running any engine
  for it, `invalid-request` with `invalid_capability`/`inapplicable-kind`,
  whatever disposition the item was handed, before any other rule below.
- If an item's kind applies to its claim's family and its disposition is
  `RequiresBound`, `Unsupported` or `InvalidRequest`, then `analyze` shall
  return, without running any engine for it, a record settled by that
  disposition with its diagnostic.
- If an item's kind applies to its claim's family, its disposition is
  `Supported(candidate)` and `engines` holds no
  engine with that `BackendId`, then `analyze` shall settle the item, without
  running any engine for it, `invalid-request` with
  `invalid_capability`/`unknown-backend` naming the candidate.
- The `analyze` operation shall run an item only on the candidate its
  disposition names, and never choose among engines.
- If an item's kind applies to its claim's family, its disposition is
  `Supported(candidate)` and the engine `engines`
  holds for that candidate does not advertise the item's (kind, mode), then
  `analyze` shall settle the item, without running any engine for it,
  `invalid-request` with `invalid_capability`/`inconsistent-candidates`.
- If `Engines::with` is given an engine whose `BackendId` the set holds, then
  it shall return `invalid_capability`/`duplicate-backend` naming that
  `BackendId` and leave the set unchanged.
- When an engine refuses an item and its settlement map gives the refusal no
  terminal record, `analyze` shall settle the item by "Engine refusals".
- If FR-120 admission does not admit the subject binding, then `analyze` shall
  return `StageFailure::Refused(AnalyzeRefusal::Binding)` with the
  admission failure, before any item runs.
- When an engine reports a proof with a certificate, `analyze` shall run the
  engine's certificate checker (FR-282) and settle the item `proved`, with
  the certification `certified`, only if the checker accepts the
  certificate.
- If the certificate checker rejects the certificate, then `analyze` shall
  settle the item `inconclusive` with cause `CertificateRejected` and the
  checker's typed rejection.
- If an engine reports a proof and the qualified core holds no certificate
  checker for that engine, then `analyze` shall settle the item `proved`,
  category success, with the certification `uncertified` naming the engine.
- When an engine reports a counterexample, `analyze` shall replay it through
  layer-6 `replay` (FR-098) and settle the item `refuted` only if the replay
  reproduces it.
- If the replay does not reproduce the counterexample, then `analyze` shall
  settle the item `inconclusive` with cause `replay_parity`; if the replay
  refuses, with cause `replay_refused` and the refusal's catalog code.
- If an engine's evaluation of an item's claim is undefined at a state or
  step, then `analyze` shall replay the path to that point through layer-6
  `replay` and, when the replay reproduces the undefined value, settle the
  item `refuted`, category violation, with cause
  `UndefinedEvaluation{where, cause}`, where `where` locates the state or
  step and `cause` is the undefined reason.
- The `analyze` operation shall give no item the `undefined` label.
- When an engine budget or an accounting limit is reached, or the `Cancel`
  handle is cancelled, `analyze` shall settle each open item `incomplete`
  with the limit or cancellation cause kept.
- The `analyze` operation shall read no clock.
- When the `Cancel` handle is cancelled with `CancelCause::Deadline`, the
  `analyze` operation shall settle each open item `incomplete` with cause
  `cancelled{source: deadline}` and shall preserve records already settled.
  When it is cancelled with `CancelCause::Requested`, each open item shall
  carry `cancelled{source: requested}`; if an engine or accounting limit
  stops an item first, `analyze` shall keep that item's typed limit
  result. A binding admission failure shall still return
  `StageFailure::Refused(AnalyzeRefusal::Binding)` before any item runs.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-281-AC-1 | Over a finite probabilistic model, an EN-5 item claiming a reachability probability bound that holds settles `proved` with category success after the EN-5 certificate checker accepts its certificate, and the record carries the certificate. | Test (TC-762) |
| FR-281-AC-2 | Over a finite model whose invariant one transition breaks, an EN-1 item claiming that invariant settles `refuted`, the record carries the counterexample, and replaying that counterexample through `replay` reproduces it. | Test (TC-762) |
| FR-281-AC-3 | A request of three items over `Engines::qsl()`, one whose claim kind no layer-A engine advertises, settled `Unsupported` with the diagnostic `unsupported_projection`/`unsupported-requested-capability` naming that kind (the settlement `negotiate_*` gives its empty candidate set), one an engine proves and one an engine refutes, each settled `Supported` on its engine, returns exactly three records, the first settled by its `unsupported` disposition with that diagnostic naming the claim kind. | Test (TC-762) |
| FR-281-AC-4 | With the test engine's certificate altered in one member before checking, the item settles `inconclusive` with cause `CertificateRejected`; with an engine that reports a proof no core certificate checker covers (EN-1 under partial-order reduction, ADR-021 RV-1), the item settles `proved`, category success, with the certification `uncertified` naming that engine, and the same item unreduced settles `certified`. This clause is observed at `analyze` over an engine set holding EN-1: the reduced item is an EN-1 item whose request selects partial-order reduction (FR-159), so EN-1 reports `HoldsReduced` and no engine or checker-less seam is added to `analyze`. The step engine's absent-certificate mapping (`BackendCertificate::Absent` to `uncertified`, FR-146) is a separate rule and does not satisfy this clause. | Test (TC-763) |
| FR-281-AC-5 | With a test engine that reports a counterexample replay does not reproduce, the item settles `inconclusive` with cause `replay_parity`; with one whose counterexample names a parameter the function lacks, `inconclusive` with cause `replay_refused`. | Test (TC-763) |
| FR-281-AC-6 | The AC-1 item with its engine's state budget set to 1 settles `incomplete` with the state-budget cause. | Test (TC-763) |
| FR-281-AC-7 | Over a finite model with one reachable state at which the invariant `100 / x > 0` divides by zero, an EN-1 item claiming that invariant settles `refuted`, category violation, with cause `UndefinedEvaluation` whose `where` names that state and whose `cause` is `division-by-zero`; replay reproduces the undefined value at that state, the record carries no `undefined` label, and FR-285 maps the outcome to exit 10. | Test (TC-763) |
| FR-281-AC-8 | With `engines` holding test engines `qsl.test-a` and `qsl.test-b`, both advertising (`value-validity`, `bounded`): an item settled `Supported(qsl.test-b)` runs on `qsl.test-b` alone and its record names it; the same item settled `InvalidRequest` with `invalid_capability`/`ambiguous-backend` naming `qsl.test-a` and `qsl.test-b`, as `negotiate_*` settles it with both candidates and no named backend (QSpec FR-290, FR-331-AC-4), returns a record settled by that disposition with that diagnostic and runs neither engine; settled `Unsupported` with `unsupported_projection`/`unsupported-requested-capability`, a record settled by that disposition and no run; settled `RequiresBound`, a record settled by that disposition with no diagnostic and no run; and settled `Supported(qsl.test-c)`, which `engines` does not hold, `invalid-request` with `invalid_capability`/`unknown-backend` naming `qsl.test-c` and no run. | Test (TC-762) |
| FR-281-AC-9 | AC-3's request returns its three records in `request_index` order 0, 1, 2. The same request whose binding's `universes` name a population the package does not declare returns `StageFailure::Refused(AnalyzeRefusal::Binding)` holding FR-120's `invalid_runtime_input`/`wrong-role-mapping`, no record, and no engine runs. | Test (TC-762) |
| FR-281-AC-10 | `Engines::with` holding `qsl.test-a`, given a second engine whose manifest's `BackendId` is `qsl.test-a`, returns `invalid_capability`/`duplicate-backend` naming `qsl.test-a`, and the set still runs the first engine for an item settled `Supported(qsl.test-a)`. With `qsl.test-a` advertising only (`value-validity`, `bounded`), a `temporal-satisfaction` item settled `Supported(qsl.test-a)` settles `invalid-request` with `invalid_capability`/`inconsistent-candidates` naming `qsl.test-a` and the item's kind and mode, and `qsl.test-a` does not run. AC-3's unsupported record carries the diagnostic naming the claim kind and no `cause` member. | Test (TC-762) |
| FR-281-AC-11 | With test engines that each refuse a `Supported` item, `analyze` settles: an `Admission` refusal `declined` with cause `invalid-input` and its diagnostic, and an incomplete admission `declined` with `incomplete-input`; a `RequiresBound` refusal `unsupported` with `unsupported_projection`/`unbounded-extent` whose payload names the item's kind, the engine's `BackendId` and the modes its manifest advertises for that kind; a `Symmetry` refusal `declined`, `invalid-input`, `invalid_runtime_input`/`invalid-value`; a `NotStatistical` refusal `invalid-request` with `invalid_capability`/`not-statistical` and no result; a refusal carrying `unsupported_construct`/`not-yet-implemented` `unsupported` with that diagnostic; and a refusal carrying `invalid_runtime_input`/`wrong-value-kind` `failed` with that diagnostic. Each record names its engine. One request holding the refused-admission case and the five cases after it returns those six records in `request_index` order; the incomplete admission is a seventh case, run as a request of its own because a test engine reports one admission outcome per run, and returns one `declined`, `incomplete-input` record. Over a model one of whose operations `modifies` a field of type `Integer` (an unbounded FR-120 root), an EN-1 item over `Engines::qsl()` settled `Supported` on EN-1 settles `unsupported` with `unsupported_projection`/`unbounded-extent` whose payload names `temporal-satisfaction`, EN-1's `BackendId` and `bounded`, EN-1's one advertised mode. | Test (TC-763) |
| FR-281-AC-12 | A `compile_fail` doctest that builds `AnalyzeLimits` with `..Default::default()` and no `deadline` fails to compile, with a compiling control that states `deadline`. A three-item request with `deadline` `Milliseconds(1)` and an uncancelled handle runs every item to its own settlement, whatever the elapsed time: `analyze` never measures the deadline. The same request with the handle cancelled `CancelCause::Deadline` from a second thread after the first item settles settles the two open items `incomplete` with cause `cancelled{source: deadline}` and keeps the first record unchanged; cancelled `Requested` first, open items carry `cancelled{source: requested}`; if an engine or accounting limit stops an item first, `analyze` keeps that item's typed limit result. A binding admission failure still returns `StageFailure::Refused(AnalyzeRefusal::Binding)` before any item runs. | Test (TC-763) |
| FR-281-AC-13 | With `engines` holding `qsl.test-a` advertising only (`value-validity`, `bounded`), a `finite-replay` item on a state clause's claim settles `invalid-request` with `invalid_capability`/`inapplicable-kind`, payload `family` `state-model`, and no engine runs, in each case: handed `Supported(qsl.test-c)`, which `engines` does not hold (not `unknown-backend`); handed `Supported(qsl.test-a)`, which does not advertise its (kind, mode) (not `inconsistent-candidates`); and handed `InvalidRequest` with `invalid_capability`/`ambiguous-backend` (not that disposition). A `temporal-satisfaction` item on a temporal clause's claim, whose kind applies, handed the same three settles `unknown-backend` naming `qsl.test-c`, `inconsistent-candidates` naming `qsl.test-a`, and `ambiguous-backend` respectively. | Test (TC-762, TC-1302) |
| FR-281-AC-14 | With a test provider registered under `qsl.test-smt` advertising (`value-validity`, `unbounded`) beside the value-validity arm, the item `function q using v(a: Integer, b: Integer): Integer pure { a div b }` settled `Supported(qsl.test-smt)` whose provider reports `refuted` with the counterexample `a` = 2^100, `b` = 0 settles `refuted`, category violation, naming `qsl.test-smt`, and the record carries that counterexample at those integers; the replay of the counterexample reproduces the `division-by-zero` violation at the `div`. The item `if b != 0 then a div b else 0` over the same parameters, whose provider reports `proved`, settles `proved` with basis `inductive` at depth 0, certification `uncertified`, naming `qsl.test-smt`. | Test (TC-1337) |
| FR-281-AC-15 | With the AC-14 provider reporting `refuted` with `a` = 7, `b` = 2 (an assignment at which the claim holds), with `b` = 0 and a missing second operand, or with a Boolean in place of an `Integer`, the item settles `inconclusive` with cause `replay_parity` and no counterexample, never `refuted`. With the provider reporting `unknown` the item settles `inconclusive` with cause `solver-inconclusive`. | Test (TC-1337) |
| FR-281-AC-16 | With the AC-14 provider registered beside the value-validity arm, the arm's item over the AC-14 claim settled `Supported(value-checker)` settles as it does with no provider registered: `unsupported` with `unsupported_projection`/`unbounded-extent`, naming `value-checker`, and the provider is not called for it; the same record results with no provider registered. | Test (TC-1337) |
| FR-281-AC-17 | Building an engine from a provider with a descriptor advertising only (`value-validity`, `bounded`) is refused naming that descriptor's `BackendId`, and with one advertising (`value-validity`, `unbounded`) succeeds; the refused provider is never called. | Test (TC-1337) |
| FR-281-AC-18 | With the AC-14 `proved` provider registered under a descriptor of origin `Linked` the record's certification is `uncertified`; under an otherwise identical descriptor of origin `Process` it is `trusted`; basis `inductive` at depth 0 in both. | Test (TC-1337) |
| FR-281-AC-19 | With the AC-14 `refuted` provider whose descriptor publishes `max_cases` 1, the item settles `incomplete` with the `ValueCheckMaxCases` limit cause and no counterexample. With the run's `Cancel` cancelled before `analyze`, the provider item settles `incomplete` with `cancelled` and the provider is not called; with a provider that cancels the run during its check and reports the AC-14 counterexample, the item settles `incomplete` with `cancelled`, never `refuted`. | Test (TC-1337) |

## Dependencies

- ADR-029 OP-2, CB-3: the operation and layer A.
- ADR-013 O-16, O-24: categories and terminal records.
- ADR-011 FB-07: replay through S6a.
- [FR-495](FR-495-reach-the-integer-division-zero-divisor-and-state-core-rows-from-qsl-source.md): the unbounded `Integer` division items the external provider arm decides.
- ADR-018, ADR-024, ADR-026, ADR-028 (drafts): the engines and their
  certificates.
- [FR-282](FR-282-check-analyze-certificates-in-the-qualified-core.md): the certificate checkers.
- [FR-098](FR-098-execute-a-replay-request.md): replay.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md): the subject binding.
- [FR-507](FR-507-apply-each-capability-kind-to-its-declaration-family-only.md): the declaration-family check that precedes every other disposition rule.
- QSpec FR-300: the `analyze` operation; QSpec FR-331: the result vocabulary.

## References

- QSL-393 (V1-A06): typed library APIs.
- QSL-390 (ARCH-50): rulings RU-2, RU-3 and RU-6 (an engine with no core
  checker settles `proved`, labelled `uncertified`), recorded on the ticket.
- QSL-366: an undefined claim evaluation settles `refuted` with cause
  `UndefinedEvaluation{where, cause}`, the cross-cutting ruling recorded on
  the ticket.
- QSpec FR-300 (STD-141): the QSpec half.

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
  `prove`.
- A `Supported` candidate that `engines` does not hold settles the item
  `invalid-request` with `invalid_capability`/`unknown-backend` naming it,
  FR-290's first row, and no engine runs.

`Engines` is the set of in-process engines `analyze` and the provider entry
(FR-280) may run, each by its `BackendId` with its FR-331 manifest.
`Engines::qsl()` holds every layer-A engine. `Engines::with` adds an
implementation of `qsl_analyze::Engine`, the trait every layer-A engine
implements: its manifest, and its run over one item, which returns the
engine's own result (a proof with its certificate or none, a counterexample,
an undefined evaluation, a budget stop) for `analyze` to settle. Tests use
it to run test engines. SMT, Kani and plugin providers are not `Engine`s:
they reach a claim through the driver's `prove`, never through `analyze`
(ADR-029 OP-2).

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
    pub deadline_ms: Option<u64>,
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
  model-normalization limits the binding is admitted under, and the
  wall-clock deadline `deadline_ms` (QSpec FR-331), each with its published
  default (FR-277).
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
item settled `requires-bound`, `unsupported` or `invalid-request` is settled
by that disposition, names no engine and carries its diagnostic. An undefined
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
- If an item's disposition is `RequiresBound`, `Unsupported` or
  `InvalidRequest`, then `analyze` shall return, without running any engine
  for it, a record settled by that disposition with its diagnostic.
- If an item's disposition is `Supported(candidate)` and `engines` holds no
  engine with that `BackendId`, then `analyze` shall settle the item, without
  running any engine for it, `invalid-request` with
  `invalid_capability`/`unknown-backend` naming the candidate.
- The `analyze` operation shall run an item only on the candidate its
  disposition names, and never choose among engines.
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

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-281-AC-1 | Over a finite probabilistic model, an EN-5 item claiming a reachability probability bound that holds settles `proved` with category success after the EN-5 certificate checker accepts its certificate, and the record carries the certificate. | Test (TC-762) |
| FR-281-AC-2 | Over a finite model whose invariant one transition breaks, an EN-1 item claiming that invariant settles `refuted`, the record carries the counterexample, and replaying that counterexample through `replay` reproduces it. | Test (TC-762) |
| FR-281-AC-3 | A request of three items over `Engines::qsl()`, one whose claim kind no layer-A engine advertises, settled `Unsupported` with the diagnostic `unsupported_projection`/`unsupported-requested-capability` naming that kind (the settlement `negotiate_*` gives its empty candidate set), one an engine proves and one an engine refutes, each settled `Supported` on its engine, returns exactly three records, the first settled by its `unsupported` disposition with that diagnostic naming the claim kind. | Test (TC-762) |
| FR-281-AC-4 | With the test engine's certificate altered in one member before checking, the item settles `inconclusive` with cause `CertificateRejected`; with a test engine that reports a proof and has no certificate checker, the item settles `proved`, category success, with the certification `uncertified` naming that engine. | Test (TC-763) |
| FR-281-AC-5 | With a test engine that reports a counterexample replay does not reproduce, the item settles `inconclusive` with cause `replay_parity`; with one whose counterexample names a parameter the function lacks, `inconclusive` with cause `replay_refused`. | Test (TC-763) |
| FR-281-AC-6 | The AC-1 item with its engine's state budget set to 1 settles `incomplete` with the state-budget cause. | Test (TC-763) |
| FR-281-AC-7 | Over a finite model with one reachable state at which the invariant `100 / x > 0` divides by zero, an EN-1 item claiming that invariant settles `refuted`, category violation, with cause `UndefinedEvaluation` whose `where` names that state and whose `cause` is `division-by-zero`; replay reproduces the undefined value at that state, the record carries no `undefined` label, and FR-285 maps the outcome to exit 10. | Test (TC-763) |
| FR-281-AC-8 | With `engines` holding test engines `qsl.test-a` and `qsl.test-b`, both advertising (`value-validity`, `bounded`): an item settled `Supported(qsl.test-b)` runs on `qsl.test-b` alone and its record names it; the same item settled `InvalidRequest` with `invalid_capability`/`ambiguous-backend` naming `qsl.test-a` and `qsl.test-b`, as `negotiate_*` settles it with both candidates and no named backend (QSpec FR-290, FR-331-AC-4), returns a record settled by that disposition with that diagnostic and runs neither engine; settled `Unsupported` with `unsupported_projection`/`unsupported-requested-capability`, a record settled by that disposition and no run; settled `RequiresBound`, a record settled by that disposition with no diagnostic and no run; and settled `Supported(qsl.test-c)`, which `engines` does not hold, `invalid-request` with `invalid_capability`/`unknown-backend` naming `qsl.test-c` and no run. | Test (TC-762) |
| FR-281-AC-9 | AC-3's request returns its three records in `request_index` order 0, 1, 2. The same request whose binding's `universes` name a population the package does not declare returns `StageFailure::Refused(AnalyzeRefusal::Binding)` holding FR-120's `invalid_runtime_input`/`wrong-role-mapping`, no record, and no engine runs. | Test (TC-762) |

## Dependencies

- ADR-029 OP-2, CB-3: the operation and layer A.
- ADR-013 O-16, O-24: categories and terminal records.
- ADR-011 FB-07: replay through S6a.
- ADR-018, ADR-024, ADR-026, ADR-028 (drafts): the engines and their
  certificates.
- [FR-282](FR-282-check-analyze-certificates-in-the-qualified-core.md): the certificate checkers.
- [FR-098](FR-098-execute-a-replay-request.md): replay.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md): the subject binding.
- QSpec FR-300: the `analyze` operation; QSpec FR-331: the result vocabulary.

## References

- QSL-393 (V1-A06): typed library APIs.
- QSL-390 (ARCH-50): rulings RU-2, RU-3 and RU-6 (an engine with no core
  checker settles `proved`, labelled `uncertified`), recorded on the ticket.
- QSL-366: an undefined claim evaluation settles `refuted` with cause
  `UndefinedEvaluation{where, cause}`, the cross-cutting ruling recorded on
  the ticket.
- QSpec FR-300 (STD-141): the QSpec half.

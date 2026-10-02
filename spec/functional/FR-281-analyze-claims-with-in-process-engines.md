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

A `proved` result is `certificate-checked` once the engine's certificate
checker in the qualified core accepts its certificate (FR-282). A proof from
a native engine with no core checker is `proved`, labelled `uncertified` (FR-290). A counterexample counts only
after S6a replay reproduces it.

An `analyze` item is a proof item. An evaluation of the claim that is
undefined at some state or step, in any engine, settles the item `refuted`
with cause `UndefinedEvaluation{where, cause}`: the claim did not hold there.
The item's category is violation, it exits 10 (FR-285), and it never carries
the `undefined` label (FR-286).

## Inputs

- `&CheckedPackage`.
- The claim items, each named by its occurrence key.
- The subject binding: the `ModelSystem` and the initial population
  (FR-120).
- Each engine's request settings and budgets, for example ADR-018's
  model-check limits.
- The accounting limits.
- `&Cancel`.

## Outputs

`Staged<AnalyzeOutcome>`: exactly one terminal record per requested item,
with the FR-331 result vocabulary, its O-16 category, its typed cause, its
basis, its counterexample or certificate where it has one, and its
accounting. An undefined claim evaluation gives a `refuted` record, category
violation, with cause `UndefinedEvaluation{where, cause}` and the point where
the evaluation was undefined as its counterexample.

## Behavior

- The `analyze` operation shall return exactly one terminal record for each
  requested item.
- If no layer-A engine accepts an item's claim kind, then `analyze` shall
  settle that item `unsupported` with a typed cause that names the claim
  kind.
- When an engine reports a proof with a certificate, `analyze` shall run the
  engine's certificate checker (FR-282) and settle the item `proved` only if
  the checker accepts the certificate.
- If the certificate checker rejects the certificate, then `analyze` shall
  settle the item `inconclusive` with cause `CertificateRejected` and the
  checker's typed rejection.
- If an engine reports a proof and the qualified core holds no certificate
  checker for that engine, then `analyze` shall settle the item `proved`,
  category success, with the trust basis `uncertified` naming the engine.
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
| FR-281-AC-3 | A request of three items, one whose claim kind no layer-A engine advertises, one an engine proves and one an engine refutes, returns exactly three records, the first `unsupported` with a cause naming the claim kind. | Test (TC-762) |
| FR-281-AC-4 | With the test engine's certificate altered in one member before checking, the item settles `inconclusive` with cause `CertificateRejected`; with a test engine that reports a proof and has no certificate checker, the item settles `proved`, category success, with the basis `uncertified` naming that engine. | Test (TC-763) |
| FR-281-AC-5 | With a test engine that reports a counterexample replay does not reproduce, the item settles `inconclusive` with cause `replay_parity`; with one whose counterexample names a parameter the function lacks, `inconclusive` with cause `replay_refused`. | Test (TC-763) |
| FR-281-AC-6 | The AC-1 item with its engine's state budget set to 1 settles `incomplete` with the state-budget cause. | Test (TC-763) |
| FR-281-AC-7 | Over a finite model with one reachable state at which the invariant `100 / x > 0` divides by zero, an EN-1 item claiming that invariant settles `refuted`, category violation, with cause `UndefinedEvaluation` whose `where` names that state and whose `cause` is `division-by-zero`; replay reproduces the undefined value at that state, the record carries no `undefined` label, and FR-285 maps the outcome to exit 10. | Test (TC-763) |

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

---
id: SR-679
title: "QSL-271 PR 469 failure-domain review of FR-100's refusal-record mapping"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@ceb5d905; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; evidence quire-exact/src/outcome.rs, qsl-foundation/src/diagnostic.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/causes.rs, qsl-eval/src/value/expression/mod.rs, qsl-semantics/src/value/model_query.rs, qsl-semantics/src/model/population.rs, qsl-semantics/src/model/refusal.rs, spec/functional/FR-109-run-a-state-clause-through-the-spine.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---

## Summary

Ticket: QSL-271 (PR agent-ix/quire-spec-language#469). This review looks for
information the new mapping loses, outcomes it merges, and exit statuses it
misclassifies.

The bare `{"kind": "refused"}` for a record-less refusal is consistent with
FR-096. FR-096 only rules that no `RefusalRecord` is built ("An empty map is
never a stand-in"). It prescribes no rendering, so the bare form is FR-100's
own choice. For kernel causes, the bare form is a defensible interim until
STD-110: the kernel variant has no catalog code, and FR-100 rightly adds no
spelling of its own. For family causes it is not, because O-17's
`catalog_code()` is total and available (FND-001).

The `CheckedInvariant` conversion in `spine::run` is consistent with FR-096
(a consumer at S6a SHALL turn it into an `InternalFault`, never a record)
and with FR-096's Status (not built in the evaluator). It is also consistent
with T-4's `InternalFault` (internal-failure category, code
`runtime_invariant`/`established-invariant-broken`).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A record-less family refusal loses its catalog code and gets exit 20 whatever the code. `CatalogCoded::catalog_code()` is total for every family cause (ADR-013 O-17, C-15), even when `catalog_fields()` is `None`. Before this PR, FR-100 rendered it with the code's `Code::exit_code`. S6a model queries raise these record-less causes: `TypeMismatch` (`ill_typed`), `AboveMaximum` (`cardinality_out_of_bound`) and `AncestorSteps` (`resource_exhausted`, which `Code::exit_code` maps to 22, incomplete). See `qsl-semantics/src/model/population.rs:1635-1690, 1805-1826` and `refusal.rs:1373-1391`. Failure scenario: through FR-109 (which reuses this mapping and supplies a snapshot's `ObjectEnvironment`), an `allInstances` whose conformance walk exceeds `ancestor_steps` prints `{"kind": "refused"}` and exits 20. It should report `resource_exhausted` and exit 22. The same code, `cardinality_out_of_bound`, renders fully from the kernel but bare from `AboveMaximum`. Fix: for a family cause with no key-table row, render `code` and `cause` from `catalog_code()` with no `fields` member (not an empty map, so FR-096's rule holds), with exit `Code::exit_code` of that code. Keep the bare form for kernel causes with no catalog code. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:102-106, 121; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md:72-73 |
| FND-002 | medium | The record's locus is dropped without being stated. A `RefusalRecord` carries `Locus`, which `Evaluation::refusal_record` resolves from `Evaluation.location`. That locus is FR-096's purpose. FR-100 renders only "code, cause and fields" and has no position member. For a record-less refusal, `Evaluation.location` is still available and is dropped too. Failure scenario: a bare `{"kind": "refused"}` from `f(x: Int[0,9]): Int[0,9] { x + 5 }` at `x = 7` (`IntegerOutOfDomain`) gives the author no cause and no position. Fix: render the resolved locus when present (for example a `locus` member with FR-095's region form), or say the omission is deliberate. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:102-106, 120; qsl-foundation/src/diagnostic.rs:969-1010 |
| FND-003 | low | The bare form merges eleven kernel variants into one indistinguishable output. The kernel's own payloads (`DivisionPairOutOfDomain{quotient_admitted, remainder_admitted}`, `IeeeNotExact{would_be}`) and `Refusal::code()`'s kernel spellings for `IeeeNanPayloadNotRepresentable`, `IeeeRationalOutOfDomain` and `ForeignReference` are discarded. This is acceptable as an interim given STD-110. The PR also drops the old Status plan to make `Refusal::code()` total, which is right, since the catalog is to be the single source. Failure scenario: until STD-110 lands, the CLI cannot tell a division-domain refusal from a text-length one. Fix: none required. Record the loss in Status so it is visibly temporary. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:133-134, 281-288; quire-exact/src/outcome.rs:109-205 |
| FND-004 | low | The `InternalFault` that `spine::run` builds is under-specified. T-4 says an `InternalFault` names its stage and violated invariant by stable identifiers, but FR-100 names neither for the `CheckedInvariant` conversion, and its `details` is `null`. S6a's own `Err(InternalFault)` (`CallFailure::Fault`, `qsl-eval/src/value/expression/mod.rs:297-303`) has no row in FR-100 at all, although it is the same category and exit. Failure scenario: two implementations give different `invariant` ids, and a `CallFailure::Fault` exits by some unstated path. Fix: fix the stage (`S6a`) and invariant id, carry them in `details`, and give `CallFailure::Fault` the same row. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:168, 236-238 |

## Verdict

Changes requested: FND-001 misclassifies a reachable exit status and drops a
catalog code that is available.

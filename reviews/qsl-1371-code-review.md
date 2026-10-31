---
id: SR-1954001
title: code-review review of QSL-1371 admitted checked-term view
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md;
  QSL-1371
review_set: subset
---

## Summary

Ticket: QSL-1371; PR 1954. Reviewed only the FR-390 amendment at the frozen candidate supplied by the lead.

Document-only lane: the diff changes only FR-390 (+102/-2). No new implementation, tests, copied vocabulary, lowered gate, or hidden execution path is introduced. Existing source confirms that reader exposure and machine adaptation are future work; neither source nor test completion is claimed.

## Verdict

**PASS** — No defects found within this method and amendment scope; this is not implementation or AC-11 acceptance.

## Review Policy

No applicable AssuranceProfile was found: a search of spec/ for type: AssuranceProfile and review_policy found none. spec/assurance contains AD-001 (ArchitectureDescription, proposed) and MP-001 through MP-006 (MeasurementPlan), not a required review policy. Direct subset review is authorized by repository instructions and the assigned brief; no enforced ix-flow acceptance gate applies.

## Method Selection

Ran code-review (documentation lane), base, integrity, dependency, failure-domain, scope-boundary and ears-conformance. Skipped rust-review and review-react because no source/manifests changed; gap-analysis and matrix because no production code changed and the brief excludes aggregate/matrix work; object/app/security/architecture reviews because no such domain objects or architecture decision is changed; evidence and risk-complexity because no verification-method allocation or new risk analysis was requested. Criterion-strength requires Jev and was not selected. No new upstream API work or implementation was performed.

## Examined Scope

```yaml
scope:
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: "  - target: ix://agent-ix/quire-spec-language/FR-400\n    type: depends_on\n\
    \  - target: ix://agent-ix/quire-spec-language/ADR-016\n    type: depends_on\n\
    \  - target: ix://agent-ix/quire-spec-language/FR-049\n    type: references"
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: "- One admitted evaluation view of either an in-process `CheckedPackage`\
    \ or\n  the complete package admitted by the contract reader from the v2 protocol\n\
    \  nodes FR-383 emits (QSpec FR-464), as the bridge below defines. An\n  `ImportView`,\
    \ a free-standing term and a decoded wire graph are not inputs."
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'QSL SHALL define one sealed `qsl_eval::value::CheckedTermView` contract
    in

    layer 5. Its concrete layer-4 input carrier SHALL be

    `qsl_package::evaluation_view::AdmittedEvaluationView<''a>`, with private variants

    that borrow either `&''a CheckedPackage` or the successful I2 read''s retained'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'admitted package, source map, effective limits and admitted dependency
    closure.

    The in-process entry SHALL be `CheckedPackage::evaluation_view(&self) ->

    AdmittedEvaluationView<''_>`; the successful I2 read SHALL expose

    `evaluation_view(&self) -> AdmittedEvaluationView<''_>` over its retained'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '`Arc<quire_contract_model::CheckedPackageV2>`. The reader-result entry
    and its

    IR variant SHALL exist only with `std`. The view contract and in-process

    variant SHALL introduce no IR or `std` dependency into the no_std path.

    '
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'The reader''s retained admitted package is the witness: a caller SHALL
    NOT

    construct the reader variant from `CheckedSemanticGraphV2`, `CheckedTerm`,

    `ImportView`, bytes, or a deserialized value. A refused, limited, cancelled or

    faulted I2 read SHALL yield no view. The view SHALL retain the reader''s exact'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'identity, source-map and dependency associations for its entire lifetime.

    Exposing the successful reader result does not change I2 admission or its

    existing staged refusal vocabulary.

    '
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'Under `std`, `qsl-package` SHALL expose its existing

    `read_checked_package_v2(bytes: &[u8], identity: LibraryName,

    limits: V2ReadLimits, evidence: &CheckedPackageEvidence) -> V2ReadOutcome`

    and the successful `V2Read` carrier, with its fields private. `V2ReadOutcome`'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'SHALL remain `Result<Staged<V2Read>, StageFailure<V2ReadRefusal>>`.

    `V2Read::evaluation_view(&self)` SHALL be the reader-result entry above.

    This exposes the existing admission result; it does not add a second reader.

    '
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'The layer-5 `CheckedTermView` SHALL select a slot with

    `select_protocol_slot(&self, slot: &ProtocolSlot) ->

    Result<SelectedProtocolTerm<''_>, CallFailure>`. `SelectedProtocolTerm` SHALL

    have private fields and read accessors for its expression handle and ordered'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'source descriptors. Each descriptor SHALL expose the admitted binder''s

    `NodeKey`, level, `ValueType` and read location. The expression handle

    `TermHandle<''a>` SHALL be opaque, tied to the exact borrowed view and owning

    body, and unavailable from an integer, a node key, a free-standing term or'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'another view. Operand handles SHALL retain that same owner; dependency
    calls

    SHALL resolve within the witness''s exact admitted closure. Slot selection and

    source ordering SHALL obey the selection and supply admission rules below.

    `ProtocolSlot`, `SelectedProtocolTerm`, source descriptors and `CallFailure`'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'SHALL remain layer-5 inputs and results; layer 4 SHALL import none of
    them.


    The view SHALL expose each selected term''s operation, ordered operands,

    literal value, checked type and location through the one shared checked-term'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'read vocabulary governed by [FR-400](FR-400-separate-checked-terms-from-the-checker.md).

    This is a read projection of admitted data; it SHALL neither build a

    `CheckedBody`, `Node` or another checked proof nor call `construct`. The

    in-process and reader projections SHALL be consumed by the existing S6a'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '`Machine`, using the same operation execution, function and dispatch lookup,

    model reads and family-result mapping. The bridge SHALL add no interpreter,

    family-dispatch table, checking pass, source recompilation or serialized-term

    execution path. The sealed `CheckedPackageEvaluation` seam SHALL evaluate'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '`AdmittedEvaluationView` as well as `CheckedPackage`; the latter SHALL

    delegate slot evaluation through its view to the same `ProtocolClause` hook.


    View creation, term projection and slot/supply admission SHALL charge no'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'evaluation counter. Once evaluation starts, the same caller meter SHALL
    reach

    the existing machine unchanged. Admission failures SHALL retain their code,

    cause and node/member/binder locus with no charge; an impossible foreign

    handle or broken admitted projection SHALL be an `InternalFault`, never a'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'successful value or a new admission authority.


    The bridge SHALL preserve ADR-011 §6.1''s layer edges and ADR-016''s execution

    arrows: byte admission and typed projection live in layer 4, execution lives'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'in layer 5, and the shared checked-term vocabulary stays below both. It
    SHALL

    NOT enable `quire-checked/construct` outside FR-400''s existing allowlist or

    place IR types in `quire-checked`, `quire-eval` or the no_std evaluation closure.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'The shared read vocabulary above is an implementation dependency, not
    an

    assertion that the current reader already supplies it. At the repository''s

    locked IR revision `8320ebe`, `CheckedPackageV2::term` returns

    `Option<&CheckedTerm>` and `graph()` exposes node identities, dependencies and'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '`semantic_type`; however `CheckedTerm::Literal.value` is `serde_json::Value`,

    `CheckedBindingName::kind` is private, and the public admitted operation views

    cover specific member shapes rather than the complete expression/type/binder

    view the machine needs. The locked `quire-checked` revision `78e1b2e` also

    precedes FR-400''s expression-term separation.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'Implementation of the reader-backed branch depends on the owning IR/shared

    checked-model lane specifying and delivering typed read access to those

    admitted literal values, operation members and binder/type facts. This

    requirement does not define upstream signatures or authorize new upstream'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'constructors. Until that dependency is delivered, AC-11''s reader equivalence

    remains unverified; a source-backed call or inspection of JSON SHALL NOT stand

    in for it. Projection MAY traverse already admitted typed data, but SHALL NOT

    repeat admission or give a free-standing decoded term checked authority.'
- id: FR-390-AC-14
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: A consumer can obtain a view from an in-process package and from a successful
    I2 reader result. Construction of a reader view or expression handle from a decoded
    graph, free-standing term, `ImportView`, raw bytes, integer or node key fails
    to compile. A refused, limited, cancelled or faulted read exposes no evaluation
    view.
- id: FR-390-AC-15
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'For every TC-1070 slot, AC-11 compares the in-process view with a view
    obtained by emitting and actually admitting that package''s bytes: the complete
    result (including value, unavailable binder, catalog code/cause, location and
    losses) and every ordered charge-log entry are equal. Repeat for unavailable and
    short-circuited supplies, each AC-9 counter boundary, and AC-10''s shared-meter
    sequence; each selection/supply refusal has the same locus and leaves its meter
    unchanged. No fixture recompiles source to produce the reader-side evaluation.'
- id: FR-390-AC-16
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: Inspection finds one slot-selection view contract and one existing machine/ProtocolClause
    execution path for both input variants, no checked-term reconstruction, no new
    `construct` caller and no second family dispatch. With `std` off the in-process
    view compiles without IR; the reader-result entry is absent, and the existing
    no_std closure acquires no dependency on IR or `std`.
- id: FR-390-AC-9
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: context_only
  excerpt: With usage `U` from a completed call, a meter with every charged counter
    at exactly its measured usage completes; with each charged counter in turn one
    below its usage and the others at theirs, the call returns `Incomplete` naming
    that counter with no value; a zero `work_units` returns `Incomplete`; a fresh
    meter completes the same call with the same value.
- id: FR-390-AC-10
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: context_only
  excerpt: One meter passed to a compensation's registration capture, activation capture,
    trigger guard, retry relation and recovery predicate in turn accumulates exactly
    the sum of the calls' charges; a `work_units` budget of that sum completes all
    of them and one below it returns `Incomplete` `work_units` at the last call; the
    calls in another order give the same total.
- id: FR-390-AC-11
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: context_only
  excerpt: Running one call twice gives equal results and charge logs; a call over
    the package the contract reader returns for FR-383's emitted nodes gives the same
    result and charge log as over the checked package that was emitted; a refused
    call leaves the meter as it was.
- id: FR-400
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: context_only
  excerpt: 'The checked terms SHALL live in the crate `quire-checked`, in the repository

    `agent-ix/quire-eval`, at [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)

    §6.1 layer C: a shared leaf above SV, K and W and below QSL layer 3, RT and

    CG. The checker (`qsl-semantics` `check`, with the definition admission for the
    integer division law)

    SHALL be the only code that constructs a checked term. The evaluator SHALL read
    checked terms from

    `quire-checked` and SHALL NOT reach the checker to evaluate one. Everything

    the separation moves keeps its meaning, its identities and it'
- id: ADR-011
  path: spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md
  role: context_only
  excerpt: '| 4 | `package` | S4, I2 byte reader (verification calls `library`) |
    3, F, C, SV, K; `quire-contract-model` for the v2 wire contract: the typed structs
    the emitter builds and the IR reader the I2 reader delegates to (X-7) |

    | 5 | `value::expression` core (S6a) < family evaluators under `value::expression`,
    including the state and temporal evaluators < `simulation` (amended by ADR-018
    LA-2: with FR-120''s `ModelSystem`, the model subject and the property-automaton
    translation) | S6a | 4, 3, F, C, E, SV, K |

    | R | `route` (#185 registry and router) | candidate sets over S4 before E7; a'
- id: ADR-016
  path: spec/decisions/ADR-016-state-model-finite-execution-mapping.md
  role: context_only
  excerpt: '| FP-3 | `StateModel` implements `FamilyContract` and not `ReferenceEvaluation`,
    and no `StateModel` variant is added to `S6aFamilyKind`. No S6a input is a `StateModel`
    item: a model form is always nested in a `Value` function body or a `ProtocolClause`
    body, and it evaluates inside that family''s `evaluate` hook through `value::model_query`,
    raising `StateModel`-owned causes (ADR-013 O-16). This applies ADR-012 §2''s #214
    rule (a contract part nothing can construct is left out). ADR-012 §2, §3, §5.1
    S1 and §13.5 are amended to say so. Reopen condition: an S6a entry whose selected
    item'
- id: AD-001
  path: spec/assurance/AD-001-shared-evaluation-core-interface.md
  role: context_only
  excerpt: 'QSL''s current std evaluator remains the reference implementation behind
    this

    interface during the migration. IR-583 delivers the shared interface and std

    reference adapter. It does not claim that the evaluator itself is no_std. The

    no_std + alloc implementation is IR-590, after S1-S5. During the bounded

    dual-implementation period, differential tests compare both implementations on
    the same checked inputs. IR-593 ends that

    period by making the no_std implementation the only evaluator and deleting the

    std-only path (FR-405).


    ## Views


    The system is viewed through the current source bo'
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Validation Evidence

Quoin 0.29.1 supplied the installed SpecReview skeleton/schema; quire 0.37.1 (engine 0.50.3) validates explicit repository scope and paths. The changed FR is grammar-clean; full spec validation emits existing advisory grammar and installed-module duplicate-definition warnings. No Rust build/test suite was necessary for this document-only amendment.

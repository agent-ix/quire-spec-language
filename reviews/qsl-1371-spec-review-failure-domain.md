---
id: SR-1954005
title: spec-review/failure-domain review of QSL-1371 admitted checked-term view
type: SpecReview
analysis: failure-domain
scope: agent-ix/quire-spec-language; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; spec/functional/FR-400-separate-checked-terms-from-the-checker.md;
  QSL-1371
review_set: subset
---

## Summary

Ticket: QSL-1371; PR 1954. Reviewed only the FR-390 amendment at the frozen candidate supplied by the lead.

Reader refusal, limit, cancellation and fault expose no view; owner-qualified handles and exact dependency closure prevent foreign input authority. Broken admitted projections produce InternalFault. Evaluation remains read-only, with existing heap-walk/meter rules and ordered result/charge equivalence retained. No new unspecified failure policy was found in this amendment.

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

## Disposition Pass 1

**Current verdict: PASS.** All original real findings in this artifact are fixed; no new findings. The original review and finding text above are retained as historical evidence.

The current FR-390 and FR-400 fixes were read in full against the prior candidate and existing NodeKind, RecordSlot, CaseArm, CheckedLiteral, kernel ValueType, semantic Location, I2 read result and resource ceilings. FR-400 owns one generic existing operation enum and exact NodeRead fields; every nested child position is mechanically substituted. FR-390 names read_node, root/sources accessors and the descriptor types, retains sealed witness authority, and resolves typed projections before exposing a view. Value/ValueType are kernel types and Location is the existing SV expression location; no upward QSL/IR dependency is introduced. Non-child evidence is retained/cloned, never fabricated. Missing charge semantics, including Attribute.derefed, prevents view creation and cannot pass AC-11 by returning Unsupported. No new defect was found in this fix round.

The contract is mergeable on this review. Implementation readiness is false until the explicitly stated IR-1193 typed reads, IR-1194 exact identity/type joins, IR-969/FR-400 shared placement and STD-611 lossless charge-semantics dependencies are delivered. Their absence does not leave a placeholder in the specified NodeRead contract and does not justify claiming AC-11 or parent QSL-852 complete.

Policy remains direct subset review: no applicable AssuranceProfile was found under spec/. AD-001 is ArchitectureDescription and MP-001 through MP-006 are MeasurementPlan; no enforced human gate applies. The same seven methods were reconsidered for the fix. No production files changed; Rust/source tests, gap-analysis and matrix remain outside this document-only scope. No implementation was performed.

## Examined Scope (disposition pass 1)

```yaml
scope:
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '### One admitted checked-term view'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'QSL SHALL define one sealed `qsl_eval::value::CheckedTermView` contract
    in

    layer 5. Its concrete layer-4 input carrier SHALL be

    `qsl_package::evaluation_view::AdmittedEvaluationView<''a>`, with private variants

    that borrow either `&''a CheckedPackage` or the successful I2 read''s retained

    admitted package, source map, effective limits and admitted dependency closure.

    The reader variant MAY retain plain kernel values/types and read descriptors

    projected from that witness; it SHALL retain no newly constructed checked term.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '

    The in-process entry SHALL be `CheckedPackage::evaluation_view(&self) ->

    AdmittedEvaluationView<''_>`; the successful I2 read SHALL expose

    `evaluation_view(&self) -> Result<AdmittedEvaluationView<''_>, ViewFailure>` over
    its retained

    `Arc<quire_contract_model::CheckedPackageV2>`. The reader-result entry and its

    IR variant SHALL exist only with `std`. The view contract and in-process

    variant SHALL introduce no IR or `std` dependency into the no_std path.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'The reader''s retained admitted package is the witness: a caller SHALL
    NOT

    construct the reader variant from `CheckedSemanticGraphV2`, `CheckedTerm`,

    `ImportView`, bytes, or a deserialized value. A refused, limited, cancelled or

    faulted I2 read SHALL yield no view. The view SHALL retain the reader''s exact

    identity, source-map and dependency associations for its entire lifetime.

    Exposing the successful reader result does not change I2 admission or its

    existing staged refusal vocabulary.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'Under `std`, `qsl-package` SHALL expose its existing

    `read_checked_package_v2(bytes: &[u8], identity: LibraryName,

    limits: V2ReadLimits, evidence: &CheckedPackageEvidence) -> V2ReadOutcome`

    and the successful `V2Read` carrier, with its fields private. `V2ReadOutcome`

    SHALL remain `Result<Staged<V2Read>, StageFailure<V2ReadRefusal>>`.

    `V2Read::evaluation_view(&self)` SHALL be the reader-result entry above.

    This exposes the existing admission result; it does not add a second reader.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'The layer-5 `CheckedTermView` SHALL select a slot with

    `select_protocol_slot(&self, slot: &ProtocolSlot) ->

    Result<SelectedProtocolTerm<''_>, CallFailure>`. `SelectedProtocolTerm` SHALL

    have private fields and read accessors for its expression handle and ordered

    source descriptors. Each descriptor SHALL expose the admitted binder''s

    `NodeKey`, level, `ValueType` and read location. The expression handle

    `TermHandle<''a>` SHALL be a layer-4 opaque value, tied to the exact borrowed
    view and owning

    body, and unavailable from an integer, a node key, a free-standing term or'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '

    another view. Operand handles SHALL retain that same owner; dependency calls

    SHALL resolve within the witness''s exact admitted closure. Slot selection and

    source ordering SHALL obey the selection and supply admission rules below.

    `ProtocolSlot`, `SelectedProtocolTerm`, source descriptors and `CallFailure`

    SHALL remain layer-5 inputs and results; layer 4 SHALL import none of them.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'The layer-4 view SHALL expose

    `read_node<''v>(&''v self, handle: &TermHandle<''v>) ->

    Result<quire_checked::NodeRead<''v, TermHandle<''v>>, InternalFault>`.

    The canonical result and its operation vocabulary SHALL be exactly

    [FR-400](FR-400-separate-checked-terms-from-the-checker.md)''s `NodeRead` and

    generic `NodeKind`, including their borrowed literal, type and location.

    Every child position of the returned `NodeKind` SHALL contain a handle of

    that same view and owning expression body; call metadata SHALL retain the'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '

    exact admitted callee identity and closure association. `SelectedProtocolTerm`

    SHALL expose `root(&self) -> &TermHandle<''_>` and

    `sources(&self) -> &[ProtocolSource]`; `ProtocolSource` SHALL expose

    `binder(&self) -> &NodeKey`, `level(&self) -> usize`,

    `value_type(&self) -> &ValueType` and `location(&self) -> &Location`.

    An unavailable literal or type projection SHALL prevent creation of the view,

    never become a late partial `NodeRead` or a successful default value.

    This is a read projection of admitted data; it SHALL neither build a'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '

    `CheckedBody`, `Node` or another checked proof nor call `construct`. The

    in-process and reader projections SHALL be consumed by the existing S6a

    `Machine`, using the same operation execution, function and dispatch lookup,

    model reads and family-result mapping. The bridge SHALL add no interpreter,

    family-dispatch table, checking pass, source recompilation or serialized-term

    execution path. The sealed `CheckedPackageEvaluation` seam SHALL evaluate

    `AdmittedEvaluationView` as well as `CheckedPackage`; the latter SHALL

    delegate slot evaluation through its view to the same `ProtocolClause` hook.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'View creation, term projection and slot/supply admission SHALL charge
    no

    evaluation counter. Once evaluation starts, the same caller meter SHALL reach

    the existing machine unchanged. Admission failures SHALL retain their code,

    cause and node/member/binder locus with no charge; an impossible foreign

    handle or broken admitted projection SHALL be an `InternalFault`, never a

    successful value or a new admission authority.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: "The bridge SHALL preserve ADR-011 \xA76.1's layer edges and ADR-016's\
    \ execution\narrows: byte admission and typed projection live in layer 4, execution\
    \ lives\nin layer 5, and the shared checked-term vocabulary stays below both.\
    \ It SHALL\nNOT enable `quire-checked/construct` outside FR-400's existing allowlist\
    \ or\nplace IR types in `quire-checked`, `quire-eval` or the no_std evaluation\
    \ closure."
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '### Typed projection admission and upstream dependencies'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'Reader view creation SHALL resolve all expression, callable and binder
    types,

    literal payloads, operation members and location associations in the admitted

    closure before yielding a view. Resolution SHALL be fallible: IR''s general

    value-type vocabulary SHALL NOT be presumed to represent every kernel

    `ValueType`. The view SHALL use the existing kernel `ValueType` as its only

    evaluation type vocabulary, with no QSL-owned type mirror, guessed identity

    or fallback to an unchecked or serialized interpretation.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '`ViewFailure` SHALL be the layer-4 closed enum

    `Unsupported(UnsupportedView)`, `Limit(LimitExceeded)` or `Fault(InternalFault)`.

    `UnsupportedView` SHALL carry the catalog pair

    `unknown_required_feature`/`unsupported-feature`, the owning package identity,

    the affected node''s `NodeKey`, its `Location`, and one typed reason:

    `MissingTypeRepresentation`, `MissingLiteralProjection`,

    `MissingOperationProjection`, `MissingBindingProjection` or

    `MissingChargeSemantics`.

    If more than one projection is unsupported, view creation SHALL report the'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '

    first by ascending package identity, node key and the reason order above.

    This is evaluation-input admission before S6a, category unsupported, exit 21;

    it SHALL yield no partial view and SHALL leave the successful I2 read intact.

    View creation SHALL accept no `Meter`; it SHALL neither create, replace nor

    charge the caller''s evaluation meter. Once a view exists, `read_node` SHALL

    return a complete canonical read or an invariant `InternalFault`, never a

    late unsupported projection. Projection storage and traversal SHALL remain'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '

    within the retained I2 resource ceilings; exhaustion SHALL yield no view and

    return `ViewFailure::Limit` with the existing I2 limit vocabulary, never

    `Unsupported`. A failed I2 read itself retains its existing staged result.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'The upstream dependencies are tracked as IR-1193 (public typed literal,

    binding-name and operation/member reads), IR-969 (checked-term placement),

    IR-1194 (semantic-type and exact identity joins), and STD-611 (lossless

    bare/explicit-dereference semantics). FR-400 owns the canonical checked-term

    read vocabulary. The projections require protocol slot and binder

    identities/levels/types, exact source occurrences and callable/dispatch closure

    associations. IR''s `expression::ValueType` is separate from the canonical'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '

    evaluation target `quire_exact::ValueType`; neither a type-name match nor a

    structural `CheckedNodeId` is an admitted identity conversion. IR-1194 must

    supply exact `NodeKey`/`EffectiveId` associations for Composite and Reference

    and unit identities for Quantity. General coverage of String, Uuid, Timestamp,

    Dimension, Alias and Union requires explicit shared-type extensions wherever

    the canonical target cannot represent the admitted type; nested Option and

    Collection types require the same joins recursively. These tracker references'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '

    record dependencies, not new spec-artifact edges or upstream API signatures.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'The reader projection SHALL preserve every evaluation- and charge-relevant

    operation flag, including `NodeKind::Attribute.derefed`. A bare object-field

    read and an explicit `deref` of the same reference SHALL retain FR-107''s

    distinct charge logs through emission, admission and `NodeRead`, even when

    their completed values are equal. If the admitted representation has erased

    that distinction, then view creation SHALL return `MissingChargeSemantics`

    with no partial view; the required AC-11 comparison remains unmet. The bridge'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '

    SHALL NOT infer explicitness from source spans, source text, a guessed flag,

    recompilation or an otherwise identical semantic graph. STD-611 and the owning

    IR contract determine the lossless encoding; this requirement prescribes no

    new wire field, operation or mode.'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: 'For AC-11 and TC-1073, the upstream extensions SHALL at least cover Boolean,

    bounded Integer, the Receipt/Attempt/Flag/Tick record types with exact member

    and declaration identities, Node''s object type, and Reference<Node> with its

    exact model/object identity, as TC-1070 fixture E defines. A type that QSL emits
    and evaluates

    remains subject to complete AC-11 parity: an unsupported projection of such a

    type SHALL count as an unmet dependency, not a passing parity case or a scope

    reduction. Unrelated IR types that QSL cannot evaluate MAY return the typed'
- id: FR-390
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: '

    unsupported admission above. A source-backed call or JSON inspection SHALL

    NOT stand in for reader-backed parity. Projection MAY traverse already

    admitted typed data, but SHALL NOT repeat admission or give a free-standing

    decoded term checked authority.'
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
    execution path for both input variants, using FR-400's single generic `NodeKind`
    read vocabulary, no checked-term reconstruction, no new `construct` caller and
    no second family dispatch. With `std` off the in-process view compiles without
    IR; the reader-result entry is absent, and the existing no_std closure acquires
    no dependency on IR or `std`.
- id: FR-390-AC-17
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: An admitted IR type without a kernel representation yields `ViewFailure::Unsupported`
    before any view exists, with `unknown_required_feature`/`unsupported-feature`,
    the affected package/node/location and `MissingTypeRepresentation`; no meter is
    accepted or charged. A missing literal, operation or binding projection yields
    its corresponding typed reason. A projection-resource limit yields `Limit`, no
    partial view. A handle from another owner returns an invariant fault rather than
    a node.
- id: FR-390-AC-18
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: Reader views resolve Boolean, bounded Integer, record/composite and Reference<Node>
    identities of TC-1070 fixture E to the exact kernel values/types of the in-process
    view, including recursively nested Option/Collection values used by a slot. AC-11/15
    fail if a required QSL-emitted type is reported unsupported; unsupported cases
    cannot replace any required result/log comparison.
- id: FR-390-AC-19
  path: spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md
  role: examined
  excerpt: A bare `view.total` read and an explicit `deref(view).total` read over
    the same observation retain their respective in-process charge logs through emitted-package
    admission and `NodeRead`, including FR-107's explicit-dereference charge distinction.
    If the representation erases that distinction, view creation returns `MissingChargeSemantics`
    before a view or meter charge exists, and AC-11 remains unmet; equal completed
    values or an unsupported result cannot pass the required parity comparison.
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
  role: examined
  excerpt: '### Canonical read projections'
- id: FR-400
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: examined
  excerpt: '`quire-checked` SHALL own one operation vocabulary for stored expression

    nodes and borrowed reads: the existing `NodeKind` SHALL become

    `NodeKind<Child = NodeId, Literal = CheckedLiteral>`. Every `NodeId` operand

    position SHALL use `Child`, including positions nested in vectors, options,

    `RecordSlot<Child = NodeId>` and `CaseArm<Child = NodeId>`; the `Literal`

    variant SHALL carry the `Literal` parameter. All other variants, metadata,

    admitted evidence and operand ordering SHALL keep their existing meaning.

    This parameterization changes representation, not the set of operations.'
- id: FR-400
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: examined
  excerpt: '

    There SHALL be no parallel `TermOperation` enum or operation catalog.'
- id: FR-400
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: examined
  excerpt: 'The shared read result SHALL be the plain descriptor

    `NodeRead<''a, Child>` with these exact public fields:'
- id: FR-400
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: examined
  excerpt: "```rust\npub struct NodeRead<'a, Child> {\n    pub operation: NodeKind<Child,\
    \ &'a quire_exact::Value>,\n    pub value_type: &'a quire_exact::ValueType,\n\
    \    pub location: &'a quire_semantic_value::location::Location,\n}\n```"
- id: FR-400
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: examined
  excerpt: 'A read SHALL substitute owner-qualified handles for every child position

    and borrow the admitted literal''s `Value`, checked `ValueType` and `Location`.

    Non-child metadata and admitted evidence MAY be cloned from the witnessed

    input into the descriptor; this SHALL NOT mint evidence or change identity.

    An in-process read SHALL project its stored `NodeKind` into this same generic

    enum, and a reader-backed read SHALL project retained typed admitted data

    into it. The evaluator SHALL match this single enum for both reads, with

    unchanged operation execution and charge points.'
- id: FR-400
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: examined
  excerpt: '

    Every cost-relevant metadata field SHALL survive this projection unchanged,

    including `NodeKind::Attribute.derefed`: a borrowed operation SHALL distinguish

    an explicit dereference from a bare field read with the same result. If the

    admitted representation lacks that fact, then it cannot supply a lossless

    `NodeRead`; FR-390''s view admission SHALL report the missing charge semantics

    before yielding a view. No projection SHALL invent the fact from erased

    source information or treat the missing fact as equivalent execution.'
- id: FR-400
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: examined
  excerpt: '`NodeRead` and the parameterized operation descriptors are plain read
    data,

    not `Node`, `CheckedBody`, `CheckedGraph` or an admission witness. Constructing

    a descriptor SHALL confer no authority to evaluate it. Only the sealed,

    witness-bound input contract of

    [FR-390](FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md)

    SHALL supply these reads to S6a. No read projection SHALL construct a checked

    term, call `construct`, validate an unchecked term or duplicate checking.

    The descriptor types SHALL require only `core`, `alloc` and the existing'
- id: FR-400
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: examined
  excerpt: '

    layer-C dependencies, and SHALL name no QSL or IR type.'
- id: FR-400-AC-11
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: examined
  excerpt: 'A stored `NodeKind` and its `NodeRead` projection match the same exhaustive
    generic enum: every variant, nested operand position, literal value, metadata/evidence
    identity, checked type and location are preserved. A probe variant makes both
    evaluator paths non-exhaustive. No parallel operation enum exists; the layer-C
    read descriptors compile without `std`, QSL or IR dependencies.'
- id: FR-400-AC-12
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: examined
  excerpt: 'Constructing a `NodeRead` or a parameterized operation descriptor cannot
    create a checked term or an admitted FR-390 view: passing a descriptor to the
    public evaluator as a checked input fails to compile. Existing checked-term constructor
    and mutation rejection checks remain failing, with their positive pairs compiling.'
- id: FR-400-AC-1
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: context_only
  excerpt: '`quire-checked`''s manifest names only `quire-semantic-value`, `quire-exact`,
    `quire-walk`, `quire-canonical`, `serde` and `thiserror` as normal dependencies,
    each with default features off, and names no QSL crate in any table. In QSL, TC-390''s
    table check asserts that `qsl-semantics` is the only crate that enables `quire-checked`''s
    `construct` feature; a fixture manifest in which `qsl-eval` enables it fails the
    check, naming the crate and the table.'
- id: FR-400-AC-2
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: context_only
  excerpt: The import-direction check reports, in the shipped (non-`#[cfg(test)]`)
    modules of `qsl-eval`, no `qsl_forms` item and no `qsl_semantics::check` item
    other than `ValueFunctionFamily` and `ProtocolClauseFamily`; `qsl-eval` names
    `WrongSnapshotCause` at `quire_semantic_value::call`. A fixture shipped `qsl-eval`
    module that imports `qsl_semantics::check::PackageDeclarations` is reported with
    its file and line.
- id: FR-400-AC-3
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: context_only
  excerpt: Run on the QSL tree, rule T12-G reports `passing`. A fixture call to a
    `quire_checked::construct` function from `qsl-eval`, `qsl-package` or `qsl-semantics`'
    `model` reports `failing` with the call site's file, line and module, and so does
    an `AdmittedIntegerDivision` constructor call from `model`. FR-059's check reports
    an RT graph whose `quire-checked` dependency enables `construct` as one FB-05
    violation, and the same graph without the feature as clean.
- id: FR-400-AC-4
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: context_only
  excerpt: In QSL, outside `quire-checked`, a struct literal of `CheckedGraph`, `CheckedBody`,
    `Node` or `OperationSelection` and a write to a `Node` field each fail to compile,
    and each paired `no_run` block naming the same type compiles. In `agent-ix/quire-eval`'s
    own gate, a doctest of `quire-checked` built with default features, where `construct`
    is off, shows that a call to a `construct` function fails to compile; QSL cannot
    run that clause, because feature unification turns `construct` on for its workspace.
- id: FR-400-AC-5
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: context_only
  excerpt: 'No `qsl_semantics::` path to a moved item resolves: `qsl_semantics::check::CheckedBody`,
    `qsl_semantics::check::CheckedGraph`, and `qsl_semantics::model::key::DeclarationKey`
    each fail to compile from another crate, and no F or layer-2 path names a value
    that moved to SV.'
- id: FR-400-AC-9
  path: spec/functional/FR-400-separate-checked-terms-from-the-checker.md
  role: context_only
  excerpt: '`quire-checked` builds for `thumbv7em-none-eabi` with default features
    off, and its manifest forbids unsafe code; no normal or build dependency in QSL
    enables its `test-support` feature.'
```

## Validation (disposition pass 1)

Quoin 0.29.1 supplied the SpecReview contract. Quire 0.37.1 (engine 0.50.3) validates the two changed requirement paths; its only EARS warnings in those paths are unchanged FR-400 line 35. All latest scratch SRs are validated at their explicit paths. No matrix or aggregate implementation gate was run.

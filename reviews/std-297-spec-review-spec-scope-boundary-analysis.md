---
id: SR-15524
title: "scope-boundary review of STD-297 checked-term boundary (QSL PR #1225)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@09f27280c4441e702eadfdae0d08691602dc974b; spec/functional/FR-400-separate-checked-terms-from-the-checker.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; source checked: qsl-semantics, qsl-eval, qsl-replay, qsl-package at origin/main 85f6f62a0"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-15524: scope-boundary review of STD-297 checked-term boundary (QSL PR #1225)

## Summary

Ticket: STD-297. This review checks FR-400's move and stay lists against the code on origin/main 85f6f62a0.

Every type name in the lists exists at the module the spec gives. Two names have duplicates: `check::ir::Visit` and
`check::hyper_clause::Visit`, and `check::ir::DispatchTable` and `model::dispatch::DispatchTable`. The `model`
admission that the spec keeps in layer 3 matches the code: shipped `qsl-eval` calls `model::normalize`,
`model::population::admit_*`, `model::object_environment` and `value::model_query` directly, never through
`check`. `check::footprint` reads only `CheckedGraph`, `CheckedStateClause`, `model::state` and
`model::operation`, so it can move to `model`.

Four parts of the split do not match what the evaluator and the simulation read, or what the moved types carry. As
written they cannot be implemented.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: the evaluation-visible `Scope` view is too small. FR-400 says qsl-eval reads only the type environment, the enum member index and the operation table through `scope()`. Shipped qsl-eval also reads: <br>- `scope.dispatch_operations()` (evaluate.rs:2214-2218). `NodeKind::Dispatch.operation` indexes it, yet the spec keeps `DispatchOperation` with the checker. <br>- `scope().imported_graph(package)` (evaluate.rs:1912-1930). This returns an imported library's `CheckedGraph` from `AdmittedImport.graph`, keyed by `ImportView::package()` (`library::PackageId`), yet the spec keeps `AdmittedImport` with the checker. <br>- `scope.ieee_profile()` (evaluate.rs:1662, 2033; `value::definition::AdmittedIeeeProfile`) and `ieee_profile_selection`. <br>C cannot name these layer-3 types. Name them in the view: move `DispatchOperation`, carry the imported graphs as C graphs under a package id that moves to C or SV, and carry the admitted IEEE and division selections as plain values. Or state how they reach the evaluator. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:92-96, 144-159 |
| FND-002 | high | major: the moved values cannot close within C, SV, K and W as stated. <br>- Moved clause records carry F spans: `CheckedTemporalClause.span` and `protocol_fairness` spans, `FormulaNode::Holds { span }`, `TemporalClaim.punctual`, `CheckedStateGraphClaim.span` and `CheckedHyperClause.span`. <br>- They carry forms values: `CheckedStateClause.spans: DeclarationSpans`, `CheckedStateClause.kind: StateClauseKind`, and `CheckedHyperClause.match_expression: Option<qsl_forms::Expression>`, a parsed AST. <br>- They carry layer-3 values: `CheckedStateClause.observations`, which holds `model::clause_observation::Observation`; `TemporalClaim.kind: check::Capability`; `SemanticNode`'s `NominalNode`, which holds `value::enumeration` and `value::unit` preimages; `ModelTimeBinding`; and F's `ProfileSelection`, `DefinitionRef`, `TemporalInterval` and `TimedInterval`. <br>The known set lists none of these. The fallback ("a field whose type cannot move stays in the checker-side record") covers only `CheckedGraph`'s own fields. Moving `Span` or `DeclarationSpans` to SV contradicts lead default 5, and a forms `Expression` cannot move at all. Moving `check::observation` to layer-3 `model` while `CheckedStateClause` (C) carries `Observations` would make C depend on layer 3. Fix: state a per-record rule that span and form fields of a moved record go to a checker-side table keyed by the record's identity. Put `Observations` and `Observation` in C. Name each layer-3 value that moves. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:101-112, 133-142, 161-165 |
| FND-003 | high | major: the `CheckedGraph` split leaves two shipped reads unplaced. <br>- `qsl_eval::value::expression::Evaluation::refusal_record(&CheckedGraph)` resolves its locus with `graph.region(location)` (evaluate.rs:99-105). This is the FR-096 path, and FR-400 keeps region resolution in the checker-side record in `check`, which AC-2 forbids qsl-eval to name. <br>- `simulation::model` reads `graph.model_selections()` (model.rs:441, 461; `model::domain_package::DomainPackageRef`). That field is in neither list. <br>Say which crate `refusal_record` moves to (for example the replay or package layer, which may read the checker-side record), and whether `model_selections` moves (then `DomainPackageRef` moves to C). | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:119-131, 195-204 |
| FND-004 | high | major: AC-2 cannot pass within S2. Shipped `qsl-eval/src/value/expression/causes.rs:17` imports `qsl_semantics::check::WrongSnapshotCause`, and FR-400 says that type "does not move in this slice; the S1 slice places them". Either exempt it in AC-2 and FB-14 until S1 lands, or make S2 depend on S1 (IR-585). In the second case AD-001's "S2 is blocked only by ... FR-400" is wrong (see SR-15525 FND-001). | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:155-159, 188-193, 243 |
| FND-005 | medium | minor: some types that the moved types carry, or that guard them, are not named. <br>- `state_clause::PopulationDomain` and `ClauseOperation` (carried by `CheckedProtocol` and `CheckedOperationFrame`). <br>- `OperationSelection`, which only `resolve_operation` builds; it has its own `compile_fail` at state_clause.rs:79 and mod.rs:3967. <br>- The rest of `node_key`'s terms that `BodyTerm` carries (`LeafTerm`, `AggregateTerm`, `FrameTerm`, `AbstractionTerm`, `MemberTerm`, `NodeRef`, `Operation`). <br>- `CheckedProtocol` member types that the simulation does not name (`CheckedChannel`, `CheckedObligations`). <br>Also, `CheckedProtocol`'s fields are all `pub` and qsl-eval reads them directly, so rule 1 (private fields) needs an accessor surface. And `DeclarationKey::validate_derivation` returns `model::refusal::ModelRefusalCause`, so it must also become a `model` function. Rule 5 should cover every `compile_fail` that guards a moved type, not two. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:85-117, 166-171, 228-231 |
| FND-006 | low | minor: some names are ambiguous. `Visit` exists as both `check::ir::Visit` and `check::hyper_clause::Visit`; write `check::ir::Visit`, as the spec already writes `check::ir::DispatchTable`. `OperationKey` is defined in `check::abstraction` (abstraction.rs:110), the module whose declaration forms stay. Say that `OperationKey` moves out of it. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:85-90, 113-117 |

## Verdict

Four majors (FND-001 to FND-004). Each was traced in the origin/main source with file and line. FR-400 is not
mergeable until the `Scope` view, the per-record closure rule, `refusal_record` and `model_selections`, and the
`WrongSnapshotCause` exemption or S1 dependency are stated. The type names, the `model` admission stay-list, the
`footprint` move, the `DeclarationKey::identity` → `model` move (T12-C) and the `OperationTable` move are sound.
Mergeable after the fixes.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | minor: shipped qsl-package mints `PackageId` itself: the emitter at `checked.rs:653` and the I2 reader at `checked_v2.rs:678`, both through `PackageId::of_preimage`. FR-400 makes `PackageId` a checked term that only `construct` builds, with `library` as "its one minter". AC-3 reports a `construct` call from `qsl-package` as `failing`. Say that qsl-package mints through `library`'s minting function, which is the one caller of `construct` for `PackageId`, so that T12-G passes on the QSL tree. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:282-286, 308 |
| FND-008 | low | minor: the closure lists do not name every type the rules reach. <br>- The `node_key` terms carry layer-3 `value::member::Member` (in `Operation.member`) and `value::definition::DefinitionReference` (in `OperationLaw.definition`). Rule 3 says layer-3 types move "as the lists above name", and neither is named. The Scope-view sentence also keeps the IEEE profile's "definition reference" with the checker, which reads as the type staying behind. <br>- `CheckedOperator` and `CheckedProtocolFairness` carry the forms values `TemporalOperator`, `TemporalNodeId`, `FairnessKind` and `FairnessGranularity`, which rule 2's known set omits. <br>Name these types. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:98-106, 127-139, 170-184 |
| FND-009 | low | minor: rule 1 keys a checker-side span table by "its node key, or its declaration index". Nested protocol members that carry a span have neither: `CheckedChannel.span`, `CheckedRelationship.span` and `CheckedRequirement.span`. Key them by the owning record's identity plus their position. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:160-169 |

## Dispositions

Round 1. Reviewed QSL #1225 at dc6e2b208 against origin/main a89136bfa, which includes #1224 and is an ancestor of the head. `make check-spec-validation` exits 0 at the head.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |

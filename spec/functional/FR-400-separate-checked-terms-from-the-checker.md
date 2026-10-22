---
id: FR-400
title: "Separate the checked terms the evaluator reads from the checker"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
---
# FR-400: Separate the checked terms the evaluator reads from the checker

## Description

The S6a evaluator reads checked terms: checked expression bodies, the checked
graph's functions, dispatch tables, clause records and the identities and
locations they carry. Those types are defined in `qsl-semantics`' `check`
module, beside the checker that builds them, so anything that evaluates a
checked term has to depend on the checker. [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md)
requires that separation to come before the no_std evaluator and to keep
checking authority, source locations, declaration identities, model
correspondence and proof closure intact. This requirement states the
boundary.

The checked terms SHALL live in the crate `quire-checked`, in the repository
`agent-ix/quire-eval`, at [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
§6.1 layer C: a shared leaf above SV, K and W and below QSL layer 3, RT and
CG. The checker (`qsl-semantics` `check`, with the library binding for a
package identity and the definition admission for the integer division law)
SHALL be the only code that constructs a checked term. The evaluator SHALL read checked terms from
`quire-checked` and SHALL NOT reach the checker to evaluate one. Everything
the separation moves keeps its meaning, its identities and its locations: a
checked package, an evaluation and a refusal record are the same before and
after the move.

## Inputs

- The checked-term types and accessors that `qsl-semantics` defines in
  `check`, `library`, `model` and `value` and that the evaluator and the
  simulation read, listed under Behavior.
- The dependency tables and resolved graphs of the QSL workspace and the
  backend repositories, and the QSL crates' `use` trees, read by the
  existing layering checks (TC-390's table check, `xtask`'s import graph,
  `tools/arch-lint`'s API-surface rules under FR-060, and FR-059's direction
  check).

## Outputs

- The crate `quire-checked` holding the moved types, with a `construct`
  module behind its `construct` feature.
- `qsl-semantics`, `qsl-package`, `qsl-eval`, `qsl-route`, `qsl-replay` and
  every other caller repointed at `quire_checked::` paths in the same change
  that moves an item, with no re-export at the old path.
- One added FR-060 API-surface rule, T12-G.

## Behavior

### Crate and dependencies

1. `quire-checked` SHALL be a crate of the repository `agent-ix/quire-eval`,
   which depends only on repositories below it. Its `[dependencies]` SHALL
   name only `quire-semantic-value` (SV), `quire-exact` (K), `quire-walk`
   (W), `quire-canonical`, `serde` and `thiserror`, each without `std`. It
   SHALL NOT depend on any crate of the QSL repository, in any dependency
   table, so QSL's dependency on it closes no repository cycle (FB-11).
2. `quire-checked` SHALL be `#![no_std]` with `alloc`, with a `std` feature
   that is on by default, and SHALL forbid unsafe code.
3. Its test-only fixtures (`CheckedBody::fixture`, `CheckedBody::from_nodes`,
   `DeclarationKey::fixture` and any other moved fixture) SHALL sit behind
   its own `test-support` feature, which only a `[dev-dependencies]` entry
   enables.
4. QSL SHALL take it as a git dependency at `branch = "main"`, as it takes
   `quire-walk`.

### What moves into `quire-checked`

A type moves when the evaluator or the simulation reads it from a checked
package. The types below move from the `qsl-semantics` module named with
each.

- **Expression terms** (`check::ir`): `Node`, `NodeId`, `Slot`, `NodeKind`,
  `CheckedBody`, `CheckedNode`, `CheckedLiteral`, `CaseArm`, `RecordSlot`,
  `RecordField`, `check::ir::Visit`, `Arithmetic`, `Connective`,
  `IntegerDivisionOperator`, `OrderedKind`, `CollectionLoss`,
  `CollectionProperty`, `DispatchCandidate` and `check::ir::DispatchTable`,
  with the types a `NodeKind` names: `check::navigation::EndSide`,
  `check::lowering::AuthoredBound`, `library::PackageNodeKey` with
  `library::PackageId`, and `value::definition::AdmittedIntegerDivision`.
- **Roots**: `CheckedExpression`, `CheckedGraph` (its evaluation-visible
  content, below), `FunctionState`, `CallableFunction`, the
  evaluation-visible part of `Signatures`, and a scope view.
- **Scope view**: what `qsl-eval` reads through `scope()`. That is the SV
  type environment, the enum member index, the operation table, the
  dispatch operations (`check::DispatchOperation`, which
  `NodeKind::Dispatch` indexes), each imported library's checked graph keyed
  by its `PackageId` (what `imported_graph` returns), and whether the package
  admitted an IEEE profile. The admitted IEEE profile itself
  (`AdmittedIeeeProfile`, which wraps a `value::definition::DefinitionReference`,
  and the declared-float
  selection) and `AdmittedImport` with its `ImportView` stay in the
  checker's `Scope`, which `qsl-package`'s emitter reads.
- **Admitted operations**: `model::operation`'s `OperationTable` and
  `OperationDeclaration` and `model::domain_package`'s `OperationEffect`,
  plain data over `DeclarationKey` and K types that the scope view
  carries.
- **Clause records the evaluator and simulation read**:
  `CheckedTypeInvariant`, `TypeSlot`, `TemporalClauseNode`,
  `CheckedTemporalClause`, `TemporalClaim`, `CheckedFormula`, `FormulaNode`,
  `FormulaNodeId`, `CheckedInterval`, `Activation`, `UnaryTemporal`,
  `BinaryTemporal`, `TemporalProfile`, `ModelTimeBinding`,
  `CheckedStateClause` with `check::observation::Observations` and
  `model::clause_observation::Observation`, `CheckedOperationFrame`,
  `ClauseOperation`, `OperationSelection`, `PopulationDomain`, `CheckedRow`,
  `CheckedRefinement`, `CheckedTerminal`, `TerminalDeclaration`,
  `CheckedTerminalPredicate`, `CheckedStateGraphClaim`, `StatePredicate`,
  `CheckedPopulationBound`, `CheckedProtocol` with its member types
  (`ProtocolNodeId`, `ControlStep`, `CheckedScheduling`, `AccessClass`,
  `CheckedChannel`, `CheckedObligations` and every other type a
  `CheckedProtocol` field holds), `CheckedHyperClause` with
  `check::hyper_clause::Visit`, `HyperForm`, `CopyGenerator` and `CopyGroup`,
  `CheckedProbabilisticModel`, `Reward` and `Workload`.
- **Identities**: the `check::node_key` terms (`BodyTerm`,
  `ApplicationTerm`, `LeafTerm`, `AggregateTerm`, `FrameTerm`,
  `AbstractionTerm`, `MemberTerm`, `NodeRef`, `check::node_key::Operation`
  and `check::node_key::Operator`), a semantic-graph projection,
  `identity::ModelCorrespondence` and `CheckedTypeNode`, the occurrence map
  (`check::family::OccurrenceMap`), `OperationKey` (moved out of
  `check::abstraction`, whose declaration forms stay), `check::Capability`,
  `model::domain_package::DomainPackageRef` and `model::key::DeclarationKey`
  as a value. The semantic-graph projection is what the evaluator reads of
  `lowering::SemanticGraph`: resolving a `WireNodeId` to its `NodeKey`, and
  a `NodeKey`'s `BodyTerm`. `SemanticGraph` and `SemanticNode`, whose nominal
  nodes hold `value::enumeration` and `value::unit` preimages, stay with the
  checker for emission.

`CheckedGraph` splits. Its evaluation-visible content moves: functions,
dispatch tables, the scope view, the occurrence map, parameter occurrences,
model correspondence, type nodes, the semantic-graph projection, effective
limits, model selections (`DomainPackageRef`), every clause record above,
operation frames, model objects, object types and nodes, population domains,
keys and bounds, declaration keys and the identity limits. The rest stays in
`qsl-semantics` in a checker-side record that holds the moved `CheckedGraph`:
the source reference, the source-region tables, the `abstraction`
declaration forms and model names it checks, the full `SemanticGraph`, the
requirement records `qsl-route` reads (`RequirementRecord`), and any other
field that is read only by checking, routing, emission or rendering. A field
read by both the checker and the evaluator moves.

### Closure of the moved values

The values a moved type carries SHALL close within `quire-checked`, SV, K
and W. Each moved record, not only `CheckedGraph`, SHALL follow three rules,
applied recursively to the types its fields hold:

1. **Source positions and forms stay with the checker.** A field holding an
   F `Span`, a `qsl-forms` `DeclarationSpans` or `ExpressionSpans`, or a
   `qsl-forms` syntax tree is not part of the moved record. It goes to a
   checker-side table in `qsl-semantics`, keyed by the record's identity:
   its node key, or its declaration index where it has no node key. A
   nested member with no node key of its own (a protocol's
   `CheckedChannel`, relationship or requirement, a fairness premise) is
   keyed by its enclosing record's key and its declaration-order index within
   that record, so every span of `CheckedChannel`, `CheckedRelationship` and
   `CheckedRequirement` has a key. This
   covers `CheckedTemporalClause`'s and its fairness premises' spans,
   `FormulaNode::Holds`'s span, `TemporalClaim`'s punctual span,
   `CheckedStateGraphClaim`'s and `CheckedHyperClause`'s spans,
   `CheckedStateClause`'s `DeclarationSpans` and `CheckedHyperClause`'s
   `match_expression`. The moved record keeps its SV `Location`s.
2. **Plain values from F or layer 2 move to SV.** A plain value type that a
   moved record carries and that `qsl-foundation` or `qsl-forms` defines
   moves down into SV in the same change and leaves no copy or re-export
   behind. The known set is
   `AbsenceMode`, `ByteDigest`, `WireNodeId`, `OccurrenceKey`,
   `IdentityLimits`, `DomainKey`, `DomainKind`, `MemoryModelName`,
   `TemporalInterval`, `TimedInterval`, `ProfileSelection` and
   `DefinitionRef` from F, and `StateClauseKind` and the protocol premises
   `AccessOrdering`, `ChannelCapacity`, `ChannelDelivery`, `ChannelOverflow`,
   `LossSelector`, `ProtocolNodeKind`, `TemporalOperator`, `TemporalNodeId`,
   `FairnessKind` and `FairnessGranularity` from layer 2.
   `qsl-foundation` keeps using the values it gives up, so it gains a normal
   dependency on SV (ADR-011 F row, X-2); SV depends on no QSL crate, so the
   edge closes no cycle. `ByteDigest`'s SHA-256 constructor (`ByteDigest::of`
   before the move) stays in `qsl-foundation` as a free function returning
   SV's `ByteDigest`, which SV builds only from 32 given bytes, so SV takes
   no hashing dependency.
3. **Values from layer 3 move to C.** A type that a moved record carries
   and that `qsl-semantics` defines moves into `quire-checked`, as the lists
   above name. This includes `value::member::Member` (the
   structured checked member identity the `node_key` terms carry) and
   `value::definition::DefinitionReference` wherever a moved record carries
   it; the lock catalog and admission that produce a `DefinitionReference`
   stay in `value::definition`. A layer-3 type that cannot move this way (one that names a
   checker-only type) is not carried by the moved record; its field stays in
   the checker-side table of rule 1.

### What stays with the checker

These stay in `qsl-semantics` and are not checked terms:
`PackageDeclarations` and its `check`, `check_expression` and
`check_with_cancel` entries; `AdmittedImport`, `ImportView`, `AdmittedModel`,
`AdmittedIeeeProfile`, `CheckContext`, the family checker modules and
`check::refusal`'s causes; `FamilyContract`, `ScopeStack`, `DiagnosticSink`
and the family marker types `ValueFunctionFamily` and
`ProtocolClauseFamily`, which implement `FamilyContract` and therefore stay
in the trait's crate; the name-resolution index of `Scope`; the full
`SemanticGraph`; and source-region resolution, which moves from
`CheckedGraph::region` to the checker-side record. `SCALAR_LIMITS_UNLIMITED`
stays a `qsl-semantics` test fixture. The S6a result vocabulary
(`FamilyOutcome`, `FamilyResult`, `EvalOutcome`) does not move in this
slice; S0 moves it to `quire-eval`
([FR-401](FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md)),
and `FamilyKind` stays in `family`. The
runtime evaluation cause `WrongSnapshotCause` leaves `check` in whichever of
S1 and S2 lands first: when the S1 slice has not moved it, X-13 moves it from
`check::refusal` to SV's `call` module, where the checker's `CheckCause` and
`qsl-eval`'s runtime snapshot cause both name it, and the S1 slice then
moves nothing for it.

Two `check` modules are runtime analyses over checked terms and `model`
items, not checking: the footprint derivation (`check::footprint`, with its
own `check::footprint::Operation`) and the observation walk
(`Observations::of` in `check::observation`). The simulation calls the
footprint derivation at run time. It SHALL move to layer-3 `model`, which
may read `quire-checked`, so the evaluator reaches it without naming
`check`. The `Observations` type moves to C with `CheckedStateClause`, which
carries it; the walk that builds it is a constructor in C's `construct`
module, called from `check`.

A method of a moved type that mints a kernel identity or names a layer-3
type does not move: only `check` calls the `NodeKey` constructor and only
`model` calls the `EffectiveId` and `PopulationId` constructors (ADR-011
§6.1, FR-060 T12-B to T12-D). `DeclarationKey::identity`, which mints an
`EffectiveId`, and `DeclarationKey::validate_derivation`, which returns
`model::refusal::ModelRefusalCause`, become `model` functions over the moved
`DeclarationKey`.

### Runtime model-admission dependencies

The evaluator's model admission stays in layer-3 `model` and `value`, which
sit below `check` and do not call it: `model::object_environment`
(`ObjectEnvironment`, `ObservedRead`, `ReadLog`), `model::population`
(`PopulationBinding` and the `admit_*` admission), `model::observation` and
`model::clause_observation`'s functions, `model::state`, `model::normalize`,
`model::domain_package`, `model::accounting`, the admission side of
`model::operation`, `model::refusal`, `value::model_query`
(`evaluate_lookup`, `evaluate_all_instances`), `value::supplied` and
`value::{divide, modulo}`. This slice does not move them. A later slice
moves a runtime admission function that needs only the meter and the object
environment to the evaluation core; one that needs a checker type stays in
`qsl-semantics`.

After this separation, the shipped modules of `qsl-eval`'s `value`,
`simulation`, `automaton`, `hyper`, `temporal`, `fairness` and
`model_subject` SHALL name no `qsl_semantics::check` item except the two
family marker types `ValueFunctionFamily` and `ProtocolClauseFamily`, and
SHALL name no `qsl_forms` item. `#[cfg(test)]` code may still build checked terms through
`qsl-semantics`' checker.

### Source locations

A checked term SHALL carry its source position as the SV
`quire_semantic_value::location::Location` it carries before the move;
evaluation, losses, refusals and call-site locations use that value
unchanged. Resolving a `Location` to a `SourceRegion` SHALL stay a
checker-side step over the checker-side record, which keeps the spans and
the `SourceMap` embedding. The method `Evaluation::refusal_record` SHALL
become a free function in `qsl-eval` over the evaluation and a region
resolver, a function from `Location` to an optional `SourceRegion`, and
SHALL return the same FR-096 record; its callers (`qsl-replay`'s spine and
the tests) pass the checker-side record's resolver, so `qsl-eval` names no
checker-side type. For every
`Location`, the resolved `SourceRegion` is the one `CheckedGraph::region`
returned before the move, so FR-096 refusal records and FR-262's location
and loss behaviour are unchanged.

### Checking authority

A checked term is a value whose existence claims that the checker or the
library binding admitted it: `Node`, `CheckedBody`, `CheckedExpression`,
`CheckedGraph`, the scope view, every moved clause record, `Observations`,
`OperationSelection`, the semantic-graph projection, `ModelCorrespondence`,
`CheckedTypeNode`, the occurrence map, `PackageId` and
`AdmittedIntegerDivision`. The moved plain value types (the node ids, the
`node_key` terms, operator and premise enums, both `Visit` types,
`Capability`, `DeclarationKey`, `OperationKey`, `DomainPackageRef`,
`DispatchOperation` and the admitted-operation data) claim nothing and keep
the visibility they have before the move. No code other than the checker
SHALL construct a checked term:

1. Every field of a checked term SHALL be private to `quire-checked`, so a
   struct literal or a field write from another crate fails to compile, and
   no accessor SHALL return a mutable reference into one. A moved checked
   term whose fields are `pub` before the move (such as `CheckedProtocol`
   and its member types, which `qsl-eval` reads field by field) SHALL expose
   read accessors in their place.
2. The only constructors of a checked term SHALL be the functions of
   `quire-checked`'s `construct` module, compiled only under its
   `construct` feature.
3. In QSL, only `qsl-semantics` SHALL enable `construct`. FR-060 rule T12-G
   SHALL allow calls into the `construct` module only from `qsl-semantics`'
   `check` and its descendant modules, and in addition from `library` for
   `PackageId` and from `value::definition` for `AdmittedIntegerDivision`,
   with no debt list. `library` keeps the one public `PackageId` minting
   function (`PackageId::of_preimage` before the move, ADR-013 O-02), which
   calls the `construct` constructor; `qsl-package`'s emitter
   (`checked.rs`) and I2 reader (`checked_v2.rs`) mint a package id through
   that `library` function and never call `construct` themselves.
4. FR-059's direction check SHALL report, as an FB-05 violation, a resolved
   IR, RT or CG dependency on `quire-checked` with its `construct` feature
   enabled. A backend reads checked terms and never builds one.
5. Every `compile_fail` check that guards a moved type SHALL move with it
   and keep failing to compile, each with its `no_run` pair that compiles.
   These are TC-244 row 1's `CheckedGraph` struct literal (`check/mod.rs`)
   and `Node` checks (`check/ir.rs`), the `OperationSelection` checks
   (`check/state_clause.rs` and `check/mod.rs`'s `resolve_operation`), and
   the `PackageId` and `PackageNodeKey` checks in `library`.

No item SHALL be re-exported from its old `qsl_semantics::` path. Each
change that moves an item repoints every caller at once (ADR-011 §7.2). The
move MAY land in steps, in this order: expression terms; clause records;
`CheckedGraph`; each step leaves the workspace green.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-400-AC-1 | `quire-checked`'s manifest names only `quire-semantic-value`, `quire-exact`, `quire-walk`, `quire-canonical`, `serde` and `thiserror` as normal dependencies, each with default features off, and names no QSL crate in any table. In QSL, TC-390's table check asserts that `qsl-semantics` is the only crate that enables `quire-checked`'s `construct` feature; a fixture manifest in which `qsl-eval` enables it fails the check, naming the crate and the table. | Test |
| FR-400-AC-2 | The import-direction check reports, in the shipped (non-`#[cfg(test)]`) modules of `qsl-eval`, no `qsl_forms` item and no `qsl_semantics::check` item other than `ValueFunctionFamily` and `ProtocolClauseFamily`; `qsl-eval` names `WrongSnapshotCause` at `quire_semantic_value::call`. A fixture shipped `qsl-eval` module that imports `qsl_semantics::check::PackageDeclarations` is reported with its file and line. | Test |
| FR-400-AC-3 | Run on the QSL tree, rule T12-G reports `passing`. A fixture call to a `quire_checked::construct` function from `qsl-eval`, `qsl-package` or `qsl-semantics`' `model` reports `failing` with the call site's file, line and module, and so does a `PackageId` constructor call from `model`. FR-059's check reports an RT graph whose `quire-checked` dependency enables `construct` as one FB-05 violation, and the same graph without the feature as clean. | Test |
| FR-400-AC-4 | In QSL, outside `quire-checked`, a struct literal of `CheckedGraph`, `CheckedBody`, `Node` or `OperationSelection` and a write to a `Node` field each fail to compile, and each paired `no_run` block naming the same type compiles. In `agent-ix/quire-eval`'s own gate, a doctest of `quire-checked` built with default features, where `construct` is off, shows that a call to a `construct` function fails to compile; QSL cannot run that clause, because feature unification turns `construct` on for its workspace. | Test |
| FR-400-AC-5 | No `qsl_semantics::` path to a moved item resolves: `qsl_semantics::check::CheckedBody`, `qsl_semantics::check::CheckedGraph`, `qsl_semantics::library::PackageId` and `qsl_semantics::model::key::DeclarationKey` each fail to compile from another crate, and no F or layer-2 path names a value that moved to SV. | Test |
| FR-400-AC-6 | `quire-checked`'s clippy `disallowed-methods` configuration refuses `NodeKey::from_digest`, `EffectiveId::from_digest` and `PopulationId::from_digest`, and `cargo clippy -p quire-checked` in `agent-ix/quire-eval`'s gate passes with it: the crate mints no kernel identity. A fixture call to `EffectiveId::from_digest` in the crate fails that clippy run. | Test |
| FR-400-AC-7 | With no edit to any expected value, `qsl-eval`'s suite, the FR-262 tests and the QSL conformance run over the QSpec checked-package fixtures (`cargo test -p qsl-package --lib checked_v2` with `QSPEC_DIR` set) pass, and every package the emitter writes for those fixtures is byte-identical to its bytes before the move, so its `package_id`, node identities and recorded source locations are unchanged. | Test |
| FR-400-AC-8 | Before the first move step, a golden file recorded from `origin/main` and kept in `qsl-semantics/tests/` holds, for a fixture unit with a function, a dispatch table, a `model` selection, a state clause and a protocol: every node's `Location`, every `NodeKey` and `DeclarationKey`, every model-correspondence entry, the `SourceRegion` each `Location` resolves to, and the code, cause and locus of the FR-096 refusal record of a refusing evaluation of that unit. After each move step, the same test reproduces the golden file byte for byte. | Test |
| FR-400-AC-9 | `quire-checked` builds for `thumbv7em-none-eabi` with default features off, and its manifest forbids unsafe code; no normal or build dependency in QSL enables its `test-support` feature. | Test |

## Dependencies

- [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md) Ownership and
  admission: the separation this requirement states.
- [FR-060](FR-060-check-qsl-api-surface-boundary.md): the API-surface check
  that evaluates T12-G.
- [FR-059](FR-059-check-backend-dependency-direction.md): the backend
  direction check that refuses a backend enabling `construct`.
- [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md):
  the refusal-record locus the checker-side record keeps.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 layer C, FB-05, FB-14 and §7.3 X-13.
- AD-001 slice S2 (IR-586). S0 (IR-583) and S1 (IR-585) move the evaluation
  vocabulary (FR-401),
  and S6 (IR-590) builds the no_std evaluator on this boundary.

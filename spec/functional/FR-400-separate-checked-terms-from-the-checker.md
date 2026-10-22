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
CG. The checker (`qsl-semantics` `check`) SHALL be the only code that
constructs a checked term. The evaluator SHALL read checked terms from
`quire-checked` and SHALL NOT reach the checker to evaluate one. Everything
the separation moves keeps its meaning, its identities and its locations: a
checked package, an evaluation and a refusal record are the same before and
after the move.

## Inputs

- The checked-term types and accessors defined in `qsl-semantics`
  (`check`, `model::key`, `model::operation`) that the evaluator and the
  simulation read, listed under Behavior.
- The dependency tables and resolved graphs of the QSL workspace and the
  backend repositories, and the QSL crates' `use` trees, read by the
  existing layering checks (TC-390's table check, `xtask`'s import graph,
  `tools/arch-lint`'s API-surface rules under FR-060, and FR-059's direction
  check).

## Outputs

- The crate `quire-checked` holding the moved types, with a `construct`
  module behind its `construct` feature.
- `qsl-semantics`, `qsl-package`, `qsl-eval`, `qsl-replay` and every other
  caller repointed at `quire_checked::` paths in the same change that moves
  an item, with no re-export at the old path.
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
package. These types, all defined in `qsl-semantics` today, move:

- **Expression terms** (`check::ir`): `Node`, `NodeId`, `Slot`, `NodeKind`,
  `CheckedBody`, `CheckedNode`, `CheckedLiteral`, `CaseArm`, `RecordSlot`,
  `RecordField`, `Visit`, `Arithmetic`, `Connective`,
  `IntegerDivisionOperator`, `OrderedKind`, `CollectionLoss`,
  `CollectionProperty`, `DispatchCandidate` and `check::ir::DispatchTable`,
  with the two types a `NodeKind` names, `check::navigation::EndSide` and
  `check::lowering::AuthoredBound`.
- **Roots**: `CheckedExpression`, `CheckedGraph` (its evaluation-visible
  content, below), `FunctionState`, `CallableFunction`, the
  evaluation-visible part of `Signatures`, and the evaluation-visible part
  of `Scope`: the SV type environment, the enum member index and the
  operation table, which are what `qsl-eval` reads through `scope()`.
- **Admitted operations**: `model::operation`'s `OperationTable` and
  `OperationDeclaration` and `model::domain_package`'s `OperationEffect`,
  plain data over `DeclarationKey` and K types that the scope view
  carries.
- **Clause records the evaluator and simulation read**:
  `CheckedTypeInvariant`, `TypeSlot`, `TemporalClauseNode`,
  `CheckedTemporalClause`, `CheckedFormula`, `FormulaNode`, `FormulaNodeId`,
  `CheckedInterval`, `Activation`, `UnaryTemporal`, `BinaryTemporal`,
  `TemporalProfile`, `CheckedStateClause`, `CheckedOperationFrame`,
  `CheckedRow`, `CheckedRefinement`, `CheckedTerminal`,
  `TerminalDeclaration`, `CheckedTerminalPredicate`, `CheckedStateGraphClaim`,
  `StatePredicate`, `CheckedPopulationBound`, `CheckedProtocol` with its
  member types (`ProtocolNodeId`, `ControlStep`, `CheckedScheduling`,
  `AccessClass` and the other types `qsl-eval`'s `simulation::protocol`
  names), `CheckedHyperClause` with `HyperForm`, `CopyGenerator` and
  `CopyGroup`, `CheckedProbabilisticModel`, `Reward` and `Workload`.
- **Identities**: `check::node_key`'s `ApplicationTerm`, `BodyTerm` and
  `Operator`, `lowering::SemanticGraph` and `SemanticNode`,
  `identity::ModelCorrespondence` and `CheckedTypeNode`, the occurrence map
  (`check::family::OccurrenceMap`), `OperationKey` and
  `model::key::DeclarationKey` as a value.

`CheckedGraph` splits. Its evaluation-visible content moves: functions,
dispatch tables, the scope view, the occurrence map, parameter occurrences,
model correspondence, type nodes, semantic graph, effective limits, every
clause record above, operation frames, model objects, object types and
nodes, population domains, keys and bounds, declaration keys and the
identity limits. The rest stays in `qsl-semantics` in a checker-side record
that holds the moved `CheckedGraph`: the source reference, the form, type,
site, synthesized, parameter and attempt spans, the `SourceMap` embedding,
the `abstraction` declaration forms and model names it checks, the
requirement records `qsl-route` reads (`RequirementRecord`), and any other
field that names a `qsl-forms` type or is read only by checking, routing,
emission or rendering. A field read by both the checker and the evaluator
moves.

The values a moved type carries SHALL close within `quire-checked`, SV, K
and W. Each value a moved type carries that sits in `qsl-foundation` or
`qsl-forms` today SHALL move down into SV in the same change, unless the S1
slice has already moved it, and SHALL leave no copy or re-export behind. The
known set is `AbsenceMode`, `ByteDigest`, `WireNodeId`, `OccurrenceKey`,
`IdentityLimits`, `DomainKey`, `DomainKind` and `MemoryModelName` from F,
and the protocol premises `AccessOrdering`, `ChannelCapacity`,
`ChannelDelivery`, `ChannelOverflow`, `LossSelector` and `ProtocolNodeKind`
from layer 2. A field whose type cannot move this way stays in the
checker-side record.

### What stays with the checker

These stay in `qsl-semantics` and are not checked terms:
`PackageDeclarations` and its `check`, `check_expression` and
`check_with_cancel` entries; `AdmittedImport`, `DispatchOperation`,
`AdmittedModel`, `CheckContext`, the family checker modules and
`check::refusal`'s causes; `FamilyContract`, `ScopeStack`, `DiagnosticSink`
and the family marker types `ValueFunctionFamily` and
`ProtocolClauseFamily`, which implement `FamilyContract` and therefore stay
in the trait's crate; the name-resolution index of `Scope`; and the
source-region resolution (`CheckedGraph::region` today), which moves to the
checker-side record. `SCALAR_LIMITS_UNLIMITED` stays a `qsl-semantics` test
fixture. The S6a result vocabulary (`FamilyOutcome`, `FamilyResult`,
`EvalOutcome`, `FamilyKind`) and the runtime evaluation cause
`WrongSnapshotCause` do not move in this slice; the S1 slice places them
with the evaluation vocabulary.

Two `check` modules are runtime analyses over checked terms and `model`
items, not checking: the footprint derivation (`check::footprint`) and the
observation walk it uses (`check::observation`). The simulation calls them
at run time. They SHALL move to layer-3 `model`, which may read
`quire-checked`, so the evaluator reaches them without naming `check`.

A method of a moved type that mints a kernel identity does not move: only
`check` calls the `NodeKey` constructor and only `model` calls the
`EffectiveId` and `PopulationId` constructors (ADR-011 §6.1, FR-060 T12-B
to T12-D). `DeclarationKey::identity`, which mints an `EffectiveId`,
becomes a `model` function over the moved `DeclarationKey`.

### Runtime model-admission dependencies

The evaluator's model admission stays in layer-3 `model` and `value`, which
sit below `check` and do not call it: `model::object_environment`
(`ObjectEnvironment`, `ObservedRead`, `ReadLog`), `model::population`
(`PopulationBinding` and the `admit_*` admission), `model::observation` and
`model::clause_observation`, `model::state`, `model::normalize`,
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
family marker types `ValueFunctionFamily` and `ProtocolClauseFamily`, and no
`qsl_forms` item. `#[cfg(test)]` code may still build checked terms through
`qsl-semantics`' checker.

### Source locations

A checked term SHALL carry its source position as the SV
`quire_semantic_value::location::Location` it carries today; evaluation,
losses, refusals and call-site locations use that value unchanged.
Resolving a `Location` to a `SourceRegion` SHALL stay a checker-side step
over the checker-side record, which keeps the spans and the `SourceMap`
embedding. For every `Location`, the resolved `SourceRegion` is the one
`CheckedGraph::region` returns before the move, so FR-096 refusal records
and FR-262's location and loss behaviour are unchanged.

### Checking authority

A checked term is a value whose existence claims that the checker admitted
it: `Node`, `CheckedBody`, `CheckedExpression`, `CheckedGraph`, every moved
clause record, `SemanticGraph`, `ModelCorrespondence`, `CheckedTypeNode` and
the occurrence map. The moved plain value types (the node ids, operator and
premise enums, `Visit`, `DeclarationKey`, `OperationKey` and the
admitted-operation data) claim nothing and keep the visibility they have
today. No code other than the checker SHALL construct a checked term:

1. Every field of a checked term SHALL be private to `quire-checked`, so a
   struct literal or a field write from another crate fails to compile, and
   no accessor SHALL return a mutable reference into one.
2. The only constructors of a checked term SHALL be the functions of
   `quire-checked`'s `construct` module, compiled only under its
   `construct` feature.
3. In QSL, only `qsl-semantics` SHALL enable `construct`. FR-060 rule T12-G
   SHALL allow calls into the `construct` module only from `qsl-semantics`'
   `check` and its descendant modules, with no debt list.
4. FR-059's direction check SHALL report, as an FB-05 violation, a resolved
   IR, RT or CG dependency on `quire-checked` with its `construct` feature
   enabled. A backend reads checked terms and never builds one.
5. The existing TC-244 row 1 `compile_fail` checks (a `CheckedGraph` struct
   literal and a `Node` field write from outside the defining crate) SHALL
   move with their types and keep failing to compile, each with its
   `no_run` pair that compiles.

No item SHALL be re-exported from its old `qsl_semantics::` path. Each
change that moves an item repoints every caller at once (ADR-011 §7.2). The
move MAY land in steps, in this order: expression terms; clause records;
`CheckedGraph`; each step leaves the workspace green.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-400-AC-1 | `quire-checked`'s manifest names only `quire-semantic-value`, `quire-exact`, `quire-walk`, `quire-canonical`, `serde` and `thiserror` as normal dependencies, each with default features off, and names no QSL crate in any table. In QSL, TC-390's table check asserts that `qsl-semantics` is the only crate that enables `quire-checked`'s `construct` feature; a fixture manifest in which `qsl-eval` enables it fails the check, naming the crate and the table. | Test |
| FR-400-AC-2 | The import-direction check reports no `qsl_semantics::check` item other than `ValueFunctionFamily` and `ProtocolClauseFamily` and no `qsl_forms` item in the shipped (non-`#[cfg(test)]`) modules of `qsl-eval`. A fixture shipped `qsl-eval` module that imports `qsl_semantics::check::PackageDeclarations` is reported with its file and line. | Test |
| FR-400-AC-3 | Run on the QSL tree, rule T12-G reports `passing`. A fixture call to a `quire_checked::construct` function from `qsl-eval`, `qsl-package` or `qsl-semantics`' `model` reports `failing` with the call site's file, line and module. FR-059's check reports an RT graph whose `quire-checked` dependency enables `construct` as one FB-05 violation, and the same graph without the feature as clean. | Test |
| FR-400-AC-4 | Outside `quire-checked`, a struct literal of `CheckedGraph`, `CheckedBody` or `Node` and a write to a `Node` field each fail to compile, and so does a call to a `construct` function when the `construct` feature is off; each paired `no_run` block naming the same type compiles. | Test |
| FR-400-AC-5 | No `qsl_semantics::` path to a moved item resolves: `qsl_semantics::check::CheckedBody`, `qsl_semantics::check::CheckedGraph` and `qsl_semantics::model::key::DeclarationKey` each fail to compile from another crate, and no F or layer-2 path names a value that moved to SV. | Test |
| FR-400-AC-6 | T12-B, T12-C and T12-D report `passing`, and `quire-checked`'s own clippy `disallowed-methods` configuration refuses `NodeKey::from_digest`, `EffectiveId::from_digest` and `PopulationId::from_digest`: it mints no kernel identity. | Test |
| FR-400-AC-7 | With no edit to any expected value, `qsl-eval`'s suite, the FR-262 tests, the QSL conformance run over the QSpec checked-package fixtures (`cargo test -p qsl-package --lib checked_v2` with `QSPEC_DIR` set) and QSpec's `checked_package_v2` test pass: every emitted package is byte-identical, so its `package_id`, node identities and recorded source locations are unchanged. | Test |
| FR-400-AC-8 | On a fixture unit with a function, a dispatch table, a `model` selection, a state clause and a protocol, every node's `Location`, every `NodeKey` and `DeclarationKey`, every model-correspondence entry, and the `SourceRegion` each `Location` resolves to equal the values the checker produced before the move; an FR-096 refusal record for a refusing evaluation of that unit has the same code, cause and locus. | Test |
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
- AD-001 slice S2 (IR-586). S1 (IR-585) moves the evaluation vocabulary,
  and S6 (IR-590) builds the no_std evaluator on this boundary.

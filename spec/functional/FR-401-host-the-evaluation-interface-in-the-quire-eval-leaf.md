---
id: FR-401
title: "Host the evaluation interface in the quire-eval leaf and place the vocabulary it names"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
---
# FR-401: Host the evaluation interface in the quire-eval leaf and place the vocabulary it names

## Description

[FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md) states the shared
evaluation interface: register a host body, begin an application, step a
session, resume a body. It places that interface in a QSL-owned leaf crate in
its own repository, with no dependency on QSL's checker, package, model
intake, routing, replay or backend crates. The interface's inputs and results
are QSL's existing evaluation vocabulary: the meter, the cancellation handle,
the evaluation, the call failure and its arm payloads, the input refusal, the
object environment, the callable identity and its admitted signature. The
leaf may not depend on the crates that define several of them: `qsl-eval`
(`Evaluation`, `CallFailure`), `qsl-semantics`' `family` (the S6a result
types) and `value::supplied` (the supplied-value stop records), and
`qsl-foundation` (the catalog vocabulary `CallFailure` and the S6a result
types carry: `InternalFault`, `CatalogCoded`, `UndefinedCoded`,
`Category`).

This requirement names the leaf crate, its repository, its
[ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
§6.1 layer and its dependencies, and places every type the interface names in
a crate the leaf may depend on. It depends on
[FR-400](FR-400-separate-checked-terms-from-the-checker.md), which creates
the repository's checked-term crate `quire-checked` at ADR-011 layer C, the
layer directly below E. It fixes how the leaf reads a checked closure, and
which moves belong to slice S0 (IR-583), slice S1 (IR-585) and slice S2
(IR-586). Every moved type keeps its meaning, so every evaluation, refusal
record and call failure is the same before and after the move.

## Inputs

- The types FR-262's Operational interface names, at their QSL paths before
  the move.
- The dependency tables of the QSL workspace and of the backend repositories,
  read by TC-390's table check and FR-059's direction check.

## Outputs

- The crate `quire-eval`, holding the FR-262 interface and the evaluation
  result vocabulary.
- `quire-semantic-value` (SV) holding the catalog vocabulary, the
  supplied-value stop records and the snapshot cause moved down to it.
- Every QSL caller repointed at the new paths in the change that moves an
  item, with no re-export at the old path (ADR-011 §7.2).

## Behavior

### Crate, repository and layer

1. The shared evaluation-core leaf SHALL be the crate `quire-eval`, in the
   repository `agent-ix/quire-eval`, which also holds FR-400's checked-term
   crate `quire-checked`. Its ADR-011 §6.1 layer is E: a shared leaf above C
   (`quire-checked`), SV, K and W, and below QSL layer 5, RT and CG.
2. The repository SHALL carry the AGPL-3.0-or-later licence, the CLA and the
   CONTENT_RIGHTS controls that `agent-ix/quire-walk` carries. Its licence,
   CLA and content-rights checks SHALL pass before it is made public.
3. `quire-eval` SHALL be `#![no_std]` with `alloc`, with no `std` feature,
   and SHALL forbid unsafe code. Its own gate SHALL build it for
   `thumbv7em-none-eabi`.
4. `quire-eval`'s `[dependencies]` SHALL be drawn only from
   `quire-checked` (C), `quire-semantic-value` (SV), `quire-exact` (K),
   `quire-walk` (W) and `thiserror`, each without `std`, and SHALL declare
   each only when its source names an item of it. It SHALL NOT depend on any
   crate of the QSL repository, in any dependency table.
5. QSL SHALL take `quire-eval` as a git dependency at `branch = "main"`, as
   it takes `quire-walk`. The QSL crates whose shipped code names its items,
   `qsl-eval`, `qsl-replay` and `qsl-bench`, add it to their
   `[dependencies]`; another crate at layer 5 or above adds it when it names
   an item. `qsl-foundation`, `qsl-cst`, `qsl-source`, `qsl-forms`,
   `qsl-semantics`, `qsl-package` and `qsl-route` do not depend on it.
6. `quire-eval` joins ADR-011 FB-05's shared-leaf class. RT and CG MAY take
   a normal edge to it. Because the repository is its own dependency source,
   FR-059's direction check classifies an edge into it as no ecosystem
   repository, with no change to that check.

### Where each interface type lives

Each type the FR-262 interface names SHALL be defined in exactly one crate,
at the path below, and SHALL leave no copy or re-export at a former path.

| Type | Owner and path | Moved by |
| --- | --- | --- |
| `Meter`, `Charge`, `ChargePoint`, `Incomplete`, `ScalarLimits` (the `quire.value.accounting/v1` meter the evaluator charges) | K, `quire_exact` | none. `qsl-semantics`' `model::accounting::Meter` is FR-150's normalization meter, not the evaluation meter, and is not part of the interface |
| `Cancel`, `CancelCause` | K, `quire_exact` | none |
| `Value`, `ValueType`, `Outcome`, `NodeKey` | K, `quire_exact` | none |
| `InputRefusal` | SV, `quire_semantic_value::call` | none |
| `Location`, `ValueLoss`, `LocatedLoss` | SV, `quire_semantic_value::{location, loss}` | none |
| `EnvironmentFailure` (`CallFailure::Storage`'s payload) | SV, `quire_semantic_value::declaration` | none |
| `ObjectClosure` | SV, `quire_semantic_value::object_closure` | none |
| `Category`, `CatalogCode`, `CatalogCoded`, `UndefinedCoded`, `UndefinedRecord`, `UndefinedReason`, `InternalFault` | SV, `quire_semantic_value::catalog` | S0, from `qsl_foundation::diagnostic` |
| The supplied-value stop records: `SuppliedFailure`, `SuppliedStop`, `SuppliedContext`, `SuppliedCause`, `SuppliedPhase`, `SuppliedEvent`, `SuppliedLocus`, `SuppliedLimit`, `SuppliedLimitKind`, `AttemptedCount`, `SuppliedNumericOperand`, `SuppliedStorage`, `SuppliedStorageUnit`, `UnsupportedHelperStop`, and the limit-field name constants `SuppliedLimitKind::field` returns | SV, `quire_semantic_value::supplied_stop` | S0, from `qsl_semantics::value::supplied` |
| `FamilyOutcome`, `FamilyResult`, `EvalOutcome` (the S6a result vocabulary) | E, `quire_eval` | S0, from `qsl_semantics::family` |
| `Evaluation`, `CallFailure` | E, `quire_eval` | S0, from `qsl-eval`'s `value::expression`, public at `qsl_eval::value` |
| The registry, its admission result (`Accepted`, `RegistrySealed`, `CallableNotAdmitted`, `SignatureMismatch`, `DuplicateBinding`), the admitted signature, the session, the step result, the body factory and invocation-state contract, and the child request | E, `quire_eval` | S0, new |
| `WrongSnapshotCause` | SV, `quire_semantic_value::call` | S1, from `qsl_semantics::check::refusal`, unless FR-400's X-13 has moved it |

These stay where they are and are not part of the leaf: `qsl-foundation`'s
`Code`, `RefusalRecord`, `Locus`, `kernel_refusal_record`, `category_of`,
`catalog_category` and the `CatalogCode` constants such as
`ALLOCATION_FAILED`; `qsl-semantics`' `FamilyKind` and `FamilyContract`
(the family contract the checker implements), `SuppliedRun`,
`SuppliedLimits` and the supplied-value admission, which S6 moves to SV
([FR-403](FR-403-implement-the-no-std-evaluator-in-quire-eval.md));
`model::object_environment`
(`ObjectEnvironment`, `ObservedRead`, `ReadLog`) and `PopulationBinding`;
and `qsl-eval`'s `QualifiedName`, `supplied_stop_code` and
`supplied_stop_cause`.

These methods change owner because their types move and the method names a
type, or is called from a crate, that does not:

- `CatalogCoded`'s provided `refusal_record` method builds a
  `RefusalRecord` at a `Locus`, which stay in `qsl-foundation`. It SHALL
  become a `qsl-foundation` function over a `&dyn CatalogCoded` and a
  locus, returning the same record.
- `Evaluation::refusal_record` SHALL become a free function in `qsl-eval`
  over the evaluation and a region resolver, a function from `Location` to
  an optional `SourceRegion`, returning the same FR-096 record, as FR-400's
  Source locations states. Its callers pass the checker-side record's
  resolver, so `qsl-eval` names no checker-side type.
- The private helpers `SuppliedEvent::of_membership`, `SuppliedEvent::of_part`,
  `SuppliedStorage::of_request` and `SuppliedStorage::of_arena`, which the
  staying `SuppliedRun` calls, SHALL become private free functions in
  `qsl-semantics`' `value::supplied` with the same results, and move with
  `SuppliedRun` in S6 (FR-403); they add no public SV API.

`CallFailure::category`, `CallFailure`'s `From<SuppliedFailure>` conversion
and `FamilyOutcome`'s `From<EvalOutcome>` conversion move with their types
unchanged. A moved type keeps its
[FR-063](FR-063-exhaustive-family-extension-seam-probe.md)
`#[cfg(seam_probe)]` probe variant (`FamilyOutcome`'s and
`WrongSnapshotCause`'s), and the receiving crate declares the `seam_probe`
cfg, so FR-063's probe builds, whose `RUSTFLAGS` also reach git
dependencies, still make every QSL match over the type non-exhaustive. `qsl-foundation`
names the moved catalog vocabulary through its dependency on SV; SV depends on
no QSL crate, so that edge closes no cycle.

### The checked closure the leaf reads

`quire-eval` MAY name C's checked terms, since layer E sits above C. It
SHALL NOT name a QSL item: `CheckedPackage`, `ObjectEnvironment`, the
checker-side record that FR-400 keeps in `qsl-semantics`, or any other type
of a QSL crate. It SHALL define the contract an evaluator implementation
supplies for one checked closure, and read the closure only through it:

1. **Callable identity.** A callable is identified by the kernel `NodeKey`
   the checker minted for the function. The leaf compares identities by
   `NodeKey` equality and resolves no name.
2. **Admitted signature.** For a callable identity, the contract SHALL
   return the admitted signature or report the identity absent from the
   closure. An admitted signature is the ordered parameter `ValueType`s and
   the result `ValueType`; two signatures are equal exactly when they have
   the same number of parameters and every position's `ValueType` and the
   result `ValueType` are equal. Parameter names are not part of it.
   Registration decides `CallableNotAdmitted` and `SignatureMismatch` from
   this answer alone.
3. **Checked input and environment.** The checked-expression input and the
   admitted object environment are types the implementation supplies through
   the contract. The leaf moves them into a session and never inspects them.
4. **Interpreted transitions.** The contract SHALL advance an interpreted
   application or expression by one finite transition, returning a
   continuation, a child request or a terminal `Evaluation` /
   `CallFailure`, so the leaf's scheduler drives interpreted and registered
   bodies through one task stack.
5. **One closure per registry.** A registry SHALL be created from exactly one
   checked closure and hold it. Begin takes the registry, not a separate
   closure, so no session pairs a registry with another closure.

The std reference adapter SHALL be in `qsl-eval`. It implements the contract
over `qsl_package::CheckedPackage` and `qsl-semantics`' `ObjectEnvironment`.
Its admitted callables are the functions of the package's checked graph and
of its dependency closure that are callable by name, each identified by its
`CallableFunction` identity, with the parameter and result `ValueType`s that
`CallableFunction` records. An FR-151 synthesized dispatch candidate body or
precondition, which is not callable by name, is not admitted, so
registration for its `NodeKey` returns `CallableNotAdmitted`, as `call`
refuses it by name (TC-196 D07). `CheckedPackageEvaluation`'s `call` and `evaluate` resolve their
`QualifiedName` or checked expression in the adapter and drive one session of
the leaf to terminal, with their existing results and input refusals.

Which of C's checked terms the leaf's contract names, and the
representation the `no_std` evaluator reads, are fixed by S6 (IR-590), not
here; until then the checked input stays a type the implementation
supplies.

### Object environment

Begin's admitted object environment is the implementation's environment type.
For the reference adapter it is `qsl-semantics`' `ObjectEnvironment`, whose
core object closure is SV's `ObjectClosure` (ADR-011 X-11) and whose
populations map holds `model`'s `PopulationBinding`. The environment SHALL
stay in layer-3 `model`: `PopulationBinding` holds a domain package and a
model index, which are not leaf vocabulary. No slice of this requirement moves
`ObjectEnvironment`, `ObservedRead` or `ReadLog`.

### Slice scope and order

- **S0 (IR-583)** SHALL move the catalog vocabulary and the supplied-value
  stop records into SV, then add `quire-eval` with the S6a result vocabulary,
  `Evaluation`, `CallFailure` and the FR-262 interface, and the reference
  adapter in `qsl-eval`. The SV move precedes the QSL change that repoints
  callers at it. S0 carries these moves because the interface's signature
  names them and the leaf can import neither `qsl-foundation` nor a QSL layer.
- **S1 (IR-585)** SHALL move `WrongSnapshotCause` from `qsl-semantics`'
  `check::refusal` to SV's `call` module, so the checker's `CheckCause` and
  `qsl-eval`'s runtime snapshot cause both name it there. That is S1's
  complete item set, and S0 does not move it. When FR-400's X-13 has
  already moved it to the same place, S1 moves nothing.
- **Item sets.** Each item SHALL move once, to the place this requirement or
  FR-400 names. S0's set (the catalog vocabulary, the stop records, the S6a
  result vocabulary, `Evaluation` and `CallFailure`) and S1's set
  (`WrongSnapshotCause`) are disjoint, and neither overlaps FR-400's F values
  and layer-2 premise enums. The first change that moves an F item into SV,
  S0's (ADR-011 X-14) or S2's (X-13), SHALL add `qsl-foundation`'s SV
  dependency; the other reuses it. Neither slice waits for the other.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-401-AC-1 | `quire-eval`'s manifest names as normal dependencies only crates from `quire-checked`, `quire-semantic-value`, `quire-exact`, `quire-walk` and `thiserror`, each with default features off, and only those its source names an item of, and names no QSL crate in any table; its crate root is `#![no_std]`, its manifest forbids unsafe code, and it builds for `thumbv7em-none-eabi`. | Test |
| FR-401-AC-2 | In QSL, TC-390's table check asserts that `qsl-eval` and `qsl-replay` name `quire-eval` in `[dependencies]` and that `qsl-foundation`, `qsl-cst`, `qsl-source`, `qsl-forms`, `qsl-semantics`, `qsl-package` and `qsl-route` name it in no table; a fixture manifest in which `qsl-semantics` depends on `quire-eval` fails the check, naming the crate and the table. | Test |
| FR-401-AC-3 | `quire_eval::{Evaluation, CallFailure, FamilyOutcome, FamilyResult, EvalOutcome}`, `quire_semantic_value::catalog::{Category, CatalogCode, CatalogCoded, UndefinedCoded, UndefinedRecord, UndefinedReason, InternalFault}`, `quire_semantic_value::supplied_stop::{SuppliedFailure, SuppliedStop, UnsupportedHelperStop}` and `quire_semantic_value::call::WrongSnapshotCause` resolve; `qsl_eval::value::Evaluation`, `qsl_eval::value::CallFailure`, `qsl_semantics::family::FamilyOutcome`, `qsl_semantics::family::FamilyResult`, `qsl_semantics::family::EvalOutcome`, `qsl_foundation::diagnostic::InternalFault`, `qsl_foundation::InternalFault`, `qsl_semantics::value::supplied::SuppliedStop` and `qsl_semantics::check::WrongSnapshotCause` each fail to compile from another crate. | Test |
| FR-401-AC-4 | With no edit to any expected value, the `qsl-eval`, `qsl-replay` and `qsl-analyze` suites, `qsl-bench`'s build, the FR-262 tests, the FR-096 refusal-record tests and the QSL conformance run over the QSpec checked-package fixtures pass: every `Evaluation` outcome, location and loss list (including the located `incomplete` result of a charge a cancelled handle denies, which FR-262 keeps as an evaluation rather than `CallFailure::Cancelled`), every `CallFailure` arm and category, and every refusal record built through the `qsl-foundation` and `qsl-eval` functions that replace the two moved methods equal the values before the move. | Test |
| FR-401-AC-5 | Through the reference adapter over a checked package with a function `f(x: Int[0, 9]): Boolean`, an FR-151 dispatched member call whose synthesized dispatch candidate is not callable by name, and a dependency package with a function `g(): Integer`, each registration into a fresh registry returns: for `f`'s `NodeKey` with signature `([Int[0, 9]], Boolean)`, `Accepted`; for `f`'s key with `([Int[0, 10]], Boolean)`, `SignatureMismatch`; for `g`'s key with `([], Integer)`, `Accepted`; for the synthesized candidate's key with its recorded signature, `CallableNotAdmitted`; for a `NodeKey` no function of the closure has, `CallableNotAdmitted`. | Test |
| FR-401-AC-6 | The Begin operation takes the registry and no separate closure or `Cancel` argument, and `quire-eval`'s source names no item of a QSL crate, including `CheckedPackage`, `ObjectEnvironment` and FR-400's checker-side record. | Inspection |
| FR-401-AC-7 | FR-059's direction check reports an RT resolved graph with a normal edge to `quire-eval` sourced from `agent-ix/quire-eval` as clean, and reports no FB-11 cycle through it. | Test |

## Dependencies

- [FR-262](FR-262-evaluate-values-and-calls-at-any-depth.md): the interface
  behaviour this requirement places.
- [FR-400](FR-400-separate-checked-terms-from-the-checker.md): layer C, the
  checked-term crate in the same repository, its F-to-SV moves and the
  region-resolver form of the FR-096 record; FR-400 lands before this
  requirement.
- [FR-059](FR-059-check-backend-dependency-direction.md): the backend
  direction check that classifies the edge.
- [FR-090](FR-090-return-a-family-outcome-or-a-typed-family-refusal.md) and
  [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md):
  the S6a result contract and the refusal records that stay unchanged.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 layer E, FB-05 and §7.3 X-14 and X-15.
- AD-001 slices S0 (IR-583), S1 (IR-585) and S2 (IR-586).

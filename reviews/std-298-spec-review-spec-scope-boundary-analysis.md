---
id: SR-15542
title: "scope-boundary review of STD-298 type placement against QSL source (QSL PR #1227)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@2013cea1d67df19afbc6f1cd905bf5f6d33a0666 against origin/main a89136bf, quire-exact 21349174, quire-semantic-value 02e5cf5c; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-15542: scope-boundary review of STD-298 type placement against QSL source (QSL PR #1227)

## Summary

Ticket: STD-298. This review checks every placement claim in FR-401's table and X-14/X-15 against the `origin/main`
source and against the locked `quire-exact` and `quire-semantic-value` revisions.

Verified:

- K holds `Meter`, `Charge`, `ChargePoint`, `Incomplete` and `ScalarLimits` (`accounting.rs`), `Cancel` and
  `CancelCause` (`cancel.rs`), and `Value`, `ValueType`, `Outcome` and `NodeKey`. `qsl-semantics`
  `model::accounting::Meter` is a separate struct, and `qsl-eval` uses `quire_exact::Meter`.
- SV holds `call::InputRefusal`, `location::Location`, `loss::{ValueLoss, LocatedLoss}`,
  `declaration::EnvironmentFailure` and `object_closure::ObjectClosure`.
- The catalog vocabulary is in `qsl-foundation/src/diagnostic.rs`. Its only foreign dependency is K
  (`CheckedInvariantCause`), plus `BTreeMap`, `Debug`, `Display` and `Hash`, so it moves to `no_std` SV
  unchanged. Every foreign-trait impl on a moved type is in `diagnostic.rs` and moves with it. No crate has an
  `impl From<_> for InternalFault` or `Category`, so the move breaks no orphan rule. No implementor overrides
  `CatalogCoded::refusal_record`. F → SV is legal, because SV depends only on K, `quire-canonical`, `serde` and
  `thiserror`, all without `std`.
- The stop records in `qsl-semantics/src/value/supplied.rs` name only K, SV (`call`, `declaration`,
  `supplied_membership`) and `InternalFault`.
- `Evaluation` (`qsl-eval/src/value/expression/evaluate.rs:76`) and `CallFailure` (`mod.rs:296`) name only
  `FamilyOutcome`, `Location`, `LocatedLoss`, `InputRefusal`, `InternalFault`, `CancelCause`, `EnvironmentFailure`,
  `SuppliedStop`, `UnsupportedHelperStop` and `Category`. All of these sit in K, SV or E after S0.
- `FamilyOutcome`, `FamilyResult` and `EvalOutcome` (`qsl-semantics/src/family/evaluation.rs`) are named in shipped
  code only by `qsl-eval`, `qsl-replay` and `qsl-bench`. Inside `qsl-semantics` they are only defined.
- `WrongSnapshotCause` (`check/refusal.rs:1557`) is self-contained.
- `CallableFunction` carries `identity: NodeKey`, `parameters: &[(String, ValueType)]` and `result: &ValueType`.
- `ObjectEnvironment` holds `ObjectClosure` and a `PopulationBinding` map. `ReadLog` and `ObservedRead` are in
  `model::object_environment`. `FamilyKind` and `FamilyContract` are in `family`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | minor: FR-401 says two methods change owner, but the supplied-stop move affects more. It also moves private inherent helpers that the staying `SuppliedRun` calls: `SuppliedEvent::of_membership` and `of_part` (`supplied.rs:184-204`), and `SuppliedStorage::of_request` and `of_arena` (`supplied.rs:390-410`), 13 references in all. Each must become public SV API or a `qsl-semantics` free function. Name them, or add a general rule. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:125-136 |

## Verdict

Every "already in K/SV" claim holds. The catalog, stop-record, S6a and `Evaluation`/`CallFailure` moves can be
implemented with no illegal edge: F → SV, E → SV/K, and no QSL crate below layer 5 needs E. `qsl-foundation`
depending on SV adds no cycle. Clean apart from FND-001.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-401 now names the four private helpers (`SuppliedEvent::of_membership`/`of_part` and `SuppliedStorage::of_request`/`of_arena`), which become private free functions in `qsl-semantics`' `value::supplied` and add no SV API. They read only public fields and enum variants of the moved types, so this works. |

---
id: SR-955
title: "QSL-358 slice 2 code review (with rust-review lane and test-oracle check) of PR 566, declaration, containment and the enum runtime values moved into quire-semantic-value"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@bcc946cd8154ceefeb1ee0f9cb476d359b726015; diff a7df1ff0...bcc946cd; quire-semantic-value/src/{declaration,containment,enumeration,lib}.rs; qsl-semantics/src/value/{operation,environment_stage,enumeration,mod}.rs; qsl-semantics/src/check/{assemble,check,mod}.rs; qsl-semantics/src/model/observation*.rs; qsl-semantics/tests/it/{model_operations,type_environment_model}.rs; tests/it/family_outcome_layering.rs; xtask/src/import_graph.rs; about 52 repointed callers"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-358 (slice 2). PR: quire-spec-language#566 at bcc946cd, diff
`a7df1ff0...bcc946cd` (slice 1's head is the base).

The coder's claims, checked against the code:

- **Move is a move.** `git diff -M` pairs `qsl-semantics/src/value/declaration.rs`
  with `quire-semantic-value/src/declaration.rs` (92%) and `containment.rs` (97%).
  Every hunk is one of: `std` to `alloc`/`core` imports, the operations cut, the
  limit cut, doc text, or `pub(crate)` to `pub` on items callers outside the
  crate now need (`fill_slots`, `composites`, `object_types`, `admits`,
  `EqualityOperand::target`, `check_equality_in`, `admits_equality_conversion`).
  No logic hunk touches construction, checked equality, equality planning,
  containment or refusal order. `EnumValue`, `EnumMemberIndex` and `compare_enum`
  are the same code as before, with `EnumValue` built through
  `EnumValue::admitted` (FND-001). Confirmed.
- **Cut 2 mapping is exact.** On a7df1ff0 the registry built a limit at three
  places: `WorkBudget::charge` (WorkBudget), the `u32` type-count overflow in
  `compute_ancestors` (WorkBudget) and `check_ancestor_steps` (NodeCount). At
  bcc946cd the same three places build `WorkUnits`, `WorkUnits` and
  `AncestorSteps`, with the same bound and actual arguments. `stage_limit` maps
  `AncestorSteps` to `NodeCount` and `WorkUnits` to `WorkBudget` and passes bound
  and actual through to `LimitExceeded::new` unchanged. The assembler's
  `admit_types` maps the failure before it matches, so its arms are unchanged.
  No other production caller matched on the admission's error type. Confirmed.
- **Cut 1 resolves identically.** The old `TypeEnvironment::operation` walked
  `object_types` (admitted types only) in `EffectiveId` order. `OperationTable::resolve`
  walks the table in `EffectiveId` order and keeps only keys with
  `types.object_type(key).is_some()`, which restores the admitted-only filter
  that `conforms` (reflexive for any key) would not give on its own. The candidate
  set, the nearest filter and the three outcomes are the same code. The assembler
  declares a type's operations only when it has some, which matches the old
  `unwrap_or_default()`. The operations of a model that fails `model_object_types`
  can reach the table, but the assembly then refuses, and `resolve` ignores types
  that were not admitted. Confirmed.
- **Leaf discipline.** No `std::` path, no `qsl_*` import in the three moved
  files. `quire-semantic-value/Cargo.toml` is unchanged by this PR: `quire-exact`,
  `quire-canonical`, `serde` and `thiserror`, none with `std`. `OperationEffect`
  and `qsl_foundation::diagnostic` no longer appear in SV. The gate log shows
  `cargo build --locked -p quire-semantic-value --target thumbv7em-none-eabi`.
  Confirmed.
- **No shims, no duplicates.** There is no `pub use quire_semantic_value` and no
  `declaration`/`containment` re-export anywhere. The flat
  `qsl_semantics::value::{ValueGraph, ..}` re-export is deleted, and its one test
  user imports from `quire_semantic_value::containment`.
  `EnumValue`, `EnumMemberIndex`, `TypeEnvironment`, `EnvironmentFailure` and
  `ValueGraph` are each defined once. `src/protocol_artifact`'s `ValueGraph` is an
  unrelated private type with the same name. Confirmed.

Rust-review lane: SV keeps `forbid(unsafe_code)`, `missing_docs` warn and clippy
`all` deny. No new `unwrap`, `panic!` or narrowing conversion is added.
`EnvironmentLimit` keeps the `u128` actual that `LimitExceeded` had.
`stage_limit` is `pub` but only `stage_failure` and its own test call it. That
is harmless.

Test oracles: `each_ceiling_is_its_stage_limit` asserts both kinds with bound
and actual. `divergence_chain_past_the_ceiling_refuses_at_check_and_at_evaluation`
(TC-220) and `work_limit` still assert the unchanged `LimitExceeded`, catalog code
and missing locus, now through `stage_failure`. `model_operations`'
`an_operation_and_its_frame_admit_and_assemble` and
`admission_is_deterministic_regardless_of_document_order` read operations through
`declared_by` and compare real effects. The `state_clauses` tests built on
`ambiguous_operation_document` exercise the ambiguous and inherited resolution
end to end. All of these are real oracles, and all pass in the gate log.

Gate: the coder's `make ci` log ends `head=bcc946cd8154ceefeb1ee0f9cb476d359b726015 exit=0`.
I did not re-run it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `EnumValue::admitted` is a new `pub` trusted constructor in the leaf a backend will depend on. Any crate can now build an `EnumValue` from any declaration key, member key, position and case, and `EnumValue::variant` turns the member key into a kernel `VariantId` through `VariantId::from_digest`. Before, only `EnumDeclaration::admit_member` could build one. It has one caller, and slice 3 replaces it, so this is recorded, not blocking. Its doc for `position` also still says "Widened from `pub(crate)` (this change)" and names `check::check::Typer::name`, which is stale history in a leaf crate. | quire-semantic-value/src/enumeration.rs:28-46,58-69,82-89 |
| FND-002 | medium | `value::operation` sits in `value`, but it imports `crate::model::domain_package::OperationEffect`, and `model::observation` and `model::observation::document` import `value::operation::OperationDeclaration`. That is a module cycle between `value` and `model` inside layer 3, against ADR-011 §6.1's `semantic_value < model`. The old `value::declaration` had the same upward import, so the cycle is carried over, not new. But this PR creates the module and picks where it goes. `import_graph`'s permitted list only checks membership, so nothing catches the cycle. The table holds `model` data, and `check` and `model` consume it, so put it in `model` (for example `model::operation`). That removes the upward edge, and `check` may already import `model`. | qsl-semantics/src/value/operation.rs:16; qsl-semantics/src/model/observation.rs:41; qsl-semantics/src/model/observation/document.rs:21; xtask/src/import_graph.rs:415-418 |
| FND-003 | low | `Scope::new` sets `operations` to an empty table. Only `PackageDeclarations`' scope build calls `with_operations`. Before, operations travelled inside `types`, so any `Scope` holding the types could resolve them. Now a `Scope` built without the extra call answers `Missing` for every operation frame, silently, in `CheckedGraph::operation_frame`. Today only the one production path builds a scope with model types, so this is a trap for later code, not a bug now. Pass the table to `Scope::new` next to `types` so it cannot be left out. | qsl-semantics/src/check/check.rs:543-559; qsl-semantics/src/check/mod.rs:684-693 |
| FND-004 | low | The code comments narrate history instead of stating what is true. `family_outcome_layering.rs`'s `BELOW_CORE` doc says "QSL-358 slice 2 moved `value::declaration` there too, and added ...". `value/mod.rs`'s list of `pub` submodules names `enumeration`, `model_query` and `operation` but leaves out `environment_stage`, which is also `pub`. The `EnumMemberIndex` doc is split around its `#[derive]`, so its second paragraph comes after the attribute (carried over from before). Say what the constant holds, add `environment_stage`, and put the doc above the derive. | tests/it/family_outcome_layering.rs:33-38; qsl-semantics/src/value/mod.rs:113-120; quire-semantic-value/src/enumeration.rs:92-107 |

## Verdict

The move is behaviour-identical. Construction, checked equality, equality
planning, containment, refusal order and the limits all hold when measured.
The cut-2 mapping is exact at all three limit sites, and cut 1 gives the same
answer as before with the admitted-only filter kept. SV takes on no `std` and no
QSL dependency. There are no shims and no duplicate types. The tests are real
oracles. FND-002 should be fixed in this PR, because the module is new and moving
it is cheap. FND-001 waits for slice 3. FND-003 and FND-004 are low.

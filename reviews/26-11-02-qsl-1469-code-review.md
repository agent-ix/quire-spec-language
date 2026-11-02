---
id: SR-202611021517
title: "Code review — QSL-1469 alias-less operation-effect declaring nodes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/lowering/model.rs; qsl-package/src/emit/tests/variant_types.rs; FR-496; FR-094; FR-360-AC-8"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-496
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-360
    type: references
---

## Summary

Ticket: QSL-1469. PR agent-ix/quire-spec-language#2119, one commit (7da97e582) over origin/main 547b1792e. Code review with the Rust review lane folded in.

The PR fixes two qsl-package lib tests. Both went red with #2083 (aaaef0ccc, FR-496 operation effects).

1. `Lowering::operation_effect_nodes` now keeps models that have no `model` alias. The dispatch bridge admits models through `AdmittedModel::from_view` with no alias. Their declaration nodes are now located under `type_declaration(selected_document)`.
2. `an_identifier_cased_variant_type_emits_and_reports_enum_member_witness` now expects four `variant_type` nodes (Grade, Level, Odd, Tier) instead of one.

## Verdict

**CHANGES REQUESTED** — fix (1) is correct and tested. Restatement (2) writes a spec deviation into the test (FND-001).

What holds, measured:

- Fix (1): node identity comes from `model_declaration_key(&model.selection.identity, …)` (model.rs:2155) and not from the location. Two alias-less models from different documents therefore get distinct nodes. The `BTreeMap` collection keeps the order deterministic. The location is used only for the `generated` occurrence and diagnostics.
- In production, `assemble.rs` admits every model `with_alias`. Alias-less models reach lowering only through the standalone dispatch bridge and test fixtures. `merge_linked` folds bridge output into the aliased package.
- Mutation check: I reverted only the model.rs hunk. `owners::model_declaration_nodes_and_clause_functions_carry_their_model_owner` then FAILED (panic at owners.rs:367, IR v2 reader refusal). It passes with the fix.
- A probe confirmed that the `Order/count` root now emits the `Sub` object_type node, that the `Sub/size` row's `declaring` names it, and that the source map is present.
- Focused runs against a fresh QSpec origin/main (68be77f8):
  - qsl-package `owners::`, `variant_types::` and `receiver_frames::`: 32 passed.
  - qsl-eval `it` dispatch_calls and state_model_family: 49 passed.
  - qsl-semantics lib `dispatch|operation_effect`: 17 passed.
- qsl-semantics `it` model_dispatch (`--features test-support`) has 2 failures (`a_dispatch_family_with_more_than_128_redefinition_steps…`, `the_checked_bridge_counts_every_walk…`, MissingClauseData). They fail identically with the fix reverted, so they come from origin/main and not from this PR.
- Rust: no `#[allow]`, no skipped or ignored tests, no compatibility shim, no new panics in production code.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium   | The variant_types test now asserts Grade, Level and Odd as `variant_type` nodes. Nothing in the unit or any FR-496 row names them. FR-094 roots only declarations that a checked node or frame subject names. FR-496 says the carrier does not change semantic graph nodes. The old assertion of one node was catching #2083's over-retention of the complete declaration view. The restatement blesses that over-retention, and fix (1) extends it to alias-less models. | qsl-package/src/emit/tests/variant_types.rs:458-473; qsl-semantics/src/check/lowering/model.rs:2047-2066 |
| FND-002 | low      | An alias-less model's generated occurrences are placed at `Origin::TypeDeclaration { name: <document identity> }`. That origin means "the type declared as name", and a document identity is not a declared type. Other synthetic sites use spaced names so they name no declared type (lowering/hybrid.rs:66). This one could collide with a declared type spelled like the identity. | qsl-semantics/src/check/lowering/model.rs:2050-2051 |

## FND-001 detail

FR-094 (spec/functional/FR-094-…md:136-140): "`check` builds the node of each model declaration that a checked node or a frame subject … names". A selected relationship is called "an unconditional exception to that reachability rule" (line 144), so the rule is exclusive.

FR-496 (spec/functional/FR-496-…md:34-35): "The member is additional checked-package data and does not change semantic graph nodes or their identities."

FR-360-AC-8 requires `Config::Tier` to be emitted as a `model`/`variant_type` node. It says nothing about other types, so it does not authorize unreachable ones. Its silence is not permission to override FR-094.

TIER_UNIT names only Tier, Settings and Account::promote. Grade, Level and Odd are variant types: no row's `declaring`, `modifies`, `creates` or `deletes` can name them. They appear only because `operation_effect_nodes` retains `record_keys()` wholesale whenever a model has a projected operation.

The minimal FR-094-conformant set is each row's declaring node plus its frame-subject nodes. It is computed from admitted records, so it is independent of whether the carrier can be formed and still satisfies FR-496-AC-5. That set would fix the missing `Sub` node (the real bug in test 1) without the extra variant types.

Recommendation: restrict `operation_effect_nodes` to the declaring and frame-subject declarations of `operation_effect_inputs()`. Restore the one-node assertion in the variant_types test, keeping the new `operation_effects` presence check.

If the owner decides that whole-view retention is intended, FR-094 and FR-496 must be amended to say so in the same PR. The test should not be restated to match code that deviates from the spec.

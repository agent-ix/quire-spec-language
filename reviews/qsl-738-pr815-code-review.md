---
id: SR-9020
title: "Code review of quire-spec-language PR #815: fields, parameters and results typed by a variant type"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@bddce547903993dcb29a7fe391d6a390a209a45b; qsl-semantics/src/check/{assemble.rs,check.rs,lowering.rs,lowering/model.rs,mod.rs}, qsl-semantics/src/model/{normalize.rs,observation.rs,observation/document.rs,observation/tree.rs}, qsl-replay/src/execute/argument.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/request.rs, qsl-semantics/tests/it/{variant_fields.rs,variant_type.rs,tc235_origin_retained.rs}, qsl-eval/tests/it/variant_fields.rs, qsl-replay/src/execute/tests/variant_fields.rs"
review_set: subset
---
# Code review of quire-spec-language PR #815

## Summary

Ticket: QSL-738. Method: code-review with its rust-review lane. PR #815 (base `impl/business-model-meanings`, merged at 2878b9bc1 including #812) admits a field, parameter or result typed by a variant type at intake. The assembler types the member by an enum shape keyed over the variant type's checked node. The checker resolves `Alias::Tier` and `Alias::Tier::Gold`. Snapshot and invocation admission read the `{"variant":{type,case}}` form in FR-360 item 9 order, and replay witness decoding includes variant types. Reviewer-only: no build or test run. Gate claims were checked against the coder's logs.

Examined:
- intake: `signature_type_refusal` callers and the removed field refusal, normalize.rs:1911-2330 (examined)
- `model_object_types` variant block, `model_value_type` Enum arm, `PackageDeclarations::assemble` name registration, assemble.rs:734-790, 1178-1230, 1848-1910, 2155-2167 (examined)
- `variant_type_members`, `VariantTypeDeclaration`, `ScopeIndex::variant_types`, `Scope::variant_*`, `Typer::variant_case`, check.rs:147-290, 640-720, 849-880, 1737-1745, 1857-1897 (examined)
- `variant_type_node_key`, lowering/model.rs:325-352; the interim emitter refusal (enum with no binding), lowering.rs:1902-1916 (examined)
- `RawNode::Variant`, `RawVariant`, `SnapshotValue::variant*`, observation.rs:545-880; `read_raw_value` variant arm, tree.rs:296-331; `References::variant_type`, `admit_variant`, document.rs:879-990, 1194-1240 (examined)
- `enum_declarations`, argument.rs:83-115; `derive`, composite_domain.rs:257-300 (examined); clause request reader, spine/request.rs:195-222 (examined)
- `tc_255_*` and `tc_260_*` scans and `RAW_WIRE_TYPE_FIELDS`, xtask/src/typestate_scan.rs:1023-1061, 1334-1480 (examined)
- deleted `a_field_typed_by_a_variant_type_keeps_its_origin` against FR-056-AC-33 and TC-1005 at the merged base (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The TC-260 failure was evaded by renaming fields, not fixed. The first gate failed `tc_260_value_type_ref_is_the_two_member_union` with "RawVariant.type_identity is a bare str" and "References.variant_types is a bare String". Commit 8aeeb13da renamed the fields to `identity` and `variants`. The values are unchanged: the same raw wire type-identity string, and `RawVariant::type_identity()` still returns it. The scan only checks field names that contain `type`, so the rename gets past it. `RawRecord.type_identity` is the exact sibling and is listed by name in `RAW_WIRE_TYPE_FIELDS`, and the PR #807 review rejected "renaming the string to get past the scan". Fix: restore `type_identity` and add `(observation.rs, RawVariant, type_identity)` to `RAW_WIRE_TYPE_FIELDS` (the existing `(observation.rs, RawNode, type_identity)` row already covers the enum variant's field). Do the same for the `References` cache key, or state why it is not a model field type. | qsl-semantics/src/model/observation.rs:552,584; qsl-semantics/src/model/observation/document.rs:884; xtask/src/typestate_scan.rs:1334-1355 |
| FND-002 | low | Any unbound name equal to a case name of any selected variant type now refuses `ill_typed`/`type-mismatch` instead of `missing_declaration`/`missing-name`, with or without a variant slot. For example, an undefined `Standard` in an Integer expression of a unit that selects the package. FR-360 item 4 gives the type-mismatch refusal to a bare case name at a variant slot only, and FR-317 refuses a name nothing binds as missing-name. Fix: check `is_variant_case_name` only when the expected type is a variant shape, else fall through to `MissingName`. | qsl-semantics/src/check/check.rs:1874-1897 |
| FND-003 | low | Replay admission derives the case keys, ranks and enum shape of each variant type a second time. It takes the rank to be the position in `EnumRecord.members` and does not apply the identifier filter that `VariantTypeDeclaration` applies. Agreement with the checker's literals depends on two derivations staying in step: intake's sort, check.rs's re-sort and `variant_type_members`. Fix: build `VariantAdmission` from the scope's `VariantTypeDeclaration` (`shape()`, `members()`), or move one builder into `check` and call it from both. | qsl-semantics/src/model/observation/document.rs:940-990; qsl-semantics/src/check/check.rs:410-478 |
| FND-004 | low | A new test doc comment carries a ticket id and a stale blocker: "until QSL-748 requirement 1 holds in the emitter". QSL-748's QSpec rows are merged, and the emitter now waits on the sibling-crate implementation. Fix: state the condition, for example "until the IR reader admits a `model`/`variant_type` node", with no ticket id. | qsl-semantics/tests/it/variant_fields.rs:1043-1046 |

## Verdict

**CONDITIONAL**: no high findings. FND-001 should be fixed in this PR. It is the scan-gaming pattern this repo has already rejected once.

Judgments the brief asked for:
- **Mint move (TC-255): a real fix.** `variant_type_members` mints a `NodeKey` from a `VariantId` digest. It moved from `value/enumeration.rs` into `check/check.rs`, next to its only caller, `VariantTypeDeclaration::new`. lowering.rs:3931 already uses the same pattern inside `check`. The scan's rule is "every mint is in check". The minting function now is, and `References::variant_type` only calls `crate::check::variant_type_node_key`, as the existing `record_node_key` call at document.rs:1017 already does.
- **Field rename (TC-260): an evasion** (FND-001).
- **Deleted `a_field_typed_by_a_variant_type_keeps_its_origin`: coverage preserved.** The refusal it pinned is gone by design (FR-360 item 1). The merged FR-056-AC-33 no longer lists a variant-typed field, and TC-1005 step 2 no longer has that mutation (#808 merge). The positive replacements are `a_field_typed_by_a_variant_type_is_admitted` and TC-1006 step 1.
- **Known gap 2 (`Gold Plus`): not an invented code.** `UnsupportedVariantType` keeps its catalog code `unsupported_construct`/`declaration-form`, and a field typed by such a type gets the existing `UnsupportedModelMember`. Both are typed. The behaviour contradicts FR-360-AC-4 and QSpec FR-141-AC-8, which is recorded in the gap analysis (SR-9021) as deferred to QSL-752 part 2. In an expression, `Config::Weird::Gold` refuses `missing-name` rather than the type-position cause. That is acceptable only while QSL-752 is open.
- **Known gap 1 (interim emitter refusal):** it is the implicit "enum with no binding" refusal of lowering.rs:1902-1916, as FR-360 item 12 directs. No stand-in lowering node was added (planner ruling (a)).

Rust lane:
- One production `expect`: `u32::try_from(member.position()).expect(..)` in `variant_case`, check.rs:1876. It copies the enum-literal precedent at check.rs:1849, and it cannot be reached within document byte limits. Not filed.
- No `unsafe`, no `as` casts, no new `#[allow]` outside test fixtures. `IdentityLimits { input_bytes: u64::MAX }` in `References::variant_type` copies the `record_keys` precedent.
- The admission order (missing `type` before `case`, extra member, foreign type, undeclared case) matches item 9. The non-object payload, and a non-text `type`/`case`, fall to `wrong-value-kind` through `read_raw_value` returning `None`.
- Equality and Set order use the one checked rank. Intake sorts `EnumRecord.members` by `&str` byte order (intake.rs:3893), `VariantTypeDeclaration::new` sorts again (redundant and harmless), and replay re-derives the order (FND-003).
- Keys do not depend on the alias. `variant_type_node_key` is a function of `ModelOwner(package, node)` only, and the case key is `mint_variant_id(node, case)`.
- Diff scope: every changed file serves FR-360. No unrelated churn, no vendoring, no SHA or pin tracking added.

Gates, from /home/peter/dev/worktrees/logs/qsl738c-*:
- In gate.log (first run), the crates step failed on the stale `variant_type.rs:1140` expectation, and xtask failed TC-255 and TC-260.
- In gate2.log, clippy exited 101 on an unused `SelectedObject` import. The crates step exited 0: semantics `it` 649 passed and 1 ignored, eval 54, replay lib 496, forms 60. The root crate exited 0 (986 passed), xtask exited 0 (182), and `make check-spec-validation` exited 0.
- bddce5479 changes only that import line in a test file, and clippy3.log shows `CLIPPY EXIT 0` after it. Running the test steps on 8aeeb13da and clippy on the head is sound, because the one later change is a test-only import.
- No `cargo fmt --check` log was found among the qsl738c logs. Treat fmt as unverified.
- The base has since advanced to a76965a76 (#811 code, #813 spec), so the merge-time gate must run on the updated base.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The new `ValueEncoder::encode` writes Boolean, Integer, variant, `Option` and `Sequence` values only. For a record (`Composite`), `Set`, `Bag`, `OrderedSet`, Text or Reference it returns `None`. A `Settings` value whose `tier` is a variant therefore cannot be encoded, even though item 9 admits a variant inside a record field. The encoder also has no production caller; it is public API that only tests exercise. It is the only writer of the FR-106 value form, so there is no duplicate implementation. `to_json` builds the arena children first and writes `type` and `case` exactly, and the sequence order is deterministic. Fix: add the record arm (`RawNode::Record` with the declaration identity) and the unordered collections in key order, or document in the doc comment that the encoder covers only these forms. | qsl-semantics/src/model/observation/encode.rs:58-102 |

## Dispositions

Round 1, reviewed at 101dbdf0809546eae22d2047c429a58f805ff612.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 101dbdf08 |
| FND-002 | fixed | 101dbdf08 |
| FND-003 | fixed | 101dbdf08 |
| FND-004 | fixed | 101dbdf08 |

## Dispositions (round 2)

Round 2, reviewed at d764d9277feeebb94c79e96e4740c92e0c94df67.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | d760c7210 |

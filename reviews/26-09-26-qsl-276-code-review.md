---
id: SR-722
title: "Code and Rust review of S2 state clause forms and model operations at intake"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@49b3398ab012c0635fc1b2306d536312e319de11; qsl-forms/src/dispatch.rs; qsl-forms/src/lib.rs; qsl-forms/src/protocol_clause.rs; qsl-forms/src/syntax.rs; qsl-forms/src/value.rs; qsl-forms/tests/it/main.rs; qsl-forms/tests/it/protocol_clause_forms.rs; qsl-forms/tests/it/value_forms.rs; qsl-semantics/src/check/assemble.rs; qsl-semantics/src/check/check/typing.rs; qsl-semantics/src/check/checked_dispatch.rs; qsl-semantics/src/check/family.rs; qsl-semantics/src/model/intake.rs; qsl-semantics/src/model/refusal.rs; qsl-semantics/src/value/declaration.rs; qsl-semantics/tests/it/main.rs; qsl-semantics/tests/it/model_operations.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: reviews
---
## Summary

Ticket: QSL-276. PR: quire-spec-language#486 at 49b3398a, base e9d235c9.
Code review with the rust-review lane, over the 17 changed files, against
FR-102, FR-103 and TC-456..458.

Sound: the S2 production (`protocol_clause::state_clause`) reads the kind
from the leading token alone, keeps the alias, context and operation spelled
and unresolved, and routes the body through the shared `value::expression`
builder, so the forms depth limit applies unchanged. The three new
`Expression` variants are handled in every exhaustive match (`children`,
`take`, `is_childless`, `checked_dispatch::open`, `family::encode_expression`,
`body_type_forms`, `Typer::infer_form`). The qualified `reaches` edge refuses
`UnrepresentedConstruct` at the edge span. Frame resolution is two-phase
(`PendingFrame` queued while reading, `resolve_pending_frames` after every
node is read), so an entry can name a type declared later. The assembler
reuses FR-056's field rule through the extracted `model_value_type`. No new
`unwrap`/`expect`/`panic!`/`unsafe` outside tests.

Reviewer ran `cargo test -p qsl-forms -p qsl-semantics --test it` at
49b3398a: 24 and 138 passed, exit 0.

## Verdict

**Changes requested.** Three high findings and one medium; the rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The assembler silently drops every state clause. `Unit::new` matches `DeclarationForm::StateClause(_) => {}`, and it is the only consumer of `ParsedUnit` forms outside qsl-forms (grep of `DeclarationForm::`). Before this PR a unit holding `invariant` refused at S2 with `NoDispatchEntry`. Now it builds, and the clause is discarded before any check. So `invariant X using v on M::Nope at current { false }` beside a checking function produces a checked, emittable package that says nothing about the invariant, and the undeclared alias `M` is never reported. That is a wrong result, not a deferral. Fix: refuse the unit at assembly with `unknown_required_feature`/`unsupported-feature` at the clause's span until FR-104 (QSL-277) checks and declares it. The FR-102 tests exercise `build_unit` only and are unaffected. | qsl-semantics/src/check/assemble.rs:431-439 |
| FND-002 | high | The FR-103-AC-5 determinism test is tautological. `reordered` is built by the same `config_version_document(attempt_update(json!([]), frame), ...)` call as `document`, so the third admission compares a document with itself. Nothing is moved, although AC-5 and TC-458 step 5 require the operation and population records to be moved to the end of the document. The test also compares only `effect().modifies`, not the equal `PackageDeclarations` that AC-5 requires. The long comment at 820-832 admits that the fixture has nothing to reorder. Fix: build a genuinely reordered document, for example a second object type placed before `ConfigVersion`, the population listed first, and the frame naming a field of the type declared later. Then assert equality of the assembled declarations (types and operations) and of all three effect lists. | qsl-semantics/tests/it/model_operations.rs:771-841 |
| FND-003 | high | The test pins behaviour that contradicts FR-103-AC-2 (coder's question 3). `validate_with_semantic_ir` maps every FCD error diagnostic to `malformed_declaration`, including FCD's `UNRESOLVED_FRAME_PATH`. FCD emits that code for an entry naming nothing and for one naming the wrong kind (constructs.rs `frames` at FCD 033e228), so QSL's own `missing_frame_declaration` can never fire. The test asserts `invalid_model_binding`/`malformed-declaration` for `modifies [ConfigVersion/missing]`, where AC-2 says `missing_declaration`/`missing-name`. TC-458's last step 2 case (`modifies` missing plus `creates` a field, report only the `modifies` refusal) is swapped for a different fixture because FCD reports both. The spec is not wrong: the mapping from FCD diagnostics to QSL refusals is QSL's own code in this repo, and `resolve_pending_frames` is a strictly finer classifier than FCD's rule. Fix: do not surface FCD `UNRESOLVED_FRAME_PATH` as a refusal and let `resolve_pending_frames` decide. Keep FCD's shape checks. Then assert `missing_declaration`/`missing-name` and restore TC-458's exact fifth case. | qsl-semantics/src/model/intake.rs:910-933; qsl-semantics/tests/it/model_operations.rs:462-505; qsl-semantics/tests/it/model_operations.rs:591-634 |
| FND-004 | medium | FR-103 requires every frame-entry refusal to be made "at the entry, retaining the operation's IR node identity, the entry, the artifact id and the span". No AC-2 test asserts anything but `code` and `cause`, so node, entry and locus are unverified. The wrong-kind arm of `resolve_frame_array` builds `malformed_declaration`, whose cause has no `entry` field, so the entry survives only inside the free-text `detail`. Fix: assert node, entry and span in the AC-2 tests. Carry the entry in a structured field for the wrong-kind refusal, as `FrameEntryMissing`/`FrameEntryUnsupported` already do. | qsl-semantics/src/model/intake.rs:2145-2153; qsl-semantics/tests/it/model_operations.rs:498-504 |
| FND-005 | low | Stale docs: `AssemblyCause::UnsupportedModelMember` and `model_object_types` still list "an operation" among the records that refuse as `UnsupportedModelMember`, which is no longer true. | qsl-semantics/src/check/assemble.rs:240-243; qsl-semantics/src/check/assemble.rs:483-490 |
| FND-006 | low | `resolve_pending_frames` writes each effect back with `if let Some(..) = records.iter_mut().find(..)`. If the operation record were not found, the resolved effect would be dropped silently and the operation would admit with the empty effect. The case is unreachable today, but the failure mode is a wrong grant, not a refusal. The immediately invoked closure `(\|\| { .. })()` is also better written as a named `fn resolve_frame(..) -> Result<OperationEffect, ModelRefusal>`. Fix: make a miss a broken-invariant refusal, or build the effect into the record in place. | qsl-semantics/src/model/intake.rs:2177-2222 |
| FND-007 | low | The FR-102-AC-1 test checks the spans of the root and its left child only. TC-456 step 1 expects each sub-expression's span to slice the source to its own text (`self.parent`, `value(self.parent)`, and so on). Fix: walk every node of `form.spans.body` and assert that each slice parses back to that node, or list the expected slices. | qsl-forms/tests/it/protocol_clause_forms.rs:97-130 |

## Rust review

- Panic surface: clean. New `panic!`/`expect` only in tests.
- Enum growth: `Expression` gains three variants and `DeclarationForm` one, and every exhaustive match is updated with no wildcard arms. `Typer::infer_form` keeps its `wildcard_enum_match_arm` deny.
- Boxing: `DeclarationForm::StateClause(Box<StateClauseForm>)` matches `Function`'s precedent.
- Visibility: the `value` helpers widen to `pub(crate)` only, with no public API leak. `OperationDeclaration` is public with accessor methods, matching `FieldDeclaration`.
- Determinism: `classification` is a `HashMap` used only for lookups, and the output order follows the `pending` and entry order, so it is deterministic.
- Idioms: see FND-006.

## Round 2 findings

These are new at 96049017 (fix round 3b94b129 plus the fmt commit 96049017).
They were found by code reading. The reviewer ran `cargo test -p qsl-forms -p
qsl-semantics -p qsl-replay` in the worktree's own target dir: all passed.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | The `UNRESOLVED_FRAME_PATH` filter lets an unresolved frame through on any type node whose QSL reader never reads `operations`. FCD makes `operations` optional on every construct kind (`Member::Operations` defaults to `Presence::Optional`, vocabulary.rs:179-187 at 033e228), and FCD's `frames` walks every type's operations. But only `read_object_type` queues a `PendingFrame`. `read_component`, `read_endpoint`, `read_connection` and `read_allocation` ignore `operations` entirely. So a systems part carrying an operation with `modifies: ["ix://.../nope"]` was refused before the fix round and now admits, with the operation silently dropped. Fix: have those four readers refuse a non-empty `operations` (`unsupported_at`), or drop only the diagnostics whose pointer lies under an operation QSL queued. Add a test with a part carrying a broken frame. | qsl-semantics/src/model/intake.rs:926; qsl-semantics/src/model/intake.rs:1723-1740; qsl-semantics/src/model/intake.rs:1742-1775; qsl-semantics/src/model/intake.rs:1776; qsl-semantics/src/model/intake.rs:1927-1941 |
| FND-009 | medium | Nothing tests the new `UnsupportedStateClause` refusal (grep: no test references it). The fix for FND-001, a high wrong-result finding, is unguarded: going back to a silent drop would keep every gate green. Fix: a test that assembles a unit holding an `invariant` beside a checking function and asserts `unknown_required_feature`/`unsupported-feature` at the clause's declaration span. | qsl-semantics/src/check/assemble.rs:1133-1141 |

## Dispositions

Round 2, reviewed at 960490173a2601bf8fd9e76b9ccf344242b18002.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3b94b129: `AssemblyCause::UnsupportedStateClause` refuses each clause at its declaration span (assemble.rs:258, 453-459, 1133-1141; qsl-replay spine.rs:371). No test guards it; see FND-009. |
| FND-002 | fixed | 3b94b129: two genuinely reordered documents (the `AuditLog` type before and after `ConfigVersion`, a cross-type frame entry), with the assembled declarations of both types compared (model_operations.rs:798-951). |
| FND-003 | fixed | 3b94b129: FCD `UNRESOLVED_FRAME_PATH` is filtered (intake.rs:926), and the AC-2 tests assert `missing_declaration`/`missing-name` and TC-458's exact fifth case. For object and interface types every filtered diagnostic still ends in a refusal: QSL accepts only `Field` (modifies) or `ObjectType` (creates/deletes), both a subset of what FCD resolves. The filter opens a hole elsewhere; see FND-008. |
| FND-004 | fixed | 3b94b129: `ModelRefusalCause::FrameEntryMalformed` carries the entry, and all four AC-2 cause tests assert node and entry (the missing case also asserts the span). |
| FND-005 | fixed | 3b94b129: the stale "an operation" lists are removed (assemble.rs:240-245, 502-510). |
| FND-006 | fixed | 3b94b129: the closure is now `fn resolve_frame` (intake.rs:2198). A miss is a `debug_assert!` rather than a refusal, so a release build still skips silently. Accepted, because the miss is unreachable by construction. |
| FND-007 | still-open | `qsl-forms/tests/it/protocol_clause_forms.rs` is unchanged at 96049017; the FR-102-AC-1 test still checks two spans. |

## Dispositions (round 3)

Round 3, reviewed at fe9224fcffc69700dd7467cabab243e76206f399. The branch is
rebased onto origin/main 6db63a3e. `git range-diff` shows all four commits
unchanged (`=`). In the four files both sides touched (`value_forms.rs`,
`assemble/tests.rs`, `family.rs`, `intake.rs`), the PR's changed lines
relative to main are identical to those before the rebase, so main's changes
are kept. The reviewer ran `cargo test -p qsl-forms -p qsl-semantics -p
qsl-replay` with `CARGO_TARGET_DIR` set to the worktree's own `target/`: all
passed, including the two new tests.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3b94b129 (bef8bb35 after the rebase); guarded now by FND-009's test |
| FND-002 | fixed | 3b94b129 (bef8bb35) |
| FND-003 | fixed | 3b94b129 (bef8bb35); the hole the filter opened is closed by FND-008 |
| FND-004 | fixed | 3b94b129 (bef8bb35) |
| FND-005 | fixed | 3b94b129 (bef8bb35) |
| FND-006 | fixed | 3b94b129 (bef8bb35); a debug_assert, accepted as unreachable |
| FND-007 | fixed | fe9224fc: `assert_every_span_slice_reparses` walks every body node and checks that each span's slice reparses as one whole expression (protocol_clause_forms.rs:70-99, called at 174). |
| FND-008 | fixed | fe9224fc: `read_component`, `read_endpoint`, `read_connection` and `read_allocation` refuse a non-empty `operations` through `unsupported_at`. That covers every reader that accepts a type node without queuing its frames: object and interface types queue theirs, and a record value type refuses. Test: `a_systems_part_carrying_an_operation_with_a_broken_frame_refuses` (model_intake.rs:1064) asserts `unsupported_construct`/`declaration-form`, which would fail without the guard. |
| FND-009 | fixed | fe9224fc: `a_state_clause_refuses_unsupported_until_fr_104` (assemble/tests.rs) asserts the exact error and the span. |

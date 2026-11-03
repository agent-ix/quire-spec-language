---
id: SR-20261103161227
title: 'Code review of PR #2195: QSL-1531 successors for retained composed criteria, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@173a1b75a; git diff origin/main...173a1b75a (PR #2195, branch peter/qsl-1531-composed-successors): qsl-semantics/src/check/{assemble.rs,check.rs,mod.rs,protocol_relationship.rs,lowering/model.rs,protocol_clause/*}, tests/it/{call_requirements_items.rs,composed_domain_models.rs,retired_native_runtime.rs}, Cargo.toml; ticket QSL-1531'
review_set: subset
date: '2026-11-03'
---

## Summary

Ticket: QSL-1531. PR: quire-spec-language#2195, head 173a1b75a, trial-merged onto origin/main f64bfc001. Method: code-review with the rust-review lane. Independent second review (Opus); an earlier code-review and gap-analysis of this PR exist on the ticket and their findings FND-002, FND-004..007 are deferred to QSL-1579 / QSL-1580. Those are not repeated here.

The PR adds five TC-1308 spine tests (rows 5, 7, 9, 11, 14, 15, 17), lets systems parts, ports and allocations through model type assembly instead of refusing `UnsupportedModelMember`, binds a protocol role `on M::<port>` to the Port's owning object type plus a new `CheckedRole.model_member`, lets a protocol `relationship` name an admitted FR-152 connection, and switches the FCD fixture helper from a temp copy to hard links.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-513-AC-3 | spec/functional/FR-513-name-the-successor-or-keep-of-each-retained-criterion-that-rides-a-retired-path.md | examined |
| FR-513-AC-4 | same | examined |
| FR-513-AC-5 | same | examined |
| FR-513-AC-6 | same | examined |
| FR-513-AC-7 | same | examined |
| FR-513-AC-8 | same | examined |
| FR-493-AC-5 | spec/functional/FR-493-*.md | examined |
| TC-1308 rows 5-17 | spec/test-cases/TC-1308-*.md | examined |
| role binding (CheckedRole.model_member / model_selection) | qsl-semantics/src/check/protocol_clause/mod.rs:217-240 | examined |
| relationship ends via admitted connection | qsl-semantics/src/check/protocol_relationship.rs:146-188 | examined |
| AdmittedModel::port_roles / admitted_connection_ends | qsl-semantics/src/check/lowering/model.rs:403-466 | examined |
| model_object_types systems arm | qsl-semantics/src/check/assemble.rs:1351-1360 | examined |
| hard_link_tree / write_overlay | tests/it/composed_domain_models.rs:93-124 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `CheckedRole.model_selection` is now set for every role whose context is qualified by a model alias (`role w on Spec::Counter`), not only a whole-model role, because the lookup changed from `model_selection(&role.context.name)` to `model_selection(alias)`. The field doc still says "when a static role is `on` a model alias". Consumers change behaviour: refinement `acts_on_abstract_model` and imported-class matching (check/refinement.rs:2693, 2649, 2721; check/mod.rs:6710) now count a qualified non-object-type role as acting on the model, and qsl-replay refinement/subject.rs:107 now binds such roles to the selection identity instead of `context.original().package()` or refusing. No test covers the widening; only the Port role needs it. | qsl-semantics/src/check/protocol_clause/mod.rs:221-235 |
| FND-002 | medium | The predicate-error test declares `function Dependent ... { Caller() }` but never asserts it refuses; FR-513-AC-4 / TC-1308 row 9 require "each caller of a refused predicate refuses too". The dependent-propagation clause is untested. | tests/it/call_requirements_items.rs:370-400 |
| FND-003 | medium | The non-Boolean predicate case asserts only `!refusal.causes().is_empty()`. Any refusal at any stage passes. FR-513-AC-4 requires the refusal "at the assembler" at the result form; the test does not pin `CompileRefusal::Assembly`, the cause, or the span. | tests/it/call_requirements_items.rs:466-475 |
| FND-004 | medium | `hard_link_tree` hard-links files from the cargo git checkout (~/.cargo/git/checkouts/filament-core-data-*/…/fixtures) into `tempfile::tempdir()`. `std::fs::hard_link` fails with EXDEV when the temp dir is on another filesystem (tmpfs `/tmp`, a separate CARGO_HOME volume), so every `architecture_bundle` test panics there. Any write that does not go through `write_overlay` (for example `std::fs::write(root.join("spec/model/Reading.md"), ..)` in tests/it/domain_protocol_emission.rs:54/383/940, safe today only because the fixture has no Reading.md) writes through the link into the shared checkout. The earlier "copy duplicates upstream fixture data" finding this answered is not vendoring (a test-time temp copy never enters the repo); the plain copy was the safer form. | tests/it/composed_domain_models.rs:93-124 |
| FND-005 | low | FR-493-AC-5 ("`quire matrix` lists no retained criterion untagged because its test was deleted") is added to `native_package_and_runtime_surfaces_are_absent`, which asserts symbol absence and nothing about the matrix. The tag silences an orphan rather than verifying the criterion; TC-1308 row 25 / FR-513-AC-15 is the matrix test that owns it. | tests/it/retired_native_runtime.rs:60 |
| FND-006 | low | Several emitted-record assertions are near-tautological: `wire.to_string().contains("x")`, `contains("meter")`, `contains("saved")`, and `declaration["node_id"].is_object()` / `semantic_type.is_object()`. The doc comment on the empty-query test ("rows 11 and 16 ... binder/type/unit selections") still describes the AC-7 claim the last commit untagged. | tests/it/call_requirements_items.rs:294, 477-479, 583-593 |
| FND-007 | low | After both branches of `ends` produce `ValueType::Reference`, the `ends.iter().any(|end| !matches!(end, ValueType::Reference(_)))` refusal is unreachable dead code. | qsl-semantics/src/check/protocol_relationship.rs:169-182 |
| FND-008 | low | `admitted_connection_ends` re-runs `classify` over the whole selected package with a fresh default meter for every protocol relationship that is not object-typed, and its `classify` error is folded into `None` (reported as `Unimplemented`, not as the exhausted/refused cause). | qsl-semantics/src/check/lowering/model.rs:433-447 |

## Verdict

Changes requested. FND-001 to FND-004 are medium and should be fixed in this PR. FND-005 to FND-008 are low.

What is right: the S3 role binding to a Port's owning object type plus `model_member`, the admitted-connection fallback for protocol relationships, and the Component/Endpoint/Allocation pass-through in `model_object_types` are coherent and confined. The TC-1308 row 5, 7, 11, 14 and 15 tests assert real values: exact call count, the shared callee node, argument order per call text, distinct receivers, typed `MissingMeasure` cycles of length 2 and 3, `ill_typed` spans inside the call and on `3 s`, `unproved-nonzero` on `rational(10, 1) / x`, the empty-query identities, `WrongValueKind` for the invalid last element, preserved duplicates and nested order, and the exact work/result bounds at U, U-1 and 0 with retry equality. Deferred gaps (QSL-1579, QSL-1580) are documented in the PR body and are not re-raised here.

Mutation spot checks were reasoned from the test text; none was executed, because the build run was stopped when the gate moved to the stack. The checks: removing propagation of a refused predicate to its callers survives (no assertion: FND-002). Making the non-Boolean predicate refuse at check instead of assembly survives (FND-003). An off-by-one in work or result charging is killed by the exact U and U-1 bounds of the row 15 test. Sharing one callee across overloads is killed by the `dependencies` and `target` equality.

Merge receipt on the trial merge with origin/main f64bfc001 (the merge was clean): `cargo fmt --check` passed. `cargo clippy --locked --workspace --all-targets -D warnings` passed. `... --all-features -D warnings` passed. `cargo test --locked --workspace` is deferred to the stack gate, on the coordinator's instruction. An earlier clippy attempt failed with E0609 on `CheckedRole.model_member` because the shared target dir held another worktree's qsl-semantics artifact that cargo treated as fresh. It passed once the sources were touched inside the lane, so it is not PR-caused.

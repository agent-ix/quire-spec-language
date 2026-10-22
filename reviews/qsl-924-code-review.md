---
id: SR-12253
title: "Code review of quire-spec-language PR #1140 (QSL-924): replay admitted-context facade compile_admitted"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9a3d2ce04c6ffcba3cdbc09803bfc2ec22f1651c; PR #1140 diff origin/main...HEAD: qsl-replay/src/{admitted,compile,lib}.rs, qsl-replay/tests/admitted_context_facade.rs, spec/functional/FR-394, spec/test-cases/TC-1088, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-394
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1088
    type: reviews
---
# Code review of quire-spec-language PR #1140

## Summary

Ticket: QSL-924. Ticket and comment text were read as data. The scope is the
PR diff only. This file includes the `rust-review` lane.

What was checked:

- **Keys are read, never rebuilt (ADR-011 T12, FR-087, Runtime AD-004).**
  Correct. `enum_table` reads `node.key()` from the `EnumDeclaration` and
  `EnumMember` nominal nodes that `Lowering::enum_nodes` inserted. No
  `NodeKey` is constructed anywhere in the diff. A missing member node is an
  `InternalFault`, not a fabricated key. `NodeKey` is re-exported at the root
  as a type only.
- **One compile path.** `compile_package` and `compile_admitted` share
  `recompile_unit`, so the owner check, the S1 limit check and the spine run
  are the same for both. The AC-1 test checks that the bytes and
  `package_id` are equal.
- **Foreign vs unknown.** `context_of` walks `CheckedPackage::dependencies()`
  as a worklist with a `visited` set. That covers the whole transitive closure
  once per package, and a diamond is handled. Own enums are looked up first.
  The checker inserts `EnumDeclaration` nodes only for the unit's own
  `scope.enums()`, so an imported enum cannot appear in the own table. Model
  variant types add `EnumMember` nodes with no declaration node. These go into
  the `members` map, but no own declaration ever looks them up, so they are
  harmless.
- **Refusal shape.** The two enums fit the ticket's four kinds. Package and
  resource refusals are raised at compile time (`AdmittedRefusal`), and
  unknown and foreign refusals are raised at lookup time
  (`AdmittedLookupRefusal`). `Fault` is a third compile-time arm for broken
  invariants. Resource is split into `StageLimit` (code
  `stage_limit_exceeded`/`resource_exhausted`) and `AboveReader`. The compile
  path raises only `Recompile`, `DependencyInput`, `LimitAboveReader` and
  `Fault`, and all four are mapped.
- **Arch boundary.** `admitted.rs` names `crate::spine` internally, which is
  allowed. `AdmittedRefusal::Package(Box<CompileRefusal>)` puts a spine type in
  a root item. The existing `ReplayRefusal::Recompile` does the same, and a
  consumer can call `.code()` without naming the type. FR-100-AC-8 is about
  `qsl_eval` paths, which this diff does not touch.
- **Rust lane.** There is no `unwrap`, `expect`, `panic!`, `unsafe` or lossy
  `as` in non-test code. The errors are typed `thiserror` enums, and the
  large variants are boxed. Accessors return borrows or `Copy` values.
- **Runs at the reviewed sha.** `cargo test -p qsl-replay --test
  admitted_context_facade`: 5 passed. `make arch-lint-api-surface-qsl`: T12-B
  to T12-F and FR-100-AC-8 pass, and T12-A is skipped (`--qsl-only`).

## Verdict

No major finding. The facade is correct and keeps the T12 rule. The one
medium finding is a vacuous AC-5 test. The other findings are low. Fix all of
them in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-394-AC-5 test is vacuous. `admitted_context_facade.rs` imports `qsl_replay::spine::{self, ParseRequest, SpineLimits}` for the AC-1 oracle, so the AC-5 test would still compile and pass if the facade needed a spine type. Its doc comment ("This file names only the crate root") is false. Fix: move the AC-5 test into its own integration-test file that imports only `qsl_replay` root items (no `spine`, no `qsl_semantics`), and have it exercise the full lookup path (`compile_admitted`, `enum_declaration`, `enum_case`, both refusal enums). | qsl-replay/tests/admitted_context_facade.rs:9; qsl-replay/tests/admitted_context_facade.rs:285-300 |
| FND-002 | low | `From<ReplayRefusal> for AdmittedRefusal` ends in a `_ =>` arm that maps every other variant to `Fault`. A compile-path refusal added later would become a silent fault instead of a compile error. Name the replay-only variants explicitly so that adding a variant forces a decision. | qsl-replay/src/admitted.rs:230-248 |
| FND-003 | low | `AdmittedRefusal::Fault` displays as `internal fault: {0:?}` (Debug output). The crate's convention, used by `ReplayRefusal::Fault` and `CallSiteRefusal::Fault`, is `internal fault in {stage}: {invariant}`. | qsl-replay/src/admitted.rs:226 |
| FND-004 | low | `Unknown { case: Some(..) }` renders as `no admitted enum declaration <id> case <c>` even though the declaration exists and only the case is missing. Use a message that says the declaration has no such case. | qsl-replay/src/admitted.rs:90 |
| FND-005 | low | Both FR-394-AC-4 `Package` assertions check only the variant. A malformed source refused for the wrong reason (for example a dependency-input error) would still pass. Assert the boxed `CompileRefusal`'s code or stage: a parse refusal for the malformed source, and the dependency-input owner refusal for the owned library. | qsl-replay/tests/admitted_context_facade.rs:254-273 |

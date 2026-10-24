---
id: SR-12119
title: "Code review of quire-spec-language PR #951: split check/protocol_clause.rs into a module directory"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@15c9be0288460b20e167c7dc6b18595541af614a; PR #951 diff against origin/main 77ab19b01: qsl-semantics/src/check/protocol_clause.rs (removed); qsl-semantics/src/check/protocol_clause/{mod,anchors,attempts,binders,coverage,types,tests}.rs; qsl-semantics/src/check/protocol_clause/tests/{anchors,attempts,binders,coverage,node_keys}.rs; spec/decisions/ADR-012-semantic-family-extension-contracts.md; spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md; tests/it/formatter_identity.rs"
review_set: subset
---
# Code review of quire-spec-language PR #951

## Summary

Ticket: QSL-847 (C0 under QSL-839). One code-review pass with the rust-review
lane folded in, over `git diff origin/main...HEAD` (16 files, +2316/-2203).
Read-only: nothing was built or run.

Preservation was checked mechanically against
`git show origin/main:qsl-semantics/src/check/protocol_clause.rs`:

- Line multiset of the old file against the concatenation of the 12 new
  files: the only differences are `use` lines, `mod` declarations,
  `pub(super)` on five private items, `super::` to `super::super::` or
  `crate::check::` paths, the `include!` path, `crate::value::DefinitionReference`
  to `crate::value::definition::DefinitionReference` in one test, one rustfmt
  re-wrap of `scope_key`'s signature (and two `let` re-wraps), new `//!`
  module docs, and two doc links rewritten as `[`check`](super::check)`.
- Item-level comparison (every `fn`, `struct`, `enum`, `type`, `impl`, with its
  attributes and doc comments, whitespace-normalised and path-normalised):
  65 items in old and new, each present exactly once, every body identical.
  59 functions; 31 `#[test]` in old and new; 30 `#[trace(...)]` in old and
  new, and the per-function attribute sets match name for name. No match arm,
  constant or literal changed.
- Visibility: `ProtocolNodeId`, `BoundAttempt`, `CheckedAttempt`,
  `CheckedProtocol` and their fields stay `pub`; `check` and `content` stay
  `pub` and are re-exported from `protocol_clause` (`pub use types::...`,
  `pub use coverage::content`). Only `scope_key`, `ScopeKey`, `resolve`,
  `check_kind`, `bind_attempts` and `shadow_refusals` gained visibility, and
  only `pub(super)`, which is the minimum for `mod.rs` to call them.
  `check/mod.rs` (`pub use protocol_clause::{CheckedAttempt, CheckedProtocol,
  ProtocolNodeId}`, `protocol_clause::check`, `protocol_clause::content`,
  `protocol_clause::CheckedAttempt`) resolves unchanged.
- `include!("../../../tests/fixtures/protocol_attempt.rs")` from
  `protocol_clause/tests.rs` resolves to `qsl-semantics/tests/fixtures/protocol_attempt.rs`.
  `tests/it/formatter_identity.rs` now reads `protocol_clause/tests/attempts.rs`,
  where `an_empty_contracts_list_binds_with_no_refusal` is a top-level `fn`
  with exactly one literal starting `post Done `, so `function_literal` still
  finds it.
- No other file in the repo includes or names the old path, apart from
  historical review artifacts.
- No `#[allow]`, no new `unwrap`/`expect` outside the moved tests.

## Verdict

FIX. The split is behaviour-preserving: every function body, every public
item and every test with its trace tags is present exactly once. One major
defect: a moved doc comment on a publicly re-exported type lost the import
its intra-doc link resolved through, so `make ci-docs` (part of `make ci`,
`RUSTDOCFLAGS="-D warnings"`) fails (FND-001). The leader's gate list did not
include `ci-docs`.

Notes (not findings): the PR body says the ADR-027 path text changed; the
diff edits ADR-017 (lines 113-115, 335), and those edits are present tense
and true (`CheckedAttempt` is in `types.rs`, `resolve` returns
`ProtocolNodeId` in `anchors.rs`, `covered_kind` is in `coverage.rs`).
ADR-012 line 1769 names `check/protocol_clause/`, also true.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Broken intra-doc link on a public item. `ProtocolNodeId`'s doc comment links [`ProtocolDeclarationForm::declarations`]. In the old file `qsl_forms::ProtocolDeclarationForm` was imported at the top; `types.rs` imports only `ClauseOperation` and `PopulationDomain`. `ProtocolNodeId` is public (`qsl_semantics::check::ProtocolNodeId` via `check/mod.rs:248`), so rustdoc documents it and resolves the link in `types.rs`'s scope, where the name is unresolved. `make ci-docs` runs `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features` and is in `CI_CHECKS`, so `make ci` fails. Fix: write the link as `[`ProtocolDeclarationForm::declarations`](qsl_forms::ProtocolDeclarationForm::declarations)` or import the type in `types.rs`. Found by reading, not by a doc build. | qsl-semantics/src/check/protocol_clause/types.rs:8; Makefile:269-270 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | Fixed | The `ProtocolNodeId` doc link in `types.rs` now carries an explicit target path, `qsl_forms::ProtocolDeclarationForm::declarations`, so rustdoc resolves it without an import. The doc comments of every other new `protocol_clause` module were checked and their links resolve in the new module scope. |

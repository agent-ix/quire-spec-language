---
id: SR-1376
title: "Code review of quire-spec-language PR #662: adapt to quire-exact Arc-shared ValueType payloads (QSL-639)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5ec9b043df24940bda88919ebf4f557f147b9fec; PR #662 diff origin/main...HEAD: qsl-semantics/src/check/check.rs, qsl-semantics/src/check/check/typing.rs, qsl-eval/tests/it/model_reference_queries.rs, qsl-semantics/tests/it/state_clauses.rs; Cargo.toml and Cargo.lock (unchanged by the PR) checked for how quire-exact resolves; counterpart agent-ix/quire-exact PR #10 head 9ea9b679"
review_set: subset
---
# Code review of quire-spec-language PR #662

## Summary

Ticket: QSL-639. PR quire-spec-language#662, head 5ec9b043d, one commit on
origin/main. It adapts QSL to quire-exact PR #10, which changes
`ValueType::Option` and `ValueType::Collection` payloads from `Box` to `Arc`.
The Rust lane (rust-review) is folded into this file.

The source change is correct and complete. Six `Box<CollectionType>` sites in
`check.rs` (2 return types, 1 parameter) and `typing.rs` (3 fields) become
`Arc<CollectionType>`. Two test constructions change from `Box::new` to
`Arc::new`. Four `use std::sync::Arc;` imports are added. A workspace grep finds
no remaining `ValueType::Option(Box::new(..))`, `ValueType::Collection(Box::new(..))`,
`Box<CollectionType>` or `Box<ValueType>`. The two `NativeType::Option(Box::new(..))`
hits in `src/checking/types.rs` are a different, crate-local enum. No compat
layer, shim or dual Box/Arc path was added.

Dependency wiring: `Cargo.toml:14` keeps
`quire-exact = { git = "https://github.com/agent-ix/quire-exact", branch = "main" }`.
There is no `[patch]` in any Cargo.toml, no `.cargo/config.toml`, and no `rev`.
`Cargo.lock` is unchanged by the PR and resolves quire-exact to
`efd4a22846ed69a5cf942797923fd6dd4f950acc`. That commit is 4 commits behind
quire-exact main (`6a36ff4b`, which adds IR-653/IR-673 #6 to #9) and still has
`Box` payloads. #10's head 9ea9b679 is main plus 2 commits.

Gates: `cargo fmt --check` passed. `cargo check` and `cargo clippy` results are
in the Verdict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `Cargo.lock` still pins quire-exact to efd4a228, which has `Box` payloads, so this head does not compile as committed: `Arc::new` and `Arc<CollectionType>` do not match the locked `ValueType`. The PR body confirms that it was verified only with an uncommitted `--config patch`. Before merge, after quire-exact #10 squash-merges, commit `cargo update -p quire-exact` on this branch so the lock moves to #10's merge commit on quire-exact main. That moves the lock past IR-653/IR-673 (#6 to #9) as well. The batch `make ci` must run on that lock. | Cargo.lock:1617-1619 |
| FND-002 | low | `check.rs` now imports `std::sync::Arc` at line 49 but keeps three fully qualified `std::sync::Arc` spellings at lines 227, 351 and 357, so the file uses two spellings for one import. Use `Arc` at those three sites. | qsl-semantics/src/check/check.rs:49; qsl-semantics/src/check/check.rs:227; qsl-semantics/src/check/check.rs:351; qsl-semantics/src/check/check.rs:357 |
| FND-003 | low | The two test sites build an option type as `ValueType::Option(Arc::new(..))`, which spells out quire-exact's pointer type. The workspace already uses the `ValueType::option(..)` constructor elsewhere, for example `equality_matrix.rs:671`, `composite_values.rs:307` and `evaluate.rs:3162`. That constructor exists before and after #10. Using it would also remove the two test-only `use std::sync::Arc;` imports, so the next payload representation change would not touch these tests. | qsl-eval/tests/it/model_reference_queries.rs:1548; qsl-semantics/tests/it/state_clauses.rs:199 |

## Verdict

**FAIL** at this head, because of FND-001. The source adaptation itself is
right. Once quire-exact #10 merges, adding the lock bump (FND-001) makes it
mergeable. FND-002 and FND-003 are low-severity idiom fixes that can go in the
same commit.

Rust lane: no `unwrap`/panic surface, `unsafe`, `#[allow]` or lint weakening
was added. No test was added or removed, and the two edited tests keep their
assertions. `Arc` clones replace `Box` clones of the same values, so behavior
is unchanged.

Gates at 5ec9b043d (via `locked-build.sh`):
- `cargo check --locked --workspace --all-targets` (committed lock): FAIL, exit 101: E0308 at qsl-semantics/src/check/check.rs:2251 and :2416, `expected Arc<CollectionType>, found Box<CollectionType>`. Re-resolving to current quire-exact main 6a36ff4b, without #10, fails the same way.
- `cargo check --workspace --all-targets` with a command-line `--config` path patch of quire-exact to #10's code (worktree at 4ed4195, whose source is identical to 9ea9b679; 4ed4195 adds only review files). The worktree `Cargo.lock` was restored afterwards: PASS, exit 0
- `cargo clippy --workspace --all-targets -- -D warnings`, same patch: PASS, exit 0, 0 warnings (`Finished dev profile ... in 1m 27s`)

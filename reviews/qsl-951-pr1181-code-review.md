---
id: SR-13760
title: 'Code review of PR #1181: migrate the string_edge marker to ix_trace_rs::string_edge (QSL-951), code-review with rust-review lane'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@9f36fcf4d; git diff origin/main...9f36fcf4d (PR #1181, branch qsl-951-migrate, merge base 7286be314): qsl-string-edge/src/detector.rs, qsl-string-edge/tests/it/*, xtask/src/{string_edge,test_suites,typestate_scan}.rs, tests/it/{string_edge_marker,family_outcome_layering}.rs, tools/arch-lint/{api_surface,qualified_core}.rs, 11 Cargo.toml manifests, Cargo.lock, README.md, qsl-string-edge/README.md, docs/family-migration-recipe.md, deleted qsl-attrs/, about 100 marker-site .rs files; ticket QSL-951'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-064
  type: reviews
---

## Summary

Ticket: QSL-951. PR: quire-spec-language#1181, head 9f36fcf4d. Method: code-review with the rust-review lane. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e. All builds ran in `git archive` scratch copies with their own target directories.

Migration completeness. Outside `spec/` and `reviews/`, `git grep qsl[-_]attrs` finds only the forbidden-pattern entry in `tests/it/string_edge_marker.rs:116`. Makefile, `.github`, `deny.toml` and docs have no `qsl-attrs` mention. Marker attribute sites: main has 178 `#[qsl_attrs::string_edge]` and 27 bare `#[string_edge]`; head has 178 `#[ix_trace_rs::string_edge]` and 27 bare. The fuzz workspace never named the crate.

Detector. `has_string_edge_marker` accepts exactly `string_edge`, `ix_trace_rs::string_edge` and `::ix_trace_rs::string_edge`. Mutants run against `cargo test -p qsl-string-edge`:

- Killed: M1 last-segment match (the coder's), M2 reject `::ix_trace_rs::` (the coder's), M3 accept deeper paths ending in `string_edge` (the coder's), M5 accept `::string_edge`, M6 accept any crate before `string_edge`, M7 drop the bare form, M8 ignore the marker on `impl` methods, M9 misspell the crate constant, M10 accept only the rooted two-segment form.
- Survived: M4, which drops the `PathArguments::None` check. It is equivalent (FND-001).

Probes: syn rejects `#[ix_trace_rs::string_edge::<u8>]` as a parse error. `#[r#string_edge]` and `#[cfg_attr(all(), ix_trace_rs::string_edge)]` are reported, the conservative direction. A marked nested `fn` is silent.

Same scan output. `cargo run -p xtask -- string-edge` gives identical output on main and on head ("no unmarked, unlisted string comparison or string match found", exit 0).

Manifests. All 11 rewired manifests use `ix-trace-rs = { git = "https://github.com/agent-ix/ix-trace-rs", branch = "main" }`, the same form as the existing entries. Each now-redundant `[dev-dependencies]` copy was removed, and normal dependencies serve tests too. qsl-source keeps the optional normal dependency under `quire-extraction` (`dep:ix-trace-rs`) and its unconditional dev-dependency. The crate is `#![cfg(feature = "quire-extraction")]`. Cargo.lock changes ix-trace-rs 0.1.0 (83c970b) to 0.1.2 (e29cc07, no dependencies), removes the `qsl-attrs` entry and its dependents' edges, and removes trybuild's chain (`glob`, `serde_spanned`, `target-tuple`, `termcolor`, `toml`, `toml_datetime`, `toml_parser`, `toml_writer`, `trybuild`, `winnow`). No other version moves.

Gates at head, all passing: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -D warnings` with default and all features; `xtask string-edge`, `route-lint`, `checked-input`; `arch-lint canonical-encoder`, `qualified-core`, `api-surface --qsl-only`; `cargo deny check bans` ("bans ok"); `cargo test -p qsl-string-edge` (42 passed, 1 ignored as on main, the network install test), `-p xtask` (201 + 16), `-p arch-lint` (83 + 9); root `--test it string_edge_marker family_outcome_layering` with default and all features (8 and 9 passed). `cargo deny check licenses` fails with the same 262 rejections on main. `deny.toml` declares no licence policy and the target is not a gate.

xtask flake. `a_declared_empty_library_gaining_an_ignored_test_fails_until_restored` passed six times alone and the full xtask suite passed five times on main, and it passed at head. The test and the code it exercises are unchanged by this PR, so any flake predates it. No failure was reproduced here.

No compat layer. There is no shim crate, re-export, alias or fallback path in the diff.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| `has_string_edge_marker` and its two call sites | qsl-string-edge/src/detector.rs:72-99,296-313 | examined |
| detector tests for AC-1/AC-8/AC-12/AC-15 | qsl-string-edge/src/detector.rs:526-596 | examined |
| marker imports in the scanner | qsl-string-edge/src/main.rs:14, qsl-string-edge/src/source.rs:8 | examined |
| 11 manifests and the root workspace member list | Cargo.toml, */Cargo.toml | examined |
| Cargo.lock diff | Cargo.lock | examined |
| xtask adapter and `UNSCANNED_MEMBERS` | xtask/src/string_edge.rs | examined |
| `QSL_CRATES` 12 to 11 | xtask/src/typestate_scan.rs:701 | examined |
| `WORKSPACE_EMPTY` and test fixtures | xtask/src/test_suites.rs:184-230,1577-1730,1966,2363 | examined |
| arch-lint fixtures | tools/arch-lint/qualified_core.rs:909-962,1156; tools/arch-lint/api_surface.rs:1253 | examined |
| layering test allow-lists | tests/it/family_outcome_layering.rs | examined |
| marker-site swaps (about 100 files) | src/**, qsl-*/src/**, xtask/src/** | examined |
| README, scanner README, migration recipe | README.md, qsl-string-edge/README.md, docs/family-migration-recipe.md | examined |
| FR-064 "The marker attribute" | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:59-86 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `matches!(segment.arguments, syn::PathArguments::None)` condition in `has_string_edge_marker` is unreachable. syn parses an attribute path mod-style and rejects generic arguments (`#[ix_trace_rs::string_edge::<u8>]` is "expected path segment after `::`"), so no parsed attribute has arguments. Mutant M4, which drops the check, survives every test. Either remove the condition or keep it with a comment that it is defensive. | qsl-string-edge/src/detector.rs:86-92 |
| FND-002 | low | The test_suites fixtures put `qsl-walk-grow` where `qsl-attrs` was but keep the old wording. `DECLARES_EMPTY` and `DECLARES_NO_ANALYZE_DOC` still give `reason: "proc-macro crate"` for `qsl-walk-grow/src/lib.rs`, which is not a proc-macro crate, and the section constant is still named `ATTRS_UNIT_SECTION`. The tests still check what they did (a declared-empty library unit suite plus its doc-test suite, with ids now in path order), but a reader is misled about the sample. | xtask/src/test_suites.rs:1605,1706,1909 |
| FND-003 | low | Moving `ix-trace-rs` out of `[dev-dependencies]` leaves an empty `[dev-dependencies]` table in qsl-cst and qsl-foundation. | qsl-cst/Cargo.toml:24, qsl-foundation/Cargo.toml:19 |

## Verdict

Mergeable on the code-review axis once the test gaps in SR-13761 are fixed; the three findings above are lows. The migration is complete and has no compat layer. The detector accepts exactly the three FR-064 paths, which the coder's mutants and the reviewer's own confirm. Scan output is the same on main and head. The manifests are consistent, the lock diff only bumps ix-trace-rs and removes trybuild's chain, and every related gate passes.

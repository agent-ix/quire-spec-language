---
id: SR-16276
title: "Code review of quire-spec-language PR #1382: FR-403 S4 repoint of SuppliedRun, render_identity, state-model prefix, modulo and Value-family causes (IR-588)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@571dfa7005fd8d75134224e48fdb8f58291a16cf; 45 files vs origin/main (merge base = origin/main); qsl-eval/src/{lib.rs,value/expression/{causes,closure,evaluate,mod}.rs}, qsl-semantics/src/{family/mod.rs,model/refusal.rs,model/normalize.rs,value/{definition,mod,supplied}.rs}, qsl-replay/src/{lib.rs,spine/*,execute/*,request.rs,supplied_limits.rs}, src/command{,/wire}.rs, manifests, Cargo.lock, tests; stacked on quire-semantic-value#62 (e331dd0a) and quire-eval#4 (012eca45), reviewed only as dependencies; rust-review lane folded in"
review_set: subset
date: '2026-10-23'
---
# Code review of quire-spec-language PR #1382

## Summary

Ticket: IR-588 (S4, FR-403 "Which `qsl-semantics` code the evaluator
needs"). Rust-review lane folded in. The PR deletes QSL's copies of
`SuppliedRun`/`SuppliedLimits` (`value::supplied`), `render_identity`, the
`"state-model"` literal, `value::modulo`, and qsl-eval's `PreconditionFailure`,
`StateModelUndefined`, `ProtocolReferenceUnsupported`, `ReferenceUnresolved`
and `identity_string`, and repoints every user at SV / quire-eval. It drops the
`qsl_replay::SuppliedLimits` re-export.

Examined and clean:

- **Moved items have one definition.** SV's `catalog::render_identity` and
  `STATE_MODEL_CATALOG_PREFIX`, SV's `supplied_stop::{SuppliedRun,
  SuppliedLimits}`, and quire-eval's `modulo` and `cause::{PreconditionFailure,
  ProtocolReferenceUnsupported, ReferenceUnresolved, StateModelUndefined}` carry
  the same bodies, catalog codes, field sets and prefix as the deleted QSL
  copies. A workspace grep finds no `pub use`, `pub type` or wrapper of any
  moved item at a QSL path. The one leftover duplicate is FND-001.
- **Deleted tests moved, not lost.** The supplied-run test (TC-906,
  FR-277-AC-5) lives in SV `supplied_stop.rs`. The `identity_string` hex test
  lives in SV `catalog.rs`. FR-090-AC-12 and FR-277-AC-5 keep QSL-side tests
  (`model_reference_queries.rs`, `execute/tests/supplied.rs`,
  `supplied_phases.rs`).
- **Blocked items verified.** `AdmittedIntegerDivision`
  (qsl-semantics/src/value/definition.rs:745) and `DeclarationKey`
  (qsl-semantics/src/model/key.rs:99) are still QSL types. quire-eval#4's
  `quire-checked` is the empty skeleton (`pub mod construct {}`). So `divide`
  and `NavigationRefusal` cannot move yet, as the PR body says.
- **Git deps are temporary, not vendoring.** Cargo.toml points SV and
  quire-eval at `ir-588-s4-sv` / `ir-588-s4-eval`. Cargo.lock pins exactly
  those PR heads (e331dd0a, 012eca45). No file is copied in. The PR body
  states the repoint to `main` after the stack lands.
- **`test-support` stays dev-only.** SV's `test-support` (the reservation
  permit) is turned on only by qsl-eval's `[dev-dependencies]` entry.
- **Ran at this head:** `cargo test -p qsl-eval --doc` (13 passed, including
  the 3 new `compile_fail`), `cargo test -p qsl-eval --test it source_call`
  (10 passed), `cargo test -p qsl-semantics --features test-support --test it
  model_population::` (72 passed), and `cargo fmt --all --check` (clean).
  Clippy and the full gate were not run.

## Verdict

**Not mergeable yet.** FND-001 leaves a second copy of the identity rendering
that FR-403 moves to SV. FND-002 means the PR's breakage list names the wrong
consumer: quire-driver, which follows QSL `main`, breaks at its next lock
update. There is also a merge-order condition, not a defect: merge SV#62 and
quire-eval#4 first, then repoint both workspace deps to `branch = "main"` and
refresh Cargo.lock before this PR merges. The repoint itself is mechanical and
correct.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `ModelRefusal::absent_key` still has its own inline copy of the identity rendering (`std::str::from_utf8`, else `format!("identity bytes 0x{} (not UTF-8)", crate::model::key::hex(&key))`). That is byte for byte what `render_identity` does, and FR-403 says its qsl-semantics population-lookup callers "call it there" (SV's `catalog`). So the rendering still has two definitions, and the two can drift. Fix: `let display = render_identity(&key);` from `quire_semantic_value::catalog`. | qsl-semantics/src/model/normalize.rs:165-176 |
| FND-002 | medium | The PR body's public-API changelog says "CG imports `SuppliedLimits` from SV instead of `qsl_replay`". But CG (quire-contract-codegen main, and its open PRs #550/#552/#553) names no `SuppliedLimits` at all. The consumer that breaks is quire-driver, whose `qsl-replay` and `qsl-semantics` deps follow `branch = "main"`. On its main it imports the deleted `qsl_semantics::value::supplied::SuppliedLimits` in src/limits.rs:97, quire-cli/src/cli.rs:22 and quire-cli/tests/binary_process.rs:23. Fix: correct the breakage list to name quire-driver, and land the paired driver adaptation (SV dep plus `quire_semantic_value::supplied_stop::SuppliedLimits`) with this PR. | PR #1382 body "Public-API changelog" |
| FND-003 | low | A relocation-history comment is now false. It says `PreconditionFailure` "moved to `value::expression::causes` ... narrowed to `pub(crate)`", but the type is now `quire_eval::PreconditionFailure` (pub) and causes.rs no longer has it. Delete the history paragraph rather than rewriting it: where an item used to live is not a feature. | qsl-semantics/src/value/mod.rs:198-203 |
| FND-004 | low | The three new `compile_fail` doctests (`value::supplied::SuppliedRun`, `model::refusal::render_identity`, `value::modulo`) have no error code. Their neighbours use `compile_fail,E0432`. Without the code, a doctest passes on any compile error, such as a typo in the snippet, so it can stop checking that the path is gone without anyone noticing. Fix: `compile_fail,E0432`. | qsl-eval/src/lib.rs:69-79 |
| FND-005 | low | Import hygiene in the repointed files. A second `use quire_semantic_value::supplied_stop::...` line sits beside the existing one instead of being merged into it (qsl-eval/src/value/expression/mod.rs:30-33, qsl-replay/src/execute.rs:41-42, execute/argument.rs:45-46, execute/composite_parity.rs:23-24, execute/value_parity.rs:20-21, execute/tests/composite_parity.rs:25-26, execute/tests/supplied.rs:17-18, qsl-eval/tests/it/supplied_phases.rs:31-38). In six test files the new `use quire_semantic_value` is placed above `use std::` (qsl-replay/tests/{call_replay_context,call_replay_context_root_only,call_requirements_facade,call_requirements_root_only}.rs, tests/it/{call_replay_context_items,call_requirements_items}.rs). | see summary |

## New findings (disposition pass 2)

Round 2 was reviewed at 93c919d55ce61f89270ca57d494df3d34de62b9e. Since b5e0c0d03 the branch has two merges of origin/main (d977308e, 93c919d5). They move the lock to quire-eval 5317524d, quire-exact ab590159 and quire-semantic-value 0814bb76, and repoint two new `crate::SuppliedLimits` users that came in from main. The PR's own hunks are otherwise unchanged. Ran at this head, with its own lock: `cargo check --locked -p qsl-replay` and `cargo check --locked --workspace --all-targets`. Both fail (FND-006). Ran with the lock changed locally to SV 472e609f (never pushed): `cargo check` and `cargo clippy -D warnings` (workspace, all targets) are clean. `cargo test` passes for qsl-eval (119 unit, 704 it, 13 doc), qsl-replay (802 lib plus all integration suites), qsl-semantics `model_population::` and `integer_division` with `--features test-support` (84), and the root `call_replay_context_items`/`call_requirements_items` (3). `cargo fmt --check` is clean. With SV 0f922ad1 the workspace check is also clean.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | The head does not compile. Cargo.lock pins quire-semantic-value at 0814bb764e8cc84acc5bc2a47806bf354b2ca2df (STD-326, SV#65). That commit adds `PropertyForm::Refinement { liveness }`, so `form_str`'s match in qsl-replay is not exhaustive: `error[E0004]: non-exhaustive patterns: PropertyForm::Refinement { .. } not covered` at qsl-replay/src/outcome.rs:290, for both the lib and lib-test targets. Every crate that depends on qsl-replay is then blocked. The PR commit message says "refresh quire-semantic-value to main", but the Refinement adaptation is a separate follow-up (QSL-1029). Fix: pin SV to a revision before 0814bb76 that has `supplied_stop`. Use `cargo update -p quire-semantic-value --precise 0f922ad161b48afa68ee924088c6ce8cee887e69`, which matches QSL#1384's pin. 472e609f also works. Both were verified locally. | Cargo.lock:1691; qsl-replay/src/outcome.rs:290 |

## Dispositions

Round 1 was reviewed at b5e0c0d030b0e7759d1eb6910f4395a7ba581f75. The fix commit is 31f5d6c09. b5e0c0d03 moves both git deps back to `branch = "main"` (SV 472e609f, quire-eval a92145c3), with one lock entry each, which clears the merge-order condition. Ran: qsl-semantics `model_population::` with `--features test-support` (72 passed), qsl-eval doctests (13 passed), `cargo fmt --check` (clean).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed 31f5d6c09 | `absent_key` now uses `let display = render_identity(&key);`, imported from `quire_semantic_value::catalog`. |
| FND-002 | still-open | The PR body now names quire-driver instead of CG. It lists three sites but misses the fourth: `src/execute.rs:231` (`supplied: qsl_semantics::value::supplied::SuppliedLimits::default()`) on quire-driver main. No quire-driver PR adapting the import exists yet, and its deps follow QSL `main`, so merging this PR breaks quire-driver at its next lock update. Still missing: list all four sites, and open the paired quire-driver PR to land with this one. |
| FND-003 | fixed 31f5d6c09 | The relocation-history paragraph about `PreconditionFailure` has been deleted from qsl-semantics/src/value/mod.rs. |
| FND-004 | fixed 31f5d6c09 | All three doctests are now `compile_fail,E0432`, and they pass. |
| FND-005 | fixed 31f5d6c09 | No changed file still has two `use quire_semantic_value::supplied_stop` lines, and the six test files now import SV after `std`. |
| FND-002 | deferred | Round 2: the PR body now lists all four quire-driver sites (src/limits.rs:97, src/execute.rs:231, quire-cli/src/cli.rs:22, quire-cli/tests/binary_process.rs:23), and they are still on quire-driver main. The paired change is quire-driver#103 (draft, 13dfc9de), owned by titan:kani:plan under IR-977/IR-588. It cannot build yet, because its lock pins SV 1a38c06f, which has no `supplied_stop` (SR-14838). quire-driver main locks QSL at 485ccaa3, so its locked build keeps working until its next QSL lock refresh, which then fails on those four imports. This is a cross-repo merge-order item, not a defect in this diff. |
| FND-006 | fixed a56b61321 | Round 3, reviewed at f25b2f55407950f73555eb9ea1622f5aba67e1b0, which is up to date with origin/main (dc267af55 merged). Cargo.lock now pins quire-semantic-value at 0f922ad161b48afa68ee924088c6ce8cee887e69, the same as QSL#1384. quire-eval stays at 5317524d and quire-exact at ab590159. At this head: `cargo check --locked --workspace --all-targets` and `cargo clippy --locked --workspace --all-targets -D warnings` are clean, and `cargo fmt --check` is clean. Tests pass: qsl-eval (119 unit, 704 it, 13 doc), qsl-replay (1009 across all targets), qsl-semantics `model_population::` and `integer_division` with test-support (84), and the root call_*_items tests (3). |

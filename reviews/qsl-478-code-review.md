---
id: SR-1231
title: "Code review of quire-spec-language PR #608: core object closure moves to quire-semantic-value"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@990e58a9e2bb3f54a0ea044cf20f412771796125; PR #608 diff against origin/main: quire-semantic-value/src/{object_closure,lib}.rs, qsl-semantics/src/model/{object_environment,population}.rs, qsl-semantics/src/model/observation/document.rs, qsl-eval/src/value/expression/{evaluate,family,mod}.rs, qsl-replay/src/spine/clause.rs, quire-exact/src/reference.rs, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, and the test modules the API change touched"
review_set: subset
---
# Code review of quire-spec-language PR #608

## Summary

Ticket: QSL-478 (slice H2 of plan v2, QSL-358 slice 4). The closure logic
moves from `qsl-semantics/src/model/object_environment.rs` to
`quire-semantic-value/src/object_closure.rs`. The move is line for line:
`new`, `find`, `contains`, `attribute`, `check_closed` and `present` are the
same code under the new names. The only edits are `std` imports becoming
`alloc` imports and `UniverseId` imported by name. `ObjectEnvironment` keeps
only `new(ObjectClosure)`, `objects()` and the FR-089 population methods.

Checks:
- **Duplication:** none. The old closure code is deleted, not kept beside
  the new copy.
- **Re-export or forwarding:** none. There is no `pub use` of `ObjectClosure`
  outside SV. `ObjectEnvironment` has no `find`, `contains` or `attribute`
  wrapper, and every caller goes through `.objects()` (qsl-eval
  evaluate.rs:1331 and :1829, qsl-replay spine/clause.rs:1241, document.rs:1523).
- **Rust lane (rust-review):** SV still builds as `no_std`.
  `cargo build -p quire-semantic-value --no-default-features --target
  thumbv7em-none-eabi` passed. It uses `alloc` collections and `thiserror`
  without std. There is no new panic, `unsafe` or integer conversion.
  `ObjectEnvironment::new` is now infallible, because the fallible part moved
  into `ObjectClosure::new`. The one wildcard arm is FND-002.
- **Naming:** `ObjectClosure` in SV, with `ObjectEnvironment` kept for the
  model type, is accepted by team-leader ruling and not flagged.
- **Spec:** the ADR-011 §6.2 rows and the X-11 row match the code. ADR-016
  still cites line numbers this PR makes stale (FND-001, per the
  team-leader ruling).
- **Gates at this head (worktree-local target):** all passed.
  - `cargo test -p qsl-semantics --tests`: 420 passed, 1 ignored, then 250 passed.
  - `cargo test -p qsl-eval --test it`: 260 passed.
  - `cargo test -p qsl-replay --lib spine::clause`: 146 passed.
  - `cargo clippy -p quire-semantic-value -p qsl-semantics --all-targets -D warnings`: clean.

## Verdict

Changes requested: three low findings, all small. There is no correctness
defect in the move.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | ADR-016 cites `ObjectEnvironment` as `model/object_environment.rs:88` and `PopulationConflict` as `:156`. This PR rewrites that file, so both line numbers are now stale. Fix: name the symbols instead, `qsl_semantics::model::object_environment::ObjectEnvironment` and `ObjectEnvironment::with_population` / `PopulationConflict`, with no line numbers. | spec/decisions/ADR-016-state-model-finite-execution-mapping.md:127-129 |
| FND-002 | low | `map_environment_refusal` matches the SV enum `ObjectClosureCause` with a `_ =>` arm that maps every other cause to an internal fault. The enum is now a cross-crate type in a shared leaf (RT and CG depend on SV), so a new cause added in SV would silently become a fault here instead of a compile error. Fix: list `DuplicateObject`, `UnknownObjectType` and `Attribute(_)` explicitly. | qsl-semantics/src/model/observation/document.rs:1473-1484 |
| FND-003 | low | Two test doc comments still name the methods this PR moved. `ObjectEnvironment::find` is now `ObjectClosure::find`, and `ObjectEnvironment::new` "as a tolerated dangling target" is now `ObjectClosure::new`. Fix: rename both references. | qsl-replay/src/spine/clause/tests.rs:911; qsl-semantics/tests/it/state_clauses.rs:4136 |

## Dispositions

Round 1, reviewed at 62451415f69bea06dbba4d7177527ce68379523b (fix commit 62451415, checked against its own diff; the branch was rebased onto main before it).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 62451415: ADR-016:127-129 now reads "`ObjectEnvironment` records the `PopulationId` → `PopulationBinding` correspondence, and `ObjectEnvironment::with_population` refuses a second, unequal record with `PopulationConflict`." No line numbers. |
| FND-002 | fixed | 62451415: `map_environment_refusal` lists `ObjectClosureCause::DuplicateObject \| UnknownObjectType \| Attribute(_)` explicitly, with no `_` arm. |
| FND-003 | fixed | 62451415: clause/tests.rs:911 names `ObjectClosure::find`, and state_clauses.rs:4136 names `ObjectClosure::new`. |

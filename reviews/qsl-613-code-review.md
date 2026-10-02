---
id: SR-1211
title: "Code review of quire-spec-language PR #596: quire-walk toolkit and qsl-walk-grow"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4aacd77e4364a077ace57655773e0de8025b70d3; PR #596 diff against origin/main: quire-walk/**, qsl-walk-grow/**, fuzz/**, qsl-bench/{Cargo.toml,src/lib.rs,src/deep_input.rs}, tools/arch-lint/{graph,metadata}.rs, tests/it/quire_walk_leaf.rs, xtask/src/string_edge.rs, Cargo.toml, Makefile, TC-898/899/903 status"
review_set: subset
---
# Code review of quire-spec-language PR #596

## Summary

Ticket: QSL-613. The PR adds two crates.
- `quire-walk` is `#![no_std]`, has no dependencies and no features, and
  forbids unsafe code. It provides `Walk`, `walk`, `Children`, `Arena`, `Id`
  and `Arena::bottom_up`.
- `qsl-walk-grow` holds only `maybe_grow`, which wraps `stacker` and is a plain
  call under `cfg(kani)`.

It also adds a deep-input fuzz target backed by `qsl_bench::deep_input`, adds
`quire-walk` to arch-lint's `SHARED_LEAVES`, and narrows CG's FB-05 exception
to `qsl-replay`.

Judged on the merits:
- **Public API.** It is small and documented: two traits/structs for the
  walk and two for the arena. The crate doc carries a working doctest. `walk`
  is a plain loop over a `Vec` with one reused `pending` buffer, and it has no
  panic, no index and no arithmetic. `Arena` and `bottom_up` have none either.
  A stop discards pending children, as documented. The hand-written `Id`
  impls avoid bounds on `T`. One soundness gap, in `Id`, is FND-001.
- **arch-lint FB-05 narrowing.** Correct, and consistent with FR-059's FB-05
  text ("CG's normal dependency on the QSL layer-6 `replay` facade"). CG's
  `main` Cargo.toml depends only on `qsl-replay` (checked through `gh api`),
  so the narrowing turns nothing red. `tc_arch_lint_direction_004` and
  `tc_arch_lint_metadata_009` test both sides.
- **Copying `Cargo.lock` into `fuzz/`.** This is not a copy that we should not
  have. It is the same repository's lockfile, written at run time,
  gitignored, and never committed, so the fuzz workspace resolves the
  versions CI builds. That is not vendoring. The fuzz crate needs its own
  workspace, because a `no_main` libFuzzer binary would break `cargo
  build/test --workspace`. The comment in `fuzz/Cargo.toml` that explains
  why is stale, though (FND-002).
- **qsl-bench depending on qsl-replay.** Fine. qsl-bench is a bench and
  tooling crate outside the qualified core, and layer 6 sits above every
  layer it already uses. `deep_input` needs `spine::compile` to drive S1 to
  S4.
- **Fuzz depth caps left at their defaults.** Correct today: the code
  comment explains that raising the forms cap sends deep `Option` types into
  S3 walks that still recurse natively. The spec status should say so
  (gap analysis, SR-1212).
- Rust lane (rust-review): `unsafe_code = "forbid"` and `clippy::all =
  "deny"` on both crates. `missing_docs` is satisfied.
- Gates run at this head with a worktree-local target dir:
  - `cargo test -p quire-walk -p qsl-walk-grow -p arch-lint -p qsl-bench`
    passed, and so did the root `quire_walk_leaf` test.
  - `cargo build --locked -p quire-walk --target thumbv7em-none-eabi` passed.
  - `cargo clippy -D warnings` on the new crates and on qsl-bench was clean.
  - I did not re-run Kani.

Rulings applied and not raised: the `quire-walk-no-std` CI wiring and the
AC-1/TC-898 target triple.

## Verdict

Changes requested: one medium finding and one low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `Id<T>` carries no arena identity, so an id from one `Arena<T>` is accepted by another. The guarantees in the docs then fail: "a node built from the ids its arena has already returned names only nodes stored before it", and in `bottom_up`, "a child's result is at its `Id::index` in that slice, which always holds it". A node holding a foreign id with a larger index makes `done[id.index()]` in the caller's closure panic. A foreign id with a smaller index silently reads an unrelated node's result. IR, CG and RT will build on this, so either brand the arena (an invariant-lifetime or per-arena token), or hand `compute` a results view with `get(Id<T>) -> Option<&R>` and state the same-arena precondition in the docs. | quire-walk/src/arena.rs:10-26; quire-walk/src/arena.rs:132-147 |
| FND-002 | low | `fuzz/Cargo.toml` says the crate is its own workspace because "cargo-fuzz builds on nightly with sanitizer flags". The Makefile runs it on the pinned stable toolchain with `-s none`. The real reason is that a `no_main` libFuzzer binary cannot sit in the main workspace's `cargo build/test --workspace`. Fix the comment. | fuzz/Cargo.toml:17-18; Makefile:214-217 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | `Arena::bottom_up` returns a plain `Vec<R>`, documented as holding "each node's result at its [`Id::index`]". After the loop, a caller that looks up a result by id goes through `Id::index` with no arena check. A foreign id with a larger index then panics on `[ ]`, and one with a smaller index reads an unrelated node's result: the same hole FND-001 closed inside `compute`. Return a checked owned view, for example an owned `Results` with `get(Id<T>) -> Option<&R>` and `into_vec`, so a foreign id always reads `None` here too. | quire-walk/src/arena.rs:186-194 |

## Dispositions

Round 1, reviewed at `4d6b74ea86d637dff8177014dfbd2ef2c62131d7` (rebased; fix commits `d0692cc6` and `4d6b74ea`).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4d6b74ea86d637dff8177014dfbd2ef2c62131d7 |
| FND-002 | fixed | d0692cc650e02e7da5d938e61fe35f287f7c0617 |

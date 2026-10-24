---
id: SR-16637
title: "Code and Rust review of QSL-986 state-space reduction test and tag gaps"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1432 (branch qsl986-state-space-gaps, head 4c4cfec5b); qsl-semantics/tests/it/model_population.rs, qsl-replay/src/spine/clause/tests/receiver_frames.rs, qsl-package/src/emit/tests/receiver_frames.rs, qsl-analyze/tests/it/quotient.rs, qsl-analyze/tests/it/reduction_world.rs, qsl-analyze/tests/it/reduction.rs, qsl-analyze/tests/it/partial_order.rs"
review_set: subset
---

## Summary

Ticket: QSL-986. This is the review pass (round 0) at the frozen head
4c4cfec5b, diffed against the origin/main merge base. The method is
code-review with the rust-review lane folded in. gap-analysis is a separate
artifact (SR-16638). spec-review was skipped because no `spec/**` or `plan/**`
file changed.

The diff is tests only, 7 files. It adds five tests, one per AC. It also adds
test helpers: `World::settle_clause`, and `replay_unit` split into
`unit_package_id` and `replay_unit_result`. In qsl-replay it adds
`run_selected`. Six existing test-crate helpers became `pub(crate)`:
`reduced`, `below_two`, `count`, `infinite` and `constraint`, plus the two
new reduction_world helpers. No production code changed.

Gates, run in an isolated worktree with its own target dir, one cargo job at
a time:

- `cargo test -p qsl-analyze --test it -- quotient reduction partial_order`:
  65 passed.
- `cargo test -p qsl-semantics --features test-support --test it --
  model_population`: 73 passed.
- `cargo test -p qsl-replay --lib -- receiver_frames`: 5 passed.
- `cargo test -p qsl-package --lib -- receiver_frames`: 10 passed.
- `cargo fmt --all -- --check` is clean.
- `cargo clippy --locked -p qsl-analyze -p qsl-replay -p qsl-package
  --all-targets -- -D warnings` is clean.
- `cargo clippy --locked -p qsl-semantics --features test-support
  --all-targets -- -D warnings` is clean.
- Known env noise: local cargo-deny 0.20.2 rejects `--config`, so
  cargo-deny was not run. It is not relevant to a tests-only diff.

Mutations were run in the review worktree and reverted after each run:

- M1, `population.rs`: the `is_receiver &&` guard on `receiver_modifies` was
  dropped. Killed by `decide_frame_scopes_a_modifies_entry_to_the_receiver`
  (FR-154-AC-2).
- M2, `population.rs`: `deletes_receiver` granted any object. Killed by
  `a_deletes_self_frame_grants_the_receiver_only_and_differs_from_deletes_type`
  (FR-154-AC-5).
- M3, `settle.rs`: a refuted record dropped its counterexample. Killed by
  `a_symmetry_refutation_settles_as_the_unreduced_refutation_does`
  (FR-160-AC-4).
- M4, `check.rs`: the state constraint was ignored by the checker
  (`if let Some(name) = None::<&str>`). It **survives**
  `every_counterexample_is_concrete_and_replays_as_an_unreduced_one_does`
  (FR-161-AC-4). See FND-001.
- M5, `observation/frame.rs`: the clause-run admission binds no receiver
  (`receiver: None`). It **survives**
  `the_frame_run_and_the_clause_admission_apply_the_receiver_scope`
  (FR-154-AC-3). See FND-003.

## Verdict

CONDITIONAL. The five tests are real and use production paths throughout:
`decide_frame`, emit then admit then `read_domain_package`, the clause-run
spine, `settle_model_check` with an FR-128 replay, and the checker. Their
assertions name values, not just shapes, and three of the four targeted
mutations were killed.

The `pub(crate)` widenings are inside test crates, which are a test binary or
a `#[cfg(test)]` module, so they add no API surface. Putting FR-154-AC-5 in
qsl-package is justified: it needs `emit` (qsl-package) and `decide_frame`
(qsl-semantics), and qsl-package already dev-depends on qsl-semantics
`test-support`. The reverse direction would be a cycle.

Leaving the existing tests untagged is the right call. Those tests use
`model.A.x` fixtures and the FR-115-AC-11 F1–F3 vectors, not the AC's
c1/c2 `Counter` vectors, and they assert no clause-run admission. Tagging
them FR-154-AC-2/3/5 would inflate coverage. The ticket's own fallback
("add the AC's vectors where the fixtures differ") is what the coder did.

The weak spots are these:

- The constrained-counterexample assertion cannot tell a constrained run from
  an unconstrained one (FND-001).
- The PR description is another ticket's (FND-002).
- AC-3's clause-run receiver binding is not exercised (FND-003).
- Two doc nits (FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-161-AC-4 test: `assert_eq!(constrained, unconstrained)` does not show the constraint was active; M4 (constraint ignored in check.rs) survives, so "FR-158-AC-1's constrained counterexample is emitted unchanged" is not distinguished from an unconstrained run | qsl-analyze/tests/it/quotient.rs:673-715 |
| FND-002 | low | PR #1432 body describes QSL-988 (HP-4 `settle_hyper_form`, FR-173..183, hyper mutations), not this diff; the ACs-covered, not-tagged, checks and mutations sections are all for another PR, so the merge record would be wrong | PR #1432 description |
| FND-003 | low | FR-154-AC-3 test: with the clause-run admission's receiver unbound (M5) the test still passes, because the only vector changes `c2.value`, which is refused with or without a receiver; the receiver scope on the admission path is not shown to grant `c1` | qsl-replay/src/spine/clause/tests/receiver_frames.rs:224-295 |
| FND-004 | low | Doc drift: qsl-package receiver_frames.rs `//!` header still lists only TC-1043 (FR-154-AC-6/7/10) and TC-573 (FR-154-AC-4) and omits the new TC-572 FR-154-AC-5 test; the FR-161-AC-4 test doc does not say why the ticket's serialize/decode round trip was replaced by Eq plus exhaustive destructuring (TemporalCounterexample has no encoded form) | qsl-package/src/emit/tests/receiver_frames.rs:2; qsl-analyze/tests/it/quotient.rs:634-641 |

## Dispositions

Round 1 checks the fix commit f5f5cc826. M3, M4 and M5 were re-run in an
isolated worktree. M4 and M5 now fail their tests, and M3 is still killed.
`cargo test` gives quotient 12, qsl-replay receiver_frames 5 and qsl-package
receiver_frames 10.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | quotient.rs:724-750 adds a run of `always holds(x.v <= 2)` under `Low` in the same world and asserts `ConstraintReached{boundary_states: 12}` over 20 stored states. M4 (constraint ignored) now fails at quotient.rs:738 |
| FND-002 | fixed | the PR #1432 body is now QSL-986's: an AC table for FR-154-AC-2/3/5, FR-160-AC-4 and FR-161-AC-4, and this PR's checks and mutations |
| FND-003 | fixed | receiver_frames.rs:291-314 adds a post that changes only `c1.value`. It succeeds in the Frame run and is admitted under `own`. M5 now fails at receiver_frames.rs:295 |
| FND-004 | fixed | the qsl-package `//!` header names TC-572 (FR-154-AC-5). The quotient.rs docs state why there is no round trip (no encoded form, QSL-1045) |

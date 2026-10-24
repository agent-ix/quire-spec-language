---
id: SR-16625
title: "Code and Rust review of QSL-984 EN-1 and strong-fairness test gaps"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #1429 (branch qsl984-en1-test-gaps, head f909c7201); qsl-analyze/tests/it/liveness.rs, qsl-analyze/tests/it/strong_hint.rs, qsl-analyze/tests/it/strong_fairness.rs, qsl-analyze/tests/it/state_graph.rs, qsl-analyze/tests/it/model_instances.rs"
review_set: subset
---

## Summary

Ticket: QSL-984. This is the review pass (round 0) at the frozen head
f909c7201, diffed against the origin/main merge base. Methods: code-review
with the rust-review lane folded in. gap-analysis is a separate artifact
(SR-16626). spec-review was skipped because no `spec/**` or `plan/**` file
changed.

The diff is tests only: five files under qsl-analyze/tests/it. `STRONG_EACH`
and `mutex_lasso_for` in strong_fairness.rs became `pub(crate)`, and no
production code changed.

Gates were run in an isolated worktree with its own target dir:
`cargo fmt --all -- --check` is clean; `cargo clippy --locked -p qsl-analyze
--all-targets -- -D warnings` is clean; `cargo test --locked -p qsl-analyze
--test it` gives 739 passed. Known env noise: local cargo-deny 0.20.2 rejects
`--config`, so cargo-deny was not run. It is not relevant to a tests-only diff.

Red checks were run in a throwaway worktree, at production code:

- `qsl-eval/src/temporal/engine.rs`: the closed-trace sentinel changed from
  `Tri::False` to `Tri::True` (false-extension removed).
  `a_bounded_always_reads_past_closure_as_false_only_under_event_position_false_extension`
  fails at liveness.rs:891, the evaluator's bounded assertion. The other two
  tagged tests pass.
- `qsl-replay/src/model_check/replay.rs`: the `UnfairLasso` early return
  dropped. `the_strong_whole_mutex_refutation_carries_the_same_hint` fails at
  strong_hint.rs:340, the new FR-133-AC-5 `UnfairLasso` assertion.

Both tagged behaviours are therefore guarded by real assertions.

Rust lane: the `Silent` property component and the `Positions` letter reader
are minimal and correct. The `TemporalLetters::activation` panic is right for
an `on origin` clause. The `usize::try_from(...).expect` is test-only. The
`pub(crate)` widening is the narrowest visibility that the cross-module reuse
needs. No unsafe code, no unbounded loop (`behaviour_values` asserts
`len < 16`), and no shims.

## Verdict

**APPROVED (round 1).** Round 1 at bd0e05f05 fixes FND-001, FND-002 and FND-003. fmt and clippy are clean, and `cargo test -p qsl-analyze --test it` gives 739 passed. The red check makes `ProductTransition::Model` encode as `terminal-stutter`, and the new canonical-bytes `assert_ne!` then fails at liveness.rs:1018.

Round 0: **APPROVED.** All three findings are low. Each one is a redundant or
type-implied assertion, or a harness-fidelity note, and none of them makes a
tag overclaim. The tagged tests assert the behaviour that their ACs name, and
the red checks confirm that they fail when that behaviour is removed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `assert_ne!(stutter, ProductTransition::Model(inc))` follows `assert_eq!(stutter, ProductTransition::TerminalStutter)`. Distinct enum variants are never equal, so the assertion cannot fail, and FR-125-AC-1's "transition identity differs from every `inc` identity" clause is carried by the type, not by a behavioural check. Drop the loop, or compare the written identities (the `ProductTransition` writer emits `terminal-stutter` against each `inc` identity's text). | qsl-analyze/tests/it/liveness.rs:1010-1012 |
| FND-002 | low | `hints.iter().all(granularity != Whole)` is implied by the earlier `hints.len() == 1` and `granularity() == Each` asserts, so it cannot fail on its own. FR-133-AC-5's "no hint for the held `Whole` constraint" is still asserted by those two lines. The extra assertion is redundant and adds no evidence. | qsl-analyze/tests/it/strong_hint.rs:330-336 |
| FND-003 | low | `evaluated()`'s `Positions` reader ignores the atom's `FormulaNodeId` and reads each position through a test closure (`is_at_most_three`, `is_three`, `is_zero`). So the clause's atom text (`c.value <= 3`, `c.value = 3`, `c.value = 0`) is never evaluated, and pairing a clause with the wrong closure would still pass. This is the repo's existing `decide(atom)` harness pattern, and FR-125-AC-2 is about profile semantics, so the tag stands. The doc comment could still say that the atoms are supplied by the test. | qsl-analyze/tests/it/liveness.rs:822-860 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (bd0e05f05): the loop now compares the canonical written bytes (`quire_canonical::to_vec`) of the stutter against each `inc` identity. A red check that makes `Model` encode as `terminal-stutter` fails it at liveness.rs:1018. |
| FND-002 | fixed | Round 1 (bd0e05f05): the redundant `all(granularity != Whole)` assert is removed. `hints.len() == 1` and `granularity() == Each` still assert AC-5's "no hint for the held `Whole` constraint". |
| FND-003 | fixed | Round 1 (bd0e05f05): the `evaluated` doc comment now states that the letter is a test closure, that the atom text is not evaluated (that is FR-107's job), and that the formula structure is what the readings exercise. |

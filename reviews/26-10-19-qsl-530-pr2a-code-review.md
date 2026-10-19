---
id: SR-13322
title: "Code review (Rust lane) of QSL-530 E9 part 2a lasso and prefix replay of a formula counterexample (PR #1044)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@499d0a250; qsl-replay/src/{execute.rs,outcome.rs,model_check/replay.rs}, qsl-analyze/tests/it/{lasso_replay,liveness,main,model_settle}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-523
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
---
# SR-13322: Code review (Rust lane) of QSL-530 E9 part 2a lasso and prefix replay of a formula counterexample (PR #1044)

## Summary

Ticket: QSL-530 (E9), PR 2a. PR quire-spec-language#1044, head 499d0a250, branch qsl-530-pr2, scoped to `git diff origin/main...499d0a250` (7 files, merge base e4052c148; merges cleanly onto origin/main bd29deeac). The rust-review lane is folded into this file. The worktree ~/dev/worktrees/qsl-530-pr2 is checked out on qsl-530-pr4, so the review read the PR head through `git show` and built an export of 499d0a250 in the scratchpad with its own target directory; the worktree was not touched.

Gates run on the export: `cargo fmt --all -- --check` clean; `cargo clippy -p qsl-replay -p qsl-analyze --all-targets -- -D warnings` clean; `cargo test -p qsl-replay -p qsl-analyze` all pass (qsl-analyze `it` 134, the 7 new `lasso_replay` tests included). `make ci` was not run, per the brief. `make arch-lint-api-surface-qsl` fails FR-100-AC-8 on `qsl-replay/src/spine/clause.rs` (`enum ClauseDisposition`) with identical rule results on an export of origin/main bd29deeac, so the failure is pre-existing and not this PR's.

FND-001, FND-002 and FND-003 were reproduced with probe tests in the scratch export only (`qsl-analyze/tests/it/probe_1044.rs` plus a `package_from_text` helper, saved under /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/probes-1044/).

## Verdict

**FAIL (not mergeable as is).** FND-001 is a soundness defect, reproduced: replay settles `reproduced-with-evaluated-witness` on a finite prefix that refutes nothing, for a clause the engine proves.

What is sound: the clause lookup by `clause_node` then the `claim` occurrence, and the fairness-set check against the recompiled clause, all before admission; the deadlock-item identity cases keep their refusals. Loop closure: a model loop needs `entry < steps` and the entry state's digest equal to the last post-state's (the probe of a non-empty open loop refuses `LoopOpen`), and a `terminal-stutter` loop needs `entry == steps` and an expansion with no successor (a stopped expansion is not terminal). The weak-fairness check numbers every transition identity taken or enabled at a loop state, reads enabledness from `ModelSystem` at each loop pre-state, keeps the stutter step outside every constraint, expands `whole` to one set and `each` to one constraint per identity, and maps the failing index back to the authored constraint. Strong constraints refuse `unsupported_construct`/`not-yet-implemented` before any fairness verdict. On a lasso, `evaluate_at` position 0 over `TemporalTrace::Lasso{prefix: entry, loop_len}` reads positions within `0..=steps` only; for `on origin` and an unguarded `on each`, a `false` at position 0 is a real violation, so a lasso that closes, is fair and evaluates false is a real fair counterexample. Under infinite-trace a finite prefix decides only a safety bad prefix (FR-328), so a liveness prefix cannot reproduce. No `unwrap`, no panic, no new `unsafe`; `UnfairLasso` carries its catalog code and S8 stage.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A finite prefix of a bounded-profile clause replays as a closed trace (false-extension) with no check that it covers the formula's horizon from every activation or ends at a terminal state (ADR-018 CX-1). A truncated prefix of a clause that holds therefore reproduces. Probe: `temporal C using tb (event-position false-extension) ... on origin { always[0,2] holds(c.value >= 0) }` over Counter. EN-1 returns `Holds`, yet the payload with an empty prefix at the non-terminal initial state replays to `Reproduced{Truth(false), trace_position: Some(0)}`, which `settle_model_check` would settle `refuted`. Fix: before evaluating, refuse (or settle `ReplayParity`) a bounded-profile prefix whose last state is not terminal and whose length does not reach the horizon from every activation it reports. Add a TC-523 test for it. The spec gap is SR-13323 FND-005. | qsl-replay/src/model_check/replay.rs:639-646 |
| FND-002 | medium | A genuine bad prefix of an infinite-trace safety clause that carries a fairness set never reproduces. `evaluate` (Goal::Clause) returns `MissingFairnessPremise` for any infinite-trace clause with a fairness set (the FR-328/FR-329 rule for supplied traces, which have no enabledness), and `Ok(_) => no_value()` settles it `NoValue`. Probe: `fair weak M::Counter::inc; always holds(c.value < 2)`. EN-1 returns the 2-step prefix; replay settles `Inconclusive(NoValue{proved: Violation, replayed: Incomplete})`, so the item is never `refuted`. A bad prefix refutes whatever the fairness set says, and replay has enabledness from `ModelSystem`. Evaluate the formula over the prefix without the premise rule, for example by `evaluate_at` per activation or a goal that skips the premise. The spec gap is SR-13323 FND-006. | qsl-replay/src/model_check/replay.rs:639-646, 654 |
| FND-003 | medium | An authored clause's `UndefinedEvaluation` counterexample was refused `UnknownTemporalClause` before this PR. It now resolves and goes to the deadlock item's `undefined()`, which checks only contract conjunctions and the `terminal when` predicate and never the clause's atoms. EN-1's own undefined-atom refutation therefore settles `ReplayParity`, which says the engine disagrees with SM-1. Probe: `always holds(c.value >= 0)` with the atom undefined at value 2. EN-1 returns `UndefinedEvaluation` at an `Atom` locus, and replay settles `Inconclusive(Verdicts{proved: Violation, replayed: Success})`. A `Terminal` locus under an authored clause would apply DL-1's `P` to an authored claim, which DL-6 forbids (unreachable today: `compile_package` refuses a `terminal` declaration as `TerminalNotEmitted`). Until FR-128-AC-5 is built, refuse an authored-clause `UndefinedEvaluation` payload `unsupported_construct`/`not-yet-implemented`, as `StrongFairness` does. | qsl-replay/src/model_check/replay.rs:389, 502-504 |
| FND-004 | low | `Reproduced` copies `payload.trace_position()` and never compares it with the evaluator's `Violated{at}`. On a prefix that runs past the first violating position, replay reports the payload's position, not the one it evaluated. Compare the two and settle `ReplayParity` when they differ, or report `at`. | qsl-replay/src/model_check/replay.rs:648-652 |
| FND-005 | low | Latent: replay does not refuse a guarded `on each ... when` clause, which EN-1 refuses (`GuardedActivation`). On a lasso, `evaluate_at` ignores activation, so the formula is read at position 0 even when the guard is false there, and a vacuously true clause could reproduce. On a prefix, `ReplayedLetters::activation` raises an `InternalFault`. Unreachable today, because the checker refuses such a clause over a model (`Temporal(NotEmitted)` in the probe). Add the same `GuardedActivation`-style refusal at `resolve_item`, so it cannot become reachable silently. | qsl-replay/src/model_check/replay.rs:581-590, 631-637 |
| FND-006 | low | Test gaps. The loop-closure test removes the only loop step, so it covers only `entry == steps`. The `entry.digest != last.digest` branch for a non-empty loop is untested (the probe shows it refuses correctly). In `a_liveness_violation_settles_refuted_through_replay`, the payload named `unfair` is an open loop, which exercises `LoopOpen`, not `UnfairLasso`. Add a non-empty open-loop case and rename the variable. | qsl-analyze/tests/it/lasso_replay.rs:197-207, 377 |

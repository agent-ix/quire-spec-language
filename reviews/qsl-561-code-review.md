---
id: SR-12257
title: "Code review (Rust lane) of QSL-561 E15d: counted protocol steps for interval distance (PR #1142)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8b2d0d516c8fab432009c599951894515c3e4591; qsl-eval/src/automaton/{counters.rs,error.rs,mod.rs,translate.rs}, qsl-eval/src/temporal/{engine.rs,letters.rs,mod.rs,outcome.rs,steps.rs}, qsl-eval/src/simulation/{protocol.rs,protocol/identity.rs,subject.rs}, qsl-analyze/src/model_check/check.rs, qsl-replay/src/model_check/replay.rs, qsl-replay/src/spine/{clause.rs,clause/temporal.rs}, qsl-analyze/tests/it/{protocol_counted_steps.rs,fill.rs,main.rs}, qsl-eval/tests/it/{automaton_kernels.rs,property_automata.rs,temporal_trace.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-213
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: reviews
---
# SR-12257: Code review (Rust lane) of QSL-561 E15d (PR #1142)

## Summary

Ticket: QSL-561. PR quire-spec-language#1142, branch qsl-561-impl, head 8b2d0d516. The review covers `git diff origin/main...HEAD` only: 22 files, +1424/-112. The rust-review lane is folded into this file. QSpec is cited by id only.

One focused test run went through `locked-build.sh`: `cargo test -p qsl-eval -p qsl-analyze --test it -- count property_automata`. It covers every new test and the existing counter and property-automaton suites, which guard the model-subject translation. It passed: 46 tests in one crate and 41 in the other, with 0 failures.

## Verdict

**Not mergeable: one major (FND-001).** Medium and low findings are recorded below and are fixed in this PR.

What is sound:

- **Step classification.** `StepKind::counts` counts `Attempt`, `CAttempt`, `Event`, `Send` and `Receive`. It leaves `Fork`, `Join`, `Finish`, `Timeout`, `CEnd`, `Duplicate`, `Lose`, `Fence` and `Memory` uncounted. That is ADR-027 PB-3's list for the kinds that exist. There is no `activate`, `spawn` or `retire` kind yet. The terminal stutter counts in every place it occurs: `distance_after(at, None)`, `advance` with parent `(_, None)`, and `false_extension_rejects`, which steps on `Letter::default()` with the uncounted atom false.
- **Model-subject translation unchanged.** `translate`, `translate_body` and `translate_negation` pass `Counting::Every`, which leaves `uncounted = None` and `exact = false`. `DeadlineCounter::step` and `ExtentCounter::step` were re-derived over `step_counted(.., true)` with `Some(0)` normalised. They agree case by case with the old `remaining` arithmetic. Shifts take the `(true, _)` arms, and `expand` returns the counted transitions alone. In `check.rs` the product key carries distance `min(reach) = 0` for `Counting::Every`. For a model subject `distance == depth` (`counts_step` defaults to `true`), so `reads_position` is unchanged.
- **Counted counters.** A deadline at slack 0 survives uncounted steps and expires on the next counted one. An extent at slack 0 still requires its operand across uncounted steps (`requires`). These match FR-338's vectors: "a false fork before that counted step ... does not reject", and "a false letter followed by an uncounted p-true letter accepts". The exhaustive short-trace kernels test both, against an independent distance oracle.
- **Evaluator windows.** `ahead`, `ahead_last`, `behind` and `behind_far` over `Steps` give PB-3's distance from `i` to `j`. The `.max(p)` and `.min(p)` clamps keep a window from reaching past its own position across shared distances. The false extension and the positions before origin each count one step. `last_within(count >= 0)` always finds `counted[0] = 0`, so `within - 1` cannot underflow.
- **EN-1 product.** Capping the counted distance at `last_read + 1` in the key is exactly enough, because `reads_position` only tells apart distances up to `last_read`. The POR pre-read in `ample` now passes the real transition, so a reading it caches matches the node it creates. POR runs only in `check_reduced` (model, `Counting::Every`), and `check_protocol` sets `chooser: None`, so commutation is unaffected.
- **Replay.** `ReplayedLetters::counts_step` maps position `i` to `prefix[i - 1]` with `checked_sub`, and treats positions past the prefix as counted, as the false extension needs. The AC-3 refutation replays to `Refuted`.
- **Refusals are typed and honest.** `PastIntervalInCountedSteps` is raised in `prepare`, before admission, and maps to `ModelCheckRefusal::Translation`. `UncountedLasso` maps to `unsupported_construct/expression-form`. Neither one hides a wrong answer. Their tracking is FND-002.
- **Panic surface.** The production code adds no `unwrap`, `expect`, indexing or unchecked `as` casts. `usize::try_from(*last).map_or(usize::MAX, ..)` saturates.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | EN-1 over a protocol subject answers `Holds` for a claim that PB-3 says is not met, when an interval obligation is still open on a reachable cycle of uncounted steps. With `translate_counted`, a pending deadline keeps slack while the steps are uncounted. A cycle of uncounted steps (for example `duplicate` then `lose` on a duplicating, lossy channel, which returns to the same state key) therefore revisits the same product key `(state, monitor, distance)`, and the run coalesces it, never rejects, and ends `Holds`. ADR-027 PB-3 says: "A loop with no counted step keeps every distance constant, so an interval that needs a counted step after the loop is entered is not met on that lasso". FR-338's non-progress vector also expects a refutation with a nonempty zero-counted loop. The loop is a fair behaviour whenever no thread can move (for example, a receiver blocked on `Constraint`), and under `scheduling adversarial` it is fair whenever it is reachable. Today the label is not `proved`, because the certificate refuses interval states, but `check_protocol`'s outcome is still a wrong `Holds`, and it would become a proof once QSL-878 lands. No test covers an uncounted cycle. Fix in this PR: record the product edges taken on uncounted steps. Before returning `Holds` for a claim with an `uncounted` atom, look for a cycle in that subgraph through a product state whose monitor has an open interval obligation. If one exists, settle `Undecided` with a typed cause rather than `Holds`. Add a test with a dup/lose channel (or a `scheduling adversarial` fork cycle) that settles that way. | qsl-analyze/src/model_check/check.rs:1549-1558, qsl-analyze/src/model_check/check.rs:1579, qsl-eval/src/automaton/translate.rs:1125-1150 |
| FND-002 | medium | Past operators with an interval over a protocol subject are refused (`PastIntervalInCountedSteps`). FR-213's Behavior and PB-3 require them to read counted distance in the model checker too ("the past operators with an interval (backwards), read distance the same way"), and FR-338 gives their form (`distance-history` buckets). The refusal is typed and sound, but the PR names no ticket that owns the remainder. The PR body says only "FR-338's counted past-window forms are the rule to implement". Fix: implement FR-338's distance-history form here, or record `Remaining work: QSL-878` in the PR body and in the refusal's doc comment, and have QSL-878's scope name counted past windows over a protocol subject. The same applies to the `UncountedLasso` remainder (the PB-3 zero-counted-loop rule in the evaluator). | qsl-eval/src/automaton/translate.rs:543, qsl-eval/src/automaton/error.rs:54-58, qsl-eval/src/temporal/engine.rs:343-345 |
| FND-003 | low | `Steps::read` runs on every evaluation, for every subject. It calls `counts_step` for every represented position and allocates a `Vec<i128>` of `len` entries before it knows any step is uncounted. It charges nothing to the meter. For a model subject every call returns `true`, so the walk and the allocation are wasted. For a bounded `on origin` clause, which reads only through its horizon, the walk now runs over the whole trace. The tuple bad-prefix bisection (`false_position`) does this on every probe, with `len` up to positions plus the formula's future reach. Fix: scan without allocating until the first uncounted step, or ask the letter source once whether it can report an uncounted step at all (default `false`). | qsl-eval/src/temporal/steps.rs:28-45, qsl-eval/src/temporal/engine.rs:343 |
| FND-004 | low | For a lasso, `Steps::read` queries the steps into positions `1..represented` but not the loop's back edge, the step from the last loop position back to the loop entry. A lasso whose only uncounted step is that back edge reads as "every step counts", so it slips past the `UncountedLasso` refusal and is evaluated in positions. No protocol lasso reaches the evaluator yet (protocol checks are safety-only, and `check.rs` emits `lasso: None`), so this is latent. Fix: for a lasso, also read the back edge, for example through a `TemporalLetters` query for the loop-closing step, or refuse when it cannot be read. | qsl-eval/src/temporal/steps.rs:36-45, qsl-replay/src/model_check/replay.rs:1110-1116 |

## Dispositions

---
id: SR-1493002
title: "code-review and rust-review of QSL-1493 finite-window evaluation (quire-spec-language PR 2165)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; qsl-eval/src/temporal/{mod,observed,timed}.rs; qsl-replay/src/spine.rs; qsl-replay/src/spine/clause.rs; qsl-replay/src/spine/clause/temporal.rs; qsl-replay/src/spine/clause/tests/temporal.rs; qsl-semantics/src/check/temporal_clause.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
---

# Code review of QSL-1493 finite-window evaluation (quire-spec-language PR 2165)

## Summary

Ticket: QSL-1493. Head 7826aa7f2a73, base 2556b4b59. Rust lane (rust-review) folded into this file.
Gates run (CARGO_BUILD_JOBS=2, CARGO_INCREMENTAL=0, via qsl-slot): workspace check and clippy
`-D warnings` clean, default and `--no-default-features`; qsl-eval tests green. No `#[allow]` added
(the one in timed.rs is pre-existing). No `unwrap`/`expect` on added production paths; one
production `unreachable!` (FND-009). No pins added.

FR-501-AC-1..12 are exercised by 12 tests on a single fixture family. AC-13..17 are out of this PR
by declaration. The evaluator core (reach, establishment, eligible instants, pending/decisive
witness, empty windows) is the generic timed-prefix evaluator driven with a closed-window flag and is
sound for the AC-1..6, 10, 12 shapes. Three FR-501 verdict classes (StraddlesBound,
ConflictingValuation, MissingHistory) are not evaluator semantics: they are post-hoc overrides in the
replay layer that match specific clause shapes (FND-001, FND-002, FND-003).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | MissingHistory is applied to every Violated verdict of any clause containing a past operator when the first stamp is above 0 and the origin is not authoritative. FR-501 says MissingHistory only when the past boundary precedes the trace and the verdict is not already singleton from the history present. `historically[0,4] holds(p)` with p false at the first stamp (a definite false from present history), or `once[0,2]` anchored where the boundary t0-2 lies inside the trace, become Indeterminate. The test covers only the one `once[0,4]` false-false case. Reproduced: a probe test with `on origin { historically[0,4] holds(c.value > 0) }`, stamps 3,4, values 0,1, W 4, C(4), HistoryCutoff returned `Indeterminate { at: 0, reason: MissingHistory }` where the present history already decides false. | qsl-replay/src/spine/clause/temporal.rs:finite_window_verdict | implementation-bug-despite-evidence |
| FND-002 | high | StraddlesBound is produced by `counter_straddles_bound`, which fires only for a root `Implies(Holds, Eventually[timed](Holds))` with the antecedent true at position 0, and always reports `at: 0`. The evaluator ignores cover (Counter only widens the establishment deadline). Any other clause shape (always, nested, a request not at position 0, a different antecedent) over a Counter cover returns a definite verdict although the cover straddles the bound, and a straddle on the matched shape overrides a Violated verdict at another activation. AC-9 and rows 20/21 pass only because they use that one shape. | qsl-replay/src/spine/clause/temporal.rs:counter_straddles_bound,finite_window_verdict | implementation-bug-despite-evidence |
| FND-003 | high | ConflictingValuation is produced by `conflicting_timestamped_valuation`, which matches only a root `Eventually[timed](Holds)` and infers a conflict from two different positions that share a stamp (one reading true, a later one false). FR-501 defines it as two admitted valuations of one atom at one position; TimestampedTraceInput carries one document per position, so a real conflict cannot be expressed and a same-stamp pair of distinct events (legal, ordered by key) is misreported as conflicting. The reported position is the first of the pair by coincidence, and the override ignores outcomes of other activations. | qsl-replay/src/spine/clause/temporal.rs:conflicting_timestamped_valuation | implementation-bug-despite-evidence |
| FND-004 | medium | The ticket asks for the ProfileNotEvaluated refusal for this profile to be removed from qsl-eval/src/temporal/engine.rs. It is untouched (engine.rs:184 still refuses TimestampedEventFiniteWindow in Unrolling::new); the PR adds a parallel `evaluate_finite_window_at` / `evaluate_observed_finite_window` entry instead, so `unrolling()` and `evaluate_observed` still return ProfileNotEvaluated for the profile, and TemporalTraceRefusal::ProfileNotEvaluated stays reachable through those public entries. | qsl-eval/src/temporal/engine.rs:184 | missing-requirement |
| FND-005 | medium | Same-stamp refusal (`same_stamp_order_refusal`) scans every equal-stamp group in the trace whenever the clause contains until/release/since/triggered, regardless of the anchor and of whether the positions are eligible. FR-501 refuses only when both positions participate in the order-sensitive operator. An activation whose window excludes the tied positions is refused, and a clause with the ties outside every window cannot run. | qsl-replay/src/spine/clause/temporal.rs:same_stamp_order_refusal | implementation-bug-despite-evidence |
| FND-006 | medium | The TCV vectors are not read from QSPEC_DIR; the AC-9 test hand-copies a subset. TC-1295 row 21 names TCV-02, TCV-05 and TCV-09..15 as direct checks of within_bound/definitely_before, but TCV-02 (2999), TCV-05 (3002) and most definitely_before pairs are not asserted, and a QSpec change to the vector file would not be seen. | qsl-replay/src/spine/clause/tests/temporal.rs:1490-1520 | correct-requirement-no-evidence |
| FND-007 | medium | Two disagreeing exit mappings: ClauseDisposition::category() maps Pending, PendingWindow and Indeterminate to Category::Inconclusive (trace exit 0, ordinary exit 22) while the new exit_code() says 11 for Indeterminate. No production caller uses exit_code, category_label or temporal_batch_exit_code (only re-exports); the monitor path (qsl-replay/src/outcome.rs:1215) still reads category().trace_exit_code(), which gives 0 for an Indeterminate. FR-285-AC-7 (exit 11 reported by the run) is met only by an opt-in accessor. The QSL-local label and 11 are the right interim values (FR-285 order 30,20,21,22,10,11,0), but nothing owns the quire-semantic-value Category change that retires the local copy, and the interim does not reach the exit path. | qsl-replay/src/spine/clause.rs:category,exit_code,temporal_batch_exit_code | missing-requirement |
| FND-008 | medium | The three-valued (Kleene) evaluation with `Tri::Unknown` for a missing atom is not the FR-501 lift over every consistent valuation: a formula such as `holds(p) or not holds(p)` with p absent evaluates Unknown (Indeterminate) where the possibility lift is true. Also `finite_window_verdict` maps an outcome that is still Pending with watermark and completeness both at or past the deadline to PendingWindow carrying a deadline in the past, where FR-501 requires Indeterminate. | qsl-eval/src/temporal/timed.rs:evaluate_finite_window_at; qsl-replay/src/spine/clause/temporal.rs:finite_window_verdict | implementation-bug-despite-evidence |
| FND-009 | low | Production hygiene: `unreachable!` in TimestampedCover::interval (a public method path via within_bound); `missing.position as usize` casts after a bounds check done elsewhere; `unwrap_or(u64::MAX)` on deadline and position conversions silently saturates a Pending deadline instead of refusing; `let _ = future;` and the unused `future` tuple field in finite_future_reach are dead code; Pending/Indeterminate atoms are reported as "holds-node@N" rather than the atom named in AC-4 (`p`). | qsl-replay/src/spine/clause/temporal.rs; qsl-eval/src/temporal/timed.rs | implementation-bug-despite-evidence |
| FND-010 | medium | Mutation check: changing the establishment test in evaluate_finite_window_at from `watermark >= deadline` to `watermark > deadline` (breaking the inclusive-deadline rule of FR-501-AC-6 and TC-1295 rows 14-16) leaves all 10 timestamped tests green, so the inclusive watermark boundary is not pinned by any value-asserting test. | qsl-eval/src/temporal/timed.rs:evaluate_finite_window_at | correct-requirement-no-evidence |

## Verdict

Not mergeable as is: FND-001, FND-002 and FND-003 are wrong results on spec-stated shapes beyond
the tested ones, in a path whose purpose is definite verdicts. The AC tests pass only for the exact
fixtures.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-011 | low | QSPEC_DIR-gated vector tests pass silently when the variable is unset: qsl-eval prints 'skipped: QSPEC_DIR not set' and returns, the replay test is `if let Some`. A run without the variable shows green without TCV-01..15 (my gate set QSPEC_DIR and they ran). | qsl-eval/tests/it/finite_window.rs:702-707; qsl-replay/src/spine/clause/tests/temporal.rs:1516 | correct-requirement-no-evidence |
| FND-012 | medium | FR-285-AC-7 (on main) says an Indeterminate finite-window run keeps its label and reason in machine output and exits 11. The label and exit are available only as in-process accessors (label_and_exit, exit_code): write_clause_run_result refuses every temporal disposition (NativeRunResultError::TemporalDisposition, native_result.rs:2248-2254, unchanged), so no record carries label, reason or exit. FR-285 interim mapping and AC-10/12 are met; the AC-7 machine-output half is not wired and no ticket owns it. Also TemporalVerdict::category() maps Indeterminate to Category::Undefined, whose trace exit is 10, the code AC-7 forbids; a consumer reading category() instead of label_and_exit gets 10 (no in-repo consumer does today, since the temporal writer refuses). Undefined is a defensible shared value for the interim (nearest value below violation) but is not what FR-285 assigns; either the doc comment on category() should state the exit is not authoritative or category() should not be public for temporal verdicts. | qsl-replay/src/spine/native_result.rs:2248; qsl-replay/src/spine/clause.rs:506-513 | missing-requirement |
| FND-013 | low | A duplicate order key within one authority is treated as comparable by require_order, so two same-stamp observations with equal keys are decided by observation identity in the canonical order, although FR-501 says the order of observations without a usable key must not decide a verdict. The spec is silent on duplicate keys (AC-7 and AC-21 do not cover them); either refuse equal keys of one authority for an order-sensitive operator or state that identity breaks the tie. | qsl-eval/src/temporal/finite_window/run.rs:481-504 | correct-requirement-no-evidence |
| FND-014 | low | TraceBinding authenticates Profile and Clock name only; ClockParameters and member dimensions are not compared because the replay side does not expose the temporal_bindings row (documented in the code). An absent binding is refused without a mismatched_dimension field. No ticket owns exposing the row. | qsl-replay/src/spine/clause/temporal.rs:213-240 | missing-requirement |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-015 | low | In the lowering of a finite-window clause, `temporal_interval` falls back to the rational TimedInterval member when `tick_bounds` returns None (an open end, a non-integer bound or a bound above u64) instead of reporting a fault; the checker already admits only closed integral intervals, so the branch is unreachable today, but a future change to the checker would silently emit a differently-typed interval for the profile. | qsl-semantics/src/check/lowering/temporal.rs:temporal_interval | implementation-bug-despite-evidence |

## Dispositions

Round 1 against stacked PR 2169 head 0d9e923a61f1 (effective diff origin/main...head, main ceecfab74). Gates: workspace check and clippy -D warnings clean (default and --no-default-features); qsl-eval and qsl-replay tests green; qsl-semantics --all-features: 4 failures identical on origin/main (control run), none new; arch-lint qualified-core PASS.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | MissingHistory now comes from the possibility lift over an unseen position before the cutoff (lift.rs ScanFrame.history, run.rs reason()). qsl-eval test finite_window.rs:1255-1265: historically[0,4] with F,T at stamps 3,4 under HistoryCutoff is Completed(false); T,T is Indeterminate MissingHistory; once/since and activations after 0 covered. Mutation history-off (frame.history forced false): 2 eval and 3 replay tests fail. |
| FND-002 | fixed | counter_straddles_bound and the root-shape override are deleted. cover.rs ClockCover/Containment is applied to every eligible-position test (Part::Maybe candidates in the lift); all 15 TCV vectors are read from QSPEC_DIR at test time (finite_window.rs:702-707, spine temporal tests 1516-1518). Mutation straddle-sure (Indeterminate -> Part::Sure): 2 eval and 2 replay tests fail. |
| FND-003 | fixed | conflicting_timestamped_valuation is deleted. Per FR-501 Verdicts (main): observations at one stamp reading one atom differently give ConflictingValuation at every position of that stamp, named at the lowest canonical position; equal readings are a singleton (AC-23). Implemented in the cell build and lift (Cell::Conflicting), generic over clause shape. |
| FND-004 | fixed | engine.rs no longer refuses the finite-window profile as ProfileNotEvaluated; a stampless trace under that profile is refused TraceCarriesNoStamps and ProfileNotEvaluated remains only for the timed profile. evaluate_finite_window_at and the parallel timed-prefix entry are deleted. |
| FND-005 | fixed | require_order (run.rs) checks only adjacent eligible candidates of an order-sensitive scan with a guard; same_stamp_order_refusal is deleted. |
| FND-006 | fixed | TCV vectors are read from QSPEC_DIR in both crates and asserted against the file's expected values. See FND-011 for the residual skip-when-unset behaviour. |
| FND-007 | fixed | one TemporalVerdict::label_and_exit now feeds both the single-report path (category_label, exit_code) and temporal_batch_outcome (ranking 30,20,21,22,10,11,0); pending 0, indeterminate 11, success 0, violation 10, incomplete 22 as FR-285-AC-10/12 state. The residual wiring gaps are new findings FND-012. |
| FND-008 | fixed | lift.rs lifts over possibility sets with atom classes by holds text and exact enumeration up to 12 ambiguous atoms; test finite_window.rs:1220-1232 asserts p or not p with p absent is true and p and not p false. A two-valued result with W at or past the deadline is Indeterminate (run.rs: Pending only when watermark < need). Mutation atom-classes-off: 1 eval and 1 replay test fail. |
| FND-009 | fixed | No unreachable!, unwrap, expect, cast or u64::MAX saturation on added production paths (checked by grep of the effective diff); atoms are named by the condition's source text; dead code removed. |
| FND-010 | fixed | Mutation watermark >= to > in Run::established: 11 eval and 9 replay tests fail. Mutation comparator authority term removed from the canonical sort: 1 eval test fails. Mutation binding refusal off: 1 replay test fails. |

Round 2 against stacked PR 2169 head 28a7a3e2e0b5 (main fd5bde253 merged in; origin/main 4152e633f merge-tested). Rows below are this round's; each finding's latest row is current.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-011 | fixed | Both QSPEC_DIR vector tests now `expect` the variable (qsl-eval finite_window.rs:704, qsl-replay temporal tests:1538) and fail when it is unset. |
| FND-012 | deferred: ticket filed by the code lead; FR-267's temporal wire table has no finite-window kinds, so the native-result record cannot carry label, reason or exit 11 until that table is extended | The category() half is fixed: it is now pub(crate) shared_category(), documented not authoritative, and ClauseDisposition::category() documents the same; label_and_exit remains the one authoritative mapping. write_clause_run_result still refuses temporal dispositions. |
| FND-013 | deferred: spec gap (FR-501 is silent on repeated order keys of one authority); repeated keys are comparable with an identity tiebreak | Needs a spec ruling, not a code change. |
| FND-014 | deferred: the binding authenticates Profile and Clock only; ClockParameters and member dimensions need the temporal_bindings row, which the replay side does not expose | A TraceBinding is now required (absent or other profile or clock name is an admit refusal with mismatched_dimension); no ticket yet for exposing the row. |

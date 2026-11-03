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

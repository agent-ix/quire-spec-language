---
id: SR-1780001
title: "code review (with rust-review) of PR #1780 (QSL-1005d): timed refinement liveness on zones"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1780 git diff origin/terra-qsl-integration...206a6646e (13 files)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1780001: code review of PR #1780 (QSL-1005d)

## Summary

Head 206a6646e, base terra-qsl-integration. This is a port of #1702 (FR-430-AC-5). I reviewed the adaptations the PR names and the new item settlement. `live.rs`'s logic is #1702's, which was reviewed there (SR-1702001 and SR-1702002, included under `reviews/`).

**The adaptations hold.**
- `check_timed_refinement` builds `safety` from `Ended` exactly as before, then decides `liveness`:
  - `Unsupported` when there is a hidden field or a strong `F_A`, whatever the safety half;
  - `Checked` only when `F_A` is non-empty and safety is `Holds`;
  - otherwise `None`.
- `live.rs`'s early ends are mapped from `TimedSafetyOutcome<()>`. A `Holds` or `Violated` arriving there is a `liveness-end` fault, never a value.
- The types are generic over `S` where they carry the obligation (`TimedRefinementOutcome<S>`). The liveness outcome carries none.

**The settlement holds.**
- `settle_timed_refinement` checks the certificate first and returns the safety record unless it is `proved`. So a rejected certificate never lets the liveness half settle the item.
- After proved safety:
  - `Unsupported` gives V-8;
  - holds gives `Proved{Exhaustive, Uncertified}` with `ClosedScope`, the same row as the untimed `settle_liveness`;
  - `Violated` is `refuted` only through `replay` of the lasso.
- `replay_timed_refinement_trace` no longer refuses a lasso. It closes the loop through `close_loop` (the digests are equal, the stamp strictly grows, and each clock either resets back to its value or lies above the ceiling).
- The safety-half rows are unchanged; they are factored into `violated`, `undecided` and `timed_stopped`.

**FND-001 (low).** `TimedLivenessOutcome::HoldsReduced { reductions }` is always built with `Vec::new()`, and its doc says "none". A field that is always empty records nothing. A `Holds { stats }` variant says what it is.

**Gates** at 206a6646e (`--all-features`, debug info off, QSPEC_DIR = qspec-63ff21f1):

| Suite | Result |
| --- | --- |
| qsl-analyze it `timed_refinement zone_ settle analyze` | 263 passed |
| qsl-analyze lib | 94 passed |
| qsl-replay lib `refinement timed lasso` | 48 passed |
| clippy (qsl-analyze, qsl-replay) and fmt | exit 0 |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `TimedLivenessOutcome::HoldsReduced.reductions` is always empty; the variant should be `Holds { stats }` | FR-143 Outputs |

## Verdict

Mergeable.

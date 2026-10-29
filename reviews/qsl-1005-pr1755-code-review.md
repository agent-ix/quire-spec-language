---
id: SR-1755001
title: "PR #1755 (QSL-1005) code review: EN-6 item adapters in analyze"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff origin/qsl-1005-dense-route...d3748413e (qsl-analyze analyze.rs, analyze/items.rs, zone_check/settle.rs; qsl-replay proof_result.rs, model_check/settle.rs); head 9516a8585 adds only #1753's test"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: reviews
---

# PR #1755 code review (Rust)

## Summary

Ticket: QSL-1005. Heads d3748413e and 9516a8585; the second adds only #1753's fix-round test.

**Soundness:**
- **`zone_item` settles `Proved{ZoneCertified, Certified}` only on `Ok(Accepted)` from `check_zone_certificate`.** `check_zone_certificate` runs over a `ZoneCertificateRequest` that re-admits the subject independently of the run.
  - A rejected certificate settles `Inconclusive(CertificateRejected)`. A checker that stops settles `Incomplete`.
  - An obligation or identity mismatch settles `Inconclusive(ReplayRefused(stale_dependency))`, the same form EN-1 uses.
  - Any other checker refusal settles `Failed`.
- **The one proved row without a certificate is `HoldsReduced`.** It settles `Proved{Reduced, Uncertified}`, which is FR-160's row and the same as EN-1's.
- **`timed_refinement_item` never settles `proved`.** `Holds` gives `Err(UncertifiedTimedRefinement)`, which settles `unsupported`/`not-yet-implemented` (FR-430-AC-8).
- **`refuted` requires a replay.** In both maps it comes only from a replay that returns `Reproduced` and stores the counterexample. `settle` then re-checks this through `TerminalRecord::of_model_settlement`, so a `refuted` value with no counterexample settles `failed`.
- **Replay outcomes.** A replay of another package is stale. A disagreeing replay is `ReplayParity`, a budget stop is `Incomplete`, and a fault is `Failed`.
- **Refusal rows.** The `zone_refusal` and `timed_refinement_refusal` rows follow FR-281's engine-refusal rows. EN-1's `to_terminal` now goes through the same shared guard.

**Gates at d3748413e** (`--all-features`, debug info off):

| Crate or check | Result |
| --- | --- |
| qsl-replay | all passing |
| qsl-analyze | it 1012, lib 93 |
| Workspace clippy `-D warnings` | exit 0 |
| fmt | exit 0 |

At 9516a8585, `dense_route` and `timed_refinement` pass 20/20.

## Verdict

**Changes requested for FND-001.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `EngineReport::Zone` settles with `model_check: None`, so the analyze record drops the zone record and its counterexample. An EN-6 refutation by `UndefinedEvaluation` therefore writes a plain `refuted` outcome item, without FR-235 V-4's and FR-281's cause `UndefinedEvaluation{where, cause}`. | qsl-analyze/src/analyze.rs:546-559, 386-404 |

## Dispositions

Round 1, head 4177e8021.

Gates (`--all-features`, debug info off):
- qsl-analyze `dense_route` and `analyze`: 17 pass.
- qsl-analyze clippy `-D warnings`: exit 0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `AnalyzeRecord` now carries `zone_check: Option<Box<ZoneCheckRecord>>`, set to `Some(zone)` for `EngineReport::Zone`. The new `counterexample()` reads EN-1's record, then EN-6's, and `outcome_item` reads the undefined evaluation through it. When I set `zone_check` back to `None` as a mutant, `an_undefined_timed_evaluation_settles_refuted_through_analyze` fails (exit 101). |

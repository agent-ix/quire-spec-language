---
id: SR-1755002
title: "PR #1755 (QSL-1005) gap analysis: EN-6 adapters against FR-243, FR-245, FR-144, FR-430-AC-8"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-analyze/tests/it/dense_route.rs, timed_refinement.rs, zone_certificate.rs, zone_replay.rs at 9516a8585"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: reviews
---

# PR #1755 gap analysis

## Summary

Ticket: QSL-1005. Plan completion: not assessed.

**Covered by the end-to-end tests through `analyze`:**
- **FR-243-AC-1.** `NoLateReply` with T=4 settles `Proved{ZoneCertified, Certified}` after FR-245 accepts its certificate. With T=3 it settles `refuted` after its counterexample replays. Both name backend en-6.
- **FR-430-AC-1 and AC-8.** `TReq` with `send` mapped to `stutter` settles `refuted` through the replay. The holding `TReq` settles `unsupported`/`not-yet-implemented` and never `proved`.

**Covered at the settle level:**
- The new checker-refusal rows: `a_refused_certificate_check_settles_stale_or_failed`.
- The undefined-evaluation refutation: `an_undefined_timed_evaluation_settles_refuted`.

## Verdict

One low gap, which follows from SR-1755001 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test through `analyze` checks an EN-6 refutation's outcome item. An `UndefinedEvaluation` counterexample should come out as `refuted` with cause `undefined-evaluation`, as EN-1's analyze tests require. The SR-1755001 FND-001 drop would have been caught. | qsl-analyze/tests/it/zone_replay.rs:851, dense_route.rs:274-301 |

## Dispositions

Round 1, head 4177e8021.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `dense_route::an_undefined_timed_evaluation_settles_refuted_through_analyze` runs FR-239-AC-5's `Ticker` through `zone_item` and `analyze`. It asserts `refuted` on en-6, an `UndefinedEvaluation` counterexample at position 2 on the record, and an outcome item that is `refuted` with cause `undefined-evaluation`/`division-by-zero` at `position 2, state …`, category violation, never `undefined`. The test passes and kills the `zone_check: None` mutant. |

---
id: SR-2026110215402
title: "QSL-580 PR #2133 gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; quantified time-invariant parsing, checking and emission"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: reviews
---
# SR-2026110215402: QSL-580 PR #2133 gap analysis

## Summary

Reviewed the changed requirements, their direct trace tags, and the source-to-form-to-package path. The matrix computations did not complete within the bounded review window, so repository-wide tag counts are unavailable; manual inspection found a semantic gap for mixed quantified and ordinary invariant bounds.

## Verdict

**CONDITIONAL** — matrix verification is unavailable, and the scoped implementation path has a medium semantic mismatch described below.

## Coverage

- Reconciliation: attempted `quoin matrix --repo <repo> --json` (45-second timeout) and `quire matrix --scope <repo> --format json` (35-second timeout); neither emitted matrix JSON. No repository-wide counts or evidence status are claimed.
- Manual changed-criterion bindings: `FR-393-AC-9` is tagged by `quantified_time_invariant_bounds_build_in_source_order_with_spans`; `FR-230-AC-56` is tagged by `a_quantified_time_invariant_bounds_the_live_slot_family`, `quantified_time_invariant_emits_binder_and_live_slot_bound`, and `unsupported_quantified_invariant_forms_refuse_without_partial_output`. `FR-230-AC-50` is tagged by the quantified element-presence refusal test.
- Reverse-gap and stub scan: no new behavior without an owning timed-language/QSpec requirement and no production stubs found in the changed code. The point-qualified timed-anchor refusal remains owned by FR-230-AC-50 / FR-342-AC-11.
- Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A parsed mixed invariant is accepted but emits one global binder scope for both the quantified and ordinary bounds, so the ordinary bound can be vacuous on an empty collection. | FR-230-AC-56; qsl-semantics/src/check/timed_model.rs:3913 |

## Detail

FR-393-AC-9 explicitly exercises a source form with both `forall(e in self.ts: e.x <= 9 ms)` and the ordinary bound `self.x < 12 ms`. S2 preserves both, but S3's `time_invariant` combines the bounds and binder in one `InvariantDraft`; lowering emits one `time_invariant` whose `binders` scope all upper bounds. The checked-package contract (FR-415-AC-22) gives those binders scope over the invariant predicate and upper bounds, while FR-415-AC-24 makes the quantified form vacuously true when its collection is empty. The ordinary `self.x` bound therefore needs an independent scope or this mixed form must refuse at S3.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The S3 checker refuses mixed ordinary and quantified bounds; the adverse test covers the empty-capable collection fixture and verifies no partial output. |


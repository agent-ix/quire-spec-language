---
id: SR-2026110216375
title: "QSL-1493 PR #2165 gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-501-AC-1..12 and FR-330-AC-9 finite-window evaluator path"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/spec/spec.md
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-501
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: reviews
---
# SR-2026110216375: QSL-1493 PR #2165 gap analysis

## Summary

Audited the timestamped finite-window criteria and their tagged Rust tests, plus the implementation path from checked clause to spine outcome. Matrix generation timed out without JSON; manual trace inspection found two untagged criteria, and code review separately records behavioral mismatches.

## Verdict

**FAIL** — FR-501-AC-5 and FR-501-AC-6 have no test trace bindings in the repository; the matrix command did not finish, so no repository-wide matrix rollup is claimed.

## Coverage

- Matrix verification: `quoin matrix --repo . --json` timed out after 60 seconds with no JSON; `quire matrix --scope . --format json` timed out after 45 seconds with no JSON (only standard module warnings). Repository-wide matrix counts/status are unavailable.
- Manual changed-criterion trace audit: FR-501-AC-1..4 and AC-7..12 have matching `#[trace]` attributes in `qsl-replay/src/spine/clause/tests/temporal.rs`; FR-330-AC-9 is tagged by `timestamped_profile_and_trace_shape_must_match`. No source/test file contains a `FR-501-AC-5` or `FR-501-AC-6` trace tag.
- Reverse-gap and stub scan: timestamped input, cover, progress, temporal outcomes, checked-interval conversion and admission behavior are owned by FR-501/FR-330; temporal activation guard behavior is owned by FR-044 and category/exit mapping by FR-285. No TODO, FIXME, `todo!()` or `unimplemented!()` was found in the changed production paths. No applicable AssuranceProfile or plan was supplied.
- Manual source-to-test note: `timestamped_windows_pending_and_decisive_witnesses` includes incomplete-input assertions relevant to AC-5 but is tagged only AC-3. The current AC-1/2 tick-window test does not exercise AC-6's specified deadline values and half-open watermark case.
- Semantic review was skipped; it was optional and not requested. Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-501-AC-5 has no test binding; the existing incomplete-input assertions are inside an AC-3-tagged test only. | FR-501-AC-5; qsl-replay/src/spine/clause/tests/temporal.rs:1300 |
| FND-002 | medium | FR-501-AC-6 has no trace-tagged test, and its specified exact deadline and half-open watermark cases are not asserted. | FR-501-AC-6; qsl-replay/src/spine/clause/tests/temporal.rs:1244 |

## Findings Detail

### FND-001

FR-501-AC-5 is absent from every code trace tag. `timestamped_windows_pending_and_decisive_witnesses` asserts incomplete completeness and short completeness cases, but its only criterion tag is FR-501-AC-3, so the matrix cannot credit those assertions to AC-5.

### FND-002

FR-501-AC-6 is absent from every code trace tag. The current AC-1/2 test covers a different upper bound and a general sparse-window example, but does not assert AC-6's `eventually[0,3]` deadline boundary or the rule that a half-open `[..,3)` watermark without an inclusive QSpec mapping is consumed only through tick 2.

## Scope

- {id: "FR-501-AC-1..12", path: "spec/functional/FR-501-evaluate-a-temporal-clause-under-the-timestamped-event-finite-window-profile.md", role: examined, excerpt: "Finite-window timestamped behavior, including eligibility, progress, evidence uncertainty, order, cover, history, exit and work-budget criteria."}
- {id: "FR-330-AC-9", path: "spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md", role: examined, excerpt: "Finite-window profile and timestamped trace shape must match; position-only traces refuse at admission."}
- {id: "FR-044", path: "spec/functional/FR-044-activate-temporal-obligations.md", role: context_only, excerpt: "An unestablished activation guard is incomplete or refused, never false."}
- {id: "qsl-replay/src/spine/clause/tests/temporal.rs", path: "qsl-replay/src/spine/clause/tests/temporal.rs", role: examined, excerpt: "Timestamped finite-window traced tests; AC-5/AC-6 are not present in trace attributes."}
- {id: "qsl-replay/src/spine/clause/temporal.rs", path: "qsl-replay/src/spine/clause/temporal.rs", role: examined, excerpt: "Spine admission, progress conversion, evaluation and temporal result mapping."}
- {id: "qsl-eval/src/temporal/observed.rs", path: "qsl-eval/src/temporal/observed.rs", role: examined, excerpt: "Observed finite-window activation and atom-reading adapter."}
- {id: "qsl-eval/src/temporal/timed.rs", path: "qsl-eval/src/temporal/timed.rs", role: examined, excerpt: "Timed finite-window window closure and temporal evaluation."}

## Bindings

- {test_id: "timestamped_windows_use_ticks_and_close_without_false_extension", ac_id: "FR-501-AC-1", trace: correct}
- {test_id: "timestamped_windows_use_ticks_and_close_without_false_extension", ac_id: "FR-501-AC-2", trace: correct}
- {test_id: "timestamped_windows_pending_and_decisive_witnesses", ac_id: "FR-501-AC-3", trace: correct}
- {test_id: "timestamped_missing_and_conflicting_atom_readings_are_indeterminate", ac_id: "FR-501-AC-4", trace: correct}
- {test_id: "timestamped_counter_cover_uses_half_open_boundary_rules", ac_id: "FR-501-AC-9", trace: correct}
- {test_id: "finite_window_report_preserves_pending_and_indeterminate_exit_classes", ac_id: "FR-501-AC-11", trace: correct}
- {test_id: "timestamped_window_charges_exact_visits_and_stops_on_the_denied_visit", ac_id: "FR-501-AC-12", trace: correct}
- {test_id: "timestamped_profile_and_trace_shape_must_match", ac_id: "FR-330-AC-9", trace: correct}
- {test_id: "order_sensitive_equal_stamps_require_one_authority_and_sort_by_key", ac_id: "FR-501-AC-7", trace: correct}
- {test_id: "timestamped_past_windows_respect_history_cutoff", ac_id: "FR-501-AC-8", trace: correct}
- {test_id: "timestamped_guarded_on_each_activations_are_evaluated", ac_id: "FR-501-AC-10", trace: correct}

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | A dedicated test now asserts incomplete and complete outcomes and carries the FR-501-AC-5 trace tag. |
| FND-002 | fixed | A dedicated test covers the inclusive deadline, next tick, and half-open watermark and carries the FR-501-AC-6 trace tag. |

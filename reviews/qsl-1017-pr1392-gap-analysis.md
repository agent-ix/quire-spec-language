---
id: SR-16321
title: "Gap analysis of quire-spec-language PR #1392: hybrid claims and interval forms"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9809b06b8cbeb1a8a02025b9a73ac02c323e1bc9; FR-393-AC-5, FR-447-AC-3, AC-7, AC-9, AC-10, AC-11 and S3 portion of AC-15; qsl-forms/tests/it/{hybrid_forms,time_forms}.rs; qsl-semantics/src/check/hybrid/tests.rs; related implementation"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-393
    type: reviews
---
# Gap analysis of quire-spec-language PR #1392

## Summary

Ticket: QSL-1017, second PR in the hybrid stack. Plan completion: not assessed.
The repo-wide `quoin matrix --repo . --json` was attempted and canceled after
several minutes without output. This review used the changed acceptance text,
trace attributes, test assertions and implementation directly. AC-15's S4
portion is assigned to PR #1394 and was not judged here.

The new tests exercise checked claim shape, activation, captures, fairness,
past and future horizons, exact interval conversion, profile fit and selected
refusals. `calls_of_pure_functions_lower_and_foreign_bodies_refuse` exercises
the S3 part of AC-7. The FR-393-AC-5 test still exercises its two negative
cases.

## Verdict

**CONDITIONAL**: one untested and untraced positive clause added to
FR-393-AC-5.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-393-AC-5 now says a timed `[q, *]` interval parses. Its `#[trace("TC-1078", "FR-393-AC-5")]` test asserts only the two malformed mixed-bound intervals. The positive S2 assertion lives in `hybrid_forms` under FR-447-AC-9, so deleting `[q, *]` support would leave every test bound to FR-393-AC-5 green. Add a positive test bound to AC-5 or retag the existing assertion, and update TC-1078 step 5. | qsl-forms/tests/it/time_forms.rs:392-410; qsl-forms/tests/it/hybrid_forms.rs:162-180; spec/test-cases/TC-1078-s2-builds-time-declaration-quantity-and-timed-interval-forms.md:25 |

## Coverage

Plan completion: not assessed. Matrix status: unavailable in this run; targeted
trace inspection only. FR-447-AC-3, AC-7, AC-9, AC-10 and AC-11 have tagged
tests with assertions that can fail. FR-447-AC-15 has partial S3 evidence in
this PR and awaits its S4 evidence in the stacked emission PR.

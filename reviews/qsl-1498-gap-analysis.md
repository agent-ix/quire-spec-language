---
id: SR-20261104091644
title: "Gap analysis — QSL-1498 exact certificate groundwork"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-replay/src/lib.rs, qsl-replay/src/probability_certificate.rs; FR-201-AC-6 exact-value slice"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: references
---

## Summary

The full Quoin matrix command did not return within the review window, so this is a manual, slice-only acceptance-to-tests check rather than a repository-wide matrix claim. The new tests exercise exact-value mismatch and threshold-miss rejection, but their direct FR-201-AC-6 tags can make a binary trace matrix treat that multi-case criterion as covered before its remaining rule/locus cases land.

## Verdict

**CONDITIONAL** — retain FR-201-AC-6 as open until the remaining specified rejection rules and loci have focused evidence; this finding concerns test coverage signaling for the delivered slice, not the separate QSL-1498 provider, replay, settlement, or timed work.

## Coverage

- Manual slice mapping: `exact_certificate_checks_each_state_and_reports_first_bad_key` checks acceptance and an exact-value inequality rejection; `exact_certificate_rejects_threshold_miss_and_cycles` checks a threshold miss and an explicit cycle stop. Both are tagged `FR-201-AC-6`.
- FR-201-AC-6 additionally specifies LongRun inequality, malformed missing-ranking, rank-missing, and rank-broken rejection cases. Those certificate forms and tests are not in this exact-value groundwork diff. The issue comment records them as remaining work.
- Broader FR-201 timed, fair, unbounded, identity, budget, input-limit, and full model/product re-enumeration behavior; all FR-202 replay; provider/settlement; and FR-204-AC-1..5 with TC-639/642 remain outside this PR, as tracked on QSL-1498.
- Quoin matrix: attempted `quoin matrix --repo . --json`; it produced no output and was interrupted after remaining pending for several minutes. No repository-wide matrix conclusion is made.
- Plan completion: not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-201-AC-6 trace tags cover only two of the criterion's rejection cases in this slice, so a binary trace matrix can report the criterion as covered before its remaining rule/locus cases are tested. | qsl-replay/src/probability_certificate.rs:437,469 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Both exact-value tests now retain the `TC-636` tag without claiming full `FR-201-AC-6` coverage. |

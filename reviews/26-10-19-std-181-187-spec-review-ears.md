---
id: SR-12912
title: "EARS review of PR #1001: new and edited refusal statements in FR-185 and FR-186"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@739cd6354; FR-185 random-parameter, workload and reward refusal statements; FR-186 scheduler, threshold, quantile, unit and confidence refusal statements; FR-393 S2 magnitude statement"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
---

## Summary

Tickets: STD-181, STD-187. PR quire-spec-language#1001. This review checks
the requirement statements the diff adds or edits against EARS: one
responsible system, one trigger or condition, and one response.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-185's random-parameter bullet adds one passive sentence carrying four refusals: "A weight that is not positive SHALL be refused [`invalid-weight`], a value of another type [`value-outside-type`], a repeated value [`repeated-support-value`], and a second distribution on one parameter [`duplicate-distribution`]." It names no responsible system, and three of its clauses have no verb. The neighbouring bullets use the form "When …, S3 SHALL refuse it [cause]". Fix: split it into four unwanted-behaviour statements, "If a weight is not positive, S3 SHALL refuse it [`invalid-weight`], naming the parameter", and so on. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:76-79 |
| FND-002 | low | FR-186 adds two passive SHALLs that name no system and no trigger: "Any other producer of a scheduler that pairs a workload with a fairness set SHALL be refused [`fairness-under-workload`]", and "A workload of another model than the subject's SHALL be refused [`foreign-workload`]". Fix: "If a claim's `under W` names a workload of another model than the subject's, S3 SHALL refuse it [`foreign-workload`], naming the workload and the model", and either the same form for fairness, with a named producer, or no SHALL at all (see SR-12909 FND-001). | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:105-109 |

## Verdict

The other edited statements conform: FR-185's precondition, workload and
reward refusals ("When …, S3 SHALL refuse it [cause], naming …"), FR-186's
qualitative-threshold, quantile-over-fraction and confidence refusals, and
FR-393's S2 magnitude statement each give one system (S2 or S3), one
condition and one response. The workload bullet's "respectively" list maps
four conditions onto four causes in one sentence, but it predates the PR and
the causes are unambiguous, so it is not a finding.

Mergeable as far as this lens goes: two low findings, no medium or high.

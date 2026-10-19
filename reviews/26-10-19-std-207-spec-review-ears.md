---
id: SR-13231
title: "EARS review of PR #1037: FR-250 solver budget unit statements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@25a16c9df; git diff origin/main...HEAD: FR-250 Behavior (solver-time, solver-steps)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-250
    type: reviews
---

## Summary

Ticket: STD-207. EARS conformance of the two requirement statements this PR adds to FR-250's Behavior list. Both are ubiquitous-form unit definitions; the first is followed by a conditional clause (fractional duration) and the second carries its key obligation in a relative clause.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-250 Behavior: solver-time bullet | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md | examined |
| FR-250 Behavior: solver-steps bullet | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "A duration with a fractional part SHALL round up to the next whole second" has no responsible system and no trigger: a duration cannot itself round. In EARS form: "If a requested solver-time duration has a fractional part, then the request writer SHALL round it up to the next whole second." Naming the subject also answers where rounding happens (SR-13230 FND-001). | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:83-86 |
| FND-002 | low | The obligation the ticket is about, that steps are never converted to time, sits in a relative clause without SHALL: "which QSL compares and reports unchanged and does not convert to time". Split it: "QSL SHALL compare and report a `solver-steps` value as the provider counts it, and SHALL NOT convert it to time." | spec/functional/FR-250-map-a-hybrid-solver-result-to-a-verdict.md:87-89 |

## Verdict

Findings: FND-001 (low), FND-002 (low). The unit statements themselves ("SHALL be a count of whole seconds of wall-clock time", "SHALL be a count of solver steps") are atomic and testable.

## Dispositions

Round 1, against head 594d3e3e1 (fix commit 594d3e3e1).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-250: "If it has a fractional part, then the request writer SHALL round it up to the next whole second when the request is admitted". |
| FND-002 | fixed | FR-250: "QSL SHALL compare and report a `solver-steps` value as the provider counts it, and SHALL NOT convert it to time." |

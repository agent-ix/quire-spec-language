---
id: SR-20261104130236
title: "Evidence review — QSL-1642 timed population scope"
type: SpecReview
scope: "FR-230-AC-55; TC-1331; qsl-semantics/tests/it/timed_model.rs"
analysis: evidence
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
---

## Summary

A focused integration test is an appropriate direct method for the compilation refusal and successful controls, and the implementation test exercises those outcomes. The test's criterion tag is ambiguous because FR-230-AC-55 names both this criterion and an existing clock-guard criterion.

## Verdict

**FAIL** — the evidence exists but cannot bind uniquely to the new obligation until the criterion id is corrected.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new integration test is tagged with a duplicated FR-230-AC-55 id, so the evidence record cannot identify whether it verifies this refusal or the existing clock-guard criterion. | FR-230-AC-55; timed_model.rs:291 |

## Review Notes

quoin advise --json was attempted with a 90-second timeout and produced no output. Manual method check: Test matches the observable compile/refusal and positive-control outcomes in TC-1331. The existing draft evidence note also records an earlier five-minute advisor stall.

## Dispositions

Round 1, reviewed against final head f9e37ff46e798976cb75b87bf15f5b6b2a6f8a67. The duplicate id is corrected: the existing quantified clock-guard criterion remains FR-230-AC-55; the named-population refusal now has the unique id FR-230-AC-58, uses TC-1334, and the implementation test carries both corrected trace tags. The focused timed-population integration test passed (1 test).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: In a timed model whose domain package declares calls, time invariant HasCalls when { forall(c in Config::calls : true) } { self.x <= 1 ms } and urgent CallsExist when { exists(c in Config::calls : true) } each refuse unsupported_construct/not-yet-implemented at Config::calls, naming the missing timed-predicate population scope, and produce no TimedModel. Time invariant and urgency predicates that read only self still check. The same named-population reads in a unit-level precondition remain admitted under FR-391. |

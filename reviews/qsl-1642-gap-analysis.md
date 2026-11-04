---
id: SR-20261104130233
title: "Gap analysis — QSL-1642 timed population refusal"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-230-AC-55, FR-391 timed-predicate boundary, FR-480-AC-17, TC-1331, qsl-semantics/src/check/timed_model.rs, qsl-semantics/tests/it/timed_model.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
---

## Summary

The changed behavior has a matching implementation and focused test, but the test's FR-230-AC-55 tag cannot identify the new criterion uniquely because that id already belongs to the clock-guard criterion. Full repository matrix and reverse-gap claims are unavailable because the matrix command timed out.

## Verdict

**FAIL** — duplicate criterion identity makes the new requirement-to-test binding ambiguous.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The implementation test is tagged with FR-230-AC-55, but that id already denotes the quantified clock-guard criterion; the timed-population criterion has no uniquely addressable test binding. | FR-230:879, FR-230:886; timed_model.rs:291 |

## Coverage

Manual scoped check: the code refuses the declared population read at the selected timed-predicate location; TC-1331 exercises invariant and urgency refusals plus self-only and unit-precondition controls; the integration test covers those cases. quire matrix --scope . --format json timed out after 120 seconds with no output, so the repository's computed matrix, reverse-gap inventory, and stub audit are not claimed. Plan completion: not assessed.

## Dispositions

Round 1, reviewed against final head f9e37ff46e798976cb75b87bf15f5b6b2a6f8a67. The duplicate id is corrected: the existing quantified clock-guard criterion remains FR-230-AC-55; the named-population refusal now has the unique id FR-230-AC-58, uses TC-1334, and the implementation test carries both corrected trace tags. The focused timed-population integration test passed (1 test).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | after_excerpt: In a timed model whose domain package declares calls, time invariant HasCalls when { forall(c in Config::calls : true) } { self.x <= 1 ms } and urgent CallsExist when { exists(c in Config::calls : true) } each refuse unsupported_construct/not-yet-implemented at Config::calls, naming the missing timed-predicate population scope, and produce no TimedModel. Time invariant and urgency predicates that read only self still check. The same named-population reads in a unit-level precondition remain admitted under FR-391. |

---
id: SR-12118
title: "spec-review of quire-spec-language PR 950 (TC-830 step 4 admission total)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c4e5206fb5f3c0f5df3fbe5081e7a67a592963ef; spec/test-cases/TC-830-union-values-round-trip-through-v2-literals-and-typed-values-at-any-de.md (step 4)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/TC-830
    type: reviews
---

## Summary

Ticket: QSL-778. One spec-review pass over the TC-830 step 4 edit (+4/-2).
The step now states A = 79,996 and its breakdown by event kind in place of
the "do not substitute the research prediction 69,997" sentence.

- Recomputed from QSV FR-109's event table (value visit, type comparison,
  child scheduling, descriptor query) and the trusted-admitted-value boundary
  (Behavior 4). Each `Branch` level: the union's value visit and descriptor
  query, two payload schedulings, the integer's visit, the option's visit,
  one type-link comparison of the option's retained payload type
  (`Composite(Tree)` against `Composite(Tree)`, equal on the first link), and
  one scheduling of the present payload. That is 8 events. The `Leaf` is a
  visit, a query, one scheduling and the integer's visit, which is 4. The
  total is 9,999 x 8 + 4 = 79,996.
- The breakdown by kind adds up: 29,999 visits + 10,000 queries + 29,998
  schedulings + 9,999 comparisons = 79,996. The visit count equals the
  29,999 semantic occurrences, as it should.
- The removed 69,997 is 79,996 minus the 9,999 option type comparisons. The
  new text keeps FR-109's rule: no revalidation of trusted descendants (each
  Option is checked against its retained type, not walked twice).
- The text states what is, with no history and no ticket ids. It agrees
  with FR-321-AC-5 ("A is the reviewed membership event sequence counted
  independently of the evaluator").

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. The corrected A = 79,996 and its per-kind breakdown are right against
QSV FR-109 and the locked QSV implementation.

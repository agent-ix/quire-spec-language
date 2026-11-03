---
id: SR-2026110217005
title: "Gap analysis of QSL-1514 and QSL-1495 tagged tests (QSL PR #2219)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@9e5fbc2935438b72a25edd1100eeeda8419369dc; PR #2219 origin/main...HEAD; QSL-1514, QSL-1495; FR-504, FR-330-AC-10/11, FR-052-AC-9..14, TC-1299, TC-144"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-504
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: reviews
---
# SR-2026110217005: Gap analysis of QSL-1514 and QSL-1495 tagged tests (QSL PR #2219)

## Summary

Ticket: QSL-1514, QSL-1495 (partial). Plan completion: not assessed. FR-504-AC-1..8 and FR-330-AC-10 are each tagged to a non-vacuous test; each would fail on the old ProfileNotEvaluated behavior. TC-144 rows 1, 4a-4f, 4h, 5 (Lasso part), 6, 7, 8 are covered by checked_route_owner; rows 2, 3, 4g, 9 and row 5's Timestamped 0,5,2 are not, consistent with the ticket's partial claim, so FR-052-AC-12 carries no tag yet.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `on each` change that anchors activations only on observations (run.rs, observed vs len) is untested: the only on-each Progressed test is event-position where observed equals len, so reverting it fails nothing. FR-504 is silent on whether a silent sample can anchor an activation, so the behavior needs a spec ruling and a fixed-sample on-each test. | qsl-eval/src/temporal/finite_window/run.rs:263-276 |
| FND-002 | low | The FR-330-AC-11 Progressed binding test is tagged TC-1299 but FR-330-AC-11 lists TC-1317 as its test case. | qsl-replay/src/spine/clause/tests/temporal/progressed.rs:318 |
| FND-003 | low | FR-504 paths with no Progressed test: MissingValuation instance/also_held_by members (FR-524), Indeterminate when a fixed-sample W has reached anchor plus reach under an open scope, and fixed-sample decisive-witness inside S. | spec/functional/FR-504-carry-progress-on-event-position-and-fixed-sample-traces.md |

## Verdict

Mapped criteria are genuinely evidenced. The remaining items are coverage gaps and spec rulings, none a regression.

---
id: SR-2264002
title: "Gap analysis of QSL-1622 monitor documents from clause runs (quire-spec-language PR 2264)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@33095598b7dfca0162ec35098f8c5e82685345a9; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md; spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md; spec/functional/FR-283-monitor-a-supplied-trace-offline-against-temporal-clauses.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: reviews
---
# SR-2264002: Gap analysis of QSL-1622 (PR 2264)

## Summary

Ticket: QSL-1622. Planless, scoped to git diff origin/main...HEAD on a trial merge with origin/main d421482f4. Plan completion: not assessed.

Units examined: FR-286-AC-8 (partly covered: AC-8 first clause tested by a_pending_clause_run_is_a_pending_monitor_document_that_exits_zero tagged TC-770/FR-286-AC-8 and FR-285-AC-9; the clause "the same run through the FR-283 monitor operation gives the same item" has no code and no test), FR-286-AC-9 (covered by indeterminate_and_mixed_clause_runs_rank_by_severity), FR-285-AC-9 monitor half (covered), FR-286 label and cause table (success, violation, pending, indeterminate, undefined tested; incomplete, unsupported, refusal rows untested), FR-283 Outputs (records rendered through from_clause_run: nothing renders them yet), TC-770 step 7 (first half only) and step 8, TC-769 step 10 (grep: no new production hit; the existing "pending"/"indeterminate" string sites in qsl-foundation and native_result are not paired with exit integers).

The monitor operation (qsl-analyze::monitor, FR-283) exists nowhere: no fn monitor, MonitorRequest or MonitorOutcome in any production source. It is a ticketed behavior owned by QSL-597 (LC9 Offline monitor, Implements FR-283, state Specify, unassigned, blocked by QSL-509, QSL-489, QSL-592, STD-143). It is not a new ticket; it is a spec gap only in that FR-286's Delivery line assigns AC-8 whole to QSL-1622 while a clause of AC-8 can only be met by QSL-597.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-286-AC-8's "same run through the FR-283 monitor operation gives the same item" and TC-770 step 7's "run the same clause through monitor" cannot be met by QSL-1622 because the operation is built by QSL-597; FR-286 Delivery says QSL-1622 delivers AC-8. Spec lane: amend the Delivery line and TC-770 step 7 to name QSL-597 for that clause, and add to QSL-597 an acceptance line that every monitor record is rendered through OutcomeItem::from_clause_run and equals the from_clause_runs item. Until then the clause is neither delivered nor owned. | spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:215,234 | missing-requirement |
| FND-002 | medium | FR-286 table rows incomplete, unsupported and refusal (and their causes) have no test; label swaps survive. | qsl-replay/src/spine/clause/tests/temporal.rs:3280-3422 | correct-requirement-no-evidence |

## Verdict

Coverage is complete for the pending, indeterminate, undefined and mixed-ranking paths and absent for the monitor-operation clause and three table rows.

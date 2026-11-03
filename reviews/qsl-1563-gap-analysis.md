---
id: SR-202611030202
title: "Gap analysis — QSL-1563 one-list check protocol refusals"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/mod.rs (PR #2171); FR-383-AC-18, AC-28, AC-29; FR-218-AC-30; FR-136-AC-8; TC-1297"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1297
    type: references
---

## Summary

Ticket: QSL-1563. The change is behavior-restoring and adds no requirement. The four failing tests tag FR-218-AC-30 (choice partition, nested repeat), FR-136-AC-8 (refinement beside a refused clause) and the requirement-cycle row; they now pass. The #2157 tests for FR-383 per-protocol results (qsl-package emit tests, TC-1297 via `check_with_protocol_results`) still pass, and three of them (`an_unwritten_construct_refuses_a_selected_profile_and_a_reference_to_it`, `a_protocol_requiring_a_refused_temporal_adds_no_second_refusal`, `a_refinement_over_an_unwritten_clause_refuses_naming_the_construct`) fail on main and pass here. Plan completion: not assessed.

## Verdict

**PASS** — no spec or trace gaps. One optional coverage note below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test pins the check() ordering contract: protocol refusals first, then refinement refusals, and no later-stage consequences after a refused protocol | qsl-semantics/src/check/mod.rs:1509-1531, 3820 |

`check_with_cancel` now stops after the refinements when a protocol refused. The four existing tests detect a dropped protocol refusal (the refinement test checks both the clause refusal and the row refusal are present), but none asserts the order or that a later-stage consequence (for example the unkeyed reference or spent work budget the `check_retaining_protocols` comment names) is absent. A small test beside `a_refusal_of_a_row_outside_the_clause_is_reported_beside_a_refused_clause` asserting the first refusal is the clause's `ProtocolAnchor` and the `operator-ineligible` row follows would pin it. Recommendation only; not a merge blocker.

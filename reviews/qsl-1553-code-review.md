---
id: SR-202611040233
title: "Code review of quire-spec-language PR 2242: QSL-1553"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7b48fcdcb; qsl-package/src/emit/tests/protocol_controls.rs"
review_set: subset
date: 2026-11-04
---

# SR-202611040233: Code review of quire-spec-language PR 2242 (QSL-1553)

## Summary

Ticket: QSL-1553. This review covers code-review plus the rust-review lane. The only code change is one test plus one helper in `qsl-package/src/emit/tests/protocol_controls.rs`. The helper `expression_occurrences_of_node_at` finds the single node that has an `expression` occurrence at a given `Location`, asserting that exactly one node does. It returns that node's `expression` occurrences in iteration order, and `CheckedGraph::occurrences` documents that order as ascending by node, role and ordinal. The test checks fixture E (`tc1070::SOURCE`) and compares full vectors of (ordinal, Location), so an extra, missing or misnumbered occurrence fails it. The paths (`[0,0,0,0]` and `[0,0,1,1,0]` in retry 0, `[1]` in trigger-guard 0, `[0,1]` in retry 0) match the fixture's expression trees. The test does not mock anything. It runs the real assembler and checker. The test has no `unwrap`/`expect` outside test code, and its names are clear. The focused test passed on head 7b48fcdcb (result recorded in Verdict).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

**Mergeable.** The focused test `each_position_of_an_application_has_its_own_occurrence_in_document_order` passes.

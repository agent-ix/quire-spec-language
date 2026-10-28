---
id: SR-1770002
title: "gap analysis of PR #1770 (QSL-580u): urgency emission"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1770 at 37cc994a9 against FR-230-AC-53, TC-685 step 36, QSpec FR-322 urgency and AC-260"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1770002: gap analysis of PR #1770 (QSL-580u)

## Summary

FR-230-AC-53 and TC-685 step 36 are covered by three tests:
- `urgency_declarations_emit_and_read_back`: the two kinds, `operation_name` and the model, read back by I2.
- `an_urgency_predicate_is_a_located_expression_node`: the declared order is not alphabetical, each node is located at its declaration, the predicate is a located expression node, and `self` is a level-0 parameter.
- `an_urgency_reading_a_presence_is_not_emitted`: `present(self.v)` is refused.

The spec edit (the FR-230 "Urgency" paragraph, AC-53 and TC-685 step 36) validates.

Gaps:
- **FND-001.** No test of two urgencies with equal content (see SR-1770001 FND-001).
- **FND-002.** The refusal is tested only for an own-place presence. FR-230 names `present(p)` of another object's place and of a quantified element too; those are the `reads_presence_place` branch, and no test covers them. Nor is there a test that a predicate not reading `self` writes no `parameter` node.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test of two urgencies with equal content | FR-322 Timed-model nodes |
| FND-002 | low | No test of the `reads_presence_place` refusal (`present(self.peer.v)`, `forall(e in self.q : present(e))`) or of a `self`-free predicate writing no parameter node | FR-230 Urgency, FR-230-AC-53 |

## Verdict

Changes requested together with SR-1770001. Both findings are low.

---
id: SR-1758002
title: "gap analysis of PR #1758 (QSL-1206 / STD-539): nested repeat progress"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1758 at 8dcb8e559 against QSpec #481 (STD-539, a58e0852) FR-464-AC-21/23, FR-042, FR-218-AC-32"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1758002: gap analysis of PR #1758 (QSL-1206 / STD-539)

## Summary

Ticket: QSL-1206. Head: 8dcb8e559.

Covered:
- FR-464-AC-23 (exhausted never counted): `a_nested_repeat_is_never_counted_through_its_exhausted_control`.
- FR-464-AC-21 item 2 (`max 0`): `a_nested_repeat_with_maximum_zero_gives_no_progress`.
- The counter reset: `a_nested_repeat_counter_restarts_on_each_entry`.
- The feasible and infeasible guard cases: the existing FR-218-AC-32 tests.

Gaps:
- **FND-001.** FR-464-AC-21's `choice` clause has no test: a nested repeat that is the only stepping part of a `choice` case follows the same rule. My probe shows an event arm plus a `max 0` repeat arm refused, and the same with `max 1` admitted.
- **FND-002.** No test covers deeper nesting through an intermediate repeat (`Outer { Mid { Inner max 0 } }`) or a repeat reached through an in-body `exhausted`. Both are what the `in_body` inheritance in `nested_repeats` decides. My probes show both refused.

Cross-PR note: STD-539 makes repeat-through-repeat a supported capability. #1737's `ProtocolConstruct::RepeatStepsThroughRepeat` (STD-493, not on main) refuses every such repeat at emission, so it needs revisiting when #1737 lands.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test of FR-464-AC-21's `choice` clause (probe: `max 0` arm refused, `max 1` arm admitted) | FR-464-AC-21 |
| FND-002 | low | No test of deeper nesting or of a repeat reached through an in-body `exhausted`, the `in_body` inheritance (probe: both refused) | FR-464 Nested repeat rule |

## Verdict

Mergeable. Both findings are low.

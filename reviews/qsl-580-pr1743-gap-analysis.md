---
id: SR-1743002
title: "gap analysis of PR #1743 (QSL-580r): postcondition presence requirements"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1743 at 309d37970 against FR-230 Presence bit (AC-12/39/41/42) and STD-526's presence ruling (QSL FR-230-AC-51)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1743002: gap analysis of PR #1743 (QSL-580r)

## Summary

Ticket: QSL-580. Head: 309d37970.

What is covered:
- `a_postcondition_requires_a_presence_of_its_post_state` checks the three conjunct forms on the S3 side.
- `a_postcondition_presence_filters_the_successors` checks Option filtering in eval.
- Both kill the obvious mutants: dropping the `presences_hold` check, or dropping `requires.push`.

Behaviour I probed that no test pins:
- A requirement that contradicts a write in the same postcondition leaves no successor.
- A union-case requirement filters a frame havoc of the union field.
- A filtered requirement reaches the zone checker through `guarded_steps`, so a state whose only operation is filtered is a deadlock for the zone checker.

All three behave correctly, so they belong in tests (FND-001, FND-002).

STD-526's presence ruling (merged into QSL FR-230 as AC-51, in #1713, which this base does not include yet) rules `present(other.v)`, `present(self.q[i])` and `present(e)` in a quantifier body in, as data predicates resolved through the place identity. This PR still refuses them as `not-yet-implemented`, and its test comment cites STD-526 for the refusal. That is within scope for a top-level-conjunct slice, but I found no open ticket for AC-51 (QSL-580 is Done) (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test that a requirement contradicting a write (`self.v = absent and present(self.v)`) leaves no successor, or that a union-case requirement filters a havoc | FR-230 Presence bit |
| FND-002 | low | No zone-side test that a filtered requirement disables the step (deadlock or urgency) | FR-230, FR-232 |
| FND-003 | low | FR-230-AC-51 (STD-526) presence forms stay refused, with no open ticket; the test comment reads STD-526 as the refusal | FR-230-AC-51 |

## Verdict

Mergeable. All three findings are low; FND-001 and FND-002 are tests to add in this PR.

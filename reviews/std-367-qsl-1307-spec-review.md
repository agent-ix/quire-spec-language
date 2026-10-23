---
id: SR-16338
title: "spec-review of specify predicate joins (quire-spec-language PR 1307)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7985372d; spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md; spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md; spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
---

# Spec review of specify predicate joins (quire-spec-language PR 1307)

## Summary

Ticket: STD-367. One spec-review pass over `git diff origin/main...7985372d`: standing rule (no drop, narrowing or deferral), testable ACs with hand-derived vectors, id uniqueness across all open PR heads, QSpec name cross-check, mergeability, and the repo gate.

Examined, clean:

- FR-206-AC-11 derived by hand: labels sorted a<c, `x and not y` gives x=a, y=c, so `join(P)` is enabled only where a is completed and c running; `[c,a]` and `[a,c]` give the same protocol; b is not an argument.
- FR-218-AC-31 names match QSpec FR-146 (`missing-name`, `operator-ineligible`, `unproved-decrease`, `missing-measure`); `[c,a,c]` counts labels once.
- Ids FR-206-AC-11, FR-216-AC-8, FR-218-AC-31, FR-383-AC-22 are on no other open head.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Mergeable after origin/main is merged: FR-383 and TC-1061 conflict with main, and FR-218, FR-383, TC-1061 and TC-1065 conflict with open #1337 (STD-390). No defect in the delta.

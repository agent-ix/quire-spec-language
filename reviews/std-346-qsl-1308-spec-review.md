---
id: SR-16339
title: "spec-review of FR-449 derives the control-to-temporal activation map from the protocol nodes (quire-spec-language PR 1308)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@89a5fc5f; spec/functional/FR-054-publish-control-temporal-activation-map.md; spec/functional/FR-449-derive-the-control-to-temporal-activation-map-from-the-protocol-nodes.md; spec/test-cases/TC-1194-the-activation-map-is-derived-from-the-protocol-nodes.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-054
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-449
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1194
    type: references
---

# Spec review of FR-449 derives the control-to-temporal activation map from the protocol nodes (quire-spec-language PR 1308)

## Summary

Ticket: STD-346. One spec-review pass over `git diff origin/main...89a5fc5f`: standing rule (no drop, narrowing or deferral), testable ACs with hand-derived vectors, id uniqueness across all open PR heads, QSpec name cross-check, mergeability, and the repo gate.

Examined, clean:

- FR-449-AC-1..4 hand-checked against FR-218 (a target named by two requirements is recorded once) and FR-383 (the `requirements` set holds the temporal clause nodes); rows in ascending node-id digest order.
- Ids FR-449 and TC-1194 are on no other open head.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The ADR-012 row keeps the label "deferred to #210 by ADR-010 §8" while its new text says the map "does not wait on #210". The row contradicts itself and carries deferral wording; rename the row to state the behaviour (the map is derived by FR-449) and drop the deferral. | spec/decisions/ADR-012-semantic-family-extension-contracts.md | wrong-requirement |
| FND-002 | medium | FR-054 stays as a RETIRED file whose ACs read "Retired", and it states its own deletion schedule ("deleted in the change that deletes the /3 producer"). Spec content is status and changelog language, and the retirement itself is left to a later code change. Retire it in this PR (delete FR-054 and TC-143, point FR-053/ADR rows at FR-449) or state it without a schedule. | spec/functional/FR-054-publish-control-temporal-activation-map.md | wrong-requirement |
| FND-003 | medium | FR-449, FR-053, FR-054, ADR-011 and ADR-012 cite QSpec FR-464 "Activation relation", FR-464-AC-3/AC-13 and FR-479-AC-4/AC-5. Those exist only in open QSpec #393 (STD-346); `make check-spec-validation` passes against QSpec main without them, so the gate does not catch it. #1308 cannot land before #393. | spec/functional/FR-449-derive-the-control-to-temporal-activation-map-from-the-protocol-nodes.md | wrong-requirement |

## Verdict

Not mergeable until FND-001..003 are dispositioned; FND-003 needs QSpec #393 merged first. Merge-tree against main and against every open PR touching the same files is clean.

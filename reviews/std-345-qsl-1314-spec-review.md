---
id: SR-16344
title: "spec-review of located_json is deleted with model_source; M-6c paths are deleted per slice (quire-spec-language PR 1314)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@45ddff19; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md; spec/decisions/ADR-016-state-model-finite-execution-mapping.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: references
---

# Spec review of located_json is deleted with model_source; M-6c paths are deleted per slice (quire-spec-language PR 1314)

## Summary

Ticket: STD-345. One spec-review pass over `git diff origin/main...45ddff19`: standing rule (no drop, narrowing or deferral), testable ACs with hand-derived vectors, id uniqueness across all open PR heads, QSpec name cross-check, mergeability, and the repo gate.

Examined, clean:

- ADR-011 X-2, the §6.1 F order and module row, the dependency row, and M-6c agree that `located_json` is deleted with `model_source`; no stale "stays in the root crate" or "moves in a follow-up" remains (git grep over the head).
- ADR-012 §15.8 step 2 and ADR-016 G-6 now read "each path is deleted in the slice that makes it unreachable"; `formal_source` stays SEAM-2 / M-6e as the ticket states.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Mergeable after origin/main is merged: ADR-011 conflicts with main, and ADR-011/ADR-012 conflict textually with open #1355/#1357/#1360/#1369/#1370/#1372/#1374. No defect in the delta.

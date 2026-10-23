---
id: SR-16318
title: "spec-review of trap closure is FR-170 replay, FR-282-AC-4 moves to FR-170-AC-8 (quire-spec-language PR 1331)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b10cf4fc04; spec/functional/FR-170-replay-state-graph-evidence.md; spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md; spec/test-cases/TC-595-replay-state-graph-evidence.md; spec/test-cases/TC-764-certificate-checkers-accept-genuine-certificates-and-reject-altered.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-170
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-595
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-764
    type: references
---

# Spec review of trap closure is FR-170 replay, FR-282-AC-4 moves to FR-170-AC-8 (quire-spec-language PR 1331)

## Summary

Ticket: STD-374. One spec-review pass over `git diff origin/main...b10cf4fc04` (agent-ix/quire-spec-language#1331), checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (derived by hand), id uniqueness across open PR heads, and mergeability. Gate: `QSPEC_DIR=<worktree of QSpec origin/main> make check-spec-validation` exit 0.

Examined, clean:

- FR-282-AC-4's obligation is moved, not dropped: FR-170-AC-8 keeps the accepted-trap case at `Lost` (closure `{Lost}`, node count 1, `refuted` `closed-scope` with no certificate) and the reached-target case, now `inconclusive` `Verdicts` naming the first target node by state-key digest.
- Hand-derived against ADR-022 §7.2 (Start -play-> Mid, Mid -lose-> Lost, Mid -win-> Won): the closure of `Lost` is `{Lost}`, holding no `Won`; a stem ending at `Mid` has closure `{Mid, Lost, Won}` whose first target node in breadth-first order is `Won`. Both match the AC.
- ADR-029 CB-2 and RU-2 exist and name closure checks "such as ADR-022's trap re-exploration" inside the qualified core, consistent with the new FR-282 paragraph. TC-764 steps renumbered and its Scope line updated; no other artifact cites TC-764 step 4.
- No deferral or follow-up wording in the diff.
- FR-170-AC-8 is unused on main and other heads.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Mergeable by content: no defects in this PR's own delta.

Conflicts with open head #1353 (STD-388) in FR-170 (textual); clean against origin/main.

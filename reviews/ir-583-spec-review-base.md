---
id: SR-15515
title: "Base checklist review of AD-001 IR-583 blocker update"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2c4cb91bf5a9c2b75986c6e0fd96779c1b2a0792; spec/assurance/AD-001-shared-evaluation-core-interface.md; PR #1224"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: reviews
---
# SR-15515: Base checklist review of AD-001 IR-583 blocker update

## Summary

Ticket: IR-583. Base checklist pass over the PR #1224 diff (`git diff origin/main...HEAD`) of AD-001, including the Hash / Digest / Pin antipattern gate. The diff correctly removes the pinned QSL `origin/main` commit SHA, but it replaces it with dated re-check records and keeps a hand-copied snapshot of Linear workflow state, owner and blocker edges inside the design. That snapshot is the tracking record that drifted and caused this PR, and it had drifted again by review time: the table says IR-583 is "In Progress / peter", while Linear now reports IR-583 Done.

Examined units:

- AD-001 System Boundary (line 39-40): "The no_std + alloc implementation is IR-590, after S1-S5. IR-269 remains a direct blocker of S1 and therefore an indirect predecessor of S6."
- AD-001 Current source boundary (line 52): "The source boundary was rechecked against QSL main on 2026-10-22."
- AD-001 Staged ownership and blockers intro (line 146): "Linear was re-read on 2026-10-22. The following dependency states are the structured blocker graph".
- AD-001 Staged ownership table, IR-583 and S1-S6 rows (lines 150-158).
- AD-001 Staged ownership prose (lines 160-166).
- AD-001 Decisions, checked-term separation bullet (lines 208-210).
- AD-001 Risks, IR-269 bullet (lines 231-233): "The structured Linear edges, rather than this design's earlier 2026-10-10 table, decide readiness."
- reviews/SR-15514 (context_only): the earlier integrity review committed on this PR.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | AD-001 keeps a hand-copied mirror of Linear state (workflow state, owner, blocker lists) plus "re-read on 2026-10-22" / "rechecked against QSL main on 2026-10-22" provenance records. This is a tracking record: it drifted once (the reason for this PR) and was already stale at review (IR-583 shown In Progress, Linear says Done). Line 232 itself says the Linear edges, not this table, decide readiness. Remove the state/owner column, the dated re-check sentences and the blocker-list mirror; keep only the design-level slice ordering rationale and cite the Linear tickets by id. | spec/assurance/AD-001-shared-evaluation-core-interface.md:52; spec/assurance/AD-001-shared-evaluation-core-interface.md:146; spec/assurance/AD-001-shared-evaluation-core-interface.md:150-158; spec/assurance/AD-001-shared-evaluation-core-interface.md:232 | wrong-requirement |

## Verdict

One high finding under the Hash / Digest / Pin antipattern gate. The edge content itself is accurate against Linear (see the dependency review), the SHA pin removal is correct, and no requirement, AC or verification method changed, so EARS and test-coverage rules do not apply to this diff. PR #1224 is already merged to main, so FND-001 has to be fixed forward under IR-583 on main.

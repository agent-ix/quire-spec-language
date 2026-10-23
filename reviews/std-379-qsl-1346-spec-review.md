---
id: SR-16326
title: "spec-review of FR-169 state-graph certificate carrying the other form's list (quire-spec-language PR 1346)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@97cac77b; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md; spec/test-cases/TC-644-state-graph-certificates-that-do-not-hold-are-rejected.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-644
    type: references
---

# Spec review of FR-169 state-graph certificate carrying the other form's list (quire-spec-language PR 1346)

## Summary

Ticket: STD-379. One spec-review pass over `git diff refs/rev/1311...97cac77b` (stacked on #1311), checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-repo names, mergeability and gates.

Gate: `QSPEC_DIR=<QSpec origin/main 0eef131f> make check-spec-validation`: exit 0, 19 warnings (none in the touched files).

Examined, clean:

- The PR is stacked on #1311 (STD-358), as its body says; the own delta is `git diff refs/rev/1311...refs/rev/1346` (FR-169 and TC-644, 2 files). The tuple shapes in AC-15 match FR-169's `ranks: (closure index, d)` and `counts: (closure index, count, order)`.
- Ids: FR-169-AC-15 appears on no other open PR head (AC-13 is #1327, AC-14 is #1311). No deferral language.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | AC-15 cannot be satisfied under FR-169's own locus rules. The Types paragraph defines `Entry { index }` as "a `ranks` or `counts` entry's closure index that names no closure state", and the Malformed bullet says a rank or count entry rejects at the `ModelState` of the closure state its index names, and at `Entry { index }` only when the index names none. AC-15 adds the entries `(0, 1, 0)` and `(0, 0)` to certificates whose closure holds 9 states (or `H`'s nodes), so index 0 names a state, yet it demands `Entry { index: 0 }`. The new sentence ("rejects `Malformed` at `Entry { index }` with that entry's closure index") contradicts the two rules it sits between. Fix: either give the other-form entry the `ModelState` locus, or define a distinct locus for a wrong-form entry and update the `Entry` definition. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:178-183, 263 | wrong-requirement |
| FND-002 | low | AC-15's last sentence (the engine's certificates, with an empty other list, are accepted) has no TC-644 step or expected result, so the accepted half of the rule is unverified by the new TC step. Fix: add the accepted case to step 7 or cite TC-643's step for it. | spec/test-cases/TC-644-state-graph-certificates-that-do-not-hold-are-rejected.md:36-38, 60-62 | correct-requirement-no-evidence |

## Verdict

Not mergeable: FND-001 (high) block the PR.

Conflicts with main: the PR inherits #1311's conflict in FR-338 (`git merge-tree` of #1311 with main conflicts; the own delta is clean). #1311 also conflicts with open #1327, #1354 and #1377, so #1346 does too through it. Not mergeable until #1311 is rebased and merged and #1346 is retargeted to main.

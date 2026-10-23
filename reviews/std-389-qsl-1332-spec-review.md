---
id: SR-16319
title: "spec-review of every integer-division fixture type-checks (quire-spec-language PR 1332)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d03913dfc3; spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md; spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md; spec/functional/FR-179-check-a-step-relation-over-reachable-transitions.md; spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md; spec/test-cases/TC-847-an-undefined-letter-fails-a-supplied-trace.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-328
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-331
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-847
    type: references
---

# Spec review of every integer-division fixture type-checks (quire-spec-language PR 1332)

## Summary

Ticket: STD-389. One spec-review pass over `git diff origin/main...d03913dfc3` (agent-ix/quire-spec-language#1332), checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (derived by hand), id uniqueness across open PR heads, and mergeability. Gate: `QSPEC_DIR=<worktree of QSpec origin/main> make check-spec-validation` exit 0.

Examined, clean:

- Truth patterns rederived by hand for the `mod` fixtures: FR-124-AC-10 `3 mod (2 - v) = 1 or v = 3` is true at 0 and 3, false at 1, undefined at 2 (as the original `2 / (2 - v) <= 1`); FR-125-AC-6 and FR-126-AC-9 `6 mod (2 - v) >= 0` defined at 0, 1 and undefined at 2, and `6 mod (2 - v) = 0 and v = 1` true only at 1 (the old `= 6` letter); FR-327-AC-5 `3 mod (2 - v) = 0` false at 0, true at 1; FR-328-AC-5 `= 2` never true; FR-329-AC-6 true at 1, undefined at 2; FR-179-AC-5 `3 mod (2 - 2 * k) > y.k` true at (0,0), false at (0,1), undefined for k=1; FR-181-AC-5 `1 mod (1 - l) = 0 and l = 0` true at l=0, undefined at l=1; FR-239-AC-5 and FR-160-AC-5 `>= 0` defined at 0 and 1, undefined at 2. All keep the original outcomes.
- The `div` rewrites (FR-166-AC-5, FR-168-AC-5, FR-170-AC-5, ADR-022 RU-5) rely on the §7.1 subject selecting an integer-division law; FR-168-AC-11's fixture on the same subject already writes `2 div`, so that holds.
- No QSpec PR names are cited.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | scope-deferral: STD-389's deliverable is every integer-division fixture, and the PR body says FR-168-AC-12 and AC-13 are "fixed in #1353 so the two PRs do not conflict" and FR-128-AC-6 and TC-539 step 4 are "left to #1342". After this PR, FR-168-AC-13 still divides integers with `/` (`2 / (2 - self...` in its state constraint) on this head and on origin/main, so the ticket's own scope is split across PRs, which the standing rule bars. #1342 has since merged and origin/main now carries `mod` in FR-128-AC-6/AC-7, FR-168-AC-12, TC-537 step 4, TC-539 and TC-1142, so only AC-13 remains, in open #1353. Fix: include AC-13 here (merging with #1353 by union) or cite #1353 as the owner in the PR and Linear and merge it first. | spec/functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md | missing-requirement |

## Verdict

Not mergeable: FND-001 (high) block the PR.

Conflicts with origin/main: FR-126 (FR-126-AC-9; main has since changed that row and its neighbours), so the PR needs a rebase that keeps main's `6 mod (3 - c.value)` text. Against open heads: clean. The sibling #1353 (STD-388) edits FR-168, FR-170 and TC-612 and merge-trees clean with this PR.

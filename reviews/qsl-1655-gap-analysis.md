---
id: SR-20261105101004
title: "Gap analysis — QSL-1655/1665 IEEE negation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2370; changed qsl-semantics, qsl-eval, and qsl-replay negation paths; FR-397-AC-21/27"
review_set: subset
---

## Summary

Ticket: QSL-1655 / QSL-1665. I mapped the changed negation checker, lowering, evaluator, and tests to FR-397-AC-21 and AC-27. The matrix engines timed out, so the findings use a manual mapping of the changed behavior and do not claim a whole-repository matrix result.

## Verdict

**CONDITIONAL** — the new tests carry both changed criterion IDs, but the state-clause outcome required by FR-397-AC-21 is not verified.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The change adds checked function-body tests for IEEE negation, but FR-397-AC-21 also requires a state clause containing `-qnan != -qnan` to evaluate to `Completed(true)`. No test exercises that clause path, leaving part of the criterion without behavioral evidence. Add an AC-tagged clause-set test for the specified result. | spec/functional/FR-397-select-the-ieee-profile-of-a-package-that-declares-or-evaluates-a-float.md:339; qsl-eval/tests/it/ieee_node_charges.rs:263-410 | correct-requirement-no-evidence |

## Coverage

- `quoin matrix --repo /home/peter/dev/worktrees/qsl-1655-1665-negation --json` timed out after 90 seconds (exit 124).
- `quire matrix --scope /home/peter/dev/worktrees/qsl-1655-1665-negation --format json` timed out after 30 seconds (exit 124).
- Matrix static/evidence status counts are unavailable. Manual fallback scoped to the changed behavior: FR-397-AC-21 and AC-27 both have new trace tags; 2 criteria examined, 2 tagged in the changed tests, 0 untagged in this scoped mapping. This is not a whole-repository matrix count.
- The changed source behavior maps to the owned FR-397 criteria; no new unowned behavior or source/test stub was found in the changed paths.
- Plan completion: not assessed.
- Semantic intent review: not requested and skipped.

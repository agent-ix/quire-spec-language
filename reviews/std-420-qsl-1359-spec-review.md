---
id: SR-16335
title: "spec-review of the FR-109-AC-10 renumbering (quire-spec-language PR 1359)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@037a8ce34; spec/functional/FR-109; spec/test-cases/TC-514; qsl-replay/src/spine/clause/tests/frame.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: references
---

# Spec-review of the FR-109-AC-10 renumbering (quire-spec-language PR 1359)

## Summary

Ticket: STD-420. One spec-review pass over `git diff origin/main...037a8ce34`, checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-spec names, and mergeability against main.

Examined, clean:

- After the change main-equivalent references are consistent: FR-109 rows AC-9 (Ratio) and AC-10 (Frame run), TC-468 cites AC-9 for Ratio, TC-514 scope and step 7 note cite AC-10, and `frame.rs` is tagged AC-10. A search of every `.md` and `.rs` in main finds no other reference to the Frame row under AC-9.

Merge status: origin/main d0a756b2b is not an ancestor (47 behind) but `git merge-tree` is clean; no conflict with the other open PRs.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

**Mergeable.** Clean. Gate `QSPEC_DIR=<QSpec main> make check-spec-validation`: exit 0, 19 warnings (baseline), none on touched files. The trace-tag edit in `frame.rs` is one string; the PR reports `cargo test -p qsl-replay --lib frame` 59 passed, which I did not rerun. FR-109-AC-10 is added by no other open head, and the only other open mention of FR-109-AC-9 (#1332, prose) means the Ratio row, which keeps the id.

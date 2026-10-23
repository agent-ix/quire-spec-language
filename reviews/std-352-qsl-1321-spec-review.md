---
id: SR-16312
title: "spec-review of byte-only intake limits and FR-027-AC-13 (quire-spec-language PR 1321)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@07b2491260; spec/functional/FR-026-run-standalone-native-workflow.md; spec/functional/FR-027-export-compiled-native-package.md; spec/functional/FR-028-run-selected-native-package.md; spec/test-cases/TC-105-standalone-package-export.md; spec/test-cases/TC-106-selected-package-execution.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-028
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-105
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-106
    type: references
---

# Spec review of byte-only intake limits and FR-027-AC-13 (quire-spec-language PR 1321)

## Summary

Ticket: STD-352. One spec-review pass over `git diff origin/main...07b2491260` (agent-ix/quire-spec-language#1321), checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (derived by hand), id uniqueness across open PR heads, and mergeability. Gate: `QSPEC_DIR=<worktree of QSpec origin/main> make check-spec-validation` exit 0.

Examined, clean:

- The file-count contradiction is resolved: FR-026 Outputs, Description, Behavior and FR-027/FR-028 now agree on two byte limits (`request_bytes`, `dependent_bytes`) and no file limit. FR-028's "64-file" preflight is removed with it.
- FR-027-AC-13 vectors: a request whose program and models total below `dependent_bytes` and whose one library source brings the total to exactly one byte over refuses naming the setting and its configured value; raising the limit to that total compiles it (the boundary is `>`, consistent with FR-026-AC-4). 65 small library files at default limits (8 MiB) compile: the old ceiling was 64 files, so 65 is the right probe.
- No deferral, narrowing or follow-up wording in the diff.
- No QSpec PR names are cited.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The PR does not merge with origin/main. Main (STD-384, #1345) rewrote the same Behavior sentence to "Library source files count toward `dependent_bytes`. Each unit of an array `program` counts as a program source." and added FR-027-AC-14 beside this PR's AC-13. `git merge-tree` conflicts in FR-027 at both places. Resolving to this PR's side silently drops the array-`program` counting clause (FR-480), and FR-026's new "every dependent file counts ... whatever its role" sentence then omits the array units too. #1303 (STD-343) also conflicts: FR-026, TC-105 and a modify/delete on FR-028 and TC-106. Fix: rebase onto origin/main keeping main's array-program sentence, and land in the order #1303 states (#1321, #1326, #1313, then #1303). | spec/functional/FR-027-export-compiled-native-package.md | wrong-requirement |

## Verdict

Mergeable after FND-001 (medium) are fixed or dispositioned by the lead.

Conflicts with origin/main: FR-027 (2 hunks). Conflicts with open heads: #1303 (4 conflicts). Clean against #1326, which carries the same TC-105 text.

---
id: SR-16315
title: "spec-review of FR-026/FR-027 as the command library contract (quire-spec-language PR 1326)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a8d5ef0c31; spec/functional/FR-010-report-native-outcomes.md; spec/functional/FR-026-run-standalone-native-workflow.md; spec/functional/FR-027-export-compiled-native-package.md; spec/functional/FR-287-reach-qsl-through-the-driver-cli.md; spec/functional/FR-028-run-selected-native-package.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-312
    type: references
---

# Spec review of FR-026/FR-027 as the command library contract (quire-spec-language PR 1326)

## Summary

Ticket: STD-355. One spec-review pass over `git diff origin/main...a8d5ef0c31` (agent-ix/quire-spec-language#1326), checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (derived by hand), id uniqueness across open PR heads, and mergeability. Gate: `QSPEC_DIR=<worktree of QSpec origin/main> make check-spec-validation` exit 0.

Examined, clean:

- The in-process restatements keep each AC's observable outcome (document or package bytes, FR-285 exit code, "no result document" in place of "empty stdout") and add "writes nothing to the process streams". FR-026-AC-5's working-directory check becomes equal outcomes from two working directories.
- FR-100-AC-5 keeps its intentional `1 / x > 0` `ill_typed` example.
- No QSpec PR names are cited.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | scope-deferral: the PR deletes FR-010-AC-3 (invalid invocation exits 20) and FR-010-AC-10 (output write failure exits 30), drops FR-026-AC-5's "command-specific arity errors precede I/O", the `--request-bytes`/`--dependent-bytes` option spelling, broken-pipe and stream-failure statements, and FR-010-AC-6's invalid OS encoding of commands and labels, on the ground that these are "driver behaviour". The destination is FR-287 Overlap, which gains prose only: FR-287 has two ACs (AC-1 target list, AC-2 in-process `command`) and none for usage, arity before I/O, option spelling, stream failure exit 30 or broken pipes, and quire-driver's spec has no AC for them (local clone, last commit 2026-10-01; searched spec/ for arity, usage error, broken pipe and the option names). The criteria are removed, not moved. Standing rule: a dropped criterion is a blocking finding even though the PR body flags it as a lead default. Fix: add testable ACs for each moved criterion in the owning spec (FR-287 or the driver's FR) in this PR, or open the STD ticket for the driver spec and cite it. | spec/functional/FR-010-report-native-outcomes.md | missing-requirement |
| FND-002 | low | The reworded AC introduces a validator warning that main does not have in this file: `ac:vague-response` "vague outcome verb `process`" at FR-028 line 56 (`make check-spec-validation` exit 0, 27 warning lines against 26 on every other head). The trigger is "in process". Fix: reword without that phrase, for example "through `command` called as a library function". | spec/functional/FR-028-run-selected-native-package.md | wrong-requirement |

## Verdict

Not mergeable: FND-001 (high) block the PR.

Conflicts with open heads: #1303 (10 files) and #1313 (FR-100, TC-450); clean against origin/main. #1303 lists #1326 before it in its merge order.

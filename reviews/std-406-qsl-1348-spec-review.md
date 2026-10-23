---
id: SR-16327
title: "spec-review of Plan-013 WASM text removal (quire-spec-language PR 1348)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@162e9ada; plan/Plan-013-complete-v1-delivery/plan.md; plan/Plan-013-complete-v1-delivery/index.md; plan/Plan-013-complete-v1-delivery/tasks/Task-052-native-tooling.md; plan/Plan-013-complete-v1-delivery/tasks/Task-053-wasm-parity.md; plan/Plan-013-complete-v1-delivery/tasks/Task-054-complete-v1-qualification.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-013
    type: references
---

# Spec review of Plan-013 WASM text removal (quire-spec-language PR 1348)

## Summary

Ticket: STD-406. One spec-review pass over `git diff origin/main...162e9ada`, checked for the no-drop/no-narrowing/no-deferral rule, testable vectors (key vectors derived by hand), id uniqueness across all open PR heads, cross-repo names, mergeability and gates.

Gate: `QSPEC_DIR=<QSpec origin/main 0eef131f> make check-spec-validation`: exit 0, 19 warnings (none in the touched files).

Examined, clean:

- A removal required by the owner correction in STD-406 (STD-22 follow-through); it narrows nothing the owner wants. `git grep -i wasm` over the PR head outside `reviews/` and `Cargo.lock` finds only the ADR-010 observed-baseline row for quire-wasm #6, which the ticket names as neutral. Task-053, V1-TOOL-009, FR-308/TC-228 (QSpec) and the A07/A08 tracks are gone with no dangling reference in `plan/`.
- The allocation table has 82 `V1-*` rows after the PR (83 on main). The "83 to 82" edits in the dependency graph, scope, evidence counts and Task-054 are consistent. Task-054 now depends on Task-052 and takes track A07; Task-052's last subtask says "merge before Task-054".
- The "FR-308" and "TC-228" strings in `qsl-*` sources and tests are QSL's own protocol FR-308 and an unrelated TC-228, not the deleted QSpec ids.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The new layer counts (28 implementation missing, 28 verification missing, 30 integration missing, 82 qualification missing) assume V1-TOOL-009 sat in the "missing" bucket of every layer. The "accepted central inventory snapshot" is not in either repo (QSpec `docs/v1-capability-inventory.md` has no per-layer status), so the assumption cannot be checked from the tree. The PR body flags it as a lead default. Fix: cite the snapshot, or state the per-layer status of the removed row. | plan/Plan-013-complete-v1-delivery/plan.md:114-117 | wrong-requirement |

## Verdict

Mergeable: no defect that blocks the PR.

Conflicts with main: none (clean merge-tree; clean with open #1303 on plan.md). Mergeable with the low finding recorded.

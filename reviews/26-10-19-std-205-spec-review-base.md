---
id: SR-13086
title: "Spec review of PR #1030 commit 439d37769: FR-156-AC-3 Ring wrap-around"
type: SpecReview
scope: "agent-ix/quire-spec-language@439d37769; commit 439d37769: FR-156-AC-3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-156
    type: reviews
---

## Summary

Ticket: STD-205. Base spec review of commit 439d37769 on PR #1030 (head 439d37769): FR-156-AC-3's `Ring` post clause. The new clause `(pre(self.v) < 2 and self.v = pre(self.v) + 1) or (pre(self.v) = 2 and self.v = 0)` gives the same transition relation as the old `mod 3` (0->1->2->0), so the AC's ample sets, C3 behaviour and six-step prefix are unchanged. Probe on PR #1024's head (a2ad83a6b), on the `Health` test package's `failures: Int[0, 1]` field with the same two forms: `self.failures = (pre(self.failures) + 1) mod 2` is refused `IllTyped(TypeMismatch)` at S3, and the explicit wrap-around type-checks; the ConfigVersion fixture's `Cycles` clause, the same form over `Int[0, 1000]`, also type-checks and runs. FR-149 states no narrowing conversion, so the explicit form is the fix STD-205 asks for. STD-205's "vector that the engine accepts the corrected unit" is TC-577, which runs the unit; no separate vector is needed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-156-AC-3 | spec/functional/FR-156-expand-ample-sets-with-the-breadth-first-proviso.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. The Ring unit's post clause now type-checks and keeps the AC's behaviour.

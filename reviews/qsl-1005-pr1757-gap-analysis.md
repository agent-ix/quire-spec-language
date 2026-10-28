---
id: SR-1757002
title: "PR #1757 (QSL-1005c) gap analysis: FR-244-AC-5/6, FR-245-AC-8/9, FR-430-AC-8 (TC-699, TC-700)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-analyze/tests/it/timed_refinement_certificate.rs, timed_refinement.rs at b8392b9f4"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: reviews
---

# PR #1757 gap analysis

## Summary

Ticket: QSL-1005 (1005c). Plan completion: not assessed.

### Covered

- **FR-244-AC-5:**
  - the TReq certificate in refinement form, accepted, with two runs giving equal certificates;
  - `dead_undo`'s locked `undo` post-state as an entry, entered by a failing edge with no `taken`.
- **FR-244-AC-6:** Init3's three initial nodes, index 1 a `locked` entry.
- **FR-245-AC-8:**
  - `positions` empty, or no part, is `Malformed`;
  - a moved target and a `Stutter` `taken` are `PostDiffers`;
  - a passing step into a locked entry is `PostDiffers`;
  - the entry's mark removed is `StepFails`;
  - a forged initial position is `InitialMismatch`;
  - an evaluation meter of 0 is `Stopped(EvaluationMeter)`;
  - another abstract package is `stale_dependency`.
- **FR-245-AC-9:**
  - Init3: the mark removed or added is `InitialMismatch`, and a replaced or dropped initial is `InitialMissing`;
  - Init3L: `admitted` from the locked node is `WitnessFails`;
  - without the invariant, the widened locked initial is `BadState`.
- **FR-430-AC-8:** proved end to end through `analyze` once the checker accepts.

## Verdict

Mergeable. There is one low gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `BadState` rule for a locked entry whose forward closure is not time-locked is tested at an initial node only (Init3 without the invariant). No test forges it at a step-level entry, entered by a failing `check_step`. My probe did: `dead_undo`'s certificate checked on `live_undo` with the entry widened gives `BadState`. That case should be pinned. | qsl-analyze/tests/it/timed_refinement_certificate.rs:460-489 |

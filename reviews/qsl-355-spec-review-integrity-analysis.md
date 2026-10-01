---
id: SR-948
title: "QSL-355 spec review (integrity) of PR 556: retired FR-019-AC-5 and residual pin prose"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@bd6ceb715d1d991272922ffe779a33e43b84007e; spec/functional/FR-019, FR-020, FR-021; spec/native-packages/tests.md; spec/test-cases/TC-080, TC-091; docs/native-linked-packages.md; spec/decisions/ADR-013 OBS-022 and O-23; spec/decisions/ADR-010 (context); spec/functional/FR-052 and spec/native-temporal/tests.md (retired-AC precedent); spec/functional/FR-007:93 (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: reviews
---
## Summary

Ticket: QSL-355. PR: quire-spec-language#556 at bd6ceb71.

- **Retire in place fits the repo.** FR-052-AC-1..8, FR-051 and FR-053 use the
  same `**RETIRED** … | Retired |` row. The quoin spec-review checklist requires
  sequential ids, so deleting the row would leave a gap in AC-5..9, and
  renumbering would churn every downstream trace tag. The slot is
  load-bearing, so the marker is not tracking ceremony. The row keeps one
  sentence of reason, matching FR-052. Accepted.
- **FR-020-AC-3, FR-021 inputs and AC-3, TC-080 and TC-091** now say "semantic
  selection/selector". They agree with the remaining wire record in
  `docs/native-linked-packages.md` and the schema.
- **ADR-013 OBS-022** now states the result (no revision literals on the wire,
  none checked by `src/package`). This resolves SR-934 FND-002 (QSL-354), which
  found the deletion had no owner.
- **ADR-010 left unchanged: correct.** Its Context says it "describes what
  exists at the revisions below" (all origin/main 2026-09-19). OBS-022/DA-14
  there are dated observations of `view.rs:39,46` at that time. Editing them
  would make the observation record false. The current decision lives in
  ADR-013.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The retired matrix row's Status is `❌ Retired`. In quoin's Status vocabulary `❌` means failed and `⛔` means retired, so the row reads as a failing criterion. The PR copies the repo's native-temporal precedent, which has the same defect. Fix: use `⛔ Retired` here (and in spec/native-temporal/tests.md:101-104,134-135). | spec/native-packages/tests.md:25 |
| FND-002 | low | FR-007's Status still says "The adopted standard pin is e897f810…". After this PR nothing in code uses that revision. The line is a version-pin record that guards nothing. Pre-existing, outside the diff, but it is the pin this PR deletes from the wire. Fix: delete the line. Similar pre-existing provenance prose: spec/spec.md:371, IT-005:23,29, README.md:342-348, docs/spec-workflow.md:208,236. | spec/functional/FR-007-validate-runtime-inputs.md:93 |

## Verdict

The spec side matches the code. Retire-in-place is the right mechanism here.
Two low findings: a wrong status glyph, and one leftover pin line in FR-007.
Neither blocks the deletion. Both are quick to fix in this PR.

---
id: SR-1754002
title: "gap analysis of PR #1754 (QSL-580t): general presence, PR-01/PR-05"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1754 at 732a1c044 against FR-230-AC-49..51, STD-526 presence ruling (PR-01, PR-05; PR-02 dropped; PR-03 later), STD-542 / QSpec #485 FR-415-AC-21"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1754002: gap analysis of PR #1754 (QSL-580t)

## Summary

Ticket: QSL-580. Head: 732a1c044.

Covered:
- PR-01 (another object's place, in a time invariant) at S3 by `another_objects_presence_is_a_data_read`, which is red at the head (SR-1754001 FND-001).
- PR-01 in eval by `another_objects_presence_reads_its_bit`.
- PR-05 (a non-object owner refuses `type-mismatch`).

Not covered:
- **FND-001.** The quantified-presence refusal assertion (`forall(c in allInstances<Config::Call>(Config::calls) : present(c.v))`) was deleted. In that fixture `Config::calls` does not resolve: the forall now refuses `MissingSegment` (the same for `c.late`), where the base refused `PresenceNotChecked` at `present(c.v)`. Nothing pins PR-03 ("comes later") as refused. The `Located` skip ignores whether the owner is a binder, so `present(c.v)` may now be admitted untested.
- **FND-002.** No test covers a union-case owner (`present(r.state.Armed)` of another object), which the code supports (`PlacePresence { case }`, `place_presences` for `[Field, Union]`).
- **FND-003.** No test covers urgency over another object's presence. My probe shows it correct: urgent exactly where the peer's `v` is live.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The PR-03 refusal assertion was deleted, the fixture cannot resolve the population, and quantified presence is pinned neither refused nor admitted | FR-230-AC-51, STD-526 PR-03 |
| FND-002 | low | No test of another object's union-case presence `present(r.f.c)` | FR-230-AC-51 |
| FND-003 | low | No test of urgency over another object's presence (probe: correct) | FR-230-AC-51 |

## Verdict

Low findings only; the verdict follows SR-1754001 (changes requested).

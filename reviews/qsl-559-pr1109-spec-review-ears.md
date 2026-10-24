---
id: SR-13584
title: 'Spec review of PR #1109: protocol wait causes, fences, ordinals and footprints, EARS'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@389b22c99; git diff origin/main...389b22c99 (PR #1109, branch qsl-559-protocol-spec): the requirement statements added or changed in spec/functional/FR-205, FR-206, FR-211, FR-216 and FR-218; tickets QSL-559, QSL-562'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-206
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-211
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
---

## Summary

Tickets: QSL-559, QSL-562. PR quire-spec-language#1109, head 389b22c99. This file checks EARS conformance of every requirement statement the diff adds or changes. `make check-spec-validation`'s grammar pass reports no warning in a changed file. Most new statements are ubiquitous SHALLs with a named subject, so they conform. FR-216's new fork, branch-end, `finish` and membership bullets follow the file's existing `R`/`W` SHALL pattern, and the FR-216 visibility bullet is unchanged in form. The four findings are statements that carry normative content without a SHALL, or that put several SHALLs on a step rather than on the system.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-206 statement (fence sentence) | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md | examined |
| FR-206 Behavior (send record bullet) | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md | examined |
| FR-211 Behavior (wait-cause bullets) | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md | examined |
| FR-216 Behavior (event/finish, fork, join, visibility bullets) | spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md | examined |
| FR-218 Channels (qualified-name bullet) | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | 'A `fence` is a step enabled by its memory model's fence rule alone and waits on nothing else' is placed in the requirement statement but has no SHALL and no system subject, so it reads as a description, not an obligation. Fix: 'The protocol system SHALL enable a `fence` step by its memory model's fence rule alone (ADR-027 SE-5).' | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:36 |
| FND-002 | low | 'A thread has one cause, the first that applies.' has no SHALL and gives no order for 'first'. The next bullet holds three SHALLs (HeadBlocked/Displaced/EmptyQueue, FullQueue, replay), with the `receive`, a step, as subject. Fix: 'The model checker SHALL report exactly one wait cause per waiting thread, the first in <stated order>', then one bullet per cause rule, and one for replay. | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:110 |
| FND-003 | low | 'A channel is named by its qualified name, ... wherever a checked clause, a queue, a transition identity or a wait cause names it' has no SHALL. The bullet around it is all SHALLs on S3. Fix: 'S3 SHALL record each checked channel under its qualified name (ADR-027 PS-7), ...'. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:188 |
| FND-004 | low | 'A `send` SHALL bind a send record ...' makes a step the subject. FR-206's other bullets make the protocol system the subject. Fix: 'The protocol system SHALL bind each `send`'s binder to a send record ...'. | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md:77 |

## Verdict

Conforms apart from four low wording findings. None of them blocks merge on its own. The blocking findings are in SR-13583 and SR-13585.

## Dispositions

Round 1, head ccae02a7d. Every new statement now has a SHALL on a named system subject: the protocol system for the fence and the send record, the model checker for the wait-cause bullets (now split into four), and S3 for the qualified channel name. The new FR-206 cancellation bullet conforms. Grammar warnings are unchanged, and none is in a changed file.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

---
id: SR-13589
title: 'Spec review of PR #1111: protocol specs aligned with QSpec PR #302, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@91a8b0938; git diff origin/main...91a8b0938 (PR #1111, branch qsl-adr027-followup, base main 693c16869): spec/decisions/ADR-027-protocol-transition-system-for-parallel.md, spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md, spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md, spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, spec/test-cases/TC-651-protocol-step-kinds.md, spec/test-cases/TC-656-protocol-terminal-states-and-deadlocks.md, spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md; tickets QSL-559, QSL-562'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-027
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-206
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-211
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-218
  type: reviews
---

## Summary

Tickets: QSL-559, QSL-562. PR quire-spec-language#1111, head 91a8b0938, scoped to `git diff origin/main...HEAD` (7 files, +44/-22), spec only. Methods: spec-review/base (SR-13589, this file: checklist, coverage and testability), spec-review/integrity (SR-13590: agreement with QSpec #302, ADR-027 and the code) and spec-review/failure-domain (SR-13591: cause uniqueness and the PS-5 derivations).

Gates on this head: `make check-spec-validation` exits 0, with 17 warnings, none in a changed file. `tools/check-index-completeness.sh` exits 0. Toolchain: quoin 0.28.3, quire 0.36.2 (engine 0.50.2).

Coverage. FR-211-AC-5's new fixtures trace to TC-656 step 5, and its expected results name each new cause. FR-206-AC-4/AC-7 trace to TC-651 step 7, and FR-218-AC-11/AC-17 to TC-1056. IDs and links are unchanged and valid. The changed text has no hash, digest, pin or tracking record, and no changelog, status or owner wording.

The `delivery exactly-once-premise` sentence. It is enough: none of the three new fixtures needs `lose` or `duplicate`, and the sentence rules both out. But it repeats the AC's opening sentence (FND-003).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-206 send bullet | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md | examined |
| FR-206-AC-4 | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md | examined |
| FR-206-AC-7 | spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md | examined |
| FR-211 Outputs and Behavior | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md | examined |
| FR-211-AC-5 | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md | examined |
| FR-218 Channels and eligibility table | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218-AC-11 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| FR-218-AC-17 | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md | examined |
| ADR-027 PS-5, PS-7, ST-3, PD-4 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md | examined |
| TC-651 step 7 | spec/test-cases/TC-651-protocol-step-kinds.md | examined |
| TC-656 step 5 | spec/test-cases/TC-656-protocol-terminal-states-and-deadlocks.md | examined |
| TC-1056 row 30 | spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md | examined |
| FR-205-AC-7 | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md | context_only |
| QSpec FR-429-AC-13 to AC-15, TC-383 TD-08h to TD-08j at f60002b | agent-ix/quire-specification spec/ | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Two clauses of FR-211-AC-5 describe causes in states that are not deadlocks, so the deadlock report never shows them. The fence clause says '`Gate` where the blocked list is computed', and the role clause says 'with a `spawn` enabled the thread is not waiting on the role'. FR-211's Outputs give a blocked list only for a deadlock counterexample's last state, and TC-656 step 5 says only 'run ... and read each deadlock counterexample's blocked threads'. QSpec #302 at f60002b rewrote its matching AC-15 to 'when the cause function is evaluated at that state ... so the state is not terminal', and TD-08j passes the state 'to the cause function directly'. Use the same wording here, and say in TC-656 step 5 that these two clauses call the cause function on a given state. | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:150; spec/test-cases/TC-656-protocol-terminal-states-and-deadlocks.md:26-29,44-50 |
| FND-002 | low | The `CommitClosed` fixture never names its template, yet the result it expects is `CommitClosed{Refund}`. It begins 'A protocol whose compensation template names commit node `Settled`', so `Refund` first appears in the cause, and TC-656 repeats the cause without saying which template is `Refund`. The fixture also gives no effect, trigger or branch layout. It is testable, but two test authors could build different protocols. Name the template `Refund` in the fixture, and say how its name renders: QSpec gives the `template` member as a `<qualified-name>`. | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:150; spec/test-cases/TC-656-protocol-terminal-states-and-deadlocks.md:47-49 |
| FND-003 | low | 'In the next three fixtures any channel has `delivery exactly-once-premise`, so no `lose` or `duplicate` step is taken' repeats the AC's opening sentence, 'Every channel below has `delivery exactly-once-premise` unless stated'. Neither the role fixture nor the fence fixture needs a channel. Drop the sentence, or keep only the opening one. | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:150 |
| FND-004 | low | PS-5's exclusion, 'an entry bound below an inner join of another policy, in a `choice` case or in a `repeat` body is not added', does not say that the `choice` or `repeat` lies inside the joined branch. FR-205-AC-7 puts `parallel P` itself inside a `repeat` body and expects `join(P)` to add `a`. Read literally, the exclusion denies that. The parenthesis 'S3's `Out`' settles it, and so does the S3 code (protocol_slots.rs:1495-1537), but the rule should say 'a `choice` case or `repeat` body inside the branch'. | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:137; spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:116 |

## Verdict

The base checklist passes, and both gates are green. The four findings are all low: two clauses no deadlock report can show (FND-001), a fixture that does not name its template (FND-002), a repeated sentence (FND-003) and a scope phrase a reader could misread (FND-004). The PR's merge blockers are in SR-13590 and SR-13591.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The new concrete `Pay` fixture in FR-211-AC-5 has two defects. (a) It says '`Refund` being the template's `<qualified-name>`', but no spec defines a qualified-name rendering for a template. QSpec FR-429 types `template` as `<qualified-name>` and defines that rendering only for channels, as the protocol's qualified name, `::`, then the handle. By that pattern the name would be `Pay::Refund`, not the bare `Refund`. Either define the rendering in PD-4 (and raise it with QSpec) or drop the clause and expect the name as rendered. (b) Its main sentence does not parse: '... deadlocks after `Fail` with `failed` true activates the registration and `Committed` then occurs'. Read 'deadlocks once `Fail`, with `failed` true, has activated the registration and `Committed` has then occurred'. The trace itself is sound. `Committed` before `Paid` makes no registration (CP-1), and between `Paid` and `Fail` it closes the registration (CP-4). Only `Committed` after activation and before `cattempt` deadlocks, with the root `FinishWaiting`. | spec/functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md:154 |

## Dispositions

Round 1, head d4cd7a4f6, checked against merged QSpec main 4ea88c8. Gates are green, as SR-13590 records.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-211-AC-5 now says 'When the cause function is evaluated at a state ...' for the `spawn` and `fence` clauses, as merged FR-429-AC-15 does, and TC-656 step 5 calls the cause function directly. |
| FND-002 | fixed | The fixture is now ADR-027 §7.1's `Pay`, with template `Refund`, commit node `Committed`, branches `pay` and `settle`, and the expected threads `(0, Refund, 0)` and `(0)`. The template-name rendering and the sentence are recorded as FND-005. |
| FND-003 | fixed | The repeated `exactly-once-premise` sentence is gone. |
| FND-004 | fixed | PS-5 now says 'Within the joined branch ...' and 'a `repeat` that encloses the `parallel` itself does not exclude the branch's entries', so FR-205-AC-7 reads correctly. |

## Dispositions (round 2)

Round 2, head e0cb217fe. The whole spec tree and the PR diff were searched for `<qualified-name>`. The remaining hits are unrelated and predate this PR (FR-101, FR-120, FR-070, FR-218, FR-308 and others); none is in the template or cause wording.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | The fixture no longer claims that `Refund` is the template's `<qualified-name>`, and its main sentence is now an ordered sequence. `Fail` with `failed` true activates the registration and creates `(0, Refund, 0)`. Then `settle` takes `Committed` while that thread is at its first compensation attempt, giving `CommitClosed{Refund}` and the root `FinishWaiting{[(0, Refund, 0)]}`, which is the sound trace. |

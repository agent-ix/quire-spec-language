---
id: SR-2026111015000000000006
title: "ears-conformance review of QSL-1400 source-declared object types (PR 2674)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; PR 2674 origin/main...b029e1e03; spec/functional/FR-569 and the amended statements of FR-104, FR-105, FR-106, FR-390"
review_set: subset
---

## Summary

Ticket: QSL-1400. EARS form of the new and amended requirement statements. FR-569's opening statement ("When a complete-V1 unit declares an object type ... SHALL admit, check and emit") is event-driven EARS, and most Behavior rules carry SHALL. A few normative rules are written as descriptive prose without SHALL.

## Verdict

PASS with one low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Some normative rules lack SHALL, so they read as description rather than obligation: the new reserved words "become reserved and are refused as identifiers"; "a two-segment model name names a model declaration"; "A snapshot's `model` member is required exactly when ..."; "A name with no `ix://` scheme names a source declaration". The same applies to the matching FR-106 amendment. | spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:125-134; spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md:329-333; spec/functional/FR-106-admit-snapshots-and-invocations.md:292-301 |

## Scope and evidence

Direct review.

- FR-569 statement and Behavior (examined).
- FR-104 amendment (examined, clean: it uses SHALL).
- FR-105 amendment (examined, clean).
- FR-390 view-creation amendment (examined, clean: SHALL admit).
- FR-106 amendment (examined): FND-001.

## Dispositions

Disposition pass 1, checked against fix commit ba4ea0fcd.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The reserved-word, segment-resolution and snapshot rules now carry SHALL: FR-569:136-145 and 395-404, and FR-106's amended snapshot paragraph. |

Disposition pass 2, checked against fix commit b3e72440c: no finding of this SR is outstanding, so this round adds no rows. The round-2 re-split checks (honest "now" cases, no remaining claim that IR admits anything now, the three old refusals removed everywhere, and the cross-AC consistency of FR-390-AC-70 to AC-73, FR-105-AC-18, FR-383-AC-67, FR-218-AC-39, TC-1407 and TC-1408) are clean.

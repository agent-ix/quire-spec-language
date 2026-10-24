---
id: SR-11184
title: "base spec review of quire-spec-language PR 860 (remove hand-maintained TestMatrix files)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@e797a8c434bd1087355bce79d47bfca1e2886232; spec/tests.md and spec/{model-linking,native-lowering,native-packages,native-readiness,native-runtime,native-temporal,native-workflow}/tests.md (deleted), spec/functional/FR-001..FR-008, FR-010, FR-018..FR-021, FR-042, FR-044, FR-056, FR-087 Verification cells, spec/integration/IT-005, plan/Plan-002..Plan-009 edits, spec/decisions ADR-010/012/014/016/017, README.md"
review_set: subset
---

## Summary

Ticket: QSL-784. Deleting the eight TestMatrix files is correct under the
no-tracking rule. Every check here asked whether the deleted text was the only
home of real AC-to-TC trace content or real requirement content.

- Moved cells: 119 `Test (TC-nnn)` cells were added. A script confirmed every
  named TC file names that exact AC. 38 were read by hand across FR-001..008,
  FR-010, FR-018..021 and FR-056. Each TC's procedure or Verifies/Traces/Scope
  line exercises that AC. None was wrong.
- Gate: the remaining FR half of check-index-completeness exits 0.
- Dangling references: none are new outside the dated plan logs and
  task-completion notes and the historical spec/reviews.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The AC-level trace is lost for 65 ACs: FR-009-AC-1..7, FR-011-AC-1..4, FR-022..FR-031 and FR-033/FR-034. These cells stay bare `Test`. Their TC files relate to the FR only and do not name the AC, so the deleted matrix rows were the only AC-to-TC mapping. A sample of 18 shows each TC clearly exercises its row's AC: FR-009-AC-2/3/4/6/7 (TC-092/093), FR-022-AC-2/3/4 (TC-096), FR-027-AC-1/2/3 (TC-105), FR-031-AC-1..4 (TC-109) and FR-034-AC-1/6 (TC-112). The quire-integration TC-094 trace for FR-009-AC-5 was stated only in TM-006's overview. Plan-008 now says "Each FR's Verification cell maps the criteria", which is false for FR-009. Fix: write the deleted rows' TCs into these cells, for example `Test (TC-093)`. | spec/functional/FR-009-lower-qualified-projections.md; FR-011; FR-022..FR-031; FR-033; FR-034; plan/Plan-008-native-lowering/plan.md:46 |
| FND-002 | medium | Wrong trace. FR-044 now says FR-036's capture-initializer scope refusal "is covered by TC-011–TC-019". Those are the native-readiness syntax and CLI cases for FR-001..004 and FR-010, and none of them mentions capture. The old "TM-002" pointer was already wrong; the edit made it a specific wrong TC claim. The refusal is FR-036-AC-2 ("illegal binder/capture scope"), Test (TC-114). | spec/functional/FR-044-activate-temporal-obligations.md:49 |
| FND-003 | medium | Real behaviour rules were deleted with no other home. TM-003's "Domain-package model intake (L2)" section was the only text stating which declaration kinds each native site binds. A type site binds an object, interface, record value or value type. A protocol role also binds a Part or Port. A protocol relationship binds a Connection or navigation relationship. Any other kind refuses at the linker. The same section said which domain field types the composed checker types (Boolean or a domain object/Interface/record value type under multiplicity 1, 0..1 or 0..n); any other field type, and a domain value type, refuse as an unsupported prerequisite. FR-036, FR-040 and FR-056 do not state this. Fix: move the paragraph into FR-036 (beside AC-9) or FR-056. | spec/model-linking/tests.md (deleted), section Domain-package model intake (L2); spec/functional/FR-036-link-composed-native-packages.md |
| FND-004 | low | FR-057-AC-1/4/11/13/14 stay bare `Test`, although TC-153/970/971/973 name them in a Scope line. Under the PR's own rule they qualify for moving. No trace is lost, because the TC files carry it. | spec/functional/FR-057-*.md |
| FND-005 | low | FR-042 says FR-050-AC-4 is "marked passed", but the status it cites no longer exists anywhere. | spec/functional/FR-042-publish-compiled-protocol-artifacts.md:469 |

## Verdict

CHANGES REQUESTED. The matrix deletion, the gate change and the reference
cleanup are correct. The 119 moved traces are accurate. Three medium findings
need fixing in this PR: 65 ACs whose only AC-level trace was deleted
(FND-001), one moved trace that is wrong (FND-002), and one deleted
linker/checker rule paragraph (FND-003). There are two lows.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 281099866 |
| FND-002 | fixed | 281099866 |
| FND-003 | fixed | 7249a08e6 |
| FND-004 | fixed | 281099866 |
| FND-005 | fixed | 281099866 |

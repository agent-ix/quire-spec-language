---
id: SR-15525
title: "integrity review of STD-297 checked-term boundary (QSL PR #1225)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@09f27280c4441e702eadfdae0d08691602dc974b; spec/functional/FR-400-separate-checked-terms-from-the-checker.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/functional/FR-059-check-backend-dependency-direction.md, spec/functional/FR-060-check-qsl-api-surface-boundary.md, spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md, spec/assurance/AD-001-shared-evaluation-core-interface.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: reviews
---
# SR-15525: integrity review of STD-297 checked-term boundary (QSL PR #1225)

## Summary

Ticket: STD-297. This review checks consistency across the six edited files and against the open PRs.

The ids are unique. Checked against current main (2478fe433, with #1224 merged) and the open PRs: no open PR adds FR-400, X-13, FB-14, T12-G or a layer C. #1223 adds NFR-015, TC-1110 and
TC-1111, and #1224 adds SR-15514. ADR-011 §7.2 says "The root crate re-exports no moved item", and X-11 says "no
re-export from `qsl-semantics`". FR-400's no-re-export rule matches both, so lead default 7 holds. Removing the
pinned `01b1b33...` SHA from AD-001 follows the repo's pin rule and is correct.

`QSPEC_DIR=... make check-spec-validation` exits 0 at the head, and `quire validate` on FR-400, ADR-011 and AD-001
reports no finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: the IR-583-before-IR-586 ordering still contradicts itself, and the head conflicts with main. On current main (2478fe433, after #1224), AD-001 says S2 "can proceed after STD-297 independently of IR-583". The slice table and the "structured graph" paragraph agree. FR-262 still says the opposite: line 65 reads "(IR-583) precedes checked-term separation (IR-586)", and lines 221-222 on main (225-226 at the head) read "IR-586 after the interface". This PR edits FR-262 in the next paragraph and is the place to fix it. <br>Judgement: FR-400 needs nothing from IR-583, because C is independent and IR-583 keeps its contract opaque until X-13. #1224's ordering is sound, so align FR-262 with it. <br>`git merge-tree` of main and the head conflicts in AD-001. Resolve it as main plus only this PR's change: take main's slice table, its "structured graph" paragraph and its Decisions, and drop this PR's IR-586 row and "Consequently" rewrite. Keep the FR-400 reference and the Risks bullets. Under its blockers, AD-001 should also name the S2 prerequisites that main's row (blocked by STD-297 only) omits: the `agent-ix/quire-eval` repository, and S1 for `WrongSnapshotCause` unless AC-2 exempts it (SR-15524 FND-004). | spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md:65, 225-226; spec/assurance/AD-001-shared-evaluation-core-interface.md:155, 160-167, 212-213, 243-245 |
| FND-002 | medium | minor: the spec contains status and changelog wording. <br>- The AD-001 row says "(re-read 2026-10-22: the IR-583 and IR-269 edges were removed)". This disappears once main's row is taken. <br>- Main's line 52, "The source boundary was rechecked against QSL main on 2026-10-22.", replaced the pinned SHA that this PR removes. When resolving the conflict, remove the dated sentence as well and do not keep either. <br>- FR-400 uses "today" four times: "all defined in `qsl-semantics` today", "`CheckedGraph::region` today", "sits in `qsl-foundation` or `qsl-forms` today" and the Inputs. <br>- The AD-001 Risks say "does not exist yet". <br>Describe the source location without "today". | spec/assurance/AD-001-shared-evaluation-core-interface.md:52, 155, 243; spec/functional/FR-400-separate-checked-terms-from-the-checker.md:82-83, 133-136, 153-154 |
| FND-003 | medium | minor: the head no longer merges cleanly. #1224 merged at 2478fe433, so the head needs a main sync. `git merge-tree` reports an AD-001 conflict with main, resolved as described in FND-001, and another with open #1223 (44f497b8c). #1223 edits the IR-587 and IR-589 rows next to the IR-586 row and appends to Risks. ADR-011 and FR-262 merge cleanly with #1223, so whichever of #1223 and #1225 merges second keeps both sides. #1223's NFR-015 says the leaf's F items move with S1, and FR-400 moves eight F items in S2. Whichever of S1 and S2 lands second must rebase onto the other, as the Risks bullet already says. | spec/assurance/AD-001-shared-evaluation-core-interface.md:52-55, 152-170, 222-249 |
| FND-004 | medium | minor: the dependency-edge records are incomplete. ADR-011's §7.1 edge table has a row for each earlier shared leaf (`quire-semantic-value`, `quire-walk`, at line 1068) but none for QSL → `quire-checked` or RT/CG/IR → `quire-checked`. The FB-05 row lists the permitted §7.1 backend edges without the C edges. The mermaid graph draws RT → QCK but no CG → QCK, and no QCK → W edge, although C depends on W. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:571, 1016-1040, 1064-1068 |
| FND-005 | low | minor: FR-060 still introduces its table with "six rules:" (line 82), and after T12-G the table has seven rows. | spec/functional/FR-060-check-qsl-api-surface-boundary.md:82 |

## Verdict

One major finding (FND-001). After #1224, the S2 ordering contradiction is between FR-262 and AD-001 on main. This PR
should fix it in FR-262 and resolve its AD-001 conflict as main plus FR-400's reference and Risks. Once that is done
and the minor findings are applied, the cross-document edits are consistent: the ADR-011 layer C row, crate map,
FB-05 amendment, FB-14, X-8 and X-13, and FR-059 and FR-060. The PR is mergeable after those fixes, a main sync and
SR-15524's majors.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | major: this PR conflicts in meaning with open #1227 (STD-298, FR-401, 2013cea1d). <br>- `WrongSnapshotCause`: #1227's X-15 says "When X-13 lands first, X-13 moves it to the same place" (SV `call`), and its AD-001 text says "whichever of S1 and S2 lands first moves `WrongSnapshotCause`". FR-400 says the type "does not move in this slice", and AC-2 and FB-14 exempt it until S1. <br>- `refusal_record`: #1227 moves `Evaluation` to E and makes `Evaluation::refusal_record` a qsl-eval free function, while FR-400 specifies it as an `Evaluation` method that takes a region resolver. <br>- FB-05: #1225 sets the shared-leaf class to four members, and #1227 adds E, which makes five. <br>The two PRs also conflict textually in ADR-011. Whichever merges second must use one rule, ideally #1227's: X-13 moves `WrongSnapshotCause` to SV `call` when S1 has not, and the exemption applies only in the other order. That PR also restates `refusal_record` as the free function and counts five members. | spec/functional/FR-400-separate-checked-terms-from-the-checker.md:198-201, 236-242, 251-254, 307; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:571, 580 |
| FND-007 | medium | minor: some X-13 and AD-001 text was not updated by the fix round. <br>- X-13 still lists `SemanticGraph` among the moved identities, whereas FR-400 and §6.2 move a projection and keep the full graph. <br>- X-13 calls `construct`'s "only QSL caller is `check`", and AD-001 Risks says "T12-G limits their callers to `check`". `library` and `value::definition` are now callers too. <br>- X-13 names only TC-244 row 1's `compile_fail` checks, while FR-400 rule 5 lists more. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1185; spec/assurance/AD-001-shared-evaluation-core-interface.md:238-243 |
| FND-008 | medium | minor: X-13 says `qsl-semantics`, `qsl-package`, `qsl-route` and `qsl-replay` add `quire-checked` to their `[dependencies]` (the X-6, X-7, X-9 and X-10 tables). Only X-8's row was amended. X-7 and X-10 state their sets "exactly", and TC-390 asserts the exact sets. Amend those rows as X-8 was amended. | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:1185 |

## Dispositions

Round 1. Reviewed QSL #1225 at dc6e2b208 against origin/main a89136bfa, which includes #1224 and is an ancestor of the head. `make check-spec-validation` exits 0 at the head. `git merge-tree` of the head with #1223 (8a42d6e84) is now clean, and with #1227 (2013cea1d) it conflicts in ADR-011.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |

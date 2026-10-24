---
id: SR-17008
title: "Gap analysis of PR #1437: FR-290-AC-1/AC-5, FR-289-AC-9 trace after QSL-1036"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@47d37b6c30436ba93810087b7fc1b07eddc89585; PR #1437 (branch qsl1036-bounded-basis); spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md (AC-1, AC-5); spec/functional/FR-289-keep-results-unchanged-by-provider-installation.md (AC-9, AC-10); spec/test-cases/TC-773-*.md; spec/test-cases/TC-774-*.md; qsl-replay tests carrying these ids"
review_set: subset
date: '2026-10-24'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-773
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-774
    type: reviews
---
## Summary

Ticket: QSL-1036. PR: quire-spec-language#1437. There is no plan bundle. Plan completion: not assessed.

`quoin matrix --json` and `quire trace --id` were tried. Neither finished
within 300 s and 120 s on this repo, so the Quoin method could not complete.
This is a manual AC-to-test check. It uses `git grep` for `#[trace]` ids at
the head and at origin/main.

Tag inventory, head versus main:

- FR-290-AC-5: main had three tags, all partial (see SR-17007). The head has
  one, `plugin_bases_read_and_settle_by_item_kind`. That tag is complete except
  for the Reduced reader clause (SR-17007 FND-001).
- FR-289-AC-9: main had two partial tags. The head has one, the new test, which
  covers the AC completely.
- FR-289-AC-10: unchanged, `plugin_proof_is_trusted_and_names_the_plugin`.
- FR-250-AC-7: unchanged.
- FR-290-AC-1: main had one tag, on `coverage_is_decided_from_the_checked_claims_own_horizon`
  (added by #1435). The head has none anywhere in the repo.

FR-290-AC-1 check: the AC requires four records. The first is a non-hybrid
process-provider `proved` record (`proved`, `BackendId`, `trusted`), with
hybrid bounded-solver deferred to AC-5. The second is FR-281-AC-1's `certified`
record naming the EN-5 checker. The third is FR-281-AC-4's `uncertified`
test-engine record. The fourth is a `certified` Kani C-09 record. The coverage
test asserts none of the FR-281 or Kani records, and no `BackendId` on a
non-hybrid record. The removed tag was overclaiming, and removing it is
correct. The AC is now untagged (FND-001). qsl-analyze/tests/it/analyze.rs:13-20
records that FR-281-AC-1's EN-5 half is still open (QSL-574, QSL-575). A
complete AC-1 test is partly blocked on that work. No existing Linear ticket
was found for an FR-290-AC-1 test (`ix-board search FR-290-AC-1`).

## Verdict

The PR's own scope is sound apart from SR-17007 FND-001. Two gaps are
outside it. FR-290-AC-1 has no tagged test after this PR (FND-001), which
needs a follow-up ticket. TC-773's text was not updated for the #1435 ruling
(FND-002), which belongs to the spec lane. Neither should block the merge on
its own.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-290-AC-1 has no tagged test after this PR. The removed tag on `coverage_is_decided_from_the_checked_claims_own_horizon` overclaimed: that test asserts no FR-281-AC-1 EN-5 `certified` record, no FR-281-AC-4 `uncertified` record, no Kani C-09 `certified` record, and no non-hybrid `BackendId`. The AC is now uncovered in the matrix. Follow-up ticket needed: "Test FR-290-AC-1 (TC-774 step 1)". One test settles a non-hybrid process-provider `proved` record (proved, provider BackendId, trusted), FR-281-AC-1's analyze record (certified, naming the EN-5 checker), FR-281-AC-4's test-engine record with no checker (uncertified, naming the engine) and a Kani C-09 `proved` record (certified), tagged `#[trace("TC-774", "FR-290-AC-1")]`. The EN-5 part is blocked on QSL-574/QSL-575. | spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md:542 |
| FND-002 | low | TC-773 step 5 and its expected result still say each of the six plugin bases, `bounded-solver` included, settles a plugin `proved` with `trusted`. After the #1435 ruling, FR-289-AC-9 makes that true only for a hybrid claim with sufficient coverage. Insufficient coverage settles `inconclusive` `InsufficientHorizon`, and a `bounded-solver` basis on a non-hybrid item is refused. The TC that the new FR-289-AC-9 test is tagged to does not describe those two cases. Fix (spec lane): state the coverage split and the non-hybrid refusal in TC-773 step 5 and its expected result. | spec/test-cases/TC-773-installing-removing-or-reordering-providers-changes-nothing-outside.md:23,33 |

---
id: SR-769
title: "QSL-310 gap analysis of PR 507 (TC-466 step 1 VersionUnchanged and step 2 postcondition terms)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6200370a44f5521e3b4618739a95535980cee603; qsl-replay/src/spine/clause/tests.rs; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md (unchanged); spec/functional/FR-106-admit-snapshots-and-invocations.md (frame check, unchanged); spec/test-cases/TC-466-s6a-evaluates-state-clauses.md (unchanged); spec/tests.md (unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-466
    type: references
---
## Summary

Ticket: QSL-310, the first slice of the canceled QSL-308. PR:
quire-spec-language#507 at 6200370a, base f17c2d4f. There is no plan
bundle, so the scope is the ticket's own asks: TC-466 step 1's
`VersionUnchanged` sub-case, TC-466 step 2's three postcondition terms, and
an invocation-selection helper like `run_config_version_current`.

1. **Step 1 `VersionUnchanged`.** Delivered and discriminating. See SR-768
   item 1. It carries `#[trace("TC-466", "FR-107-AC-1")]`, and FR-107-AC-1
   is the AC that names `VersionUnchanged` over unchanged-version and
   changed-version.
2. **Step 2's three terms.** Delivered as three tests, each carrying
   `#[trace("TC-466", "FR-107-AC-2")]`. FR-107-AC-2 is the AC that names
   exactly these three terms. Two of the three discriminate; the third
   (`pre(present(self.parent) implies ...)`) does not. See SR-768 FND-001.
3. **Invocation helper.** Delivered as `run_config_version_invocation`
   (SR-768 item 4).

The trace tags are correct against FR-107's own AC table (lines 115-116):
AC-1 is the corpus truth-value row that includes `VersionUnchanged`, and
AC-2 is the `pre`/post read row with the three terms.

Remaining TC-466 work: step 3 (b)-(e) is tracked in QSL-311 (Backlog). Step
1's `ParentOrder`/`NoCycle` and step 3(a) were already built before this PR.

Underspecified code: none added. The diff is test-only.

Semantic review: done inline with SR-768, because the diff is one test file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-466's `## Status` prose is stale after this PR. It still says "Steps 2 and 3 (the `VersionUnchanged` sub-case of step 1 included) are **Pending QSL-279**" and explains why `evaluate_clause` could not be reached. This PR builds and passes step 2 and the `VersionUnchanged` sub-case end to end, so the text should list them as built and leave only step 3 (b)-(e) pending, under QSL-311. The `spec/tests.md` row can stay `Planned` while step 3 is open. | spec/test-cases/TC-466-s6a-evaluates-state-clauses.md:58-77 |

## Verdict

PASS with one low finding. The ticket's asks are all delivered with correct
trace tags. The weak discrimination of the third step-2 term is recorded as
a code finding in SR-768 FND-001.

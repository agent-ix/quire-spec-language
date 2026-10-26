---
id: SR-17222
title: "spec-review disposition of SR-17221 findings on hyper emission FR-488 and TC-1266 (quire-spec-language PR 1548, head f6fd494f8)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f6fd494f8; spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md; spec/test-cases/TC-1266-a-hyper-clause-emits-a-hyperproperty-claim-node.md; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md; spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-488
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: references
---

# Disposition of SR-17221 on QSL-1071 (quire-spec-language PR 1548)

## Summary

Ticket: QSL-1071. Re-review of fix commit f6fd494f8 against SR-17221. `quire validate` is clean (no errors; only the repo-wide duplicate-archetype and inline-data-schema warnings that every spec file shows) for FR-488, FR-315, FR-487, TC-1266, `spec/reviews/qsl-1071-pr1548/base.md` and the root `reviews/qsl-1071-pr1548-17221-spec-review.md`, run from the repository root. Nine of nine SR-17221 findings are resolved in FR-488. Two new findings: TC-1266 still cites the pre-split AC numbers, and PR 1548 was merged at e72495ac3 before the fix commit, so the fixes are not on main. QSpec-owned Q-1..Q-6 are not judged beyond QSL's local choices.

## Dispositions

| Finding | Disposition |
|---------|-------------|
| FND-001 | Resolved. "Consequence" states the wrapper is a single-trace temporal claim to outside consumers; the I2 reader skips it in root listings, a looked-up wrapper returns `clause-root-unresolved`, QSL consumers (FR-338, FR-315, FR-337 listing) go through the reader; AC-15 tests it; Q-5 carries the QSpec ask. |
| FND-002 | Resolved. Fairness aggregate uses FR-370's member names (`kind` `fairness`, `fairness_kind`, `granularity`, `declaration`, `name`); Q-1 states nothing validates `declaration`/`name`; `systems_interface` declaring node refused as FR-370-AC-11 does (AC-13). |
| FND-003 | Resolved. One level rule (slot index in its term's slot list), sharing by name, level and type, same read at two levels stated; AC-8 tests it; Q-3 asks QSpec for scope. |
| FND-004 | Resolved. Roots defined (no-root and two-component reads refused), precedence list of five, QSL-1094 named as lifting ticket, alias case tied to Q-2; AC-17, AC-18. |
| FND-005 | Resolved. "Profiles" section, refusal 3 for bounded-profile fairness, AC-19 and AC-20. Consistent with FR-337 interval writing. |
| FND-006 | Resolved. `conjuncts() -> Vec<(MatchPart, usize)>` added to Inputs, ordered by `span()`; part recomputed from the checked clause; AC-9. |
| FND-007 | Resolved in FR-488. ACs split to AC-1..21, each one falsifiable, node-ceiling AC-21 added. TC-1266 not updated: see FND-001 below. |
| FND-008 | Resolved. Q-6 added; AC-12 states the renamed-variable result. |
| FND-009 | Resolved. `temporal_profile` wording; FR-315 and FR-487 lines re-wrapped. |

No contradiction found with merged FR-315 ("Product identity", AC-12), FR-487 (slots, sites, accessors), FR-337 with captures (the wrapper's `captures` argument is the empty aggregate, six arguments in FR-337's order), FR-370, FR-383-style lookup refusals, or FR-322 (parameter key name, level, type).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|----|----------|---------|-------|-------|
| FND-001 | medium | TC-1266 still maps its steps to the pre-split AC numbers. Step 1 expects "the wrapper and slots of FR-488-AC-2" (AC-2 is the quantifier/parameter content; the wrapper is AC-4/5); step 2 "AC-3's subjects, parameters and slots" (AC-3 is the reader triple; SavesPower is AC-6); step 3 "AC-4's conjunct terms" and its action "match variants of FR-488-AC-4" (AC-4 is the wrapper; match is AC-7/8/9); step 4 "AC-5's anchor references" (AC-10); step 5 "AC-6's NodeIds" (AC-11/12); step 6 "AC-7's fairness aggregates" (AC-13); step 7 "AC-8's equal occurrences and lookup refusals" (AC-14/15); step 8 "AC-9" (AC-16/17/18, and the timed clause is AC-18). Only step 10 (AC-19..21) and the Scope line match. A tester following TC-1266 verifies the wrong criteria and AC-15 and AC-18 have no step. Fix: renumber steps 1..9 expected results to the split ACs, add AC-15 (wrapper skipped) to step 7 and AC-18 to step 8. | TC-1266 Test Procedure, Expected Results | wrong-requirement |
| FND-002 | high | PR 1548 is MERGED (merge 683db94d8, head e72495ac3, 2026-10-25T15:50Z). f6fd494f8 exists only on the branch and is not on main, so main carries the unfixed FR-488 and TC-1266 and SR-17220/17221 findings are unresolved there. Pushing to the branch cannot land. Fix: open a follow-up PR from this branch (rebased on main) with the fix commit, the TC-1266 renumbering of FND-001, and these reviews. | PR 1548 state | correct-requirement-no-evidence |

## Verdict

FAIL until FND-002 and FND-001 are handled: FR-488 content itself passes (all nine prior findings resolved, no new contradictions), but TC-1266 is inconsistent with the AC split and the fixes are not on main. No further spec change to FR-488 is required.

---
id: SR-11415
title: "Gap analysis — QSL-794 FR-364, FR-106-AC-23..27 and AC-39, check 11.5 (PR 880)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@c8b1583be44d3bc1ad495842e6af63dd65fc9a65; FR-364-AC-1..4, FR-364-AC-10..14, FR-106-AC-23..27, FR-106-AC-39, FR-103-AC-2, FR-115-AC-8..10, TC-1033, TC-1035, TC-1037 rows 1-12, TC-1041; qsl-semantics/tests/it/relationship_extents.rs, qsl-semantics/tests/it/relationship_snapshots.rs"
review_set: subset
---

## Summary

Ticket: QSL-794 (slice b). Planless gap analysis of this PR's own commit:
each in-scope AC is traced to the tests tagged with it, and the coder's
mutants plus the ones the dispatch named are run mentally against those
tests. Plan completion: not assessed. No build was run.

## Verdict

**FAIL: 1 blocking finding (FND-001, a test that cannot fail for the
property it is named for).**

Which test kills which mutant:
- Source and target ends swapped (6.8, condition 3): killed by
  `check_6_8_...` (the `end` field for both sides), by the base admission
  tests, and by TC-1033 step 4.
- Condition 3 skipping an absent full reference: killed by TC-1041 row 17
  (`condition_three_checks_the_type_of_an_end_whose_object_is_absent`).
- Cardinality checked at admission: not expressible, because no bound
  reaches the store. TC-1035 row 5 and TC-1033 step 7 assert admission.
- Check 8 before check 7: killed by TC-1035 row 18 and TC-1041 row 19
  (Incomplete, not dangling).
- 6.9 off: killed by TC-1035 row 32, not by `check_6_9` (the store backstop).
- Check 7's unlisted-end rule off: killed by rows 30 and 31 and TC-1041
  row 19.
- Relationship grant ignored in 11.5: killed by TC-1035 row 23 and TC-1037
  rows 1 and 5.
- A relationship not complete in both snapshots producing a witness:
  killed by TC-1035 row 27 and TC-1037 row 11 (both halves).
- Source then target order: killed by TC-1037 row 7.
- DeclarationKey order replaced by document order: **survives** (FND-001).
- Witness population taken from the target end: survives. Every fixture
  end is in `Q`. Recorded below.
- Step 5 run per population: survives with the single population `Q`.
  Step 5 before step 4 is killed by TC-1037 row 9. Recorded below.

Coder's gaps, judged:
1. `modifies [relationship]` cannot be declared. Merged FR-103-AC-2 (as
   amended by QSL-794's own spec PR #868) says `modifies [Supersedes]`
   admits and the effect modifies it, and `creates [Supersedes]` refuses
   `malformed-declaration`. On `main` intake refuses it
   `unknown_required_feature`/`unsupported-feature`. The pre-existing test
   `a_modifies_entry_naming_a_relationship_refuses_unsupported_feature`
   (tagged TC-458, FR-103-AC-2) asserts the opposite of the merged AC. That
   is a failing AC on `main`, not introduced by this PR's diff. This PR's
   check 11.5 gives the specified verdict for every frame a package can
   declare today. A user cannot grant a link change (intake refuses the
   whole unit), so every changed link of a complete-in-both relationship is
   a violation. **Follow-up, not a finding of this PR**, but it needs its
   own ticket before QSL-794 closes: FR-103-AC-2 (relationship half),
   TC-458 step 2 part 4 (flip the existing test), the checked-package frame
   node and lowering carrying a relationship key (QSpec FR-340 / FR-013
   relationship grant), and FR-115-AC-8 end to end through a `Frame` run.
2. TC-1037 rows 1 to 12 at the `FrameVerdict` level, tagged `TC-1037` only:
   no over-claim. The tag claims no FR-115 AC, so FR-115-AC-8..10 stay open
   in the matrix, which is honest.
3. `pipe` Connection sub-cases not built: FR-364-AC-1 and FR-106-AC-24 "a
   connection" stay unasserted. The code path (`model_facts` keeps only
   relationships whose two ends are object types, as `systems::classify`
   does) gives the specified `wrong-role-mapping`, but its filter is not
   exercised. Recorded medium.
4. The bundle key leaves out the invocation digest: a departure from the
   FR-364 key text with no reachable wrong `resolve` (see SR-11414,
   recorded medium).
5. Partial only from an enumerator failure: holds.

Recorded (medium/low, not blocking):
- Medium: TC-1041 row 20, fourth case. The TC puts D on the manifest and D′
  on the closure and expects part `closure`. The test puts a `Document`
  origin on the manifest against a `Label` selected key and expects
  `manifest`. Its "same identity, digest differs" case therefore differs by
  variant, not by digest. The `documents` (selected document digests)
  component is never varied, so FR-364-AC-14's "another selected document
  digest" is not asserted.
- Medium: FR-115's "for a link change, the population of its source end"
  has no cross-population test.
- Medium: no row has two differing bundle parts, so the order manifest,
  closure, types is unasserted. TC-1041 does not require it.
- Low: step 5 "once, after every population" is not separated from a
  per-population run (one population in every fixture).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1037 row 6 (and FR-115-AC-9 "a changed link of `Pump-housing-Tank` before one of `Sys-composition-Pump`") cannot fail for its property. Both snapshots list housing first and composition second, so document order equals ascending `DeclarationKey` order. An implementation that walks entries in document order passes, and the added `assert!(housing_key() < composition_key())` checks only the fixture. No other test exercises FR-106 check 11.5's "relationships are walked in ascending `DeclarationKey` order". List composition before housing in both snapshots, so key order and document order disagree. | qsl-semantics/tests/it/relationship_snapshots.rs:1611-1638 |

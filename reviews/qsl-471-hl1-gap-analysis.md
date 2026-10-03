---
id: SR-1276
title: "Gap analysis of quire-spec-language HL1: an import names its library by identity alone"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4965fe5daac7032cd45d86a39f00857b23db9b96; HL1's own commits only, git diff 2f0581fe0..4965fe5da: acceptance criteria FR-099-AC-1/2/3/6, FR-091-AC-24, FR-098-AC-6, FR-087-AC-12, FR-087-AC-14, TC-253, TC-282, TC-405, TC-444, TC-446 against the tests that trace them"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-253
    type: reviews
---
# Gap analysis of quire-spec-language HL1

## Summary

Ticket: QSL-471 (HL1), with QSL-40 and QSL-39. No PR yet (stacked on LC1).
This is a manual criteria-to-tests check over the criteria HL1 edits or whose
tests HL1 edits.

Backed, with an oracle that would fail on the old behaviour:

- FR-099-AC-2 / TC-446 step 2: `an_import_names_only_its_library_identity`
  (qsl-cst) admits both identity-only forms, checks the alias, and refuses
  `version`, `digest` and both with `invalid_syntax` and no selection.
  `an_import_selects_by_identity_and_the_lock_binds_the_recompiled_package_id`
  compiles against two library sources at two versions and checks that each
  lock holds that source's own recomputed `package_id`, and that the two
  differ.
- FR-099-AC-1, AC-3 (cycle, missing, wrapped, dependency input), AC-6: the
  `dependency_tests` fixtures only drop the import tokens. Their assertions
  are unchanged.
- FR-091-AC-24 / TC-405 step 4: the `spine.rs` test now uses `import
  "test/units" as u;`.
- FR-098-AC-6 / TC-444 step 7: `tc_444_a_package_with_a_dependency_replays_
  and_names_a_stale_one` matches the amended AC (`PackageIdMismatch` with the
  request's and the recompiled `package_id`). Whether that AC is right is
  SR-1277 FND-001.
- FR-087-AC-14 / TC-253: `e4_refuses_a_conflicting_diamond` keeps the version
  and `package_id` diamond cases.

No production code in the diff lacks an owning requirement: the deletions are
what FR-099's amended Behavior and ADR-015 D-2 state.

## Verdict

One high gap: TC-253 step 10 still requires an E4 refusal that HL1 deleted,
and the test traced to TC-253 dropped that half without the TC changing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-253 step 10 still says "the first link refuses `DependencyIdentityMismatch` (`stale_dependency`) naming the recorded and the recomputed `package_id`". HL1 deleted `LinkRefusal::DependencyIdentityMismatch` and `Import.digest`, so no link can refuse that way, and `e4_refuses_a_conflicting_diamond` (traced `TC-253`) dropped the stale half. TC-253 now states a result no test checks and the code cannot produce. Fix: rewrite step 10 (and its procedure step) to the diamond refusal alone, as FR-087-AC-14 now reads. | spec/test-cases/TC-253-verified-binding-three-conditions-and-refusals.md:91-95; qsl-package/src/emit/tests.rs:2764 |

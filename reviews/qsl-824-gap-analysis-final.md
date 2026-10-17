---
id: SR-12078
title: "Gap analysis of quire-spec-language PR #931 final head against QSL-824 and NFR-013"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@74651fc5a8aeaa94b7c2c709e31bdf05791f2223; PR #931: Makefile (test-deep-format), xtask/src/ci_feature_lane_tests.rs, xtask/src/ci_feature_lane_tests/cargo_double.rs, spec/non-functional/NFR-013-isolate-native-ci-feature-artifacts.md; NFR-013-AC-1, NFR-013-AC-3, QSL-132 acceptance"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-013
    type: reviews
---
# Gap analysis of quire-spec-language PR #931 (final head)

## Summary

Ticket: QSL-824 (ticket text read as data). Planless, scoped to the PR diff.
Plan completion: not assessed. This is the second round. SR-12066 reviewed
1fe47e139.

## Verdict

Pass. No major gap found.

- QSL-132 / NFR-013. No lane can execute an artifact built under different
  features. The deep stage shares only the default lane's directory. Its
  selection (`test --locked --workspace --tests`) builds the same feature set
  and profile as Default position 3, and Cargo runs only fingerprint-matched
  binaries. No target directory is shared across feature sets. The nine
  NFR-013 lane vectors are unchanged (NFR-013-AC-3).
- NFR-013 text. "`test-deep-format` runs `test --locked --workspace --tests`
  with the default lane's feature selection, so it uses the default lane's
  directory (or its explicit override) and reuses that lane's executables" is
  true. The Makefile and the TC-915 assertions match it, and the leader's
  zero-`Compiling` run log backs the reuse claim. "The routing policy changes
  no Cargo flag of the two feature lanes, no CI check and no aggregate
  dependency order" is also true: CI_CHECKS order and both lanes' recipes are
  unchanged. This fixes the round-1 nit.
- NFR-013-AC-1 / AC-3 tests. `deep_formatter_target_runs_each_full_size_case_and_is_in_ci`
  (TC-915, traced to AC-1 and AC-3) pins the stage to the default child under
  no root and under a caller root, and to the explicit override. The aggregate
  test pins its place after both lanes.
- QSL-824 "keep every real test". Nothing is lowered. The three ignored
  `formatter_identity` tests are unchanged in source. They run at full size
  with `--ignored --exact --test-threads=1` in `make ci` (CI_CHECKS still lists
  `test-deep-format`). Each name must list exactly once. The leader's log
  shows the three tests ran.
- Not assessed here: QSL-824's before/after wall-time and target-size report.
  This read-only review measured neither.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

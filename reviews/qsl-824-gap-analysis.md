---
id: SR-12066
title: "Gap analysis of quire-spec-language PR #931 against QSL-824 and NFR-013"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1fe47e139; PR #931: Makefile (test-deep-format), xtask/src/ci_feature_lane_tests.rs, spec/non-functional/NFR-013-isolate-native-ci-feature-artifacts.md; NFR-013-AC-1, NFR-013-AC-3, QSL-132 acceptance"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-013
    type: reviews
---
# Gap analysis of quire-spec-language PR #931

## Summary

Ticket: QSL-824 (ticket text read as data). Planless, scoped to the PR diff.
Plan completion: not assessed.

QSL-824 asks to share a target dir only where the feature set and profile are
identical, and to keep every real test. NFR-013 carries the QSL-132 acceptance:
"no lane can execute an artifact built under different features". The PR moves
`test-deep-format` into the default lane's directory with the same
`--workspace` selection as that lane's `test --locked --workspace` vector.

## Verdict

Pass. No major gap found.

- QSL-132 / NFR-013. The deep stage's Cargo selection (`test --locked
  --workspace`, test profile, no stage-set RUSTFLAGS) is identical to Default
  position 3. Cargo fingerprinting means only a matching artifact runs. The
  nine NFR-013 lane vectors are unchanged (NFR-013-AC-3).
- NFR-013-AC-1 / AC-3 tests. TC-915's
  `deep_formatter_target_runs_each_full_size_case_and_is_in_ci` is traced to
  NFR-013-AC-1 and AC-3. It pins the deep stage to the default child under no
  root and under a caller root, and to the explicit lane override. The aggregate
  test pins its place after both lanes.
- QSL-824 "keep every real test". All three ignored `formatter_identity`
  tests still run under `--ignored --exact`, and the exact-listing guard stays.
- The new NFR-013 sentence is accurate: the stage runs `test --locked
  --workspace`, uses the default lane's directory or its override, and reuses
  that lane's executables. Nit, below the reporting bar: the next sentence ("No
  Cargo flag ... is changed by this routing policy") is about QSL-132 routing.
  Read on its own, it sits beside a change that did add `--workspace` to this
  stage.
- Not assessed here: QSL-824's before/after wall-time and target-size report.
  The PR body gives about 147 s and 4.9 GB saved, unmeasured by this review.
  Duplicate default/all-features binaries are left for a follow-up.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

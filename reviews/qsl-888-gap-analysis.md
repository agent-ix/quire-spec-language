---
id: SR-12180
title: "Gap analysis of quire-spec-language PR #1018 against QSL-888, TC-915 and NFR-013"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b571740e523d9dd51978a34ef3792adb394f2b8d; PR #1018: Makefile (test-deep-format), xtask/src/ci_feature_lane_tests.rs; spec/test-cases/TC-915-native-ci-feature-routing.md, spec/non-functional/NFR-013-isolate-native-ci-feature-artifacts.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1018

## Summary

Ticket: QSL-888. Plan completion: not assessed (planless). Scoped to the PR
diff. The computed matrix was not re-run: no criterion, test or trace tag was
added or removed, and the five TC-885 tests keep their tags.

QSL-888's asks, each examined:

- "Merge the dependency-chain invocation back into the light run: Makefile, the
  xtask DEEP calls, and the TC-915 oracle." Done. The Makefile has one run of
  all five cases; `deep_calls` builds that single argv; the TC-915 tests that
  call `deep_calls` (`deep_formatter_target_runs_each_full_size_case_and_is_in_ci`,
  `aggregate_reaches_default_then_all_features_without_changing_recipe_order`)
  compare it exactly. There is no other xtask DEEP call site.
- "Keep every check; no check lowered." Done. The exact-listing guard, its 0/2
  count controls, the `CI_CHECKS` inclusion check and the exact argv oracle are
  unchanged in strength. Every case still runs at full size; no `#[ignore]`
  text, trace tag or test body changed.
- "Measure the wall time before and after in the PR body." Done: 574.6 s
  before, 278.3 s after (`make test-deep-format`), with peak RSS for each
  configuration.
- NFR-013 names `test --locked --workspace --tests` in the default lane's
  directory for `test-deep-format` and no thread count or invocation count, so
  no spec text goes stale. TC-915's text names neither the split nor a thread
  count.

## Verdict

Mergeable. Every ask of QSL-888 is met and no check is weaker. No gap found;
the memory-headroom wording is recorded in the code review (SR-12179).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

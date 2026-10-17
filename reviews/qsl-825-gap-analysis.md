---
id: SR-12082
title: "Gap analysis of quire-spec-language PR #937 against QSL-825 and NFR-013"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7956f9efcab154e35a7d63fad864ff7e7cd73fda; PR #937: xtask/src/feature_scope.rs, xtask/src/test_suites.rs, xtask/src/ci_feature_lane_tests.rs; spec/non-functional/NFR-013-isolate-native-ci-feature-artifacts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-013
    type: reviews
---
# Gap analysis of quire-spec-language PR #937

## Summary

Ticket: QSL-825. Planless gap analysis. Plan completion: not assessed.

QSL-825 outcomes checked (ticket text is data, re-measured):

- Prove the scope per package from cargo metadata, not from a hand list: met.
  `all_features_decisions` reads two `cargo metadata` resolves on every run,
  and no package list is committed.
- The all-features clippy and build still cover every package: met. The
  Makefile `ci-all-features` clippy line and `ci-docs` are unchanged.
- NFR-013/QSL-132: no shared target dir across feature sets: met. Each lane
  keeps its own `CARGO_TARGET_DIR` child, and the decisions are computed inside
  the all-features lane's own invocation.
- Every test still runs in at least one lane: met today. The default lane runs
  `--workspace`, and `cargo tree` confirms the 9 skips are identical across
  feature sets.
- Tests first in ci_feature_lane_tests.rs: met. The real-workspace decision
  test was added there.

Rule fixtures present: identical features skip, own features keep, dependency
change keeps, a chain of dependencies, reachable only under all-features, sole
requester keeps, a covered requester skips, malformed and mismatched
documents, `--exclude` placement before `--`, longest-directory ownership,
empty declarations of excluded packages dropped, and a real-cargo fixture
workspace run with the computed excludes.

## Verdict

Changes requested. One medium finding: the tests never check the invariant
that SR-12081 FND-001 breaks. One medium finding: NFR-013 still states the old
all-features test command.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test checks the invariant the exclusion depends on: the packages that stay must resolve the same under `--all-features --exclude <skipped>` as under the full `--all-features` resolve. The sole-requester fixtures only cover direct manifest requests. The real-cargo fixture has no proc-macro, no build-dependency, no transitive request and no `default-features = false` edge. So the measured host `serde_core` `result,std` regression (SR-12081 FND-001) passes every test in the PR. A fixture needs a skipped package whose dev-dependency subgraph alone enables a feature that a kept package also compiles, plus a host/target split (SR-12081 FND-002). It should assert that the package is kept, or that the lane falls back to the whole workspace. | xtask/src/feature_scope.rs:454-509; xtask/src/test_suites.rs:3314-3377 |
| FND-002 | medium | NFR-013's Scope table states the all-features test command as `test --locked --workspace --all-features`, and says "The routing policy changes no Cargo flag of the two feature lanes". The lane now runs `cargo test --locked --workspace --all-features --exclude <computed packages>` (and the same for the `--no-run` enumeration). The PR body says NFR-013 is unchanged. No requirement or acceptance criterion owns the new package-scoping behaviour: the skip rule, the fail-safe fallback, or "every test runs in at least one lane". Either NFR-013's table and statement get updated, or a requirement with ACs is added that the new tests trace to. | spec/non-functional/NFR-013-isolate-native-ci-feature-artifacts.md:33-59; xtask/src/test_suites.rs:1468-1483 |

## Dispositions

| ID | Disposition |
| --- | --- |
| FND-001 | fixed: fixtures for a dev-dependency chain, a request through a dependency, a build dependency, a platform-specific request and `default-features = false` (each asserts the candidate is put back), a control that stays excluded, `settle` unit tests, and a real-workspace test asserting the remaining packages' resolve equals the full resolve. |
| FND-002 | fixed: NFR-013's lane table, scoping text and new NFR-013-AC-4, and TC-915 step 7, state the computed `--exclude` list and the rule. |

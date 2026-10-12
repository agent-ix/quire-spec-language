---
id: SR-7819
title: "Scoped gap/oracle review of QSL-720 formatter CI change"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3db652c6edefe9b9f01519525243a67e94a377f6; Makefile, tests/it/formatter_identity.rs, xtask/src/ci_feature_lane_tests.rs; NFR-013-AC-1..3, FR-003-AC-9, TC-915"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-013
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-915
    type: reviews
---

## Summary

Performed a manual, change-scoped acceptance-criteria-to-test/code and oracle review. The formatter size controls and gate routing are present, but the feature-lane argument preservation oracle was updated to accept commands that contradict NFR-013-AC-3. The repository-wide computed matrix was attempted with `quire matrix --scope . --format json`, produced no result after more than two minutes, and was stopped; repository-wide matrix/reverse-gap conclusions are therefore not assessed.

## Verdict

CONDITIONAL — scoped review found the NFR-013-AC-3 mismatch below; the full repository gap-analysis was not completed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The modified expected-argv oracle encodes the new `--skip` options as correct, so it cannot detect that the implementation violates NFR-013-AC-3's unchanged-vector contract. Preserve the requirement's vectors or update the owning contract and its evidence before treating the oracle as authoritative. | xtask/src/ci_feature_lane_tests.rs:28-39,108-114; NFR-013-AC-3 | wrong-requirement |

## Coverage

- Examined: NFR-013-AC-1/2/3, FR-003-AC-9, TC-915, the three changed paths, the exact-name skip wiring, and the independent expected-argv oracle.
- Manual trace check: the new Make routing test carries trace tags for NFR-013-AC-1/AC-3; the changed formatter tests retain FR-003-AC-9 tags. The three deep/large test cases are explicitly invoked by `test-deep-format`, and that target is included in `CI_CHECKS`.
- Oracle limitation: the expected arrays were changed in lockstep with the Makefile and therefore encode, rather than challenge, the new skip-argument behavior against the unchanged NFR.
- Matrix: attempted but unavailable within the review window (no stdout after >2 minutes); no repo-wide matrix or reverse-gap claim is made.
- Plan completion: not assessed.

## Dispositions

- FND-001 — fixed `44d0993591db98a64782e7029a8a49cb1fbf82fb`. `xtask/src/ci_feature_lane_tests.rs` restores the unchanged feature-lane argv expectations and drives the actual Make recipes through a Cargo double. Its deep-target assertion checks both the exact `--ignored --exact` listing/execution calls and that zero or multiple ignored matches fail. `test-deep-format` remains in `CI_CHECKS`. Independent disposition review at this exact head: PASS; no builds/tests were run in that review.


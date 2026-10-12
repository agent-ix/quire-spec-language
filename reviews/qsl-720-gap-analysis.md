---
id: SR-7819
title: "Gap analysis — QSL-720 formatter/CI changes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e64eeb642b6d8fc1b5fcbda18ea8f6533ce10762; Makefile, tests/it/formatter_identity.rs, xtask/src/ci_feature_lane_tests.rs, xtask/src/ci_feature_lane_tests/cargo_double.rs, reviews/qsl-720-code-review.md, reviews/qsl-720-gap-analysis.md; NFR-013-AC-1..3, FR-003-AC-9, TC-915"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/NFR-013", type: reviews }
  - { target: "ix://agent-ix/quire-spec-language/FR-003", type: reviews }
  - { target: "ix://agent-ix/quire-spec-language/TC-915", type: reviews }
---

## Summary

At the exact reviewed head, the change-scoped spec/test/oracle review found the prior NFR-013-AC-3 gap fixed and no new changed-scope gap. The repository-wide computed matrix and reverse-gap audit remain incomplete because `quoin matrix --repo . --json` produced no output for more than two minutes and was stopped.

## Verdict

**CONDITIONAL** — no changed-scope gap remains, but repository-wide matrix and reverse-gap conclusions are unavailable.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The modified expected-argv oracle encodes the new `--skip` options as correct, so it cannot detect that the implementation violates NFR-013-AC-3's unchanged-vector contract. Preserve the requirement's vectors or update the owning contract and its evidence before treating the oracle as authoritative. | xtask/src/ci_feature_lane_tests.rs:28-39,108-114; NFR-013-AC-3 | wrong-requirement |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 44d0993591db98a64782e7029a8a49cb1fbf82fb |

Round 3, exact-head recheck: `xtask/src/ci_feature_lane_tests.rs` now records the actual Make recipes against the unchanged complete argv oracle; the prior erroneous `--skip` expectations are absent. The test for `test-deep-format` verifies the exact ignored-test listing and invocation, checks target membership in `CI_CHECKS`, and rejects zero or duplicate selected tests. Rechecked the finding against the complete PR diff at the SHA in `scope`; no new changed-scope finding was identified. The repository-wide Quoin matrix attempt had no output after more than two minutes and was interrupted, so this is not a completed full-repository gap audit.

## Coverage

- Change-scoped examined criteria: NFR-013-AC-1, AC-2, AC-3; FR-003-AC-9; TC-915.
- Reviewed paths: `Makefile`, `tests/it/formatter_identity.rs`, `xtask/src/ci_feature_lane_tests.rs`, `xtask/src/ci_feature_lane_tests/cargo_double.rs`, and the two changed review artifacts.
- Manual trace check: the added Make-routing test binds NFR-013-AC-1/AC-3; formatter identity cases retain FR-003-AC-9 tags. The named deep target selects the three ignored cases and is present in `CI_CHECKS`.
- Matrix reconciliation: `quoin matrix --repo . --json` attempted; no output after more than two minutes, then interrupted. No repo-wide counts or reverse-gap claim is made.
- Plan completion: not assessed.
- No build/test gate was run or inferred; GitHub CLA checks are not code qualification evidence.

---
id: SR-7818
title: "Code and Rust review — QSL-720 formatter/CI changes"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@e64eeb642b6d8fc1b5fcbda18ea8f6533ce10762; Makefile, tests/it/formatter_identity.rs, xtask/src/ci_feature_lane_tests.rs, xtask/src/ci_feature_lane_tests/cargo_double.rs, reviews/qsl-720-code-review.md, reviews/qsl-720-gap-analysis.md; NFR-013-AC-1..3, FR-003-AC-9, TC-915"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/NFR-013", type: reviews }
  - { target: "ix://agent-ix/quire-spec-language/FR-003", type: reviews }
  - { target: "ix://agent-ix/quire-spec-language/TC-915", type: reviews }
---

## Summary

Re-reviewed the complete QSL-720 pull-request diff at `e64eeb642b6d8fc1b5fcbda18ea8f6533ce10762`, including the Make routing, Rust integration tests, Cargo recorder, and committed review artifacts. No new code-review or Rust finding was identified; the prior NFR-013-AC-3 finding remains fixed by `44d0993591db98a64782e7029a8a49cb1fbf82fb`.

## Verdict

**PASS (review only)** — the exact-head review found no open code/Rust finding. This does not assert that the queued build gate passed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The feature-lane receipts append three test-harness `--skip` filters, changing the Cargo argument vectors that NFR-013-AC-3 explicitly requires to remain unchanged. The new aggregate target preserves execution in default features, but does not make the feature-lane commands conform to the stated exact argv contract. Amend the owning requirement or preserve the required vectors before approval. | Makefile:202,210; NFR-013-AC-3 | wrong-requirement |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 44d0993591db98a64782e7029a8a49cb1fbf82fb |

Round 3, exact-head recheck: the default/all-feature workspace test recipes no longer append the three test-harness `--skip` filters, and the independent expected argv vectors match the unchanged NFR-013 contract. The three depth cases are individually marked ignored and invoked by `test-deep-format` using `--ignored --exact`; each listing must contain exactly one ignored test, with zero and duplicate matches rejected. The named target remains included in `CI_CHECKS`. Rechecked the finding against the complete PR diff at the SHA in `scope`; no additional finding was found. No build gate was run or inferred in this review.

## Examined scope

- NFR-013-AC-1, AC-2, and AC-3; FR-003-AC-9; TC-915.
- `Makefile`, `tests/it/formatter_identity.rs`, both `xtask/src/ci_feature_lane_tests*.rs` files, and both changed review artifacts.
- Rust-specific review: repo `AGENTS.md`/`CLAUDE.md`, Rust test tracing, ignored-test target routing, mutation/oracle behavior, stack-bound control, panic surface, and exact current diff.
- The prior FND-001 disposition was checked against commit `44d0993591db98a64782e7029a8a49cb1fbf82fb` and current source at the reviewed head.

## Verification

- `git diff --check origin/main...HEAD`: reported trailing blank lines in the two review artifacts; no source whitespace issue was reported.
- GitHub checks read at review time: CLA and CLA gate checks passed; these are not the PR's local build gate.
- No Cargo build/test/clippy gate was run during this review. The queued gate is independent and was not awaited.

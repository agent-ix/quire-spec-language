---
id: SR-12156
title: "Gap analysis of quire-spec-language PR #993 against QSL-828, FR-003-AC-9, TC-885 and NFR-013"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a52e2ed0fdd6004e00a2956f5c8368f8e93a7119; PR #993: tests/it/formatter_identity.rs, tests/support/front_end.rs, Makefile (test-deep-format), xtask/src/ci_feature_lane_tests.rs, Cargo.toml; spec/functional/FR-003-format-native-source.md, spec/test-cases/TC-885-formatting-keeps-the-checked-package-identity.md, spec/test-cases/TC-915-native-ci-feature-routing.md, spec/non-functional/NFR-013-isolate-native-ci-feature-artifacts.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #993

## Summary

Ticket: QSL-828. Plan completion: not assessed (planless). Scoped to the PR
diff; the computed matrix was not re-run because no criterion or tag was
added or removed, only one TC-885 test split into three.

- FR-003-AC-9 / TC-885: "For each complete-V1 fixture unit in the
  repository's tests that checks, checking its formatted source through the
  S1 to S4 spine yields a checked package identity equal to the original's,
  and formatting the formatted source again yields identical bytes." The AC
  quantifies per fixture unit. Each split test covers one generator unit at
  the full 100,000 levels with the full assertion set, so a per-test
  `#[trace("TC-885", "FR-003-AC-9")]` tag is honest. The dependency-chain
  case keeps both chain workloads (3 and 1,000 libraries) and every reached
  library is still compared.
- NFR-013 / TC-915: NFR-013 says `test-deep-format` runs
  `test --locked --workspace --tests` with the default lane's selection and
  directory. That stays true; NFR-013 names no thread count. The TC-915 oracle
  is updated to the new argv and still runs against the real Makefile recipe.
- QSL-828 asks for the cause, a cut in time without lowering the proof, and
  before/after seconds. The PR body reports per-case before/after seconds
  (stage about 1500 s serial to 450 s test time) and peak memory, and names the
  remaining cost: the dependency chain is quadratic in chain length because
  each package lists its whole dependency closure (`CheckedDependencySelection`),
  a package-format change in quire-contract-model that the PR leaves out.

## Verdict

Mergeable. No AC lost coverage and no test was weakened. One low finding:
the deferred quadratic step has no ticket.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The PR identifies the remaining super-linear step (each package in an n-library chain lists its whole dependency closure, quadratic in time and memory, now the 16.5 GB / 94 s floor) and defers it as a quire-contract-model package-format change. QSL-828's own candidate list includes "fix a super-linear step", and a Linear search finds no ticket for this one. Without a ticket the cause is recorded only in a PR body. File it against quire-contract-model's checked-package format. | tests/it/formatter_identity.rs:1795-1810 |

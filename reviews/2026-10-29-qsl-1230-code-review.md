---
id: SR-60051
title: "Code review — QSL-1230 LT-16 access classification performance"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; rebased head 075502a06 plus working diff: qsl-eval/src/simulation/model/application.rs, qsl-eval/src/simulation/model/frames.rs, qsl-eval/src/simulation/model/walk.rs, qsl-eval/src/simulation/protocol/steps.rs; ticket QSL-1230"
review_set: subset
date: 2026-10-29
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: reviews
---

## Summary

Reviewed the LT-16 access-classification optimization and its callers. The location filter is derived from the precomputed potential access table, exact transition identities still determine selected work, and all candidates in the argument domain remain budgeted.

## Verdict

**PASS** — the optimization preserves access classification and candidate charging; LT-16 passes with a measured reduction from 696.92 s on the recorded clean-main baseline to 448.17 s on rebased head `075502a06`.

## Scope

| Path | Review |
| --- | --- |
| `qsl-eval/src/simulation/model/application.rs` | reviewed selected-transition entry points |
| `qsl-eval/src/simulation/model/frames.rs` | reviewed candidate identity, budget charge, and early filter order |
| `qsl-eval/src/simulation/model/walk.rs` | reviewed output initialization |
| `qsl-eval/src/simulation/protocol/steps.rs` | reviewed potential-location selection and enabled-access use |
| `qsl-analyze/tests/it/rc11_oracle.rs` | existing LT-16 behavior test exercised |

No applicable AssuranceProfile was found under `spec/`. No workflow or gate files changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Gate evidence

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass.
- `cargo clippy --workspace --all-targets`: pass.
- `make string-edge route-lint checked-input`: all three checks pass.
- `cargo test --workspace`: all suites before `qsl-package` passed; 11 `qsl-package` tests failed. On clean main `075502a06` with QSpec `5734a3719`, the C14 positive-fixture test, I2 positive-fixture test, and timed-model readback test reproduce. The failures are outside the changed crate and are listed in the PR body.
- `cargo deny check`: fails on the repository's current license policy because transitive AGPL-3.0-or-later dependencies are not explicitly allowed; no dependency or license files changed.

## Gap Analysis

`quoin gap-analysis` is not an installed Quoin 0.29.1 command (`command not found`). The ticket's measurable behavior was compared manually with the existing LT-16 oracle and the FR-220 classifier path; no specification or trace tag changed. LT-16 passed in 448.17 s on `075502a06`. A repository-wide `quire matrix --scope <repo> --format tsv` attempt emitted existing catalog diagnostics and did not finish within 60 seconds, so no repository-wide matrix result is claimed.

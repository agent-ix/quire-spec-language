---
id: SR-1988003
title: "Code review — QSL-588 statistical sample-step limit"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-replay/src/model_check/limits.rs, qsl-replay/src/statistical.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: references
---

## Summary

Reviewed the isolated QSL-588 prerequisite PR that adds the timed sample-step limit name and table row. The exhaustive mapping/test entry is correct, but the new `StatisticalLimits::value` arm returns the default for a caller-set limit.

## Verdict

**CONDITIONAL** — one medium API correctness finding remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `StatisticalLimits::value(MaxSampleSteps)` reports the fixed default instead of the caller's configured sample-step budget. | qsl-replay/src/statistical.rs:216 |

## Verification

- `cargo fmt --all -- --check`: passed.
- `cargo check --locked -p qsl-replay --no-default-features`: passed.
- `cargo clippy --locked -p qsl-replay --all-targets --all-features -- -D warnings`: passed.
- The focused `every_limit_spells_its_name_in_snake_case` test passed.
- Full `cargo test --locked -p qsl-replay --no-default-features`: 904 unit tests and 176 integration tests passed; one unrelated integration test failed. The same failure (`refinement_ring::an_exhausted_abstract_meter_stops_the_step_check`) reproduced on base `b24908841`.
- `cargo deny check`: failed on existing `agent-ix-extraction-frontend` AGPL-3.0-or-later licensing policy; this PR changes no dependency or deny configuration.
- Repository `quire matrix` and `quoin matrix` both exceeded 120 seconds without a result.

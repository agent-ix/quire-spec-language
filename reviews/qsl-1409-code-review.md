---
id: SR-1988003
title: "Code review — QSL-1409 RA view encoding"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-eval/src/simulation/protocol/ra.rs, qsl-analyze/tests/it/rc11_oracle.rs, spec/functional/FR-222-explore-the-release-acquire-memory-model.md, spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md, spec/test-cases/TC-668-rc11-partial-sc-order.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-223", type: references }
---

## Summary

Reviewed PR #1992 at `cf40d0e9c8824e8b7deb0a2b459a711d74562b3f`, including the Rust change, its call sites, the RA state-key contract, and the LT-16/LT-12 oracles. The ordered merge preserves the old output for matching entries and the old omission behavior for unmatched locations or message ids; no correctness defects were found.

## Verdict

**PASS** — the state-key encoding change is behavior-preserving on the inspected cases, and the existing litmus assertions are behaviorally meaningful.

## Coverage

- Scope: `RaComponent::view_enc`, `FR-222` state-key serialization, `FR-223-AC-8`/LT-16 and `FR-223-AC-5`/LT-12, and `TC-668` steps 5 and 7.
- The LT-16 test asserts 7,607 states and 19,574 edges and excludes the forbidden final outcome. The LT-12 helper compares RC11 outcomes against RA/SC/TSO and asserts the expected RA-only outcome count.
- The code review found no changed test, CI configuration, API, dependency, unsafe block, or cap/limit. The reviewed change does not weaken either litmus assertion.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy -p qsl-eval --all-targets --all-features -- -D warnings`: passed (exit 0, 46.33 s).
- Coder-reported focused verification on this head: LT-16 release passed with its state/edge and forbidden-outcome assertions; LT-12 RA/SC/TSO outcome test passed. The separate aggregate gate was not run during review.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

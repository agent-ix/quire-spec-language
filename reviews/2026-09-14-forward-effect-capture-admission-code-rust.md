---
id: SR-437
title: "Code and Rust review of forward-effect capture admission"
type: SpecReview
analysis: code-review
scope: "quire-spec-language#108; Plan-009/Task-042; FR-049; NFR-009; TC-142; src/state/evaluation.rs; tests/compiled_protocol_v2.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-142
    type: reviews
---

## Summary

`/rust-review` examined the QSL #108 Rust delta at PR readiness. The change
adds the one missing admitted-binder case required when a registration capture
initializer reads its authored compensation forward effect.

## Verdict

**PASS** — no actionable Rust, API-boundary, test-alignment, panic, unsafe,
conversion, resource, or stub finding remains.

## Rust review

- Production adds no public API, allocation, recursion, callback, parser,
  dynamic dispatch, lock, async path, blocking work, `unsafe`, or panic.
- Admission is not generalized: only `(Registration, ForwardEffect)` selects
  `CompensationRegistration`. The existing checks still require the exact
  anchor, compensation subject, value type, model and authority evidence.
- All other anchor/binder combinations continue to return the existing typed
  `Authority` refusal; initialized captures remain evaluator-owned and caller
  capture inputs remain `SurplusBinding`.
- TC-142 makes both Full and Partial retry/recovery expressions read the
  registration and activation captures, supplies only external source values,
  and checks missing/crossed forward inputs, direct capture substitution,
  replay and one-short accounting.
- The test helper remains test-only and constructs complete exact object
  populations; no production approximation or default value was introduced.

## Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | pass |
| Clippy, all targets, no default features, warnings denied | pass |
| Focused TC-142 initialized-capture controls | pass |
| Full no-default-feature suite | pass; 3 inherited private-packet tests ignored |
| No-default-feature build | pass |
| Fixture audit self-test and model-byte audit | pass |
| Repository constraints | no hosted CI dispatch or workflow change; `resources/native-v1/` unchanged; package remains `publish = false` |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved code or Rust finding remains. | #108; Task-042; FR-049; NFR-009; TC-142 |

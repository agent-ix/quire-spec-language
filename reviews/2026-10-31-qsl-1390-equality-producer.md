---
id: SR-1957108
title: "Code review — QSL-1390 equality producer data"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/check.rs, qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/ir.rs, qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/claims/tests.rs, qsl-semantics/src/check/family.rs, qsl-semantics/src/check/state_model.rs, qsl-semantics/src/check/sum_case.rs, qsl-semantics/src/check/termination.rs, qsl-package/src/emit/tests/converted_equality.rs"
review_set: subset
---

## Summary

Reviewed the QSL-owned equality producer facts retained from typing through lowering, including nested equality applications, and the focused producer tests for QSpec EQ-01..05. The slice does not change the IR wire writer; host no-default checks, Clippy, formatting, and focused tests pass. The embedded-target `make build-no-std` gate fails in this environment because rustc cannot find `core` for `thumbv7em-none-eabi`, despite rustup listing that target as installed.

## Verdict

**PASS** — no code-review findings; the target-specific build failure is environmental and is recorded in the check evidence.

## Findings

| ID      | Severity | Summary                          | Refs |
| ------- | -------- | -------------------------------- | ---- |
| FND-001 | low      | No findings (placeholder)        | -    |

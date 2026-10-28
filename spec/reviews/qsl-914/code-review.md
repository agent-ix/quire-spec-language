---
id: SR-17020
title: "Code review of QSL-914 refinement rows and whole-model roles"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-914 diff from <sha removed>: qsl-semantics/src/check/{check.rs,mod.rs,protocol_clause/{coverage.rs,mod.rs,types.rs},protocol_controls.rs,protocol_slots.rs,refinement.rs}, qsl-semantics/tests/it/{refinement_declarations.rs,refinement_protocol.rs}"
review_set: subset
---

## Summary

The direct Rust review covered the changed checker paths and the FR-135-AC-8 and FR-218-AC-24 behavior. Refinement row expressions are admitted from the pinned value-complete and model-complete layer closures independently of the unit header, and the first protocol reference is refused at its expression span with `expression-form` and the protocol layer identity. A static role on a model alias retains that model selection identity; replicated roles refuse at the alias without a population refusal. No applicable AssuranceProfile was found. The scoped computed matrix tags both changed criteria. No workflow files changed.

## Verdict

**CONDITIONAL.** Changed-scope integration tests and the package all-target check pass. The complete all-features package test command has 11 failures in unchanged test areas, and the required whole-repository gap-analysis command is unavailable in the installed Quoin release.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `cargo test -p qsl-semantics --all-features` fails 11 tests outside the changed behavior: enum name resolution, checked-dispatch span expectations, hybrid typing and protocol observation/anchor cases. The targeted refinement declaration and protocol suites pass, so this does not identify a QSL-914 regression; it limits the package-wide test evidence. | qsl-semantics/src/check/assemble/tests.rs:1756; qsl-semantics/src/check/checked_dispatch.rs:1916; qsl-semantics/src/check/hybrid/tests.rs:661; qsl-semantics/src/check/protocol_clause/tests.rs:128; qsl-semantics/src/check/protocol_controls/model_state_tests.rs:336 |

## Coverage

The scoped `quire matrix` reports FR-135-AC-8 and FR-218-AC-24 as `tagged` by the changed integration test file. The other untagged criteria reported from those two full requirements are outside QSL-914's touched criteria. `cargo fmt --all -- --check`, `cargo check -p qsl-semantics --all-targets`, the 29 refinement declaration integration tests, the 25 refinement protocol integration tests, and all three new focused tests passed. `cargo clippy -p qsl-semantics --all-targets --all-features -- -D warnings` stops on three pre-existing `large_enum_variant` warnings in `qsl-forms/src/syntax.rs`. The full package test result is recorded in FND-001. Quoin 0.29.1 does not provide the `gap-analysis` command named by the installed skill; the direct review and scoped static matrix were used. Plan completion: not assessed.

## Dispositions

- FND-001: deferred: this review is limited to QSL-914, and the failing cases are in unchanged test areas; confirm the full package suite against the next clean main baseline before claiming a green package-wide test gate.

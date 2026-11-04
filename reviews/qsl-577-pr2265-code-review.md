---
id: SR-2265001
title: QSL-577 PR #2265 code review
type: SpecReview
analysis: code-review
date: '2026-11-04'
scope: 'agent-ix/quire-spec-language; PR #2265 at 2421444fe429fce556931ab674d64426f666dbff; selected-package delay fixture and regression'
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: references
---

## Summary

Static code and Rust review of the two changed integration-test files. The fixture helper admits the primary and additional package documents together. The regression first asserts that an exponential delay on subject `M` is refused, then selects `Other`, gives `Other::Call::send` an exponential delay, and builds the digital-clock product for claim `Deadline` over `M`. The product assertion would fail if admission rejected every delay declaration in the package graph; current admission filters to the claim subject alias. No test logic or fixture validity defect was found.

`cargo fmt --all -- --check` and `git diff --check origin/main...HEAD` passed. No build, Clippy or test was run, per reviewer instruction. The final head also tags the regression with `FR-204-AC-3` and `TC-639`. The observed PR checks are CLA success, internal CLA success and external CLA skipped; they provide no Rust gate result.

## Scope

```yaml
scope:
  - id: delay_admission_is_scoped_to_the_selected_subject_model
    path: qsl-analyze/tests/it/digital_clocks.rs
    role: examined
    excerpt: "The selected Other package declares exponential delay; the probabilistic product is requested over M and must build as an MDP."
  - id: retx_document
    path: qsl-analyze/tests/it/digital_clocks.rs
    role: examined
    excerpt: "Builds the primary package document with the operation parameter needed by the Retx fixture."
  - id: world_built_with_packages
    path: qsl-analyze/tests/it/timed_world.rs
    role: examined
    excerpt: "Passes the primary and additional domain documents through package_input before admitting the unit's selections."
  - id: admit
    path: qsl-analyze/src/exact_probabilistic/digital.rs
    role: context_only
    excerpt: "Delay admission only considers operations whose model alias equals the checked probabilistic claim's subject alias."
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No code or behavior findings (placeholder) | - |

## Verdict

CONDITIONAL: static review found the fixture and regression sound. Runtime gates were not run by instruction.

## Coverage

Rust review and the Rust lane of code-review were applied to the changed integration tests. The scenario uses the real model-admission and product-building path, not mocks. The added helper reuses the existing fixture construction path. No production or CI workflow files changed. No applicable AssuranceProfile was found under `spec/`. Test trace mapping is recorded in the separate gap-analysis artifact.

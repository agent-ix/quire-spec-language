---
id: SR-20261110094156900001
title: code-review review of weighted long-run admission precedence clarification
type: SpecReview
analysis: base
scope: agent-ix/quire-spec-language; QSL-1826 PR2656; spec/functional/FR-201-check-a-probability-certificate.md,
  spec/test-cases/TC-1402-check-weighted-dtmc-long-run-certificates.md
review_set: subset
---

## Summary

Reviewed correction PR2656 frozen 8a73900e30e3b36896283597b803622be8d26232 against 19bf7f0b906f351be9506e8c745817666b54be77. This two-file clarification is specification only.

## Verdict

**PASS** — The two-file specification-only diff preserves the existing implementation boundary and changes no runtime, API, transport, dependency, limit or solver. No duplicate semantic owner, pin, tracking ledger or fabricated admitted subject is added. TC1402 now distinguishes the actual admission refusal from defensive profile checks.

## Assurance Context

No applicable active AssuranceProfile exists in the local spec scope. Direct delegated subset review; no enforced workflow or full-runtime acceptance is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Existing model-admission\n  refusals, including FR-187's `invalid_probabilistic_model`/`negative-reward`,\n\
    \  SHALL be preserved before profile evaluation; negative weights or\n  unsupported\
    \ reward types encountered after admission SHALL preserve\n  explicit unsupported\
    \ outcomes;"
- id: TC-1402
  path: spec/test-cases/TC-1402-check-weighted-dtmc-long-run-certificates.md
  role: examined
  excerpt: "Distinguish FR-187's earlier finite-domain negative-reward\n   admission\
    \ refusal from defensive post-admission weight checks; do not\n   relabel the\
    \ earlier refusal as profile unsupported. "
- id: FR-187
  path: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  role: context_only
  excerpt: "When `ModelSystem::new` admits a model, it SHALL evaluate every reward\
    \ on\n  every argument vector of its operation's finite parameter domains, a random\n\
    \  parameter ranging over its support, and refuse admission with the\n  refusal\
    \ record `invalid_probabilistic_model`/`negative-reward`, carrying\n  `operation`,\
    \ `reward` and `arguments`, for the first negative value in\n  canonical order."
```

## Evidence and limits

Source inspected at qsl-eval/src/simulation/model.rs:939, probabilistic.rs:479-510 and probabilistic/tests.rs:366-392. FR187:105-110 owns the finite-domain refusal. Scoped normative validation passed 2/2 grammar-clean; no cargo, runtime tests, whole matrix, solver or whole-expansion budget qualification was run. Dependency/object/risk/evidence lenses add no new subject for this two-file error-precedence clarification and were not selected.

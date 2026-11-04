---
id: SR-164204
title: "Scope-boundary review — QSL-1642 timed population scope"
type: SpecReview
analysis: scope-boundary
scope: "spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md; spec/functional/FR-391-name-a-population-in-a-state-clause.md; spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: references
---

## Summary

The change assigns timed-predicate scope to the QSL S3 timed-model checker, while FR-391 remains the owner of named-population semantics for state clauses and `holds` atoms. QSpec's grammar and package population declarations remain external inputs.

## Verdict

**PASS** — the responsibility boundary is explicit and no new external dependency is introduced.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found. | - |

## Responsibility Allocation

| Requirement | Owning component | Class |
| --- | --- | --- |
| FR-230-AC-55 | QSL S3 timed-model predicate checker | core |
| FR-391 named-population positions | QSL clause checker | core |
| FR-480 model-alias resolution | QSL package assembler | core |

## External Dependencies

| Dependency | Boundary | Treatment |
| --- | --- | --- |
| QSpec FR-415 timed syntax | Shared grammar and source forms | Assumed; grammar is unchanged |
| Admitted package population declarations | Population identity and member type | Consumed through existing QSL package admission |

## Scope

The change does not add an ambient population binding to timed predicates, alter `self`, or change population reads in clauses and temporal atoms. No transport, persistence, or new package contract is added.

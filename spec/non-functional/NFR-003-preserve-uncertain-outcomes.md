---
id: NFR-003
title: "Preserve refusal and incompleteness through the pipeline"
type: NFR
quality_attribute: reliability
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-005"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-006"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-106"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-107"
    type: constrains
---
# NFR-003: Preserve refusal and incompleteness through the pipeline

## Statement

If a required pipeline stage refuses or remains incomplete, then the compiler shall withhold a completed logical result.

## Scope

Source processing through linking, validation, reference evaluation and backend qualification.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Boolean outcomes from incomplete stages | 0 | 0 | Adverse pipeline cases |
| Dropped unsupported obligations | 0 | 0 | Obligation census comparison |

## Verification

Exercise missing models, unsupported features, invalid populations and exhausted work. A parser-only test run cannot satisfy the unimplemented evaluator cases.

## Dependencies

- [FR-005](../functional/FR-005-link-shared-model.md)
- [FR-006](../functional/FR-006-check-defined-expressions.md)
- [FR-106](../functional/FR-106-admit-snapshots-and-invocations.md)
- [FR-107](../functional/FR-107-evaluate-state-clauses-at-s6a.md)
- [NFR-009](../non-functional/NFR-009-bound-composed-evaluation.md)

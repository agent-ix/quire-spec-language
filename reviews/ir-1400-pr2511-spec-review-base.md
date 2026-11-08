---
id: SR-2511003
title: base review of QSL PR 2511
type: SpecReview
analysis: base
scope: agent-ix/quire-spec-language; IR-1400; spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md,
  spec/test-cases/TC-1339-fallible-declared-type-check-consumption.md
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-534
  type: references
---

## Summary

Review of IR-1400 consumer PR 2511 against its frozen source and specification. No applicable AssuranceProfile is declared in the repository. All four criteria were read from the parsed Quire AST. FR-534 and TC-1339 validate; no ID, link, scope, error-contract, hash/digest/pin/tracking-record or test-procedure defect was found. Tests distinguish real allocator denial from synthetic capacity controls and preserve the owner API as the sole traversal owner.

## Verdict

**PASS** — no confirmed defect in this method; the lead’s final combined-tree gate remains required.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

All four criteria were read from the parsed Quire AST. FR-534 and TC-1339 validate; no ID, link, scope, error-contract, hash/digest/pin/tracking-record or test-procedure defect was found. Tests distinguish real allocator denial from synthetic capacity controls and preserve the owner API as the sole traversal owner.

## Examined scope

```yaml
scope:
- id: FR-534-AC-1
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: With allocation enabled, valid declared scalar, nested collection, composite,
    reference and enum/variant fixtures keep their previous outcomes; missing declarations
    and IEEE-bearing set elements retain their existing semantic cause, original source
    location and first-refusal priority, including a fixture with more than one possible
    semantic failure. Evaluator collection/equality owner-API checks retain their
    original assertions.
- id: FR-534-AC-2
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: A real allocator denial during the owner's production type-check traversal
    reaches the QSL declared-type consumer as a located typed allocation refusal at
    Typing; the diagnostic is resource_exhausted/allocation-failed with exactly requested
    containing the owner's actual native unit and amount. No bound, setting, type-mismatch
    or stage-limit replacement appears.
- id: FR-534-AC-3
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: Allocation and capacity owner errors carrying both native units preserve
    class, amount and unit independently through the QSL mapping and diagnostic projection;
    capacity projects resource_exhausted/reservation-size-overflow with exactly upstream
    and no invented configured bound or allocator-denial claim.
- id: FR-534-AC-4
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: After a real owner-traversal allocation failure, the admitted environment
    and input type equal their pre-check values, no successful checked result escapes,
    and the identical check after restoring allocation agrees with a fresh successful
    check.
- id: FR-534 Description
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: 'When the expression checker validates a declared value type against its
    shared

    `TypeEnvironment`, the checker SHALL consume the owner''s fallible

    `try_check_type` result without changing semantic refusal behavior or losing a

    storage failure''s class, native request amount or native unit.'
- id: FR-534 Inputs
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: 'The admitted shared type environment, declared `ValueType` and existing
    source

    `Location` at `Typer::check_declared_type`.'
- id: FR-534 Outputs
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: 'Successful validation, the existing located semantic `CheckRefusal`, or
    a

    located typed storage refusal at the existing `CheckingStage::Typing` stage.

    No checked result is produced by a failed check.'
- id: FR-534 Behavior
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: 'When the owner returns `Ok(())`, the checker SHALL continue its existing

    QSL-owned enum and variant binding validation.

    When the owner returns `TypeCheckError::Refused(IllTyped)`, the checker SHALL

    apply the existing `CheckRefusal::from_ill_typed` mapping at the original

    location, preserving diagnostic code, cause, refusal priority and order.


    When the owner returns `TypeCheckError::Allocation(StorageRequest)`, the

    checker SHALL retain the allocator-denial class and exact native request in

    its typed refusal and diagnostic projection.

    When the owner returns `TypeCheckError::Capacity(StorageRequ'
- id: FR-534 Dependencies
  path: spec/functional/FR-534-preserve-fallible-declared-type-check-failures.md
  role: examined
  excerpt: '- [QSV FR-132](ix://agent-ix/quire-semantic-value/FR-132) owns fallible
    type traversal, reservation measurement and semantic ordering.

    - [TC-1339](../test-cases/TC-1339-fallible-declared-type-check-consumption.md)
    defines consumer verification.'
- id: TC-1339
  path: spec/test-cases/TC-1339-fallible-declared-type-check-consumption.md
  role: examined
  excerpt: 'Verify [FR-534](../functional/FR-534-preserve-fallible-declared-type-check-failures.md)

    through QSL''s actual declared-type consumer and diagnostic projection. The

    semantic controls run with allocation enabled and sufficient existing limits.

    The resource controls distinguish a real production allocator denial from

    small, explicit owner-error mapping controls.'
bindings: []
```

## Validation evidence

Focused declared-type tests, replay native-field projection, enum binding and variant checking controls passed in the coder’s local logs. Scoped three-crate check and Clippy passed. The reviewer inspected these logs and the production call chain; no competing Cargo or aggregate gate was started. The lead retains the final combined-tree gate.

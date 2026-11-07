---
id: FR-534
title: "Preserve fallible declared-type check failures"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-025
    type: references
  - target: ix://agent-ix/quire-semantic-value/FR-132
    type: depends_on
---
# FR-534: Preserve fallible declared-type check failures

## Description

When the expression checker validates a declared value type against its shared
`TypeEnvironment`, the checker SHALL consume the owner's fallible
`try_check_type` result without changing semantic refusal behavior or losing a
storage failure's class, native request amount or native unit.

## Inputs

The admitted shared type environment, declared `ValueType` and existing source
`Location` at `Typer::check_declared_type`.

## Outputs

Successful validation, the existing located semantic `CheckRefusal`, or a
located typed storage refusal at the existing `CheckingStage::Typing` stage.
No checked result is produced by a failed check.

## Behavior

When the owner returns `Ok(())`, the checker SHALL continue its existing
QSL-owned enum and variant binding validation.
When the owner returns `TypeCheckError::Refused(IllTyped)`, the checker SHALL
apply the existing `CheckRefusal::from_ill_typed` mapping at the original
location, preserving diagnostic code, cause, refusal priority and order.

When the owner returns `TypeCheckError::Allocation(StorageRequest)`, the
checker SHALL retain the allocator-denial class and exact native request in
its typed refusal and diagnostic projection.
When the owner returns `TypeCheckError::Capacity(StorageRequest)`, the checker SHALL retain the distinct reservation-size-overflow class and exact native
request in its typed refusal and diagnostic projection.
The diagnostic convention follows
[FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md):

| Owner failure | Code | Cause | Payload |
| --- | --- | --- | --- |
| Allocation | `resource_exhausted` | `allocation-failed` | Exactly `requested: <unit>:<amount>` |
| Capacity | `resource_exhausted` | `reservation-size-overflow` | Exactly `upstream: <unit>:<amount>` |

Here `<unit>` is the owner's `bytes` or `additional-elements` and `<amount>`
is the unpadded decimal `StorageRequest::amount()`. Neither payload denotes a
configured limit. QSL SHALL preserve these failures through the existing
located checking and downstream diagnostic boundaries without substituting
`IllTyped`, `TypeMismatch`, a stage limit, cancellation or an internal fault.
QSL SHALL retain the owner-native failure information without cloning or
reimplementing the owner's type environment or introducing a compatibility
`check_type` shim. QSL SHALL carry the owner's error or native `StorageRequest`
directly rather than introducing mirror request or failure types.
The resource-failure carrier SHALL avoid allocating a box
solely to wrap an allocator-denial payload.

When validation fails, the checker SHALL leave the caller-visible admitted
environment and input type unchanged and expose no successful checked result.
When allocation is restored, the checker SHALL permit an identical retry to
yield the same result as a fresh check.

This requirement covers the owner API call in declared-type checking and the
QSL evaluator test consumers of that owner API. The QSL-owned enum-validation
worklist, unrelated methods named `check_type`, and a general allocation audit
are outside this change. Existing configured admission limits under
[FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md) and source
limits under [FR-025](FR-025-compile-rule-model-source.md) retain their contracts.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-534-AC-1 | With allocation enabled, valid declared scalar, nested collection, composite, reference and enum/variant fixtures keep their previous outcomes; missing declarations and IEEE-bearing set elements retain their existing semantic cause, original source location and first-refusal priority, including a fixture with more than one possible semantic failure. Evaluator collection/equality owner-API checks retain their original assertions. | Test |
| FR-534-AC-2 | A real allocator denial during the owner's production type-check traversal reaches the QSL declared-type consumer as a located typed allocation refusal at Typing; the diagnostic is resource_exhausted/allocation-failed with exactly requested containing the owner's actual native unit and amount. No bound, setting, type-mismatch or stage-limit replacement appears. | Test |
| FR-534-AC-3 | Allocation and capacity owner errors carrying both native units preserve class, amount and unit independently through the QSL mapping and diagnostic projection; capacity projects resource_exhausted/reservation-size-overflow with exactly upstream and no invented configured bound or allocator-denial claim. | Test |
| FR-534-AC-4 | After a real owner-traversal allocation failure, the admitted environment and input type equal their pre-check values, no successful checked result escapes, and the identical check after restoring allocation agrees with a fresh successful check. | Test |

## Dependencies

- [QSV FR-132](ix://agent-ix/quire-semantic-value/FR-132) owns fallible type traversal, reservation measurement and semantic ordering.
- [TC-1339](../test-cases/TC-1339-fallible-declared-type-check-consumption.md) defines consumer verification.

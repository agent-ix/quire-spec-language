---
id: TC-1339
title: "Fallible declared-type checks preserve semantics and native storage failures"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-534
    type: verifies
  - target: ix://agent-ix/quire-semantic-value/FR-132
    type: references
---
# TC-1339: Fallible declared-type checks preserve semantics and native storage failures

## Description

Verify [FR-534](../functional/FR-534-preserve-fallible-declared-type-check-failures.md)
through QSL's actual declared-type consumer and diagnostic projection. The
semantic controls run with allocation enabled and sufficient existing limits.
The resource controls distinguish a real production allocator denial from
small, explicit owner-error mapping controls.

## Test Procedure

1. Check valid scalars, nested collections, admitted composites and references,
   and QSL-owned enum/variant declarations. Check missing composite/reference
   declarations, IEEE-bearing set elements and a type containing multiple
   semantic failures. Assert established success or semantic refusal code,
   cause, exact location and first-refusal ordering. Retain the existing
   collection algebra and equality matrix assertions while migrating their
   direct owner API calls.
2. Construct the environment, input type and expected successful result before
   arming allocation denial. Use an isolated real allocator fixture to deny a
   reservation inside the owner's production `try_check_type` traversal as
   invoked by QSL's `Typer::check_declared_type`. Disarm before assertions or
   diagnostic rendering. Capture the actual denied request and assert the
   returned located typed cause retains the owner's class, native unit and
   amount. Assert the downstream diagnostic remains at Typing and projects
   resource_exhausted/allocation-failed with exactly
   `requested: <unit>:<amount>`, with no configured bound or setting. A
   fabricated error, configured-budget denial or denial solely in the later
   QSL enum worklist does not satisfy this step.
3. Exercise the actual QSL owner-error conversion and diagnostic projection
   with explicit `TypeCheckError::Allocation` and `TypeCheckError::Capacity`
   values for distinct requests in bytes and additional elements. Assert
   equality of the retained request's unit and amount, distinct typed classes,
   original location and Typing stage. Capacity yields exactly
   `upstream: <unit>:<amount>` and reservation-size-overflow; allocation yields
   exactly `requested: <unit>:<amount>` and allocation-failed. These mapping
   controls verify otherwise unreachable capacity branches without claiming
   that an allocator denial produced capacity overflow.
4. Compare environment and input with their pre-denial values; assert the
   failed check returned no successful checked result. With denial disabled,
   retry the identical check and compare its result with the fresh successful
   control.

Use the existing repository test organization: new integration cases belong
in `tests/it` with their existing integration root registration. If accessing
the private consumer or installing a process-global allocator requires an
isolated library-unit fixture or dedicated test binary, document that reason
and keep allocator state isolated from unrelated tests. The owner's existing
`qsv-failing-allocator` fixture exposes `FailingAllocator`, `arm` and `disarm`
as dev-only APIs; reuse that fixture or existing QSL fault controls where
available, without introducing a replacement test framework. Do not require a
production injection API solely for the test.

Tag implemented tests with `TC-1339` and the relevant `FR-534-AC-1` through
`FR-534-AC-4` identifiers. File/source-text presence assertions do not verify
these behavioral criteria.

## Expected Results

Semantic outcomes retain their established mapping, location and ordering.
The real owner-traversal denial reaches QSL with its exact native request;
capacity and allocation remain distinct through typed and diagnostic results.
Neither becomes an invented limit or a semantic failure. The environment and
input remain unchanged, and removing denial permits a successful identical
retry.

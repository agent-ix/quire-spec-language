---
id: SR-322
title: "Failure-domain review of composed values and rational model admission"
type: SpecReview
analysis: failure-domain
scope: "FR-040/041; TC-119/120; referenced native state definitions and contracts"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-041
    type: reviews
---

## Summary

PASS. This is the selected failure-domain lens accompanying SR-321, evaluated against the actual source/model/IR boundaries and the [state contract](../../../resources/native-v1/proposals/quire-v1/state-contract.md).

Strict frontend/admission failures preserve source correspondence and expose no partial model. Explicit `/1` and `/2` selection, nominal role/site ownership, and separate source/native/IR digest domains prevent identity substitution. Zero denominator, signed-width and normalized-domain failures are distinguished from resource incompleteness and unsupported proof prerequisites; a selected `/2` artifact cannot enter historical linking.

Predicates, guards and ordered queries are pure; every authored type/profile form is checked even when skipped at evaluation. Immutable receiver/observation anchors constrain presence facts and `pre`; sum checks every prefix. Acyclic predicate dependencies remain distinct from cyclic references: finite-graph rules specify positive-length paths, expansion limits and disconnected/self-loop behavior. Depth, shared visits, exhaustion propagation and fresh retries are explicit obligations.

TC-119/120 name adverse controls for these boundaries. FR-040 remains the full planned checking contract; producer admission or typed-only evidence cannot satisfy its proof, runtime-input or execution obligations. This review executed no Rust tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No additional failure-domain requirement is needed for this scope. | [FR-040](../../functional/FR-040-check-composed-values.md), [FR-041](../../functional/FR-041-admit-rational-native-model-profile.md), [State graph](../../../resources/native-v1/proposals/quire-v1/definitions/state-graph.md) |

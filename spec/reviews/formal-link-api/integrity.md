---
id: SR-048
title: "integrity review of the formal linker API"
type: SpecReview
analysis: integrity
scope: "FR-013, formal-environment-linking.md, TC-020–024, TC-030–034 and IT-005 linking seam"
review_set: all
evaluated_revision: "c78792a"
review_date: "2026-09-08"
---

## Summary

FR-013 specifies the actual input/output and ambiguity seam rather than another model service.

## Verdict

PASS for the concrete linking specification and implementation readiness.
The complete state workflow remains open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No unresolved finding in this scoped API specification. | FR-013; IT-005; TC-030–034 |

## Analysis

FR-013 traces through US-002 to StR-001 and refines FR-005 with six Test criteria. Its public output borrows immutable IR environments and owns the parsed source, making mutation and source replacement impossible through that API. The initial context has an explicit self State declaration; layouts cannot synthesize it. Shape propagation only enables name lookup and does not imply guarded definedness, unit equality or Boolean roots. Current invariant linkage therefore remains distinct from FR-006 acceptance. Existing resource and diagnostic requirements remain scoped; the new bounds and related locations are explicit in the API contract. Canonicalization failure keeps its structured upstream diagnostic.

## Provenance

The actual QUOIN specify/spec-matrix/spec-review skills and retained base plus
seven analyses govern this review. Agent A performed the selected reviews
serially under the no-subagent assignment. Catalog authoring contracts, actual
advisor output and validation were read. The declined optional gap-analysis
semantic comparison is not included. This is not an independent B/C acceptance.

## Fixture correction re-review

IT-005 consistency re-review at 3e8d348: PASS for the corrected specification.
The IT and TC field spelling now agree. The signed domain was already required; no new type-system behavior, coercion or weaker acceptance criterion is introduced.


Evaluated ecaf4cf before continuing the corrected qualification implementation.
PASS for this correction under the previously selected all-analysis review set.
The exact owner().package().as_str() mapping is unchanged. The count field is an authored fixture declaration, not a rename performed by the linker or a second authority.

---
id: SR-1933003
title: "Failure-domain review for FR-493 retirement ordering"
type: SpecReview
analysis: failure-domain
scope: "Deletion ordering, orphaned citations and shared modules in FR-493"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1276
    type: reviews
---

## Summary

The main risks are deleting a module another lane still imports and leaving citations to deleted requirements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Deleting `formal_source` would break `checking::composed` and the handoff writer. Fixed in FR-493. | FR-493, src/checking/composed.rs |
| FND-002 | medium | The compiled-protocol reader is a dependency of the handoff writers and composed emission; deleting it first strands them. Fixed by ordering in FR-493-AC-4. | FR-493-AC-4 |
| FND-003 | medium | Retained requirements cite retired ones (FR-100, FR-031, FR-277, NFR-008, NFR-009, NFR-003, IT-002, US-003, StR-001, FR-104, FR-098). Between this specification and the code slice the citations are still valid; FR-493 lists them so the slice re-points them. No earlier gate checks it. | FR-493 retained table |
| FND-004 | low | TC-137 C2 stands in for an allocator refusal with an injected denial; it proves the typed record for a non-limit denial, not a real allocation failure. | NFR-009-AC-3 |

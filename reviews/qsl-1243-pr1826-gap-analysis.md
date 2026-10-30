---
id: SR-1826002
title: "PR #1826 (QSL-1243) gap analysis: FR-281-AC-2, QSpec FR-368-AC-17, AC-20, FR-425, FR-429, FR-438"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-analyze/tests/it race_freedom.rs, derived_request.rs, weak_memory.rs, litmus.rs at d690d8299"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: reviews
---

# PR #1826 gap analysis

## Summary

Ticket: QSL-1243. Plan completion: not assessed.

**Covered.**
- **FR-281-AC-2 over a protocol subject.**
  - The written deadlock-freedom and race-freedom items run on EN-1 through `derived_protocol_item` and settle at their request indices.
  - With the threads storing to different cells, both are `proved`.
  - With both storing to `x`, race-freedom is `refuted` with the replayed race on `x`, and deadlock-freedom is `proved`.
  - The run's `max_depth` of 1 is dropped, and the probe confirms the test catches it.
- **QSpec FR-368-AC-20.**
  - `max_states` 1 gives `limit-reached`.
  - `max_messages` 2 reaches both the run and the certificate (the record states 2, and the result is `proved`). The probe confirms the certificate half.
  - An unset limit keeps the default 4.
- **Item mismatches.** A formula item is refused as `NotDerived`, and a run or certificate for another item as `OtherItem`.
- **FR-425 with FR-368-AC-17.**
  - `ra` and `tso` items over `SB` give two protocol subjects, each with its own deadlock-freedom and race-freedom items, in first-occurrence order.
  - One clause under two sets of models is two items.

## Verdict

Mergeable. One low gap. The test for SR-1826001 FND-001 belongs with its fix.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | **Only `max_messages` is tested as reaching both the run and the certificate check.** No test sets `max_store_buffer` (under `tso`) or `max_live_instances` (an `on each` protocol) through `derived_protocol_item`. Dropping one of the six assignments would go unseen. The probe shows the `max_messages` test does catch its own case. | QSpec FR-368-AC-20, FR-439, FR-428 |

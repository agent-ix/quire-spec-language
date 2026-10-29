---
id: SR-1813002
title: "PR #1813 (QSL-1235) gap analysis: QSpec FR-368-AC-14, AC-17, AC-22, FR-438-AC-2, FR-211, FR-224-AC-3"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-analyze/tests/it race_freedom.rs, protocol_system.rs, derived_request.rs, weak_memory.rs, litmus.rs at 3271b6a80"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: reviews
---

# PR #1813 gap analysis

## Summary

Ticket: QSL-1235. Plan completion: not assessed.

**Covered.**
- **FR-368-AC-14 (producer half).** The protocol clause node's `declaration` anchor. The source map is checked to hold it.
- **FR-368-AC-17.** The six-item order across a protocol subject and a model subject over the same model.
- **FR-438-AC-2 / FR-224-AC-3.** The race item for two items over one weak subject with a non-atomic access, and none under `sc` or with all accesses atomic. Its identity differs between `tso` and `ra`.
- **FR-211 / FR-429.** `terminal any` gives no deadlock-freedom item, and two items over `Fill` give one.

## Verdict

Mergeable. One low gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | **The `NoProtocolNode` path is only counted, never pinned, and QSpec does not state it.** No test asserts that a protocol subject with no clause node gets an `unanchored` entry with cause `NoProtocolNode`, the `unsupported_projection` detail and no request item. TC-669 counts these items together with written ones, so the cases are indistinguishable there. QSpec FR-368 says a protocol subject's anchor "always names a `source_map` entry" and does not cover a clause with no node. Settling it `unsupported`, by analogy with FR-331's unemitted package, is reasonable, but it is QSL's own reading. Add the assertion, and raise the spec silence with the planner. | QSpec FR-368-AC-22, FR-368 "Claim and anchor", FR-331 |

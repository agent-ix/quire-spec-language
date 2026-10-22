---
id: SR-14074
title: "EARS review of STD-247 FR-223 fence lifetime statement (PR #1194, QSpec PR #333)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@a3c2673b34bb35ab546454473df4188732a87c3d; spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md; agent-ix/quire-specification@e69cade8882527e8f53844c9fc14e5b70116ff90; spec/functional/memory/FR-437-explore-release-acquire-memory.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
---
# SR-14074: EARS review of STD-247 FR-223 fence lifetime statement

## Summary

Ticket: STD-247. The one new requirement statement is FR-223's fence-lifetime bullet. FR-437's Fence lifetime bullet is descriptive and has no SHALL. `make check-spec-validation` reports no EARS warning on FR-223.

Units examined (role, id, path):

- examined: FR-223 fence bullet (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The bullet joins two obligations with two SHALLs: a state-driven 'SHALL keep a `seq_cst` fence live while ...' and an event-driven 'SHALL remove it from `P` when no view does'. Split it into two statements, or keep one SHALL: 'While some view ... covers a `seq_cst` fence as the latest live fence of its thread, `Ra` SHALL keep it in `P`, and no longer.' | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:75-77 |

## Verdict

One low finding.

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

## New findings (disposition pass 1)

Reviewed at QSL b84ca6c3081eae373223c0e8712129d8244f1de5 (#1194, own diff against origin/std-ra-246-qsl f12e88a34; fix commit b84ca6c30) and QSpec eadbce6aa1d4dce3908a8242acc4be9f07574a3d (#333, own diff against origin/std-ra-246-qspec 0943c2c; fix commit eadbce6). The RA-5 change FND-001 and FND-002 rest on landed in the stack base, QSL 15786c00f (#1192, STD-239) and QSpec 84dc697 (#331).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | The new fence statement draws two warnings from `make check-spec-validation` at b84ca6c30, which the base f12e88a34 does not have: FR-223 line 84 `[ears:unclassifiable]` and `[ears:missing-subject]`. The cause is the line wrap between `` `Ra` `` and `SHALL`: the same text with `` `Ra` SHALL keep the fence in `P`, and no longer. `` on one line validates clean (checked on a scratch copy). The #1194 body's "no diagnostics on touched files" is therefore not true at this head. Rewrap so `` `Ra` `` begins the line that carries `SHALL`. | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:82-84 |

## Dispositions

Round 1, reviewed at QSL b84ca6c3081eae373223c0e8712129d8244f1de5 (#1194, own diff against origin/std-ra-246-qsl f12e88a34; fix commit b84ca6c30) and QSpec eadbce6aa1d4dce3908a8242acc4be9f07574a3d (#333, own diff against origin/std-ra-246-qspec 0943c2c; fix commit eadbce6). The RA-5 change FND-001 and FND-002 rest on landed in the stack base, QSL 15786c00f (#1192, STD-239) and QSpec 84dc697 (#331).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | One state-driven statement with one SHALL. Its line wrap trips the EARS lint, recorded as FND-002. After: - While some view, at rest or in flight on a channel, has a `seq_cst` fence as the latest live fence of its thread that the view's frontier covers, `Ra` SHALL keep the fence in `P`, and no longer. |

## Dispositions (pass 2)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-002 | fixed | FR-223's fence-lifetime statement is rewrapped so `Ra` and `SHALL` are on one line. `make check-spec-validation` reports no diagnostic on FR-223 at the new head. |

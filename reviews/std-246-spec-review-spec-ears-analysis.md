---
id: SR-14071
title: "EARS review of STD-246 FR-223 collection statement (PR #1193, QSpec PR #332)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@851e3a6a155b7b7c17ee088688e83d49139b68ad; spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md; agent-ix/quire-specification@eba4eabe67c00ba55c36230429451fb06719183a; spec/functional/memory/FR-437-explore-release-acquire-memory.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: reviews
---
# SR-14071: EARS review of STD-246 FR-223 collection statement

## Summary

Ticket: STD-246. The new or changed requirement statements are FR-223's collection bullet and its key bullet. ADR-025 rows, FR-437 bullets (descriptive, no SHALL) and AC rows are not EARS statements. `make check-spec-validation` reports no EARS warning on FR-223.

Units examined (role, id, path):

- examined: FR-223 collection bullet (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- examined: FR-223 key bullet (spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The event-driven statement has two unclear parts. '(or, for a read, every later write to it)' drops 'SC', so it reads as all later writes, while PSC-4(a) and `F(ℓ)` apply only to later SC writes; for a removed write the parenthetical describes the same set. 'a thread whose view covers it' does not say which view (the current view at the later event) or what 'covers' means (the frontier test of PSC-2). Use: '... of every later SC write to its location, and of every later SC event on its location by a thread whose current view's frontier covers it'. | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:70-74 |

## Verdict

One low finding. The key bullet is a well-formed ubiquitous statement with one subject and one SHALL.

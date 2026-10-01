---
id: SR-946
title: "Base review of PR #557 (delete RevisionPin from ADR-013 and ADR-011)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@cdbc1fdb1077fe1432e7903b23af9af2bb1b1dc4; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: subset
---

## Summary

Ticket: QSL-361. Base checklist over the PR diff (`git diff origin/main...HEAD`,
one commit cdbc1fdb): ADR-013 O-23 (pin-representation paragraph and its
equality line deleted), ADR-013 §3.1 intro ("nine" to "eight") and T-9 row
deleted, ADR-013 §9 OBS-034 row trimmed, ADR-013 §10 "T-1 to T-9" to "T-1 to
T-8"; ADR-011 §9 OBS-034 (secondary) row trimmed and the ADR-011 "Answered by
ADR-013" paragraph (T-9 dropped, list rejoined with "and").

No IDs are renumbered: T-9 was the last ADR-013 §3.1 row, so T-1 to T-8 keep
their ids. No code uses `RevisionPin`: `git grep -i 'RevisionPin|revision_pin'`
finds nothing in QSL at HEAD (outside review history) or in origin/main of
quire-contract-ir, quire-contract-codegen, quire-contract-runtime and
quire-specification. No test embeds either ADR: no `include_str!`/`include_bytes!`
and no `.rs`, `.toml`, `.py`, `.sh` or Makefile path names `spec/decisions`.
O-23's kept row (one revision per lock) is the rule FR-061 checks
(FR-061 cites ADR-011 §7.1 one-revision-per-lock).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Every deletion removes a statement about `RevisionPin` and nothing
else; the kept O-23 table row and the release-pins paragraph are untouched.
The trimmed sentences read correctly: ADR-011 OBS-034 (secondary) is "Direction
per §7.1."; ADR-013 OBS-034 keeps the revision-authority and `pins.json`
deletion; the ADR-011 answered list ends "(T-6) and capability types crossing
as data (T-7). The executor key (T-8) is settled by owner ruling".

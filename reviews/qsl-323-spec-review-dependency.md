---
id: SR-812
title: "Dependency review of QSL-323: the removed IR root to QSL edge"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language@4c51991de72f82db507d584499f87a5fff7ae0ac; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-812: Dependency review of QSL-323: the removed IR root to QSL edge

## Summary

Ticket: QSL-323. PR agent-ix/quire-spec-language#534.

The PR states that the IR root → QSL edge is removed (ADR-011 FB-05, §7.1,
OBS-029, T-5). Measured on IR main `3e7935f`: neither `Cargo.toml` nor
`crates/quire-contract-model/Cargo.toml` declares a QSL dependency. On CG main
`37ff360`: CG depends on `quire-contract-ir` and `qsl-replay`, so CG can hold
the C-09 map and the packet with no new edge. QSL `qsl-package` →
`quire-contract-model` remains, so an IR → `qsl-replay` edge would close a
QSL ⇄ IR cycle; the stated reason is correct.

Examined and consistent: FB-05, the §7.1 graph and edge table, OBS-029, T-5
and the M-6b row. Not consistent: the M-6d lane row, and ADR-017's PF-8 row
that cites it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The M-6d lane row still lists "IR's predicate and temporal admission over QSL types" as work deleted with #218 and agent-ix/quire-contract-ir#141. OBS-029 in the same ADR now says IR declares no QSL dependency, so IR has no admission over QSL types to delete. #141 is closed. T-3 copies lane deletions into #218's exit criteria, so #218 would carry an exit criterion that names code which does not exist. Fix: drop that clause from M-6d, or state that IR's v2 admission reads the v2 wire and names no QSL type. | ADR-011:1073; ADR-011 OBS-029, T-3 |
| FND-002 | low | ADR-017 PF-8 still lists "M-6d IR predicate and temporal admission over QSL types, IR root → QSL edge" as IR-owned open work under agent-ix/quire-contract-ir#153 (open). ADR-011 OBS-029 now says the edge is removed. The two ADRs disagree on whether the edge still exists. Fix: update the PF-8 row to match OBS-029, or narrow it to the temporal intake that #153 still owns. | ADR-017:369; ADR-011 OBS-029 |

## Verdict

Changes requested for FND-001. The edge claims themselves are correct and
measured; the M-6d row and the ADR-017 cross-reference still describe the edge
and IR's QSL-typed admission as outstanding.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 17b7269c |
| FND-002 | fixed | 17b7269c |

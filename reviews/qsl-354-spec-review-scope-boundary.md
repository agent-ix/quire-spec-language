---
id: SR-934
title: "Scope-boundary review of PR #550 (driver owns Kani run, replay and terminal record)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@6d60826c4fb5e9e1f7acebcaf349965cd4cdf43d; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/decisions/ADR-016-state-model-finite-execution-mapping.md, spec/functional/FR-069-implement-typed-proof-result-envelope.md"
review_set: subset
---

## Summary

Ticket: QSL-354. Responsibility allocation between quire-driver, CG,
qsl-replay and quire-integration after the change. The driver, C-09 and
replay-type allocations match the settled decisions; three older sentences
now allocate the same responsibility to a different owner or to nobody.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-013 O-24 Owner still says CG "produces the FR-331 envelope as the backend provider", while T-13 now says the driver writes the item's FR-331 terminal record. Two owners for writing the FR-331 result. Fix: amend O-24 to say who writes the record and who produces the envelope. | ADR-013:821, ADR-011:1401, ADR-011:719 |
| FND-002 | medium | OBS-022 and OBS-034 now say a literal that restates a Cargo pin "is deleted" and CG's pin literals and `pins.json` "are deleted", in present tense with no owner. The package view's `ir_revision` literal is live (src/package/view.rs:46), and with #215's pin duty removed nothing owns the deletion. Fix: state it as a target and name the owning repository or change. | ADR-013:1265, ADR-013:1271, ADR-013:801-814 |
| FND-003 | low | ADR-011 S8 still names "CG replay adapter (reconstruction and comparator)" and O-27 says "CG's replay adapter compares parity", while C-09 now settles parity from qsl-replay's `WitnessArmResult` settlement and T-13 has the driver call `qsl_replay::replay`. The new E9 row ("The orchestrating driver (T-13) runs it, and CG's replay adapter reaches QSL through layer-6 `replay` only") does not say who calls `replay` or what the CG comparator still decides. | ADR-011:196, ADR-011:262, ADR-013:924, ADR-011:1401 |

## Verdict

Decisions 1, 2, 5 and 6 are allocated as settled: quire-driver runs S6b, E9
and writes the terminal record; CG keeps C-09 and O-25; qsl-replay keeps the
replay and terminal types; `spine` is public for `command` and the driver
only (quire-integration is not named); the crossing test lives in
quire-integration with no QSL → CG edge; FR-069 names CG for C-09.

## Dispositions

Round 1, reviewed at a00ace78 (fix commit a00ace78 on top of 6d60826c).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a00ace78 |
| FND-002 | fixed | a00ace78 |
| FND-003 | fixed | a00ace78 |

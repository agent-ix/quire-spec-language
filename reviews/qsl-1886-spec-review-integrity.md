---
id: SR-2026111305294500000004
title: "Spec integrity review of QSL-1886 capture semantics"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR 2695 head e7b13c3125a89d3045ce10dc5a0f0dc9a94ac080; FR-207-AC-7, FR-209-AC-9, ADR-027 PS-8 (and FO-3, ST-13, AE-1 as context), FR-381 stage table, ADR-036 PA-16"
review_set: subset
---

## Summary

Ticket: QSL-1886. Integrity check of the added spec text against its
neighbours. FR-207-AC-7 reads each name only where FR-381's stage table allows:
the registration capture reads `x` and `big`, the trigger guard reads `f` and
`big`, the activation capture reads `big`, and recovery reads `seen` and
`armed`. No stage reads the activation binder outside `registration-capture`.
FR-209-AC-9's captures read the activation binder, earlier captures and the
`over` input, which the `protocol-capture` row allows. They are consistent with
ST-13 and AE-1. The PS-8 phrase is consistent with ST-13's freezing. The
remaining gaps are in rules this PR now relies on but did not update.

## Verdict

**CONDITIONAL**: one medium gap (FO-3) and two low ones.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-027 FO-3 (initial states) says the initial state holds "no binders". It does not say that an `on origin` instance's protocol captures are evaluated there, nor what an undecided capture gives. The stepper now does both (it evaluates them in `start`, and gives `UndecidedInitialGuard` on undecided), and PS-8 now keys captures for every live instance. ST-13 covers `on each` only | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:150 |
| FND-002 | low | PS-8's new phrase "maps each live instance's protocol captures to their frozen values" gives no key shape, while every other map in PS-8 names its key tuple. The code keys it by (instance ordinal, capture name) | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:140 |
| FND-003 | low | ADR-036 PA-16 still lists "protocol captures, `activation on each` with compensation" among the constructs `qsl-eval` refuses. This PR removes both refusals, so that example list is now stale | spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md:144 |

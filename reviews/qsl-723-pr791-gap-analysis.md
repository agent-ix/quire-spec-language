---
id: SR-7864
title: "QSL-723 PR 791 gap analysis: TC-235 M19"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b467c6780481a06fd4be31e32a0ad4850d20ab24; TC-235 M19 runs 1-7; qsl-semantics/tests/it/tc235_state_machine.rs; qsl-semantics/src/model/normalize.rs; qsl-semantics/src/model/intake.rs"
review_set: subset
---

## Summary

Plan completion: not assessed

Ticket: QSL-723. This is a manual check of QSpec TC-235 row M19 against its test and source, over the PR diff. No computed matrix and no build were run. QSL-723 AC-1 requires each run to be "asserted with the TC's full stated outcome". The PR title says it makes M19 pass. The test covers runs 1-5 and 7, and run 5 uses a changed input (see SR-7863 FND-001).

## Verdict

PASS WITH FINDINGS on coverage. One run is missing, and two runs assert only admission.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | M19 run 6 has no test. Run 6 is `OrderLifecycle` invariant `Extra` used as the guard of `ExpressLifecycle` `[0]`, refused `malformed-declaration` at `TypeDefinition.transitions[0].guard` after 24 charges (FR-208-AC-12). Normalize has the rule, at the guard check over `machine_chain`, but nothing fails if it is removed. | qsl-semantics/tests/it/tc235_state_machine.rs:352-504; qsl-semantics/src/model/normalize.rs:1463-1486 |
| FND-002 | medium | Runs 1 and 7 assert only admission, the charge count and (run 7) the type key. The row also states: effective states `{Draft, Placed, Cancelled, Expedited}`; effective transitions `OrderLifecycle` `[0]`-`[2]` plus its own `[0]`; effective operations `place`, `cancel` and `expedite`; field `placed`; `NotYetPlaced` as the guard of inherited `[0]` (and, in run 7, of `ExpressLifecycle` `[0]`); and conformance to `OrderLifecycle`. None of these is asserted. | qsl-semantics/tests/it/tc235_state_machine.rs:353-395 |
| FND-003 | low | `tc235_m19_state_machine_specialization_vectors` and the other tests in this file have no `#[trace("QSpec-TC-235")]`, which the sibling TC-235 tests carry, so the matrix does not count them. | qsl-semantics/tests/it/tc235_state_machine.rs:351 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d92fa4df8 |
| FND-002 | fixed | d92fa4df8 |
| FND-003 | fixed | d92fa4df8 |

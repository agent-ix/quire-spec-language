---
id: SR-827
title: "QSL-336 failure-domain review of PR 539 (FR-122)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@15d200d648a0fc06ec3d5a6c4270debfb40ee913; spec/functional/FR-122-replay-a-state-clause-counterexample.md; spec/test-cases/TC-517-replay-a-state-clause-counterexample.md; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-517
    type: reviews
---
## Summary

Ticket: QSL-336. PR: quire-spec-language#539 at 15d200d6.

Checked identity confusion (envelope vs payload vs recompile), refusal
ordering, missing and edited documents, wrong observation form, zero budget,
determinism and internal faults. The identity design is sound: one carrier
per identity, checked before admission, with AC-3 proving no document is read
first. Admission failures, stale `package_id` and missing names each refuse
with no result.

## Verdict

No high or medium failure-mode gap. Three low gaps where an outcome or a
member's reading is unstated.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `obligation_identity` is defined for a state-clause packet but neither checked against the recompiled clause's `operation-contract` record nor declared uninterpreted, and the result does not retain it. A packet with the right `clause_node`/`occurrence_key` and another obligation's identity settles `reproduced` silently, the two-carrier disagreement G-2 names. Either check it or state it is not interpreted, as for `selected_function`. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:60-67 |
| FND-002 | low | FR-107's `evaluate_clause` can return `CallFailure::Input` (`UnknownClause`, `ObservationsMismatch`). The settlement bullets name every other outcome but not this one. It is unreachable after name lookup and admission for the same clause, but an implementer must still pick between `InternalFault`, `NoValue` and a refusal. State it is an `InternalFault`. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:110-128 |
| FND-003 | low | The Witness-arm result's FR-351 record is unspecified. FR-072 and FR-098-AC-2 put an FR-351 record on `reproduced-with-evaluated-witness`. FR-122's Outputs list none, AC-1 asserts none, and it does not say whether the envelope's transcript or `Input` assignments are read at all (a state-clause packet has no parameters for them to bind). State what the record holds (FR-116's implementation uses the whole Boolean as deciding element, index 0) and that the source arm only selects the result arm. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:77-83, 113-118 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The FND-003 fix justifies ignoring the transcript and `Input` assignments with "since a state clause has no parameters for them to bind". That is false for preconditions and postconditions: FR-104/FR-105 build the clause node over `self`, `result` and the operation's parameters, FR-107 evaluates each operation parameter to its admitted value, and AC-4's `ReachesTarget` reads `probe`'s parameter `target`. The behaviour (bind nothing from the source arm) is right; the reason contradicts FR-107 and invites reading parameterized operations as out of scope. Reword: every value the clause reads (`self`, the operation's parameters and result) comes from the admitted documents, so the source arm binds nothing. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:76-79 |
| FND-005 | low | The FR-351 record is stated only for a `Witness`-arm result "that reproduces" and for a result with no value. A `Witness`-arm `inconclusive`/`Verdicts` result has a value (`true`) and its record presence is unstated; FR-098/FR-116's code attaches the record whenever a value exists. State it (the record accompanies the value on the `Witness` arm). | spec/functional/FR-122-replay-a-state-clause-counterexample.md:100-104 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | cf00f646 |
| FND-002 | fixed | cf00f646 |
| FND-003 | fixed | cf00f646 |

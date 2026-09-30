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


## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | Check 10, extended to `PreCall`, does not say a reference parameter must name an object of the snapshot. The code refuses a dangling reference parameter for an invocation (`dangling_reference`, resolved against the pre snapshot, `qsl-semantics/src/model/observation.rs:981-994`), but the spec never states it, and the new `PreCall` admission is written from the spec. Failure: a `PreCall` with `target` naming a key absent from the complete snapshot admits; `reaches(self, target, parent)` never meets it and returns `false`; the replay settles `reproduced` on a counterexample no real call could produce. State in check 10 that a reference parameter resolves in the pre snapshot (the `PreCall` snapshot) and a key absent from its complete population refuses `dangling_reference`, and add a case to FR-106-AC-8 / TC-464 step 5. | spec/functional/FR-106-admit-snapshots-and-invocations.md:238-246 |
| FND-007 | low | A `PreCall`'s `self` and `parameters` sit in the payload, outside any digest, and FR-122's Outputs retain only the documents admission read. So a `PreCall` result cannot show which values it reproduced (`target` `a`). Retain the observation's `self` and parameters, or the payload's observation, in the result. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:100-108 |


## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | Check 10 now refuses a dangling reference parameter only when the key is absent from a *complete* population, and the required-population rule (population of `self`, then populations named by object fields) does not include a population a parameter names. So a parameter naming a key in an unmarked population that no field reaches is neither refused nor `Incomplete`. A clause that only compares it (`reaches(self, target, parent)` never dereferences `target`) evaluates `false`, and replay settles `reproduced`. Add every population a parameter reference names to the required populations, so check 7 returns `Incomplete` for it. | spec/functional/FR-106-admit-snapshots-and-invocations.md:157-159, 246-252 |


## New findings (disposition pass 5)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | FR-106-AC-8's new case adds "a second population `archive` of `ConfigVersion` objects" to the `probe` unit. Two unbounded populations whose member type is `ConfigVersion` make every clause on `ConfigVersion` refuse `ambiguous_declaration`/`ambiguous-name` at S3 (FR-104's context-population rule, FR-104-AC-5), so `ReachesTarget` never compiles and the case cannot reach admission. TC-465 hit the same trap and declares `archive`'s member type as `Sub` for that reason (TC-465 notes under row 43). Declare `archive` over `Sub` with `a1` a `Sub` object, in FR-106-AC-8 and TC-464 step 5. | spec/functional/FR-106-admit-snapshots-and-invocations.md:306; spec/test-cases/TC-464-snapshots-and-invocations-admit.md:37-39 |
| FND-010 | low | Check 7 now reads reference-valued parameters to find required populations, but check 10 validates parameters later. A parameter reference naming a population the package does not declare becomes a required population "absent from its snapshot", so admission returns `Incomplete` where check 10's value rule would refuse it. State that only a parameter reference to a declared population of the package makes it required (or that check 10's value check on reference parameters runs first). | spec/functional/FR-106-admit-snapshots-and-invocations.md:157-162, 229 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | cf00f646 |
| FND-002 | fixed | cf00f646 |
| FND-003 | fixed | cf00f646 |
| FND-004 | fixed | 0a72b19a |
| FND-005 | fixed | 0a72b19a |
| FND-006 | fixed | 82e69a12 |
| FND-007 | fixed | 82e69a12 |
| FND-008 | fixed | 0bedd077 |

---
id: SR-792
title: "Scope-boundary review of ADR-016 state, model and finite execution mapping"
type: SpecReview
analysis: scope-boundary
scope: "spec/decisions/ADR-016-state-model-finite-execution-mapping.md; amendments to ADR-012 Status, ADR-013 §6 Outcomes row, FR-089 Status and spec/spec.md; context ADR-011 to ADR-015, FR-089, FR-101, FR-120"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: reviews
---
# SR-792: Scope-boundary review of ADR-016

## Summary

Round 1. Reviewed commit `47b1b806` on `spec/19-arch40-mapping`: the new
ADR-016 and its amendments to ADR-012 Status, the ADR-013 §6 Outcomes row,
FR-089 Status and `spec/spec.md`. Linear QSL-19 (#220) was read as data. Code
claims were checked in the worktree.

The question: does ADR-016 stay in its lane? QSL-19 asks for a mapping that
leaves "no identity/typestate/conversion/outcome/capability/bound/replay
decision" to feature work, with no A04/A05 implementation, no new model
behavior and no finite simulator feature. ADR-016 Status also says it
"reopens none of [ADR-011 to ADR-015's] cells". So each decision it makes
must either match the text of the record that owns it, or amend that record
in the same change.

Allocation of the decisions ADR-016 touches:

| Decision in ADR-016 | Owning record | In QSL-19's lane? | Consistent with owner text? |
| --- | --- | --- | --- |
| §1 SC-1 to SC-8, agreement rule | ADR-011 E6, ADR-012 §15.4, ADR-014 §1 (B-2/B-3) | yes | yes |
| §2 ID-1 to ID-4, ID-6 to ID-11 | ADR-013 O-03 to O-09, O-25, ADR-011 §2.2 | yes | yes |
| §2 ID-5 `PopulationId` preimage gains `members` | ADR-013 O-13 Population row, T-6, QC-21; FR-089 Behavior | yes (identity) | no: owner text not amended (FND-001); premise false (FND-002) |
| §2 two keyings of a population domain | ADR-012 §15.7, ADR-014 §4 | yes | yes |
| §3 EX-1 universe "in kind" B-4 | ADR-014 §1 bound taxonomy (#222) | no | no (FND-004) |
| §3 EX-2 to EX-8 | FR-101, FR-120, ADR-014 TR-2, TR-6, TR-7, §7 | yes | yes |
| §3 EX-9 evidence scope of a result | ADR-014 §8 | partly | adds a comparison rule no FR carries (FND-009) |
| §4 FE-1 to FE-5 | FR-103, FR-106, FR-107, FR-114 to FR-116, FR-120 | yes | yes; FE-4 declines new model behavior |
| §5 ND-1 to ND-4 | FR-101, ADR-013 R-05 | yes | yes |
| §6 finite exhaustion is not proof | ADR-014 §8, ADR-013 O-16, O-25 | yes | yes |
| §7 FP-3 no `StateModel` S6a variant | ADR-012 §2, §3, §5.1 S1, Q210-3 row | yes (typestate/outcome) | no: second exception to ADR-012's "every family except `Relation`" (FND-003) |
| §7 FP-4 simulation `Outcome` canonical | ADR-014 §7; ADR-013 §6, O-16; ADR-011 §8 | yes | partly: ADR-013 amended in one of three places (FND-005); ADR-011 §8 wording (FND-008) |
| §9 G-6 state lane deletion | ADR-012 §15.8 step 2, ADR-011 M-6c | restates | list differs (FND-007) |
| §9 G-7 `native_model` deletion | ADR-012 §15.8 step 3, ADR-011 M-6d, M-6e | restates | owner differs (FND-006) |

What holds:

- §1 matches ADR-014 §1. SC-2's `stage_limit_exceeded` is the B-3
  `TypeEnvironmentLimits` case, and SC-6's `resource_exhausted` is B-2.
- The "two keyings" paragraph matches ADR-012 §15.7 `DomainKey` and ADR-014
  §1 ("No value of one kind converts into another kind").
- EX-5 keeps ADR-014 TR-2's `TemporalPosition` apart from FR-101's step
  index. EX-6 matches ADR-014 §7: "S6a is not cancellable". §6 matches
  ADR-014 §8: "a result is evidence only for what it evaluated".
- G-1 and G-4 cite FR-101-AC-12 to AC-14 and FR-120-AC-1 to AC-12 by
  reference. They restate neither requirement.
- FE-4 adds no model behavior. It records the S6a refusal as a guard and
  names QSpec admitting a population-valued clause expression as the reopen
  condition. That respects the non-goal.
- The ADR-012 Status amendment and the `spec.md` row only point at ADR-016.
- ID-5's claim that no production path admits two bindings into one
  evaluation holds. The `admit_binding` call in `qsl-eval/src/value/
  expression/evaluate.rs:2015` is inside `#[cfg(test)]` (`:1946`). The other
  non-test callers are `qsl-bench/src/model.rs:309` and `:332`.

What does not hold: three decisions change text owned by ADR-012 and
ADR-013 without amending it, although Status says no cell is reopened
(FND-001, FND-003, FND-005). ID-5 rests on a false premise about the
declared maximum (FND-002). EX-1 puts a second type into ADR-014's B-4 kind
(FND-004). G-6 and G-7 restate ADR-012 §15.8 with a different list and a
different owner (FND-006, FND-007).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | ID-5 decides the `PopulationId` preimage. QSL-19 requires identity decisions, so the decision is in lane. But the preimage is stated by ADR-013 O-13 Population row, T-6 and QC-21, and by FR-089 Behavior, and ADR-016 amends none of them. O-13 says the preimage is "the domain package it was admitted against, the `population_key` declaring it, and a closed three-state admission-role discriminator ... so two admissions that share every one of these facts share a `PopulationId`". FR-089 Behavior says the same as a SHALL, and FR-089-AC-1 tests only those three facts. With `members` added, that sentence is false. G-3 then defers "FR-089 amended to match" to QSL-68, which leaves the spec half of an identity decision to feature work, against QSL-19's acceptance. ADR-016 Status still says it "reopens none of their cells". Fix: in this commit, amend the O-13 Population row, the T-6 `PopulationId` sentence and the QC-21 row with the new preimage. Amend FR-089 "PopulationId is opaque and content-addressed over the admission" and FR-089-AC-1. Drop "FR-089 amended to match" from G-3. Change Status to name the cells ADR-016 amends. If QC-21 has been filed against quire-specification, correct the filed text too. | ADR-016 Status, §2 ID-5, §9 G-3; ADR-013 O-13 Population row (line 345), T-6 (line 951), QC-21 (line 1137); FR-089 Behavior, FR-089-AC-1 |
| FND-002 | high | ID-5 says "The maximum is read from the declaration that `(domain_package_selection, population_key)` already names, so it cannot differ between two admissions that agree on those." The code and the owning FRs say otherwise. `admit_binding` takes `declared_maximum: Option<u64>` from its caller (`qsl-semantics/src/model/population.rs:811`) and stores it in `PopulationBinding.declared_maximum` (`:559`). FR-089 Inputs lists the "declared maximum that FR-084's `admit_binding`/`admit_invocation` already take", and FR-089-AC-5 compares the binding's own maximum. So two admissions that agree on package, key, role and members, but differ in maximum, still mint one id. `with_population`'s refusal stays reachable through the public API, and making it an `InternalFault` would report a caller input as an internal failure. Fix: choose one. (a) Add the declared maximum to the preimage beside `members`. (b) Make admission read the maximum from the population record and remove the parameter; that changes FR-084's inputs, so amend FR-084 and assign it in G-3. Keep the guard a refusal until one of them lands. | ADR-016 §2 ID-5, §9 G-3, Consequences; FR-089 Inputs, FR-089-AC-5; FR-084; `population.rs:559`, `:811` |
| FND-003 | high | FP-3 removes `StateModel` from S6a evaluation. ADR-012 names `Relation` as the only family without an S6a hook, in five places. §2 Stage hooks row: "`evaluate` at S6a for every family except `Relation`". The paragraph after the traits: "`ReferenceEvaluation` is implemented by every family except `Relation`". §3 `StateModel` row, Evaluation and lowering: "model normalize and population evaluation". §5.1 S1 row: "one variant per family that implements `ReferenceEvaluation`". Q210-3 row (line 1079): "`evaluate` (every family except `Relation`, including the simulation lane)". FP-3's reason follows ADR-012's own #214 rule ("a contract part that nothing can legitimately construct is left out"), so the direction fits the rule. But it adds a second exception that only an ADR-012 amendment can record, and ADR-016 amends only ADR-012's Status. Fix: amend these five places in this commit to say that `StateModel` implements `FamilyContract` but not `ReferenceEvaluation`, and that its model query functions run inside the `Value` and `ProtocolClause` `evaluate` hooks and raise `StateModel` causes. Add the ADR-012 amendment to "Amendments made with this record". | ADR-016 §7 FP-3, Alternatives; ADR-012 §2 (lines 228, 268), §3 (line 452), §5.1 S1 (line 560), Q210-3 row (line 1079) |
| FND-004 | medium | EX-1 says a universe "in kind ... is an ADR-014 B-4 proof bound ... carried as FR-120's `PopulationUniverse`, not converted into `ProofBound`". ADR-014 §1 owns the taxonomy and says: "Six kinds exist. Each is its own type with one owner." B-4's one type is `ProofBound`. It is supplied "for one unbounded domain of an item, to ask a bounded-mode backend", and it is part of the obligation identity. A universe fits none of that. FR-120 "Bounds" gives a universe for every simulated population, including one whose declaration has a maximum: with a universe the root is `Population(Some(n))`, and without one it is `Population(None)`. It is keyed by a simulation root, not a `DomainKey`, and no backend reads it. Calling it B-4 puts a second type into one kind, which ADR-014 forbids and QSL-19 does not own. Fix: drop the B-4 classification from EX-1 and from the Alternatives entry ("Their kind is recorded (EX-1)"). State that a universe is a simulation request input, not one of ADR-014's bound kinds, and that EX-9 scopes it. If a bound kind is wanted, file an ADR-014 amendment instead. | ADR-016 §3 EX-1, Alternatives; ADR-014 §1 B-4 row and lead; FR-120 Inputs, "Bounds" |
| FND-005 | medium | The ADR-013 amendment is partial. The §6 Outcomes row now says `qsl_eval::simulation::Outcome` is "the converged S6a-layer outcome, mapped by `Outcome::category()`". Two unamended ADR-013 sentences say the opposite. O-16 (line 533): "Simulation (lane D) converges into S6a (ADR-011 §8), so its outcomes are the kernel `Outcome` through S6a." §6 lead (line 1039): "lane D converges into S6a, whose outcomes are the kernel `Outcome`." FP-4 itself is consistent with ADR-014 §7, which created `Outcome::category()` in layer 5. The §6 table also lists lane-private types that "have no conversion to a canonical type", so a canonical type does not belong in its cells. Fix: amend O-16 line 533 and the §6 lead line 1039 to say that exploration outcomes are `qsl_eval::simulation::Outcome`, mapped to O-16 by `Outcome::category()` (ADR-014 §7). Move the new sentence out of the §6 table cell into the lead, and keep only "the lane-D simulation `Outcome` before convergence" in the row. | ADR-016 §7 FP-4, Amendments; ADR-013 O-16 (line 533), §6 lead (line 1039), §6 Outcomes row; ADR-014 §7 |
| FND-006 | medium | G-7 gives `native_model`'s deletion to "QSL-68 (#120) and QSL-67 (#121), each in the PR that lands its S3 checker". The owning records say otherwise. ADR-012 §15.8 step 3 deletes it "in the PR that removes the last of those importers (ADR-011 §7.3 M-6d and M-6e)". ADR-011 gives M-6d, which removes the SEAM-3 `protocol_artifact` handoffs, to #218. It gives M-6e to every family ticket, and "the last one deletes the remainder". The last importer may go with #218 or another family ticket, so neither QSL-67 nor QSL-68 may land that PR. Fix: set G-7's owner to "the ticket whose PR removes the last importer (ADR-011 M-6d, M-6e)". Limit QSL-67 and QSL-68's share to deleting their own `StateModel` composed families under M-6e. | ADR-016 §9 G-7; ADR-012 §15.8 step 3; ADR-011 §7.3 M-6d, M-6e rows |
| FND-007 | low | Status says ADR-016 "does not restate" ADR-012 §15. G-6 restates §15.8 step 2 with a shorter list. It omits FR-100's `0-draft` route of CLI `run` and native `compile` and `lower`, which §15.8 step 2 retires with QSL-5. It also omits `lowering`, IT-010 (SEAM-4) and the dev dependencies on CG, IR and RT, which ADR-011 M-6c deletes in the same PR. ADR-011 M-6c names #120, #121 and #164 as owners and says "The PR that lands spine clause execution deletes native `run`"; G-6 names QSL-67 with QSL-5. Fix: make G-6's criterion "ADR-012 §15.8 step 2 and ADR-011 M-6c, as listed there". Name the owner as the ticket whose PR lands spine clause execution, per ADR-011 M-6c. | ADR-016 Status, §9 G-6; ADR-012 §15.8 step 2; ADR-011 M-6c row |
| FND-008 | low | FP-4 says `ModelSystem` is the only production `TransitionSystem`, and that simulation "is ... not a family". ADR-011 §8 D row says lane D "gains an implementer only through a family evaluator (#220)". With FP-3 also removing any `StateModel` evaluator, a reader cannot tell whether `ModelSystem` meets ADR-011 §8. Fix: add one sentence to FP-4 saying that `ModelSystem` reaches the engine through the `ProtocolClause` evaluator (FE-3), which is the family evaluator ADR-011 §8 means. If that is not the intent, amend ADR-011 §8. | ADR-016 §7 FP-3, FP-4, §4 FE-3; ADR-011 §8 D row |
| FND-009 | low | EX-9 defines "the identity of an exploration result" as its whole request and says "Two results compare only when these are equal". No type in FR-101 or FR-120 carries such an identity, and no function compares two results. No G-row builds or tests one. Read as a rule, it is a new simulator feature, which QSL-19 excludes. Read as a statement of scope, it repeats ADR-014 §8. Fix: reword EX-9 as a scope statement that cites ADR-014 §8 ("a result is evidence only for what it evaluated"), and drop "compare". If a comparable result identity is wanted, add it as a gap under G-4 with an FR-120 criterion. | ADR-016 §3 EX-9, §6; ADR-014 §8; FR-101; FR-120 |

## Disposition

Every finding above is fixed on `spec/19-arch40-mapping` in the ADR-016 rewrite and its listed amendments (ADR-011 M-6c and §8; ADR-012 §2, §3, §5.1, §13.5; ADR-013 O-13, T-6, QC-21, O-16, §6; FR-089; FR-120; `spec/tests.md`), except as noted below.

No finding is declined.

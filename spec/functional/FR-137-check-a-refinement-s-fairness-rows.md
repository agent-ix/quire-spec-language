---
id: FR-137
title: "Check a refinement's fairness rows at S3"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: depends_on
---
# FR-137: Check a refinement's fairness rows at S3

## Description

The refinement checker (FR-135) SHALL check the declaration's `assume` rows
into the concrete fairness set `F_C` and its `ensure` rows into the abstract
fairness set `F_A` (ADR-020 RM-6, as amended by ADR-027 PA-3). Each row is
an ADR-018 FA-1 constraint over its own model's operations, checked as
FR-123 checks a clause's fairness constraint.

## Use case

A specification author states the fairness the concrete model may assume
and the abstract fairness the refinement must deliver. With no `ensure`
row the refinement is a safety refinement; with one, a concrete model that
spins where the abstract model must move is refuted. Over a protocol, the
scheduler fairness the protocol already carries is assumed without being
written again.

## Inputs

- The parsed `assume` and `ensure` rows, each `fair <kind> [<granularity>]
  <operation>`, with spans.
- The concrete and abstract models' admitted operations; over a concrete
  protocol subject, its checked protocol clause with its `scheduling`
  member (ADR-027 PA-3).

## Outputs

- `CheckedRefinement.concrete_fairness` and `.abstract_fairness`, each a
  `Vec<FairnessConstraint>` as FR-123 defines it, extended over a protocol
  subject with `origin: FairnessOrigin::{Authored, Scheduler}`.

## Behavior

- The checker SHALL check each row's kind and granularity by FR-123's
  rules: the one kind is `weak`, a row with no granularity SHALL check as
  `Whole`, and a row written `whole` or `each` SHALL check as written.
- An `assume` row's operation SHALL resolve to an operation of the concrete
  model, and an `ensure` row's to an operation of the abstract model. A row
  whose operation resolves in neither model SHALL refuse
  `missing_declaration`/`missing-name`; a row whose operation resolves only
  in the other model SHALL refuse
  `invalid_model_binding`/`malformed-declaration`, naming the model it
  belongs to.
- The checker SHALL record each constraint of each set once, in source
  order, so two equal rows check as one constraint.
- The checker SHALL accept a declaration whose `F_C` or `F_A` is empty.
- While the concrete side is a protocol subject that does not declare
  `scheduling adversarial`, the checker SHALL add the protocol's derived
  scheduler constraints to `F_C`: one `fair weak whole` per thread target
  (each `parallel` branch, the root and each compensation template), each
  with origin `Scheduler`.
- While the concrete side is a protocol subject that declares `scheduling
  adversarial`, the checker SHALL add no scheduler constraint.
- Over a concrete protocol subject, the checker SHALL add the authored
  `assume` rows to `F_C` beside the scheduler constraints.
- Both sets are part of the refinement's node identity (FR-135).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-137-AC-1 | ADR-020 §8's `CasRefinesCounter` checks `F_C` as six `Weak` `Each` constraints in source order (`beginA`, `beginB`, `commitA`, `commitB`, `retryA`, `retryB`) and `F_A` as `[Weak Each Spec::Counter::inc]`. With the `ensure` row written `ensure fair weak Spec::Counter::inc`, `F_A` is `[Weak Whole inc]` and the node identity differs. With the `ensure` row removed, `F_A` is empty. | Test (TC-542) |
| FR-137-AC-2 | `assume fair weak Spec::Counter::inc` refuses `invalid_model_binding`/`malformed-declaration` naming `Spec`; `ensure fair weak Impl::Counter::peek` refuses the same naming `Impl`; `assume fair weak Impl::Counter::nope` refuses `missing_declaration`/`missing-name`. `assume fair weak each Impl::Counter::beginA` written twice checks as one constraint. | Test (TC-542) |
| FR-137-AC-3 | Over FR-136-AC-4's concrete protocol subject with no `assume` row, `F_C` holds three `Weak Whole` constraints with origin `Scheduler`, for branch `A`, branch `B` and the root. With `assume fair weak each incA` added, `F_C` holds those three and the authored constraint. With `scheduling adversarial` on the protocol, `F_C` holds only the authored constraint. | Test (TC-542) |

## Dependencies

- ADR-020 §1 RM-6 and §11 RU-2; ADR-018 §4 FA-1 and FA-6; ADR-027 §4 PA-1
  and PA-3.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (the constraint form and its checks),
  [FR-135](FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md).
- FR-143 reads both sets.

## References

- The QSpec half (fairness rows in the grammar): Linear STD-133.

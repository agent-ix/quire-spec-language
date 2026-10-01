---
id: FR-172
title: "Check hyper clauses over behaviours and relations over model executions at S3"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-171
    type: depends_on
---
# FR-172: Check hyper clauses over behaviours and relations over model executions at S3

## Description

The S3 TemporalTrace `check` SHALL check a `hyper` clause over `behaviours`
and a `relation` over model executions (ADR-023 §1): bind each trace
variable to a model alias, bind object parameters to population universes,
type indexed atoms and step labels, split the `match` block into its
universal and existential parts, check each variable's fairness set, check
the `align skip` list, and check each execution binding. The checked
clause's identity binds everything that changes its meaning (ADR-023 HV-7).

## Use case

A verification operator writes noninterference over one model, the same
alias twice, and a power comparison over two models. The checker binds each
trace to its model, refuses an atom that reads one model's field through
the other model's trace, and refuses a quantifier with no model, each at its
span.

## Inputs

- `HyperBehavioursForm` and `ModelRelationForm` (FR-171).
- The unit's model aliases and the admitted domain packages, with each
  model's operations, populations and fields (FR-103).
- The unit's resolved temporal profile selection (FR-110).

## Outputs

```rust
pub struct CheckedHyperClause {
    pub aliases: Vec<ModelAlias>,                  // distinct aliases, in first-use order
    pub parameters: Vec<ObjectParameter>,          // (name, alias, population)
    pub quantifiers: Vec<CheckedQuantifier>,       // (kind, variable, alias, fairness set)
    pub mu_universal: Vec<CheckedStepPredicate>,   // μ_U conjuncts
    pub mu_existential: Vec<CheckedStepPredicate>, // μ_E conjuncts
    pub align_skip: Vec<DeclarationKey>,           // empty for lockstep
    pub body: CheckedTemporalFormula,              // over indexed atoms
}

pub struct CheckedModelRelation {
    pub executions: Vec<(Name, DeclarationKey)>,   // each a model operation, in order
    pub body: CheckedExpression,
}
```

- A typed `CheckRefusal` with a span, and no checked clause, on refusal.

## Behavior

### Trace variables and aliases

- S3 SHALL resolve each quantifier's `of` alias to a model alias of the
  unit. An alias that names no model SHALL refuse
  `missing_declaration`/`missing-name` at the alias.
- If a quantifier of a `behaviours` clause has no `of`, then S3 SHALL
  refuse `unsupported_construct`/`expression-form` at the quantifier. If a
  quantifier of a finite trace domain clause has `of`, then S3 SHALL refuse
  the same code at the quantifier (ADR-023 HS-2).
- Two quantifiers that name the same alias SHALL range over the same
  subject; quantifiers that name different aliases SHALL range over
  different subjects.

### Object parameters

- S3 SHALL resolve each object parameter `v: V::T` to a population of alias
  `V`'s model whose element type is `T`. The clause SHALL check to one
  instance per key of each parameter's universe, and to the product of the
  universes for parameters over several aliases (ADR-023 HS-3).

### Indexed atoms and step labels

- Every model member read in a body atom SHALL carry exactly one trace
  variable, `e @ t`. A member read with no trace variable SHALL refuse
  `ill_typed` at the read. One atom MAY read several trace variables.
- If a member read carries a trace variable bound to a model other than the
  member's, then S3 SHALL refuse `ill_typed` at the read (ADR-023 HS-4).
- S3 SHALL type `t.step.op` as the operation-or-`stutter` label of `t`'s
  model, `t.step.receiver` as its receiver key, and `t.step.O.p` as
  `Option<T>` for parameter `p: T` of operation `O` of `t`'s model.

### Match

- S3 SHALL refuse a `match` conjunct that reads state, `ill_typed` at the
  read: `μ` reads step labels only (ADR-023 HS-5).
- S3 SHALL flatten `μ` into its top-level conjuncts and place each conjunct
  that reads only universal variables in `mu_universal` and every other
  conjunct in `mu_existential` (ADR-023 HM-3).

### Fairness and profile

- S3 SHALL check each quantifier's fairness set as FR-123 checks a clause's
  fairness set, over the operations of that quantifier's model, with
  `whole` unmarked. A constraint naming an operation of another model SHALL
  refuse `missing_declaration`/`missing-name` at the operation.
- A `behaviours` clause under the infinite-trace profile SHALL check its
  body as an infinite-trace formula over indexed atoms, interval operators
  admitted as FR-123 admits them. A `behaviours` clause under any other
  profile SHALL check and record its profile, so negotiation settles it V-8
  (FR-173).

### Align skip

- S3 SHALL resolve each `align skip` entry to an operation of a bound
  alias's model. An operation of no bound alias, an operation named twice
  and an empty list SHALL refuse `unsupported_construct`/`expression-form`
  at the entry or the list (ADR-023 PA-1).
- If a clause with `align skip` carries a non-empty fairness set on any
  variable, then S3 SHALL refuse `unsupported_construct`/`expression-form`
  at the constraint (ADR-023 PA-3).

### Relations over model executions

- S3 SHALL check an execution binding whose qualified name resolves to an
  operation of a model alias as a model execution binding. A binding to any
  other qualified name SHALL keep QSpec FR-191's finite reading.
- The body SHALL check under the value profile and MAY read, per execution
  `x`, its arguments `x.p`, its receiver's pre-state `pre(x.self.f)`,
  post-state `x.self.f` and result `x.result` (ADR-023 HS-8).

### Identity

- The checked clause's identity SHALL bind every subject alias in order,
  the quantifier prefix with each variable's kind and alias in order, each
  variable's fairness set, `μ`, the `align skip` list and the instance's
  parameter keys (ADR-023 HV-7, ADR-013 O-09).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-172-AC-1 | ADR-023 §1's `NonInterference` checks with one alias `V`, two universal quantifiers over it, one instance for universe `{v}`, both `μ` conjuncts in `mu_universal` and none in `mu_existential`. `SavesPower` checks with aliases `S` and `N`. §8.2's `Opaque` checks with its one conjunct in `mu_existential`. | Test (TC-597) |
| FR-172-AC-2 | Refusals at the named span: `forall trace a` with no `of` in a `behaviours` clause (`unsupported_construct`, the quantifier); `of V` in a finite trace domain clause (the quantifier); `of W` naming no model (`missing_declaration`, the alias); `v.l` with no trace variable and `d.used @ n` with `d: S::Device` (`ill_typed`, the read); a `match` conjunct reading `v.l @ a` (`ill_typed`); `fair weak N::Device::tick` on a variable of `S` (`missing_declaration`). | Test (TC-597) |
| FR-172-AC-3 | §13's clause with `align skip { V::Vault::mix }` checks with `align_skip` holding `mix`; `align skip { }`, `align skip { V::Vault::mix, V::Vault::mix }`, `align skip { N::Device::tick }` in a clause over `V` alone, and `align skip` with `fair weak V::Vault::step` each refuse `unsupported_construct`/`expression-form`. | Test (TC-597) |
| FR-172-AC-4 | FR-171-AC-3's `Det` checks with two model execution bindings to `step`. `NonInterference` and the same clause with its quantifiers' fairness sets swapped between `a` and `b` (one empty, one `weak step`) have different node identities, as do the clause and the same clause with an extra `match` conjunct. | Test (TC-597) |

## Dependencies

- ADR-023 §1 HS-1 to HS-9, §2 HM-3, §6 HV-7, §13 PA-1 and PA-3; ADR-018
  FA-1, FA-6, QS-5; ADR-013 O-09.
- [FR-171](FR-171-build-forms-for-hyper-clauses-over-behaviours.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (fairness and interval checking),
  [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md),
  [FR-110](FR-110-resolve-header-profile-selections-at-e3.md).
- QSpec owns the grammar, FR-191's extension to model subjects and the
  identity members of the checked-package v2 `hyperproperty` node (ADR-023
  QS-1, QS-2, QS-7).

## References

- ADR-023. QSpec half: Linear STD-136 (ADR-023 QS-1, QS-2, QS-7).

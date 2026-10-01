---
id: FR-171
title: "Build S2 forms for hyper clauses over behaviours and relations over model executions"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
---
# FR-171: Build S2 forms for hyper clauses over behaviours and relations over model executions

## Description

The S2 value form builder SHALL build a form for a `hyper` clause whose
domain is `behaviours` and for a `relation` clause whose execution bindings
name model operations (ADR-023 HS-1 to HS-9), in place of the
`UnrepresentedConstruct` refusal it gives them today. The forms carry the
syntax only; S3 (FR-172) resolves names and checks them. A `hyper` clause
over a declared finite trace domain and a `relation` over other qualified
names keep the forms QSpec FR-191's finite reading gives them.

## Use case

A verification operator writes §1's `NonInterference` clause and a
`relation Det` over two executions of `V::Vault::step`. The compiler builds
a form for each instead of refusing them, so they reach the checker.

## Inputs

- The S1 CST: `HyperClause` with its `TraceDomain`, `Quantifier`s, optional
  `match` block, optional `align skip` list and body; `RelationClause` with
  its execution bindings and body.

## Outputs

```rust
pub struct HyperBehavioursForm {
    pub name: Name,
    pub profile: ProfileRef,
    pub parameters: Vec<ObjectParameterForm>,      // `behaviours (v: V::Vault, …)`
    pub quantifiers: Vec<QuantifierForm>,          // in source order
    pub match_block: Option<ExpressionForm>,       // μ
    pub align_skip: Option<Vec<QualifiedName>>,    // `align skip { O, … }`
    pub body: TemporalExpressionForm,              // indexed atoms as written
    pub span: Span,
}

pub struct QuantifierForm {
    pub kind: QuantifierKind,                      // Forall, Exists
    pub variable: Name,
    pub alias: Option<Name>,                       // `of <alias>`
    pub fairness: Vec<FairnessConstraintForm>,     // ADR-018 FA-1 spelling
    pub span: Span,
}

pub struct ModelRelationForm {
    pub name: Name,
    pub profile: ProfileRef,
    pub executions: Vec<ExecutionBindingForm>,     // (variable, qualified name), in order
    pub body: ExpressionForm,
    pub span: Span,
}
```

## Behavior

- S2 SHALL build a `HyperBehavioursForm` for every `hyper` clause whose
  domain is `behaviours`, with or without object parameters, holding each
  quantifier in source order with its `of` alias and fairness set as
  written.
- S2 SHALL build each indexed atom `e @ t` as an expression form holding the
  member read `e` and the trace variable `t`, and each step label
  (`t.step.op`, `t.step.receiver`, `t.step.O.p`) as a step-label form.
- S2 SHALL build the `match` block as one expression form and the `align
  skip` list as the operation names in source order.
- S2 SHALL build a `ModelRelationForm` for every `relation` clause, holding
  each execution binding's variable and qualified name in order, and its
  body with `pre(…)`, argument, `self` and `result` reads as written.
- S2 SHALL build no form that resolves a name; whether a binding names a
  model operation, and whether an alias names a model, are S3's (FR-172).
- The S2 depth and node bounds of FR-091 SHALL apply to these forms as to
  every other form.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-171-AC-1 | ADR-023 §1's `NonInterference` builds a `HyperBehavioursForm` with one object parameter `v: V::Vault`, quantifiers `forall a of V` and `forall b of V` in order with empty fairness sets, a `match` form holding both step-label equalities, no `align skip`, and a body holding the indexed atoms `v.l @ a` and `v.l @ b`. §1's `SavesPower` builds one with two object parameters and aliases `S` and `N`. | Test (TC-596) |
| FR-171-AC-2 | `forall trace a of V fair { weak V::Vault::step }` builds a quantifier holding one fairness constraint form; §13's clause with `align skip { V::Vault::mix }` builds `align_skip` holding `V::Vault::mix`. | Test (TC-596) |
| FR-171-AC-3 | `relation Det using v over (x: V::Vault::step, y: V::Vault::step) { x.i = y.i and pre(x.self.l) = pre(y.self.l) implies x.self.l = y.self.l }` builds a `ModelRelationForm` with executions `x` and `y` in order and the body's `pre` reads as written. None of AC-1 to AC-3's clauses refuses `UnrepresentedConstruct`. | Test (TC-596) |

## Dependencies

- ADR-023 §1 HS-1 to HS-9, §13 PA-1; ADR-012 §3 (`TemporalTrace` row, as
  amended by ADR-023).
- [FR-091](FR-091-produce-value-forms-and-assemble-package-declarations.md)
  (the S2 builder, its bounds and its refusal causes).
- QSpec owns the surface syntax (ADR-023 QS-1); the spellings here follow
  ADR-023 §1 and §13.

## References

- ADR-023. QSpec half: Linear STD-136 (ADR-023 QS-1).

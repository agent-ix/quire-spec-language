---
id: FR-274
title: "Build random-parameter, workload and reward forms at S2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-022
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-405
    type: depends_on
---
# FR-274: Build random-parameter, workload and reward forms at S2

## Description

When a `1-draft` unit declares a random parameter, a workload or a reward
(ADR-024 PM-1, PM-2, PM-5), S2 (`qsl-forms`) SHALL build one form for it
from S1's parse, so S3 can check it (FR-185). QSpec FR-405 and its
shared grammar own the spelling; this requirement specifies the CST
productions and the forms QSL builds from them.

## Use case

A modeller writes `operation Server::attempt(random d: Quantity<Time> ~ {
1 ms: 90, 3 ms: 7, 12 ms: 3 })`, `reward duration = d;` in its body and
`workload Steady on Service { weight Server::request = 1; … }`. Parsing and
form building succeed, and S3 receives one form per declaration with every
span it needs for a diagnostic.

## Inputs

- A recovery-free `LosslessCst` of a `1-draft` unit (ADR-011 E2).
- `FormsLimits`.

## Outputs

```rust
pub struct RandomParameterForm {        // on its operation's parameter
    pub parameter: Spanned<Identifier>,
    pub value_type: TypeForm,
    pub support: Vec<(Expression, Expression)>, // (value, weight), declared order
}
pub struct RewardForm {                 // in its operation's body
    pub name: Spanned<Identifier>,
    pub expression: Expression,
}
pub struct WorkloadForm {               // DeclarationForm::Workload
    pub name: Spanned<Identifier>,
    pub model: Spanned<QualifiedName>,
    pub weights: Vec<(Spanned<QualifiedName>, Expression)>, // declared order
}
```

## Behavior

- S1 SHALL parse a `random` modifier on an operation parameter followed by
  `~` and a braced, comma-separated list of `value: weight` pairs, a
  `reward name = expression;` member in an operation body, and a top-level
  `workload name on model { weight operation = expression; … }`
  declaration, as QSpec's shared grammar spells them.
- `LeadingTokenKind` SHALL gain `Workload`, selecting the one production
  `probabilistic::workload`.
- S2 SHALL build a `RandomParameterForm` on each `random` parameter, a
  `RewardForm` for each `reward` member and a `WorkloadForm` for each
  `workload` declaration, keeping every pair, weight and reward in declared
  order with its span.
- S2 SHALL leave the values, weights, completeness and reads unchecked for
  S3 (FR-185).
- When a `random` modifier has an empty support list, S2 SHALL refuse it
  with `FormsCause::EmptySupport` at the braces.
- S2 SHALL charge each pair, weight and reward against `FormsLimits` as it
  charges any other form node.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-274-AC-1 | ADR-024 §7.2's `Service` builds: `attempt` has two `RandomParameterForm`s, `d` with the pairs `(1 ms, 90)`, `(3 ms, 7)`, `(12 ms, 3)` and `outcome` with `(Ok, 98)`, `(Fail, 2)` in that order; `attempt` has one `RewardForm` `duration` whose expression is `d`; `Steady` is a `WorkloadForm` on `Service` with three weights in declared order. Every form carries the span of its source text. | Test (TC-643) |
| FR-274-AC-2 | A unit beginning `workload` dispatches to `probabilistic::workload` and to no other production. | Test (TC-643) |
| FR-274-AC-3 | `random d: Quantity<Time> ~ { }` refuses `EmptySupport` at the braces; `random d: Quantity<Time> ~ { 1 ms: 0 }` and a workload missing an operation both build, and are refused only at S3 (FR-185-AC-2, FR-185-AC-3). | Test (TC-643) |

## Dependencies

- ADR-024 PM-1, PM-2, PM-5, QS-1 to QS-3; ADR-012 §3, §4.3.
- [FR-067](FR-067-add-s2-forms-and-retire-seam-5.md) (the S2 forms core),
  [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md) (the
  S3 checks of these forms).

## References

- QSpec half, which owns the grammar and semantics of random parameters,
  workloads and rewards: QSpec FR-405 (Linear STD-137).
- Owning ticket: Linear QSL-371.

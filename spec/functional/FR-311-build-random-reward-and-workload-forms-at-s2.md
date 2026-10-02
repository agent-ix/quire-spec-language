---
id: FR-311
title: "Build random, reward and workload declaration forms at S2"
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
# FR-311: Build random, reward and workload declaration forms at S2

## Description

When a `1-draft` unit that selects the complete facets holds a `random-decl`,
a `reward-decl` or a `workload-decl`, the productions of QSpec's shared
grammar (§ "Probabilistic, timed and real-time forms", Linear STD-137), S2
(`qsl-forms`) SHALL build one declaration form for it from S1's parse, so
S3 can check it (FR-185). QSpec owns the productions and their meaning
(QSpec FR-405); this requirement specifies the forms QSL builds.

## Use case

A modeller writes `random Server::attempt(d) ~ { 1 ms: 90, 3 ms: 7, 12 ms:
3 };`, `reward duration on Server::attempt = d;` and `workload Steady on
Service { … }`. S2 builds one form per declaration with every span S3 needs
for a diagnostic.

## Inputs

- A recovery-free `LosslessCst` of a `1-draft` unit (ADR-011 E2).
- `FormsLimits`.

## Outputs

`DeclarationForm` gains three variants, one per production:

```rust
pub struct RandomForm {                 // random-decl
    pub operation: Spanned<QualifiedName>,
    pub parameter: Spanned<Identifier>,
    pub support: Vec<(Expression, Spanned<ExactValue>)>, // weighted-value, declared order
}
pub struct RewardForm {                 // reward-decl
    pub name: Spanned<Identifier>,
    pub operation: Spanned<QualifiedName>,
    pub expression: Expression,
}
pub struct WorkloadForm {               // workload-decl
    pub name: Spanned<Identifier>,
    pub model: Spanned<Identifier>,
    pub weights: Vec<(Spanned<QualifiedName>, Spanned<ExactValue>)>, // workload-weight, declared order
}
```

## Behavior

- `LeadingTokenKind` SHALL gain `Random`, `Reward` and `Workload`, each
  selecting exactly one production in the `probabilistic` family:
  `random_decl`, `reward_decl` and `workload_decl`.
- S2 SHALL build each form from its production's CST, keeping every
  `weighted-value` and `workload-weight` in declared order with its span.
- S2 SHALL leave the values' types, the weights' positivity, a workload's
  completeness and a reward's reads unchecked for S3 (FR-185).
- S2 SHALL charge each weighted value, weight and expression node against
  `FormsLimits` as it charges any other form node.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-311-AC-1 | ADR-024 §7.2's `Service`, written in the shared grammar's productions, builds two `RandomForm`s on `Server::attempt`, for `d` with `(1 ms, 90)`, `(3 ms, 7)`, `(12 ms, 3)` and for `outcome` with `(Ok, 98)`, `(Fail, 2)` in that order; one `RewardForm` `duration` on `Server::attempt` whose expression is `d`; and a `WorkloadForm` `Steady` on `Service` with three weights in declared order. Every form carries the span of its source text. | Test (TC-890) |
| FR-311-AC-2 | A unit whose declaration begins `random`, `reward` or `workload` dispatches to `random_decl`, `reward_decl` or `workload_decl` respectively, and to no other production. | Test (TC-890) |
| FR-311-AC-3 | `random Server::attempt(d) ~ { 1 ms: 0 };` and a workload that omits `Server::reset` both build at S2 and are refused only at S3 (FR-185-AC-2, FR-185-AC-3). | Test (TC-890) |

## Dependencies

- ADR-024 PM-1, PM-2, PM-5, QS-1 to QS-3; ADR-012 §3, §4.3.
- [FR-067](FR-067-add-s2-forms-and-retire-seam-5.md) (the S2 forms core),
  [FR-185](FR-185-declare-random-parameters-workloads-and-rewards.md) (the
  S3 checks of these forms).

## References

- QSpec half: the shared grammar's `random-decl`, `weighted-value`,
  `reward-decl`, `workload-decl` and `workload-weight` productions, and
  QSpec FR-405 for their meaning (Linear STD-137).
- Owning ticket: Linear QSL-371.

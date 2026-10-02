---
id: TC-596
title: "S2 builds forms for hyper clauses over behaviours and relations over model executions"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-171
    type: verifies
---
# TC-596: S2 builds forms for hyper clauses over behaviours and relations over model executions

## Description

Verify that S2 builds `HyperBehavioursForm` and `ModelRelationForm` for ADR-023's clauses, with quantifiers, aliases, fairness, `match`, `align skip`, indexed atoms and execution bindings as written, and refuses none of them.

Scope: FR-171-AC-1 to FR-171-AC-3.

## Test Procedure

Fixtures: ADR-023 §1's `NonInterference` and `SavesPower`; a quantifier with `fair { weak V::Vault::step }`; §13's clause with `align skip { V::Vault::mix }`; FR-171-AC-3's `Det`.

1. Build `NonInterference` and `SavesPower`.
2. Build the clause with the fairness constraint and the clause with `align skip`.
3. Build `Det`.

Tag the tests `#[trace("TC-596", "FR-171-AC-n")]`.

## Expected Results

- Step 1: one object parameter, two ordered quantifiers with `of V` and empty fairness sets, a `match` form with both conjuncts, the indexed atoms; two parameters and aliases `S` and `N`.
- Step 2: one fairness constraint form on the quantifier; `align_skip` holding `V::Vault::mix`.
- Step 3: executions `x` and `y` in order with the `pre` reads as written; no step refuses `UnrepresentedConstruct`.

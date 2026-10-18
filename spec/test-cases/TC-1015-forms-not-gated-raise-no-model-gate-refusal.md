---
id: TC-1015
title: "Forms this requirement does not gate raise no model-gate refusal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: verifies
---
# TC-1015: Forms this requirement does not gate raise no model-gate refusal

## Description

Verify that only the families of FR-361 item 5 are gated. Scope: FR-361-AC-4 and FR-361-AC-13. This catches a form admitted that must be refused (`deref` or `reaches` under a value-profile alias) and a gate that refuses a state clause or another form for lacking the model profile. The model-gate refusal is told apart from FR-110's layer refusal by what it retains: the model gate retains a family of item 5 and the required layer; the layer refusal names only the selected layer.

## Test Procedure

1. Compile a unit with the header profile `quire.value.complete/v1`, a `model` declaration and a `pre`, a `post` and an `invariant` against its alias.
2. For each of `Population<M::A>` as a type, `allInstances<M::A>(...)`, the field navigation `a.f` on an object-typed value, a `hyper` clause, a `relation` clause and a state-graph clause, compile one unit that holds only that form, first with the header profile `quire.model.complete/v1` and then with `quire.value.complete/v1`.
3. Compile one declaration holding a `deref` and one holding a `reaches`, each with a `using` alias that resolves first to `quire.value.complete/v1`, then to `quire.state.graph/v1`, then to `quire.model.complete/v1`; repeat with the value-profile alias in a unit whose header also selects `quire.state.graph/v1` through a second alias.

## Expected Results

1. The unit compiles.
2. Under `quire.model.complete/v1` each unit is admitted. Under `quire.value.complete/v1` each unit is admitted or refused by FR-110's layer rule naming `quire.value.complete/v1`; no refusal retains a family of item 5 or the required layer `quire.model.complete/v1`.
3. With the value-profile alias each declaration is refused once by FR-110's layer rule naming that layer, retaining no family of item 5; with the other two aliases each compiles; with the value-profile alias in the multi-profile unit each declaration is still refused by the layer rule.

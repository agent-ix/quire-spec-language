---
id: TC-114
title: "Bind exact composed dependencies and declaration-owned roles"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: verifies
---
# TC-114: Bind exact composed dependencies and declaration-owned roles

## Description

Exercise composed linking through the public Rust boundary with exact supplied
models/definitions and source units. Concrete observations are deliberately absent
from the valid template. Integration with a real domain-package bundle belongs to
TC-148 and IT-012.

`tests/composed_namespace.rs` and `tests/composed_linking.rs` exercise the source
inventory and native declaration-reference portions through `admit_namespace`.
Their unresolved profile/model selectors do not qualify later binding.
`tests/composed_binding.rs` exercises the combined three-family path with the
actual native rule-model source and exact normative definition/rule inputs.
`composed_definitions`, `composed_definition_source`, `composed_models` and
`composed_scopes` cover its adverse selections, original owners, lexical/causal
environments and bounded retries. Binding against admitted domain-package
declarations and the cross-family type/capture/instance boundary are completed
by TC-148 under IT-012. None of these static cases
claims runtime observations or family evaluation.

## Test Procedure

1. Split the standard order/refund package across two inventoried units. Include
   forward predicate references, an operation contract, a temporal requirement and
   protocol node references. Inspect exact target kinds, unit-qualified source
   identities, nominal model owners and role/anchor/scope requirements.
2. Reuse an alias spelling in different units with different exact imports. Reuse
   a clock-role spelling in distinct declarations. Require distinct owners.
3. Independently omit a required unit, add an unrelated file outside the inventory,
   duplicate a declaration/name authority, replace a temporal target with a state
   target and reference a trigger outside its scope. Check the relevant original
   source occurrences and every affected declaration's disposition.
   Supply individually valid units from different available editions in one
   inventory, then separately change the inventory edition while keeping all
   headers equal. Both conflicts must refuse namespace admission and retain the
   inventory selection and each conflicting header location. Keep an otherwise
   identical matching-edition control that admits.
4. Create self recursion, mutual predicate recursion and a definition-dependency
   cycle. Compare with a diamond dependency and explicitly bounded protocol
   repetition; the latter two are valid graph shapes.
5. Mutate one selected definition revision, digest, required dependency, model
   declaration key or domain package identity at a time. Substitute a domain
   package `sha256-jcs` digest in a native byte-digest position. Keep an unrelated valid
   declaration and require its binding to survive each dependency-local refusal.
6. Read the public accounting version, dimension charging rules, capacities and
   effective limits before traversal. Derive expected charges independently for
   controlled chain/diamond/cycle cases, including the declared treatment of
   shared dependencies, cache hits and revisits. Repeat with zero, exact and
   one-step-insufficient budgets for each exercised dimension. Check refusal
   before unaffordable work and counter-overflow exhaustion through the same
   accounting boundary. Retry with sufficient budget and inspect the original
   source/model inputs and prior report for mutation. Never infer the expected
   exact limit from the run's own reported consumption.

## Expected Results

Every inventoried declaration has an explicit disposition. Exact, acyclic
dependencies bind before downstream checking. Missing source inventory prevents
namespace admission; dependency-local failures retain unrelated bindings without
claiming complete package admission. Wrong kinds, nominal owners, anchors and
scope cannot be repaired by equal spelling or field shape. Resource exhaustion
is distinct from semantic refusal. Assertions compare typed causes and exact
locations, never an English message alone.

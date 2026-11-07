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

Steps 1 to 6 verify the retired FR-036-AC-1 to AC-4 and AC-7 and are retained as
migration history: the composed linker they exercised is deleted, and scoped
names, dependencies and work accounting are owned by FR-113, FR-218 and FR-255.
Step 7 verifies the active FR-036-AC-8 through the spine check. Binding against
admitted domain-package declarations and the cross-family type/capture/instance
boundary are completed by TC-148 under IT-012. None of these static cases
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
5. Mutate one selected definition revision, required dependency, model
   declaration key or domain package identity at a time. Keep an unrelated valid
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
7. Through `qsl_replay::spine::parse`, `select` and `check`, compile
   `protocol P using ob` with `ob` selecting `quire.observation.binding/v1`
   against the exact registered definition inventory. Inspect the typed
   per-use profile outcome on the check result: it retains the exact
   `quire.protocol.finite-global/v1` dependency and its transitive requirements,
   while the protocol site separately refuses `WrongProfileKind`. Repeat with
   the required protocol definition omitted, conflicting or incompatible; the
   per-use outcome retains the typed closure cause and the declaration remains
   refused or incomplete, with no complete checked package.

## Expected Results

Every inventoried declaration has an explicit disposition. Exact, acyclic
dependencies bind before downstream checking. Missing source inventory prevents
namespace admission; dependency-local failures retain unrelated bindings without
claiming complete package admission. Wrong kinds, nominal owners, anchors and
scope cannot be repaired by equal spelling or field shape. Resource exhaustion
is distinct from semantic refusal. Assertions compare typed causes and exact
locations, never an English message alone.

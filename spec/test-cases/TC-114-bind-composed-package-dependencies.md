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
from the valid template. Integration with the real producer belongs to IT-009.

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
4. Create self recursion, mutual predicate recursion and a definition-dependency
   cycle. Compare with a diamond dependency and explicitly bounded protocol
   repetition; the latter two are valid graph shapes.
5. Mutate one selected definition revision, digest, required dependency, model
   export owner or producer/native correspondence at a time. Substitute a model
   canonical digest in its native byte-digest position. Keep an unrelated valid
   declaration and require its binding to survive each dependency-local refusal.
6. Repeat chain/diamond/cycle cases with zero, exact and one-step-insufficient
   traversal budgets. Retry with sufficient budget and inspect the original
   source/model inputs for mutation.

## Expected Results

Every inventoried declaration has an explicit disposition. Exact, acyclic
dependencies bind before downstream checking. Missing source inventory prevents
namespace admission; dependency-local failures retain unrelated bindings without
claiming complete package admission. Wrong kinds, nominal owners, anchors and
scope cannot be repaired by equal spelling or field shape. Resource exhaustion
is distinct from semantic refusal. Assertions compare typed causes and exact
locations, never an English message alone.

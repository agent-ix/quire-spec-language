---
id: TC-115
title: "Preserve static meaning and every requested capability"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: verifies
---
# TC-115: Preserve static meaning and every requested capability

## Description

Exercise separation of static linking, runtime binding and downstream capability
disposition. These are planned compiler controls, not temporal-engine qualification.

## Test Procedure

1. Link the same three-family source/model/definition inventory with no runtime
   observations. Compare its retained static components while varying supplied
   assessment population, window, trace and backend selections independently.
2. Change only a compile resource limit. Require unchanged static meaning and a
   changed retained configuration. Then explicitly change a required profile or
   model selection and require a different static selection; stale dependencies
   must refuse rather than inherit the new meaning.
3. Request one supported state operation and one recognized but unsupported
   temporal projection. Inspect both clause/capability entries and the aggregate
   disposition. Delete either entry in the consumer test input and require the
   requested-inventory check to fail.
4. Return an unsupported family-checking result for one parsed declaration.
   Require its syntax to remain inspectable without presenting its body as checked
   or executable. A backend refusal must not rewrite source/profile selection.
5. Run historical package reading/rebinding controls unchanged. Attempt to submit
   a partial composed report through the historical reader/runner after replacing
   only its profile label.

## Expected Results

Static comparisons use the declared semantic components, not an invented
canonical hash. Assessment and backend changes do not change those components.
Every requested capability remains visible, and any required unsupported request
prevents complete aggregate success. The historical runner refuses incompatible
composed input; its accepted fixture bytes and identities remain unchanged.

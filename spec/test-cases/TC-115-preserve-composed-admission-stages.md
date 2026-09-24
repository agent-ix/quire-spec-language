---
id: TC-115
title: "Preserve static meaning and every requested capability"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-115: Preserve static meaning and every requested capability

## Description

Exercise separation of static linking, runtime binding and downstream capability
disposition. These are compiler controls, not temporal-engine qualification.

## Test Procedure

1. Link the same three-family source/model/definition inventory with no runtime
   observations. Compare its retained static components while varying supplied
   assessment population, window, trace and backend selections independently.
2. Change only a compile resource limit. Require unchanged static meaning and a
   changed retained configuration. Then explicitly change a required profile or
   model selection and require a different static selection; stale dependencies
   must refuse rather than inherit the new meaning.
3. Request two required pairs, `operation-contract` and
   `temporal-satisfaction`. Supply `negotiate_*` settlements as fixture records:
   `operation-contract` settles `supported` and `temporal-satisfaction` settles
   `unsupported` for an empty candidate set. Inspect both clause/capability
   entries and the aggregate joined from the FR-331 accounting records on
   request index. Delete either entry in the consumer test input and require
   the requested-inventory check to fail. Then request `operation-contract`
   (required) and `finite-replay` (not required) on one state declaration.
4. Have the family checker refuse one parsed declaration with
   `unsupported_construct`/`declaration-form`. Require its syntax to remain
   inspectable without presenting its body as checked or executable. Neither
   that refusal nor an `unsupported` settlement rewrites source/profile
   selection.
5. Run historical package reading/rebinding controls unchanged. Attempt to submit
   a partial composed report through the historical reader/runner after replacing
   only its profile label.

## Expected Results

Static comparisons use the declared semantic components, not an invented
canonical hash. Assessment and backend changes do not change those components.
Both requested pairs remain visible at their request indices, and the required
`unsupported` pair makes complete aggregate success unavailable. On the state
declaration, `operation-contract` is admitted and `finite-replay` is an
inapplicable capability naming the state family (FR-057-AC-11); the declaration
body is still admitted. The historical runner refuses incompatible
composed input; its accepted fixture bytes and identities remain unchanged.

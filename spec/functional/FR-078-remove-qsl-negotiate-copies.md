---
id: FR-078
title: "Remove QSL's negotiate_* copies from value::ieee and value::division"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-011
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
---
# FR-078: Remove QSL's negotiate_* copies from value::ieee and value::division

## Description

QSL SHALL remove the `negotiate_*` functions in `value::ieee` and
`value::division` (`negotiate_ieee` and `negotiate_integer_division`, and
the `IeeeBackendCapabilities` type they use). These are RT capability
predicates, not settlement points: `quire-contract-codegen`'s `negotiate_*`
arms over the closed backend-kind enum (S9) are the only settlement points
(ADR-012 §5.1, §10 OBS-004). QSL's value semantics SHALL retain their
evaluation functions for IEEE and integer-division arithmetic; those
functions evaluate values and are not negotiation, and this requirement
removes no evaluation behavior.

## Inputs

- `value::ieee` and `value::division` as they exist on `main`, including
  their `negotiate_*` functions and `IeeeBackendCapabilities`.

## Outputs

- `value::ieee` and `value::division` with no `negotiate_*` function and no
  `IeeeBackendCapabilities` type, and with their evaluation functions
  behaviorally unchanged.

## Behavior

### The copies are removed, not renamed or gated

`value::ieee` and `value::division` SHALL define no function whose name
matches the pattern `negotiate_*`, and SHALL define no
`IeeeBackendCapabilities` type or equivalent capability-predicate type under
another name serving the same purpose. Renaming the function, moving it
behind a feature flag, or reintroducing an equivalent predicate under a
different identifier does not satisfy this requirement; the negotiation-
shaped logic is removed from QSL, not relocated within QSL.

### Evaluation functions are unaffected

`value::ieee` and `value::division` SHALL retain every function that
evaluates an IEEE or integer-division value expression, with unchanged
input/output behavior. A test suite exercising these evaluation functions
before the removal SHALL pass unchanged after it.

### CG may take these predicates as inputs, but QSL does not own them

A `quire-contract-codegen` `negotiate_*` arm MAY take an equivalent
predicate as one of its own inputs when CG needs it (ADR-012 §10 OBS-004);
that predicate, if CG defines one, is CG's own code, not a QSL copy this
requirement is required to keep in sync.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-078-AC-1 | `value::ieee` defines no `negotiate_*`-named function and no `IeeeBackendCapabilities` type, verified by a source scan of the module after removal. | Test (TC-201) |
| FR-078-AC-2 | `value::division` defines no `negotiate_*`-named function, verified by a source scan of the module after removal. | Test (TC-201) |
| FR-078-AC-3 | Given a fixture set of IEEE and integer-division value expressions and their expected evaluated results, evaluating each fixture through `value::ieee` and `value::division` after the removal yields the same results as before the removal. | Test (TC-202) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §10
  (ADR-010 OBS-004): "`negotiate_integer_division`, `negotiate_ieee` and
  `IeeeBackendCapabilities` are RT capability predicates... they are not
  settlement points and settle no disposition... QSL has no copy of them;
  #213 S-1 removes the QSL copies when X-1 moves `division` and `ieee` into
  `quire-exact`... QSL value semantics keep their evaluation functions,
  which are not negotiation."
- ADR-012 §14.1 assigns this removal to
  [quire-spec-language#213](https://github.com/agent-ix/quire-spec-language/issues/213)
  S-1, triggered when X-1 moves `division` and `ieee` into `quire-exact`
  (ADR-011 §6.2); this requirement specifies the removal's observable
  behavior for `#185`'s scope regardless of which ticket lands the commit.

## Status

Specified under
[quire-spec-language#185](https://github.com/agent-ix/quire-spec-language/issues/185).
Not yet implemented; `value::ieee` and `value::division` still define
`negotiate_ieee`, `negotiate_integer_division` and
`IeeeBackendCapabilities` on `main`.

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
| FR-078-AC-1 | **RETIRED by QSL-131 O3 (2026-09-23), which deletes `value::ieee` as a module.** `value::ieee` defined no `negotiate_*`-named function and no `IeeeBackendCapabilities` type, verified by a source scan of the module after removal. This criterion was satisfied when QSL-131 Slice A (PR #290) removed `negotiate_ieee` and `IeeeBackendCapabilities` from the file. `value::ieee` no longer exists at all: QSL-131 O3 deletes the module and repoints every caller onto `quire_exact::evaluate_ieee`/`compare_ieee` directly, so there is no module left for a source scan to read. The criterion's claim was true when made and is now permanently unfalsifiable rather than false; it is retired, not amended, since there is no successor module for it to describe. TC-201's doctest continues to assert the stronger, still-live fact that `negotiate_ieee` and `IeeeBackendCapabilities` are unresolvable from the crate's `value` path at all. | Test (TC-201), retired |
| FR-078-AC-2 | **RETIRED by QSL-131 K2 (#339), which deletes `value::division` as a module.** `value::division` defined no `negotiate_*`-named function, verified by a source scan of the module after removal. This criterion was satisfied when QSL-131 Slice A (PR #290) removed `negotiate_integer_division` from the file. `value::division` no longer exists at all: QSL-131 K2 deletes the module and repoints every caller onto `quire_exact::divide`/`modulo` directly, so there is no module left for a source scan to read. The criterion's claim was true when made and is now permanently unfalsifiable rather than false; it is retired, not amended, since there is no successor module for it to describe. | Test (TC-201), retired |
| FR-078-AC-3 | Given a fixture set of IEEE and integer-division value expressions and their expected evaluated results, evaluating each fixture through `value::ieee` and `value::division` after the removal yields the same results as before the removal. | Test (TC-202) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §10
  (ADR-010 OBS-004): "`negotiate_integer_division`, `negotiate_ieee` and
  `IeeeBackendCapabilities` are RT capability predicates... they are not
  settlement points and settle no disposition... QSL has no copy of them:
  X-1 (#213 S-1) moved `division` and `ieee` into `quire-exact` without
  cutting these (its own summary said so), so QSL carried both until
  QSL-131 removed them (ADR-011 §6.1's K-leaf bullet). QSL value semantics
  keep their evaluation functions, which are not negotiation."
- ADR-012 §14.1's ownership row assigns "Removal of the QSL `negotiate_*`
  copies from `value::ieee` and `value::division` (OBS-004); `quire-exact`'s
  own `division`/`ieee` (moved in by X-1, #213 S-1) never carried them" to
  QSL-131, which implements this requirement.

## Status

Specified under
[quire-spec-language#185](https://github.com/agent-ix/quire-spec-language/issues/185).
Implemented by QSL-131 (PR #290): `value::ieee` and `value::division` define
no `negotiate_*` function and no `IeeeBackendCapabilities` type.

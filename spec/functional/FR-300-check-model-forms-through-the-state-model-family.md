---
id: FR-300
title: "Check model expression forms through the StateModel family"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/US-006
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-063
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-300: Check model expression forms through the StateModel family

## Description

When the S3 checker types a model expression form nested in an enclosing
family's body, the checker SHALL type that form through the `StateModel`
family's `check` hook, and every refusal that hook raises SHALL be a
`StateModel`-owned cause.

## Inputs

- One model expression form, nested in a `Value` function body or a
  `ProtocolClause` body: `deref` (a field read, `NodeKind::Attribute`),
  `allInstances<T>(p)`, `lookup<T>(p, r)`, `reaches` inside a clause, or a
  dispatched call
  `receiver.member(args)` (QSpec FR-151 rule
  `quire.model.dispatch.single/v1`).
- The shared `CheckContext` the enclosing family's typer holds, including the
  unit's admitted model effective view and its `TypeEnvironment`
  ([FR-082](FR-082-resolve-conformance-subsetting-and-redefinition.md)).

## Outputs

The form's checked node (`NodeKind::Attribute`, `AllInstances`, `Lookup`,
`Reaches`, or a dispatched call bound to its FR-083 dispatch table) and its
static type, or a located refusal whose cause belongs to `StateModel`.

## Behavior

### One hook per form

The `StateModel` family SHALL implement `FamilyContract` (ADR-012 §1,
[FR-062](FR-062-implement-checked-family-contract.md)) with a `check` hook
that takes one model expression form and the shared `CheckContext`, as the
`Value` family's function-application checker takes one nested call
(ADR-016 FP-2). The enclosing family's typer SHALL type each of the five
model forms by calling that hook from a thin arm. The hook SHALL answer the
static questions ADR-016 SC-1 to SC-4 assign to S3: member existence on a
static object type, static conformance, the dispatched call's selected
operation, and a population's declared maximum and element type. It reads no
population, snapshot or universe.

### Causes are attributed to StateModel

Every refusal the hook raises SHALL carry `FamilyKind::StateModel`. Its
catalog code SHALL render with the `state-model` family prefix (the
`FamilyKind::StateModel` arm of `catalog_code()`), and its QSpec code and
cause SHALL be the ones QSpec FR-151 and FR-153 fix for that question (for
example `missing_declaration`/`missing-name` for an absent member, or
`ill_typed`/`type-mismatch` for a `lookup` key whose static type does not
conform).

### Seams

Each `match` that calls a family's S2 or S3 hook SHALL have a `StateModel`
arm and no `_` arm, and the `catalog_code()` family-prefix match SHALL keep
its `StateModel` arm. The FR-063 seam probe SHALL report each of these
`StateModel` arms: the S1 `catalog_code()` prefix arm, the S2 hook arm and
the S3 hook arm. `StateModel` takes part in no S6a seam (ADR-016 FP-3).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-300-AC-1 | For each of the five model forms, a unit whose form is well-typed compiles, and the form's checked node and static type are the ones QSpec FR-151 and FR-153 fix (`deref` typed by its declared field type, `allInstances<T>(p)` as `Set<T>[0,n]` for a population declared with maximum `n`, `lookup<T>(p, r)` as `T`). | Test (TC-790) |
| FR-300-AC-2 | For each of the five model forms, an ill-formed instance (a read of a member the static type does not declare, a `lookup` key whose static type does not conform, a dispatched call to an undeclared operation) refuses at S3 with a cause whose family is `FamilyKind::StateModel`, whose catalog code carries the `state-model` prefix, and whose QSpec code and cause are the ones QSpec FR-151 or FR-153 fix. | Test (TC-790) |
| FR-300-AC-3 | `xtask seam-probe` reports the `StateModel` arm of the S1 `catalog_code()` prefix match, of the S2 hook match and of the S3 hook match as seam locations, and fails when any of the three is absent from its report. | Test (TC-791) |

## Dependencies

- **Upstream:** ADR-012 §1, §3 and §5.1 define the family contract and the
  seams; ADR-016 FP-1 to FP-3 place the model forms in `StateModel` and give
  it no S6a hook; [FR-063](FR-063-exhaustive-family-extension-seam-probe.md)
  supplies the seam probe; QSpec FR-151 and FR-153 own the model semantics
  and their catalog codes.
- **Downstream:** [FR-301](FR-301-render-state-model-causes-under-the-enclosing-family.md)
  carries the same attribution into evaluation;
  [FR-302](FR-302-link-dispatch-during-unit-compile.md) links the dispatch
  table a dispatched call's checked node names.

## References

- ADR-016 §9 G-2a and G-2b.
- Linear QSL-382 (specification), QSL-68 (implementation).

---
id: FR-135
title: "Check a refinement declaration's subjects and state mapping at S3"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
---
# FR-135: Check a refinement declaration's subjects and state mapping at S3

## Description

QSL's layer-3 `check` module SHALL check a refinement declaration as a
`Relation` family declaration (ADR-012 §3, ADR-017 AR-1) and produce a
`CheckedRefinement` (ADR-020 RM-1 to RM-4). This requirement covers the
header, the population map and the object map. Step rows are FR-136,
fairness rows FR-137, history rows FR-138 and population-valued expressions
FR-139. The declaration's surface spelling is the shared grammar's
(ADR-020 QS-1); this requirement checks the parsed S2 form.

## Use case

A specification author writes a refinement between an abstract model and a
concrete model. QSL tells them, at compile time and at the source span,
which abstract population or field has no mapping, which has two, and
which mapping expression has the wrong type, before any model check runs.

## Inputs

- The parsed S2 refinement form: name, profile selection, abstract and
  concrete model aliases, population rows, object rows with field rows, step
  rows, history rows and fairness rows, each with its span.
- The unit's admitted domain packages and model declarations (FR-103), the
  resolved profile selection (FR-110), and, when the abstract alias names a
  dependency's model, that dependency's checked package from the S4
  closure (ADR-015).

## Outputs

```rust
pub struct CheckedRefinement {
    pub name: DeclarationKey,
    pub profile: ProfileSelection,              // infinite-trace
    pub abstract_side: AbstractSide,
    pub concrete_model: ModelAlias,             // the declaring package
    pub populations: Vec<PopulationRow>,        // abstract pop <- concrete pop
    pub objects: Vec<ObjectRow>,
    pub steps: Vec<StepRow>,                    // FR-136
    pub history: Vec<HistoryRow>,               // FR-138
    pub concrete_fairness: Vec<FairnessConstraint>, // F_C, FR-137
    pub abstract_fairness: Vec<FairnessConstraint>, // F_A, FR-137
}

pub enum AbstractSide {
    Local(ModelAlias),
    Dependency { alias: ModelAlias, package_id: PackageId },
}

pub struct ObjectRow {
    pub abstract_type: DeclarationKey,
    pub concrete_type: DeclarationKey,
    pub fields: Vec<FieldRow>,                  // visible fields
    pub hidden: Vec<DeclarationKey>,            // ADR-020 AX-2
}

pub struct FieldRow { pub field: DeclarationKey, pub expr: CheckedExpr }
```

A typed `CheckRefusal` with a span, and no `CheckedRefinement`, on refusal.

## Behavior

### Header and subjects

- The checker SHALL resolve the concrete alias to a `model` declaration of
  the declaring unit, and the abstract alias to a `model` declaration of the
  declaring unit or of a dependency package compiled against supplied
  libraries. An alias that resolves to neither SHALL refuse
  `missing_declaration`/`missing-name` at the alias's span.
- The checker SHALL refuse a refinement whose unit does not select the
  infinite-trace profile with `unknown_profile`/`wrong-selection-role` at
  the alias, as QSpec FR-375 states.
- The abstract subject's contract clauses SHALL be read from the declaring
  package when the abstract alias is local, and from the dependency's
  checked package otherwise.

### Population map

- For each population of the abstract model, the checker SHALL require
  exactly one `population` row naming it on the left and one population of
  the concrete model on the right. An abstract population with no row SHALL
  refuse `missing_declaration`/`missing-name`, naming the population; one
  with two rows SHALL refuse `invalid_model_binding`/`conflicting-binding`,
  naming both rows.
- A row whose left side is not an abstract population, or whose right side
  is not a concrete population, SHALL refuse
  `invalid_model_binding`/`malformed-declaration` at the row's span.
- A concrete population that no row names SHALL be concrete-only state and
  SHALL need no row.

### Object map

- For each object type that an abstract population admits, the checker
  SHALL require exactly one `object` row whose `from` names an object type
  of the row's source population. A missing row SHALL refuse
  `missing_declaration`/`missing-name`, naming the abstract type; two rows
  SHALL refuse `invalid_model_binding`/`conflicting-binding`; a `from` type
  outside the source population SHALL refuse
  `invalid_model_binding`/`malformed-declaration`.
- Each field row `f = e` SHALL name a declared field of the abstract type.
  A field named twice SHALL refuse
  `invalid_model_binding`/`conflicting-binding`, naming both rows. An
  undeclared field SHALL refuse `missing_declaration`/`missing-name`.
- The checker SHALL check `e` with `self` bound to the concrete object type
  and the concrete model's fields, functions, history fields (FR-138) and
  population-valued forms (FR-139) in scope, admitting the forms a state
  clause body admits (ADR-016 FE-4).
- The checker SHALL lift references: a field of declared type
  `Reference<T_A>` SHALL accept an expression of type `Reference<T_C>`
  exactly when an object row maps `T_A` from `T_C`.
- The checker SHALL check `e`'s type against the abstract field's declared
  type, after reference lifting, with `TypeEnvironment::conforms` (ADR-016
  SC-2), and SHALL refuse `ill_typed`/`type-mismatch` at `e`'s span when it
  does not conform.
- An abstract field with no row SHALL be recorded in `hidden` (ADR-020
  AX-2).

### Package and identity

- The `CheckedRefinement` SHALL enter the checked package as a node, so its
  identity enters the `package_id` and the obligation identity of every
  item that cites it (ADR-013 O-02, O-09). Its v2 wire node is QSpec's
  (ADR-020 QS-4).
- Two refinement declarations that differ in any row SHALL have different
  node identities.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-135-AC-1 | ADR-020 §8's `CasRefinesCounter`, with `Spec` and `Impl` both local, checks to a `CheckedRefinement` with one population row (`counters` from `counters`), one object row with the field row `value = self.value` and no hidden field. The same declaration with `Spec` supplied as a dependency package checks with `AbstractSide::Dependency` carrying that package's `package_id`. | Test (TC-540) |
| FR-135-AC-2 | Refusals, each at its span with no `CheckedRefinement`: the abstract alias `Nope`, resolving to no model (`missing_declaration`/`missing-name`); the `population` row removed (`missing_declaration`/`missing-name` naming `Spec::counters`); the row written twice (`invalid_model_binding`/`conflicting-binding`); the row reversed as `population Impl::counters from Spec::counters` (`invalid_model_binding`/`malformed-declaration`); the `object` row removed (`missing_declaration`/`missing-name` naming `Spec::Counter`); `value` mapped twice (`invalid_model_binding`/`conflicting-binding` naming both rows); the declaration in a unit that selects a bounded profile (`unknown_profile`/`wrong-selection-role` at the alias). | Test (TC-540) |
| FR-135-AC-3 | `value = self.busyA` refuses `ill_typed`/`type-mismatch` at the expression's span. In ADR-020 §8's `RingIsQueue`, a field of type `Reference<Q::Queue>` mapped by `self.ring` (of type `Reference<R::Ring>`) checks, and the same field mapped by an expression of type `Reference<R::Slot>` refuses `ill_typed`/`type-mismatch`. | Test (TC-540) |
| FR-135-AC-4 | With the `value` row removed, `CasRefinesCounter` checks with `value` in `hidden`. Changing the `value` row to `value = self.tmpA` changes the refinement's node identity and the `package_id`. | Test (TC-540) |

## Dependencies

- ADR-020 §1 RM-1 to RM-4 and §4 AX-2; ADR-012 §3; ADR-017 AR-1; ADR-016
  SC-2 and FE-4; ADR-013 O-02 and O-09; ADR-015.
- [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md) (admitted models),
  [FR-110](FR-110-resolve-header-profile-selections-at-e3.md) (profile
  selection).
- FR-136 to FR-139 check the remaining rows into the same
  `CheckedRefinement`.

## References

- The QSpec half (grammar, semantics, wire node): QSpec FR-375 (Linear STD-133).

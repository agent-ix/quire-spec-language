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
- The checker SHALL lift a reference only at the top level of a row's type,
  and SHALL admit a whole-population reference row (`Set<Reference<T_C>>[0,
  n]`, FR-139) as FR-139 types it. A field row whose type holds a
  `Reference<T_C>` nested inside an option, record, union or collection type
  that is not such a population reference SHALL use the feature
  `nested-reference-row` (below), because lifting inside a nested value is not
  built (ADR-020 RM-3 states the top-level rule only).
- A refinement whose abstract model declares a relationship SHALL use the
  feature `abstract-relationships` (below): no row maps a relationship
  (ADR-020 RM-2 to RM-4), so its links could never hold in a mapped state and
  every abstract clause that reads one would pass or refute by accident.
- The checker SHALL check `e`'s type against the abstract field's declared
  type, after reference lifting, with `TypeEnvironment::conforms` (ADR-016
  SC-2), and SHALL refuse `ill_typed`/`type-mismatch` at `e`'s span when it
  does not conform.
- An abstract field with no row SHALL be recorded in `hidden` (ADR-020
  AX-2).

### Refinement features the build does not check

QSpec FR-375 and the FR-136 to FR-149 family specify more refinement forms
than a build may yet check. QSpec FR-375 owns the closed set of feature names
and the not-yet-built refusal; this section states how S3 applies them. The
build holds a closed **support set** of refinement feature names,
a build capability constant and not a request or test parameter. It names
three features: `history-row` (FR-138), spelled by a `history` row;
`protocol-abstract-side` (FR-147), spelled by `abstract <A>::<protocol>`; and
`refinement-internal-list` (FR-147), spelled by a non-empty resolved internal
list for the refinement, which is located at the first token of the name after
`abstract`, once per refinement. The list is resolved first and only a
non-empty resolved list reaches this refusal (FR-147).

- A declaration that uses a feature outside the support set SHALL refuse
  `unsupported_construct`/`not-yet-implemented`, once for each row or refinement that uses
  the feature, and SHALL produce no `CheckedRefinement`. This is the refusal
  FR-341-AC-5 classifies `unsupported` (a QSL implementation gap).
- The refusal's span is the first token of the row for a `history` row (the
  `history` keyword), and the span of the `<protocol>` name for a protocol
  abstract side. It names the feature, the refinement's name and the exact
  selected profile identity (QSpec FR-272's payload for the code).
- The refusal is decided before the feature's own checks, so no other
  diagnostic about that row is reported; the rest of the declaration is
  checked as usual and its own refusals are reported beside it.
- `nested-reference-row` is spelled by a field row of the first kind above,
  with the span of the row's expression. `abstract-relationships` is spelled
  by an abstract model that declares a relationship, with the span of the
  abstract alias in the declaration header. Both names are QSL-proposed:
  QSpec FR-375's closed set holds only the three features above, and the
  QSpec asks add them. `abstract-relationships` is a gap in QSpec, which
  defines no mapping of links, not a feature this build has left unbuilt.
- A feature in the support set is checked by its own requirement.

`unknown_required_feature` is not used: a source refusal, which FR-341
classifies `refused`, would tell the author the unit is wrong when the build
is incomplete. `unimplemented_capability` is not used: FR-271 defines it as
valid complete-V1 meaning the selected producer does not implement, which
fits, but FR-272 lists no cause for it. `not-yet-implemented` under
`unsupported_construct` is the pair QSpec FR-375 and FR-272 name for this
standing.

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
| FR-135-AC-5 | A declaration with one `history` row (`RegisterHistory`, FR-138), in a build whose support set omits `history-row`, refuses `unsupported_construct`/`not-yet-implemented` at the `history` keyword of that row naming `history-row`, `RegisterHistory` and the profile identity, with no other diagnostic for the row and no `CheckedRefinement`; two such rows give two refusals. A declaration whose abstract side is a protocol (`CasTwice`, FR-147), in a build whose support set omits `protocol-abstract-side`, refuses the same pair at the span of `Twice`, naming `protocol-abstract-side`. Both classify `unsupported` (FR-341-AC-5). A refinement given a non-empty resolved internal list refuses the same pair naming `refinement-internal-list` at the first token of the name after `abstract`, and a list with a resolution defect reports the defect and not this pair (FR-147-AC-7, AC-11). `CasRefinesCounter`, which uses none of the three features, checks as FR-135-AC-1 states. The vector refuses a row that uses a feature the build has not built; it does not select a support set. | Test (TC-540) |
| FR-135-AC-6 | `RingIsQueue` with a `Sequence<Reference<Queue>>[0, 2]` field mapped by a row of type `Sequence<Reference<Ring>>[0, 2]` refuses `unsupported_construct`/`not-yet-implemented` naming `nested-reference-row` at the row's expression; a population-reference row `cells = R::slots` checks, as does the same field as a plain `Reference<Queue>` mapped by `self`. | Test (TC-540) |
| FR-135-AC-7 | `RingIsQueue` with the abstract `Queue` declaring a relationship refuses `unsupported_construct`/`not-yet-implemented` naming `abstract-relationships` at the abstract alias `Q` of the declaration header, and no `CheckedRefinement`. | Test (TC-540) |

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

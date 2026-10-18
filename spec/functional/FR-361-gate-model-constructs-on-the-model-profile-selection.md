---
id: FR-361
title: "Gate every model construct on the unit's selection of the model profile"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-309
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-001
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-271
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-272
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-453
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-450
    type: depends_on
  - target: ix://agent-ix/quire-specification/ADR-006
    type: depends_on
---
# FR-361: Gate every model construct on the unit's selection of the model profile

## Description

When a unit's header selects `quire.model.complete/v1`, QSL SHALL admit the
model constructs this requirement lists. When a unit's header selects no
header profile that resolves to `quire.model.complete/v1`, QSL SHALL refuse
each such construct the unit itself writes with `unsupported_construct`, at
the construct, and SHALL produce no checked package.

QSpec's shared grammar states the rule for the `operation-body`, the
dispatched operation call and `lookup` form, and QSpec FR-450-AC-3 states it
for the `abstraction-decl`; QSL FR-304 applies it to the abstraction form.
Until this requirement is built, QSL accepts every one of them under a package
that selects only `quire.value.complete/v1`, and FR-102 and FR-302 record that
the gate is not enforced. This requirement is the target state. FR-110's
layer selection (FR-110-AC-1, AC-7, AC-8) is built, so the header resolves
`quire.model.complete/v1`; the gate of this requirement is not. The sentences
that record the missing gate in FR-102, FR-302 and in this paragraph are
deleted when the gate of this requirement is implemented.

A **model construct** is a syntactic form listed in item 5. The unit
**selects the model profile** when at least one of its header `profile`
declarations names the identity of the `quire.model.complete/v1` row of the
DefinitionLock catalog's `header_selectable_layers` (FR-110).

## Target design

### Selection

1. **A header selection and nothing else.** A unit selects the model
   profile only by naming `quire.model.complete/v1` in a header
   `profile <alias> = "<identity>";` declaration. QSL SHALL NOT infer the
   selection from a construct, from a `model` declaration, from a
   supplied domain package, from an alias or from the fixture a unit
   belongs to, and SHALL NOT fall back to `quire.value.complete/v1` when the
   selection is absent.
2. **One resolution path.** E3 resolves the identity against the same
   `header_selectable_layers` rows FR-110 uses for the other four layers. The
   model profile adds no second header-profile mechanism and no new refusal
   code. The five rows (`quire.state.core/v1`, `quire.state.queries/v1`,
   `quire.state.graph/v1`, `quire.value.complete/v1` and
   `quire.model.complete/v1`) are header-selectable (FR-110, QSpec FR-453).
   Every other header identity keeps FR-110's routing: a clause profile
   resolves in its clause family's catalog (FR-110, ADR-011), and an identity
   no catalog holds refuses `unknown_profile`/`unsupported-selection`, while
   a `qualification_catalog` row only refuses
   `unknown_profile`/`wrong-selection-role`.
3. **The catalog holds the model lock.** `DefinitionLock::pinned()` SHALL hold
   the rows of QSpec's complete-model lock beside the complete-value lock's
   (the complete-model lock requires the complete-value lock). Both locks name
   a role `root` (the complete-value root and the complete-model root). The
   `root` of FR-110's header refusal, and the `root` row it is compared with,
   stay the complete-value lock's, as FR-110 states. The complete-model
   `root` row is the `quire.model.complete/v1` layer, reached through
   `header_selectable_layers`, so the two never collide. A unit whose header
   profiles resolve to the model layer SHALL emit, in `definition_selections`,
   the layer, every layer its `requires` closure names (FR-110), the
   complete-value lock's `package_selection` rows FR-110 selects for a union
   holding `quire.value.complete/v1`, and the complete-model lock's
   `package_selection.always` rows.
4. **Closure.** Selecting the model layer admits the forms of the layers its
   `requires` closure names (FR-110 "Layer selection"). A unit that selects
   the model profile therefore needs no second header profile to use
   value-complete forms.

### Model construct families

5. QSL SHALL treat these as the model constructs, each with its refusal
   cause. QSpec's shared grammar names the first, third and fourth rows, and
   QSpec FR-450-AC-3 names the second.

   | Family | Construct | Cause |
   | --- | --- | --- |
   | `operation-body` | the `body <label> using <alias> on <model alias>::...` declaration | `declaration-form` |
   | `abstraction` | the `abstraction-decl` declaration | `declaration-form` |
   | `dispatch-call` | a postfix `receiver.member(args)` operation call | `expression-form` |
   | `lookup` | the `lookup<T>(p, r) absent m` expression | `expression-form` |

6. **The `model` declaration is ungated.** QSpec states no gate for the
   `model` declaration, and QSL adds none. State clauses (`invariant`, `pre`
   and `post`) name a model alias, so a unit that selects any of the four
   other layers writes them against a model declared in that unit and is
   admitted (FR-110 "Layer selection": state core admits them). The gate is
   on the forms the unit itself writes. A unit that does not select the model
   profile can still name a model through an import `as` alias of a library
   that selects it (QSpec's shared grammar allows an import `as` alias as the
   model alias); the forms of item 5 that the unit writes against that alias
   are refused, but the leak by which the unit reaches the library's model
   through clauses is not closed by this requirement, and no QSpec row is
   requested for it.
7. **Forms this requirement does not gate.** QSL does not gate the
   `Population<T>` type, `allInstances`, field or relationship navigation, the
   `hyper`, `relation` and state-graph clauses, or the state-clause family.
   For `Population<T>`, `allInstances` and navigation this is QSL's reading:
   QSpec's shared grammar closes its model-forms paragraph with a refusal
   rule that can be read to cover those forms along with navigation, and
   leaves open which of them it covers. A QSpec clarification of that is
   requested; until it arrives QSL gates exactly the families of
   item 5. FR-110's layer rule still refuses a form outside a selected
   layer's admitted-form set (for example `reaches`, which state core
   refuses). The object navigation forms `deref` and `reaches` are gated by
   that layer rule alone, with no exception: item 17 states it.
8. **Protocol, memory and temporal forms are not gated by this requirement.**
   QSpec's shared grammar (its facet grammar section) names
   `quire.protocol.complete/v1` as the facet of the protocol forms, including
   the memory clause, memory order and fence (QSpec ADR-006 makes it the one
   protocol admission profile), and `quire.temporal.complete/v1` as the
   temporal facet. Neither is a model construct, and QSL's
   `header_selectable_layers` has no row for either. Not gated: QSpec names
   `quire.protocol.complete/v1` and `quire.temporal.complete/v1`; QSL has no
   such layer. QSL adds no layer and no gate for them here; FR-308 and FR-309
   name the protocol facet accordingly.
9. **Why the test is on the unit.** QSpec's shared grammar says a recognised
   construct whose required facet is absent receives a located
   `unsupported_construct` for a package, and QSpec FR-001 refuses a form with
   `unsupported_construct` naming the unit's selected layers. FR-110's
   per-alias layer rule alone would leave a construct with no `using` alias
   (for example `abstraction-decl`) ungated. FR-110 and FR-453 state no
   unit-level model rule themselves; the unit-level reading is QSpec's
   package-level wording, applied by QSL to the families of item 5.

### Stage, precedence and origin

10. **E3 selects and S2 builds.** E3 resolves the header selection (FR-110).
    S2 builds each construct's form from the CST whether or not the unit
    selects the model profile. S2 does not resolve the profile and does not
    refuse a model construct (FR-102). The S2 sentence of FR-304 that had S2
    refuse an unselected abstraction form states the S3 rule of this
    requirement.
11. **S3 refuses.** S3 SHALL refuse each model construct in a unit that does
    not select the model profile, in source order, and SHALL report every
    such construct, not the first. Each refusal is `unsupported_construct`
    (QSpec FR-271) with the cause of item 5 (QSpec FR-272, native-diagnostics).
    Its origin is the construct's source span: the declaration's span for a
    declaration form, the expression's span for an expression form. The
    refusal retains the recognised form's family, the owning declaration, the
    header layers the unit selected in source order, and the required layer
    `quire.model.complete/v1`. The refusal is S3's, so a unit with an E3
    refusal (FR-110) reports that one and produces no S3 refusal.
12. **One refusal per construct.** In a unit that does not select the model
    profile, the refusal of item 11 is the only refusal a construct of item 5
    receives. FR-110's per-alias layer rule does not also report it, even when
    its `using` alias resolves to a layer whose admitted-form set does not
    name the form. The reason is that the unit-level cause (the model profile
    is not selected) is the one a user can fix; the layer rule's refusal names
    only a layer. In a unit that selects the model profile, a construct whose
    `using` alias resolves to a lower layer is refused by FR-110's layer rule
    alone, and this requirement reports nothing.
13. **Presence in unreachable syntax counts.** A model construct in a
    declaration nothing uses is refused the same way (QSpec
    native-diagnostics: presence in unreachable syntax does not erase the
    refusal).
14. **Per unit.** The test applies to the unit being compiled and, separately,
    to each library the S4 source resolution compiles (FR-110 "Placement"). A
    library that selects the model profile does not give its importer the
    selection, and an importer's selection does not give its library the
    selection.
15. **No produced package.** A unit with a model-construct refusal produces
    no checked package, no lock and no bytes (ADR-011 §2.3, S3 row).

### Admission under the selection

16. A unit that selects the model profile SHALL admit each family of item 5
    exactly as FR-056, FR-102, FR-103, FR-302 and FR-304 state. This
    requirement changes no behaviour of a selecting unit.

### Retiring the object navigation exception

17. **The `quire.value.complete/v1` admitted-form set holds no object
    navigation form.** QSpec's `quire.state.graph/v1` adds the object
    navigation forms (`deref`, `reaches`) and `quire.value.complete/v1` does
    not require it. FR-110 records a temporary exception by which the
    admitted-form set of `quire.value.complete/v1` also holds them. The
    exception is deleted as one change with the gate, which leaves:
    (a) the `ObjectNavigation` entry of the `ValueComplete` row of the layer
    table, with the comment that records the exception;
    (b) the FR-110 bullet that records the exception; and
    (c) the fixtures whose declaration writes `deref` or `reaches` against a
    `using` alias that resolves to `quire.value.complete/v1`, each of which
    moves to an alias that resolves to `quire.state.graph/v1` or
    `quire.model.complete/v1` (both admit the forms), whatever other profiles
    the unit's header selects.
    After the deletion a declaration that writes `deref` or `reaches` is
    admitted exactly when its `using` alias resolves to a layer whose
    `requires` closure names `quire.state.graph/v1`, and otherwise is refused
    by FR-110's layer rule naming that layer. No model-gate refusal (item 11)
    applies to the forms.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-361-AC-1 | A unit with the header profile `quire.value.complete/v1`, a `model` declaration with an operation and one `body` declaration for it refuses once, `unsupported_construct`/`declaration-form` at the body declaration's span, retaining the family `operation-body`, the owning declaration, the selected header layers and the required layer `quire.model.complete/v1`, and produces no package. The same unit with the header profile `quire.model.complete/v1` compiles and sets `has_body`. | Test (TC-1012) |
| FR-361-AC-2 | A value-profile unit that holds a `model` declaration and calls `receiver.operation(args)` refuses once, `unsupported_construct`/`expression-form` at the call's span with the family `dispatch-call`; one that holds a population binding and `lookup<T>(p, r) absent m` refuses once the same way with the family `lookup` at that expression's span. The same two units with the model profile compile: the call resolves to the receiver's effective operation and the lookup reads the population binding. | Test (TC-1013) |
| FR-361-AC-3 | A value-profile unit that holds a `model` declaration and an `abstraction-decl` refuses once, `unsupported_construct`/`declaration-form` at the declaration's span with the family `abstraction`; with the model profile it compiles to the same `CheckedAbstractionRelation` FR-304-AC-1 gives. A declaration with no binding refuses `invalid_syntax` before this check under either profile. | Test (TC-1014) |
| FR-361-AC-4 | A unit with the header profile `quire.value.complete/v1` that holds a `pre`, a `post` and an `invariant` against a model alias compiles. For `Population<T>`, `allInstances`, field navigation and a `hyper`, `relation` or state-graph clause, the same unit with the model profile admits the form, and with the value profile the refusal, if any, is FR-110's layer rule naming the selected layer and retains no family of item 5. | Test (TC-1015) |
| FR-361-AC-5 | A unit that holds a `model` declaration and `pre`, `post` and `invariant` clauses against its alias, and whose header profile is each of `quire.state.core/v1`, `quire.state.queries/v1`, `quire.state.graph/v1` and `quire.value.complete/v1`, compiles with no model-gate refusal. | Test (TC-1016) |
| FR-361-AC-6 | A value-profile unit holding a construct of every family of item 5 refuses once per construct, in source order, every refusal retaining the selected layers and the required layer; a construct inside a declaration nothing uses is refused too. A library that selects the model profile imported by a unit that does not, and the reverse, each give the lacking unit its refusals; the importing unit's own forms written against the library's import alias are refused. | Test (TC-1017) |
| FR-361-AC-7 | The selection is the header and nothing else: a unit with no header profile naming the model identity, a `model` declaration, a supplied domain package and a model construct refuses the construct and never compiles. | Test (TC-1017) |
| FR-361-AC-8 | A header profile naming `quire.model.complete/v1` resolves; its emitted lock's `definition_selections` hold the model layer, `quire.state.graph/v1`, `quire.value.complete/v1` and the layers they require, the complete-value lock's `package_selection` rows FR-110 selects for the union, the complete-model lock's `package_selection.always` rows, and no layer that requires the model layer. | Test (TC-1017) |
| FR-361-AC-9 | In a unit that also holds model constructs, a header profile naming a `qualification_catalog` row (for example the `ieee_profile` identity) refuses `unknown_profile`/`wrong-selection-role`, and one naming `test:unknown-profile` (held by no catalog) refuses `unknown_profile`/`unsupported-selection`; the E3 refusal is the only one reported. An identity a clause family's catalog holds routes to that catalog (FR-110, ADR-011) and is not covered here. | Test (TC-1017) |
| FR-361-AC-10 | Under `make conformance`, the rows of `DefinitionLock::pinned()` that come from the complete-model lock (role, authority and identity, in order, and its package-selection rules) equal QSpec's complete-model lock, read at run time. | Test (TC-1017) |
| FR-361-AC-11 | In a value-profile unit, S2 builds the forms of all four families and reports no refusal; the refusals of AC-1 to AC-3 are S3's. | Test (TC-1017) |
| FR-361-AC-12 | A `body ... using V` where `V` resolves to `quire.value.complete/v1` in a unit that selects only that profile receives one refusal (the family refusal of item 11), not two; the same declaration in a unit that also selects the model profile through another alias receives FR-110's layer refusal alone. | Test (TC-1017) |
| FR-361-AC-13 | A declaration that writes a `deref` and a declaration that writes a `reaches`, each with a `using` alias that resolves to `quire.value.complete/v1`, are each refused once by FR-110's layer rule naming that layer and retain no family of item 5; the same declarations with an alias that resolves to `quire.state.graph/v1` and with one that resolves to `quire.model.complete/v1` compile, including in a unit whose header also selects another profile. | Test (TC-1015) |

## Dependencies

- FR-110 states header profile resolution, the `header_selectable_layers`
  rows and the layer rule; the model layer is one of its five layers and this
  requirement adds the unit-level model test and the complete-model lock rows.
  FR-110's layer selection (FR-110-AC-1, AC-7, AC-8) is built; this
  requirement reads it (items 2 to 4).
- FR-102 and FR-302 record that the `operation-body` gate is not enforced;
  they cross-reference this requirement. FR-304 states the abstraction
  refusal this requirement places at S3. FR-308 and FR-309 name the protocol
  facet, which this requirement does not gate. FR-056 states the `model`
  declaration and domain-package intake, FR-103 the model operations.
- QSpec ADR-006, shared grammar (its facet grammar and model forms
  sections), `model-complete.md`, `complete-model-lock.json`,
  `native-diagnostics.md` (the `unsupported_construct` causes), FR-001,
  FR-271, FR-272, FR-450-AC-3 and FR-453.

## References

- Owning ticket: QSL-750.

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
---
# FR-361: Gate every model construct on the unit's selection of the model profile

## Description

When a unit's header selects `quire.model.complete/v1`, QSL SHALL admit the
model constructs this requirement lists. When a unit's header selects no
header profile that resolves to `quire.model.complete/v1`, QSL SHALL refuse
each such construct with `unsupported_construct`, at the construct, and SHALL
produce no checked package.

QSpec's shared grammar states the rule for the `operation-body`, the
dispatched operation call and `lookup` form, and the `abstraction-decl`; QSL
FR-304 applies it to the abstraction form. Until this
requirement is built, QSL accepts every one of them under a package that
selects only `quire.value.complete/v1`, and FR-102 and FR-302 record that
the gate is not enforced.

A **model construct** is a syntactic form listed in "Model construct
families" below. The unit **selects the model profile** when at least one of
its header `profile` declarations names the identity of the
`quire.model.complete/v1` row of the DefinitionLock catalog's
`header_selectable_layers` (FR-110).

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
   model profile adds no second header-profile mechanism and no new
   refusal code. A header identity that names no `header_selectable_layers`
   row keeps FR-110's refusals: `unknown_profile`/`unsupported-selection`
   when it names no catalog row, and `unknown_profile`/`wrong-selection-role`
   when it names a `qualification_catalog` row only. The five rows of
   `header_selectable_layers` (`quire.state.core/v1`, `quire.state.queries/v1`,
   `quire.state.graph/v1`, `quire.value.complete/v1` and
   `quire.model.complete/v1`) are header-selectable (FR-110, QSpec FR-453), so
   `unknown_profile` refuses every identity outside those five.
3. **The catalog holds the model lock.** `DefinitionLock::pinned()` SHALL hold
   the rows of QSpec's complete-model lock beside the complete-value lock's
   (the complete-model lock requires the complete-value lock). A unit whose
   header profiles resolve to the model layer SHALL emit, in
   `definition_selections`, the layer, every layer its `requires` closure
   names (FR-110) and the complete-model lock's `package_selection.always`
   rows.
4. **Closure.** Selecting the model layer admits the forms of the layers its
   `requires` closure names (FR-110 "Layer selection"). A unit that selects
   the model profile therefore needs no second header profile to use
   value-complete forms.

### Model construct families

5. QSL SHALL treat these as the model constructs, each with its refusal
   cause (QSpec shared grammar names the first, the third, the fifth and the
   sixth; the `model` declaration is QSL's rule, item 5a):

   | Family | Construct | Cause |
   | --- | --- | --- |
   | `operation-body` | the `body <label> using <alias> on <model alias>::...` declaration | `declaration-form` |
   | `model-import` | the header-position `model <alias> = "<identity>" digest "<hex>";` declaration | `declaration-form` |
   | `abstraction` | the `abstraction-decl` declaration | `declaration-form` |
   | `dispatch-call` | a postfix `receiver.member(args)` operation call | `expression-form` |
   | `lookup` | the `lookup<T>(p, r) absent m` expression | `expression-form` |

5a. **The `model` declaration is gated by QSL's own rule.** QSpec states no
    gate for the `model` declaration. QSL gates it (family `model-import`) so
    that a value-only unit cannot reach a model operation, and a QSpec row
    stating the gate is requested. Until QSpec states it, the rule is QSL's.
6. **Forms QSpec states no model gate for stay ungated.** These are not model
   constructs: the state-clause family (`invariant`, `pre`, `post`, and
   `pre(e)`, `result`, `self`, `reaches`), because state core admits
   preconditions, postconditions and invariants (FR-110 "Layer selection"); the
   `Population<T>` type, `allInstances` and field or relationship navigation;
   and the `hyper`, `relation` and state-graph clauses. A clause whose target
   names a model operation reaches that operation only through a `model`
   declaration, which family `model-import` refuses (item 5a).
6a. **Protocol, memory and temporal forms are not gated by this requirement.**
    QSpec's shared grammar ("Complete-V1 facet grammar") names
    `quire.protocol.complete/v1` as the facet of the protocol forms, including
    the memory clause, memory order and fence (ADR-006 makes it the one
    protocol admission profile), and `quire.temporal.complete/v1` as the
    temporal facet. Neither is a model construct, and QSL's
    `header_selectable_layers` has no row for either. Not gated: QSpec names
    `quire.protocol.complete/v1` and `quire.temporal.complete/v1`; QSL has no
    such layer. QSL adds no layer and no gate for them here; FR-308 and FR-309
    name the protocol facet accordingly.
6b. **Why the test is on the unit.** QSpec's shared grammar says a recognised
    construct whose required facet is absent receives a located
    `unsupported_construct` for a package, and FR-001 refuses a form with
    `unsupported_construct` naming the unit's selected layers. FR-110's
    per-alias layer rule alone would leave a construct with no `using` alias
    (for example `abstraction-decl`) ungated, so the unit-level test is
    needed; FR-110 and FR-453 state no unit-level model rule themselves, so
    the unit-level reading of the model gate for the dispatched call, `lookup`,
    `operation-body` and `abstraction-decl` is QSpec's package-level wording,
    and its application to the `model-import` family is QSL's (item 5a).
7. A form that the DefinitionLock's admitted-form set of a layer does not
   name stays refused by FR-110's layer rule. This requirement adds the
   unit-level selection test; it replaces no layer rule.

### Stage and origin

8. **E3 selects and S2 builds.** E3 resolves the header selection (FR-110).
   S2 builds each construct's form from the CST whether or not the unit
   selects the model profile. S2 does not resolve the profile and does not
   refuse a model construct (FR-102). The S2 sentences of FR-304 and FR-308 that
   have S2 refuse an unselected form state the S3 rule of this requirement.
9. **S3 refuses.** S3 SHALL refuse each model construct in a unit that does
   not select the model profile, in source order, and SHALL report every
   such construct, not the first. Each refusal is `unsupported_construct` with
   the cause of item 5. Its origin is the construct's source span: the
   declaration's span for a declaration form, the expression's span for an
   expression form. The refusal retains the recognised form's family (item 5),
   the owning declaration, the header layers the unit selected in
   source order, and the required layer `quire.model.complete/v1`. The
   refusal is S3's, so a unit with an E3 refusal (FR-110) reports that one and
   produces no S3 refusal.
10. **Presence in unreachable syntax counts.** A model construct in a
    declaration nothing uses is refused the same way (QSpec
    `native-diagnostics`: presence in unreachable syntax does not erase the
    refusal).
11. **Per unit.** The test applies to the unit being compiled and, separately,
    to each library the S4 source resolution compiles (FR-110 "Placement"). A
    library that selects the model profile does not give its importer the
    selection, and an importer's selection does not give its library the
    selection.
12. **No produced package.** A unit with a model-construct refusal produces
    no checked package, no lock and no bytes (ADR-011 §2.3, S3 row).

### Admission under the selection

13. A unit that selects the model profile SHALL admit each family of item 5
    exactly as FR-056, FR-102, FR-103, FR-302 and FR-304
    state. This requirement changes no behaviour of a selecting unit.
14. Every fixture that exercises a model construct SHALL select
    `quire.model.complete/v1`, and every value-only fixture keeps
    `quire.value.complete/v1` and gains a refusal vector for each family of
    item 5.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-361-AC-1 | A unit with the header profile `quire.value.complete/v1` and one `body` declaration refuses `unsupported_construct`/`declaration-form` at the declaration's span, retaining the family `operation-body`, the owning declaration, the selected header layers and the required layer `quire.model.complete/v1`, and produces no package; the model declaration the body targets is refused too, family `model-import`. The same unit with the header profile `quire.model.complete/v1` compiles and sets `has_body`. | Test (TC-1012) |
| FR-361-AC-2 | A value-profile unit that calls `receiver.operation(args)` refuses `unsupported_construct`/`expression-form` at the call's span with the family `dispatch-call`; one that holds `lookup<T>(p, r) absent m` refuses the same way with the family `lookup` at that expression's span. The same two units with the model profile pass the refusal check. | Test (TC-1013) |
| FR-361-AC-3 | A value-profile unit with an `abstraction-decl` refuses `unsupported_construct`/`declaration-form` at the declaration's span with the family `abstraction`; with the model profile it compiles to the same `CheckedAbstractionRelation` FR-304-AC-1 gives. A declaration with no binding refuses `invalid_syntax` before this check under either profile. | Test (TC-1014) |
| FR-361-AC-4 | A value-profile unit holding a protocol form (FR-308), a memory clause, ordering or fence (FR-309), a `hyper`, `relation` or state-graph clause, a `Population<T>` type, `allInstances`, field navigation or a state clause raises no `unsupported_construct` from this requirement; no gate is invented for a facet QSL has no layer for. | Test (TC-1015) |
| FR-361-AC-5 | A value-profile unit holding a `model` declaration refuses `unsupported_construct`/`declaration-form` at the declaration's span with the family `model-import`, and produces no package, even when a domain package is supplied for it and even when the unit holds no other model construct. A value-profile unit holding a `pre` or `post` clause over only value-complete operands compiles. | Test (TC-1016) |
| FR-361-AC-6 | A value-profile unit holding a model construct of every family refuses once per construct, in source order, every refusal retaining the selected layers and the required layer; a construct inside a declaration nothing uses is refused too. A library that selects the model profile imported by a unit that does not, and the reverse, each give the lacking unit its refusals. | Test (TC-1017) |
| FR-361-AC-7 | The selection is the header and nothing else: a unit with no header profile naming the model identity and a `model` declaration, a supplied domain package and a model construct refuses as AC-1 to AC-5 state and never compiles; a unit naming `quire.value.complete/v1` as the only header profile never gets the model constructs admitted. | Test (TC-1017) |
| FR-361-AC-8 | A header profile naming `quire.model.complete/v1` resolves; its emitted lock's `definition_selections` hold the model layer, `quire.state.graph/v1`, `quire.value.complete/v1` and the layers they require, and the complete-model lock's `package_selection.always` rows, and no layer that requires the model layer. | Test (TC-1017) |
| FR-361-AC-9 | A header profile naming an identity outside the five header-selectable layers refuses: a `qualification_catalog` row (for example the `ieee_profile` identity) `unknown_profile`/`wrong-selection-role`, `quire.protocol.complete/v1` and `test:unknown-profile` `unknown_profile`/`unsupported-selection`, in a unit that also holds model constructs; the E3 refusal is the only one reported. | Test (TC-1017) |
| FR-361-AC-10 | Under `make conformance`, the rows of `DefinitionLock::pinned()` that come from the complete-model lock (role, authority and identity, in order, and its package-selection rules) equal QSpec's complete-model lock, read at run time. | Test (TC-1017) |

## Dependencies

- FR-110 states header profile resolution, the `header_selectable_layers`
  rows and the layer rule; the model layer is one of its five layers and
  this requirement adds the unit-level model test and the complete-model lock
  rows. FR-110-AC-7 and FR-110-AC-8 (layer selection) are the mechanism this
  requirement's item 2 and item 4 use.
- FR-102 and FR-302 record that the `operation-body` gate is not enforced;
  they cross-reference this requirement. FR-304 and FR-308 state the
  abstraction and protocol refusals this requirement places at S3. FR-309
  states the memory forms, whose facet is the protocol facet. FR-056 states the `model` declaration and
  domain-package intake.
- QSpec ADR-006, shared grammar ("Complete-V1 facet grammar" and the model forms
  sections), `model-complete.md`, `complete-model-lock.json`,
  `native-diagnostics.md` (the `unsupported_construct` causes), FR-001,
  FR-450-AC-3 and FR-453.

## References

- Owning ticket: QSL-750.

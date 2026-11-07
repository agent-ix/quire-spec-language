---
id: SR-1718001
title: "spec-review of FR-533 observation admission (QSL-1718)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; commit 58d7c903b; spec/functional/FR-533-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md (Observation admission, FR-533-AC-5, AC-6, AC-22, Dependencies); spec/test-cases/TC-1333-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md rows 5, 6, 22; spec/functional/FR-531-evaluate-from-a-checked-package-read-from-its-bytes-alone.md (two table rows)"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-533
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-531
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1333
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1070
    type: references
---

# Spec review of FR-533 observation admission (QSL-1718)

## Summary

Base checklist review of commit 58d7c903b against its parent, with each claim checked
against the code. No AssuranceProfile is applicable to this spec-only change.
The source route is `qsl-package/src/evaluation_view.rs`
(`AdmittedEvaluationView::admit_protocol_observation`), `qsl-eval/src/value/expression/slot_supply.rs`
(`CheckedHost`, `admit_slot_supply`), `qsl-semantics/src/check/mod.rs` (`model_selections`,
`protocol_population_types`), `qsl-semantics/src/model/key.rs` (`population_node_key`) and
`qsl-semantics/src/check/assemble.rs` (the object declarations of `scope().types()`). The bare
route is `qsl-package/src/bare.rs` and `qsl-eval/src/value/expression/bare.rs`. The Contract IR
code is `quire-contract-model` `checked_package/v2` at checkout 2312df5 (`mod.rs`,
`model_population_catalog.rs`, `runtime_type_projection.rs`).

These claims hold. The source route checks the model and then calls `admit_current_snapshot`
over `[model]`, as step 1 and step 4 say. `DomainPackageRef` is the identity alone. Both
selection lists are in ascending identity order: the source collects them from a
`BTreeMap` by identity, and IR documents the same order. The catalog's roots,
populations and members are in ascending order, and objects come in structural identity
order. The key recipe matches the source: `population_node_key(key)` =
`model_node_key(key.package, "relation", "population", key.node)`, over the
population record's `DeclarationKey`. The whole-projection refusals hold: IR refuses a
document's `variant_types` and non-empty `units` with `UnavailableIdentityWitness`, and it
skips the systems interface, state machines and processes in the completeness check.
TC-1070 rows 34 to 46 are a good fit: row 35 evaluates to 5, row 40's
`N::externalOthers` fails the population lookup (`wrong-model-selection`), and row 41's
`elsewhereNodes` fails `conforms` (`wrong-value-kind`). The fixture already exists in
`qsl-semantics/tests/support/tc1070.rs`. `missing-type-representation` and the other catalog
pairs are already in use. AC ids do not collide, AC-22 is new, TC-1333's scope and trace tag
are updated, and FR-533-AC-13's exact parity is not touched.

Defects follow. Two parity claims are false against the source: the declaration name
(FND-001) and the IR whole-projection refusals, which are wider than stated (FND-002,
FND-003). The type of the bare observation is unstated (FND-004). AC-6's first vector
passes through a check other than the one it is meant to test (FND-005). AC-5 has no
guard against reading a source package (FND-006). The catalog's limit diagnostic cannot
be built from the budget the spec names (FND-007).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The type-environment row names each bare `ObjectTypeDeclaration` by `original_identity()` (an `ix://<package>/<artifact>` identity). The source route names it `format!("{alias}::{artifact}")` (assemble.rs, object declarations). The kernel reports a type under its declared name: `observation/extents.rs` `type_name` puts `declared.name()` into extent refusal records. So step 4's claim that `admit_current_snapshot`'s `Refused` results "are returned as the source route returns them" is false for any record that names an object type. AC-5(a)'s "equal records" holds only because no E-P observation refusal names a type, and AC-5(c) compares `attributes` and `conforms` but not names. IR carries no model alias (`selected_model_packages` doc), so the source name cannot always be rebuilt. Fix: state the name the bare route uses and list the name difference in admission records as unmet parity (as "Names and locations" does for locations). Alternatively, give a rule that rebuilds `<identity>::<artifact>` when the selection has no alias. Add an AC-5 vector that compares the name, or the record of a type-naming refusal. | FR-533 "Observation admission" type-environment row and step 4; FR-533-AC-5(a),(c) | wrong-requirement |
| FND-002 | medium | The unmet-dependency paragraph says the whole projection refuses only "a selected document that declares a variant (enum) type or a model-owned unit". With `reached = None`, `project_type_declarations_for_nodes` also refuses any checked-graph node of form `ScalarType(Enum)` or `ScalarType(CompoundUnit)`, whoever owns it, and any model-owned `Unit` node (`UnavailableIdentityWitness`, first loop of runtime_type_projection.rs). A package with a compound-unit node or an enum node anywhere in its graph therefore refuses at step 3 when the source route would admit it. Fix: list the graph-node refusals (compound units, enum nodes of any owner) in the unmet-dependency paragraph and in Dependencies, or state that such packages are out of scope. | FR-533 "Observation admission" last paragraph; Dependencies "Observation admission" | wrong-requirement |
| FND-003 | medium | The rule calls `project_selected_type_declarations(identities, [], source_unit_rows, budget)` with no tuple rows. A selected document with a tuple type then refuses `UnavailableIdentityWitness` at `tuple_value_type`, because `tuple_by_node` has no row for it. The source route declares that document's types. The handoff (`ReadIdentityRows`: declarations, fields, source records, source unions) carries no `SelectedTupleDeclarationRow`. This gap is QSL's own, not an IR dependency, and the spec does not list it. Fix: carry the checked graph's `selected_tuple_declaration_rows` in the identity handoff and pass them, or list model tuple types as an unmet parity owned by QSL. | FR-533 "Observation admission" type-environment row; Dependencies | missing-requirement |
| FND-004 | medium | The section says the bare admission returns "`ObservationAdmission` of an admitted protocol observation" that "exposes no `CheckedPackage` and no `AdmittedEvaluationView`". `ProtocolSupply::observation` is typed `ObservationAdmission<AdmittedProtocolObservation<'a>>`, but `AdmittedProtocolObservation` holds `package: &CheckedPackage` and exposes `evaluation_view()`. `SlotHost::observation_is_for_view` also takes that type. The spec does not name the bare observation's type, or how one `ProtocolSupply` carries either route's observation. AC-6 depends on this: it supplies a bare observation to a bare call and states the source-view case. Fix: name the type, for example one `AdmittedProtocolObservation` whose owner is a source view or a `BarePackage` identity token, and say how `ProtocolSupply` and the binding check read it. | FR-533 "Observation admission" first paragraph; "Supply admission" paragraph; FR-533-AC-6 | missing-requirement |
| FND-005 | medium | In AC-6's first vector, row 35's value and metadata are supplied to the second package's slot call. That metadata carries E-P's checked population node keys and `Other` references of E-P's model identity. `admit_slot_supply` refuses at `host.population_types(&metadata.population)` (None, `wrong-model-selection`) before it reaches `holds_model(observation.model_selection())` or `observation_is_for_view`. The vector would pass if the bare host accepted any observation, so it does not test what it claims, that the observation was admitted by the first package. Fix: give the second package's own population keys and model identity in the value and metadata, so that only the observation comes from the first package and the refusal comes from `holds_model` or the binding check. | FR-533-AC-6; TC-1333 row 6; slot_supply.rs population-type check | correct-requirement-no-evidence |
| FND-006 | medium | AC-5 asserts that the two routes give equal admission results, and AC-4 is meant to rule out a source package, but its call-graph search starts only at `evaluate_protocol_slot`. `BarePackage::admit_protocol_observation` and the reads `with_supply` makes for the catalog and the declarations are outside it. An implementation that admits through a `CheckedPackage` held beside it in the test, or through its `scope().types()`, would pass AC-5. Fix: add `BarePackage::admit_protocol_observation` and `with_supply` to AC-4's search roots, or extend AC-3's replacement vector to admission. | FR-533-AC-4; FR-533-AC-5 | correct-requirement-no-evidence |
| FND-007 | medium | The catalog's allowances are said to be "the budget's unconsumed work and storage", and its exhaustion is said to give `NativeJoinFailure::PopulationCatalog` "with the stage limit diagnostic". `selected_model_population_catalog` takes its own `CheckedModelPopulationCatalogLimits` and charges nothing to the `NativeJoinBudget`. `NativeJoinFailure::with_budget`, through `NativeJoinLimits::refusal`, builds the diagnostic only from `budget.first_unpaid()`, which the catalog never sets, so the stated diagnostic has no source. The spec also does not say whether the catalog's charges reduce the budget afterwards, or which bound and attempted count the diagnostic reports: the remaining allowance, or the configured `i2.*` ceiling. `entries` counts descriptors, while storage is in storage units. Fix: say that the diagnostic's bound and attempted count come from the catalog error's `limit` and `consumed` (plus the budget's prior consumption, if the configured ceiling is reported), and say whether the catalog's work and entries are charged back to the budget. | FR-533 "Observation admission" budget paragraph; FR-533-AC-22 second sentence; TC-1333 row 22 | missing-requirement |
| FND-008 | low | The row says attributes are "the object's `declared_fields()` in their selected order". IR documents `declared_fields()` as "this declaration's own fields in authored order" and `fields()` as "every effective field in selected declaration order", so the phrase mixes the two accessors. The source attributes are an object's own `FieldMember` records in the producer's record order, with Clock-typed fields left out, and carry `with_redefines`. The rule's `FieldDeclaration(name, value_type, presence)` drops `redefines` and does not say what happens to a Clock field. Fix: name the accessor and the order exactly. State that Clock fields are excluded and redefinition links kept, as on the source route, or list either as unmet parity. | FR-533 "Observation admission" type-environment row | wrong-requirement |
| FND-009 | low | The catalog row says "every population of the selected documents is listed". The source table (`protocol_population_types` from `named_populations` over `population_domains`) drops a population that has no member resolving to a declared type or no `ix://` name segment, and keeps only the members it can resolve, without alias normalization. IR instead refuses the whole catalog (`UnresolvedMember` `MissingType`/`WrongKind`/`AliasCycle`), and it alias-normalizes members. For a population whose member is an alias or does not resolve, the two routes answer differently (Some or step 3 against None), but the "population types" fact is stated as equal. Fix: state these cases as unmet parity, or limit the claim to populations whose members all name declared object types with no aliases. | FR-533 "Observation admission" population-types row; FR-533-AC-5(c) | wrong-requirement |
| FND-010 | low | Two edge outcomes are unstated. (a) The text says a retained projection or catalog refusal "affects no slot call whose supply names no object reference", but not what a slot call whose supply does name one gets: step 3's `missing-type-representation` record at `Package`, or a `Binder` refusal from supply admission. (b) Step 4 says a snapshot object whose type the bare environment does not declare "refuses as step 3 states", but not how the bare route tells that case apart from `admit_current_snapshot`'s other refusals, for example by checking snapshot object types first. Also, the "four questions" undercounts `SlotHost`, which has five methods; `binder_level` is not mentioned. Fix: give the record and locus for (a), the detection rule for (b), and name `binder_level`'s bare source. | FR-533 "Observation admission" budget paragraph, step 4, "Supply admission" paragraph | missing-requirement |

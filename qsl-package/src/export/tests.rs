// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-797 to TC-800, TC-802, TC-803, TC-806 and TC-807: the S3 check of an
//! abstraction relation (FR-304, FR-305) and its per-item export (FR-307),
//! over relations built in the test from checked packages.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::Code;
use qsl_semantics::check::{
    AbstractionKey, AbstractionRefusal, AuthoredBinding, Capability, CheckedBinding, FrameBinding,
    MalformedEntry, ObjectBinding, OperationKey, PackageDeclarations, PopulationBinding, RustField,
    RustParameter, RustPath, RustReceiver,
};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::Identifier;
use quire_semantic_value::checking::CheckingLimits;

use super::{export_bindings, BoundItem, ExportRefusal};
use crate::CheckedPackage;

#[path = "../../../qsl-semantics/tests/support/abstraction.rs"]
mod fixture;

/// The unit's clauses: a function reading `ConfigVersion` fields, an
/// invariant over `ConfigVersion` (so over its population), a
/// postcondition of `attemptUpdate`, and an invariant reading `Base`'s
/// field through a `Sub` receiver.
const CLAUSES: &str = "\
function bump using v(c: M::ConfigVersion): Integer pure { deref(c).version + 1 }\n\
invariant Versioned using v on M::ConfigVersion at current { self.version >= 0 }\n\
post Updated using v on M::ConfigVersion::attemptUpdate { true }\n\
invariant ThroughSub using v on M::Sub at current { self.f >= 0 }\n";

/// The unit with `clauses` against the fixture package, checked and linked.
fn package(clauses: &str) -> CheckedPackage {
    let document = serde_json::to_vec(&fixture::document()).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model M = {:?} version \"1\" digest \"sha256-jcs:{}\";\n{clauses}",
        fixture::PACKAGE,
        qsl_semantics::model::key::hex(digest),
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:abstraction", "fixture", "fixture:1"),
        "program.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), []);
    let raw = parsed.source().reference().clone();
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &built.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the package admits");
    let graph = PackageDeclarations::assemble(raw, built, models, Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    CheckedPackage::link(graph)
}

fn declaration(suffix: &str) -> DeclarationKey {
    DeclarationKey {
        package: fixture::PACKAGE.to_owned(),
        node: fixture::id(suffix),
    }
}

fn name(text: &str) -> Identifier {
    Identifier::new(text).expect("an identifier")
}

fn object(owner: &str, rust_type: &str, fields: &[(&str, &str)]) -> AuthoredBinding {
    AuthoredBinding::Object {
        key: declaration(owner),
        value: ObjectBinding {
            rust_type: RustPath::new(rust_type),
            fields: fields
                .iter()
                .map(|(field, rust)| (name(field), RustField::new(*rust)))
                .collect(),
        },
    }
}

fn population() -> AuthoredBinding {
    AuthoredBinding::Population {
        key: declaration("configs"),
        value: PopulationBinding {
            collection: RustPath::new("state::configs"),
        },
    }
}

fn frame(owner: &str, operation: &str, parameters: &[(&str, &str)]) -> AuthoredBinding {
    AuthoredBinding::Frame {
        key: OperationKey {
            declaring: declaration(owner),
            operation: name(operation),
        },
        value: FrameBinding {
            function: RustPath::new("update"),
            receiver: RustReceiver::SelfValue,
            parameters: parameters
                .iter()
                .map(|(parameter, rust)| (name(parameter), RustParameter::new(*rust)))
                .collect::<BTreeMap<_, _>>(),
        },
    }
}

fn attempt_update() -> AuthoredBinding {
    frame(
        "ConfigVersion",
        "attemptUpdate",
        &[("n", "count"), ("m", "limit")],
    )
}

fn config_object() -> AuthoredBinding {
    object(
        "ConfigVersion",
        "ConfigVersion",
        &[("version", "version"), ("note", "note")],
    )
}

fn refused(package: &CheckedPackage, bindings: &[AuthoredBinding]) -> Vec<AbstractionRefusal> {
    package
        .graph()
        .check_abstraction(bindings)
        .expect_err("the relation refuses")
}

fn sole(refusals: Vec<AbstractionRefusal>) -> AbstractionRefusal {
    let [refusal] = <[AbstractionRefusal; 1]>::try_from(refusals).expect("exactly one refusal");
    refusal
}

fn wire(key: quire_exact::NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*key.as_bytes())
}

fn clause_item(package: &CheckedPackage, clause: &str) -> OccurrenceKey {
    let clause = package.graph().state_clause(clause).expect("the clause");
    OccurrenceKey::new(wire(clause.identity()), clause.claim().clone())
}

fn frame_item(package: &CheckedPackage) -> OccurrenceKey {
    let graph = package.graph();
    let selection = graph
        .resolve_operation(&name("M"), &name("ConfigVersion"), &name("attemptUpdate"))
        .expect("the operation resolves");
    let (_, frame) = graph
        .operation_frame(&selection)
        .expect("the unit has the frame");
    OccurrenceKey::new(wire(frame.frame()), frame.frame_origin().clone())
}

fn function_item(package: &CheckedPackage) -> OccurrenceKey {
    let mut items = package
        .graph()
        .requirements()
        .iter()
        .filter(|(_, record)| record.requirements().kind() == Capability::ValueValidity)
        .map(|(key, _)| key.clone());
    let (Some(item), None) = (items.next(), items.next()) else {
        panic!("one value-validity record");
    };
    item
}

fn config_key() -> AbstractionKey {
    AbstractionKey::Object(declaration("ConfigVersion"))
}

fn population_key() -> AbstractionKey {
    AbstractionKey::Population(declaration("configs"))
}

fn operation_key() -> AbstractionKey {
    AbstractionKey::Frame(OperationKey {
        declaring: declaration("ConfigVersion"),
        operation: name("attemptUpdate"),
    })
}

#[test]
#[trace("TC-797", "FR-304-AC-1")]
fn a_relation_checks_into_one_binding_per_key() {
    let package = package(CLAUSES);
    let relation = package
        .graph()
        .check_abstraction(&[config_object(), population(), attempt_update()])
        .expect("the relation checks");
    assert_eq!(relation.len(), 3);
    let keys: Vec<&AbstractionKey> = relation.iter().map(|(key, _)| key).collect();
    assert_eq!(keys, [&config_key(), &population_key(), &operation_key()]);
    let Some(CheckedBinding::Object(bound)) = relation.get(&config_key()) else {
        panic!("the object type is bound");
    };
    assert_eq!(bound.rust_type, RustPath::new("ConfigVersion"));
    assert_eq!(bound.fields.len(), 2);
    assert!(matches!(
        relation.get(&population_key()),
        Some(CheckedBinding::Population(PopulationBinding { collection }))
            if collection == &RustPath::new("state::configs")
    ));
}

#[test]
#[trace("TC-797", "FR-304-AC-2")]
fn a_partial_relation_checks() {
    let package = package(CLAUSES);
    let relation = package
        .graph()
        .check_abstraction(&[config_object()])
        .expect("the partial relation checks");
    assert_eq!(relation.len(), 1);
    assert!(relation.get(&population_key()).is_none());
}

#[test]
#[trace("TC-798", "FR-304-AC-3")]
fn a_key_that_resolves_to_nothing_refuses_missing_name() {
    let package = package(CLAUSES);
    for binding in [
        object("Missing", "Missing", &[]),
        frame("ConfigVersion", "rename", &[]),
    ] {
        let refusal = sole(refused(&package, std::slice::from_ref(&binding)));
        assert_eq!(refusal.code(), Code::MissingDeclaration);
        assert_eq!(refusal.cause(), "missing-name");
        assert_eq!(
            refusal,
            AbstractionRefusal::MissingDeclaration {
                key: binding.key(),
                package: fixture::PACKAGE.to_owned(),
            }
        );
    }
}

#[test]
#[trace("TC-799", "FR-304-AC-4")]
fn two_bindings_for_one_key_refuse_conflicting_binding() {
    let package = package(CLAUSES);
    let equal = config_object();
    let different = object("ConfigVersion", "Other", &[]);
    for (first, second) in [
        (equal.clone(), equal.clone()),
        (equal.clone(), different.clone()),
    ] {
        let refusal = sole(refused(&package, &[first.clone(), second.clone()]));
        assert_eq!(refusal.code(), Code::InvalidModelBinding);
        assert_eq!(refusal.cause(), "conflicting-binding");
        assert_eq!(
            refusal,
            AbstractionRefusal::ConflictingBinding {
                key: config_key(),
                first: Box::new(first),
                second: Box::new(second),
            }
        );
    }
}

#[test]
#[trace("TC-800", "FR-304-AC-5")]
fn malformed_parameter_and_field_maps_refuse_malformed_declaration() {
    let package = package(CLAUSES);
    let cases = [
        (
            frame("ConfigVersion", "attemptUpdate", &[("n", "count")]),
            MalformedEntry::OmittedParameter(name("m")),
        ),
        (
            frame(
                "ConfigVersion",
                "attemptUpdate",
                &[("n", "count"), ("m", "limit"), ("z", "extra")],
            ),
            MalformedEntry::UnknownParameter(name("z")),
        ),
        (
            frame(
                "ConfigVersion",
                "attemptUpdate",
                &[("n", "same"), ("m", "same")],
            ),
            MalformedEntry::SharedRustParameter {
                first: name("m"),
                second: name("n"),
                rust: RustParameter::new("same"),
            },
        ),
        (
            object("ConfigVersion", "ConfigVersion", &[("missing", "x")]),
            MalformedEntry::UnknownField(name("missing")),
        ),
    ];
    for (binding, entry) in cases {
        let refusal = sole(refused(&package, std::slice::from_ref(&binding)));
        assert_eq!(refusal.code(), Code::InvalidModelBinding);
        assert_eq!(refusal.cause(), "malformed-declaration");
        assert_eq!(
            refusal,
            AbstractionRefusal::MalformedDeclaration {
                key: binding.key(),
                entry,
            }
        );
    }
    let inherited = object("Sub", "Sub", &[("f", "f")]);
    let relation = package
        .graph()
        .check_abstraction(&[inherited])
        .expect("a subtype binds a field its supertype declares");
    let Some(CheckedBinding::Object(bound)) =
        relation.get(&AbstractionKey::Object(declaration("Sub")))
    else {
        panic!("Sub is bound");
    };
    assert!(bound.fields.contains_key(&name("f")));
}

#[test]
#[trace("TC-802", "FR-305-AC-1", "FR-305-AC-2")]
fn a_frame_binding_relates_to_its_frame_and_anchor_when_a_clause_names_it() {
    let without = package("invariant Versioned using v on M::ConfigVersion at current { true }\n");
    let relation = without
        .graph()
        .check_abstraction(&[attempt_update()])
        .expect("a frame no clause names still checks");
    let Some(CheckedBinding::Frame(unnamed)) = relation.get(&operation_key()) else {
        panic!("the frame is bound");
    };
    assert_eq!(unnamed.frame_nodes(), None);

    let with = package(CLAUSES);
    let relation = with
        .graph()
        .check_abstraction(&[attempt_update()])
        .expect("the relation checks");
    let Some(CheckedBinding::Frame(named)) = relation.get(&operation_key()) else {
        panic!("the frame is bound");
    };
    let selection = with
        .graph()
        .resolve_operation(&name("M"), &name("ConfigVersion"), &name("attemptUpdate"))
        .expect("the operation resolves");
    let (_, checked) = with.graph().operation_frame(&selection).expect("the frame");
    let nodes = named.frame_nodes().expect("a clause names the operation");
    assert_eq!(nodes.frame(), checked.frame());
    assert_eq!(nodes.anchor(), checked.anchor());
}

#[test]
#[trace("TC-803", "FR-305-AC-3")]
fn an_inherited_operations_frame_binds_only_at_its_declaring_type() {
    let package = package(CLAUSES);
    let at_sub = frame("Sub", "op", &[]);
    let refusal = sole(refused(&package, std::slice::from_ref(&at_sub)));
    assert_eq!(
        refusal,
        AbstractionRefusal::MissingDeclaration {
            key: at_sub.key(),
            package: fixture::PACKAGE.to_owned(),
        }
    );
    package
        .graph()
        .check_abstraction(&[frame("Base", "op", &[])])
        .expect("the declaring type binds the operation");
}

/// The package with `bindings` bound.
fn bound(bindings: &[AuthoredBinding]) -> CheckedPackage {
    let mut package = package(CLAUSES);
    package
        .bind_abstraction(bindings)
        .expect("the relation checks");
    package
}

fn unbound_keys(refusal: &ExportRefusal) -> Vec<(&AbstractionKey, &str)> {
    let ExportRefusal::Unbound { elements, .. } = refusal else {
        panic!("{refusal:?}: expected an unbound refusal");
    };
    elements
        .iter()
        .map(|element| (&element.key, element.package.as_str()))
        .collect()
}

#[test]
#[trace("TC-806", "FR-307-AC-1", "FR-307-AC-2", "FR-307-AC-3")]
fn the_export_refuses_each_item_with_an_unbound_element_and_returns_the_others() {
    let package = bound(&[config_object()]);
    let reads_fields = function_item(&package);
    let over_population = clause_item(&package, "Versioned");
    let results = export_bindings(&package, &[reads_fields.clone(), over_population.clone()]);
    let [Ok(first), Err(second)] = &results[..] else {
        panic!("{results:?}: expected one bound and one refused item");
    };
    assert_eq!(first.item, reads_fields);
    assert_eq!(first.bindings.len(), 1);
    assert_eq!(first.bindings[0].0, config_key());
    assert!(matches!(first.bindings[0].1, CheckedBinding::Object(_)));
    assert_eq!(second.code(), Code::MissingDeclaration);
    assert_eq!(
        unbound_keys(second),
        [(&population_key(), fixture::PACKAGE)]
    );

    let two = export_bindings(&package, &[clause_item(&package, "Updated")]);
    let [Err(refusal)] = &two[..] else {
        panic!("{two:?}: expected one refusal");
    };
    assert_eq!(
        unbound_keys(refusal),
        [
            (&population_key(), fixture::PACKAGE),
            (&operation_key(), fixture::PACKAGE)
        ]
    );

    let unknown = OccurrenceKey::new(
        WireNodeId::from_digest([7; 32]),
        package
            .graph()
            .state_clause("Versioned")
            .expect("the clause")
            .claim()
            .clone(),
    );
    let mixed = export_bindings(&package, &[reads_fields, over_population, unknown.clone()]);
    assert_eq!(mixed[..2], results[..]);
    assert_eq!(mixed[2], Err(ExportRefusal::UnknownItem { item: unknown }));
}

#[test]
#[trace("TC-807", "FR-307-AC-4")]
fn the_export_references_the_receivers_static_type() {
    let item = |package: &CheckedPackage| clause_item(package, "ThroughSub");
    let base_only = bound(&[object("Base", "Base", &[("f", "f")])]);
    let results = export_bindings(&base_only, &[item(&base_only)]);
    let [Err(refusal)] = &results[..] else {
        panic!("{results:?}: expected one refusal");
    };
    assert_eq!(
        unbound_keys(refusal),
        [(
            &AbstractionKey::Object(declaration("Sub")),
            fixture::PACKAGE
        )]
    );

    let with_sub = bound(&[object("Sub", "Sub", &[("f", "f")])]);
    let results = export_bindings(&with_sub, &[item(&with_sub)]);
    let [Ok(exported)] = &results[..] else {
        panic!("{results:?}: expected one bound item");
    };
    assert_eq!(
        exported
            .bindings
            .iter()
            .map(|(key, _)| key)
            .collect::<Vec<_>>(),
        [&AbstractionKey::Object(declaration("Sub"))]
    );
}

#[test]
#[trace("TC-807", "FR-307-AC-5")]
fn frame_and_postcondition_items_reference_the_operation_key() {
    let with = bound(&[population(), config_object(), attempt_update()]);
    let items = |package: &CheckedPackage| [frame_item(package), clause_item(package, "Updated")];
    for result in export_bindings(&with, &items(&with)) {
        let exported = result.expect("the frame is bound");
        assert!(exported
            .bindings
            .iter()
            .any(|(key, binding)| key == &operation_key()
                && matches!(binding, CheckedBinding::Frame(_))));
    }

    let without = bound(&[population(), config_object()]);
    for result in export_bindings(&without, &items(&without)) {
        let refusal = result.expect_err("the frame is unbound");
        assert_eq!(
            unbound_keys(&refusal),
            [(&operation_key(), fixture::PACKAGE)]
        );
    }
}

#[test]
#[trace("TC-798")]
fn a_key_naming_a_declaration_of_another_kind_refuses_malformed_declaration() {
    let package = package(CLAUSES);
    let field = AuthoredBinding::Object {
        key: declaration("ConfigVersion/version"),
        value: ObjectBinding {
            rust_type: RustPath::new("Version"),
            fields: BTreeMap::new(),
        },
    };
    let not_a_population = AuthoredBinding::Population {
        key: declaration("ConfigVersion"),
        value: PopulationBinding {
            collection: RustPath::new("state::configs"),
        },
    };
    for (binding, entry) in [
        (field, MalformedEntry::NotAnObjectType),
        (not_a_population, MalformedEntry::NotAPopulation),
    ] {
        assert_eq!(
            sole(refused(&package, std::slice::from_ref(&binding))),
            AbstractionRefusal::MalformedDeclaration {
                key: binding.key(),
                entry,
            }
        );
    }
}

/// FR-304-AC-9: a systems interface is no object type, so a binding of it, or
/// of an operation it declares, refuses `MalformedDeclaration`.
#[test]
#[trace("FR-304-AC-9", "TC-1087")]
fn a_systems_interface_bound_as_an_object_type_refuses_malformed_declaration() {
    let package = package(CLAUSES);
    let panel = object("Panel", "Panel", &[("level", "level")]);
    assert_eq!(
        sole(refused(&package, std::slice::from_ref(&panel))),
        AbstractionRefusal::MalformedDeclaration {
            key: panel.key(),
            entry: MalformedEntry::NotAnObjectType,
        }
    );
    let operation = frame("Panel", "adjust", &[]);
    assert_eq!(
        sole(refused(&package, std::slice::from_ref(&operation))),
        AbstractionRefusal::MalformedDeclaration {
            key: operation.key(),
            entry: MalformedEntry::NotAnObjectType,
        }
    );
}

#[test]
#[trace("TC-800")]
fn a_parameter_mapped_to_the_receivers_spelling_refuses_malformed_declaration() {
    let package = package(CLAUSES);
    let mut binding = frame(
        "ConfigVersion",
        "attemptUpdate",
        &[("n", "store"), ("m", "limit")],
    );
    let AuthoredBinding::Frame { value, .. } = &mut binding else {
        panic!("a frame binding");
    };
    value.receiver = RustReceiver::Parameter(RustParameter::new("store"));
    assert_eq!(
        sole(refused(&package, std::slice::from_ref(&binding))),
        AbstractionRefusal::MalformedDeclaration {
            key: binding.key(),
            entry: MalformedEntry::ReceiverParameter {
                parameter: name("n"),
                rust: RustParameter::new("store"),
            },
        }
    );
}

#[test]
#[trace("TC-806", "FR-307-AC-1")]
fn a_let_bound_read_references_the_same_object_types_as_the_inline_read() {
    let through_let = package(
        "function bump using v(c: M::ConfigVersion): Integer pure { let x = deref(c).version in x + 1 }\n",
    );
    let inline = package(CLAUSES);
    for package in [&through_let, &inline] {
        let results = export_bindings(package, &[function_item(package)]);
        let [Err(refusal)] = &results[..] else {
            panic!("{results:?}: ConfigVersion is unbound, so the item refuses");
        };
        assert_eq!(unbound_keys(refusal), [(&config_key(), fixture::PACKAGE)]);
    }
}

/// Binding an abstraction changes the package, so its remembered emission is
/// dropped.
#[test]
fn bind_abstraction_resets_the_remembered_emission() {
    let mut package = package(CLAUSES);
    crate::emit_checked(&package).expect("the package emits");
    assert!(package.remembered_emission().is_some());
    package
        .bind_abstraction(&[config_object()])
        .expect("the relation checks");
    assert!(package.remembered_emission().is_none());
}

#[test]
#[trace("TC-1082", "FR-307-AC-6")]
fn with_nothing_bound_every_item_is_refused_and_the_export_succeeds() {
    let package = package(CLAUSES);
    let items = [function_item(&package), clause_item(&package, "Versioned")];
    let results = export_bindings(&package, &items);
    assert_eq!(results.len(), items.len());
    for (result, item) in results.iter().zip(&items) {
        let Err(refusal @ ExportRefusal::Unbound { item: refused, .. }) = result else {
            panic!("{result:?}: an unbound item must be refused");
        };
        assert_eq!(refused, item);
        assert_eq!(refusal.code(), Code::MissingDeclaration);
    }
    assert!(results.iter().all(Result::is_err));
}

#[test]
#[trace("TC-1082", "FR-307-AC-6")]
fn an_item_that_references_no_element_is_bound_with_no_bindings() {
    let package = package(
        "function addBoth using v(x: Int[0, 9], y: Int[0, 9]): Int[0, 18] pure { x + y }\n",
    );
    let mut items = package.graph().requirements().keys().cloned();
    let (Some(item), None) = (items.next(), items.next()) else {
        panic!("one requirement record");
    };
    let results = export_bindings(&package, std::slice::from_ref(&item));
    assert_eq!(
        results,
        [Ok(BoundItem {
            item,
            bindings: Vec::new()
        })]
    );
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-710-AC-5 (TC-1943): a package input supplies a model unit in place of
//! a domain package document, and intake admits it as it admits the compiled
//! document.

use filament_semantic_value::checking::{CheckMode, CheckingLimits};
use fsl_semantics::check::PackageDeclarations;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::domain_package::DomainPackage;
use fsl_semantics::model::intake::{
    PackageInput, admit, admit_unit, compile_model_unit, package_input, package_input_from_sources,
    read_domain_package, read_records,
};
use ix_trace_rs::trace;

/// FR-710-AC-1's model unit.
const SHOP: &str = "language \"ix:native\" edition \"1-draft\";
domain \"example/shop\";
object Order identity (id) {
  id: Int[0, 999];
  total: Int[0, 100];
  operation pay(): Boolean modifies self.total {
    pre CanPay { self.total > 0 };
    post Paid { self.total = pre(self.total) - 1 };
  }
  invariant Nonnegative { self.total >= 0 };
}
population orders: Order;
";

/// A unit selecting `identity` as `M`, with one invariant over `M::Order`.
fn unit(identity: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile P = \"filament.model.complete/v1\";\n\
         model M = \"{identity}\";\n\
         invariant I using P on M::Order at current {{ self.total >= 0 }}\n"
    )
}

fn assert_origin_sources(value: &serde_json::Value, source_identity: &str) {
    match value {
        serde_json::Value::Object(members) => {
            for (key, value) in members {
                if key == "origin" {
                    assert_eq!(
                        value["source"]["sourceIdentity"], source_identity,
                        "{value}"
                    );
                }
                assert_origin_sources(value, source_identity);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                assert_origin_sources(value, source_identity);
            }
        }
        _ => {}
    }
}

/// The unit's selections admitted against an input holding the model unit.
fn selected_unit_declarations(
    identity: &str,
    packages: &PackageInput,
) -> Result<PackageDeclarations, String> {
    let text = unit(identity);
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "model-unit-input"),
        "unit.native",
        text.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = fsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .map_err(|refusal| format!("{refusal:?}"))?;
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        models,
        Vec::new(),
    )
    .map_err(|refusal| format!("{refusal:?}"))
}

#[trace("FR-710-AC-5", "TC-1943")]
#[test]
fn a_supplied_model_unit_is_admitted_as_its_compiled_document() {
    let source = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:shop-model"),
        "shop.fsl",
        SHOP.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("the model source metadata is admitted");
    let source_packages = package_input_from_sources([("example/shop", &source)])
        .expect("the model unit compiles to a source-derived package");
    let compiled_document = source_packages["example/shop"].clone();
    let document_packages = package_input([compiled_document.as_slice()])
        .expect("the compiled document becomes package input");
    let checked_source = checked_domain_package("example/shop", &source_packages);
    let checked_document = checked_domain_package("example/shop", &document_packages);
    assert_eq!(checked_source, checked_document);
    let mut declarations = selected_unit_declarations("example/shop", &source_packages)
        .expect("the model unit is admitted");
    declarations
        .link_dispatch(ModelNormalizationLimits::default())
        .expect("dispatch links");
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .expect("the invariant over M::Order checks");
}

fn checked_domain_package(identity: &str, packages: &PackageInput) -> DomainPackage {
    let (selection, document) = admit(identity, packages, fsl_foundation::IntakeLimits::default())
        .expect("package document is admitted under its declared identity");
    read_domain_package(selection, &document).expect("admitted document reads as checked records")
}

#[trace("FR-710-AC-5", "TC-1943")]
#[test]
fn a_model_unit_for_another_identity_is_refused_wrong_model_selection() {
    let source = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:shop-model"),
        "shop.fsl",
        SHOP.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("source metadata admitted");
    let packages = package_input_from_sources([("example/other", &source)])
        .expect("the selected identity is retained for FR-154 admission");
    let refusal =
        selected_unit_declarations("example/other", &packages).expect_err("the identities differ");
    assert!(refusal.contains("WrongModelSelection"), "{refusal}");
}

#[trace("FR-710-AC-6", "TC-2998")]
#[trace("FR-710-AC-1", "TC-1942")]
#[test]
fn compile_model_unit_returns_its_identity_and_stable_document_bytes() {
    let source = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:shop-model"),
        "shop.fsl",
        SHOP.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("source metadata admitted");
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:shop-model"),
        "shop.fsl",
        SHOP.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let compiled = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect("the model unit compiles");
    let repeated = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect("the same model unit compiles again");
    assert_eq!(compiled.identity(), "example/shop");
    assert_eq!(compiled.bytes(), repeated.bytes());
    let document: serde_json::Value =
        serde_json::from_slice(compiled.bytes()).expect("compiled document is JSON");
    assert_eq!(document["contractVersion"], "2.0.0");
    assert_eq!(document["package"]["identity"], "example/shop");
    assert_eq!(
        document["types"][0]["fields"][0]["origin"]["source"]["sourceIdentity"],
        "ix://agent-ix/test:shop-model"
    );
    assert_eq!(document["types"].as_array().unwrap().len(), 1);
    let order = &document["types"][0];
    assert_eq!(order["identity"], "ix://example/shop/Order");
    assert_eq!(order["fields"].as_array().unwrap().len(), 2);
    for (index, name) in ["id", "total"].into_iter().enumerate() {
        let field = &order["fields"][index];
        assert_eq!(field["identity"], format!("ix://example/shop/Order/{name}"));
        assert_eq!(field["multiplicity"]["lower"], 1);
        assert_eq!(field["multiplicity"]["upper"], 1);
        assert!(field["origin"].is_object());
    }
    assert_eq!(order["operations"].as_array().unwrap().len(), 1);
    let operation = &order["operations"][0];
    assert_eq!(operation["identity"], "ix://example/shop/Order/pay");
    assert_eq!(
        operation["returns"]["typeRef"],
        "ix://filament/native/Boolean"
    );
    assert_eq!(
        operation["frame"]["modifies_receiver"][0],
        "ix://example/shop/Order/total"
    );
    assert_eq!(operation["pre"][0]["text"], "self.total > 0");
    assert_eq!(
        operation["post"][0]["text"],
        "self.total = pre(self.total) - 1"
    );
    assert!(operation["origin"].is_object());
    assert_eq!(order["clauses"].as_array().unwrap().len(), 1);
    assert_eq!(order["clauses"][0]["clauseId"], "Nonnegative");
    let populations = document["populations"].as_array().unwrap();
    assert_eq!(populations.len(), 1);
    assert_eq!(populations[0]["identity"], "ix://example/shop/orders");
    assert_eq!(populations[0]["extent"], "closed");
    let constructs = document["constructs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|construct| {
            (
                construct["kind"]["name"].as_str().unwrap(),
                construct["construct"]["meaning"].as_str().unwrap(),
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        constructs,
        std::collections::BTreeSet::from([
            ("object_type", "filament.meaning.model.object-type/v1"),
            ("population", "filament.meaning.model.population/v1"),
        ])
    );
    let admitted = package_input([SHOP.as_bytes()]).expect("model unit compiles to package input");
    let source_packages = package_input_from_sources([("example/shop", &source)])
        .expect("source-labelled model compiles");
    let mut declarations = selected_unit_declarations("example/shop", &source_packages)
        .expect("the source-supplied AC-1 model with its invariant is admitted");
    declarations
        .link_dispatch(ModelNormalizationLimits::default())
        .expect("dispatch links");
    declarations
        .check(
            CheckMode::Linked,
            CheckingLimits::default(),
            &crate::reward_evaluator::ZeroRewardEvaluator,
        )
        .expect("the invariant over the source-supplied model checks");
    let admitted_document: serde_json::Value =
        serde_json::from_slice(&admitted["example/shop"]).expect("admitted document is JSON");
    assert_eq!(
        admitted_document["types"][0]["operations"][0]["name"],
        "pay"
    );
}

#[trace("FR-710-AC-6", "TC-2998")]
#[trace("FR-710-AC-1", "TC-1942")]
#[test]
fn package_input_from_sources_keeps_source_labels_and_compiled_document_conflicts() {
    let source = fsl_cst::diagnostic::read_source(
        fsl_foundation::SourceIdentity::new("agent-ix", "test:shop-model"),
        "shop.fsl",
        SHOP.as_bytes(),
        fsl_cst::Limits::default().source_bytes,
    )
    .expect("source metadata admitted");
    let packages =
        package_input_from_sources([("example/shop", &source)]).expect("source compiles to input");
    let document: serde_json::Value =
        serde_json::from_slice(&packages["example/shop"]).expect("compiled document is JSON");
    assert_origin_sources(&document, "ix://agent-ix/test:shop-model");
    let selected = unit("example/shop");
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "model-unit-source-intake"),
        "unit.native",
        selected.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("the model-selection unit parses");
    let built = fsl_forms::build_unit(&parsed).expect("the model-selection unit builds");
    admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("source-compiled package input is admitted under its selected identity");
    let compiled = package_input([SHOP.as_bytes()]).expect("anonymous model unit compiles");
    let conflict = package_input([SHOP.as_bytes(), compiled["example/shop"].as_slice()])
        .expect_err("a model unit and its compiled document share one identity");
    assert!(
        format!("{conflict:?}").contains("ConflictingPackage"),
        "{conflict:?}"
    );
}

#[trace("FR-710-AC-2", "TC-1942")]
#[test]
fn a_model_unit_compiles_its_relationship_and_population() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/graph";
object Node { rank: Int[0, 9]; relationship next: Node [0, 1]; }
population nodes: Node;
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "graph-model"),
        "graph.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let compiled = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect("the relationship model compiles");
    let document: serde_json::Value =
        serde_json::from_slice(compiled.bytes()).expect("compiled document is JSON");
    assert_eq!(
        document["types"][0]["relationships"][0]["identity"],
        "ix://example/graph/relationship/Node-next-Node"
    );
    assert_eq!(
        document["types"][0]["relationships"][0]["targetEnd"]["multiplicity"]["lower"],
        0
    );
    let relationship = &document["types"][0]["relationships"][0];
    assert_eq!(relationship["sourceEnd"]["role"], "next");
    assert!(relationship["targetEnd"].get("role").is_none());
    assert_eq!(relationship["sourceEnd"]["multiplicity"]["lower"], 0);
    assert!(relationship["sourceEnd"]["multiplicity"].get("upper").is_none());
    assert_eq!(document["populations"][0]["extent"], "closed");
    let packages = package_input([source.as_bytes()]).expect("model unit becomes package input");
    let (_, admitted) = admit(
        "example/graph",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the compiled relationship document");
    read_records("example/graph", &admitted).expect("the relationship records are read");
}

#[trace("FR-710-AC-2", "TC-1942")]
#[test]
fn a_relationship_inverse_names_and_bounds_the_source_end() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/graph";
object Node { relationship next: Node [0, 1] inverse prev [0, 2]; }
population nodes: Node;
"#;
    let packages = package_input([source.as_bytes()]).expect("model unit becomes package input");
    let document: serde_json::Value =
        serde_json::from_slice(&packages["example/graph"]).expect("compiled document is JSON");
    let relationship = &document["types"][0]["relationships"][0];
    assert_eq!(relationship["sourceEnd"]["role"], "next");
    assert_eq!(relationship["targetEnd"]["role"], "prev");
    assert_eq!(relationship["targetEnd"]["multiplicity"]["upper"], 1);
    assert_eq!(relationship["sourceEnd"]["multiplicity"]["lower"], 0);
    assert_eq!(relationship["sourceEnd"]["multiplicity"]["upper"], 2);
    admit(
        "example/graph",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the inverse end");
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_names_every_native_type() {
    let natives = [
        "Boolean", "Integer", "Timestamp", "UUID", "String", "Bytes", "Duration", "JsonObject",
        "Decimal[0, 2; 10, 2; exact]",
    ];
    let fields = natives
        .iter()
        .enumerate()
        .map(|(position, native)| format!("  f{position}: {native};\n"))
        .collect::<String>();
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\ndomain \"example/natives\";\nobject Holder {{\n{fields}}}\n"
    );
    let packages = package_input([source.as_bytes()]).expect("native type names resolve");
    let document: serde_json::Value =
        serde_json::from_slice(&packages["example/natives"]).expect("compiled document is JSON");
    for (position, native) in natives.iter().enumerate() {
        let name = native.split('[').next().unwrap();
        assert_eq!(
            document["types"][0]["fields"][position]["typeRef"],
            format!("ix://filament/native/{name}")
        );
    }
    admit(
        "example/natives",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the native types");
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn a_model_unit_with_an_unresolved_relationship_target_refuses_at_the_name() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/graph";
object Node { relationship next: Ghost [0, 1]; }
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "ghost-model"),
        "ghost.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let diagnostics = fsl_semantics::model::intake::compile_model_unit_with_causes(
        &parsed,
        ModelNormalizationLimits::default(),
    )
    .expect_err("an undeclared relationship target is refused");
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("one located refusal: {diagnostics:?}");
    };
    assert_eq!(diagnostic.code, fsl_cst::CompleteCode::MissingDeclaration);
    assert_eq!(diagnostic.cause.as_str(), "missing-name");
    let region = diagnostic.region.as_ref().expect("located refusal");
    let span = parsed
        .source()
        .render(region)
        .expect("source region renders");
    assert_eq!(span.start.byte, source.find("Ghost").expect("target name"));
    assert_eq!(span.end.byte, span.start.byte + "Ghost".len());
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn duplicate_object_declarations_refuse_at_the_second_name() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/graph";
object Node { }
object Node { }
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "duplicate-model"),
        "duplicate.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let diagnostics = fsl_semantics::model::intake::compile_model_unit_with_causes(
        &parsed,
        ModelNormalizationLimits::default(),
    )
    .expect_err("duplicate object identities are refused");
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("one located refusal: {diagnostics:?}");
    };
    assert_eq!(diagnostic.code, fsl_cst::CompleteCode::AmbiguousDeclaration);
    assert_eq!(diagnostic.cause.as_str(), "ambiguous-name");
    let span = parsed
        .source()
        .render(diagnostic.region.as_ref().expect("located refusal"))
        .expect("source region renders");
    let second_name = source.rfind("Node").expect("second declaration name");
    assert_eq!(span.start.byte, second_name);
    assert_eq!(span.end.byte, second_name + "Node".len());
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn an_operation_on_a_record_refuses_as_an_unsupported_construct() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/records";
record Row { amount: Integer; operation update(); }
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "record-operation-model"),
        "records.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let diagnostics = fsl_semantics::model::intake::compile_model_unit_with_causes(
        &parsed,
        ModelNormalizationLimits::default(),
    )
    .expect_err("FSpec does not admit operations on record value types");
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("one located refusal: {diagnostics:?}");
    };
    assert_eq!(diagnostic.code, fsl_cst::CompleteCode::UnsupportedConstruct);
    assert_eq!(diagnostic.cause.as_str(), "declaration-form");
    let span = parsed
        .source()
        .render(diagnostic.region.as_ref().expect("located refusal"))
        .expect("source region renders");
    let operation = source.find("operation").expect("operation token");
    assert_eq!(span.start.byte, operation);
    assert_eq!(span.end.byte, operation + "operation".len());
}

#[trace("FR-710-AC-4", "TC-1943")]
#[test]
fn a_reserved_domain_identity_refuses_at_the_string() {
    let source = r#"language "ix:native" edition "1-draft";
domain "filament/native/x";
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "reserved-model-unit"),
        "reserved.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let diagnostics = fsl_semantics::model::intake::compile_model_unit_with_causes(
        &parsed,
        ModelNormalizationLimits::default(),
    )
    .expect_err("the native domain namespace is reserved");
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("one located refusal: {diagnostics:?}");
    };
    assert_eq!(diagnostic.code, fsl_cst::CompleteCode::InvalidModelBinding);
    assert_eq!(diagnostic.cause.as_str(), "malformed-declaration");
    let span = parsed
        .source()
        .render(diagnostic.region.as_ref().expect("located refusal"))
        .expect("source region renders");
    let identity = source
        .find("\"filament/native/x\"")
        .expect("domain identity");
    assert_eq!(span.start.byte, identity);
    assert_eq!(span.end.byte, identity + "\"filament/native/x\"".len());
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_a_bounded_native_type_alias() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/scalars";
type Small = Int[0, 3];
object Item { amount: Small; }
population items: Item;
"#;
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "scalar-model"),
        "scalar.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source metadata admitted");
    let compiled = compile_model_unit(&parsed, ModelNormalizationLimits::default())
        .expect("the scalar alias model compiles");
    let document: serde_json::Value =
        serde_json::from_slice(compiled.bytes()).expect("compiled document is JSON");
    assert_eq!(document["types"][0]["scalar"], "integer");
    assert_eq!(
        document["types"][0]["constraints"][0]["operands"]["value"],
        "0"
    );
    assert_eq!(
        document["types"][0]["constraints"][1]["operands"]["value"],
        "3"
    );
    let packages =
        package_input([source.as_bytes()]).expect("compiled alias becomes package input");
    admit(
        "example/scalars",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the compiled scalar alias");
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_a_record_value_type() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/records";
record Point { x: Integer; }
"#;
    let packages = package_input([source.as_bytes()]).expect("record model unit compiles");
    let (_, document) = admit(
        "example/records",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the record value type");
    let records = read_records("example/records", &document).expect("record is read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::RecordValueType(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_enum_and_union_cases() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/variants";
enum Tier { Gold, Standard }
union Mark { Circle(Integer), Blank }
"#;
    let packages = package_input([source.as_bytes()]).expect("variant model unit compiles");
    let (_, document) = admit(
        "example/variants",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the variant types");
    let records = read_records("example/variants", &document).expect("variant cases are read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::Enum(_)
        )
    }));
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::Variant(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_a_map_type() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/maps";
map Index = Map<Text[0, 8; nfc], Integer>[0, 4];
"#;
    let packages = package_input([source.as_bytes()]).expect("map model unit compiles");
    let (_, document) = admit(
        "example/maps",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the map type");
    let records = read_records("example/maps", &document).expect("map type is read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::MapType(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_dimensions_and_units() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/units";
dimension Length;
unit m: Length = rational(1, 1);
"#;
    let packages = package_input([source.as_bytes()]).expect("unit model compiles");
    let compiled: serde_json::Value =
        serde_json::from_slice(&packages["example/units"]).expect("compiled model unit is JSON");
    assert_eq!(compiled["dimensions"][0]["name"], "Length");
    assert_eq!(compiled["units"][0]["name"], "m");
    admit(
        "example/units",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the dimension and unit");
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_an_event_type() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/events";
event Placed { timestamp: Timestamp; occurs timestamp; }
"#;
    let packages = package_input([source.as_bytes()]).expect("event model compiles");
    let (_, document) = admit(
        "example/events",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the event type");
    let records = read_records("example/events", &document).expect("event type is read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::EventType(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_a_namespace() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/namespaces";
object Node { amount: Integer; }
namespace Terms { members Node; term node_term = "a node"; }
"#;
    let packages = package_input([source.as_bytes()]).expect("namespace model compiles");
    let (_, document) = admit(
        "example/namespaces",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the namespace");
    let records = read_records("example/namespaces", &document).expect("namespace is read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::Namespace(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_a_systems_interface() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/systems";
interface Pump { rate: Integer; operation start(); }
"#;
    let packages = package_input([source.as_bytes()]).expect("interface model compiles");
    let (_, document) = admit(
        "example/systems",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the systems interface");
    let records = read_records("example/systems", &document).expect("interface is read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::ObjectType(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_systems_parts_ports_connections_and_allocations() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/topology";
object Sys { level: Integer; }
interface Pump { rate: Integer; }
part pump: Sys [1, 1] in Sys;
port out_port: Pump [1, 1] out on pump;
port in_port: Pump [1, 1] in on pump;
connection Pipe pump.out_port -> pump.in_port;
allocate Alloc from pump to pump;
"#;
    let packages = package_input([source.as_bytes()]).expect("systems model compiles");
    let (_, document) = admit(
        "example/topology",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits systems model declarations");
    let records = read_records("example/topology", &document).expect("systems records are read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::Component(_)
        )
    }));
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::Endpoint(_)
        )
    }));
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::Allocation(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_a_state_machine() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/machines";
event Placed { timestamp: Timestamp; occurs timestamp; }
machine Gate { active: Boolean; operation toggle(); states (Open, Shut); transition Open -> Shut on toggle emits Placed; }
"#;
    let packages = package_input([source.as_bytes()]).expect("state machine model compiles");
    let (_, document) = admit(
        "example/machines",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the state machine");
    let records = read_records("example/machines", &document).expect("machine is read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::StateMachine(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_a_process() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/processes";
event Placed { timestamp: Timestamp; occurs timestamp; }
process Flow { n: Integer; operation go(); step start: begin consumes Placed; }
"#;
    let packages = package_input([source.as_bytes()]).expect("process model compiles");
    let (_, document) = admit(
        "example/processes",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the process");
    let records = read_records("example/processes", &document).expect("process is read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::Process(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_a_persistence_interface() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/stores";
object Node { amount: Integer; }
store Nodes persists Node { operation write_record(entry: Reference<Node>); }
"#;
    let packages = package_input([source.as_bytes()]).expect("store model compiles");
    let (_, document) = admit(
        "example/stores",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the store");
    let records = read_records("example/stores", &document).expect("store is read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::PersistenceInterface(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_operation_contracts_and_invariants() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/contracts";
object Node {
  id: Integer;
  operation bump(): Boolean modifies self.id {
    pre Room { self.id < 9 };
    post Grew { self.id > pre(self.id) };
  }
  invariant Bounded { self.id >= 0 };
}
"#;
    let packages = package_input([source.as_bytes()]).expect("contract model compiles");
    let (_, document) = admit(
        "example/contracts",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the contracts and invariant");
    let records = read_records("example/contracts", &document).expect("contract records are read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::OperationMember(_)
        )
    }));
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::Clause(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn a_model_unit_compiles_and_admits_an_option_field_type() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/options";
object Holder { maybe: Option<Integer>; }
"#;
    let packages = package_input([source.as_bytes()]).expect("option field model compiles");
    let (_, document) = admit(
        "example/options",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the option field");
    let records = read_records("example/options", &document).expect("option records are read");
    assert!(records.iter().any(|record| {
        matches!(
            record,
            fsl_semantics::model::domain_package::DomainPackageRecord::OptionType(_)
        )
    }));
}

#[trace("FR-710-AC-3", "TC-1942")]
#[test]
fn one_model_unit_compiles_and_admits_every_declaration_form() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/all";
type Small = Int[0, 3];
record Point { x: Integer; }
tuple Pair(Integer, Boolean);
enum Tier { Gold, Standard }
union Mark { Circle(Integer), Blank }
map Index = Map<Text[0, 8; nfc], Integer>[0, 4];
dimension Length;
unit m: Length = rational(1, 1);
object Node identity (id) {
  id: Int[0, 9];
  note: Option<Integer>;
  tags: Integer [0, *] ordered unique;
  relationship next: Node [0, 1] inverse prev [0, 1];
  operation bump(): Boolean modifies self.id {
    pre Room { self.id < 9 };
    post Grew { self.id > pre(self.id) };
  }
  invariant Bounded { self.id >= 0 };
}
event Placed { timestamp: Timestamp; occurs timestamp; }
machine Gate { active: Boolean; operation toggle(); states (Open, Shut); transition Open -> Shut on toggle emits Placed; }
process Flow { n: Integer; operation go(); step start: begin consumes Placed; }
store Nodes persists Node { operation write_record(entry: Reference<Node>); }
namespace Terms { members Node, Gate; term node_term = "a node"; }
population nodes: Node;
open symmetric population peers: Node;
interface Pump { rate: Integer; operation start(); }
object Sys { level: Integer; }
part pump: Sys [1, 1] in Sys;
port out_port: Pump [1, 1] out on pump;
port in_port: Pump [1, 1] in on pump;
connection Pipe pump.out_port -> pump.in_port;
allocate Alloc from pump to pump;
"#;
    let packages = package_input([source.as_bytes()]).expect("all-form model unit compiles");
    let document: serde_json::Value =
        serde_json::from_slice(&packages["example/all"]).expect("compiled model unit is JSON");
    let meanings = document["constructs"]
        .as_array()
        .expect("constructs is an array")
        .iter()
        .filter_map(|construct| construct["construct"]["meaning"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for meaning in [
        "filament.meaning.model.object-type/v1",
        "filament.meaning.model.value-type/v1",
        "filament.meaning.model.record-value-type/v1",
        "filament.meaning.model.tuple-value-type/v1",
        "filament.meaning.model.variant-type/v1",
        "filament.meaning.model.event-type/v1",
        "filament.meaning.model.state-machine/v1",
        "filament.meaning.model.process/v1",
        "filament.meaning.model.persistence-interface/v1",
        "filament.meaning.model.namespace/v1",
        "filament.meaning.model.population/v1",
        "filament.meaning.systems.interface/v1",
        "filament.meaning.systems.part/v1",
        "filament.meaning.systems.port/v1",
        "filament.meaning.systems.connection/v1",
        "filament.meaning.systems.allocation/v1",
    ] {
        assert!(
            meanings.contains(meaning),
            "missing construct meaning {meaning}"
        );
    }
    let types = document["types"].as_array().expect("types is an array");
    let type_named = |name: &str| {
        let identity = format!("ix://example/all/{name}");
        types
            .iter()
            .find(|ty| ty["identity"].as_str() == Some(identity.as_str()))
            .unwrap_or_else(|| panic!("missing compiled type {name}"))
    };
    let machine = type_named("Gate");
    assert_eq!(machine["states"].as_array().unwrap().len(), 2);
    assert_eq!(machine["transitions"].as_array().unwrap().len(), 1);
    assert_eq!(
        machine["transitions"][0]["from"],
        "ix://example/all/Gate/Open"
    );
    assert_eq!(
        machine["transitions"][0]["to"],
        "ix://example/all/Gate/Shut"
    );
    assert_eq!(
        machine["transitions"][0]["trigger"],
        "ix://example/all/Gate/toggle"
    );
    let process = type_named("Flow");
    assert_eq!(process["identityFields"][0], "ix://example/all/Flow/n");
    assert_eq!(process["steps"][0]["name"], "start");
    assert_eq!(process["steps"][0]["stepKind"], "command");
    assert_eq!(
        process["steps"][0]["consumes"][0],
        "ix://example/all/Placed"
    );
    let store = type_named("Nodes");
    assert_eq!(store["persists"][0], "ix://example/all/Node");
    assert_eq!(
        store["operations"][0]["params"][0]["typeRef"],
        "ix://example/all/Node"
    );
    let node = type_named("Node");
    let option_identity = node["fields"][1]["typeRef"]
        .as_str()
        .expect("Option field references its option type");
    assert!(
        types.iter().any(|ty| {
            ty["identity"].as_str() == Some(option_identity) && ty["kind"] == "option"
        })
    );
    let connection = type_named("Pipe");
    assert_eq!(connection["sourceEnd"]["type"], "ix://example/all/out_port");
    assert_eq!(connection["targetEnd"]["type"], "ix://example/all/in_port");
    admit(
        "example/all",
        &packages,
        ModelNormalizationLimits::default().intake,
    )
    .expect("FR-154 admits the all-form document");
}

/// A named model-unit range is a schema-valid type definition and reaches
/// the checked attribute with its authored bounds intact.
#[test]
#[trace("FR-710-AC-5", "TC-1943")]
fn a_named_integer_model_type_admits_and_retains_its_bounds() {
    let source = r#"language "ix:native" edition "1-draft";
domain "example/shop";
type Total = Int[0, 3];
object Order { total: Total; }
population orders: Order;
"#;
    let packages = package_input([source.as_bytes()])
        .expect("the authored model unit compiles to package input");
    let _checked_document = checked_domain_package("example/shop", &packages);
    let mut declarations = selected_unit_declarations("example/shop", &packages)
        .expect("the named type's compiled document passes strict intake");
    declarations.link_dispatch(ModelNormalizationLimits::default()).expect("dispatch links");
    let graph = declarations.check(
        CheckMode::Linked, CheckingLimits::default(),
        &crate::reward_evaluator::ZeroRewardEvaluator,
    ).expect("the invariant over the named bounded field checks");
    let types = graph.scope().types();
    let order = types.object_types().find(|object| object.name() == "Order")
        .expect("the model declares Order");
    let attributes = types.attributes(order.key()).expect("Order has attributes");
    let total = attributes.iter().find(|attribute| attribute.field().name() == "total")
        .expect("Order declares total");
    let filament_exact::ValueType::Int(bounds) = total.field().value_type() else {
        panic!("the named field retains its integer range: {:?}", total.field().value_type());
    };
    assert_eq!(bounds.lower().to_string(), "0");
    assert_eq!(bounds.upper().to_string(), "3");
}

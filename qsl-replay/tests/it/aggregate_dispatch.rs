// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-480-AC-3: cross-unit dispatch links model aliases to shared declarations.

use ix_trace_rs::trace;
use qsl_cst::Limits as SourceLimits;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_replay::spine::{self, admit_parsed_unit_list, check_units, select_units, ParseRequest};
use qsl_replay::{DependencyInput, SourceIdentity, SuppliedLibrary};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    AdmissionUsage, AdmittedObservations, DocumentRef, Observation,
};
use quire_exact::{
    Cancel, FieldValue, Integer, LimitKind, Meter, ObjectId, ObjectReference, UniverseId,
    Value as Quire,
};
use quire_semantic_value::object_closure::ObjectClosure;
use serde_json::{json, Value};

const PACKAGE: &str = "example/config-version";
const HISTORY: &str = "ix://example/config-version/config_history";
const EXAMPLE: &str = include_str!("../../../examples/config-version/model.semantic-ir.json");

fn query(name: &str) -> Value {
    let identity = format!("ix://{PACKAGE}/ConfigVersion/{name}");
    json!({
        "identity": identity,
        "name": name,
        "params": [],
        "returns": {
            "typeRef": "ix://quire/native/Integer",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [], "post": [],
        "origin": {"source": {
            "sourceIdentity": format!("ix://{PACKAGE}/spec"),
            "path": "spec.qspec", "startLine": 1, "startColumn": 1,
        }},
        "frame": {"modifies": [], "creates": [], "deletes": []},
    })
}

fn model_document() -> Vec<u8> {
    let mut envelope: Value = serde_json::from_str(EXAMPLE).expect("fixture JSON");
    let types = envelope["types"].as_array_mut().expect("types");
    let config = types
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/ConfigVersion"))
        .expect("ConfigVersion");
    let base_size = query("size");
    config["operations"] = json!([base_size, query("other")]);
    let sub_size = query("size");
    let mut sub_size = sub_size;
    sub_size["identity"] = json!(format!("ix://{PACKAGE}/Sub/size"));
    sub_size["redefines"] = json!(format!("ix://{PACKAGE}/ConfigVersion/size"));
    let mut probe = query("probe");
    probe["identity"] = json!(format!("ix://{PACKAGE}/Sub/probe"));
    types.push(json!({
        "identity": format!("ix://{PACKAGE}/Sub"),
        "displayName": format!("ix://{PACKAGE}/Sub"),
        "kind": {"module": PACKAGE, "name": "object_type"},
        "roles": [], "constraints": [], "extensions": [], "unknownPolicy": "reject",
        "origin": {"generated": {
            "generatorIdentity": format!("ix://{PACKAGE}/Sub"), "generatorVersion": "1.0.0",
            "inputIdentities": [format!("ix://{PACKAGE}/Sub")],
        }},
        "supertypes": [format!("ix://{PACKAGE}/ConfigVersion")], "fields": [],
        "operations": [sub_size, probe],
    }));
    // D10's canonical reference is a `Sub`, so the fixture's history
    // population admits that declared subtype while retaining its original
    // ConfigVersion member.
    let history = envelope["populations"]
        .as_array_mut()
        .expect("populations")
        .iter_mut()
        .find(|population| population["identity"] == format!("ix://{PACKAGE}/config_history"))
        .expect("config_history");
    history["members"]
        .as_array_mut()
        .expect("population members")
        .push(json!(format!("ix://{PACKAGE}/Sub")));
    serde_json::to_vec(&envelope).expect("valid model JSON")
}

fn parse_unit(source: &SourceIdentity, model: &str, declarations: &str) -> spine::ParsedUnitSource {
    parse_unit_for_package(source, model, PACKAGE, declarations)
}

fn parse_unit_for_package(
    source: &SourceIdentity,
    model: &str,
    package: &str,
    declarations: &str,
) -> spine::ParsedUnitSource {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile m = \"quire.model.complete/v1\";\n\
         model {model} = {package:?};\n{declarations}"
    );
    spine::parse(
        &ParseRequest {
            source,
            path: "unit.native",
            bytes: text.as_bytes(),
        },
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("unit parses")
    .into_value()
}

fn linked(body_b: &str) -> Result<Vec<spine::CheckedUnit>, String> {
    let source_a = SourceIdentity::new("test", "ac3:a", "a", "1");
    let source_b = SourceIdentity::new("test", "ac3:b", "b", "1");
    let a = parse_unit(
        &source_a,
        "M",
        "body sizeA using m on M::ConfigVersion::size() { 1 }\n",
    );
    let b = parse_unit(&source_b, "N", body_b);
    let dependencies = DependencyInput::default();
    let admitted = admit_parsed_unit_list(vec![b, a], &dependencies)
        .map_err(|error| format!("admit: {error:?}"))?;
    let document = model_document();
    let packages = package_input([document.as_slice()]).expect("model package input");
    let models = select_units(
        &admitted,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .map_err(|error| format!("select: {error:?}"))?
    .into_value();
    check_units(
        &admitted,
        "aggregate-dispatch".to_owned(),
        &models,
        &dependencies,
        spine::SpineLimits::default(),
        &Cancel::new(),
    )
    .map(|checked| checked.into_value())
    .map_err(|error| format!("check: {error:?}"))
}

fn linked_clauses(reverse: bool) -> Vec<spine::CheckedUnit> {
    let source_a = SourceIdentity::new("test", "ac1:a", "u1", "1");
    let source_b = SourceIdentity::new("test", "ac1:b", "u2", "1");
    let a = parse_unit(
        &source_a,
        "M",
        "body sizeA using m on M::ConfigVersion::size() { 1 }\n\
         pre PA using v on M::ConfigVersion::size { false }\n",
    );
    let b = parse_unit(
        &source_b,
        "M",
        "pre PB using v on M::ConfigVersion::size { false }\n",
    );
    let dependencies = DependencyInput::default();
    let units = if reverse { vec![b, a] } else { vec![a, b] };
    let admitted = admit_parsed_unit_list(units, &dependencies).expect("units admitted");
    let document = model_document();
    let packages = package_input([document.as_slice()]).expect("model package input");
    let models = select_units(
        &admitted,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("model aliases selected")
    .into_value();
    check_units(
        &admitted,
        "split-clause-package".to_owned(),
        &models,
        &dependencies,
        spine::SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("split clauses link")
    .into_value()
}

fn d10(swapped: bool, split_pb1: bool, reverse: bool) -> Vec<spine::CheckedUnit> {
    let source_a = SourceIdentity::new("test", "d10:u1", "u1", "1");
    let source_b = SourceIdentity::new("test", "d10:u2", "u2", "1");
    let header = |body: &str| {
        format!(
            "body sizeA using m on M::ConfigVersion::size() {{ 1 }}\n\
             body sizeB using m on M::Sub::size() {{ 2 }}\n\
             {body}"
        )
    };
    let lookup = "(lookup<M::ConfigVersion>(M::config_history, self) absent undefined) = self";
    let false_clause = "false";
    let (pb1, pb2) = if swapped {
        (false_clause, lookup)
    } else {
        (lookup, false_clause)
    };
    let (a_clauses, b_clauses) = if split_pb1 {
        (
            format!(
                "pre PA using m on M::ConfigVersion::size {{ false }}\npre PB1 using m on M::Sub::size {{ {pb1} }}\n"
            ),
            format!("pre PB2 using m on M::Sub::size {{ {pb2} }}\n"),
        )
    } else {
        (
            "pre PA using m on M::ConfigVersion::size { false }\n".to_owned(),
            format!(
                "pre PB1 using m on M::Sub::size {{ {pb1} }}\npre PB2 using m on M::Sub::size {{ {pb2} }}\n"
            ),
        )
    };
    let a = parse_unit(
        &source_a,
        "M",
        &header(&format!(
            "{a_clauses}pre I using m on M::Sub::probe {{ self.size() = 2 }}\n"
        )),
    );
    let b = parse_unit(&source_b, "M", &b_clauses);
    let dependencies = DependencyInput::default();
    let units = if reverse { vec![b, a] } else { vec![a, b] };
    let admitted = admit_parsed_unit_list(units, &dependencies).expect("D10 units admitted");
    let document = model_document();
    let packages = package_input([document.as_slice()]).expect("D10 model input");
    let models = select_units(
        &admitted,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("D10 model selected")
    .into_value();
    check_units(
        &admitted,
        "d10-package".to_owned(),
        &models,
        &dependencies,
        spine::SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("D10 package links")
    .into_value()
}

fn d10_single(swapped: bool) -> spine::CheckedUnit {
    let source = SourceIdentity::new("test", "d10:single", "single", "1");
    let lookup = "(lookup<M::ConfigVersion>(M::config_history, self) absent undefined) = self";
    let (pb1, pb2) = if swapped {
        ("false", lookup)
    } else {
        (lookup, "false")
    };
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile m = \"quire.model.complete/v1\";\n\
         model M = {PACKAGE:?};\n\
         body sizeA using m on M::ConfigVersion::size() {{ 1 }}\n\
         body sizeB using m on M::Sub::size() {{ 2 }}\n\
         pre PA using m on M::ConfigVersion::size {{ false }}\n\
         pre PB1 using m on M::Sub::size {{ {pb1} }}\n\
         pre PB2 using m on M::Sub::size {{ {pb2} }}\n\
         pre I using m on M::Sub::probe {{ self.size() = 2 }}\n"
    );
    let parsed = spine::parse(
        &ParseRequest {
            source: &source,
            path: "single.native",
            bytes: text.as_bytes(),
        },
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("single D10 source parses")
    .into_value();
    let dependencies = DependencyInput::default();
    let admitted =
        admit_parsed_unit_list(vec![parsed], &dependencies).expect("single D10 unit admitted");
    let document = model_document();
    let packages = package_input([document.as_slice()]).expect("single D10 model input");
    let models = select_units(
        &admitted,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("single D10 model selected")
    .into_value();
    let populations: Vec<_> = models
        .models()
        .iter()
        .flat_map(|model| model.view.domain_package().records.iter())
        .filter_map(|record| match record {
            qsl_semantics::model::domain_package::DomainPackageRecord::Population(population) => {
                Some(population.key.node.clone())
            }
            _ => None,
        })
        .collect();
    assert!(
        populations
            .iter()
            .any(|name| name.ends_with("config_history")),
        "selected package population records: {populations:?}"
    );
    check_units(
        &admitted,
        "d10-single-package".to_owned(),
        &models,
        &dependencies,
        spine::SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("single D10 package links")
    .into_value()
    .remove(0)
}

fn run_d10(checked: &spine::CheckedUnit) -> (String, u64) {
    let package = checked.package();
    let graph = package.graph();
    let clause = graph.state_clause("I").expect("I precondition");
    let effective = clause.context();
    let reference = ObjectReference::new(
        UniverseId::new(
            quire_exact::NodeKey::new("d10", vec![quire_exact::PathSegment::Text("d1".into())], 0)
                .expect("d1 key"),
        ),
        effective,
        ObjectId::new("d1".to_owned()).expect("d1 id"),
    );
    let environment = ObjectEnvironment::new(
        ObjectClosure::new(
            graph.scope().types(),
            vec![(
                reference.clone(),
                vec![
                    (
                        "versionNumber",
                        FieldValue::Present(Quire::Integer(Integer::from(1_i64))),
                    ),
                    ("parent", FieldValue::Absent),
                ],
            )],
            &[],
        )
        .expect("closed environment"),
    )
    .with_population_members(std::collections::BTreeMap::from([(
        HISTORY.to_owned(),
        Vec::new(),
    )]))
    .with_population_completeness(std::collections::BTreeMap::from([(
        HISTORY.to_owned(),
        true,
    )]));
    let observations = AdmittedObservations {
        usage: AdmissionUsage::default(),
        clause: clause.identity(),
        current: None,
        pre: Some(Observation {
            identity: DocumentRef {
                authority: "test".into(),
                identity: "d10".into(),
                revision_namespace: "ns".into(),
                revision: "1".into(),
            },
            environment,
            populations: std::collections::BTreeMap::from([(HISTORY.to_owned(), true)]),
            out_of_range: vec![],
        }),
        post: None,
        self_object: reference,
        invocation: None,
        self_kind: qsl_semantics::model::observation::SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: vec![],
        result: None,
        created: vec![],
        deleted: vec![],
    };
    let mut meter = Meter::new(spine::default_accounting(1_000_000));
    let evaluation = package
        .evaluate_clause(
            &QualifiedName::unqualified("I").expect("qualified name"),
            &observations,
            &mut meter,
        )
        .expect("D10 evaluates");
    (
        format!("{:?}", evaluation.evaluation.outcome),
        meter.consumed(LimitKind::WorkUnits),
    )
}

#[trace("FR-480-AC-1")]
#[test]
fn split_pre_clauses_link_independently_of_request_order() {
    let forward = linked_clauses(false);
    let reverse = linked_clauses(true);
    assert_eq!(forward.len(), 2);
    assert_eq!(reverse.len(), 2);
    assert_eq!(forward[0].source().identity().identity.as_str(), "ac1:a");
    assert_eq!(forward[1].source().identity().identity.as_str(), "ac1:b");
    assert_eq!(reverse[0].source().identity().identity.as_str(), "ac1:a");
    assert_eq!(reverse[1].source().identity().identity.as_str(), "ac1:b");
    let forward_bytes = qsl_package::emit_checked(forward[0].package())
        .expect("forward aggregate emits")
        .package()
        .bytes()
        .to_vec();
    let reverse_bytes = qsl_package::emit_checked(reverse[0].package())
        .expect("reverse aggregate emits")
        .package()
        .bytes()
        .to_vec();
    assert_eq!(
        forward_bytes, reverse_bytes,
        "canonical package bytes are order-independent"
    );
}

#[trace(
    "QSpec-TC-196",
    "QSpec-TC-1210",
    "FR-120-AC-19",
    "FR-120-AC-20",
    "FR-480-AC-1"
)]
#[test]
fn split_d10_clauses_match_the_single_order_oracle_in_every_link_order() {
    for swapped in [false, true] {
        let single_unit_oracle = run_d10(&d10_single(swapped));
        let mut placement_oracle = None;
        for split_pb1 in [false, true] {
            for reverse in [false, true] {
                let checked = d10(swapped, split_pb1, reverse);
                let first = run_d10(&checked[0]);
                let opposite = d10(swapped, split_pb1, !reverse);
                let second = run_d10(&opposite[0]);
                assert_eq!(first, second, "link order changed D10 result/charges");
                assert_eq!(
                    first, single_unit_oracle,
                    "split D10 differs from its single-unit result/charges"
                );
                let first_emission =
                    qsl_package::emit_checked(checked[0].package()).expect("D10 package emits");
                let second_emission = qsl_package::emit_checked(opposite[0].package())
                    .expect("D10 package emits in opposite order");
                let first_wire: Value = serde_json::from_slice(first_emission.package().bytes())
                    .expect("canonical package wire is JSON");
                let second_wire: Value = serde_json::from_slice(second_emission.package().bytes())
                    .expect("opposite-order package wire is JSON");
                assert_eq!(
                    first_emission.package().bytes(),
                    second_emission.package().bytes(),
                    "link order changed package bytes"
                );
                assert_eq!(
                    first_wire["lock"]["sources"],
                    json!([
                        {"authority": "test", "identity": "d10:u1"},
                        {"authority": "test", "identity": "d10:u2"}
                    ]),
                    "emitted source identities are canonical U1/U2 order"
                );
                assert_eq!(
                    first_wire["lock"]["sources"], second_wire["lock"]["sources"],
                    "request order changed emitted source list"
                );
                if !swapped && !split_pb1 {
                    let nodes = first_wire["semantic_graph"]["nodes"]
                        .as_array()
                        .expect("emitted nodes");
                    let shared_false = nodes
                        .iter()
                        .find(|node| {
                            node["node_tag"] == "value"
                                && node["semantic_form"] == "literal"
                                && node["body"]["value"] == false
                                && node["occurrences"].as_array().is_some()
                        })
                        .expect("PA and PB2 share the false expression node");
                    let occurrences = shared_false["occurrences"]
                        .as_array()
                        .expect("false expression occurrences");
                    assert_eq!(
                        occurrences,
                        json!((0..6)
                            .map(|ordinal| json!({"role": "expression", "ordinal": ordinal}))
                            .collect::<Vec<_>>())
                        .as_array()
                        .expect("expected occurrence array")
                    );
                    let node_id = &shared_false["node_id"];
                    let rows = first_wire["source_map"]
                        .as_array()
                        .expect("source-map rows")
                        .iter()
                        .filter(|row| row["node_id"] == *node_id)
                        .collect::<Vec<_>>();
                    assert_eq!(rows.len(), 6, "all false occurrences have source rows");
                    let sources = rows
                        .iter()
                        .map(|row| row["regions"][0]["source"]["identity"].as_str().unwrap())
                        .collect::<Vec<_>>();
                    assert_eq!(sources.first(), Some(&"d10:u1"));
                    let first_u1 = sources.iter().position(|source| *source == "d10:u1");
                    let first_u2 = sources.iter().position(|source| *source == "d10:u2");
                    assert!(
                        first_u1 < first_u2,
                        "shared false occurrence source rows must put U1 before U2: {sources:?}"
                    );
                }
                if let Some(expected) = &placement_oracle {
                    assert_eq!(
                        &first, expected,
                        "placement/request order changed D10 result/charges"
                    );
                } else {
                    placement_oracle = Some(first.clone());
                }
                let text = &first.0;
                if !swapped {
                    assert!(
                        text.contains("AbsentKey") || text.contains("absent-key"),
                        "expected absent-key: {text}"
                    );
                } else {
                    assert!(
                        text.contains("PreconditionFalse") || text.contains("precondition-false"),
                        "expected precondition false: {text}"
                    );
                }
            }
        }
    }
}

#[trace("QSpec-FR-151-AC-16", "FR-480-AC-3")]
#[test]
fn aggregate_dispatch_links_distinct_operations_across_model_aliases() {
    let distinct = linked("body otherB using m on N::ConfigVersion::other() { 2 }\n");
    assert!(
        distinct.is_ok(),
        "distinct operations under M/N link: {distinct:?}"
    );

    let refusal = linked("body duplicateB using m on N::ConfigVersion::size() { 2 }\n")
        .expect_err("a second body for the same resolved operation refuses");
    assert!(
        refusal.contains("DuplicateOperationBody") || refusal.contains("duplicate-declaration"),
        "expected FR-302 duplicate-declaration refusal, got {refusal}"
    );
}

/// A repeated alias with different package identities remains local to its
/// source unit: each operation body resolves through that unit's model.
#[trace("FR-480-AC-3")]
#[test]
fn aggregate_resolves_a_reused_model_alias_per_source_unit() {
    const OTHER_PACKAGE: &str = "example/alternate-config-version";
    let first_document = model_document();
    let second_document = String::from_utf8(model_document())
        .expect("model JSON is UTF-8")
        .replace(PACKAGE, OTHER_PACKAGE)
        .replace("ConfigVersion", "ConfigV2")
        .into_bytes();
    let packages = package_input([first_document.as_slice(), second_document.as_slice()])
        .expect("both model packages admit");
    let source_a = SourceIdentity::new("test", "alias-a", "a", "1");
    let source_b = SourceIdentity::new("test", "alias-b", "b", "1");
    let a = parse_unit_for_package(
        &source_a,
        "M",
        PACKAGE,
        "body sizeA using m on M::ConfigVersion::size() { 1 }\n",
    );
    let b = parse_unit_for_package(
        &source_b,
        "M",
        OTHER_PACKAGE,
        "body otherB using m on M::ConfigV2::other() { 2 }\n",
    );
    let dependencies = DependencyInput::default();
    let admitted = admit_parsed_unit_list(vec![b, a], &dependencies).expect("units admit");
    let models = select_units(
        &admitted,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("each source-local M selects its package")
    .into_value();
    assert_eq!(models.models().len(), 2, "both identities remain selected");
    let checked = check_units(
        &admitted,
        "source-local-aliases".to_owned(),
        &models,
        &dependencies,
        spine::SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("each source's M resolves to its own model identity")
    .into_value();
    assert_eq!(checked.len(), 2);

    let duplicate = parse_unit_for_package(
        &source_a,
        "M",
        PACKAGE,
        "model M = \"example/alternate-config-version\";\ntype Dummy = Integer;\n",
    );
    let duplicate_list = admit_parsed_unit_list(vec![duplicate], &dependencies)
        .expect("duplicate aliases with distinct identities parse");
    let duplicate_models = select_units(
        &duplicate_list,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("both same-unit package identities admit")
    .into_value();
    let refusal = check_units(
        &duplicate_list,
        "same-unit-duplicate-alias".to_owned(),
        &duplicate_models,
        &dependencies,
        spine::SpineLimits::default(),
        &Cancel::new(),
    )
    .expect_err("a same-unit duplicate model alias refuses");
    assert!(format!("{refusal:?}").contains("DuplicateAlias"));

    let prior_source = SourceIdentity::new("test", "alias-prior", "prior", "1");
    let repeated_source = SourceIdentity::new("test", "alias-repeated", "repeated", "1");
    let prior = parse_unit_for_package(&prior_source, "M", PACKAGE, "type Prior = Integer;\n");
    let repeated = parse_unit_for_package(
        &repeated_source,
        "M",
        PACKAGE,
        "model M = \"example/alternate-config-version\";\ntype Repeated = Integer;\n",
    );
    let units = admit_parsed_unit_list(vec![repeated, prior], &dependencies)
        .expect("cross-unit repeated model selections admit for assembly");
    let models = select_units(
        &units,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("both model identities admit before alias validation")
    .into_value();
    let refusal = check_units(
        &units,
        "cross-unit-dedup-preserves-local-duplicate".to_owned(),
        &models,
        &dependencies,
        spine::SpineLimits::default(),
        &Cancel::new(),
    )
    .expect_err("a later unit's M=I then M=J selections remain a duplicate");
    assert!(format!("{refusal:?}").contains("DuplicateAlias"));
}

/// Import aliases are scoped to their declaring source unit: each unit's
/// \x60m::f()\x60 resolves against its own selected library, even when the aliases
/// name different function signatures.
#[trace("FR-480-AC-3")]
#[test]
fn aggregate_resolves_a_reused_import_alias_per_source_unit() {
    let header =
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
    let library = |identity: &str, name: &str, declaration: &str| SuppliedLibrary {
        identity: identity.to_owned(),
        source: SourceIdentity::new("test-library", name, "git", "1"),
        path: format!("{name}.native"),
        bytes: format!("{header}{declaration}").into_bytes(),
    };
    let dependencies = DependencyInput::new([
        library(
            "example/boolean-lib",
            "boolean-lib",
            "function f using v(x: Integer): Boolean pure { true }\n",
        ),
        library(
            "example/integer-lib",
            "integer-lib",
            "function f using v(x: Integer): Integer pure { x }\n",
        ),
    ])
    .expect("distinct library identities admit");
    let source_a = SourceIdentity::new("test", "import-a", "a", "1");
    let source_b = SourceIdentity::new("test", "import-b", "b", "1");
    let parse = |source: &SourceIdentity, library: &str, body: &str| {
        let text = format!("{header}import \"{library}\" as m;\n{body}");
        spine::parse(
            &ParseRequest {
                source,
                path: "unit.native",
                bytes: text.as_bytes(),
            },
            SourceLimits::default(),
            &Cancel::new(),
        )
        .expect("importing unit parses")
        .into_value()
    };
    let a = parse(
        &source_a,
        "example/boolean-lib",
        "function a using v(): Boolean pure { m::f(1) }\n",
    );
    let b = parse(
        &source_b,
        "example/integer-lib",
        "function b using v(): Integer pure { m::f(1) }\n",
    );
    let admitted = admit_parsed_unit_list(vec![b, a.clone()], &dependencies)
        .expect("source units admit in either request order");
    let packages = package_input(std::iter::empty::<&[u8]>()).expect("empty model input");
    let models = select_units(
        &admitted,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("units select with no models")
    .into_value();
    let checked = check_units(
        &admitted,
        "source-local-imports".to_owned(),
        &models,
        &dependencies,
        spine::SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("each m::f resolves against its declaring unit's import")
    .into_value();
    assert_eq!(checked.len(), 2);

    let source_without_import = SourceIdentity::new("test", "import-none", "none", "1");
    let no_import = spine::parse(
        &ParseRequest {
            source: &source_without_import,
            path: "unit.native",
            bytes: format!("{header}function c using v(): Integer pure {{ m::f(1) }}\n").as_bytes(),
        },
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("unit without an import parses")
    .into_value();
    let admitted =
        admit_parsed_unit_list(vec![a, no_import], &dependencies).expect("source units admit");
    let models = select_units(
        &admitted,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("units select with no models")
    .into_value();
    let refusal = check_units(
        &admitted,
        "source-local-import-no-leak".to_owned(),
        &models,
        &dependencies,
        spine::SpineLimits::default(),
        &Cancel::new(),
    )
    .expect_err("unit B cannot use unit A's import alias");
    let refusal = refusal
        .into_refused()
        .expect("the alias leakage case stops with a refusal");
    let spine::CompileRefusal::Check { refusals, .. } = *refusal else {
        panic!("expected an unbound qualified-name refusal, got {refusal:?}");
    };
    assert!(refusals.iter().any(|refusal| matches!(
        &refusal.cause,
        qsl_semantics::check::CheckCause::MissingQualifiedName(name) if name.name == "m::f"
    )));
}

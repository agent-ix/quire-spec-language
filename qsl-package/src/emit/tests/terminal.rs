// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-411 (TC-1142): a checked model `terminal` declaration emits as QSpec
//! FR-476's `temporal`/`terminal` node, QSL's I2 read admits it and reads the
//! declaration back. The subject is the
//! ConfigVersion domain package, whose one object type `ConfigVersion` is the
//! type its one population ranges over.

use quire_contract_model::CheckedPackageRefusalCause as Cause;
use quire_contract_model::CheckedPackageRefusalCode as RefusalCode;

use super::admission_corpus::config_version_unit;
use super::*;
use crate::checked_v2::{read_terminal, read_terminals, ReadTerminalDeclaration, V2ReadRefusal};

const WHEN: &str = "terminal Done using v on Config when { self.versionNumber = 3 };";
/// A declaration other than a `terminal`, so the unit is not empty.
const NONE: &str = "function noop using v(): Boolean pure { true }";
const ANY: &str = "terminal Halt using v on Config any;";

fn document() -> &'static [u8] {
    config_version_unit::DOMAIN_PACKAGE.as_bytes()
}

/// A unit selecting the ConfigVersion model, then `declarations`.
fn unit(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model Config = \"example/config-version\";\n\
         {declarations}\n"
    )
}

struct Compiled {
    text: String,
    emission: Emission,
    wire: Value,
}

fn compile(declarations: &str) -> Compiled {
    let text = unit(declarations);
    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
    let graph = assemble_with_models(text.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    Compiled {
        text,
        emission,
        wire,
    }
}

fn evidence(wire: &Value) -> CheckedPackageEvidence {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(document()), document().to_vec())
        .expect("a fresh evidence key");
    evidence
}

fn read_envelope(envelope: &Value, wire: &Value) -> Read {
    read_v2(
        &jcs(envelope),
        library(),
        V2ReadLimits::default(),
        &evidence(wire),
    )
}

/// QSL's I2 read of the emitted bytes: IR's admitted package.
fn admit(compiled: &Compiled) -> Arc<quire_contract_model::CheckedPackageV2> {
    crate::checked_v2::read_checked_package_v2(
        compiled.emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence(&compiled.wire),
    )
    .unwrap_or_else(|failure| panic!("the I2 read admits the package: {failure:?}"))
    .into_value()
    .admitted
}

fn terminal_nodes(wire: &Value) -> Vec<&Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["semantic_form"] == "terminal")
        .collect()
}

fn terminal_node(wire: &Value) -> &Value {
    only_node(wire, "terminal")
}

fn binding<'v>(body: &'v Value, name: &str) -> &'v Value {
    body["members"]
        .as_array()
        .expect("an aggregate")
        .iter()
        .find(|member| member["name"] == name)
        .map(|member| &member["value"])
        .unwrap_or_else(|| panic!("a binding {name} in {body}"))
}

fn target(term: &Value) -> &Value {
    assert_eq!(term["term"], "reference", "{term}");
    &term["target"]
}

fn node_with<'w>(wire: &'w Value, key: &Value) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == key)
        .unwrap_or_else(|| panic!("a node {key}"))
}

fn key(value: &Value) -> NodeKey {
    qsl_foundation::node_wire::node_key_from_json(value).expect("a structural node key")
}

/// FR-411-AC-1: `when` emits one `temporal`/`terminal` node with its four
/// bindings, `parameters` one reference to the level-0 `self` parameter,
/// `condition` a reference to the predicate's `expression` node, no
/// `declaration` and one `claim` occurrence at the declaration; `any` emits
/// empty `parameters` and `condition`; no declaration emits no node. The I2
/// read returns `When`, `Any` and `None`.
#[trace("TC-1142", "FR-411-AC-1")]
#[test]
fn a_terminal_declaration_emits_its_node_and_the_i2_read_returns_it() {
    let when = compile(WHEN);
    let node = terminal_node(&when.wire);
    assert_eq!(node["node_tag"], "temporal");
    assert!(node.get("declaration").is_none_or(Value::is_null));
    let body = &node["body"];
    let names: Vec<&str> = body["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|member| member["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["model", "mode", "parameters", "condition"]);
    let model = node_with(&when.wire, target(binding(body, "model")));
    assert_eq!(model["node_tag"], "model");
    assert_eq!(binding(body, "mode")["value"], "when");
    let parameters = binding(body, "parameters")["members"].as_array().unwrap();
    let [parameter] = parameters.as_slice() else {
        panic!("one parameter: {parameters:?}");
    };
    let parameter = node_with(&when.wire, target(parameter));
    assert_eq!(parameter["semantic_form"], "parameter");
    assert_eq!(binding(&parameter["body"], "name")["value"], "self");
    assert_eq!(binding(&parameter["body"], "level")["value"], "0");
    let condition = node_with(&when.wire, target(binding(body, "condition")));
    assert_eq!(condition["node_tag"], "expression");
    // The condition is the predicate's own `self.versionNumber = 3`: an
    // integer equality over a read of `self` and the literal 3.
    assert_eq!(
        condition["body"]["operation"]["identity"],
        "quire.op.integer.eq"
    );
    let [read, literal] = condition["body"]["arguments"]
        .as_array()
        .unwrap()
        .as_slice()
    else {
        panic!("two operands: {condition}");
    };
    let read = node_with(&when.wire, target(read));
    assert_eq!(
        read["body"]["operation"]["identity"],
        "quire.op.record.project"
    );
    assert_eq!(read["body"]["operation"]["member"]["name"], "versionNumber");
    assert_eq!(
        node_with(&when.wire, target(literal))["semantic_form"],
        "conversion"
    );
    // `self` is typed `Reference<T>` for the model's one object type, which is
    // the node the terminal's `model` binding names.
    let self_type = node_with(&when.wire, &parameter["semantic_type"]);
    assert_eq!(self_type["node_tag"], "composite_type");
    assert_eq!(self_type["semantic_form"], "reference");
    assert_eq!(target(&self_type["body"]["members"][0]), &model["node_id"]);
    let claims: Vec<&Value> = when.wire["source_map"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["node_id"] == node["node_id"] && entry["role"] == "claim")
        .collect();
    let [claim] = claims[..] else {
        panic!("one claim occurrence: {claims:?}");
    };
    let region = &claim["regions"][0];
    let start = usize::try_from(region["start"].as_u64().unwrap()).unwrap();
    let end = usize::try_from(region["end"].as_u64().unwrap()).unwrap();
    assert_eq!(&when.text[start..end], WHEN);

    let any = compile(ANY);
    let node = terminal_node(&any.wire);
    assert_eq!(binding(&node["body"], "mode")["value"], "any");
    assert_eq!(binding(&node["body"], "parameters")["members"], json!([]));
    assert_eq!(binding(&node["body"], "condition")["members"], json!([]));

    let none = compile(NONE);
    assert_eq!(terminal_nodes(&none.wire).len(), 0);

    let model_of = |compiled: &Compiled| {
        let package = admit(compiled);
        let terminals = read_terminals(&package).expect("the terminals read");
        let model = terminals.first().map(|terminal| terminal.model.clone());
        (package, terminals, model)
    };
    let (package, terminals, model) = model_of(&when);
    let model = model.expect("one terminal");
    let [read] = &terminals[..] else {
        panic!("one terminal: {terminals:?}");
    };
    assert_eq!(read.node, key(&terminal_node(&when.wire)["node_id"]));
    assert_eq!(
        read.declaration,
        ReadTerminalDeclaration::When {
            parameter: Some(key(target(&parameters[0]))),
            condition: key(target(binding(
                &terminal_node(&when.wire)["body"],
                "condition"
            ))),
        }
    );
    assert_eq!(
        read_terminal(&package, &model),
        Ok(read.declaration.clone())
    );

    let (package, terminals, any_model) = model_of(&any);
    let [read] = &terminals[..] else {
        panic!("one terminal: {terminals:?}");
    };
    assert_eq!(read.declaration, ReadTerminalDeclaration::Any);
    assert_eq!(
        read_terminal(&package, &any_model.expect("one terminal")),
        Ok(ReadTerminalDeclaration::Any)
    );

    let package = admit(&none);
    assert_eq!(read_terminals(&package), Ok(Vec::new()));
    assert_eq!(
        read_terminal(&package, &model),
        Ok(ReadTerminalDeclaration::None)
    );
}

/// QSL's I2 read of `compiled`'s wire after `edit` changes the terminal node,
/// with `dependencies` set to the body's targets: IR's refusal.
fn refusal_after(
    compiled: &Compiled,
    edit: impl FnOnce(&mut Value),
) -> quire_contract_model::CheckedPackageRefusal {
    let mut envelope = compiled.wire.clone();
    let node = envelope["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|node| node["semantic_form"] == "terminal")
        .expect("the terminal node");
    edit(node);
    let mut targets: Vec<Value> = node["body"]["members"]
        .as_array()
        .expect("an aggregate")
        .iter()
        .flat_map(|member| {
            let value = &member["value"];
            match value["term"].as_str() {
                Some("reference") => vec![value["target"].clone()],
                _ => value["members"]
                    .as_array()
                    .map(|members| members.iter().map(|m| m["target"].clone()).collect())
                    .unwrap_or_default(),
            }
        })
        .collect();
    targets.sort_by_key(|value| qsl_foundation::node_wire::node_key_from_json(value).unwrap());
    node["dependencies"] = Value::Array(targets);
    match read_envelope(&envelope, &compiled.wire) {
        Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) => *refusal,
        other => panic!("an IR refusal, not {other:?}"),
    }
}

/// FR-411-AC-4: fed to QSL's I2 read, the `when` node with `mode` `some`
/// refuses `invalid_package`/`invalid-value` at the `mode` binding, and with
/// `condition` the empty aggregate refuses `invalid_semantic_graph` at the
/// `condition` binding.
#[trace("TC-1142", "FR-411-AC-4")]
#[test]
fn the_i2_read_refuses_a_malformed_terminal_node_at_its_locus() {
    let compiled = compile(WHEN);
    let position = nodes(&compiled.wire)
        .iter()
        .position(|node| node["semantic_form"] == "terminal")
        .expect("the terminal node");
    let body = format!("/semantic_graph/nodes/{position}/body/members");

    let some = refusal_after(&compiled, |node| {
        node["body"]["members"][1]["value"]["value"] = json!("some");
    });
    assert_eq!(some.code, RefusalCode::InvalidPackage);
    assert_eq!(some.cause, Some(Cause::InvalidValue));
    assert_eq!(
        some.loci[0].path.as_ref().map(|path| path.as_str()),
        Some(&*format!("{body}/1"))
    );

    let empty = refusal_after(&compiled, |node| {
        node["body"]["members"][3]["value"] = json!({"term": "aggregate", "members": []});
    });
    assert_eq!(empty.code, RefusalCode::InvalidSemanticGraph);
    assert_eq!(
        empty.loci[0].path.as_ref().map(|path| path.as_str()),
        Some(&*format!("{body}/3"))
    );
}

/// FR-382-AC-6, second half (TC-1059): with the model alias naming a model,
/// each of the `when` and `any` units checks and its package holds the
/// `temporal`/`terminal` node.
#[trace("TC-1059", "FR-382-AC-6")]
#[test]
fn a_checked_terminal_declaration_is_held_by_the_package_as_its_node() {
    for (declaration, mode) in [(WHEN, "when"), (ANY, "any")] {
        let compiled = compile(declaration);
        let node = terminal_node(&compiled.wire);
        assert_eq!(binding(&node["body"], "mode")["value"], mode);
    }
}

/// FR-411-AC-1: a package's terminal declarations resolve through its emitted
/// nodes: the mode is the node's, the declaration is found by the node's
/// `claim` occurrence, and a model with no node has none.
#[trace("TC-1142", "FR-411-AC-1")]
#[test]
fn a_packages_terminals_resolve_through_their_emitted_nodes() {
    use crate::checked_v2::{resolve_terminals, PackageTerminalDeclaration};

    let resolved = |declaration: &str| {
        let text = unit(declaration);
        let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
        let graph = assemble_with_models(text.as_bytes(), &packages)
            .expect("the unit assembles")
            .check(CheckingLimits::default())
            .expect("the package checks");
        let package = CheckedPackage::link(graph);
        let terminals = resolve_terminals(&package)
            .expect("the terminals resolve")
            .into_iter()
            .map(|terminal| {
                (
                    terminal.model.to_owned(),
                    terminal.name.to_owned(),
                    matches!(terminal.declaration, PackageTerminalDeclaration::Any),
                )
            })
            .collect::<Vec<_>>();
        terminals
    };
    assert_eq!(
        resolved(WHEN),
        [("Config".to_owned(), "Done".to_owned(), false)]
    );
    assert_eq!(
        resolved(ANY),
        [("Config".to_owned(), "Halt".to_owned(), true)]
    );
    assert_eq!(resolved(NONE), []);
}

/// FR-411-AC-1: resolution follows the node. With the declaration's node
/// taken out of the graph while the S3 value still holds the declaration, the
/// terminals do not resolve, naming the model; with the node present they do.
#[trace("TC-1142", "FR-411-AC-1")]
#[test]
fn resolution_follows_the_node_not_the_checked_declaration() {
    use crate::checked_v2::{resolve_terminals, TerminalResolveRefusal};

    let packages = qsl_semantics::model::intake::package_input([document()]).unwrap();
    let checked = || {
        assemble_with_models(unit(WHEN).as_bytes(), &packages)
            .expect("the unit assembles")
            .check(CheckingLimits::default())
            .expect("the package checks")
    };
    let mut graph = checked();
    let [node] = &graph.terminal_nodes()[..] else {
        panic!("one terminal node");
    };
    assert_eq!(
        resolve_terminals(&CheckedPackage::link(checked()))
            .expect("resolves")
            .len(),
        1
    );
    assert!(graph
        .semantic_graph_mut()
        .remove(node.node.clone())
        .is_some());
    assert_eq!(
        resolve_terminals(&CheckedPackage::link(graph)).err(),
        Some(TerminalResolveRefusal::Disagrees {
            model: "Config".to_owned()
        })
    );
}

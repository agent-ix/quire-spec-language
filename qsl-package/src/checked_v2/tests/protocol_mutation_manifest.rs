// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec's executable protocol-node mutation manifest and layout rows, replayed
//! in place through QSL's I2 reader (FR-383-AC-20, TC-1063).
//!
//! Neither file, nor the base fixture they edit, is copied into QSL: each row's
//! RFC 6902 edit is applied to the base read from the QSpec checkout, and the
//! edited package is read by [`read_v2`](super::read_v2).

use super::*;
use std::collections::BTreeSet;

fn fixtures_directory() -> std::path::PathBuf {
    qspec_directory()
        .expect("QSPEC_DIR must name a quire-specification checkout")
        .join("proposals/checked-package-v2/fixtures")
}

fn unescape(segment: &str) -> String {
    segment.replace("~1", "/").replace("~0", "~")
}

/// Applies the RFC 6902 `add`, `remove` and `replace` operations a row uses.
fn apply(id: &str, base: &Value, edit: &[Value]) -> Value {
    let mut package = base.clone();
    for op in edit {
        let path = op["path"].as_str().expect("an edit path");
        let (parent, last) = path.rsplit_once('/').expect("an absolute pointer");
        let last = unescape(last);
        let target = package
            .pointer_mut(parent)
            .unwrap_or_else(|| panic!("{id}: {path}: parent exists"));
        let operation = op["op"].as_str().expect("an edit op");
        match (operation, target) {
            ("remove", Value::Object(map)) => {
                map.remove(&last)
                    .unwrap_or_else(|| panic!("{id}: {path}: member exists"));
            }
            ("remove", Value::Array(items)) => {
                items.remove(last.parse::<usize>().expect("an index"));
            }
            ("replace", Value::Object(map)) => {
                assert!(map.contains_key(&last), "{id}: {path}: member exists");
                map.insert(last, op["value"].clone());
            }
            ("replace", Value::Array(items)) => {
                items[last.parse::<usize>().expect("an index")] = op["value"].clone();
            }
            ("add", Value::Object(map)) => {
                map.insert(last, op["value"].clone());
            }
            ("add", Value::Array(items)) => {
                let at = if last == "-" {
                    items.len()
                } else {
                    last.parse().expect("an index")
                };
                items.insert(at, op["value"].clone());
            }
            (other, _) => panic!("{id}: {path}: unsupported operation {other}"),
        }
    }
    package
}

/// The evidence the base is read against: the manifest's own domain document
/// (`acme/orders`) and the base's required features.
fn base_evidence(
    manifest: &Value,
    directory: &std::path::Path,
    base: &Value,
) -> CheckedPackageEvidence {
    let mut evidence = fixture_evidence(None, base, base, &[]);
    let document = manifest["base"]["domain_document"]
        .as_str()
        .expect("the base's domain document");
    let path = directory.join(document);
    let bytes =
        std::fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let decoded: Value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("parsing {}: {error}", path.display()));
    let identity = decoded["package"]["identity"]
        .as_str()
        .expect("the domain document's package identity")
        .to_owned();
    evidence
        .insert_domain_package_document(identity, bytes)
        .expect("one domain document");
    evidence
}

fn declaration_name(node: &Value) -> String {
    match node.get("declaration") {
        Some(declaration) => declaration["qualified_name"]
            .as_array()
            .expect("a qualified name")
            .iter()
            .map(|segment| segment.as_str().expect("a name segment"))
            .collect::<Vec<_>>()
            .join("::"),
        None => node["node_id"]["path"].to_string(),
    }
}

fn operator_of(node: &Value) -> &str {
    node["body"]["operation"]["member"]["operator"]
        .as_str()
        .unwrap_or_default()
}

fn argument<'a>(node: &'a Value, name: &str) -> Option<&'a Value> {
    node["body"]["arguments"]
        .as_array()?
        .iter()
        .find(|argument| argument["name"] == name)
        .map(|argument| &argument["value"])
}

fn node_named<'a>(nodes: &'a [Value], name: &str) -> Result<&'a Value, String> {
    nodes
        .iter()
        .find(|node| declaration_name(node) == name)
        .ok_or_else(|| format!("no node named {name}"))
}

fn parameter_level(nodes: &[Value], binder: &str) -> Option<u64> {
    nodes.iter().find_map(|node| {
        let path = node["node_id"]["path"].as_array()?;
        let named =
            path.first()? == "value" && path.get(1)? == "parameter" && path.get(4)? == binder;
        named.then(|| path.get(7)?.as_str()?.parse().ok()).flatten()
    })
}

fn has_cycle(edges: &[(String, String)]) -> bool {
    fn visit(
        at: &str,
        edges: &[(String, String)],
        active: &mut Vec<String>,
        done: &mut BTreeSet<String>,
    ) -> bool {
        if active.iter().any(|name| name == at) {
            return true;
        }
        if !done.insert(at.to_owned()) {
            return false;
        }
        active.push(at.to_owned());
        let cyclic = edges
            .iter()
            .filter(|(from, _)| from == at)
            .any(|(_, to)| visit(to, edges, active, done));
        active.pop();
        cyclic
    }
    let mut done = BTreeSet::new();
    edges
        .iter()
        .any(|(from, _)| visit(from, edges, &mut Vec::new(), &mut done))
}

fn edge_holds(relation: &str, from: &Value, to: &Value) -> bool {
    match relation {
        "message" => {
            operator_of(from) == "send"
                && operator_of(to) == "receive"
                && argument(from, "channel") == argument(to, "channel")
                && argument(to, "of").is_some_and(|of| of["target"] == from["node_id"])
        }
        "effect" => {
            operator_of(to) == "effect"
                && argument(to, "of").is_some_and(|of| of["target"] == from["node_id"])
        }
        "join" => {
            operator_of(to) == "parallel"
                && argument(to, "branches").is_some_and(|branches| {
                    branches.to_string().contains(&from["node_id"].to_string())
                })
        }
        _ => true,
    }
}

/// One property assertion over the nodes QSL's I2 reader returned; the
/// defect, when it fails.
fn check_assertion(assertion: &Value, nodes: &[Value], base_nodes: &[Value]) -> Result<(), String> {
    let kind = assertion["kind"].as_str().expect("an assertion kind");
    match kind {
        "admitted" => Ok(()),
        "package_byte_equal" => {
            let unchanged = base_nodes
                .iter()
                .zip(nodes)
                .all(|(before, after)| before == after);
            if base_nodes.len() == nodes.len() && unchanged {
                Ok(())
            } else {
                Err("the read-back package differs from the base".to_owned())
            }
        }
        "binder_levels" => {
            for level in assertion["levels"].as_array().expect("levels") {
                let binder = level["binder"].as_str().expect("a binder");
                let found = parameter_level(nodes, binder);
                if found != level["level"].as_u64() {
                    return Err(format!("binder {binder} level is {found:?}"));
                }
            }
            Ok(())
        }
        "runtime_subject_kinds" => {
            for entry in assertion["operators"].as_array().expect("operators") {
                let operator = entry["operator"].as_str().expect("an operator");
                if !nodes.iter().any(|node| operator_of(node) == operator) {
                    return Err(format!("no {operator} node in the read-back package"));
                }
            }
            Ok(())
        }
        "causal_edges" => {
            let mut graph = Vec::new();
            for edge in assertion["edges"].as_array().expect("edges") {
                let from_name = edge["from"]["node"].as_str().expect("an edge source");
                let to_name = edge["to"]["node"].as_str().expect("an edge target");
                let from = node_named(nodes, from_name)?;
                let to = node_named(nodes, to_name)?;
                let relation = edge["relation"].as_str().expect("a relation");
                if !edge_holds(relation, from, to) {
                    return Err(format!(
                        "{relation} edge {from_name} to {to_name} is absent"
                    ));
                }
                graph.push((from_name.to_owned(), to_name.to_owned()));
            }
            if assertion["acyclic"] == true && has_cycle(&graph) {
                return Err("the causal edges are cyclic".to_owned());
            }
            Ok(())
        }
        "activation_relation" => {
            for row in assertion["rows"].as_array().expect("rows") {
                let clause = node_named(nodes, row["clause"].as_str().expect("a clause"))?;
                let activation = argument(clause, "activation")
                    .and_then(|value| value["members"].as_array())
                    .ok_or("a clause without an activation")?;
                let named = |name: &str| {
                    activation
                        .iter()
                        .find(|member| member["name"] == name)
                        .map(|member| member["value"].clone())
                };
                let activation_kind = named("kind").map(|kind| kind["value"].clone());
                if activation_kind != Some(row["activation"].clone()) {
                    return Err("activation kind differs".to_owned());
                }
                let temporal = node_named(
                    nodes,
                    row["temporal_clause"].as_str().expect("a temporal clause"),
                )?;
                let required = argument(clause, "requirements").is_some_and(|value| {
                    value.to_string().contains(&temporal["node_id"].to_string())
                });
                let trigger_matches = match row.get("trigger").and_then(Value::as_str) {
                    Some(trigger) => {
                        let node = node_named(nodes, trigger)?;
                        named("trigger").is_some_and(|value| value["target"] == node["node_id"])
                    }
                    None => named("trigger").is_none(),
                };
                if !required || !trigger_matches {
                    return Err("trigger or requirement differs".to_owned());
                }
            }
            Ok(())
        }
        other => Err(format!("unknown assertion kind {other}")),
    }
}

fn pointer_of(location: &quire_contract_model::CheckedPackageRefusalLocation) -> Option<&str> {
    location.path.as_ref().map(|path| path.as_str())
}

/// A schema refusal: `malformed_wire`, no cause, the first error's path as the
/// locus and no `document_pointer`, at the schema stage.
fn check_wire_refusal(expect: &Value, path: &str, all: &[String]) -> Result<(), String> {
    if expect["stage"] != "schema" || expect["code"] != "malformed_wire" {
        return Err(format!(
            "the schema refused first at {path}, but the row expects {} at stage {}",
            expect["code"], expect["stage"]
        ));
    }
    if !expect["cause"].is_null() || expect.get("document_pointer").is_some() {
        return Err("a wire refusal has no cause and no document_pointer".to_owned());
    }
    if expect["locus"].as_str() != Some(path) {
        let held = expect["locus"]
            .as_str()
            .is_some_and(|locus| all.iter().any(|error| error == locus));
        let why = if held {
            "the expected locus is a later schema error: this validator orders errors differently from the manifest's"
        } else {
            "the expected locus is not among this validator's schema errors"
        };
        return Err(format!(
            "wire locus {path:?}, expected {} ({why})",
            expect["locus"]
        ));
    }
    Ok(())
}

/// What the row expects of a refusal, against the reader's: exact code,
/// cause, locus (and `document_pointer` where the row gives one), with the
/// row's stage telling a wire refusal from a reader one.
fn check_refusal(expect: &Value, outcome: &Read, edited: &Value) -> Result<(), String> {
    let Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) = outcome else {
        return Err(format!("expected a refusal, read {outcome:?}"));
    };
    let wire = refusal.code == CheckedPackageRefusalCode::MalformedWire;
    let stage = expect["stage"].as_str().expect("a stage");
    if wire != (stage == "schema") {
        return Err(format!(
            "stage {stage}, but the reader refused {:?} (an earlier or later step answered)",
            refusal.code
        ));
    }
    let code = serde_json::to_value(refusal.code).expect("a code value");
    if code != expect["code"] {
        return Err(format!("code {code}, expected {}", expect["code"]));
    }
    let cause = serde_json::to_value(refusal.cause).expect("a cause value");
    if cause != expect["cause"] {
        return Err(format!("cause {cause}, expected {}", expect["cause"]));
    }
    let primary = refusal.loci.first().and_then(pointer_of);
    if primary != expect["locus"].as_str() {
        return Err(format!(
            "locus {primary:?} (document_pointer {:?}), expected {}",
            refusal
                .document_pointer
                .as_ref()
                .map(|pointer| pointer.as_str()),
            expect["locus"]
        ));
    }
    if let Some(pointer) = expect.get("document_pointer") {
        let found = refusal
            .document_pointer
            .as_ref()
            .map(|pointer| pointer.as_str());
        if found != pointer.as_str() {
            return Err(format!("document_pointer {found:?}, expected {pointer}"));
        }
    }
    if let Some(retains) = expect.get("retains") {
        check_retained(retains, refusal, edited)?;
    }
    Ok(())
}

/// The refusal retains the node it refused (its `locus`): the row's actual
/// form and key are that node's.
fn check_retained(
    retains: &Value,
    refusal: &quire_contract_model::CheckedPackageRefusal,
    edited: &Value,
) -> Result<(), String> {
    let retained = serde_json::to_value(&refusal.locus).expect("a retained node id");
    let node = edited["semantic_graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["node_id"] == retained))
        .ok_or_else(|| format!("the refusal retains no node of the package ({retained})"))?;
    if let Some(kind) = retains.get("actual_kind") {
        if node["semantic_form"] != *kind {
            return Err(format!(
                "retained form {}, expected {kind}",
                node["semantic_form"]
            ));
        }
    }
    if let Some(key) = retains.get("actual_key").and_then(Value::as_str) {
        if declaration_name(node) != key {
            return Err(format!(
                "retained key {}, expected {key}",
                declaration_name(node)
            ));
        }
    }
    Ok(())
}

#[derive(Default)]
struct Executed {
    admit: usize,
    refuse: usize,
    property: usize,
}

/// Replays one row; the defect, when the reader's answer is not the row's.
fn replay_row(
    row: &Value,
    base: &Value,
    manifest: &Value,
    directory: &std::path::Path,
    schema: &jsonschema::JSONSchema,
    executed: &mut Executed,
) -> Result<(), String> {
    let id = row["id"].as_str().expect("a row id");
    assert_eq!(
        row["base"], manifest["base"]["fixture"],
        "{id}: the row edits the manifest's base"
    );
    let edit = row["edit"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let edited = apply(id, base, edit);
    // The published schema decides the wire first: a package it refuses is
    // reported `malformed_wire` at the first error's path and never reaches
    // the reader, whose later checks cover the shapes the schema admits.
    let wire_errors: Vec<String> = schema
        .validate(&edited)
        .err()
        .map(|errors| {
            errors
                .map(|error| error.instance_path.to_string())
                .collect()
        })
        .unwrap_or_default();
    let wire_error = wire_errors.first().cloned();
    let outcome = match &wire_error {
        Some(_) => None,
        None => {
            let evidence = base_evidence(manifest, directory, &edited);
            let name = edited["name"].as_str().expect("a package name");
            Some(read_v2(
                &fixture_wire_bytes(&edited),
                identity(name),
                V2ReadLimits::default(),
                &evidence,
            ))
        }
    };
    let expect = &row["expect"];
    match row["class"].as_str().expect("a class") {
        "admit" => {
            executed.admit += 1;
            match outcome {
                Some(Read::Verified { .. }) => Ok(()),
                other => Err(format!(
                    "expected admission, read {other:?} (schema {wire_error:?})"
                )),
            }
        }
        "refuse" => {
            executed.refuse += 1;
            match (&outcome, &wire_error) {
                (None, Some(path)) => check_wire_refusal(expect, path, &wire_errors),
                (Some(outcome), _) => check_refusal(expect, outcome, &edited),
                (None, None) => Err("neither the schema nor the reader answered".to_owned()),
            }
        }
        "property" => {
            executed.property += 1;
            let Some(Read::Verified { admitted, .. }) = outcome else {
                return Err(format!("expected admission, read {outcome:?}"));
            };
            let nodes: Vec<Value> = admitted
                .graph()
                .nodes
                .iter()
                .map(|node| serde_json::to_value(node).expect("a node value"))
                .collect();
            let base_nodes = base["semantic_graph"]["nodes"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default();
            for assertion in expect["assertions"].as_array().expect("assertions") {
                check_assertion(assertion, &nodes, base_nodes)
                    .map_err(|defect| format!("{}: {defect}", assertion["kind"]))?;
            }
            Ok(())
        }
        other => Err(format!("class {other} is not executable")),
    }
}

/// A `pending` entry runs through its same-id layout row, `CH14a` through
/// `CH14a-1` to `CH14a-8`; a `later` entry has no row here.
fn pending_is_executed(id: &str, layout_ids: &BTreeSet<&str>) -> bool {
    layout_ids.contains(id)
        || (1..=8).all(|index| layout_ids.contains(format!("{id}-{index}").as_str()))
}

/// QSpec's protocol-node mutation manifest and layout rows, read in place and
/// replayed through QSL's I2 reader. Each `admit` row is admitted, each
/// `refuse` row is refused with exactly its code, cause and locus (a wire
/// refusal does not satisfy a later semantic row), each `property` row's
/// assertions hold over the read-back package; every class runs a nonzero
/// number of rows and every row of both files is executed.
///
/// The wire locus of a schema-stage row is the first error of the published
/// schema. The manifest fixed those loci with `jsonschema` 0.55.0 and this
/// test validates with 0.17.1, so first-error order is validator specific: a
/// mismatch says whether the expected locus is a later error (order differs)
/// or no error at all (the validators disagree).
#[trace("FR-383-AC-20", "TC-1063")]
#[test]
fn conformance_protocol_mutation_manifest_replays_through_i2() {
    let directory = fixtures_directory();
    let manifest = read_fixture(&directory.join("protocol-node-mutation-manifest.json"));
    let layout = read_fixture(&directory.join("protocol-node-layout-rows.json"));
    assert_eq!(
        manifest["version"],
        "quire.protocol-node-mutation-manifest/v1"
    );
    let base = read_fixture(&directory.join(manifest["base"]["fixture"].as_str().expect("a base")));
    let manifest_rows = manifest["rows"].as_array().expect("manifest rows");
    let layout_rows = layout["rows"].as_array().expect("layout rows");
    let schema_document = read_fixture(&directory.join("../schema.json"));
    let schema = jsonschema::JSONSchema::compile(&schema_document)
        .unwrap_or_else(|error| panic!("the published schema compiles: {error}"));
    let mut executed = Executed::default();
    let mut defects = Vec::new();
    for row in manifest_rows.iter().chain(layout_rows) {
        if let Err(defect) = replay_row(row, &base, &manifest, &directory, &schema, &mut executed) {
            defects.push(format!("{}: {defect}", row["id"]));
        }
    }
    let total = executed.admit + executed.refuse + executed.property;
    let layout_ids: BTreeSet<&str> = layout_rows
        .iter()
        .filter_map(|row| row["id"].as_str())
        .collect();
    let pending = manifest["pending"].as_array().expect("pending entries");
    let pending_without_row = pending
        .iter()
        .filter(|entry| {
            !pending_is_executed(entry["id"].as_str().expect("a pending id"), &layout_ids)
        })
        .count();
    let later = manifest["later"].as_array().expect("later entries").len();
    println!(
        "conformance: protocol mutation manifest executed {} admit, {} refuse, {} property rows \
         ({} manifest rows + {} layout rows); not executed (no executable row): {later} later, \
         {pending_without_row} of {} pending",
        executed.admit,
        executed.refuse,
        executed.property,
        manifest_rows.len(),
        layout_rows.len(),
        pending.len()
    );
    assert!(
        executed.admit > 0 && executed.refuse > 0 && executed.property > 0,
        "every class executes a row"
    );
    assert_eq!(
        total,
        manifest_rows.len() + layout_rows.len(),
        "every executable row is executed"
    );
    assert_eq!(
        (executed.admit, executed.refuse, executed.property, total),
        (25, 90, 4, 119),
        "the executed set FR-383-AC-20 states"
    );
    assert_eq!(
        pending_without_row, 0,
        "every pending entry runs through its layout row"
    );
    assert!(
        defects.is_empty(),
        "{} rows differ:\n{}",
        defects.len(),
        defects.join("\n")
    );
}

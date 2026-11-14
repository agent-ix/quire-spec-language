// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1011 (FR-576-AC-1 to AC-9): the emission carries, for every operation
//! FR-496 keys, the checked body function node of an operation with an
//! `operation-body` or the explicit absence `no_body`, whether or not a
//! dispatched call reaches the body. The model is the
//! `example/config-version` package with `ConfigVersion` (`A`) and the
//! subtypes the cases need.

use quire_contract_model::CheckedNodeId;

use super::*;
use crate::OperationBodyRef;

const PACKAGE: &str = "example/config-version";
const EXAMPLE: &str = include_str!("../../../../examples/config-version/model.semantic-ir.json");

fn query(owner: &str, name: &str, redefines: Option<&str>) -> Value {
    let identity = format!("ix://{PACKAGE}/{owner}/{name}");
    let mut operation = json!({
        "identity": identity,
        "name": name,
        "params": [],
        "returns": {
            "typeRef": "ix://quire/native/Integer",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false,
        },
        "pre": [],
        "post": [],
        "origin": {"source": {
            "sourceIdentity": format!("ix://{PACKAGE}/spec"),
            "path": "spec.qspec",
            "startLine": 1,
            "startColumn": 1,
        }},
        "frame": {"modifies": [], "creates": [], "deletes": []},
    });
    if let Some(parent) = redefines {
        operation["redefines"] = json!(format!("ix://{PACKAGE}/{parent}/{name}"));
    }
    operation
}

/// `ConfigVersion` declaring `operations`, and each of `subtypes` as
/// `(name, supertype, operations)`; an empty supertype declares none.
fn package(operations: Vec<Value>, subtypes: &[(&str, &str, Vec<Value>)]) -> Vec<u8> {
    let mut envelope: Value = serde_json::from_str(EXAMPLE).expect("the example parses");
    let types = envelope["types"].as_array_mut().expect("types");
    let config_version = types
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/ConfigVersion"))
        .expect("ConfigVersion");
    config_version["operations"] = json!(operations);
    for (name, supertype, operations) in subtypes {
        let identity = format!("ix://{PACKAGE}/{name}");
        types.push(json!({
            "identity": identity,
            "displayName": identity,
            "kind": {"module": PACKAGE, "name": "object_type"},
            "roles": [],
            "origin": {"generated": {
                "generatorIdentity": identity,
                "generatorVersion": "1.0.0",
                "inputIdentities": [identity],
            }},
            "constraints": [],
            "extensions": [],
            "unknownPolicy": "reject",
            "supertypes": if supertype.is_empty() {
                json!([])
            } else {
                json!([format!("ix://{PACKAGE}/{supertype}")])
            },
            "fields": [],
            "operations": operations,
        }));
    }
    serde_json::to_vec(&envelope).expect("valid JSON")
}

fn unit(declarations: &str, aliases: &[&str]) -> String {
    let models: String = aliases
        .iter()
        .map(|alias| format!("model {alias} = {PACKAGE:?};\n"))
        .collect();
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         {models}{declarations}\n"
    )
}

/// Compile through the dispatch link and the check, then emit.
fn compile(
    document: &[u8],
    declarations: &str,
    aliases: &[&str],
) -> Result<Emission, Vec<CheckRefusal>> {
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let mut assembled = assemble_with_models(unit(declarations, aliases).as_bytes(), &packages)
        .expect("the unit assembles");
    let pending = assembled.link_dispatch(Default::default())?;
    let graph = assembled
        .check(CheckingLimits::default())
        .map_err(|refusals| pending.report(refusals))?;
    Ok(emit_checked(&CheckedPackage::link(graph)).expect("the package emits"))
}

fn emitted(document: &[u8], declarations: &str) -> Emission {
    compile(document, declarations, &["Config"])
        .unwrap_or_else(|refusals| panic!("the unit compiles: {refusals:?}"))
}

type Key = (String, String);

/// Each row as `(declaring owner node, operation)` with its body.
fn rows(emission: &Emission) -> Vec<(Key, OperationBodyRef)> {
    let wire = wire(emission);
    emission
        .operation_body_rows()
        .expect("the package carries operation_bodies")
        .iter()
        .map(|row| {
            assert_eq!(row.document, PACKAGE);
            let declaring = nodes(&wire)
                .iter()
                .find(|node| node["node_id"] == json!(row.declaring))
                .expect("the declaring node is emitted");
            (
                (
                    declaring["owner"]["node"].as_str().unwrap().to_owned(),
                    row.operation.clone(),
                ),
                row.body.clone(),
            )
        })
        .collect()
}

fn owner(name: &str) -> String {
    format!("ix://{PACKAGE}/{name}")
}

fn body_of(rows: &[(Key, OperationBodyRef)], declaring: &str, operation: &str) -> OperationBodyRef {
    rows.iter()
        .find(|((owner_node, name), _)| owner_node == &owner(declaring) && name == operation)
        .unwrap_or_else(|| panic!("a row for {declaring}.{operation}: {rows:?}"))
        .1
        .clone()
}

fn function(body: OperationBodyRef) -> CheckedNodeId {
    match body {
        OperationBodyRef::Function(id) => id,
        OperationBodyRef::NoBody => panic!("a body function node"),
    }
}

/// The function node `id` of the emitted package.
fn node_of<'w>(wire: &'w Value, id: &CheckedNodeId) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| node["node_id"] == json!(id))
        .expect("the body node is emitted")
}

const CALLED: &str = "pre c using v on Config::ConfigVersion::probe { self.size() > 0 }\n";

fn base_operations() -> Vec<Value> {
    vec![
        query("ConfigVersion", "size", None),
        query("ConfigVersion", "empty", None),
        query("ConfigVersion", "probe", None),
    ]
}

/// FR-576-AC-1, AC-9: `A.size` has a body that a dispatched call reaches. Its
/// row references the body function node, whose owner is the operation's own
/// and whose clause is `body`; the node yields the authored expression with
/// its source provenance.
#[trace("FR-576-AC-1", "FR-576-AC-9", "TC-1011")]
#[test]
fn a_called_body_is_referenced_by_its_row() {
    let document = package(base_operations(), &[]);
    let emission = emitted(
        &document,
        &format!("body sizeA using v on Config::ConfigVersion::size() {{ 41 }}\n{CALLED}"),
    );
    let rows = rows(&emission);
    let id = function(body_of(&rows, "ConfigVersion", "size"));
    let wire = wire(&emission);
    let node = node_of(&wire, &id);
    assert_eq!(node["owner"]["node"], owner("ConfigVersion/size"));
    assert!(
        serde_json::to_string(&node["body"])
            .unwrap()
            .contains("\"body\""),
        "the clause binding is body: {node}"
    );
    assert!(
        wire["source_map"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["node_id"] == json!(id)),
        "the body node has source provenance"
    );
}

/// FR-576-AC-2: `A.size` has a body no call reaches. The package still holds
/// its body function node and a row referencing it, and the node's id is the
/// one it has in the package where a dispatched call reaches it.
#[trace("FR-576-AC-2", "TC-1011")]
#[test]
fn an_uncalled_body_has_the_node_id_it_has_when_called() {
    let document = package(base_operations(), &[]);
    let body = "body sizeA using v on Config::ConfigVersion::size() { 41 }\n";
    let called = rows(&emitted(&document, &format!("{body}{CALLED}")));
    let uncalled_emission = emitted(&document, body);
    let uncalled = rows(&uncalled_emission);
    let id = function(body_of(&uncalled, "ConfigVersion", "size"));
    assert_eq!(
        id,
        function(body_of(&called, "ConfigVersion", "size")),
        "the node id does not depend on a call"
    );
    node_of(&wire(&uncalled_emission), &id);
}

/// FR-576-AC-3: an operation without an `operation-body` has exactly one row
/// whose `body` is `no_body`, and the package holds no function node owned by
/// the operation.
#[trace("FR-576-AC-3", "TC-1011")]
#[test]
fn an_operation_without_a_body_has_a_no_body_row_and_no_node() {
    let document = package(base_operations(), &[]);
    let emission = emitted(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 41 }\n",
    );
    let rows = rows(&emission);
    let empty: Vec<_> = rows
        .iter()
        .filter(|((_, name), _)| name == "empty")
        .collect();
    assert_eq!(empty.len(), 1);
    assert_eq!(empty[0].1, OperationBodyRef::NoBody);
    let owned = nodes(&wire(&emission))
        .iter()
        .filter(|node| node["owner"]["node"] == owner("ConfigVersion/empty"))
        .count();
    assert_eq!(owned, 0, "no node, default value or empty body");
}

/// FR-576-AC-4: `A` and `B` each declare `size`, `A` with a body and `B`
/// without. The rows are separate, and permuting declaration order emits an
/// equal member.
#[trace("FR-576-AC-4", "TC-1011")]
#[test]
fn same_named_operations_of_two_owners_have_separate_rows() {
    let document = |reverse: bool| {
        let mut subtypes = vec![("B", "", vec![query("B", "size", None)])];
        let mut operations = base_operations();
        if reverse {
            subtypes.reverse();
            operations.reverse();
        }
        package(operations, &subtypes)
    };
    let declarations = |reverse: bool| {
        let mut lines = [
            "body sizeA using v on Config::ConfigVersion::size() { 41 }\n",
            CALLED,
        ];
        if reverse {
            lines.reverse();
        }
        lines.concat()
    };
    let forward = emitted(&document(false), &declarations(false));
    let rows_forward = rows(&forward);
    assert!(matches!(
        body_of(&rows_forward, "ConfigVersion", "size"),
        OperationBodyRef::Function(_)
    ));
    assert_eq!(
        body_of(&rows_forward, "B", "size"),
        OperationBodyRef::NoBody
    );
    let permuted = emitted(&document(true), &declarations(true));
    assert_eq!(
        forward.operation_body_rows(),
        permuted.operation_body_rows()
    );
}

/// FR-576-AC-5: a subtype that only inherits `size` has no row; a subtype
/// that redefines it with its own body has its own row naming its own node,
/// distinct from the declaring type's.
#[trace("FR-576-AC-5", "TC-1011")]
#[test]
fn an_inherited_operation_has_no_row_and_a_redefinition_has_its_own() {
    let document = package(
        base_operations(),
        &[
            ("S", "ConfigVersion", vec![]),
            (
                "R",
                "ConfigVersion",
                vec![query("R", "size", Some("ConfigVersion"))],
            ),
        ],
    );
    let emission = emitted(
        &document,
        &format!(
            "body sizeA using v on Config::ConfigVersion::size() {{ 41 }}\n\
             body sizeR using v on Config::R::size() {{ 42 }}\n{CALLED}"
        ),
    );
    let rows = rows(&emission);
    assert!(
        rows.iter()
            .all(|((declaring, _), _)| declaring != &owner("S")),
        "S declares nothing: {rows:?}"
    );
    let a = function(body_of(&rows, "ConfigVersion", "size"));
    let r = function(body_of(&rows, "R", "size"));
    assert_ne!(a, r);
    assert_eq!(
        node_of(&wire(&emission), &r)["owner"]["node"],
        owner("R/size")
    );
}

/// FR-576-AC-6: a document selected through two model aliases emits one row
/// per operation, not one per alias.
#[trace("FR-576-AC-6", "TC-1011")]
#[test]
fn two_aliases_of_one_document_emit_one_row_per_operation() {
    let document = package(base_operations(), &[]);
    let single = emitted(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 41 }\n",
    );
    let aliased = compile(
        &document,
        "body sizeA using v on Config::ConfigVersion::size() { 41 }\n",
        &["Config", "Other"],
    )
    .unwrap_or_else(|refusals| panic!("the unit compiles: {refusals:?}"));
    assert_eq!(aliased.operation_body_rows().map(<[_]>::len), Some(3));
    assert_eq!(single.operation_body_rows(), aliased.operation_body_rows());
}

/// FR-576-AC-7: the row of an operation that is a candidate of a dispatch
/// table carries the candidate's operation-body identity, and the member's
/// keys equal the `operation_effects` keys of the same package.
#[trace("FR-576-AC-7", "TC-1011")]
#[test]
fn a_candidates_row_carries_the_tables_operation_body_identity() {
    let document = package(
        base_operations(),
        &[(
            "R",
            "ConfigVersion",
            vec![query("R", "size", Some("ConfigVersion"))],
        )],
    );
    let emission = emitted(
        &document,
        &format!(
            "body sizeA using v on Config::ConfigVersion::size() {{ 41 }}\n\
             body sizeR using v on Config::R::size() {{ 42 }}\n{CALLED}"
        ),
    );
    let wire = wire(&emission);
    let rows = rows(&emission);
    let candidates: Vec<&Value> = wire["checked_term_data"]["dispatch_tables"]
        .as_array()
        .expect("a dispatch table")
        .iter()
        .flat_map(|table| table["candidates"].as_array().unwrap())
        .collect();
    assert!(!candidates.is_empty());
    for body in ["ConfigVersion", "R"] {
        let id = json!(function(body_of(&rows, body, "size")));
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate["operation_body"] == id),
            "{body}.size is a candidate with its row's body identity"
        );
    }
    let keys: Vec<Value> = emission
        .operation_body_rows()
        .unwrap()
        .iter()
        .map(|row| json!([row.document, row.declaring, row.operation]))
        .collect();
    let effect_keys: Vec<Value> = wire["operation_effects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| json!([row["document"], row["declaring"], row["operation"]]))
        .collect();
    assert_eq!(keys, effect_keys, "the same keys in the same order");
}

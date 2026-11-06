// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-419: a selected model's otherwise unused type positions root nominal units.

use ix_trace_rs::trace;
use quire_spec_language::checked_handoff::{write, CheckedHandoffRequest, Corpus};
use quire_spec_language::emit_checked_v2::Request;
use serde_json::{json, Value};

const U: &str = "ix://quire/unit/U";
const MODEL: &str = "agent-ix/checked-handoff";

/// Add one declaration to the real recipe model with its own unit aliases
/// (`Amount`, `Total`) taken back to integers. Its only unit reference is
/// the position under test; no source expression, alias or other field names U.
fn model(position: &str) -> Value {
    let mut model = super::checked_handoff::integer_model();
    let identity = format!("ix://{MODEL}/UnusedType");
    let origin = json!({"generated": {"generatorIdentity": identity,
        "generatorVersion": "1.0.0", "inputIdentities": [identity]}});
    let mut declared = json!({"identity": identity, "displayName": "UnusedType",
        "roles": [], "origin": origin, "constraints": [], "extensions": [],
        "unknownPolicy": "reject"});
    let members = match position {
        "sequence" => json!({"kind": "sequence", "items": U}),
        "option" => json!({"kind": "option", "inner": U}),
        "alias" => json!({"kind": "alias", "target": U}),
        "rational" => {
            // Rational TypeDefinition.unit must name the model's own namespace,
            // unlike a typeRef, which may name a source-declared unit.
            model["dimensions"] = json!([{"identity": format!("ix://{MODEL}/dimension/Money"),
                "name": "Money", "base": true, "terms": [], "origin": origin}]);
            model["units"] = json!([{"identity": format!("ix://{MODEL}/unit/U"),
                "name": "U", "dimension": format!("ix://{MODEL}/dimension/Money"),
                "scale": "1", "offset": "0", "origin": origin}]);
            json!({"kind": {"module": MODEL, "name": "value_type"},
                "scalar": "rational", "unit": U})
        }
        "field" => json!({"kind": {"module": MODEL, "name": "record"},
            "fields": [{"identity": format!("{identity}/value"), "name": "value",
                "typeRef": U, "presence": "required", "nullable": false,
                "defaultKind": "none", "multiplicity": {"lower": 1, "upper": 1,
                    "ordered": false, "unique": true}, "origin": origin}]}),
        "tuple" => {
            model["constructs"].as_array_mut().unwrap().push(json!({
                "kind": {"module": MODEL, "name": "tuple"}, "moduleVersion": "1.0.0",
                "construct": {"identity": "none", "shape": "tuple", "members": {"positions": "required"},
                    "meaning": "quire.meaning.model.tuple-value-type/v1"}}));
            json!({"kind": {"module": MODEL, "name": "tuple"}, "positions": [{
                "identity": format!("{identity}/p0"), "typeRef": U, "origin": origin}]})
        }
        "parameter" | "result" => {
            let identity = format!("ix://{MODEL}/Workflow");
            let mut operation = json!({"identity": format!("{identity}/measure"),
                "name": "measure", "params": [], "pre": [], "post": [],
                "frame": {"modifies": [], "creates": [], "deletes": []}, "origin": origin});
            let multiplicity = json!({"lower": 1, "upper": 1, "ordered": false, "unique": true});
            if position == "parameter" {
                operation["params"] = json!([{"identity": format!("{identity}/measure/value"),
                    "name": "value", "typeRef": U, "presence": "required", "nullable": false,
                    "defaultKind": "none", "multiplicity": multiplicity,
                    "origin": origin}]);
            } else {
                operation["returns"] =
                    json!({"typeRef": U, "nullable": false, "multiplicity": multiplicity});
            }
            let workflow = model["types"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|declaration| declaration["identity"] == identity)
                .unwrap();
            workflow["operations"]
                .as_array_mut()
                .unwrap()
                .push(operation);
            return model;
        }
        other => panic!("unknown position {other}"),
    };
    declared
        .as_object_mut()
        .unwrap()
        .extend(members.as_object().unwrap().clone());
    model["types"].as_array_mut().unwrap().push(declared);
    model
}

fn assert_root(position: &str) {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("unit.native");
    let model_path = dir.path().join("model.json");
    std::fs::write(&model_path, serde_json::to_vec(&model(position)).unwrap()).unwrap();
    std::fs::write(
        &source,
        "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.model.complete/v1\";\n\
        model M = \"agent-ix/checked-handoff\";\n\
        dimension Money;\nunit Base : Money = rational(1, 1);\n\
        unit U : Money = rational(2, 1) * Base;\n\
        unit Unused : Money = rational(7, 1) * Base;\n\
        invariant Ready using v on M::Workflow at current { self.ready }",
    )
    .unwrap();
    let request = CheckedHandoffRequest {
        emit: Request {
            source,
            package_name: "unit-roots".into(),
            models: vec![model_path],
            libraries: Vec::new(),
            execution_points: None,
            temporal_selections: Vec::new(),
            out: None,
        },
        corpus: Corpus::None,
        native_join: qsl_package::V2ReadLimits::fixture().native_join_limits(),
    };
    let out = dir.path().join("handoff");
    write(&request, &out).unwrap_or_else(|error| {
        panic!("{position}: production writer and IR read-back: {error:?}")
    });
    let wire: Value =
        serde_json::from_slice(&std::fs::read(out.join("checked-package.json")).unwrap()).unwrap();
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let declared = |name: &str| {
        nodes
            .iter()
            .find(|node| node["declaration"]["qualified_name"] == json!([name]))
    };
    let unit = declared("U").expect("the sole model type reference roots U");
    let base = declared("Base").expect("U's target-unit closure");
    let dimension = declared("Money").expect("U's dimension closure");
    assert_eq!(unit["semantic_form"], "unit");
    assert_eq!(dimension["semantic_form"], "dimension");
    for node in [unit, base] {
        assert_eq!(node["semantic_type"], dimension["node_id"]);
        assert_eq!(node["owner"]["kind"], "source");
        assert_eq!(node["owner"], dimension["owner"]);
    }
    let dependencies = unit["dependencies"].as_array().unwrap();
    assert_eq!(dependencies.len(), 2);
    assert!(dependencies.contains(&dimension["node_id"]));
    assert!(dependencies.contains(&base["node_id"]));
    assert_eq!(base["dependencies"], json!([dimension["node_id"]]));
    assert!(declared("Unused").is_none());
}

#[trace("FR-094-AC-15", "TC-419")]
#[test]
fn a_field_alone_roots_its_unit() {
    assert_root("field");
}

#[trace("FR-094-AC-15", "TC-419")]
#[test]
fn a_tuple_position_alone_roots_its_unit() {
    assert_root("tuple");
}

#[trace("FR-094-AC-15", "TC-419")]
#[test]
fn a_sequence_item_alone_roots_its_unit() {
    assert_root("sequence");
}

#[trace("FR-094-AC-15", "TC-419")]
#[test]
fn an_option_inner_alone_roots_its_unit() {
    assert_root("option");
}

#[trace("FR-094-AC-15", "TC-419")]
#[test]
fn an_alias_target_alone_roots_its_unit() {
    assert_root("alias");
}

#[trace("FR-094-AC-15", "TC-419")]
#[test]
fn an_operation_parameter_alone_roots_its_unit() {
    assert_root("parameter");
}

#[trace("FR-094-AC-15", "TC-419")]
#[test]
fn an_operation_result_alone_roots_its_unit() {
    assert_root("result");
}

#[trace("FR-094-AC-15", "TC-419")]
#[test]
fn a_rational_scalar_unit_alone_roots_its_unit() {
    assert_root("rational");
}

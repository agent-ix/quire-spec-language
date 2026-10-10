// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-151: conformance through actual IR intake and authored clause checking.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

use crate::model_operations::{
    admit_and_assemble_with_body, config_version_document, empty_frame, operation,
};

const PACKAGE: &str = "example/config-version";

fn identity(path: &str) -> String {
    format!("ix://{PACKAGE}/{path}")
}

fn origin(path: &str) -> Value {
    json!({"source": {"sourceIdentity": identity("spec"), "path": path,
        "startLine": 1, "startColumn": 1}})
}

fn object(name: &str, parent: Option<&str>, fields: Vec<Value>, operations: Vec<Value>) -> Value {
    json!({"identity": identity(name), "displayName": name,
        "kind": {"module": PACKAGE, "name": "object_type"}, "roles": [],
        "origin": origin(name), "constraints": [], "extensions": [],
        "unknownPolicy": "reject", "supertypes": parent.map(identity).into_iter().collect::<Vec<_>>(),
        "fields": fields, "operations": operations, "relationships": []})
}

fn field(owner: &str, name: &str, ty: &str, lower: u64, upper: Option<u64>, parent: Option<&str>) -> Value {
    let path = format!("{owner}/{name}");
    let mut value = json!({"identity": identity(&path), "name": name, "typeRef": identity(ty),
        "presence": if lower == 0 { "optional" } else { "required" },
        "nullable": false, "defaultKind": "none", "origin": origin(&path),
        "multiplicity": {"lower": lower, "upper": upper.map_or(Value::Null, |n| json!(n)),
            "ordered": false, "unique": true}});
    if let Some(parent) = parent {
        value["redefines"] = json!(identity(parent));
    }
    value
}

fn scalar(template: &Value, name: &str, maximum: i64) -> Value {
    let mut value = template.clone();
    value["identity"] = json!(identity(name));
    value["displayName"] = json!(name);
    value["origin"] = origin(name);
    for constraint in value["constraints"].as_array_mut().expect("scalar constraints") {
        let keyword = constraint["keyword"].as_str().expect("bound keyword").to_owned();
        constraint["identity"] = json!(identity(&format!("{name}/constraints/{keyword}")));
        constraint["origin"] = origin(name);
        if keyword == "max" {
            constraint["operands"]["value"] = json!(maximum);
        }
    }
    value
}

fn writer(owner: &str, writes: &[&str], parent: Option<&str>) -> Value {
    let mut value = operation("set", json!([]), None, json!({
        "modifies": writes.iter().map(|path| identity(path)).collect::<Vec<_>>(),
        "creates": [], "deletes": []}));
    value["identity"] = json!(identity(&format!("{owner}/set")));
    value["origin"] = origin(&format!("{owner}/set"));
    if let Some(parent) = parent {
        value["redefines"] = json!(identity(parent));
    }
    value
}

fn document(types: impl FnOnce(&Value) -> Vec<Value>) -> Vec<u8> {
    let base = config_version_document(operation("unused", json!([]), None, empty_frame()),
        Vec::new(), Vec::new(), json!([]));
    let mut value: Value = serde_json::from_slice(&base).expect("fixture JSON");
    let scalar = value["types"].as_array().expect("types").iter()
        .find(|ty| ty["scalar"] == "integer").expect("bound scalar").clone();
    value["types"] = json!(types(&scalar));
    value["populations"] = json!([]);
    serde_json::to_vec(&value).expect("fixture encoding")
}

fn check(document: &[u8], body: &str) -> Result<(), Vec<(Code, Option<&'static str>)>> {
    let declarations = admit_and_assemble_with_body(document, body).map_err(|refusal| {
        refusal.errors.iter().map(|error| (error.cause.code(),
            Some(error.cause.catalog_code().cause()))).collect::<Vec<_>>()
    })?;
    declarations.check(CheckingLimits::default()).map(|_| ()).map_err(|refusals| {
        refusals.iter().map(|refusal| (refusal.cause.code(), refusal.cause.cause())).collect()
    })
}

#[trace("FR-082-AC-4", "FR-082-AC-8", "QSpec-TC-196")]
#[test]
fn actual_postconditions_decide_all_six_refinement_vectors() {
    for (row, expected) in [(0, false), (1, true), (2, false), (3, false), (4, true), (5, false)] {
        let numeric = row >= 3;
        let narrowed_name = if numeric { "cs" } else if row == 2 { "xr" } else { "xb" };
        let bytes = document(|template| vec![
            scalar(template, "Count", 9), scalar(template, "Small", 5),
            object("A", None, vec![field("A", "x", "A", 0, Some(1), None),
                field("A", "c", "Count", 1, Some(1), None)],
                vec![writer("A", &["A/x", "A/c"], None)]),
            object("B", Some("A"), vec![field("B", narrowed_name,
                if numeric { "Small" } else if row == 2 { "B" } else { "A" },
                if row == 2 { 0 } else { 1 }, Some(1), Some(if numeric { "A/c" } else { "A/x" }))],
                if [1, 4, 5].contains(&row) { vec![writer("B", &["A/x", "A/c"], Some("A/set"))] } else { vec![] }),
        ]);
        let body = match row {
            1 => "post QB using v on Config::B::set { present(self.xb) }",
            4 => "post QC using v on Config::B::set { self.cs <= 5 }",
            5 => "post QD using v on Config::B::set { self.cs <= 6 }",
            _ => "function noop using v(): Boolean pure { true }",
        };
        let result = check(&bytes, body);
        if expected {
            assert_eq!(result, Ok(()), "R08 row {row}");
        } else {
            assert_eq!(result, Err(vec![(Code::UndefinedExpression, Some("unproved-refinement"))]),
                "R08 row {row} must reach its conformance obligation");
        }
    }
}

#[trace("FR-082-AC-4", "QSpec-TC-196")]
#[test]
fn immediate_parent_and_owning_writer_decide_the_three_level_chain() {
    for (child_maximum, parent_guard, expected) in [(3, 5, None),
        (7, 5, Some((Code::IllTyped, "variance-result"))),
        (3, 6, Some((Code::UndefinedExpression, "unproved-refinement")))] {
        let bytes = document(|template| vec![
            scalar(template, "DA", 9), scalar(template, "DB", 5), scalar(template, "DC", child_maximum),
            object("A", None, vec![field("A", "x", "DA", 1, Some(1), None)],
                vec![writer("A", &["A/x"], None)]),
            object("B", Some("A"), vec![field("B", "xb", "DB", 1, Some(1), Some("A/x"))],
                vec![writer("B", &["B/xb"], Some("A/set"))]),
            object("C", Some("B"), vec![field("C", "xc", "DC", 1, Some(1), Some("B/xb"))],
                vec![writer("C", &["C/xc"], Some("B/set"))]),
        ]);
        let body = format!("post QB using v on Config::B::set {{ self.xb <= {parent_guard} }}\n\
            post QC using v on Config::C::set {{ self.xc <= 3 }}");
        assert_eq!(check(&bytes, &body), expected.map_or(Ok(()), |(code, cause)|
            Err(vec![(code, Some(cause))])), "DC={child_maximum}, QB={parent_guard}");
    }
}

#[trace("FR-082-AC-1", "QSpec-TC-196")]
#[test]
fn model_multiplicity_failures_reach_conformance_instead_of_generic_assembly() {
    for (parent_lower, parent_upper, lower, upper, ordered, compatible) in [
        (1, Some(3), 0, Some(5), false, false),
        (0, Some(5), 1, Some(3), false, true),
        (0, None, 0, Some(5), false, true),
        (0, Some(5), 0, None, false, false),
        (0, Some(5), 0, Some(5), true, false),
    ] {
        let bytes = document(|_| {
            let mut child = field("B", "n", "A", lower, upper, Some("A/n"));
            child["multiplicity"]["ordered"] = json!(ordered);
            vec![object("A", None, vec![field("A", "n", "A", parent_lower, parent_upper, None)], vec![]),
                object("B", Some("A"), vec![child], vec![])]
        });
        let expected = if compatible { Ok(()) } else {
            Err(vec![(Code::IllTyped, Some("multiplicity-narrowing"))])
        };
        assert_eq!(check(&bytes, "function noop using v(): Boolean pure { true }"), expected);
    }
}

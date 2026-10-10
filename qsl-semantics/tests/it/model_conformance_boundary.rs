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

#[trace("FR-082-AC-1", "FR-082-AC-8", "QSpec-TC-196")]
#[test]
fn all_five_static_failures_and_presence_survive_record_order_reversal() {
    for reverse in [false, true] {
        let bytes = document(|template| {
            let parameter = |owner: &str, ty: &str| json!({"identity": identity(&format!("{owner}/set/arg")),
                "name": "arg", "typeRef": identity(ty), "presence": "required", "nullable": false,
                "defaultKind": "none",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "origin": origin(&format!("{owner}/set/arg"))});
            let mut parent = writer("A", &["A/p"], None);
            parent["params"] = json!([parameter("A", "A")]);
            parent["returns"] = json!({"typeRef": identity("B"), "nullable": false,
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true}});
            let mut child = writer("B", &["A/p", "A/bad"], Some("A/set"));
            child["params"] = json!([parameter("B", "B")]);
            child["returns"] = json!({"typeRef": identity("A"), "nullable": false,
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true}});
            let mut types = vec![scalar(template, "Count", 9), scalar(template, "Wide", 10),
                object("A", None, vec![field("A", "bad", "Count", 1, Some(1), None),
                    field("A", "p", "A", 0, Some(1), None)], vec![parent]),
                object("B", Some("A"), vec![field("B", "bad", "Wide", 0, Some(3), Some("A/bad")),
                    field("B", "p", "A", 1, Some(1), Some("A/p"))], vec![child])];
            if reverse { types.reverse(); }
            types
        });
        assert_eq!(check(&bytes, "function noop using v(): Boolean pure { true }"),
            Err(vec![(Code::IllTyped, Some("variance-result")),
                (Code::IllTyped, Some("multiplicity-narrowing")),
                (Code::UndefinedExpression, Some("unproved-refinement")),
                (Code::IllTyped, Some("variance-parameter")),
                (Code::IllTyped, Some("variance-result")),
                (Code::IllTyped, Some("effect-escape"))]), "reverse={reverse}");
    }
}

#[trace("FR-082-AC-4", "FR-104", "QSpec-TC-196")]
#[test]
fn admitted_clause_failures_precede_the_conformance_stage() {
    let bytes = document(|template| vec![scalar(template, "Count", 9), scalar(template, "Small", 5),
        object("A", None, vec![field("A", "x", "Count", 1, Some(1), None)], vec![writer("A", &["A/x"], None)]),
        object("B", Some("A"), vec![field("B", "xb", "Small", 1, Some(1), Some("A/x"))],
            vec![writer("B", &["B/xb"], Some("A/set"))])]);
    for (expression, code, cause) in [("missing", Code::MissingDeclaration, "missing-name"),
        ("1", Code::IllTyped, "type-mismatch")] {
        let body = format!("post QB using v on Config::B::set {{ {expression} }}");
        assert_eq!(check(&bytes, &body), Err(vec![(code, Some(cause))]), "{expression}");
    }
}

#[trace("FR-082-AC-4", "FR-104", "QSpec-TC-196")]
#[test]
fn an_unrelated_or_removed_guard_cannot_prove_an_owning_writer() {
    for (body, extra_owner) in [
        ("post QB using v on Config::B::set { true }", None),
        ("post QS using v on Config::S::set { self.xs <= 5 }", Some("S")),
        ("post QC using v on Config::C::set { self.xc <= 3 }", Some("C")),
    ] {
        let bytes = document(|template| {
            let mut types = vec![scalar(template, "Count", 9), scalar(template, "Small", 5), scalar(template, "Tiny", 3),
                object("A", None, vec![field("A", "x", "Count", 1, Some(1), None)], vec![writer("A", &["A/x"], None)]),
                object("B", Some("A"), vec![field("B", "xb", "Small", 1, Some(1), Some("A/x"))],
                    vec![writer("B", &["B/xb"], Some("A/set"))])];
            if let Some(owner) = extra_owner {
                let sibling = owner == "S";
                types.push(object(owner, Some(if sibling { "A" } else { "B" }),
                    vec![field(owner, if sibling { "xs" } else { "xc" }, if sibling { "Small" } else { "Tiny" },
                        1, Some(1), Some(if sibling { "A/x" } else { "B/xb" }))],
                    vec![writer(owner, &[if sibling { "S/xs" } else { "C/xc" }], Some(if sibling { "A/set" } else { "B/set" }))]));
            }
            types
        });
        assert_eq!(check(&bytes, body), Err(vec![(Code::UndefinedExpression, Some("unproved-refinement"))]),
            "guard={body}");
    }
}

#[trace("FR-082-AC-3", "QSpec-TC-196")]
#[test]
fn subsetting_runs_both_axes_at_the_runtime_boundary() {
    for row in 0..3 {
        let bytes = document(|_| {
            let mut subset = field("A", "ys", if row == 1 { "A" } else { "B" },
                if row == 2 { 0 } else { 1 }, Some(if row == 2 { 4 } else { 2 }), None);
            subset["subsets"] = json!([identity("A/ysup")]);
            vec![object("A", None, vec![field("A", "ysup", "B", 1, Some(3), None), subset], vec![]),
                object("B", Some("A"), vec![], vec![])]
        });
        let expected = match row {
            0 => Ok(()),
            1 => Err(vec![(Code::IllTyped, Some("subsetting-type"))]),
            _ => Err(vec![(Code::IllTyped, Some("multiplicity-narrowing"))]),
        };
        assert_eq!(check(&bytes, "function noop using v(): Boolean pure { true }"), expected, "row={row}");
    }
}

#[trace("FR-082-AC-2", "QSpec-TC-196")]
#[test]
fn contested_field_and_operation_targets_have_one_structural_authority() {
    for operation_contest in [false, true] {
        let bytes = document(|_| {
            if operation_contest {
                let first = writer("B", &[], Some("A/set"));
                let mut second = first.clone();
                second["identity"] = json!(identity("B/set2"));
                second["name"] = json!("set2");
                second["origin"] = origin("B/set2");
                vec![object("A", None, vec![], vec![writer("A", &[], None)]),
                    object("B", Some("A"), vec![], vec![first, second])]
            } else {
                vec![object("A", None, vec![field("A", "x", "A", 1, Some(1), None)], vec![]),
                    object("B", Some("A"), vec![field("B", "xb", "A", 1, Some(1), Some("A/x")),
                        field("B", "xb2", "A", 1, Some(1), Some("A/x"))], vec![])]
            }
        });
        assert_eq!(check(&bytes, "function noop using v(): Boolean pure { true }"),
            Err(vec![(Code::InvalidModelBinding, Some("redefinition-target"))]), "operations={operation_contest}");
    }
}

#[trace("FR-082-AC-10", "QSpec-TC-196", "QSpec-TC-198")]
#[test]
fn reference_covariance_uses_the_complete_model_without_a_writer_obligation() {
    for (parent_type, child_type, compatible) in [("A", "B", true),
        ("B", "A", false), ("A", "C", false)] {
        let bytes = document(|_| vec![
            object("A", None, vec![field("A", "x", parent_type, 1, Some(1), None)], vec![]),
            object("B", Some("A"), vec![field("B", "xb", child_type, 1, Some(1), Some("A/x"))], vec![]),
            object("C", None, vec![], vec![]),
        ]);
        let expected = if compatible { Ok(()) } else {
            Err(vec![(Code::IllTyped, Some("variance-result"))])
        };
        assert_eq!(check(&bytes, "function noop using v(): Boolean pure { true }"), expected,
            "Reference<{child_type}> redefines Reference<{parent_type}>");
    }
}

#[trace("FR-081-AC-1", "FR-081-AC-7", "FR-081-AC-8", "TC-213")]
#[test]
fn original_inventory_retains_scalar_and_exact_per_key_source_generated_origins() {
    use std::collections::{BTreeMap, BTreeSet};
    use qsl_semantics::model::accounting::ModelNormalizationLimits;
    use qsl_semantics::model::intake::admit_unit;
    use qsl_semantics::model::key::{DeclarationKey, RULE_QUALIFY};
    use crate::model_operations::{config_unit_with_body, parse_and_build};

    let bytes = document(|template| {
        let mut a = object("A", None, vec![field("A", "x", "Count", 1, Some(1), None)],
            vec![writer("A", &["A/x"], None)]);
        a["origin"] = json!({"source": {"sourceIdentity": identity("spec"),
            "path": "spec/A.md", "startLine": 2, "startColumn": 1, "endLine": 8, "endColumn": 9}});
        let mut b = object("B", Some("A"), vec![], vec![]);
        b["origin"] = json!({"source": {"sourceIdentity": identity("spec"),
            "path": "spec/B.md", "startLine": 3, "startColumn": 2}});
        let mut count = scalar(template, "Count", 9);
        count["origin"] = json!({"generated": {"generatorIdentity": identity("generator"),
            "generatorVersion": "1.2.3", "inputIdentities": [identity("A"), identity("B")]}});
        vec![a, b, count]
    });
    let baseline: Value = serde_json::from_slice(&bytes).expect("original document");
    let key = |name: &str| DeclarationKey { package: PACKAGE.to_owned(), node: identity(name) };
    let expected: BTreeMap<_, _> = [
        (key("A"), baseline["types"][0]["origin"].clone()),
        (key("B"), baseline["types"][1]["origin"].clone()),
        (key("Count"), baseline["types"][2]["origin"].clone()),
        (key("A/x"), baseline["types"][0]["fields"][0]["origin"].clone()),
        (key("A/set"), baseline["types"][0]["operations"][0]["origin"].clone()),
    ].into_iter().collect();
    let mut baseline_ids = None;
    let mut baseline_view = None;
    for row in 0..9 {
        let mut input = baseline.clone();
        match row {
            0 => {},
            1 => input["types"].as_array_mut().expect("types").reverse(),
            2 => input["source"]["version"] = json!("1.0.1"),
            3 => {
                let a = input["types"][0]["origin"].clone();
                input["types"][0]["origin"] = input["types"][1]["origin"].clone();
                input["types"][1]["origin"] = a;
            }
            4 => input["types"][0]["origin"]["source"]["endColumn"] = json!(10),
            5 => { input["types"][0]["origin"]["source"].as_object_mut().expect("source").remove("endLine"); }
            6 => input["types"][2]["origin"]["generated"]["generatorVersion"] = json!("1.2.4"),
            7 => input["types"][2]["origin"]["generated"]["inputIdentities"].as_array_mut().expect("inputs").reverse(),
            8 => input["types"][2]["origin"] = input["types"][1]["origin"].clone(),
            _ => unreachable!("bounded vector table"),
        }
        let bytes = serde_json::to_vec(&input).expect("vector document");
        let (unit, packages) = config_unit_with_body(&bytes, "function noop using v(): Boolean pure { true }");
        let built = parse_and_build(&unit);
        let selected = admit_unit(&built.selections().models, &packages, ModelNormalizationLimits::UNLIMITED)
            .expect("actual admitted original inventory");
        assert_eq!(selected.len(), 1);
        let selection = &selected[0];
        let retained_keys: BTreeSet<_> = selection.original_keys().cloned().collect();
        assert_eq!(retained_keys, expected.keys().cloned().collect(), "inventory vector {row}");
        let retained: BTreeMap<_, _> = selection.original_keys().map(|key| {
            let node = selection.original_node(key).expect("own original node");
            (key.clone(), node["origin"].clone())
        }).collect();
        let mut offered = BTreeMap::new();
        for ty in input["types"].as_array().expect("types") {
            let mut nodes = vec![ty];
            for collection in ["fields", "operations"] {
                if let Some(entries) = ty.get(collection).and_then(Value::as_array) { nodes.extend(entries); }
            }
            for node in nodes {
                offered.insert(DeclarationKey { package: PACKAGE.to_owned(),
                    node: node["identity"].as_str().expect("declared identity").to_owned() }, node["origin"].clone());
            }
        }
        assert_eq!(retained, offered, "actual origin retention vector {row}");
        if row < 3 { assert_eq!(retained, expected); } else { assert_ne!(retained, expected, "origin mutant {row}"); }
        let entries = selection.view.declarations();
        let effective_originals: BTreeSet<_> = entries.iter().map(|entry| entry.preimage.original.clone()).collect();
        assert!(effective_originals.is_subset(&retained_keys));
        assert!(!effective_originals.contains(&key("Count")), "scalar receives no fabricated effective entry");
        for original in [key("A"), key("B"), key("A/x"), key("A/set")] {
            assert!(entries.iter().any(|entry| entry.preimage.original == original
                && entry.preimage.derivation.iter().any(|fact| fact.rule == RULE_QUALIFY)), "qualify {original:?}");
        }
        let qualifying_ids: BTreeSet<_> = entries.iter().filter(|entry|
            entry.preimage.derivation.iter().any(|fact| fact.rule == RULE_QUALIFY))
            .map(|entry| entry.effective_id).collect();
        if row == 0 {
            baseline_ids = Some(qualifying_ids);
            baseline_view = Some(selection.view.identity());
        } else {
            assert_eq!(Some(qualifying_ids), baseline_ids, "origins and selection digest cannot re-key declarations");
            assert_ne!(Some(selection.view.identity()), baseline_view, "different content selects a different view");
        }
    }
}

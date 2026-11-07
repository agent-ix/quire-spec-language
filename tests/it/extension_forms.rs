// SPDX-License-Identifier: AGPL-3.0-or-later
//! A selected extension's declaration form checks against its typed-node
//! schema and packages as an `extension`/`extension_form` node (FR-354
//! "Admission" and "Emission"; TC-882 steps 1 to 3; QSpec FR-467, FR-469).
use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_foundation::diagnostic::Code;
use qsl_replay::spine::{
    check, package, parse_selecting_extensions, select, ProfileCatalogInput, CompileRefusal, DependencyInput, EmittedUnit,
    PackageLimits, ParseRequest, SpineLimits,
};
use qsl_semantics::check::{CheckCause, ExtensionCause};
use qsl_semantics::library::bundle::{
    Definition, DefinitionCatalog, DefinitionRole, PackageLimits as CatalogLimits, ReaderAuthority,
};
use quire_exact::Cancel;
use serde_json::Value;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\";\nprofile Retention = \"retention\";\n";
const FORM: &str = "retention Audit { subject: \"audit\", days: 30 }";

/// TC-882's `retention` extension whose node kind is `node_kind`, held after
/// catalog validation.
fn held(node_kind: &str) -> DefinitionCatalog {
    held_typed(node_kind, "Int[1, 3650]")
}

/// As [`held`], with `days` typed `days_type`.
fn held_typed(node_kind: &str, days_type: &str) -> DefinitionCatalog {
    let document = serde_json::json!({
        "grammar_schema": {"productions": [
            {"keyword": "retention", "clauses": ["subject", "days"]}
        ]},
        "typed_node_schema": {"nodes": [
            {"keyword": "retention", "node_kind": node_kind, "clauses": [
                {"name": "subject", "type": "Text"},
                {"name": "days", "type": days_type}
            ]}
        ]}
    });
    DefinitionCatalog::new(vec![extension_definition("retention", &document)]).unwrap()
}

/// An `Extension`-role definition of `agent-ix`/`identity` holding `document`.
fn extension_definition(identity: &str, document: &Value) -> Definition {
    Definition::from_exact_bytes(
        &ReaderAuthority::fixture(),
        "agent-ix",
        identity,
        DefinitionRole::Extension,
        BTreeSet::new(),
        BTreeSet::new(),
        document.to_string().as_bytes(),
    )
    .unwrap()
}

fn identity() -> SourceIdentity {
    SourceIdentity::new("test", "extension-forms")
}

/// `declarations` through S1 to S4 under the held extension.
fn compile(held: &DefinitionCatalog, declarations: &str) -> Result<EmittedUnit, Box<CompileRefusal>> {
    compile_source(held, &format!("{HEADER}{declarations}\n"))
}

/// The whole `source` through S1 to S4 under `held`.
fn compile_source(held: &DefinitionCatalog, source: &str) -> Result<EmittedUnit, Box<CompileRefusal>> {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let refusal = |failure| {
        qsl_replay::spine::refusal_or_fault(failure)
            .unwrap_or_else(|fault| panic!("the front end faulted: {fault:?}"))
    };
    let source_identity = identity();
    let parsed = parse_selecting_extensions(
        &ParseRequest {
            source: &source_identity,
            path: "unit.native",
            bytes: source.as_bytes(),
        },
        ProfileCatalogInput {
            catalog: held,
            limits: CatalogLimits::default(),
        },
        limits.source,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let models = select(
        &parsed,
        &qsl_semantics::model::intake::PackageInput::default(),
        limits.model,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    let checked = check(
        &parsed,
        "extension-forms".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(refusal)?
    .into_value();
    Ok(package(&checked, PackageLimits::default(), &cancel)
        .map_err(refusal)?
        .into_value())
}

fn wire(emitted: &EmittedUnit) -> Value {
    serde_json::from_slice(emitted.package().bytes()).expect("a JSON wire")
}

fn nodes(wire: &Value) -> &[Value] {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("a node list")
}

/// The one `extension` node of `wire`.
fn extension_node(wire: &Value) -> &Value {
    let found: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node["node_tag"] == "extension")
        .collect();
    assert_eq!(found.len(), 1, "one extension node");
    found[0]
}

fn node_by_id<'w>(wire: &'w Value, id: &Value) -> &'w Value {
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

/// The region `[start, end)` of the `source_map` entry of `node_id` at
/// `role`.
fn regions<'w>(wire: &'w Value, node_id: &Value, role: &str) -> Vec<&'w Value> {
    wire["source_map"]
        .as_array()
        .expect("a source map")
        .iter()
        .filter(|entry| &entry["node_id"] == node_id && entry["role"] == role)
        .flat_map(|entry| entry["regions"].as_array().expect("regions"))
        .collect()
}

fn refusal_cause(refusal: &CompileRefusal) -> &ExtensionCause {
    let CompileRefusal::Check { refusals, .. } = refusal else {
        panic!("expected a check refusal, got {refusal:?}");
    };
    let [first] = refusals.as_slice() else {
        panic!("expected one refusal, got {refusals:?}");
    };
    let CheckCause::Extension(cause) = &first.cause else {
        panic!("expected an extension refusal, got {:?}", first.cause);
    };
    cause
}

#[trace("TC-882", "FR-354-AC-1")]
#[test]
fn a_selected_extension_form_checks_as_one_typed_node_with_both_subnodes() {
    let emitted = compile(&held("retention"), FORM).expect("the unit compiles");
    let wire = wire(&emitted);
    let form = extension_node(&wire);
    assert_eq!(form["semantic_form"], "extension_form");
    assert_eq!(form["body"]["node_kind"], "retention");
    assert_eq!(
        form["body"]["definition"],
        serde_json::json!({"authority": "agent-ix", "identity": "retention"})
    );
    let clauses = form["body"]["clauses"].as_array().expect("clauses");
    let names: Vec<_> = clauses.iter().map(|clause| &clause["name"]).collect();
    assert_eq!(names, ["subject", "days"]);
    // Each clause is a reference to a literal subnode typed as the schema
    // types it: `Text` and `Int[1, 3650]`.
    let subject = node_by_id(&wire, &clauses[0]["value"]["target"]);
    let days = node_by_id(&wire, &clauses[1]["value"]["target"]);
    assert_eq!(subject["body"]["value"], "audit");
    assert_eq!(days["body"]["value"], "30");
    let subject_type = node_by_id(&wire, &subject["semantic_type"]);
    assert_eq!(subject_type["semantic_form"], "text");
    let days_type = node_by_id(&wire, &days["semantic_type"]);
    assert_eq!(days_type["semantic_form"], "integer_range");
    let bounds: Vec<_> = days_type["body"]["members"]
        .as_array()
        .expect("bounds")
        .iter()
        .map(|bound| bound["value"]["value"].as_str().expect("a bound"))
        .collect();
    assert_eq!(bounds, ["1", "3650"]);
}

#[trace("TC-882", "FR-354-AC-2")]
#[test]
fn a_clause_value_of_the_wrong_type_refuses_ill_typed_naming_the_clause_and_both_types() {
    let declaration = "retention Audit { subject: \"audit\", days: \"thirty\" }";
    let refusal = compile(&held("retention"), declaration)
        .expect_err("`days: \"thirty\"` is not an `Int[1, 3650]`");
    let ExtensionCause::TypeMismatch {
        clause,
        span,
        expected,
        actual,
    } = refusal_cause(&refusal)
    else {
        panic!("expected a type mismatch, got {refusal:?}");
    };
    assert_eq!(clause, "days");
    assert_eq!(expected, "Int[1, 3650]");
    assert_eq!(actual, "Text");
    let source = format!("{HEADER}{declaration}\n");
    let at = source.find("\"thirty\"").expect("the value");
    assert_eq!((span.start, span.end), (at, at + "\"thirty\"".len()));
    let CompileRefusal::Check { refusals, region } = &*refusal else {
        unreachable!("refusal_cause matched a check refusal");
    };
    assert_eq!(refusals[0].cause.code(), Code::IllTyped);
    assert_eq!(refusals[0].cause.cause(), Some("type-mismatch"));
    let region = region.as_ref().expect("the refusal is located");
    assert_eq!((region.start(), region.end()), (at as u64, (at + 8) as u64));
}

#[trace("TC-882", "FR-354-AC-2")]
#[test]
fn a_source_level_refusal_is_not_an_invalid_package() {
    for declaration in [
        "retention Audit { subject: \"audit\" }",
        "retention Audit { days: 30, subject: \"audit\" }",
        "retention Audit { subject: \"audit\", }",
    ] {
        let refusal = compile(&held("retention"), declaration).expect_err("S1 refuses the form");
        let CompileRefusal::Source(diagnostic) = &*refusal else {
            panic!("expected an S1 refusal for {declaration}, got {refusal:?}");
        };
        assert_eq!(diagnostic.code.as_str(), "invalid_syntax", "{declaration}");
        assert_eq!(diagnostic.cause.as_str(), "unexpected-token", "{declaration}");
    }
}

#[trace("TC-882", "FR-354-AC-6")]
#[test]
fn the_emitted_node_carries_its_declaration_owner_clauses_dependencies_and_span() {
    let emitted = compile(&held("retention"), FORM).expect("the unit compiles");
    let wire = wire(&emitted);
    let form = extension_node(&wire);
    assert_eq!(form["declaration"]["qualified_name"], serde_json::json!(["Audit"]));
    assert_eq!(
        form["owner"],
        serde_json::json!({"kind": "source", "authority": "test", "identity": "extension-forms"})
    );
    assert_eq!(form["semantic_type"], form["node_id"], "its own type");
    assert!(form["recursion_group"].is_null());
    let clauses = form["body"]["clauses"].as_array().expect("clauses");
    for clause in clauses {
        assert_eq!(clause["term"], "binding");
        assert_eq!(clause["value"]["term"], "reference");
    }
    // `dependencies` is exactly the clause targets, in node-id order.
    let mut targets: Vec<Value> = clauses
        .iter()
        .map(|clause| clause["value"]["target"].clone())
        .collect();
    targets.sort_by_key(|id| id.to_string());
    let mut dependencies: Vec<Value> = form["dependencies"].as_array().expect("deps").to_vec();
    dependencies.sort_by_key(|id| id.to_string());
    assert_eq!(dependencies, targets);
    // One `declaration` occurrence at ordinal 0, spanning the whole form.
    assert_eq!(
        form["occurrences"],
        serde_json::json!([{"role": "declaration", "ordinal": 0}])
    );
    let spans = regions(&wire, &form["node_id"], "declaration");
    let [region] = spans.as_slice() else {
        panic!("one region, got {spans:?}");
    };
    let source = format!("{HEADER}{FORM}\n");
    let start = source.find(FORM).expect("the form");
    assert_eq!(region["start"], start);
    assert_eq!(region["end"], start + FORM.len());
    // The package lists the definition as a selection and as no feature.
    let selections = wire["lock"]["definition_selections"]
        .as_array()
        .expect("selections");
    assert_eq!(
        selections.last(),
        Some(&serde_json::json!({"authority": "agent-ix", "identity": "retention"})),
        "the extension row is last, after the closed rows and laws"
    );
    assert!(!wire["lock"]["profile_selections"]
        .to_string()
        .contains("retention"));
    assert!(!wire["lock"]["required_features"]
        .as_array()
        .expect("features")
        .iter()
        .any(|feature| feature == "retention"));
}

#[trace("TC-882", "FR-354-AC-6")]
#[test]
fn the_declared_name_and_clause_values_enter_the_ids_and_the_span_enters_none() {
    let id = |node_kind: &str, declaration: &str| {
        let emitted = compile(&held(node_kind), declaration).expect("the unit compiles");
        let wire = wire(&emitted);
        let form = extension_node(&wire).clone();
        (form, wire["semantic_graph"].clone())
    };
    let (base, base_graph) = id("retention", FORM);
    // A different declared name is a different node id.
    let (renamed, _) = id(
        "retention",
        "retention Log { subject: \"audit\", days: 30 }",
    );
    assert_ne!(base["node_id"], renamed["node_id"]);
    // A different node kind and a different clause value change the node
    // (its body, whose clause targets are the subnodes' ids).
    let (kind, _) = id("audit_retention", FORM);
    assert_ne!(base["body"], kind["body"]);
    let (days, _) = id(
        "retention",
        "retention Audit { subject: \"audit\", days: 31 }",
    );
    assert_ne!(base["body"], days["body"]);
    assert_ne!(
        base["body"]["clauses"][1]["value"]["target"],
        days["body"]["clauses"][1]["value"]["target"]
    );
    // Two blank lines before the form move its span and change no id.
    let (moved, moved_graph) = id("retention", &format!("\n\n{FORM}"));
    assert_eq!(base["node_id"], moved["node_id"]);
    assert_eq!(base_graph, moved_graph);
}

#[trace("TC-882", "FR-354-AC-2")]
#[test]
fn an_integer_outside_the_declared_range_refuses_naming_the_clause_and_both_ranges() {
    for (days, actual) in [("0", "Int[0, 0]"), ("3651", "Int[3651, 3651]")] {
        let declaration = format!("retention Audit {{ subject: \"audit\", days: {days} }}");
        let refusal = compile(&held("retention"), &declaration).expect_err("out of range");
        let ExtensionCause::TypeMismatch {
            clause,
            expected,
            actual: found,
            ..
        } = refusal_cause(&refusal)
        else {
            panic!("expected a type mismatch, got {refusal:?}");
        };
        assert_eq!(clause, "days");
        assert_eq!(expected, "Int[1, 3650]");
        assert_eq!(found, actual);
    }
}

#[trace("TC-882", "FR-354-AC-2")]
#[test]
fn a_type_spelling_outside_the_v1_reader_refuses_naming_the_spelling() {
    let refusal = compile(&held_typed("retention", "Sequence<Integer>"), FORM).expect_err("unsupported");
    let ExtensionCause::TypeMismatch {
        clause, expected, ..
    } = refusal_cause(&refusal)
    else {
        panic!("expected a type mismatch, got {refusal:?}");
    };
    assert_eq!(clause, "days");
    assert_eq!(expected, "Sequence<Integer>");
}

/// An extension of `agent-ix`/`identity` with one clause-less `keyword` form.
fn bare_extension(identity: &str, keyword: &str) -> Definition {
    extension_definition(
        identity,
        &serde_json::json!({
            "grammar_schema": {"productions": [{"keyword": keyword, "clauses": []}]},
            "typed_node_schema": {"nodes": [
                {"keyword": keyword, "node_kind": keyword, "clauses": []}
            ]}
        }),
    )
}

#[trace("TC-490", "FR-110-AC-15")]
#[test]
fn extension_rows_end_the_lock_ascending_by_identity() {
    let catalog =
        DefinitionCatalog::new(vec![bare_extension("ext.a", "ka"), bare_extension("ext.b", "kb")])
            .unwrap();
    let source = format!(
        "{}profile B = \"ext.b\";\nprofile A = \"ext.a\";\nrecord Window {{ lo: Integer; }}\n",
        "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\";\n"
    );
    let emitted = compile_source(&catalog, &source).expect("the unit compiles");
    let wire = wire(&emitted);
    let selections = wire["lock"]["definition_selections"]
        .as_array()
        .expect("selections");
    let tail: Vec<_> = selections.iter().rev().take(2).rev().cloned().collect();
    assert_eq!(
        tail,
        [
            serde_json::json!({"authority": "agent-ix", "identity": "ext.a"}),
            serde_json::json!({"authority": "agent-ix", "identity": "ext.b"}),
        ]
    );
    let features = wire["lock"]["required_features"].to_string();
    assert!(!features.contains("ext."), "no feature row: {features}");
    assert!(!wire["lock"]["profile_selections"].to_string().contains("ext."));
}

#[trace("TC-490", "FR-110-AC-14")]
#[test]
fn a_header_identity_the_catalog_does_not_hold_as_an_extension_refuses_unknown_profile() {
    let source = format!("{HEADER}record Window {{ lo: Integer; }}\n");
    let refuse = |catalog: &DefinitionCatalog| {
        let refusal = compile_source(catalog, &source).expect_err("the profile is refused");
        let CompileRefusal::Profile { refusals, .. } = &*refusal else {
            panic!("expected a profile refusal, got {refusal:?}");
        };
        (refusals[0].code(), refusals[0].cause())
    };
    let other_role = extension_definition_with_role("retention", DefinitionRole::Source);
    for catalog in [DefinitionCatalog::default(), other_role] {
        assert_eq!(refuse(&catalog), (Code::UnknownProfile, "unsupported-selection"));
    }
}

fn extension_definition_with_role(identity: &str, role: DefinitionRole) -> DefinitionCatalog {
    let definition = Definition::from_exact_bytes(
        &ReaderAuthority::fixture(),
        "agent-ix",
        identity,
        role,
        BTreeSet::new(),
        BTreeSet::new(),
        b"{}",
    )
    .unwrap();
    DefinitionCatalog::new(vec![definition]).unwrap()
}

/// An extension `kinds` whose `scalars` form has one clause per `(name, type)`.
fn scalar_extension(clauses: &[(&str, &str)]) -> DefinitionCatalog {
    let names: Vec<&str> = clauses.iter().map(|(name, _)| *name).collect();
    let typed: Vec<Value> = clauses
        .iter()
        .map(|(name, ty)| serde_json::json!({"name": name, "type": ty}))
        .collect();
    let document = serde_json::json!({
        "grammar_schema": {"productions": [{"keyword": "retention", "clauses": names}]},
        "typed_node_schema": {"nodes": [
            {"keyword": "retention", "node_kind": "retention", "clauses": typed}
        ]}
    });
    DefinitionCatalog::new(vec![extension_definition("retention", &document)]).unwrap()
}

#[trace("TC-882", "FR-354-AC-7")]
#[test]
fn each_v1_clause_type_spelling_admits_its_value_and_an_unsupported_one_refuses() {
    let clauses = [
        ("b", "Boolean"),
        ("i", "Integer"),
        ("t", "Text"),
        ("r", "Int[0, 9]"),
        ("w", "Window"),
    ];
    let record = "record Window { lo: Integer; }\n";
    let form = "retention F { b: true, i: 7, t: \"x\", r: 3, w: Window { lo: 1 } }";
    let emitted = compile(&scalar_extension(&clauses), &format!("{record}{form}"))
        .expect("every v1 spelling admits its value");
    let wire = wire(&emitted);
    assert_eq!(
        extension_node(&wire)["body"]["clauses"].as_array().unwrap().len(),
        5
    );
    // `i` spelled `Sequence<Integer>` is held and refused at S3.
    let respelled = [
        ("b", "Boolean"),
        ("i", "Sequence<Integer>"),
        ("t", "Text"),
        ("r", "Int[0, 9]"),
        ("w", "Window"),
    ];
    let refusal = compile(&scalar_extension(&respelled), &format!("{record}{form}"))
        .expect_err("`Sequence<Integer>` is outside the v1 spellings");
    let ExtensionCause::TypeMismatch {
        clause, expected, actual, ..
    } = refusal_cause(&refusal)
    else {
        panic!("expected a type mismatch, got {refusal:?}");
    };
    assert_eq!((clause.as_str(), expected.as_str()), ("i", "Sequence<Integer>"));
    assert_eq!(actual, "Integer");
}

/// `retention` plus `archive <name> { policy: <expr> }` typing `policy` by the
/// node kind `retention`, and `link <name> { to: <expr> }` typing `to` by `link`.
fn node_kind_catalog() -> DefinitionCatalog {
    let document = serde_json::json!({
        "grammar_schema": {"productions": [
            {"keyword": "retention", "clauses": ["subject", "days"]},
            {"keyword": "archive", "clauses": ["policy"]},
            {"keyword": "link", "clauses": ["to"]}
        ]},
        "typed_node_schema": {"nodes": [
            {"keyword": "retention", "node_kind": "retention", "clauses": [
                {"name": "subject", "type": "Text"},
                {"name": "days", "type": "Int[1, 3650]"}
            ]},
            {"keyword": "archive", "node_kind": "archive", "clauses": [
                {"name": "policy", "type": "retention"}
            ]},
            {"keyword": "link", "node_kind": "link", "clauses": [
                {"name": "to", "type": "link"}
            ]}
        ]}
    });
    DefinitionCatalog::new(vec![extension_definition("retention", &document)]).unwrap()
}

#[trace("TC-882", "FR-354-AC-8")]
#[test]
fn a_node_kind_clause_references_the_named_form_whatever_the_order() {
    let emitted = compile(
        &node_kind_catalog(),
        &format!("archive Logs {{ policy: Audit }}\n{FORM}"),
    )
    .expect("a form may name a form declared after it");
    let wire = wire(&emitted);
    let archive = nodes(&wire)
        .iter()
        .find(|node| node["node_tag"] == "extension" && node["body"]["node_kind"] == "archive")
        .expect("the archive node");
    let retention = nodes(&wire)
        .iter()
        .find(|node| node["node_tag"] == "extension" && node["body"]["node_kind"] == "retention")
        .expect("the retention node");
    assert_eq!(
        archive["body"]["clauses"][0]["value"]["target"],
        retention["node_id"]
    );
}

#[trace("TC-882", "FR-354-AC-8")]
#[test]
fn a_node_kind_clause_naming_nothing_or_another_kind_refuses() {
    let missing = compile(&node_kind_catalog(), "archive Logs { policy: Missing }")
        .expect_err("`Missing` names no form");
    let CompileRefusal::Check { refusals, .. } = &*missing else {
        panic!("expected a check refusal, got {missing:?}");
    };
    assert_eq!(refusals[0].cause.code(), Code::MissingDeclaration);
    assert_eq!(refusals[0].cause.cause(), Some("missing-name"));
    assert!(matches!(&refusals[0].cause, CheckCause::MissingName(name) if name == "Missing"));

    let other = compile(
        &node_kind_catalog(),
        "archive Other { policy: Logs }\narchive Logs { policy: Other }",
    )
    .expect_err("`Logs` is an `archive`, not a `retention`");
    let ExtensionCause::TypeMismatch {
        clause,
        expected,
        actual,
        ..
    } = refusal_cause_first(&other)
    else {
        panic!("expected a type mismatch, got {other:?}");
    };
    assert_eq!((clause.as_str(), expected.as_str(), actual.as_str()), ("policy", "retention", "archive"));
}

#[trace("TC-882", "FR-354-AC-8")]
#[test]
fn forms_whose_node_kind_clauses_reach_themselves_refuse_at_every_form_on_the_cycle() {
    let cycle = compile(
        &node_kind_catalog(),
        "link A { to: B }\nlink B { to: A }",
    )
    .expect_err("A and B reach each other");
    let causes = extension_causes(&cycle);
    let named: Vec<_> = causes
        .iter()
        .map(|cause| match cause {
            ExtensionCause::Cycle { cycle, .. } => cycle.clone(),
            other => panic!("expected a cycle, got {other:?}"),
        })
        .collect();
    assert_eq!(
        named,
        [
            vec!["A".to_owned(), "B".to_owned(), "A".to_owned()],
            vec!["B".to_owned(), "A".to_owned(), "B".to_owned()],
        ]
    );
    for cause in causes {
        assert_eq!((cause.code(), cause.cause()), (Code::IllTyped, "type-mismatch"));
    }
    let own = compile(&node_kind_catalog(), "link C { to: C }").expect_err("C reaches itself");
    let [ExtensionCause::Cycle { cycle, .. }] = extension_causes(&own).as_slice() else {
        panic!("expected one cycle, got {own:?}");
    };
    assert_eq!(cycle, &["C".to_owned(), "C".to_owned()]);
}

#[trace("TC-883", "FR-354-AC-9")]
#[test]
fn an_extension_form_and_a_record_of_one_name_refuse_ambiguous_name() {
    let refusal = compile(
        &node_kind_catalog(),
        &format!("{FORM}\nrecord Audit {{ n: Integer; }}"),
    )
    .expect_err("one name, two declarations");
    assert_eq!(refusal.code(), Code::AmbiguousDeclaration);
    assert_eq!(refusal.cause(), Some("ambiguous-name"));
}

fn extension_causes(refusal: &CompileRefusal) -> Vec<&ExtensionCause> {
    let CompileRefusal::Check { refusals, .. } = refusal else {
        panic!("expected a check refusal, got {refusal:?}");
    };
    refusals
        .iter()
        .map(|refusal| match &refusal.cause {
            CheckCause::Extension(cause) => cause.as_ref(),
            other => panic!("expected an extension refusal, got {other:?}"),
        })
        .collect()
}

fn refusal_cause_first(refusal: &CompileRefusal) -> &ExtensionCause {
    extension_causes(refusal)[0]
}

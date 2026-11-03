// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-510 static subject behavior through the public spine facade.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::StageFailure;
use qsl_foundation::{SettingLimits, SourceIdentity};
use qsl_replay::spine::{self, ComponentKind, DependencyInput, ParseRequest, SpineLimits};
use qsl_replay::SuppliedLibrary;
use quire_exact::Cancel;
use serde_json::Value;

const MODEL: &[u8] = include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");
const ANSWER: &str = "function answer using v(): Integer pure { 1 }\n";
const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"quire.value.complete/v1\";\n\
model M = \"acme/orders\";\n\
function answer using v(): Integer pure { 1 }\n";

fn raised_limits() -> SpineLimits {
    let mut limits = SpineLimits::default();
    let bounds = limits.bounds();
    for (setting, bound) in bounds {
        assert!(limits.set_bound(setting, bound + 1), "{setting:?}");
    }
    limits
}

fn checked(
    source_text: &str,
    model: &[u8],
    limits: SpineLimits,
    libraries: Vec<SuppliedLibrary>,
) -> spine::CheckedUnit {
    let identity = SourceIdentity::new("test", "unit", "git", "1");
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "unit.native",
            bytes: source_text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .unwrap_or_else(|failure| panic!("parse refused source:\n{source_text}\n{failure:?}"))
    .into_value();
    let input = [("acme/orders".to_owned(), model.to_vec())]
        .into_iter()
        .collect();
    let admitted = spine::select(&parsed, &input, limits.model, &cancel)
        .expect("select")
        .into_value();
    let dependencies = DependencyInput::new(libraries).expect("dependency input");
    spine::check(
        &parsed,
        "fixture".to_owned(),
        &admitted,
        &dependencies,
        limits,
        &cancel,
    )
    .expect("check")
    .into_value()
}

fn library(identity: &str, name: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.to_owned(),
        source: SourceIdentity::new("test", name, "git", "1"),
        path: format!("{name}.native"),
        bytes: b"language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\npredicate from_library using v (x: Integer): Boolean { x > 0 }\n".to_vec(),
    }
}

fn library_importing(
    identity: &str,
    name: &str,
    dependency_identity: &str,
    dependency_alias: &str,
) -> SuppliedLibrary {
    let mut library = library(identity, name);
    let source = String::from_utf8(library.bytes).expect("fixture source is UTF-8");
    library.bytes = source
        .replace(
            "profile v =",
            &format!("import \"{dependency_identity}\" as {dependency_alias};\nprofile v ="),
        )
        .into_bytes();
    library
}

#[trace("FR-510-AC-1", "TC-1305")]
#[test]
fn raised_limits_change_provenance_without_changing_subject() {
    let normal = checked(SOURCE, MODEL, SpineLimits::default(), vec![]);
    let raised = checked(SOURCE, MODEL, raised_limits(), vec![]);
    assert_eq!(
        normal.static_subject().unwrap(),
        raised.static_subject().unwrap()
    );
    assert!(normal
        .static_subject()
        .unwrap()
        .differences(raised.static_subject().unwrap())
        .is_empty());
    assert_ne!(normal.build_provenance(), raised.build_provenance());
}

#[trace("FR-510-AC-2", "TC-1305")]
#[test]
fn emitted_unit_retains_the_same_static_subject() {
    let checked = checked(SOURCE, MODEL, SpineLimits::default(), vec![]);
    let before = checked.static_subject().unwrap().clone();
    let emitted = spine::package(&checked, spine::PackageLimits::default(), &Cancel::new())
        .expect("emit")
        .into_value();
    assert_eq!(&before, emitted.static_subject().unwrap());
    assert!(before
        .differences(emitted.static_subject().unwrap())
        .is_empty());
}

#[trace("FR-510-AC-6", "TC-1305")]
#[test]
fn static_subject_comparison_reports_component_differences() {
    let baseline = checked(SOURCE, MODEL, SpineLimits::default(), vec![]);
    let edited = checked(
        &SOURCE.replace("{ 1 }", "{ 2 }"),
        MODEL,
        SpineLimits::default(),
        vec![],
    );
    let subject = baseline.static_subject().unwrap();
    let repeat = checked(SOURCE, MODEL, SpineLimits::default(), vec![]);
    assert_eq!(subject, repeat.static_subject().unwrap());
    assert_eq!(
        subject.differences(edited.static_subject().unwrap()),
        [ComponentKind::Sources]
    );
}

#[trace("FR-510-AC-3", "TC-1305")]
#[test]
fn static_changes_are_reported_by_their_exact_component() {
    let baseline = checked(SOURCE, MODEL, SpineLimits::default(), vec![]);
    let base = baseline.static_subject().unwrap();

    let changed_source = SOURCE.replace("{ 1 }", "{ 2 }");
    let source_subject = checked(&changed_source, MODEL, SpineLimits::default(), vec![]);
    assert_eq!(
        base.differences(source_subject.static_subject().unwrap()),
        [ComponentKind::Sources]
    );

    let mut changed_model: Value = serde_json::from_slice(MODEL).unwrap();
    changed_model["package"]["version"] = Value::String("2.0.0".to_owned());
    changed_model["source"]["version"] = Value::String("2.0.0".to_owned());
    let changed_model = serde_json::to_vec(&changed_model).unwrap();
    let model_subject = checked(SOURCE, &changed_model, SpineLimits::default(), vec![]);
    assert_eq!(
        base.differences(model_subject.static_subject().unwrap()),
        [ComponentKind::Models]
    );

    let unused = checked(
        SOURCE,
        MODEL,
        SpineLimits::default(),
        vec![library("unused", "unused")],
    );
    assert!(base
        .differences(unused.static_subject().unwrap())
        .is_empty());

    let profile_base = SOURCE.replace(ANSWER, "record Baseline { sample: Integer; }\n");
    let profile_source = profile_base.replace("quire.value.complete/v1", "quire.state.core/v1");
    let profile_baseline = checked(&profile_base, MODEL, SpineLimits::default(), vec![]);
    let profile = checked(&profile_source, MODEL, SpineLimits::default(), vec![]);
    assert_eq!(
        profile_baseline
            .static_subject()
            .unwrap()
            .differences(profile.static_subject().unwrap()),
        [ComponentKind::Sources, ComponentKind::Profiles]
    );
}

#[trace("FR-510-AC-3", "TC-1305")]
#[test]
fn importing_a_resolved_library_changes_sources_and_dependencies() {
    let imported = SOURCE.replace("model M =", "import \"lib\" as d;\nmodel M =");
    let subject = checked(
        &imported,
        MODEL,
        SpineLimits::default(),
        vec![library("lib", "lib")],
    );
    let baseline = checked(SOURCE, MODEL, SpineLimits::default(), vec![]);
    assert_eq!(
        baseline
            .static_subject()
            .unwrap()
            .differences(subject.static_subject().unwrap()),
        [ComponentKind::Sources, ComponentKind::Dependencies]
    );
}

#[trace("FR-510-AC-3", "TC-1305")]
#[test]
fn predicate_reference_rows_retain_each_occurrence_owner_and_target() {
    let source = SOURCE
        .replace("model M =", "import \"lib\" as d;\nmodel M =")
        .replace(
            ANSWER,
            "function other using v(): Boolean pure { d::from_library(1) }\n\
             function answer using v(): Boolean pure { d::from_library(1) and d::from_library(2) }\n",
        );
    let libraries = vec![library("lib", "lib")];
    let checked_unit = checked(&source, MODEL, SpineLimits::default(), libraries.clone());
    let references = checked_unit
        .static_subject()
        .unwrap()
        .dependency_references();
    let unreached = source
        .replace("d::from_library(1)", "true")
        .replace("d::from_library(2)", "true");
    let baseline = checked(&unreached, MODEL, SpineLimits::default(), libraries.clone());
    assert_eq!(
        baseline
            .static_subject()
            .unwrap()
            .differences(checked_unit.static_subject().unwrap()),
        [ComponentKind::Sources, ComponentKind::Dependencies]
    );
    let repeated_check = checked(&source, MODEL, SpineLimits::default(), libraries);
    assert_eq!(
        references,
        repeated_check
            .static_subject()
            .unwrap()
            .dependency_references()
    );
    assert_eq!(references.len(), 3, "each call occurrence is retained");
    assert_eq!(
        references
            .iter()
            .map(|reference| reference.referring_declaration.as_str())
            .collect::<Vec<_>>(),
        ["other", "answer", "answer"]
    );
    assert!(references
        .iter()
        .all(|reference| reference.kind
            == qsl_semantics::check::CheckedDependencyKind::PredicateCall));
    assert_eq!(references[1].target, references[2].target);
    assert_ne!(references[1].site, references[2].site);
    assert!(references
        .iter()
        .all(|reference| reference.library.as_deref() == Some("lib")));
}

#[trace("FR-510-AC-3", "TC-1305")]
#[test]
fn imported_predicate_reference_retains_the_exact_library_target() {
    let source = SOURCE
        .replace("model M =", "import \"lib\" as d;\nmodel M =")
        .replace(
            ANSWER,
            "predicate answer using v (): Boolean { d::from_library(1) }\n",
        );
    let checked_unit = checked(
        &source,
        MODEL,
        SpineLimits::default(),
        vec![library("lib", "lib")],
    );
    let subject = checked_unit.static_subject().unwrap();
    let [reference] = subject.dependency_references() else {
        panic!(
            "one imported predicate reference: {:?}",
            subject.dependency_references()
        );
    };
    assert_eq!(reference.referring_declaration, "answer");
    assert_eq!(reference.name, "d::from_library");
    assert_eq!(reference.library.as_deref(), Some("lib"));
    assert!(subject.dependencies()[0]
        .declaration_kinds
        .contains(&qsl_semantics::check::CheckedDependencyKind::PredicateCall));
}

#[trace("FR-510-AC-3", "TC-1305")]
#[test]
fn aggregate_units_keep_sibling_imports_and_references_out_of_each_subject() {
    let source_a = SourceIdentity::new("test", "unit-a", "git", "1");
    let source_b = SourceIdentity::new("test", "unit-b", "git", "1");
    let unit_a = SOURCE
        .replace("model M =", "import \"lib-a\" as d;\nmodel M =")
        .replace(
            ANSWER,
            "function calls using v(x: Integer): Boolean pure { d::from_library(x) }\n",
        );
    let unit_b_header = SOURCE
        .replace("acme/orders", "acme/inventory")
        .replace(
            "profile v = \"quire.value.complete/v1\";\n",
            "profile v = \"quire.value.complete/v1\";\nprofile p = \"quire.protocol.finite-global/v1\";\n",
        )
        .replace(ANSWER, "");
    let unit_b = format!(
        "{}{}function plain using v(): Integer pure {{ 1 }}\n",
        unit_b_header,
        protocol("FlowB", "agent")
    );
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parse_unit = |source: &SourceIdentity, bytes: &str| {
        spine::parse(
            &ParseRequest {
                source,
                path: "unit.native",
                bytes: bytes.as_bytes(),
            },
            limits.source,
            &cancel,
        )
        .expect("parse unit")
        .into_value()
    };
    let dependencies = DependencyInput::new(vec![
        library_importing("lib-a", "lib-a", "lib-b", "b"),
        library_importing("lib-b", "lib-b", "lib-c", "c"),
        library("lib-c", "lib-c"),
    ])
    .expect("input");
    let admitted = spine::admit_parsed_unit_list(
        vec![
            parse_unit(&source_a, &unit_a),
            parse_unit(&source_b, &unit_b),
        ],
        &dependencies,
    )
    .expect("distinct units are admitted");
    let inventory_model = String::from_utf8(MODEL.to_vec())
        .expect("fixture JSON is UTF-8")
        .replace("acme/orders", "acme/inventory")
        .into_bytes();
    let model_input = [
        ("acme/orders".to_owned(), MODEL.to_vec()),
        ("acme/inventory".to_owned(), inventory_model),
    ]
    .into_iter()
    .collect();
    let models = spine::select_units(
        &admitted,
        &model_input,
        limits.model,
        limits.source,
        &cancel,
    )
    .expect("unit model selections are admitted")
    .into_value();
    let checked = spine::check_units(
        &admitted,
        "aggregate-static-subject".to_owned(),
        &models,
        &dependencies,
        limits,
        &cancel,
    )
    .expect("aggregate units check")
    .into_value();
    let checked_a = checked
        .iter()
        .find(|unit| unit.source().identity() == &source_a)
        .expect("unit A is checked");
    let checked_b = checked
        .iter()
        .find(|unit| unit.source().identity() == &source_b)
        .expect("unit B is checked");
    let subject_a = checked_a.static_subject().unwrap();
    assert_eq!(
        subject_a
            .dependencies()
            .iter()
            .map(|dependency| dependency.identity.as_str())
            .collect::<Vec<_>>(),
        ["lib-a", "lib-b", "lib-c"]
    );
    let [reference] = subject_a.dependency_references() else {
        panic!("unit A retains its own imported call: {:?}", subject_a);
    };
    assert_eq!(reference.referring_declaration, "calls");
    assert_eq!(reference.library.as_deref(), Some("lib-a"));
    assert_eq!(subject_a.sources().len(), 4);

    let subject_b = checked_b.static_subject().unwrap();
    assert!(subject_b.dependencies().is_empty());
    assert!(subject_b.dependency_references().is_empty());
    assert_eq!(subject_b.sources().len(), 1);
    assert_eq!(subject_b.models()[0].package, "acme/inventory");
    assert_eq!(subject_b.roles()[0].type_identity, "acme/inventory::Gadget");
}

#[trace("FR-510-AC-3", "TC-1305")]
#[test]
fn temporal_requirement_reference_retains_owner_and_checked_target() {
    let header = SOURCE
        .replace(
            "model M =",
            "profile p = \"quire.protocol.finite-global/v1\";\nprofile t = \"quire.temporal.infinite-trace/v1\";\nmodel M =",
        )
        .replace(ANSWER, "");
    let source = format!(
        "{}temporal Due using t over (item: M::Gadget) clock \"steps\" on origin {{\n\
           always holds(d::from_library(1))\n\
         }}\n\
         {}",
        header,
        protocol("Flow", "x").replace(
            "role x on M::Gadget;\n",
            "role x on M::Gadget;\nrequires temporal Due;\n",
        ),
    );
    let source = source.replace("model M =", "import \"lib\" as d;\nmodel M =");
    let checked_unit = checked(
        &source,
        MODEL,
        SpineLimits::default(),
        vec![library("lib", "lib")],
    );
    let references = checked_unit
        .static_subject()
        .unwrap()
        .dependency_references();
    assert_eq!(
        references.len(),
        2,
        "formula call and requirement are retained"
    );
    let call = references
        .iter()
        .find(|reference| {
            reference.kind == qsl_semantics::check::CheckedDependencyKind::PredicateCall
        })
        .expect("the temporal formula's imported call");
    assert_eq!(call.referring_declaration, "Due");
    assert_eq!(call.name, "d::from_library");
    assert_eq!(call.library.as_deref(), Some("lib"));
    let reference = references
        .iter()
        .find(|reference| {
            reference.kind == qsl_semantics::check::CheckedDependencyKind::TemporalObligation
        })
        .expect("the protocol requirement");
    assert_eq!(reference.referring_declaration, "Flow");
    assert_eq!(reference.name, "Due");
    assert_eq!(
        reference.target,
        checked_unit
            .package()
            .graph()
            .temporal_clauses()
            .iter()
            .find(|clause| clause.clause().name() == "Due")
            .expect("Due has a checked temporal target")
            .identity()
    );
    assert!(matches!(
        &reference.site,
        qsl_semantics::check::CheckedDependencySite::TemporalRequirement(_)
    ));
    assert!(reference.library.is_none());
}

#[trace("FR-510-AC-3", "TC-1305")]
#[test]
fn operation_contract_reference_retains_owner_site_and_checked_target() {
    let mut model: Value = serde_json::from_slice(MODEL).unwrap();
    let gadget = model["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|record| record["displayName"] == "Gadget")
        .unwrap();
    gadget["operations"] = serde_json::json!([{
        "identity": "ix://acme/orders/Gadget/attemptUpdate",
        "name": "attemptUpdate",
        "params": [],
        "returns": {
            "typeRef": "ix://quire/native/Boolean",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            "nullable": false
        },
        "pre": [],
        "post": [],
        "origin": {"source": {
            "sourceIdentity": "ix://acme/orders/spec",
            "path": "model.json",
            "startLine": 1,
            "startColumn": 1
        }},
        "frame": {"modifies": [], "creates": [], "deletes": []}
    }]);
    let model = serde_json::to_vec(&model).unwrap();
    let header = SOURCE
        .replace(
            "model M =",
            "profile p = \"quire.protocol.finite-global/v1\";\nmodel M =",
        )
        .replace(ANSWER, "");
    let source = format!(
        "{}pre Ready using v on M::Gadget::attemptUpdate {{ true }}\n\
         {}{}",
        header,
        protocol("Flow", "x")
            .replace("run sequence Main { }", "run sequence Main { attempt Tried by x on M::Gadget::attemptUpdate contracts [Ready] as (attempted: Boolean) { true }; }"),
        ANSWER
    );
    let checked_unit = checked(&source, &model, SpineLimits::default(), vec![]);
    let references = checked_unit
        .static_subject()
        .unwrap()
        .dependency_references();
    let [reference] = references else {
        panic!("one operation contract reference: {references:?}");
    };
    assert_eq!(reference.referring_declaration, "Flow");
    assert_eq!(
        reference.kind,
        qsl_semantics::check::CheckedDependencyKind::OperationContract
    );
    assert_eq!(reference.name, "Ready");
    assert_eq!(
        reference.target,
        checked_unit
            .package()
            .graph()
            .state_clause("Ready")
            .expect("Ready has a checked operation contract")
            .identity()
    );
    assert!(matches!(
        &reference.site,
        qsl_semantics::check::CheckedDependencySite::OperationContract(_)
    ));
    assert!(reference.library.is_none());
}

#[trace("FR-510-AC-4", "TC-1305")]
#[test]
fn roles_with_equal_names_keep_distinct_owners() {
    let declarations = SOURCE
        .replace(
            "model M =",
            "profile p = \"quire.protocol.finite-global/v1\";\nmodel M =",
        )
        .replace(ANSWER, "");
    let source = format!(
        "{}{}{}{}",
        declarations,
        protocol("First", "x"),
        protocol("Second", "x"),
        ANSWER
    );
    let checked_unit = checked(&source, MODEL, SpineLimits::default(), vec![]);
    let subject = checked_unit.static_subject().unwrap();
    assert_eq!(subject.roles().len(), 2);
    assert_ne!(subject.roles()[0].owner, subject.roles()[1].owner);
    let renamed = source.replacen("role x on", "role y on", 1);
    let renamed = checked(&renamed, MODEL, SpineLimits::default(), vec![]);
    assert_eq!(
        subject.differences(renamed.static_subject().unwrap()),
        [ComponentKind::Sources, ComponentKind::Roles]
    );
}

fn protocol(name: &str, role: &str) -> String {
    format!(
        "protocol {name} using p over (input: M::Gadget) on origin {{\n\
         role {role} on M::Gadget;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\
         }}\n"
    )
}

#[trace("FR-510-AC-5", "TC-1305")]
#[test]
fn a_refused_check_produces_no_static_subject() {
    let bad_source = SOURCE
        .replace("function answer", "function broken")
        .replace("{ 1 }", "{ unknown }");
    let identity = SourceIdentity::new("test", "bad", "git", "1");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "bad.native",
            bytes: bad_source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("parse")
    .into_value();
    let input = [("acme/orders".to_owned(), MODEL.to_vec())]
        .into_iter()
        .collect();
    let admitted = spine::select(&parsed, &input, limits.model, &cancel)
        .expect("select")
        .into_value();
    let refusal = spine::check(
        &parsed,
        "fixture".to_owned(),
        &admitted,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .expect_err("invalid body is refused");
    assert!(matches!(
        refusal,
        StageFailure::Refused(_) | StageFailure::RefusedWithPayload(_, _)
    ));
}

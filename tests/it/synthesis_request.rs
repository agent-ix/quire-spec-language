// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1036 (TC-3016) and FR-1037 (TC-3017): a `synthesis` declaration is
//! emitted as one `claim`/`synthesis_request` node (FSpec FR-322
//! "Synthesis-request nodes"), and the run request's one `synthesis` member
//! selects a declaration by its declared name and runs a value synthesis,
//! writing the `filament.synthesis-outcome/v1` document (FSpec FR-823).

use fsl_foundation::SourceIdentity;
use fsl_replay::spine::DependencyInput;
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::support::config_version_spine::{spine, Case};
use crate::support::front_end::emitted;

const VALUE_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.value.complete/v1\";\n";

const MODEL_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"filament.model.complete/v1\";\n\
     model Config = \"example/config-version\";\n";

fn pick(block: &str, objective: &str) -> String {
    format!(
        "{VALUE_HEADER}type Digit = Int[0, 9];\n\
         synthesis Pick using v grammar Digit domain Digit {objective} satisfies {{ {block} }};\n"
    )
}

/// The emitted package of `unit` as JSON.
fn package(unit: &str, packages: &fsl_semantics::model::intake::PackageInput) -> Value {
    let emission = emitted(
        SourceIdentity::new("test", "synthesis-request"),
        "unit.native",
        unit.as_bytes(),
        packages,
        &DependencyInput::default(),
    )
    .unwrap_or_else(|refusal| panic!("the unit compiles and emits: {refusal:?}"));
    serde_json::from_slice(emission.package().bytes()).expect("a JSON package")
}

/// The emitted package bytes of `unit`.
fn package_bytes(unit: &str, packages: &fsl_semantics::model::intake::PackageInput) -> Vec<u8> {
    emitted(
        SourceIdentity::new("test", "synthesis-request"),
        "unit.native",
        unit.as_bytes(),
        packages,
        &DependencyInput::default(),
    )
    .unwrap_or_else(|refusal| panic!("the unit compiles and emits: {refusal:?}"))
    .package()
    .bytes()
    .to_vec()
}

/// The package's `claim`/`synthesis_request` nodes.
fn synthesis_nodes(wire: &Value) -> Vec<&Value> {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("a node list")
        .iter()
        .filter(|node| node["node_tag"] == "claim" && node["semantic_form"] == "synthesis_request")
        .collect()
}

/// The body binding names of `node`, in order.
fn binding_names(node: &Value) -> Vec<&str> {
    node["body"]["members"]
        .as_array()
        .expect("an aggregate body")
        .iter()
        .filter_map(|member| member["name"].as_str())
        .collect()
}

fn binding<'a>(node: &'a Value, name: &str) -> &'a Value {
    node["body"]["members"]
        .as_array()
        .expect("an aggregate body")
        .iter()
        .find(|member| member["name"] == name)
        .unwrap_or_else(|| panic!("binding {name}: {node}"))
}

/// FR-1036-AC-1, FSpec FR-322-AC-467: `Pick` is one declared
/// `synthesis_request` node with the six bindings and one `claim_names` row.
#[test]
#[trace("TC-3016", "FR-1036-AC-1")]
#[trace("FSpec-TC-2037", "FSpec-FR-322-AC-467")]
fn a_value_synthesis_is_one_synthesis_request_node() {
    let wire = package(&pick("Pick * Pick = 49", ""), &Default::default());
    let nodes = synthesis_nodes(&wire);
    assert_eq!(nodes.len(), 1, "{wire}");
    let node = nodes[0];
    assert_eq!(node["node_id"]["path"], json!(["Pick"]), "{node}");
    assert_eq!(
        binding_names(node),
        [
            "grammar",
            "domain",
            "candidate",
            "objective",
            "satisfies",
            "symmetry"
        ],
        "{node}"
    );
    assert_eq!(
        binding(node, "grammar")["value"],
        binding(node, "domain")["value"],
        "{node}"
    );
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let target = |name: &str| {
        let reference = &binding(node, name)["value"]["target"];
        nodes
            .iter()
            .find(|candidate| &candidate["node_id"] == reference)
            .unwrap_or_else(|| panic!("{name} resolves: {node}"))
    };
    // `Digit` is `Int[0, 9]`: both names are the one integer-range node S3
    // resolved, not a node minted for the declaration.
    for name in ["grammar", "domain"] {
        assert_eq!(target(name)["node_tag"], "bounded_domain", "{node}");
        assert_eq!(target(name)["semantic_form"], "integer_range", "{node}");
    }
    // The candidate is the `value`/`parameter` node of the declaration's own
    // name; a binder is referenced by its id and is not a graph node.
    assert!(
        binding(node, "candidate")["value"]["target"]["path"]
            .as_array()
            .is_some_and(|path| path.contains(&json!("parameter"))
                && path.contains(&json!("Pick"))),
        "the candidate parameter is named for the declaration: {node}"
    );
    assert_eq!(
        binding(node, "objective")["value"]["members"],
        json!([]),
        "{node}"
    );
    assert_eq!(
        binding(node, "symmetry")["value"]["value"],
        "identity",
        "{node}"
    );
    let rows = wire["claim_names"].as_array().expect("claim_names rows");
    assert_eq!(
        rows.iter().filter(|row| row["name"] == "Pick").count(),
        1,
        "{wire}"
    );
    let objective = package(&pick("Pick >= 3", "minimize Pick"), &Default::default());
    let node = synthesis_nodes(&objective)[0];
    let members = &binding(node, "objective")["value"]["members"];
    assert_eq!(members[0]["value"]["value"], "minimize", "{node}");
    assert_eq!(members[2]["value"]["value"], "Pick", "{node}");
}

/// FR-1036-AC-2: a plan synthesis's node names the object type as `grammar`
/// and the length range as `domain`, types its candidate by the object type (a block comparing it to itself),
/// and the strict reader reads the declaration back as a plan synthesis.
#[test]
#[trace("TC-3016", "FR-1036-AC-2")]
fn a_plan_synthesis_node_names_the_object_type_and_the_lengths() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let document = std::fs::read(directory.path().join("spine-model.semantic-ir.json")).unwrap();
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the spine model admits");
    let unit = format!(
        "{MODEL_HEADER}type Lengths = Int[0, 3];\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         satisfies {{ Reach = Reach }};\n"
    );
    let bytes = package_bytes(&unit, &packages);
    let wire: Value = serde_json::from_slice(&bytes).expect("a JSON package");
    let node = synthesis_nodes(&wire)[0];
    let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
    let target = |name: &str| {
        let reference = &binding(node, name)["value"]["target"];
        nodes
            .iter()
            .find(|candidate| &candidate["node_id"] == reference)
            .unwrap_or_else(|| panic!("{name} resolves: {node}"))
    };
    assert_eq!(target("grammar")["node_tag"], "model", "{node}");
    assert_eq!(target("grammar")["semantic_form"], "object_type", "{node}");
    assert_eq!(target("domain")["semantic_form"], "integer_range", "{node}");
    // The binder is referenced by id, not a graph node: it is a
    // `value`/`parameter` named for the declaration.
    let candidate = &binding(node, "candidate")["value"]["target"];
    let path = candidate["path"].as_array().expect("a path");
    assert!(
        path.contains(&json!("parameter")) && path.contains(&json!("Reach")),
        "the candidate is the declaration's own parameter: {candidate}"
    );
    assert!(
        !binding(node, "satisfies")["value"].is_null(),
        "the block is a node: {node}"
    );
    assert_eq!(
        binding(node, "objective")["value"]["members"],
        json!([]),
        "{node}"
    );

    let admitted = fsl_package::admit_checked_package(
        &bytes,
        &packages,
        fsl_package::V2ReadLimits::bounded(fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT),
    )
    .expect("the reader admits the package");
    let row = admitted
        .claim_names()
        .unwrap_or_default()
        .iter()
        .find(|row| row.name.as_ref() == "Reach")
        .expect("Reach has a claim-name row");
    assert!(
        fsl_package::is_plan_synthesis(&admitted, &row.declaration),
        "the reader reads the object type as the grammar"
    );
}

/// FR-1036-AC-2: the candidate reference resolves to the `value`/`parameter`
/// node of the declaration's own name, typed by the domain for a value
/// synthesis and by the `ConfigVersion` node for a plan synthesis.
#[test]
#[trace("TC-3016", "FR-1036-AC-2")]
#[ignore = "QSL-2235: candidate reference does not resolve to the typed parameter node; needs FR-1036-AC-2 ruling"]
fn the_candidate_reference_resolves_to_its_typed_parameter_node() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let document = std::fs::read(directory.path().join("spine-model.semantic-ir.json")).unwrap();
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the spine model admits");
    let plan = format!(
        "{MODEL_HEADER}type Lengths = Int[0, 3];\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         satisfies {{ Reach = Reach }};\n"
    );
    for (unit, packages, typed_by) in [
        (pick("Pick * Pick = 49", ""), Default::default(), "domain"),
        (plan, packages, "grammar"),
    ] {
        let wire = package(&unit, &packages);
        let node = synthesis_nodes(&wire)[0];
        let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
        let candidate = nodes
            .iter()
            .find(|candidate| candidate["node_id"] == binding(node, "candidate")["value"]["target"])
            .unwrap_or_else(|| panic!("the candidate resolves: {node}"));
        assert_eq!(candidate["node_tag"], "value", "{candidate}");
        assert_eq!(candidate["semantic_form"], "parameter", "{candidate}");
        assert_eq!(
            candidate["semantic_type"],
            binding(node, typed_by)["value"]["target"],
            "the candidate is typed by the {typed_by} node: {candidate}"
        );
    }
}

// ---- FR-1037: the run request's `synthesis` member ----------------------

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:w60:{file}"),
        "document": "W60", "formal_revision": 1})
}

/// Runs `unit` through the command adapter with `synthesis` as the
/// request's `synthesis` member, returning its exit code and document.
fn run(unit: &str, synthesis: Value) -> (u8, Value) {
    run_with(unit, json!({"synthesis": synthesis}))
}

/// Runs `unit` with `members` added to the request beside its program.
fn run_with(unit: &str, members: Value) -> (u8, Value) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("unit.native"), unit).unwrap();
    let mut request = json!({"format": "native-run/1", "request": {
        "models": [], "program": {"source": source("unit.native")}}});
    for (name, member) in members.as_object().unwrap() {
        request["request"][name] = member.clone();
    }
    let path = directory.path().join("request.json");
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (
            result.exit_code,
            serde_json::from_str(&result.value.to_string()).expect("a JSON document"),
        ),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

fn names(value: &Value, text: &str) {
    assert!(value.to_string().contains(text), "names {text}: {value}");
}

const OUTCOME_MEMBERS: [&str; 13] = [
    "format",
    "package",
    "declaration",
    "status",
    "check",
    "candidate",
    "objective",
    "optimal",
    "explored",
    "lineage",
    "counterexample",
    "limits",
    "cause",
];

/// FR-1037-AC-1, FSpec FR-823-AC-1/AC-2: the outcome document for a
/// validated, an exhausted, an optimal and a limited run.
#[test]
#[trace("TC-3017", "FR-1037-AC-1")]
#[trace("TC-2104", "FR-754-AC-3")]
#[trace("FSpec-TC-2038", "FSpec-FR-823-AC-1")]
#[trace("FSpec-TC-2038", "FSpec-FR-823-AC-2")]
fn the_synthesis_member_writes_the_synthesis_outcome() {
    let (exit, found) = run(
        &pick("Pick * Pick = 49", ""),
        json!({"declaration": "Pick"}),
    );
    let outcome = &found["synthesis"];
    assert_eq!(exit, 0, "{found}");
    assert_eq!(
        outcome["format"], "filament.synthesis-outcome/v1",
        "{found}"
    );
    for member in OUTCOME_MEMBERS {
        assert!(outcome.get(member).is_some(), "{member}: {found}");
    }
    assert_eq!(outcome["status"], "candidate", "{found}");
    assert_eq!(outcome["check"], "validated", "{found}");
    assert_eq!(
        outcome["candidate"],
        json!({"type": "integer", "value": "7"}),
        "{found}"
    );
    assert_eq!(outcome["explored"]["visited"], 8, "{found}");
    assert_eq!(outcome["explored"]["work"], 8, "{found}");
    assert_eq!(outcome["lineage"]["index"], 7, "{found}");
    let (exit, none) = run(
        &pick("Pick * Pick = 50", ""),
        json!({"declaration": "Pick"}),
    );
    assert_eq!(exit, 10, "{none}");
    assert_eq!(none["synthesis"]["status"], "no-candidate", "{none}");
    assert_eq!(none["synthesis"]["explored"]["visited"], 10, "{none}");
    assert_eq!(none["synthesis"]["candidate"], Value::Null, "{none}");
    let (_, optimal) = run(
        &pick("Pick >= 3", "minimize Pick"),
        json!({"declaration": "Pick"}),
    );
    assert_eq!(
        optimal["synthesis"]["candidate"],
        json!({"type": "integer", "value": "3"}),
        "{optimal}"
    );
    assert_eq!(optimal["synthesis"]["optimal"], true, "{optimal}");
    assert_eq!(optimal["synthesis"]["explored"]["work"], 4, "{optimal}");
    assert_eq!(
        optimal["synthesis"]["objective"]["direction"], "minimize",
        "{optimal}"
    );
    assert_eq!(optimal["synthesis"]["explored"]["visited"], 4, "{optimal}");
    assert_eq!(
        optimal["synthesis"]["objective"]["expression"], "Pick",
        "{optimal}"
    );
    assert_eq!(
        optimal["synthesis"]["objective"]["value"],
        json!({"type": "integer", "value": "3"}),
        "{optimal}"
    );
    let (exit, maximum) = run(
        &pick("Pick >= 3", "maximize Pick"),
        json!({"declaration": "Pick"}),
    );
    assert_eq!(exit, 0, "{maximum}");
    assert_eq!(
        maximum["synthesis"]["candidate"],
        json!({"type": "integer", "value": "9"}),
        "{maximum}"
    );
    assert_eq!(maximum["synthesis"]["optimal"], true, "{maximum}");
    assert_eq!(maximum["synthesis"]["explored"]["visited"], 1, "{maximum}");
    assert_eq!(maximum["synthesis"]["lineage"]["index"], 0, "{maximum}");
    let (exit, last_unit) = run(
        &pick("Pick >= 3", "minimize Pick"),
        json!({"declaration": "Pick", "limits": {"max_work": 4}}),
    );
    assert_eq!(exit, 0, "the check is not charged to max_work: {last_unit}");
    assert_eq!(last_unit["synthesis"]["check"], "validated", "{last_unit}");
    assert_eq!(last_unit["synthesis"]["explored"]["work"], 4, "{last_unit}");
    let (exit, limited_objective) = run(
        &pick("Pick >= 3", "minimize Pick"),
        json!({"declaration": "Pick", "limits": {"max_work": 3}}),
    );
    assert_eq!(exit, 22, "{limited_objective}");
    assert_eq!(limited_objective["synthesis"]["status"], "incomplete");
    assert_eq!(limited_objective["synthesis"]["candidate"], Value::Null);
    assert_eq!(limited_objective["synthesis"]["explored"]["work"], 3);
    names(
        &limited_objective["synthesis"]["cause"],
        "synthesis.max_work",
    );
    let (exit, limited) = run(
        &pick("Pick * Pick = 50", ""),
        json!({"declaration": "Pick", "limits": {"max_candidates": 3}}),
    );
    assert_eq!(exit, 22, "{limited}");
    assert_eq!(limited["synthesis"]["status"], "incomplete", "{limited}");
    names(&limited["synthesis"]["cause"], "synthesis.max_candidates");
    let (exit, deadline) = run(
        &pick("Pick = 7", ""),
        json!({"declaration": "Pick", "accounting": {}, "deadline_ms": 0}),
    );
    assert_eq!(exit, 22, "{deadline}");
    assert_eq!(deadline["synthesis"]["status"], "incomplete", "{deadline}");
    assert_eq!(
        deadline["synthesis"]["explored"]["visited"], 0,
        "{deadline}"
    );
    assert_eq!(deadline["synthesis"]["explored"]["work"], 0, "{deadline}");
    assert_eq!(
        deadline["synthesis"]["cause"]["kind"], "deadline",
        "{deadline}"
    );
}

/// FR-1037-AC-2, FSpec FR-823-AC-3/AC-4: selection by declared name, an
/// omitted name over one and two declarations, and kind-closed members.
#[test]
#[trace("TC-3017", "FR-1037-AC-2")]
#[trace("FSpec-TC-2038", "FSpec-FR-823-AC-3")]
#[trace("FSpec-TC-2038", "FSpec-FR-823-AC-4")]
fn the_synthesis_member_selects_one_declaration_or_refuses() {
    let only = pick("Pick * Pick = 49", "");
    let (exit, value) = run(&only, json!({}));
    assert_eq!(exit, 0, "{value}");
    assert_eq!(value["synthesis"]["declaration"], "Pick", "{value}");
    let two = format!(
        "{only}synthesis Other using v grammar Digit domain Digit satisfies {{ Other = 1 }};\n"
    );
    let with_predicate =
        format!("{only}predicate Positive using v (x: Digit): Boolean {{ x >= 0 }}\n");
    for (unit, synthesis, code, cause, pointer) in [
        (
            two.clone(),
            json!({}),
            "invalid_runtime_input",
            "missing-member",
            "/request/synthesis/declaration",
        ),
        (
            only.clone(),
            json!({"declaration": "Nope"}),
            "missing_declaration",
            "missing-name",
            "/request/synthesis/declaration",
        ),
        (
            with_predicate,
            json!({"declaration": "Positive"}),
            "missing_declaration",
            "missing-name",
            "/request/synthesis/declaration",
        ),
        (
            only.clone(),
            json!({"declaration": "Pick", "initial": "snapshot.json"}),
            "invalid_runtime_input",
            "unknown-member",
            "/request/synthesis/initial",
        ),
        (
            only.clone(),
            json!({"declaration": "Pick", "limits": {"work": 1}}),
            "invalid_runtime_input",
            "unknown-member",
            "/request/synthesis/limits/work",
        ),
        (
            only.clone(),
            json!({"declaration": "Pick", "limits": {"max_work": 0}}),
            "invalid_runtime_input",
            "invalid-value",
            "/request/synthesis/limits/max_work",
        ),
        (
            only.clone(),
            json!({"declaration": "Pick", "engines": []}),
            "invalid_runtime_input",
            "unknown-member",
            "/request/synthesis/engines",
        ),
    ] {
        let (exit, value) = run(&unit, synthesis);
        assert_eq!(exit, 20, "{value}");
        assert_eq!(value["code"], code, "{value}");
        assert_eq!(value["cause"], cause, "{value}");
        assert_eq!(value["details"]["path"], pointer, "{value}");
        assert!(value["synthesis"].get("status").is_none(), "{value}");
    }
}

/// FR-1037: the `synthesis` member is the request's only run member, so a
/// `tl_lowering` beside it refuses instead of being ignored.
#[test]
#[trace("TC-3017", "FR-1037-AC-2")]
fn a_synthesis_beside_another_run_member_refuses() {
    let (exit, value) = run_with(
        &pick("Pick * Pick = 49", ""),
        json!({"synthesis": {"declaration": "Pick"}, "tl_lowering": {}}),
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid-request", "{value}");
    names(&value, "mutually exclusive");
    assert!(value["synthesis"].get("status").is_none(), "{value}");
    let (exit, value) = run_with(
        &pick("Pick * Pick = 49", ""),
        json!({"synthesis": {"declaration": "Pick"}, "verification_plan": {"plan": "Release"}}),
    );
    assert_eq!(exit, 20, "{value}");
    assert_eq!(value["code"], "invalid-request", "{value}");
    names(&value, "mutually exclusive");
    assert!(value["synthesis"].get("status").is_none(), "{value}");
}

/// The plan candidate projects generated records rooted at the authored
/// synthesis span, and the strict reader admits every declaration's region.
#[test]
#[trace("FR-1036-AC-2", "FSpec-FR-800-AC-1")]
fn a_source_plan_candidate_emits_its_record_types_and_source_regions() {
    let directory = tempfile::tempdir().unwrap();
    spine::write(directory.path(), Case::Healthy).unwrap();
    let document = std::fs::read(directory.path().join("spine-model.semantic-ir.json")).unwrap();
    let packages = fsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the spine model admits");
    let unit = format!(
        "{}type Lengths = Int[0, 3];\n\
         synthesis Reach using v grammar Config::ConfigVersion domain Lengths \
         minimize Reach.length satisfies {{ Reach.length >= 1 and Reach.final.versionNumber >= 0 }};\n",
        MODEL_HEADER.replace(
            "filament.model.complete/v1",
            "filament.temporal.infinite-trace/v1"
        ),
    );
    let emitted = emitted(
        SourceIdentity::new("test", "synthesis-request"),
        "unit.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .expect("the source plan emits with its generated type declarations");
    let records = emitted.emission().selected_source_record_rows();
    assert!(
        records.iter().any(|row| row
            .members
            .iter()
            .map(|field| field.admitted_name.as_ref())
            .eq(["length", "final", "initial", "steps"])),
        "the plan record has a producer identity row"
    );
    assert!(
        records
            .iter()
            .all(|row| row.source_authority.as_ref() == "test"
                && row.source_identity.as_ref() == "synthesis-request"),
        "generated records retain the authored source identity"
    );
    let operation = emitted
        .emission()
        .selected_enum_declaration_rows()
        .iter()
        .find(|row| {
            row.qualified_declaration
                .iter()
                .map(|segment| segment.as_ref())
                .eq(["Reach", "Config", "Op"])
        })
        .expect("the plan operation enum has its producer identity row");
    assert_eq!(
        (
            operation.source_authority.as_ref(),
            operation.source_identity.as_ref()
        ),
        ("test", "synthesis-request")
    );
    let bytes = emitted.package().bytes();
    fsl_package::admit_checked_package(
        bytes,
        &packages,
        fsl_package::V2ReadLimits::bounded(fsl_replay::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT),
    )
    .expect("all generated plan declarations and occurrences admit");
    // Operand slots belong to their holder's row; resolve them before
    // comparing the claim's reference to the parameter's own node id.
    let mut wire: Value = serde_json::from_slice(bytes).expect("a JSON package");
    fsl_semantics::check::wire_id::resolve_operand_slots(&mut wire)
        .expect("every operand slot names its holder's row entry");
    let claim = synthesis_nodes(&wire)[0];
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("graph nodes");
    let candidates = nodes
        .iter()
        .filter(|node| node["node_id"] == binding(claim, "candidate")["value"]["target"])
        .collect::<Vec<_>>();
    let [candidate] = candidates.as_slice() else {
        panic!("the plan candidate resolves to exactly one parameter node: {candidates:?}");
    };
    assert_eq!(candidate["node_tag"], "value", "{candidate}");
    assert_eq!(candidate["semantic_form"], "parameter", "{candidate}");
    let plan_types = nodes
        .iter()
        .filter(|node| node["node_id"] == candidate["semantic_type"])
        .collect::<Vec<_>>();
    let [plan_type] = plan_types.as_slice() else {
        panic!("the parameter is typed by exactly one emitted plan record: {plan_types:?}");
    };
    assert_eq!(plan_type["semantic_form"], "record", "{plan_type}");
    assert_eq!(
        records
            .iter()
            .filter(
                |row| serde_json::to_value(&row.structural_node).expect("a node id serializes")
                    == plan_type["node_id"]
            )
            .count(),
        1,
        "the exact plan record has one producer identity row"
    );
    assert_eq!(
        binding_names(plan_type),
        ["length", "final", "initial", "steps"]
    );
}

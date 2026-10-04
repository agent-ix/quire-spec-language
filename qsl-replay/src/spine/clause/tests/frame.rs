// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-514 (FR-115): `run_clause`'s `Frame` selection over FR-108's
//! ConfigVersion unit, package and invocation fixtures. `attemptUpdate`'s
//! frame modifies exactly `versionNumber` and creates and deletes nothing.

use super::*;
use crate::spine::{CallOutcome, CallRefusal, OperationName};
use qsl_semantics::model::observation::{AdmissionFailure, AdmissionRecord, FrameChange};
use quire_exact::Identifier;

/// `text` as an identifier.
pub(super) fn identifier(text: &str) -> Identifier {
    Identifier::new(text).expect("an identifier")
}

/// `Config::ConfigVersion::<name>`.
fn operation(name: &str) -> OperationName {
    OperationName {
        model: identifier("Config"),
        object: identifier("ConfigVersion"),
        operation: identifier(name),
    }
}

/// One `config_history` object: `key` at `version`, naming `parent` when
/// `Some`.
pub(super) fn object(key: &str, version: i64, parent: Option<&str>) -> serde_json::Value {
    let parent = match parent {
        Some(parent) => json!({"present": {"reference": {
            "population": config_version_population_identity(), "key": parent,
        }}}),
        None => json!({"absent": {}}),
    };
    json!({
        "key": key, "type": config_version_type(),
        "fields": {"versionNumber": {"integer": version.to_string()}, "parent": parent},
    })
}

/// A pre or post snapshot holding `objects` in `config_history`.
pub(super) fn snapshot(
    label: &DocumentRef,
    observation: &str,
    objects: &[serde_json::Value],
) -> Vec<u8> {
    json!({
        "format": "quire.state.snapshot/v1",
        "identity": document_identity_json(label),
        "observation": observation,
        "model": {
            "identity": CONFIG_VERSION_PACKAGE_IDENTITY, "version": "1.0.0",
            "digest": format!("sha256-jcs:{}", config_version_model_digest_hex()),
        },
        "populations": [{
            "population": config_version_population_identity(), "complete": true,
            "objects": objects,
        }],
    })
    .to_string()
    .into_bytes()
}

/// One invocation of `attemptUpdate` with self `child`, and its request.
pub(super) struct FrameInput {
    invocation: DocumentRef,
    pre: DocumentRef,
    post: DocumentRef,
    request: ClauseRunRequest,
}

/// Builds `pre_objects`/`post_objects`' snapshots and an invocation of
/// `attemptUpdate` over them, `edit` applied to the invocation document
/// before its digest is taken, into a `Frame` request selecting
/// `operation_name` over `request_builder`'s unit.
pub(super) fn frame_input(
    request_builder: fn(ClauseRunSelection) -> ClauseRunRequest,
    operation_name: &str,
    pre_objects: &[serde_json::Value],
    post_objects: &[serde_json::Value],
    edit: impl FnOnce(&mut serde_json::Value),
) -> FrameInput {
    let model_digest_hex = config_version_model_digest_hex();
    let (pre, pre_bytes) =
        document_ref_and_bytes("frame-pre", |label| snapshot(label, "pre", pre_objects));
    let (post, post_bytes) =
        document_ref_and_bytes("frame-post", |label| snapshot(label, "post", post_objects));
    let (invocation, invocation_bytes) = document_ref_and_bytes("frame-invocation", |label| {
        let bytes = config_version_invocation_bytes(label, &model_digest_hex, &pre, &post, "child");
        let mut document: serde_json::Value =
            serde_json::from_slice(&bytes).expect("the fixture is JSON");
        edit(&mut document);
        document.to_string().into_bytes()
    });
    let mut request = request_builder(ClauseRunSelection::Frame {
        operation: operation(operation_name),
        invocation: invocation.clone(),
    });
    request.snapshots.insert(pre.digest, pre_bytes);
    request.snapshots.insert(post.digest, post_bytes);
    request
        .invocations
        .insert(invocation.digest, invocation_bytes);
    FrameInput {
        invocation,
        pre,
        post,
        request,
    }
}

/// changed-version: `child.versionNumber` moves from 2 to 3, nothing else
/// changes.
fn changed_version() -> FrameInput {
    frame_input(
        config_version_request,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 3, Some("root"))],
        |_| {},
    )
}

/// The identity of the one `state`/`frame` node the compiled unit emits.
fn emitted_frame_identity() -> NodeKey {
    let (unit, packages) = config_version_unit_and_packages();
    let compiled = compose(
        source(),
        "frame.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .expect("the ConfigVersion unit compiles");
    let wire: serde_json::Value =
        serde_json::from_slice(compiled.emitted.bytes()).expect("the emitted package is JSON");
    let frames: Vec<&serde_json::Value> = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("the package lists its nodes")
        .iter()
        .filter(|node| node["semantic_form"] == "frame")
        .collect();
    let [frame] = frames[..] else {
        panic!("one frame node: {frames:#?}");
    };
    let digest = frame["node_id"]["digest"]
        .as_str()
        .expect("a node id digest");
    let identity = compiled
        .package
        .graph()
        .semantic_graph()
        .nodes()
        .map(|node| node.key())
        .find(|key| {
            qsl_foundation::digest::WireNodeId::from_digest(*key.as_bytes()).to_string() == digest
        })
        .expect("the emitted frame node is a node of the checked graph");
    identity
}

pub(super) fn run(input: FrameInput) -> super::super::ClauseRunReport {
    run_clause(input.request).expect("a well-formed request always reports")
}

/// The admission record of a stage-`admit` refusal or incomplete.
fn admit_record(report: &super::super::ClauseRunReport) -> &AdmissionRecord {
    match &report.disposition {
        ClauseDisposition::Admit(
            AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
        ) => record,
        other => panic!("expected an admit disposition, got {other:?}"),
    }
}

/// TC-514 step 1 (FR-115-AC-1): changed-version, whose post changes only
/// `child.versionNumber`, reports `evaluate`, `success`, `truth: true`,
/// exit 0, with the frame node's identity and the three documents in its
/// provenance.
#[trace("TC-514", "FR-115-AC-1")]
#[test]
fn a_frame_respecting_invocation_succeeds_with_its_provenance() {
    let input = changed_version();
    let expected_documents = vec![
        input.invocation.clone(),
        input.pre.clone(),
        input.post.clone(),
    ];
    let report = run(input);
    assert!(
        matches!(
            report.disposition,
            ClauseDisposition::Evaluate(CallOutcome::Completed(super::super::CallValue::Boolean(
                true
            )))
        ),
        "{:?}",
        report.disposition
    );
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Success
    );
    assert_eq!(report.disposition.truth(), Some(true));
    assert_eq!(report.disposition.category().exit_code(), 0);
    assert_eq!(report.provenance.frame, Some(emitted_frame_identity()));
    assert_eq!(report.provenance.documents, expected_documents);
}

/// TC-514 step 2 (FR-115-AC-2): forbidden-parent-change (post sets
/// `child.parent` absent) is a violation with a witness naming `child`,
/// `parent` and the frame's `modifies`, cause
/// `frame_violation`/`unauthorized-change` -- never a refusal.
#[trace("TC-514", "FR-115-AC-2")]
#[test]
fn a_change_outside_the_frame_is_a_violation_with_its_witness() {
    let input = frame_input(
        config_version_request,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 2, None)],
        |_| {},
    );
    let (invocation, pre, post) = (
        input.invocation.clone(),
        input.pre.clone(),
        input.post.clone(),
    );
    let report = run(input);
    let ClauseDisposition::FrameViolation(witness) = &report.disposition else {
        panic!("expected FrameViolation, got {:?}", report.disposition);
    };
    assert_eq!(
        witness.code,
        qsl_foundation::diagnostic::Code::FrameViolation
    );
    assert_eq!(witness.code.as_str(), "frame_violation");
    assert_eq!(witness.cause, "unauthorized-change");
    assert_eq!(
        (&witness.invocation, &witness.pre, &witness.post),
        (&invocation, &pre, &post)
    );
    assert_eq!(witness.population, config_version_population_identity());
    let FrameChange::FieldWrite {
        object,
        field,
        modifies,
    } = &witness.change
    else {
        panic!("expected a field write, got {:?}", witness.change);
    };
    assert_eq!((object.as_str(), field.as_str()), ("child", "parent"));
    let modifies: Vec<&str> = modifies.iter().map(|key| key.node.as_str()).collect();
    assert_eq!(
        modifies,
        [format!("{}/versionNumber", config_version_type())]
    );
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Violation
    );
    assert_eq!(report.disposition.truth(), Some(false));
    assert_eq!(report.disposition.category().exit_code(), 10);
}

/// TC-514 step 2 (FR-115-AC-2): a post that adds `c2` to `config_history`
/// is a violation whose witness names the creation of `c2` against the
/// frame's empty `creates`.
#[trace("TC-514", "FR-115-AC-2")]
#[test]
fn a_creation_outside_the_frame_is_a_violation_naming_it() {
    let report = run(frame_input(
        config_version_request,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[
            object("root", 1, None),
            object("child", 2, Some("root")),
            object("c2", 3, Some("root")),
        ],
        |_| {},
    ));
    let ClauseDisposition::FrameViolation(witness) = &report.disposition else {
        panic!("expected FrameViolation, got {:?}", report.disposition);
    };
    let FrameChange::Created {
        object,
        type_name,
        creates,
    } = &witness.change
    else {
        panic!("expected a creation, got {:?}", witness.change);
    };
    assert_eq!(object, "c2");
    assert_eq!(type_name.node, config_version_type());
    assert!(creates.is_empty(), "{creates:?}");
    assert_eq!(report.disposition.category().exit_code(), 10);
}

/// TC-514 step 2 (FR-115 Behavior, "the created or deleted object and its
/// type"): a post that drops `c2` from `config_history` is a violation
/// whose witness names the deletion of `c2` against the frame's empty
/// `deletes`.
#[trace("TC-514", "FR-115-AC-2")]
#[test]
fn a_deletion_outside_the_frame_is_a_violation_naming_it() {
    let report = run(frame_input(
        config_version_request,
        "attemptUpdate",
        &[
            object("root", 1, None),
            object("child", 2, Some("root")),
            object("c2", 3, Some("root")),
        ],
        &[object("root", 1, None), object("child", 2, Some("root"))],
        |_| {},
    ));
    let ClauseDisposition::FrameViolation(witness) = &report.disposition else {
        panic!("expected FrameViolation, got {:?}", report.disposition);
    };
    let FrameChange::Deleted {
        object,
        type_name,
        deletes,
    } = &witness.change
    else {
        panic!("expected a deletion, got {:?}", witness.change);
    };
    assert_eq!(object, "c2");
    assert_eq!(type_name.node, config_version_type());
    assert!(deletes.is_empty(), "{deletes:?}");
    assert_eq!(report.disposition.category().exit_code(), 10);
}

/// TC-514 step 2 (FR-115 Behavior, checks 1 and 3 to 10 with `self`
/// required in the pre snapshot only): a post that deletes `child`, the
/// invocation's `self`, admits and is a violation naming the deletion of
/// `child` against the frame's empty `deletes`, never a `wrong-role-mapping`
/// refusal over `self`'s absence from post.
#[trace("TC-514", "FR-115-AC-2")]
#[test]
fn a_deletion_of_self_outside_the_frame_is_a_violation_naming_it() {
    let report = run(frame_input(
        config_version_request,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None)],
        |_| {},
    ));
    let ClauseDisposition::FrameViolation(witness) = &report.disposition else {
        panic!("expected FrameViolation, got {:?}", report.disposition);
    };
    let FrameChange::Deleted {
        object, deletes, ..
    } = &witness.change
    else {
        panic!("expected a deletion, got {:?}", witness.change);
    };
    assert_eq!(object, "child");
    assert!(deletes.is_empty(), "{deletes:?}");
}

/// TC-514 step 2 (FR-115-AC-2): changed-version declaring `created:
/// [child]` disagrees with the computed empty delta: `evaluate`, `refusal`,
/// `population_delta_mismatch`/`delta-disagreement`, exit 20.
#[trace("TC-514", "FR-115-AC-2")]
#[test]
fn a_disagreeing_declared_delta_refuses_at_evaluate() {
    let report = run(frame_input(
        config_version_request,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 3, Some("root"))],
        |document| {
            document["created"] = json!([{
                "population": config_version_population_identity(), "key": "child",
            }]);
        },
    ));
    let ClauseDisposition::Evaluate(CallOutcome::Refused(refusal)) = &report.disposition else {
        panic!(
            "expected Evaluate(Refused(_)), got {:?}",
            report.disposition
        );
    };
    let code = match refusal {
        CallRefusal::Record { code, .. } | CallRefusal::Family { code, .. } => code,
    };
    assert_eq!(
        (code.code(), code.cause()),
        ("population_delta_mismatch", "delta-disagreement")
    );
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Refusal
    );
    assert_eq!(report.disposition.category().exit_code(), 20);
}

/// TC-514 step 3 (FR-115-AC-3): an expected `package_id` from another unit
/// refuses `compile`, `stale_dependency`, naming both.
#[trace("TC-514", "FR-115-AC-3")]
#[test]
fn a_stale_package_refuses_before_evaluation() {
    let other = compiled().emitted.package_id();
    let mut input = changed_version();
    input.request.expected_package_id = Some(other);
    let report = run(input);
    let ClauseDisposition::StalePackage { expected, actual } = &report.disposition else {
        panic!("expected StalePackage, got {:?}", report.disposition);
    };
    assert_eq!(*expected, other);
    assert_ne!(*actual, other);
    assert_eq!(report.disposition.stage(), ClauseRunStage::Compile);
}

/// TC-514 step 3 (FR-115-AC-3): stale invocation bytes, labels, model and
/// operation each refuse at `admit`, and none reaches `evaluate`.
#[trace("TC-514", "FR-115-AC-3")]
#[test]
fn stale_or_mismatched_documents_refuse_at_admit() {
    // Invocation bytes edited after the digest was taken.
    let mut edited = changed_version();
    let bytes = edited
        .request
        .invocations
        .get_mut(&edited.invocation.digest)
        .expect("the invocation is provided");
    *bytes = String::from_utf8(bytes.clone())
        .expect("UTF-8")
        .replace("\"boolean\":true", "\"boolean\":false")
        .into_bytes();
    // Another `model` digest in the invocation document.
    let other_model = frame_input(
        config_version_request,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 3, Some("root"))],
        |document| {
            document["model"]["digest"] = json!(format!("sha256-jcs:{}", "0".repeat(64)));
        },
    );
    // An invocation of `probe`, not `attemptUpdate`.
    let probe = frame_input(
        config_version_request,
        "attemptUpdate",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 3, Some("root"))],
        |document| document["operation"] = json!("probe"),
    );

    for (input, code, cause) in [
        (edited, "stale_dependency", "byte-digest-mismatch"),
        (
            other_model,
            "invalid_model_binding",
            "wrong-model-selection",
        ),
        (probe, "wrong_snapshot", "wrong-invocation"),
    ] {
        let report = run(input);
        assert_eq!(report.disposition.stage(), ClauseRunStage::Admit, "{cause}");
        let record = admit_record(&report);
        assert_eq!((record.code.as_str(), record.cause), (code, cause));
    }
}

/// TC-514 step 4 (FR-115-AC-4): `Config::ConfigVersion::missing`, and
/// `probe`, a package operation no clause or attempt names, each refuse
/// `select`, `missing_declaration`/`missing-name`.
#[trace("TC-514", "FR-115-AC-4")]
#[test]
fn an_unknown_or_unnamed_operation_refuses_at_select() {
    fn probe_request(selection: ClauseRunSelection) -> ClauseRunRequest {
        let (unit, packages) =
            config_version_unit_and_packages_for(config_version_step3_domain_document());
        config_version_request_for(unit, packages, "frame-probe.native", selection)
    }
    let missing = changed_version();
    let mut missing_request = missing.request;
    missing_request.selection = ClauseRunSelection::Frame {
        operation: operation("missing"),
        invocation: missing.invocation,
    };
    let unnamed = frame_input(
        probe_request,
        "probe",
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 2, Some("root"))],
        |_| {},
    );
    for (request, name) in [
        (missing_request, "Config::ConfigVersion::missing"),
        (unnamed.request, "Config::ConfigVersion::probe"),
    ] {
        let report = run_clause(request).expect("reports");
        let ClauseDisposition::MissingName { name: reported } = &report.disposition else {
            panic!("expected MissingName, got {:?}", report.disposition);
        };
        assert_eq!(reported, name);
        assert_eq!(report.disposition.stage(), ClauseRunStage::Select);
        assert_eq!(
            report.disposition.category().exit_code(),
            qsl_foundation::diagnostic::Code::MissingDeclaration
                .category()
                .exit_code()
        );
        assert_eq!(report.provenance.frame, None);
    }
}

/// TC-514 step 4 (FR-115-AC-4): changed-version with its pre snapshot
/// removed from the provision reports `admit`, `incomplete`,
/// `unavailable_observation`, never a violation.
#[trace("TC-514", "FR-115-AC-4")]
#[test]
fn an_absent_pre_snapshot_is_incomplete_not_a_violation() {
    let mut input = changed_version();
    input.request.snapshots.remove(&input.pre.digest);
    let report = run(input);
    assert!(
        matches!(
            report.disposition,
            ClauseDisposition::Admit(AdmissionFailure::Incomplete(_))
        ),
        "{:?}",
        report.disposition
    );
    assert_eq!(
        admit_record(&report).code.as_str(),
        "unavailable_observation"
    );
    assert_eq!(
        report.disposition.category(),
        qsl_foundation::diagnostic::Category::Incomplete
    );
}

/// TC-514 step 5 (FR-115-AC-5): running one `Frame` request twice gives
/// equal reports, including usage.
#[trace("TC-514", "FR-115-AC-5")]
#[test]
fn a_frame_run_is_deterministic() {
    let first = run(changed_version());
    let second = run(changed_version());
    assert_eq!(format!("{first:?}"), format!("{second:?}"));
    assert_eq!(
        first.usage.admission_consumed,
        second.usage.admission_consumed
    );
    assert!(first.usage.admission_consumed.document_bytes > 0);
}

/// TC-514 step 4 (FR-115-AC-4): `Frame` on `Nope::ConfigVersion::attemptUpdate`,
/// whose alias no `model` declaration binds, and on
/// `Config::Missing::attemptUpdate`, whose type the selected package does
/// not declare, each refuse `select`, `missing_declaration`/`missing-name`.
#[trace("TC-514", "FR-115-AC-4")]
#[test]
fn an_unresolved_model_alias_or_object_type_refuses_at_select() {
    for (model, object, name) in [
        (
            "Nope",
            "ConfigVersion",
            "Nope::ConfigVersion::attemptUpdate",
        ),
        ("Config", "Missing", "Config::Missing::attemptUpdate"),
    ] {
        let mut input = changed_version();
        input.request.selection = ClauseRunSelection::Frame {
            operation: OperationName {
                model: identifier(model),
                object: identifier(object),
                operation: identifier("attemptUpdate"),
            },
            invocation: input.invocation,
        };
        let report = run_clause(input.request).expect("reports");
        let ClauseDisposition::MissingName { name: reported } = &report.disposition else {
            panic!("expected MissingName, got {:?}", report.disposition);
        };
        assert_eq!(reported, name);
        assert_eq!(report.disposition.stage(), ClauseRunStage::Select);
        assert_eq!(
            report.disposition.category().exit_code(),
            qsl_foundation::diagnostic::Code::MissingDeclaration
                .category()
                .exit_code()
        );
    }
}

/// TC-514 step 6 (FR-115-AC-6, ADR-017 PF-3): over a package where `Sub`
/// specializes `ConfigVersion` and declares no operation, the resolver
/// takes `Config::Sub::attemptUpdate` to `Sub`'s declaration key in the
/// `Config` package, and that selection picks `ConfigVersion`'s frame, the
/// operation's declaring type, with `Sub` as the context.
#[trace("TC-514", "FR-115-AC-6")]
#[test]
fn an_inherited_operation_selects_its_declaring_frame_through_the_resolver() {
    let (unit, packages) =
        config_version_unit_and_packages_for(config_version_domain_document_with_sub());
    let compiled = compile_config_version_unit(&unit, &packages);
    let graph = compiled.package.graph();
    let resolve = |object: &str| {
        graph
            .resolve_operation(
                &identifier("Config"),
                &identifier(object),
                &identifier("attemptUpdate"),
            )
            .unwrap_or_else(|| panic!("Config::{object} resolves"))
    };
    let via_sub = resolve("Sub");
    let via_config_version = resolve("ConfigVersion");

    assert_eq!(
        via_sub.object(),
        &qsl_semantics::model::key::DeclarationKey {
            package: CONFIG_VERSION_PACKAGE_IDENTITY.to_owned(),
            node: format!("ix://{CONFIG_VERSION_PACKAGE_IDENTITY}/Sub"),
        }
    );
    assert_eq!(via_sub.operation().as_str(), "attemptUpdate");

    let (sub, sub_frame) = graph
        .operation_frame(&via_sub)
        .expect("Sub inherits attemptUpdate's frame");
    let (config_version, declaring_frame) = graph
        .operation_frame(&via_config_version)
        .expect("ConfigVersion declares attemptUpdate");
    assert_ne!(sub, config_version, "the context is Sub itself");
    assert_eq!(sub_frame, declaring_frame);
    assert_eq!(sub_frame.operation().declaring, config_version);
}

/// TC-514 step 6 (FR-115-AC-6): a unit selecting two domain packages that
/// both declare `ConfigVersion`, as `Config` and `Copy`. Each alias
/// resolves `ConfigVersion` to its own package's declaration key and
/// selects that package's `attemptUpdate` frame.
#[trace("TC-514", "FR-115-AC-6")]
#[test]
fn each_model_alias_resolves_its_own_packages_object_type() {
    const COPY_PACKAGE_IDENTITY: &str = "test/config-copy";
    let original = config_version_domain_document();
    let copy = String::from_utf8(original.clone())
        .expect("the document is UTF-8")
        .replace(CONFIG_VERSION_PACKAGE_IDENTITY, COPY_PACKAGE_IDENTITY)
        .into_bytes();
    let (unit, mut packages) = config_version_unit_and_packages_for(original);
    let copy_packages = qsl_semantics::model::intake::package_input([copy.as_slice()]);
    let [(copy_digest, _)] = copy_packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let (header, declarations) = unit
        .split_once("invariant ParentOrder")
        .expect("the unit declares ParentOrder after its selections");
    let unit = format!(
        "{header}model Copy = {COPY_PACKAGE_IDENTITY:?} version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         invariant ParentOrder{declarations}\
         post CopyUnchanged using v on Copy::ConfigVersion::attemptUpdate {{ \
         self.versionNumber = pre(self.versionNumber) }}\n",
        hex(copy_digest)
    );
    packages.extend(copy_packages);
    let compiled = compile_config_version_unit(&unit, &packages);
    let graph = compiled.package.graph();

    let mut frames = Vec::new();
    for (alias, package) in [
        ("Config", CONFIG_VERSION_PACKAGE_IDENTITY),
        ("Copy", COPY_PACKAGE_IDENTITY),
    ] {
        let selection = graph
            .resolve_operation(
                &identifier(alias),
                &identifier("ConfigVersion"),
                &identifier("attemptUpdate"),
            )
            .unwrap_or_else(|| panic!("{alias}::ConfigVersion resolves"));
        assert_eq!(
            selection.object(),
            &qsl_semantics::model::key::DeclarationKey {
                package: package.to_owned(),
                node: format!("ix://{package}/ConfigVersion"),
            },
            "{alias} resolves in its own package"
        );
        let (_, frame) = graph
            .operation_frame(&selection)
            .unwrap_or_else(|| panic!("{alias}'s attemptUpdate has a frame"));
        frames.push(frame.frame());
    }
    assert_ne!(frames[0], frames[1], "each package's own frame");
}

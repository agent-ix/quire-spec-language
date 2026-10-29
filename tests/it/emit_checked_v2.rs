// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `emit_checked_v2` runnable producer: it emits the bytes the spine's
//! producer emits for a source, those bytes read back through Contract IR's
//! strict checked-package v2 reader (the reader QSL's I2 read delegates to),
//! and a source the producer refuses exits nonzero with a diagnostic. Every
//! expectation is derived from the producer or the reader; no digest or byte
//! is frozen.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::DependencyInput;
use quire_contract_model::{
    ApplicationOperator, CheckedNodeId, CheckedOccurrenceRole, CheckedPackageEvidence,
    CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult, CheckedSemanticNodeV2,
    CheckedTerm,
};
use quire_spec_language::emit_checked_v2::{emit, run, Error, Request, MODEL_DIGEST_TOKEN};
use serde_json::{json, Value};

fn args(items: &[&Path]) -> Vec<OsString> {
    items
        .iter()
        .map(|item| item.as_os_str().to_owned())
        .collect()
}

fn strings(items: &[&str]) -> Vec<OsString> {
    items.iter().map(OsString::from).collect()
}

/// `tests/fixtures/spine-model.native` with its model digest filled in
/// (`crate::support::spine_model::unit`), written to `directory` for the
/// runnable output to read.
fn spine_model_source(directory: &Path) -> PathBuf {
    let path = directory.join("spine-model.native");
    std::fs::write(&path, crate::support::spine_model::unit()).unwrap();
    path
}

/// IR's strict read of `bytes`, supplied `documents` as domain evidence.
fn read_back(bytes: &[u8], documents: &[&[u8]]) -> CheckedPackageV2 {
    let wire: Value = serde_json::from_slice(bytes).expect("the output is JSON");
    assert_eq!(wire["contract_version"], "quire.checked-package/v2");
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    for (digest, document) in qsl_semantics::model::intake::package_input(documents.iter().copied())
    {
        evidence
            .insert_domain_package_document(qsl_semantics::model::key::hex(&digest), document)
            .expect("a fresh evidence key");
    }
    match CheckedPackageV2::read(bytes, CheckedPackageReadLimits::bounded(), &evidence) {
        CheckedPackageV2ReadResult::Admitted(package) => *package,
        other => panic!("strict IR admission failed: {other:?}"),
    }
}

/// The producer's own bytes for `source` over `documents`, by the spine's
/// four operations rather than by the runnable output.
///
/// The bytes carry the source's labels and display path, so the oracle is
/// given the labels the runnable output derives from `path`.
fn producer_bytes(path: &Path, source: &str, documents: &[&[u8]]) -> Vec<u8> {
    crate::support::front_end::emitted(
        SourceIdentity::new("local", format!("source:{}", path.display()), "file", "1"),
        &path.display().to_string(),
        source.as_bytes(),
        &qsl_semantics::model::intake::package_input(documents.iter().copied()),
        &DependencyInput::default(),
    )
    .expect("the producer emits")
    .package()
    .bytes()
    .to_vec()
}

/// The checked-package v2 bytes of the spine fixtures, from the runnable
/// output, are the producer's bytes and read back admitted; `--out` writes the
/// same bytes the default stdout carries.
#[test]
fn the_output_is_the_producers_bytes_and_reads_back() {
    let source = crate::support::spine_model::unit();
    let document = crate::support::spine_model::DOCUMENT;
    let directory = tempfile::tempdir().unwrap();
    let out = directory.path().join("package.json");
    let model = Path::new("tests/fixtures/spine-model.semantic-ir.json");
    let program = spine_model_source(directory.path());
    let program = program.as_path();

    let mut stdout = Vec::new();
    let mut command = strings(&["--model"]);
    command.extend(args(&[model, program]));
    run(&command, &mut stdout).expect("the fixture emits");
    assert!(!stdout.is_empty());
    assert_eq!(stdout, producer_bytes(program, &source, &[document]));
    read_back(&stdout, &[document]);

    let mut elsewhere = Vec::new();
    let mut command = strings(&["--out"]);
    command.extend(args(&[&out]));
    command.extend(strings(&["--model"]));
    command.extend(args(&[model, program]));
    run(&command, &mut elsewhere).expect("--out emits");
    assert!(elsewhere.is_empty(), "--out leaves stdout empty");
    assert_eq!(std::fs::read(&out).unwrap(), stdout);
}

/// The digest `emit_checked_v2` derives for a domain package document.
fn model_digest(document: &[u8]) -> String {
    let packages = qsl_semantics::model::intake::package_input([document]);
    qsl_semantics::model::key::hex(packages.keys().next().unwrap())
}

/// The switch template with its digest filled from the model file, as the
/// producer renders it; the test derives it, nothing is frozen.
fn rendered_switch_source() -> String {
    let document = std::fs::read("examples/switch-transition/switch.semantic-ir.json").unwrap();
    std::fs::read_to_string("examples/switch-transition/switch.native.in")
        .unwrap()
        .replace(MODEL_DIGEST_TOKEN, &model_digest(&document))
}

/// No file under `examples/switch-transition/` carries a digest literal: FR-056
/// selects models by digest, so a committed one would silently stale on a
/// model edit.
#[test]
fn the_switch_example_directory_carries_no_digest_literal() {
    for entry in std::fs::read_dir("examples/switch-transition").unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        for (at, _) in text.match_indices("sha256-jcs:") {
            let tail = &text[at + "sha256-jcs:".len()..];
            let run = tail.bytes().take_while(u8::is_ascii_hexdigit).count();
            assert!(run < 64, "{} carries a digest literal", path.display());
        }
    }
}

/// The QSL-owned Boolean Switch pair (`examples/switch-transition/`): the
/// derived Boolean model-field transition (FR-105-AC-13) is emitted from the
/// checked-in source and domain model, and the package reads back admitted
/// through IR's strict reader. The producer's `state_transition` node is found
/// by shape and not by a frozen id, and the source's `digest` is checked
/// against the model file rather than frozen in this test.
#[test]
fn the_switch_example_pair_emits_a_transition_and_is_admitted() {
    let model = Path::new("examples/switch-transition/switch.semantic-ir.json");
    let program = Path::new("examples/switch-transition/switch.native.in");
    let document = std::fs::read(model).unwrap();
    let template = std::fs::read_to_string(program).unwrap();
    assert!(
        template.contains(MODEL_DIGEST_TOKEN),
        "the committed source is a template, not a digest-carrying source"
    );
    let source = rendered_switch_source();
    assert!(
        source.contains(&format!("sha256-jcs:{}", model_digest(&document))),
        "the rendered source selects the model file by its derived digest"
    );
    let directory = tempfile::tempdir().unwrap();
    let rendered = directory.path().join("switch.native");
    let mut with_source_out = strings(&["--source-out"]);
    with_source_out.extend(args(&[&rendered, Path::new("--model"), model, program]));
    emit(&Request::parse(&with_source_out).unwrap()).expect("renders and emits");
    assert_eq!(std::fs::read_to_string(&rendered).unwrap(), source);

    let mut command = strings(&["--model"]);
    command.extend(args(&[model, program]));
    let bytes = emit(&Request::parse(&command).unwrap()).expect("the transition form emits");
    assert_eq!(bytes, producer_bytes(program, &source, &[&document]));
    read_back(&bytes, &[&document]);
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    let transitions = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["node_tag"] == "state" && node["semantic_form"] == "transition")
        .count();
    assert_eq!(transitions, 1, "one derived transition for (set, on)");
}

/// The admitted graph node `id` names.
fn node<'a>(package: &'a CheckedPackageV2, id: &CheckedNodeId) -> &'a CheckedSemanticNodeV2 {
    package
        .graph()
        .nodes
        .iter()
        .find(|node| &node.node_id == id)
        .expect("a referenced node is in the graph")
}

/// The target of a `reference` term.
fn target(term: &CheckedTerm) -> &CheckedNodeId {
    match term {
        CheckedTerm::Reference { target } => target,
        other => panic!("expected a reference, found {other:?}"),
    }
}

/// A text literal member `name` of a parameter's aggregate body.
fn parameter_name(node: &CheckedSemanticNodeV2) -> String {
    let CheckedTerm::Aggregate { members } = &node.body else {
        panic!("a parameter body is an aggregate");
    };
    members
        .iter()
        .find_map(|member| match member {
            CheckedTerm::Binding { name, value } if name.as_ref() == "name" => match &**value {
                CheckedTerm::Literal { value, .. } => value.as_str().map(str::to_owned),
                _ => None,
            },
            _ => None,
        })
        .expect("a parameter names itself")
}

/// IR FR-067-AC-7, QSL-authored half, composed from the QSL side: the emitted
/// Switch package is read through IR's public strict reader and its typed
/// `state_transition` application is asserted, never the wire JSON. The
/// expectations come from the model file (the `Switch` type, its Boolean `on`
/// field and Boolean `set(flag)` operation) and from the rendered source (the
/// `post` clause body `self.on` and its byte region); no node id, digest or
/// count is frozen. QSL reads IR only through `quire-contract-model`'s public
/// reader, so there is no IR to QSL dependency.
#[trace("TC-1271", "FR-105-AC-13")]
#[test]
fn the_switch_transition_reads_back_typed_subject_input_post_claim_and_source() {
    let model = Path::new(SWITCH_MODEL);
    let program = Path::new(SWITCH_TEMPLATE);
    let document = std::fs::read(model).unwrap();
    let source = rendered_switch_source();
    let mut command = strings(&["--model"]);
    command.extend(args(&[model, program]));
    let bytes = emit(&Request::parse(&command).unwrap()).expect("the transition form emits");
    let package = read_back(&bytes, &[&document]);

    // The model file's own declarations, so nothing here is a copied literal.
    let model_json: Value = serde_json::from_slice(&document).unwrap();
    let switch = &model_json["types"][0];
    assert_eq!(switch["displayName"], "Switch");
    let field = switch["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["name"] == "on")
        .expect("the Switch has an `on` field");
    assert_eq!(field["typeRef"], "ix://quire/native/Boolean");
    let operation = &switch["operations"][0];
    assert_eq!(operation["name"], "set");
    let params = operation["params"].as_array().unwrap();
    assert_eq!(params.len(), 1, "one ordered Boolean input");
    assert_eq!(params[0]["typeRef"], "ix://quire/native/Boolean");

    let transitions: Vec<_> = package
        .graph()
        .nodes
        .iter()
        .filter(|node| {
            matches!(
                node.body,
                CheckedTerm::Application {
                    operator: ApplicationOperator::StateTransition,
                    ..
                }
            )
        })
        .collect();
    assert_eq!(transitions.len(), 1, "one typed transition application");
    let transition = transitions[0];
    let CheckedTerm::Application {
        arguments,
        result_type,
        ..
    } = &transition.body
    else {
        unreachable!("filtered to an application");
    };

    // The five bindings, in the reader's published order.
    let bindings: Vec<(&str, &CheckedTerm)> = arguments
        .iter()
        .map(|argument| match argument {
            CheckedTerm::Binding { name, value } => (name.as_ref(), &**value),
            other => panic!("a transition argument is a binding: {other:?}"),
        })
        .collect();
    let names: Vec<&str> = bindings.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["anchor", "subject", "inputs", "pre", "post"]);
    let [anchor, subject, inputs, pre, post] = [0, 1, 2, 3, 4].map(|at| bindings[at].1);

    // The Boolean result type, the same node the `on` field resolves to.
    let boolean = node(&package, result_type);
    assert_eq!(boolean.semantic_form.as_ref(), "boolean");
    assert_eq!(transition.semantic_type, *result_type);

    // Anchor: the model's `set` operation.
    let anchor = node(&package, target(anchor));
    assert_eq!(anchor.semantic_form.as_ref(), "operation_anchor");
    let CheckedTerm::OperationAnchorBody(anchor_body) = &anchor.body else {
        panic!("an anchor body is a typed operation anchor");
    };
    assert_eq!(anchor_body.execution_point, None, "a no-point anchor");
    let anchored = anchor_body.members.iter().find_map(|member| match member {
        CheckedTerm::Binding { name, value } if name.as_ref() == "operation" => match &**value {
            CheckedTerm::Literal { value, .. } => value.as_str(),
            _ => None,
        },
        _ => None,
    });
    assert_eq!(anchored, operation["name"].as_str());

    // Subject: the actual `Switch.on` field projection, typed Boolean, whose
    // declaration is the `Switch` model type.
    let subject = node(&package, target(subject));
    assert_eq!(
        subject.semantic_type, *result_type,
        "the subject is Boolean"
    );
    let CheckedTerm::Application { operation, .. } = &subject.body else {
        panic!("the subject is a typed projection application");
    };
    let (declaration, field_name) = operation
        .projection_field()
        .expect("admitted operation views")
        .expect("the subject projects a field");
    assert_eq!(Some(field_name), field["name"].as_str());
    let owner = node(&package, declaration);
    assert_eq!(owner.semantic_form.as_ref(), "object_type");
    assert!(
        matches!(
            &owner.owner,
            Some(quire_contract_model::CheckedNodeOwner::Model { node, .. })
                if Some(&**node) == switch["identity"].as_str()
        ),
        "the subject's field belongs to the Switch: {:?}",
        owner.owner
    );

    // Inputs: the ordered Boolean `flag`, in the model's declared order.
    let CheckedTerm::Aggregate { members } = inputs else {
        panic!("inputs are an ordered aggregate");
    };
    let flags: Vec<(String, &CheckedNodeId)> = members
        .iter()
        .map(|member| {
            let parameter = node(&package, target(member));
            (parameter_name(parameter), &parameter.semantic_type)
        })
        .collect();
    let declared: Vec<&str> = params.iter().map(|p| p["name"].as_str().unwrap()).collect();
    assert_eq!(
        flags
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        declared
    );
    assert!(
        flags.iter().all(|(_, ty)| *ty == result_type),
        "every input is Boolean"
    );

    // Pre-state: the source states no `pre` clause, so none is bound.
    assert_eq!(
        pre,
        &CheckedTerm::Aggregate { members: vec![] },
        "the source states no pre clause"
    );

    // Post-state: the `post` clause, a Boolean result over `self.on`.
    let post = node(&package, target(post));
    assert_eq!(post.semantic_form.as_ref(), "state_clause");
    assert_eq!(
        post.semantic_type, *result_type,
        "the post result is Boolean"
    );
    let CheckedTerm::Application { operation, .. } = &post.body else {
        panic!("the post clause is a typed application");
    };
    assert_eq!(
        operation.state_clause_kind().expect("admitted views"),
        Some(quire_contract_model::StateClauseKind::Postcondition)
    );

    // The claim occurrence, and the source region it points at.
    assert_eq!(transition.occurrences.len(), 1);
    assert_eq!(transition.occurrences[0].role, CheckedOccurrenceRole::Claim);
    let ordinal = transition.occurrences[0].ordinal;
    let rows: Vec<_> = package
        .source_map()
        .iter()
        .filter(|row| row.node_id == transition.node_id && row.role == CheckedOccurrenceRole::Claim)
        .collect();
    assert_eq!(rows.len(), 1, "one source row for the claim occurrence");
    assert_eq!(rows[0].ordinal, ordinal);
    let [region] = rows[0].regions.as_slice() else {
        panic!("one source region");
    };
    let body = "self.on";
    let start = source
        .find(&format!("{{ {body} }}"))
        .expect("the post body")
        + 2;
    assert_eq!(
        (region.start, region.end),
        (start as u64, (start + body.len()) as u64),
        "the region is the post clause body in the rendered source"
    );
    assert_eq!(&source[start..start + body.len()], body);
    assert_eq!(&*region.source.authority, "local");
    assert_eq!(
        &*region.source.identity,
        format!("source:{}", program.display())
    );
    // The post clause is the same claim occurrence at the same region.
    let post_row = package
        .source_map()
        .iter()
        .find(|row| row.node_id == post.node_id && row.role == CheckedOccurrenceRole::Claim)
        .expect("the post clause's claim row");
    assert_eq!(post_row.regions, rows[0].regions);
}

/// Fail closed: the typed readback is not a fallback to a generic term. A
/// transition whose bindings are reordered, or whose claim occurrence is
/// another role, is refused by the strict reader instead of admitting as a
/// differently shaped term.
#[test]
fn a_malformed_switch_transition_is_refused_not_read_as_a_generic_term() {
    let model = Path::new(SWITCH_MODEL);
    let document = std::fs::read(model).unwrap();
    let mut command = strings(&["--model"]);
    command.extend(args(&[model, Path::new(SWITCH_TEMPLATE)]));
    let bytes = emit(&Request::parse(&command).unwrap()).unwrap();
    let wire: Value = serde_json::from_slice(&bytes).unwrap();
    let position = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node| node["node_tag"] == "state" && node["semantic_form"] == "transition")
        .expect("the transition node");

    let refused = |mutate: &dyn Fn(&mut Value)| {
        let mut wire = wire.clone();
        mutate(&mut wire["semantic_graph"]["nodes"][position]);
        let bytes = serde_json::to_vec(&wire).unwrap();
        let mut evidence = CheckedPackageEvidence::new();
        for feature in wire["lock"]["required_features"].as_array().unwrap() {
            evidence.support_feature(feature.as_str().unwrap());
        }
        for (digest, document) in qsl_semantics::model::intake::package_input([document.as_slice()])
        {
            evidence
                .insert_domain_package_document(qsl_semantics::model::key::hex(&digest), document)
                .unwrap();
        }
        CheckedPackageV2::read(&bytes, CheckedPackageReadLimits::bounded(), &evidence)
    };
    // Control: the unmutated node admits through the same path.
    assert!(matches!(
        refused(&|_| {}),
        CheckedPackageV2ReadResult::Admitted(_)
    ));
    for (label, mutate) in [
        (
            "subject and inputs swapped",
            (&|node: &mut Value| node["body"]["arguments"].as_array_mut().unwrap().swap(1, 2))
                as &dyn Fn(&mut Value),
        ),
        (
            "claim occurrence replaced by a declaration",
            &|node: &mut Value| node["occurrences"][0]["role"] = json!("declaration"),
        ),
    ] {
        let result = refused(mutate);
        assert!(
            matches!(result, CheckedPackageV2ReadResult::Refused(_)),
            "{label}: {result:?}"
        );
    }
}

/// A source the producer refuses (its `model` is not supplied) writes no bytes
/// and fails with the refusing stage's nonzero exit code; a malformed command
/// line and an unreadable file fail the same way.
#[test]
fn a_refused_source_writes_nothing_and_exits_nonzero() {
    let directory = tempfile::tempdir().unwrap();
    let program = spine_model_source(directory.path());
    let mut stdout = Vec::new();
    let refusal = run(&args(&[&program]), &mut stdout).expect_err("the model is not supplied");
    assert!(matches!(refusal, Error::Producer(_)), "{refusal:?}");
    assert_ne!(refusal.exit_code(), 0);
    assert!(!refusal.to_string().is_empty());
    assert!(stdout.is_empty());

    for bad in [
        strings(&[]),
        strings(&["--model"]),
        strings(&["--bogus", "x"]),
        strings(&["--library", "no-equals", "x"]),
        strings(&["a.native", "b.native"]),
    ] {
        let refusal = run(&bad, &mut stdout).expect_err("a malformed command line");
        assert!(matches!(refusal, Error::Usage), "{bad:?}: {refusal:?}");
        assert_ne!(refusal.exit_code(), 0);
    }
    let missing = run(
        &strings(&["/nonexistent/emit-checked-v2.native"]),
        &mut stdout,
    )
    .expect_err("an unreadable file");
    assert!(matches!(missing, Error::Read { .. }), "{missing:?}");
    assert_ne!(missing.exit_code(), 0);
    assert!(stdout.is_empty());
}

/// The ceiling: a source of exactly the source-size ceiling emits, one byte
/// more is the producer's size refusal (exit 22, no bytes, without the
/// producer's `--limit` hint, which names a flag this tool lacks); an
/// over-ceiling domain package or library file is an explicit refusal naming
/// the file, exit 20, never truncated.
#[test]
fn the_size_ceiling_is_exact_and_over_cap_inputs_refuse_explicitly() {
    use qsl_foundation::source::MAX_SOURCE_BYTES;
    let directory = tempfile::tempdir().unwrap();
    let mut text = std::fs::read_to_string("tests/fixtures/spine-compile.native").unwrap();
    assert!(text.len() < MAX_SOURCE_BYTES);
    text.push('\n');
    text.push_str(&" ".repeat(MAX_SOURCE_BYTES - text.len()));
    assert_eq!(text.len(), MAX_SOURCE_BYTES);
    let program = directory.path().join("at-cap.native");
    std::fs::write(&program, &text).unwrap();
    let mut stdout = Vec::new();
    run(&args(&[&program]), &mut stdout).expect("a source at the ceiling emits");
    assert!(!stdout.is_empty());

    let over = directory.path().join("over-cap.native");
    std::fs::write(&over, format!("{text} ")).unwrap();
    let mut refused = Vec::new();
    let refusal = run(&args(&[&over]), &mut refused).expect_err("one byte over the ceiling");
    assert!(matches!(refusal, Error::Producer(_)), "{refusal:?}");
    assert_eq!(refusal.exit_code(), 22, "{refusal}");
    assert!(!refusal.to_string().contains("--limit"), "{refusal}");
    assert!(refused.is_empty());

    let model = directory.path().join("big.semantic-ir.json");
    std::fs::write(&model, vec![b' '; MAX_SOURCE_BYTES + 1]).unwrap();
    let mut command = strings(&["--model"]);
    command.extend(args(&[&model, &program]));
    let refusal = run(&command, &mut refused).expect_err("an over-cap model file");
    assert!(matches!(refusal, Error::TooLarge { .. }), "{refusal:?}");
    assert_eq!(refusal.exit_code(), 20);
    assert!(
        refusal.to_string().contains("big.semantic-ir.json"),
        "{refusal}"
    );
    let mut command = strings(&["--library"]);
    command.push(OsString::from(format!("lib={}", model.display())));
    command.extend(args(&[&program]));
    let refusal = run(&command, &mut refused).expect_err("an over-cap library file");
    assert!(matches!(refusal, Error::TooLarge { .. }), "{refusal:?}");
    assert!(refused.is_empty());
}

/// Accepted execution points (FR-104-AC-15) supplied with
/// `--execution-points` reach the emitted state clause, the bytes differ from
/// the point-free emission, and IR's strict reader admits them; an unknown
/// clause, a clause kind that does not accept the point and a malformed file
/// each write no bytes and exit nonzero.
#[test]
fn accepted_execution_points_are_emitted_and_bad_points_refuse() {
    let model = Path::new("examples/switch-transition/switch.semantic-ir.json");
    let program = Path::new("examples/switch-transition/switch.native.in");
    let document = std::fs::read(model).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let points = |text: &str| {
        let file = directory.path().join(format!("points-{:x}.json", {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut hasher);
            hasher.finish()
        }));
        std::fs::write(&file, text).unwrap();
        file
    };
    let command = |extra: &[&Path]| {
        let mut command = strings(&["--model"]);
        command.extend(args(&[model]));
        for file in extra {
            command.extend(strings(&["--execution-points"]));
            command.extend(args(&[file]));
        }
        command.extend(args(&[program]));
        command
    };

    let plain = emit(&Request::parse(&command(&[])).unwrap()).expect("point-free emits");
    let accepted = points(r#"{"Q1": {"kind": "post", "operation": "set"}}"#);
    let mut stdout = Vec::new();
    run(&command(&[&accepted]), &mut stdout).expect("the accepted point emits");
    assert_ne!(stdout, plain, "the point changes the emitted bytes");
    read_back(&stdout, &[&document]);
    let wire: Value = serde_json::from_slice(&stdout).unwrap();
    let clause = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["semantic_form"] == "state_clause")
        .expect("the state clause node");
    assert_eq!(
        clause["body"]["operation"]["member"]["execution_point"],
        json!({"kind": "post", "operation": "set"}),
    );
    let bare: Value = serde_json::from_slice(&plain).unwrap();
    assert!(bare["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["semantic_form"] == "state_clause")
        .all(|node| node["body"]["operation"]["member"]["execution_point"].is_null()));

    // (file text, refused by the checker rather than by the points reader)
    for (text, producer) in [
        (
            r#"{"NotAClause": {"kind": "post", "operation": "set"}}"#,
            true,
        ),
        (r#"{"Q1": {"kind": "pre", "operation": "set"}}"#, true),
        (
            r#"{"Q1": {"kind": "post", "operation": "not a name!"}}"#,
            true,
        ),
        (r#"{"Q1": {"kind": "sideways"}}"#, false),
        (
            r#"{"Q1": {"kind": "post", "operation": "set"}, "Q1": {"kind": "post", "operation": "set"}}"#,
            false,
        ),
        ("{}", false),
        (
            r#"{"Q1": {"kind": "post", "operation": "set", "extra": 1}}"#,
            false,
        ),
        ("not json", false),
    ] {
        let mut refused = Vec::new();
        let refusal = run(&command(&[&points(text)]), &mut refused).expect_err(text);
        if producer {
            assert!(matches!(refusal, Error::Producer(_)), "{text}: {refusal:?}");
        } else {
            assert!(
                matches!(refusal, Error::Points { .. }),
                "{text}: {refusal:?}"
            );
            assert_eq!(refusal.exit_code(), 20, "{text}");
        }
        assert_ne!(refusal.exit_code(), 0);
        assert!(refused.is_empty(), "{text}");
    }
}

/// Several accepted points in one file each reach their own clause: the
/// example source plus a second clause, both over operation `set`.
#[test]
fn several_accepted_execution_points_each_reach_their_clause() {
    let model = Path::new("examples/switch-transition/switch.semantic-ir.json");
    let document = std::fs::read(model).unwrap();
    let source = rendered_switch_source() + "pre Q0 using v on M::Switch::set { true }\n";
    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("two-clauses.native");
    std::fs::write(&program, source).unwrap();
    let file = directory.path().join("points.json");
    std::fs::write(
        &file,
        r#"{"Q0": {"kind": "pre", "operation": "set"}, "Q1": {"kind": "post", "operation": "set"}}"#,
    )
    .unwrap();
    let mut command = strings(&["--model"]);
    command.extend(args(&[model]));
    command.extend(strings(&["--execution-points"]));
    command.extend(args(&[&file, &program]));
    let mut stdout = Vec::new();
    run(&command, &mut stdout).expect("both points emit");
    read_back(&stdout, &[&document]);
    let wire: Value = serde_json::from_slice(&stdout).unwrap();
    let mut points: Vec<String> = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["semantic_form"] == "state_clause")
        .map(|node| node["body"]["operation"]["member"]["execution_point"].to_string())
        .collect();
    points.sort();
    let mut expected = vec![
        json!({"kind": "pre", "operation": "set"}).to_string(),
        json!({"kind": "post", "operation": "set"}).to_string(),
    ];
    expected.sort();
    assert_eq!(points, expected, "each clause carries its own point");
}

/// A points file over the file ceiling is an explicit refusal naming it
/// (exit 20), never truncated.
#[test]
fn an_over_cap_points_file_is_refused_explicitly() {
    use qsl_foundation::source::MAX_SOURCE_BYTES;
    let directory = tempfile::tempdir().unwrap();
    let points = directory.path().join("big-points.json");
    std::fs::write(&points, vec![b' '; MAX_SOURCE_BYTES + 1]).unwrap();
    let mut command = strings(&["--model"]);
    command.extend(args(&[Path::new(
        "examples/switch-transition/switch.semantic-ir.json",
    )]));
    command.extend(strings(&["--execution-points"]));
    command.extend(args(&[&points]));
    command.extend(args(&[Path::new(
        "examples/switch-transition/switch.native.in",
    )]));
    let mut stdout = Vec::new();
    let refusal = run(&command, &mut stdout).expect_err("an over-cap points file");
    assert!(matches!(refusal, Error::TooLarge { .. }), "{refusal:?}");
    assert_eq!(refusal.exit_code(), 20);
    assert!(refusal.to_string().contains("big-points.json"), "{refusal}");
    assert!(stdout.is_empty());
}

const SWITCH_MODEL: &str = "examples/switch-transition/switch.semantic-ir.json";
const SWITCH_TEMPLATE: &str = "examples/switch-transition/switch.native.in";

/// `--source-out` is written only after the producer accepted the template
/// and carries exactly the template with every `{model_digest}` replaced:
/// `{other}` text is untouched, and `{{model_digest}}` becomes the digest in
/// single braces (a plain replace of the inner token). A producer refusal, a
/// template refusal (no model, several models, a model that is not a domain
/// package) and an unreadable source each leave no `--source-out` file and no
/// bytes; a source that is not a template is not copied.
#[test]
fn source_out_is_written_only_for_an_accepted_template() {
    let directory = tempfile::tempdir().unwrap();
    let model = Path::new(SWITCH_MODEL);
    let document = std::fs::read(model).unwrap();
    let digest = model_digest(&document);
    let out = directory.path().join("rendered.native");
    let run_with = |program: &Path, models: &[&Path]| {
        let mut command = strings(&["--source-out"]);
        command.extend(args(&[&out]));
        for model in models {
            command.extend(strings(&["--model"]));
            command.extend(args(&[model]));
        }
        command.extend(args(&[program]));
        let mut stdout = Vec::new();
        let result = run(&command, &mut stdout);
        (result, stdout)
    };

    // Success: present and correct; `{other}` untouched, `{{model_digest}}`
    // is the digest inside single braces, every placeholder is filled.
    let template = std::fs::read_to_string(SWITCH_TEMPLATE).unwrap()
        + "// {other} {{model_digest}} {model_digest}\n";
    let accepted = directory.path().join("accepted.native.in");
    std::fs::write(&accepted, &template).unwrap();
    let (result, stdout) = run_with(&accepted, &[model]);
    result.expect("the template emits");
    assert!(!stdout.is_empty());
    let rendered = std::fs::read_to_string(&out).unwrap();
    assert_eq!(rendered, template.replace(MODEL_DIGEST_TOKEN, &digest));
    assert!(rendered.contains(&format!("// {{other}} {{{digest}}} {digest}\n")));
    assert!(!rendered.contains(MODEL_DIGEST_TOKEN));
    assert!(std::fs::read_dir(directory.path())
        .unwrap()
        .all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp")));
    std::fs::remove_file(&out).unwrap();

    // Producer refusal: an invalid clause after a valid template.
    let refused = directory.path().join("refused.native.in");
    std::fs::write(
        &refused,
        template.clone() + "post Q9 using v on M::Nope::x { true }\n",
    )
    .unwrap();
    let (result, stdout) = run_with(&refused, &[model]);
    assert!(matches!(result, Err(Error::Producer(_))), "{result:?}");
    assert!(
        stdout.is_empty() && !out.exists(),
        "a producer refusal leaves no file"
    );

    // Template refusals: no model, several models, an empty model, a model
    // that is not a domain package.
    let (result, stdout) = run_with(&accepted, &[]);
    assert!(
        matches!(result, Err(Error::TemplateModels { supplied: 0 })),
        "{result:?}"
    );
    assert!(stdout.is_empty() && !out.exists());
    let (result, stdout) = run_with(&accepted, &[model, model]);
    let refusal = result.expect_err("several models");
    assert!(
        matches!(refusal, Error::TemplateModels { supplied: 2 }),
        "{refusal:?}"
    );
    assert_eq!(refusal.exit_code(), 20);
    assert!(stdout.is_empty() && !out.exists());
    for (name, content) in [
        ("empty.json", ""),
        ("junk.json", "{\"not\": \"a package\"}"),
    ] {
        let bad = directory.path().join(name);
        std::fs::write(&bad, content).unwrap();
        let (result, stdout) = run_with(&accepted, &[&bad]);
        let refusal = result.expect_err("an unusable model");
        assert!(
            matches!(refusal, Error::TemplateModel),
            "{name}: {refusal:?}"
        );
        assert!(!refusal.to_string().contains("exactly one"), "{refusal}");
        assert_eq!(refusal.exit_code(), 20);
        assert!(stdout.is_empty() && !out.exists(), "{name}");
    }

    // A non-UTF-8 template is filled byte-wise and the producer refuses it;
    // substitution is never silently skipped, and nothing is written.
    let mut bytes = template.into_bytes();
    bytes.extend_from_slice(b"// \xff\xfe\n");
    let binary = directory.path().join("binary.native.in");
    std::fs::write(&binary, bytes).unwrap();
    let (result, _) = run_with(&binary, &[model]);
    assert!(matches!(result, Err(Error::Producer(_))), "{result:?}");
    assert!(!out.exists());

    // A source that is not a template is not copied.
    let plain = directory.path().join("plain.native");
    std::fs::write(&plain, rendered_switch_source()).unwrap();
    let (result, _) = run_with(&plain, &[model]);
    result.expect("a plain source emits");
    assert!(!out.exists(), "a non-template source is not copied");
}

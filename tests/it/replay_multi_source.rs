// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1210 / FR-480-AC-7: package source references replay in any order and
//! malformed package source lists refuse before the spine recompiles.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_cst::Limits as SourceLimits;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{self, ParseRequest};
use qsl_replay::{
    call_site, replay, replay_state_clause, DependencyEntryWire, DependencyInput, EnvelopeSource,
    ObligationIdentity, QualifiedName, ReplayLimits, ReplayRefusal, ReplayRequestWire,
    ReplaySource, ReplayTargetMembers, RequestTargetWire, StateClauseCounterexample,
    StateEnvironment, SuppliedLibrary, WitnessEnvelope, WitnessPacket,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::observation::{ClauseSelectionInput, DocumentRef, SelectedObject};
use quire_exact::{Cancel, PathSegment, ScalarLimits};
use serde_json::{json, Value};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

const FIRST: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function p using v(): Boolean pure { h() }\n";
const CALL_SITE_SEED: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function p using v(): Boolean pure { h() }\n\
    function h using v(): Boolean pure { true }\n";
const SECOND: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function h using v(): Boolean pure { true }\n";
const AUTHORITY: &str = "agent-ix";
const FIRST_ID: &str = "test:replay-first";
const SECOND_ID: &str = "test:replay-second";
const PACKAGE: &str = "example/config-version";
const HISTORY: &str = "ix://example/config-version/config_history";
const LIBRARY_FIRST: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function f using v(x: Boolean): Boolean pure { g(x) }\n";
const LIBRARY_SECOND: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function g using v(x: Boolean): Boolean pure { x }\n";
const LIBRARY_CALLER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    import \"test/library\" as l;\n\
    function p using v(): Boolean pure { l::f(true) }\n";

fn selected() -> QualifiedName {
    QualifiedName::new(vec![quire_exact::Identifier::new("p").unwrap()]).unwrap()
}

fn reference(identity: &str) -> (String, String) {
    (AUTHORITY.to_owned(), identity.to_owned())
}

fn request(source_order: Vec<&str>, documents: Vec<Vec<u8>>) -> ReplayRequestWire {
    let first_identity = SourceIdentity::new(AUTHORITY, FIRST_ID);
    let selection = call_site(
        first_identity,
        "fixture".to_owned(),
        FIRST_ID,
        CALL_SITE_SEED.as_bytes(),
        std::iter::empty::<&[u8]>(),
        &DependencyInput::default(),
        &selected(),
    )
    .expect("the function site is discoverable in the first source");
    let site = selection.site;
    let target = RequestTargetWire::function(selected(), site.declaration.clone());
    ReplayRequestWire {
        package_name: "fixture".to_owned(),
        profile_selections: Vec::new(),
        sources: source_order.into_iter().map(reference).collect(),
        dependencies: Vec::new(),
        replay_target: target,
        source: ReplaySource::Input(Vec::new()),
        obligation_identity: site.obligation_identity().clone(),
        backend: "tc-1210".to_owned(),
        state_environment: StateEnvironment::default(),
        accounting_limits: UNLIMITED,
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: vec![
            (
                AUTHORITY.to_owned(),
                FIRST_ID.to_owned(),
                FIRST.as_bytes().to_vec(),
            ),
            (
                AUTHORITY.to_owned(),
                SECOND_ID.to_owned(),
                SECOND.as_bytes().to_vec(),
            ),
        ],
        documents,
    }
}

fn d10_model_document() -> Vec<u8> {
    fn query(name: &str) -> Value {
        let identity = format!("ix://{PACKAGE}/ConfigVersion/{name}");
        json!({
            "identity": identity,
            "name": name,
            "params": [],
            "returns": {
                "typeRef": "ix://quire/native/Integer",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                "nullable": false,
            },
            "pre": [], "post": [],
            "origin": {"source": {"sourceIdentity": format!("ix://{PACKAGE}/spec"), "path": "spec.qspec", "startLine": 1, "startColumn": 1}},
            "frame": {"modifies": [], "creates": [], "deletes": []},
        })
    }

    let mut envelope: Value =
        serde_json::from_str(crate::support::config_version_spine::spine::DOMAIN_PACKAGE)
            .expect("domain model fixture parses");
    let types = envelope["types"].as_array_mut().expect("types");
    let config = types
        .iter_mut()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/ConfigVersion"))
        .expect("ConfigVersion type");
    config["operations"] = json!([query("size"), query("other")]);
    let mut sub_size = query("size");
    sub_size["identity"] = json!(format!("ix://{PACKAGE}/Sub/size"));
    sub_size["redefines"] = json!(format!("ix://{PACKAGE}/ConfigVersion/size"));
    let mut probe = query("probe");
    probe["identity"] = json!(format!("ix://{PACKAGE}/Sub/probe"));
    types.push(json!({
        "identity": format!("ix://{PACKAGE}/Sub"),
        "displayName": format!("ix://{PACKAGE}/Sub"),
        "kind": {"module": PACKAGE, "name": "object_type"},
        "roles": [], "constraints": [], "extensions": [], "unknownPolicy": "reject",
        "origin": {"generated": {"generatorIdentity": format!("ix://{PACKAGE}/Sub"), "generatorVersion": "1.0.0", "inputIdentities": [format!("ix://{PACKAGE}/Sub")]}},
        "supertypes": [format!("ix://{PACKAGE}/ConfigVersion")], "fields": [],
        "operations": [sub_size, probe],
    }));
    let history = envelope["populations"]
        .as_array_mut()
        .expect("populations")
        .iter_mut()
        .find(|population| population["identity"] == HISTORY)
        .expect("config history population");
    history["members"]
        .as_array_mut()
        .expect("population members")
        .push(json!(format!("ix://{PACKAGE}/Sub")));
    serde_json::to_vec(&envelope).expect("model document serializes")
}

struct D10Units {
    sources: Vec<(&'static str, String)>,
    model_document: Vec<u8>,
    occurrence_key: qsl_foundation::source::provenance::OccurrenceKey,
    clause_node: quire_exact::NodeKey,
    package_bytes: Vec<u8>,
}

fn d10_units(reverse: bool) -> D10Units {
    let lookup = "(lookup<M::ConfigVersion>(M::config_history, self) absent undefined) = self";
    let first = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile m = \"quire.model.complete/v1\";\n\
         model M = {PACKAGE:?};\n\
         body sizeA using m on M::ConfigVersion::size() {{ 1 }}\n\
         body sizeB using m on M::Sub::size() {{ 2 }}\n\
         pre PA using m on M::ConfigVersion::size {{ false }}\n\
         pre PB1 using m on M::Sub::size {{ {lookup} }}\n\
         pre I using m on M::Sub::probe {{ self.size() = 2 }}\n"
    );
    let second = String::from(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile m = \"quire.model.complete/v1\";\n\
         model M = \"example/config-version\";\n\
         pre PB2 using m on M::Sub::size { false }\n",
    );
    let identities = [
        SourceIdentity::new("test", "d10:u1"),
        SourceIdentity::new("test", "d10:u2"),
    ];
    let texts = [first, second];
    let order = if reverse { [1, 0] } else { [0, 1] };
    let requests = order
        .iter()
        .map(|index| ParseRequest {
            source: &identities[*index],
            path: identities[*index].identity.as_str(),
            bytes: texts[*index].as_bytes(),
        })
        .collect::<Vec<_>>();
    let parsed = requests
        .iter()
        .map(|request| {
            spine::parse(request, SourceLimits::default(), &Cancel::new())
                .expect("D10 unit parses")
                .into_value()
        })
        .collect();
    let dependencies = DependencyInput::default();
    let admitted = spine::admit_parsed_unit_list(parsed, &dependencies).expect("units admit");
    let model = d10_model_document();
    let packages = package_input([model.as_slice()]).expect("D10 model inputs");
    let models = spine::select_units(
        &admitted,
        &packages,
        ModelNormalizationLimits::default(),
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("D10 model selections admit")
    .into_value();
    let checked = spine::check_units(
        &admitted,
        "d10-package".to_owned(),
        &models,
        &dependencies,
        spine::SpineLimits::default(),
        &Cancel::new(),
    )
    .expect("D10 links")
    .into_value();
    let clause = checked[0]
        .package()
        .graph()
        .state_clause("I")
        .expect("D10 precondition exists");
    let clause_node = clause.identity().clone();
    let occurrence_key = qsl_foundation::source::provenance::OccurrenceKey::new(
        clause_node.clone(),
        clause.claim().clone(),
    );
    let package_bytes =
        qsl_package::emit_checked(checked[0].package(), qsl_package::EmitLimits::fixture())
            .expect("D10 package emits")
            .package()
            .bytes()
            .to_vec();
    let sources = order
        .iter()
        .map(|index| {
            (
                if *index == 0 { "d10:u1" } else { "d10:u2" },
                texts[*index].clone(),
            )
        })
        .collect();
    D10Units {
        sources,
        model_document: model,
        occurrence_key,
        clause_node,
        package_bytes,
    }
}

fn d10_replay(reverse: bool) -> (qsl_replay::StateClauseReplayResult, Vec<u8>) {
    let D10Units {
        sources: units,
        model_document: model,
        occurrence_key,
        clause_node,
        package_bytes,
    } = d10_units(reverse);
    let target = QualifiedName::new(vec![quire_exact::Identifier::new("I").unwrap()]).unwrap();
    let snapshot_ref = DocumentRef {
        authority: "test".to_owned(),
        identity: "d10:observation".to_owned(),
    };
    let snapshot = json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": snapshot_ref.authority,
            "identity": snapshot_ref.identity,
        },
        "observation": "pre",
        "model": {"identity": PACKAGE, "version": "1.0.0"},
        "populations": [{
            "population": HISTORY,
            "complete": true,
            "objects": [{
                "key": "d1",
                "type": format!("ix://{PACKAGE}/Sub"),
                "fields": {
                    "versionNumber": {"integer": "1"},
                    "parent": {"absent": {}},
                },
            }],
        }],
    })
    .to_string()
    .into_bytes();
    let observation = ClauseSelectionInput::PreCall {
        snapshot: snapshot_ref.clone(),
        self_object: Some(SelectedObject {
            population: HISTORY.to_owned(),
            key: "d1".to_owned(),
        }),
        parameters: BTreeMap::new(),
    };
    let packet = WitnessPacket {
        obligation_identity: Some(ObligationIdentity::new(
            quire_exact::NodeKey::new("d10", vec![PathSegment::Text("I".into())], 0)
                .expect("obligation node"),
            None,
            0,
        )),
        occurrence_key: Some(occurrence_key),
        clause_node: Some(clause_node),
        replay_target: ReplayTargetMembers::state_clause(target.clone()),
        profile_selections: Some(Vec::new()),
        run_limits: Some(UNLIMITED),
        declared_domains: Some(Vec::new()),
        backend: Some("tc-1210".to_owned()),
        trace_position: Some(None),
        source: Some(EnvelopeSource::Input(Vec::new())),
        family_payload: Some(StateClauseCounterexample {
            observation,
            witness: None,
        }),
    };
    let envelope = WitnessEnvelope::reconstruct(packet, ReplayLimits::default())
        .expect("D10 replay envelope reconstructs");
    let references = units
        .iter()
        .map(|(identity, _)| ("test".to_owned(), (*identity).to_owned()))
        .collect::<Vec<_>>();
    let mut byte_provision = units
        .iter()
        .map(|(identity, text)| {
            (
                "test".to_owned(),
                (*identity).to_owned(),
                text.as_bytes().to_vec(),
            )
        })
        .collect::<Vec<_>>();
    byte_provision.push(("agent-ix".to_owned(), PACKAGE.to_owned(), model.clone()));
    byte_provision.push((
        snapshot_ref.authority.clone(),
        snapshot_ref.identity.clone(),
        snapshot.clone(),
    ));
    let wire = ReplayRequestWire {
        package_name: "d10-package".to_owned(),
        profile_selections: Vec::new(),
        sources: references,
        dependencies: Vec::new(),
        replay_target: RequestTargetWire::state_clause(target),
        source: ReplaySource::Input(Vec::new()),
        obligation_identity: envelope.obligation_identity(),
        backend: "tc-1210".to_owned(),
        state_environment: StateEnvironment::default(),
        accounting_limits: UNLIMITED,
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision,
        documents: vec![model, snapshot],
    };
    let replayed = replay_state_clause(wire, &envelope, ReplayLimits::default())
        .expect("D10 state clause replays from either reference order");
    (replayed, package_bytes)
}

fn dependency_reference(identity: &str) -> (String, String) {
    ("test".to_owned(), identity.to_owned())
}

fn multi_source_dependency_request(
    sources: Vec<(String, String)>,
    documents: Vec<Vec<u8>>,
    extra_source: Option<(String, String, Vec<u8>)>,
) -> (ReplayRequestWire, Vec<u8>) {
    let first = SuppliedLibrary {
        identity: "test/library".to_owned(),
        source: SourceIdentity::new("test", "library-u1"),
        path: "library-u1.native".to_owned(),
        bytes: LIBRARY_FIRST.as_bytes().to_vec(),
    };
    let second = SuppliedLibrary {
        identity: "test/library".to_owned(),
        source: SourceIdentity::new("test", "library-u2"),
        path: "library-u2.native".to_owned(),
        bytes: LIBRARY_SECOND.as_bytes().to_vec(),
    };
    let units = vec![first.clone(), second.clone()];
    let dependencies = DependencyInput::new([first.clone()])
        .expect("one library identity admits")
        .with_library_units("test/library", units)
        .expect("both units belong to one library");
    let selection = call_site(
        SourceIdentity::new(AUTHORITY, FIRST_ID),
        "fixture".to_owned(),
        FIRST_ID,
        LIBRARY_CALLER.as_bytes(),
        std::iter::empty::<&[u8]>(),
        &dependencies,
        &selected(),
    )
    .expect("the call site resolves through the multi-unit dependency");
    let package = selection.package;
    let dependency = DependencyEntryWire {
        identity: "test/library".to_owned(),
        sources,
    };
    let target = RequestTargetWire::function(selected(), selection.site.declaration.clone());
    let mut byte_provision = vec![
        (
            AUTHORITY.to_owned(),
            FIRST_ID.to_owned(),
            LIBRARY_CALLER.as_bytes().to_vec(),
        ),
        (
            "test".to_owned(),
            "library-u1".to_owned(),
            LIBRARY_FIRST.as_bytes().to_vec(),
        ),
        (
            "test".to_owned(),
            "library-u2".to_owned(),
            LIBRARY_SECOND.as_bytes().to_vec(),
        ),
    ];
    if let Some(source) = extra_source {
        byte_provision.push(source);
    }
    let wire = ReplayRequestWire {
        package_name: "fixture".to_owned(),
        profile_selections: Vec::new(),
        sources: vec![reference(FIRST_ID)],
        dependencies: vec![dependency],
        replay_target: target,
        source: ReplaySource::Input(Vec::new()),
        obligation_identity: selection.site.obligation_identity().clone(),
        backend: "tc-1210".to_owned(),
        state_environment: StateEnvironment::default(),
        accounting_limits: UNLIMITED,
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision,
        documents,
    };
    (wire, package)
}

fn multi_source_dependency_replay(reverse: bool) -> (qsl_replay::ReplayResult, Vec<u8>) {
    let sources = if reverse {
        vec![
            dependency_reference("library-u2"),
            dependency_reference("library-u1"),
        ]
    } else {
        vec![
            dependency_reference("library-u1"),
            dependency_reference("library-u2"),
        ]
    };
    let (wire, package) = multi_source_dependency_request(sources, Vec::new(), None);
    let replayed = replay(wire, ReplayLimits::default())
        .expect("multi-unit dependency recompiles and replays");
    (replayed, package)
}

#[trace("TC-1210", "FR-480-AC-7")]
#[test]
fn replay_accepts_canonical_and_reversed_dependency_unit_sources() {
    let (canonical, canonical_package) = multi_source_dependency_replay(false);
    let (reversed, reversed_package) = multi_source_dependency_replay(true);
    assert_eq!(canonical_package, reversed_package);
    assert_eq!(canonical, reversed);
    let qsl_replay::ReplayResult::Input(result) = canonical else {
        panic!("input replay result expected");
    };
    assert_eq!(
        result.value(),
        Some(qsl_replay::EvaluatedValue::Boolean(true))
    );
}

fn assert_dependency_source_list_refusal(mut wire: ReplayRequestWire) {
    wire.byte_provision[0].2 = b"not valid QSL".to_vec();
    let refusal = replay(wire, ReplayLimits::default())
        .expect_err("invalid dependency source lists refuse before source parsing");
    assert!(matches!(
        &refusal,
        ReplayRefusal::DependencySources { index: 0, .. }
    ));
    assert_eq!(refusal.code(), qsl_foundation::Code::InvalidPackage);
    assert_eq!(refusal.cause(), Some("invalid-value"));
    assert_eq!(
        refusal.pointer().as_deref(),
        Some("/package/dependencies/0/sources")
    );
}

#[trace("TC-1210", "FR-480-AC-7")]
#[test]
fn empty_dependency_source_list_refuses_at_its_sources_pointer() {
    let (wire, _) = multi_source_dependency_request(Vec::new(), Vec::new(), None);
    assert_dependency_source_list_refusal(wire);
}

#[trace("TC-1210", "FR-480-AC-7")]
#[test]
fn duplicate_dependency_source_owner_refuses_at_its_sources_pointer() {
    let repeated = dependency_reference("library-u1");
    let (wire, _) =
        multi_source_dependency_request(vec![repeated.clone(), repeated], Vec::new(), None);
    assert_dependency_source_list_refusal(wire);
}

#[trace("TC-1210", "FR-480-AC-7")]
#[test]
fn definition_document_dependency_source_refuses_at_its_sources_pointer() {
    let document = crate::support::config_version_spine::spine::DOMAIN_PACKAGE;
    let identity = "example/config-version";
    let (wire, _) = multi_source_dependency_request(
        vec![reference(identity)],
        vec![document.as_bytes().to_vec()],
        Some((
            AUTHORITY.to_owned(),
            identity.to_owned(),
            b"not valid QSL".to_vec(),
        )),
    );
    assert_dependency_source_list_refusal(wire);
}

#[trace("TC-1210", "FR-480-AC-7")]
#[test]
fn replay_accepts_canonical_and_reversed_unit_sources() {
    let (canonical, canonical_package_id) = d10_replay(false);
    let (reversed, reversed_package_id) = d10_replay(true);
    assert_eq!(canonical_package_id, reversed_package_id);
    assert_eq!(canonical.package_name(), reversed.package_name());
    assert_eq!(canonical.clause_node(), reversed.clause_node());
    assert_eq!(canonical.occurrence_key(), reversed.occurrence_key());
    assert_eq!(canonical.result(), reversed.result());
}

#[trace("TC-1210", "FR-480-AC-7")]
#[test]
fn duplicate_unit_source_refuses_before_recompile() {
    let mut wire = request(vec![FIRST_ID, FIRST_ID], Vec::new());
    wire.byte_provision[0].2 = b"not valid QSL".to_vec();
    let refusal =
        replay(wire, ReplayLimits::default()).expect_err("a repeated unit owner is refused");
    assert!(matches!(refusal, ReplayRefusal::PackageSources(_)));
    assert_eq!(refusal.code(), qsl_foundation::Code::InvalidPackage);
    assert_eq!(refusal.cause(), Some("invalid-value"));
    assert_eq!(refusal.pointer().as_deref(), Some("/package/sources"));
}

#[trace("TC-1210", "FR-480-AC-7")]
#[test]
fn definition_document_source_refuses_before_recompile() {
    let document = crate::support::config_version_spine::spine::DOMAIN_PACKAGE;
    let mut wire = request(
        vec!["example/config-version"],
        vec![document.as_bytes().to_vec()],
    );
    wire.byte_provision.push((
        AUTHORITY.to_owned(),
        "example/config-version".to_owned(),
        b"not valid QSL".to_vec(),
    ));
    let refusal = replay(wire, ReplayLimits::default())
        .expect_err("a definition document cannot be a source unit");
    assert!(matches!(refusal, ReplayRefusal::PackageSources(_)));
    assert_eq!(refusal.code(), qsl_foundation::Code::InvalidPackage);
    assert_eq!(refusal.cause(), Some("invalid-value"));
    assert_eq!(refusal.pointer().as_deref(), Some("/package/sources"));
}

fn supplied(identity: &str, source: &str, text: &str) -> SuppliedLibrary {
    SuppliedLibrary {
        identity: identity.to_owned(),
        source: SourceIdentity::new("test", source),
        path: format!("{source}.native"),
        bytes: text.as_bytes().to_vec(),
    }
}

/// FR-098: a unit shared by two libraries other than the first one refuses
/// whichever library was searched first (A=[a1], B=[b1,s], C=[c1,s]).
#[trace("FR-098-AC-7")]
#[test]
fn shared_unit_owner_is_found_in_any_other_library() {
    let a1 = supplied("test/a", "a1", LIBRARY_FIRST);
    let b1 = supplied("test/b", "b1", LIBRARY_FIRST);
    let c1 = supplied("test/c", "c1", LIBRARY_FIRST);
    let dependencies = DependencyInput::new([a1, b1.clone(), c1.clone()])
        .expect("three library identities admit")
        .with_library_units("test/b", vec![b1, supplied("test/b", "s", LIBRARY_SECOND)])
        .expect("B holds s alone");
    let refused = dependencies
        .with_library_units("test/c", vec![c1, supplied("test/c", "s", LIBRARY_SECOND)])
        .expect_err("s is already B's");
    assert!(
        matches!(
            refused,
            qsl_replay::spine::DependencyInputRefusal::SharedOwner { .. }
        ),
        "{refused:?}"
    );
}

/// FR-098: the request's temporal selections reach a library source's
/// emission; a selection naming no declaration there is surplus and refuses.
#[trace("FR-098-AC-7")]
#[test]
fn temporal_selection_on_a_library_source_reaches_its_emission() {
    let sources = vec![
        dependency_reference("library-u1"),
        dependency_reference("library-u2"),
    ];
    let (wire, _) = multi_source_dependency_request(sources, Vec::new(), None);
    let library = spine::parse(
        &ParseRequest {
            source: &SourceIdentity::new("test", "library-u1"),
            path: "library-u1.native",
            bytes: LIBRARY_FIRST.as_bytes(),
        },
        SourceLimits::default(),
        &Cancel::new(),
    )
    .expect("the library unit parses")
    .into_value();
    let selection = qsl_package::TemporalClockSelection {
        source: library.source().reference().clone(),
        declaration_span: qsl_foundation::source::Span { start: 0, end: 1 },
        clock: qsl_package::ClockConfiguration::EventPosition {
            sequence_authority: "none".into(),
        },
    };
    let refused =
        qsl_replay::replay_with_temporal_selections(wire, &[selection], ReplayLimits::default())
            .expect_err("a selection naming no temporal declaration is surplus");
    assert!(
        matches!(refused, ReplayRefusal::Recompile(_)),
        "{refused:?}"
    );
}

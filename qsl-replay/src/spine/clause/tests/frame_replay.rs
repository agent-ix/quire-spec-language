// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-515 (FR-116): `crate::replay_frame` over FR-108's
//! ConfigVersion unit. Each envelope is built by hand for
//! `Config::ConfigVersion::attemptUpdate`, its anchor, frame and occurrence
//! identities taken from the compiled package, with the unit's source, the
//! domain package and the invocation documents in the byte provision.
//! `attemptUpdate`'s frame modifies exactly `versionNumber`.

use super::frame::{identifier, object, snapshot};
use super::*;
use crate::spine::OperationName;
use crate::{
    replay_frame, ClaimedChange, DisagreementCause, FamilyPayload, FrameCounterexample,
    FrameIdentityMismatch, FrameOperation, FrameReplayResult, ProfileSelection, ReplayRefusal,
    ReplayRequestRefusal, ReplayRequestWire, ReplayResult, ReplaySource, StateEnvironment, Verdict,
    Witness, WitnessEnvelope, WitnessPacket, WitnessSettlement,
};
use qsl_foundation::diagnostic::Category;
use qsl_foundation::digest::{DigestDomain, DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::library::PackageId;
use qsl_semantics::model::observation::{AdmissionFailure, FrameChange, SelectedObject};
use quire_exact::Identifier;

/// The labels `source()` gives the unit, as the request's source reference
/// spells them; the recompile displays the unit under its identity.
const AUTHORITY: &str = "agent-ix";
pub(super) const IDENTITY: &str = "clause-run-fixture";

/// A compiled ConfigVersion unit and what a request needs to recompile it.
pub(super) struct Unit {
    pub(super) bytes: Vec<u8>,
    pub(super) domain_document: Vec<u8>,
    pub(super) compiled: ComposedUnit,
}

pub(super) fn unit_for(unit: String, domain_document: Vec<u8>) -> Unit {
    let packages = qsl_semantics::model::intake::package_input([domain_document.as_slice()]);
    let compiled = compose(
        source(),
        IDENTITY,
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .expect("the ConfigVersion unit compiles");
    Unit {
        bytes: unit.into_bytes(),
        domain_document,
        compiled,
    }
}

pub(super) fn config_version_unit() -> Unit {
    let (unit, _) = config_version_unit_and_packages();
    unit_for(unit, config_version_domain_document())
}

#[trace("FR-003-AC-9")]
#[test]
fn private_frame_source_variants_keep_format_identity() {
    let _ = config_version_unit();
    let _ = parent_modifying_unit();
}

/// The ConfigVersion domain package with `attemptUpdate`'s frame also
/// modifying `parent`.
fn parent_modifying_unit() -> Unit {
    let mut document: serde_json::Value =
        serde_json::from_slice(&config_version_domain_document()).expect("the fixture is JSON");
    let modifies = document["types"][1]["operations"][0]["frame"]["modifies"]
        .as_array_mut()
        .expect("attemptUpdate's frame lists modifies");
    modifies.push(json!(format!("{}/parent", config_version_type())));
    let document = document.to_string().into_bytes();
    let (unit, _) = config_version_unit_and_packages_for(document.clone());
    unit_for(unit, document)
}

pub(super) fn wire(node: quire_exact::NodeKey) -> WireNodeId {
    WireNodeId::from_digest(*node.as_bytes())
}

/// `attemptUpdate`'s anchor node, frame node and frame occurrence in
/// `unit`'s compiled package.
fn identities(unit: &Unit) -> (WireNodeId, WireNodeId, OccurrenceKey) {
    let graph = unit.compiled.package.graph();
    let selection = graph
        .resolve_operation(
            &identifier("Config"),
            &identifier("ConfigVersion"),
            &identifier("attemptUpdate"),
        )
        .expect("Config::ConfigVersion resolves");
    let (_, frame) = graph
        .operation_frame(&selection)
        .expect("attemptUpdate is named by VersionUnchanged");
    (
        wire(frame.anchor()),
        wire(frame.frame()),
        OccurrenceKey::new(wire(frame.frame()), frame.frame_origin().clone()),
    )
}

/// The wire node id of the one emitted node whose `semantic_form` is
/// `form`: the emitted package, not the checked graph the replay reads.
fn emitted_node(unit: &Unit, form: &str) -> String {
    let package: serde_json::Value =
        serde_json::from_slice(unit.compiled.emitted.bytes()).expect("the emitted package is JSON");
    let nodes: Vec<&serde_json::Value> = package["semantic_graph"]["nodes"]
        .as_array()
        .expect("the package lists its nodes")
        .iter()
        .filter(|node| node["semantic_form"] == form)
        .collect();
    let [node] = nodes[..] else {
        panic!("one {form} node: {nodes:#?}");
    };
    node["node_id"]["digest"]
        .as_str()
        .expect("a node id digest")
        .to_owned()
}

fn operation(name: &str) -> FrameOperation {
    FrameOperation {
        object: crate::QualifiedName::new(vec![
            Identifier::new("Config").unwrap(),
            Identifier::new("ConfigVersion").unwrap(),
        ])
        .unwrap(),
        operation: Identifier::new(name).unwrap(),
    }
}

fn child_change(field: &str) -> ClaimedChange {
    ClaimedChange::FieldWrite {
        object: SelectedObject {
            population: config_version_population_identity(),
            key: "child".to_owned(),
        },
        field: field.to_owned(),
    }
}

/// One invocation of `attemptUpdate` with self `child`: its three document
/// references and their bytes.
struct Invocation {
    invocation: DocumentRef,
    pre: DocumentRef,
    post: DocumentRef,
    documents: Vec<(DocumentRef, Vec<u8>)>,
}

fn invocation(
    pre_objects: &[serde_json::Value],
    post_objects: &[serde_json::Value],
    edit: impl FnOnce(&mut serde_json::Value),
) -> Invocation {
    let model_digest_hex = config_version_model_digest_hex();
    let (pre, pre_bytes) =
        document_ref_and_bytes("replay-pre", |label| snapshot(label, "pre", pre_objects));
    let (post, post_bytes) =
        document_ref_and_bytes("replay-post", |label| snapshot(label, "post", post_objects));
    let (invocation, invocation_bytes) = document_ref_and_bytes("replay-invocation", |label| {
        let bytes = config_version_invocation_bytes(label, &model_digest_hex, &pre, &post, "child");
        let mut document: serde_json::Value =
            serde_json::from_slice(&bytes).expect("the fixture is JSON");
        edit(&mut document);
        document.to_string().into_bytes()
    });
    Invocation {
        documents: vec![
            (invocation.clone(), invocation_bytes),
            (pre.clone(), pre_bytes),
            (post.clone(), post_bytes),
        ],
        invocation,
        pre,
        post,
    }
}

/// forbidden-parent-change: the post sets `child.parent` absent.
fn forbidden_parent_change() -> Invocation {
    invocation(
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 2, None)],
        |_| {},
    )
}

/// changed-version: `child.versionNumber` moves from 2 to 3, inside the
/// frame.
fn changed_version() -> Invocation {
    invocation(
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 3, Some("root"))],
        |_| {},
    )
}

pub(super) fn source_digest(bytes: &[u8]) -> DigestRecord {
    DigestRecord::mint(
        DigestDomain::SourceBytesV1,
        qsl_foundation::ByteDigest::of(bytes).as_bytes(),
    )
}

pub(super) fn package_digest(package_id: PackageId) -> DigestRecord {
    DigestRecord::mint(DigestDomain::PackageSemanticV2, *package_id.as_bytes())
}

pub(super) fn witness_source() -> ReplaySource {
    ReplaySource::Witness(Witness::parse("<<<assertion|frame_harness|frame|>>>").unwrap())
}

/// FR-098's request for `source_bytes` at `package_id`, with the domain
/// package and `documents` in the byte provision.
pub(super) fn request(
    source_bytes: &[u8],
    domain_document: &[u8],
    package_id: DigestRecord,
    documents: &[(DocumentRef, Vec<u8>)],
) -> ReplayRequestWire {
    let source = source_digest(source_bytes);
    let jcs = qsl_semantics::model::intake::PackageDocument::parse(
        domain_document,
        qsl_foundation::IntakeLimits::default(),
    )
    .expect("the domain package parses")
    .jcs_digest();
    let sha256_jcs = |digest: [u8; 32]| {
        (
            Some(DigestDomain::Sha256Jcs.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::Sha256Jcs, digest).hex(),
        )
    };
    let mut byte_provision = vec![
        (
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            source.hex(),
            source_bytes.to_vec(),
        ),
        {
            let (domain, hex) = sha256_jcs(jcs);
            (domain, hex, domain_document.to_vec())
        },
    ];
    for (reference, bytes) in documents {
        let (domain, hex) = sha256_jcs(reference.digest);
        byte_provision.push((domain, hex, bytes.clone()));
    }
    ReplayRequestWire {
        profile_selections: Vec::new(),
        package_id: (
            Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
            package_id.hex(),
        ),
        source_digests: vec![(
            AUTHORITY.to_owned(),
            IDENTITY.to_owned(),
            "git".to_owned(),
            "1".to_owned(),
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            source.hex(),
        )],
        dependencies: Vec::new(),
        selected_function: crate::QualifiedName::new(vec![
            Identifier::new("attemptUpdate").unwrap()
        ])
        .unwrap(),
        source: witness_source(),
        obligation_identity: [2; 32],
        backend: "kani-backend-1".to_owned(),
        state_environment: StateEnvironment::new(Vec::new()),
        accounting_limits: default_accounting(1_000_000),
        stage_limits: std::collections::BTreeMap::new(),
        declared_domains: Vec::new(),
        byte_provision,
    }
}

/// The envelope carrying `payload` at `package_id`, from `source_bytes`,
/// its `clause_node` and `occurrence_key` the payload's frame node and
/// frame occurrence.
fn envelope(
    source_bytes: &[u8],
    package_id: DigestRecord,
    source: ReplaySource,
    payload: FrameCounterexample,
) -> WitnessEnvelope<FrameCounterexample> {
    envelope_with(source_bytes, package_id, source, payload, |_| {})
}

/// [`envelope`], with `adjust` applied to the packet before it
/// reconstructs.
fn envelope_with(
    source_bytes: &[u8],
    package_id: DigestRecord,
    source: ReplaySource,
    payload: FrameCounterexample,
    adjust: impl FnOnce(&mut WitnessPacket<FrameCounterexample>),
) -> WitnessEnvelope<FrameCounterexample> {
    let (clause_node, occurrence) = (payload.frame, payload.occurrence.clone());
    let mut packet = packet(
        source_bytes,
        package_id,
        source,
        clause_node,
        occurrence,
        payload,
    );
    adjust(&mut packet);
    WitnessEnvelope::reconstruct(packet, crate::ReplayLimits::default())
        .expect("a complete packet reconstructs")
}

/// A complete packet carrying `payload` at `package_id`, from
/// `source_bytes`, with `clause_node` and `occurrence_key` as given.
pub(super) fn packet<P: FamilyPayload>(
    source_bytes: &[u8],
    package_id: DigestRecord,
    source: ReplaySource,
    clause_node: WireNodeId,
    occurrence_key: OccurrenceKey,
    payload: P,
) -> WitnessPacket<P> {
    WitnessPacket {
        obligation_identity: Some([1; 32]),
        occurrence_key: Some(occurrence_key),
        clause_node: Some(clause_node),
        selected_function: Some(
            crate::QualifiedName::new(vec![Identifier::new("attemptUpdate").unwrap()]).unwrap(),
        ),
        package_id: Some((
            Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
            package_id.hex(),
        )),
        source_digests: Some(vec![(
            AUTHORITY.to_owned(),
            IDENTITY.to_owned(),
            "git".to_owned(),
            "1".to_owned(),
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            source_digest(source_bytes).hex(),
        )]),
        profile_selections: Some(vec![ProfileSelection::new(
            "quire.profile.v1".to_owned(),
            "finite-state".to_owned(),
        )]),
        run_limits: Some(default_accounting(1_000_000)),
        declared_domains: Some(Vec::new()),
        backend: Some("kani-backend-1".to_owned()),
        trace_position: Some(None),
        source: Some(source),
        family_payload: Some(payload),
    }
}

/// One replay case over the base unit: `input`'s documents in the
/// provision, `payload` built from the compiled identities with
/// `change`, then `adjust` applied to the request and payload.
struct Case {
    unit: Unit,
    input: Invocation,
    wire: ReplayRequestWire,
    envelope: WitnessEnvelope<FrameCounterexample>,
}

fn case(input: Invocation, change: ClaimedChange) -> Case {
    case_with(input, change, |_, _| {})
}

fn case_with(
    input: Invocation,
    change: ClaimedChange,
    adjust: impl FnOnce(&mut ReplayRequestWire, &mut FrameCounterexample),
) -> Case {
    let unit = config_version_unit();
    let (anchor, frame, occurrence) = identities(&unit);
    let package_id = package_digest(unit.compiled.emitted.package_id());
    let mut payload = FrameCounterexample {
        operation: operation("attemptUpdate"),
        anchor,
        frame,
        occurrence,
        invocation: input.invocation.clone(),
        change,
    };
    let mut wire = request(
        &unit.bytes,
        &unit.domain_document,
        package_id,
        &input.documents,
    );
    adjust(&mut wire, &mut payload);
    let envelope = envelope(&unit.bytes, package_id, witness_source(), payload);
    Case {
        unit,
        input,
        wire,
        envelope,
    }
}

fn replay(case: Case) -> Result<FrameReplayResult, ReplayRefusal> {
    replay_frame(case.wire, &case.envelope, crate::ReplayLimits::default())
}

fn witness_arm(result: &FrameReplayResult) -> &crate::WitnessArmResult {
    match result.result() {
        ReplayResult::Witness(arm) => arm,
        ReplayResult::Input(arm) => panic!("expected the witness arm, got {arm:?}"),
    }
}

fn found_field_write(result: &FrameReplayResult) -> (&str, &str) {
    let found = result.found().expect("the replay found a violation");
    match &found.change {
        FrameChange::FieldWrite { object, field, .. } => (object.as_str(), field.as_str()),
        other => panic!("expected a field write, got {other:?}"),
    }
}

/// TC-515 step 1 (FR-116-AC-1): forbidden-parent-change claiming (`child`,
/// `parent`) reproduces with an evaluated witness, and the result holds the
/// source digest, `package_id`, the payload's anchor, frame and occurrence
/// identities -- which are the emitted package's own `operation_anchor` and
/// `frame` nodes -- the three document identities and digests, and both
/// changes.
#[trace("TC-515", "FR-116-AC-1")]
#[test]
fn a_frame_counterexample_reproduces_and_keeps_its_identities() {
    let case = case(forbidden_parent_change(), child_change("parent"));
    let payload = case.envelope.family_payload().clone();
    let (documents, unit_bytes, package_id) = (
        (
            case.input.invocation.clone(),
            case.input.pre.clone(),
            case.input.post.clone(),
        ),
        case.unit.bytes.clone(),
        case.unit.compiled.emitted.package_id(),
    );
    assert_eq!(
        payload.anchor.to_string(),
        emitted_node(&case.unit, "operation_anchor")
    );
    assert_eq!(payload.frame.to_string(), emitted_node(&case.unit, "frame"));
    let result = replay(case).expect("the replay settles");

    let arm = witness_arm(&result);
    assert_eq!(
        arm.settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(arm.category(), Category::Violation);
    assert_eq!(arm.disagreement(), None);
    assert_eq!(result.source().digest(), source_digest(&unit_bytes));
    assert_eq!(result.source().identity(), IDENTITY);
    assert_eq!(result.package_id(), package_id);
    assert_eq!(result.operation(), &payload.operation);
    assert_eq!(result.anchor(), payload.anchor);
    assert_eq!(result.frame(), payload.frame);
    assert_eq!(result.occurrence(), &payload.occurrence);
    assert_eq!(
        (result.invocation(), result.pre(), result.post()),
        (&documents.0, &documents.1, &documents.2)
    );
    assert_eq!(result.claimed(), &child_change("parent"));
    assert_eq!(found_field_write(&result), ("child", "parent"));
    let found = result.found().unwrap();
    assert_eq!(found.code.as_str(), "frame_violation");
    assert_eq!(found.cause, "unauthorized-change");
}

/// TC-515 step 2 (FR-116-AC-2): changed-version stays inside the frame, so
/// the replay finds nothing: `inconclusive`, `Verdicts`, `violation` proved
/// and `success` replayed, with no found change.
#[trace("TC-515", "FR-116-AC-2")]
#[test]
fn a_frame_respecting_invocation_is_inconclusive_by_verdicts() {
    let result =
        replay(case(changed_version(), child_change("versionNumber"))).expect("the replay settles");
    let arm = witness_arm(&result);
    assert_eq!(arm.settlement(), WitnessSettlement::Inconclusive);
    assert_eq!(
        arm.disagreement(),
        Some(&DisagreementCause::Verdicts {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Success),
        })
    );
    assert!(arm.record().is_none());
    assert!(result.found().is_none());
    assert_eq!(result.claimed(), &child_change("versionNumber"));
}

/// TC-515 step 2 (FR-116-AC-2): forbidden-parent-change claiming (`child`,
/// `versionNumber`) still reproduces, and the result keeps the payload's
/// change beside the replay's (`child`, `parent`), as found.
#[trace("TC-515", "FR-116-AC-2")]
#[test]
fn a_mismatched_claim_reproduces_keeping_both_changes() {
    let result = replay(case(
        forbidden_parent_change(),
        child_change("versionNumber"),
    ))
    .expect("the replay settles");
    assert_eq!(
        witness_arm(&result).settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(result.claimed(), &child_change("versionNumber"));
    assert_eq!(found_field_write(&result), ("child", "parent"));
}

/// TC-515 step 2 (FR-116-AC-2): an invocation whose `created` lists `child`
/// disagrees with the computed empty delta; the check refuses
/// `population_delta_mismatch`, which completes no value: `inconclusive`,
/// `NoValue`.
#[trace("TC-515", "FR-116-AC-2")]
#[test]
fn a_disagreeing_declared_delta_is_inconclusive_with_no_value() {
    let input = invocation(
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 3, Some("root"))],
        |document| {
            document["created"] = json!([{
                "population": config_version_population_identity(), "key": "child",
            }]);
        },
    );
    let result = replay(case(input, child_change("versionNumber"))).expect("the replay settles");
    let arm = witness_arm(&result);
    assert_eq!(arm.settlement(), WitnessSettlement::Inconclusive);
    assert_eq!(
        arm.disagreement(),
        Some(&DisagreementCause::NoValue {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Refusal),
        })
    );
    assert_eq!(arm.value(), None);
    assert!(result.found().is_none());
}

/// TC-515 step 3 (FR-116-AC-3): an envelope whose identities are taken from
/// a package whose `attemptUpdate` frame also modifies `parent` refuses
/// `stale_dependency`/`content-mismatch` naming both frame identities --
/// before admission: the pre snapshot is also missing from the provision,
/// and admission would have refused that first. All three identities come
/// from that package, as a real producer would emit them: its anchor
/// differs too, since the anchor node references the frame node, and the
/// refusal still names the frames.
#[trace("TC-515", "FR-116-AC-3")]
#[test]
fn a_stale_frame_identity_refuses_before_admission() {
    let other = parent_modifying_unit();
    let (other_anchor, other_frame, other_occurrence) = identities(&other);
    let mut recompiled = None;
    let input = forbidden_parent_change();
    let pre = input.pre.clone();
    let case = case_with(input, child_change("parent"), |wire, payload| {
        recompiled = Some((payload.anchor, payload.frame));
        payload.anchor = other_anchor;
        payload.frame = other_frame;
        payload.occurrence = other_occurrence;
        let pre_hex = DigestRecord::mint(DigestDomain::Sha256Jcs, pre.digest).hex();
        wire.byte_provision.retain(|(_, hex, _)| *hex != pre_hex);
    });
    let (recompiled_anchor, recompiled_frame) = recompiled.unwrap();
    assert_ne!(other_frame, recompiled_frame);
    assert_ne!(other_anchor, recompiled_anchor);
    let refusal = replay(case).unwrap_err();
    assert_eq!(refusal.code(), qsl_foundation::Code::StaleDependency);
    assert!(
        matches!(
            &refusal,
            ReplayRefusal::FrameIdentity(mismatch)
                if matches!(**mismatch, FrameIdentityMismatch::Frame { payload, recompiled }
                    if payload == other_frame && recompiled == recompiled_frame)
        ),
        "{refusal:?}"
    );
    assert!(refusal
        .to_string()
        .starts_with("stale_dependency/content-mismatch"));
}

/// FR-116 Behavior: a stale anchor node, or a frame occurrence minted at
/// another ordinal, refuses the same way, each naming both identities.
#[trace("TC-515", "FR-116-AC-3")]
#[test]
fn a_stale_anchor_or_occurrence_refuses_content_mismatch() {
    let stale_anchor = WireNodeId::from_digest([0xAA; 32]);
    let refusal = replay(case_with(
        forbidden_parent_change(),
        child_change("parent"),
        |_, payload| payload.anchor = stale_anchor,
    ))
    .unwrap_err();
    assert!(
        matches!(
            &refusal,
            ReplayRefusal::FrameIdentity(mismatch)
                if matches!(**mismatch, FrameIdentityMismatch::Anchor { payload, .. }
                    if payload == stale_anchor)
        ),
        "{refusal:?}"
    );

    let mut stale_occurrence = None;
    let refusal = replay(case_with(
        forbidden_parent_change(),
        child_change("parent"),
        |_, payload| {
            let origin = payload.occurrence.origin();
            let other = OccurrenceKey::new(
                payload.occurrence.node(),
                quire_exact::Origin::new(origin.role().clone(), origin.ordinal() + 1),
            );
            stale_occurrence = Some(other.clone());
            payload.occurrence = other;
        },
    ))
    .unwrap_err();
    let stale_occurrence = stale_occurrence.unwrap();
    assert!(
        matches!(
            &refusal,
            ReplayRefusal::FrameIdentity(mismatch)
                if matches!(&**mismatch, FrameIdentityMismatch::Occurrence { payload, recompiled }
                    if *payload == stale_occurrence && *recompiled != stale_occurrence)
        ),
        "{refusal:?}"
    );
}

/// A request whose source does not compile, for the ConfigVersion unit's
/// `package_id`, and the step-1 payload built from the compiled unit.
fn uncompilable_request() -> (Unit, ReplayRequestWire, FrameCounterexample) {
    let unit = config_version_unit();
    let input = forbidden_parent_change();
    let (anchor, frame, occurrence) = identities(&unit);
    let payload = FrameCounterexample {
        operation: operation("attemptUpdate"),
        anchor,
        frame,
        occurrence,
        invocation: input.invocation.clone(),
        change: child_change("parent"),
    };
    let package_id = package_digest(unit.compiled.emitted.package_id());
    let broken = b"this is not a QSL unit\n".to_vec();
    let wire = request(&broken, &unit.domain_document, package_id, &input.documents);
    (unit, wire, payload)
}

/// TC-515 step 3 (FR-116-AC-6): an envelope whose `clause_node` is not its
/// payload's frame node refuses `stale_dependency`/`content-mismatch`
/// naming both, before recompiling: the request's source does not compile,
/// and the same request with a consistent envelope refuses at the
/// recompile.
#[trace("TC-515", "FR-116-AC-6")]
#[test]
fn an_envelope_clause_node_other_than_the_frame_refuses_before_recompiling() {
    let (unit, wire, payload) = uncompilable_request();
    let package_id = package_digest(unit.compiled.emitted.package_id());
    let consistent = envelope(&unit.bytes, package_id, witness_source(), payload.clone());
    let refusal = replay_frame(
        uncompilable_request().1,
        &consistent,
        crate::ReplayLimits::default(),
    )
    .unwrap_err();
    assert!(
        matches!(refusal, ReplayRefusal::Recompile(_)),
        "{refusal:?}"
    );

    let (_, other_frame, _) = identities(&parent_modifying_unit());
    assert_ne!(other_frame, payload.frame);
    let frame = payload.frame;
    let stale = envelope_with(
        &unit.bytes,
        package_id,
        witness_source(),
        payload,
        |packet| {
            packet.clause_node = Some(other_frame);
        },
    );
    let refusal = replay_frame(wire, &stale, crate::ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(
            &refusal,
            ReplayRefusal::FrameIdentity(mismatch)
                if matches!(**mismatch, FrameIdentityMismatch::EnvelopeFrame { envelope, payload }
                    if envelope == other_frame && payload == frame)
        ),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), qsl_foundation::Code::StaleDependency);
    let message = refusal.to_string();
    assert!(message.starts_with("stale_dependency/content-mismatch"));
    assert!(message.contains(&other_frame.to_string()), "{message}");
    assert!(message.contains(&frame.to_string()), "{message}");
}

/// TC-515 step 3 (FR-116-AC-6): an envelope whose `occurrence_key` is not
/// its payload's frame occurrence refuses `stale_dependency`/
/// `content-mismatch` naming both, before recompiling.
#[trace("TC-515", "FR-116-AC-6")]
#[test]
fn an_envelope_occurrence_other_than_the_frame_occurrence_refuses_before_recompiling() {
    let (unit, wire, payload) = uncompilable_request();
    let package_id = package_digest(unit.compiled.emitted.package_id());
    let origin = payload.occurrence.origin();
    let other = OccurrenceKey::new(
        payload.occurrence.node(),
        quire_exact::Origin::new(origin.role().clone(), origin.ordinal() + 1),
    );
    let occurrence = payload.occurrence.clone();
    let stale = envelope_with(
        &unit.bytes,
        package_id,
        witness_source(),
        payload,
        |packet| {
            packet.occurrence_key = Some(other.clone());
        },
    );
    let refusal = replay_frame(wire, &stale, crate::ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(
            &refusal,
            ReplayRefusal::FrameIdentity(mismatch)
                if matches!(&**mismatch, FrameIdentityMismatch::EnvelopeOccurrence { envelope, payload }
                    if *envelope == other && *payload == occurrence)
        ),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), qsl_foundation::Code::StaleDependency);
    let message = refusal.to_string();
    assert!(message.starts_with("stale_dependency/content-mismatch"));
    assert!(message.contains(&format!("{other:?}")), "{message}");
    assert!(message.contains(&format!("{occurrence:?}")), "{message}");
}

/// TC-515 step 3 (FR-116-AC-3): a source edit that changes the
/// `package_id` refuses by FR-098's stale `package_id` rule.
#[trace("TC-515", "FR-116-AC-3")]
#[test]
fn a_source_edit_refuses_by_the_stale_package_rule() {
    let input = forbidden_parent_change();
    let unit = config_version_unit();
    let original = package_digest(unit.compiled.emitted.package_id());
    let mut edited = unit.bytes.clone();
    edited.extend_from_slice(
        b"function extra using v(a: Config::ConfigVersion): Integer pure { 2 }\n",
    );
    let (anchor, frame, occurrence) = identities(&unit);
    let payload = FrameCounterexample {
        operation: operation("attemptUpdate"),
        anchor,
        frame,
        occurrence,
        invocation: input.invocation.clone(),
        change: child_change("parent"),
    };
    let wire = request(&edited, &unit.domain_document, original, &input.documents);
    let envelope = envelope(&edited, original, witness_source(), payload);
    let refusal = replay_frame(wire, &envelope, crate::ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(
            &refusal,
            ReplayRefusal::PackageIdMismatch { requested, recompiled }
                if *requested == original && !recompiled.matches(&original)
        ),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), qsl_foundation::Code::StaleDependency);
}

/// FR-116 Behavior: a request whose `package_id` is the recompiled one but
/// an envelope produced from another package refuses by FR-098's stale
/// `package_id` rule, naming the envelope's.
#[trace("TC-515", "FR-116-AC-3")]
#[test]
fn a_stale_envelope_package_id_refuses_by_the_stale_package_rule() {
    let case = case(forbidden_parent_change(), child_change("parent"));
    let other = package_digest(parent_modifying_unit().compiled.emitted.package_id());
    let current = case.unit.compiled.emitted.package_id();
    assert!(!current.matches(&other));
    let payload = case.envelope.family_payload().clone();
    let envelope = envelope(&case.unit.bytes, other, witness_source(), payload);
    let refusal = replay_frame(case.wire, &envelope, crate::ReplayLimits::default()).unwrap_err();
    assert!(
        matches!(
            &refusal,
            ReplayRefusal::PackageIdMismatch { requested, recompiled }
                if *requested == other && *recompiled == current
        ),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), qsl_foundation::Code::StaleDependency);
}

/// TC-515 step 3 (FR-116-AC-3): an `operation` naming `missing` refuses
/// `missing_declaration`/`missing-name`.
#[trace("TC-515", "FR-116-AC-3")]
#[test]
fn a_missing_operation_refuses_missing_name() {
    let refusal = replay(case_with(
        forbidden_parent_change(),
        child_change("parent"),
        |_, payload| payload.operation = operation("missing"),
    ))
    .unwrap_err();
    assert!(
        matches!(
            &refusal,
            ReplayRefusal::UnknownOperation { operation: named, .. }
                if named.to_string() == "Config::ConfigVersion::missing"
        ),
        "{refusal:?}"
    );
    assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
    assert!(refusal
        .to_string()
        .starts_with("missing_declaration/missing-name"));
}

/// TC-515 step 4 (FR-116-AC-4): the pre snapshot absent from the byte
/// provision refuses with FR-106's `unavailable_observation` record.
#[trace("TC-515", "FR-116-AC-4")]
#[test]
fn an_absent_pre_snapshot_refuses_with_the_admission_record() {
    let input = forbidden_parent_change();
    let pre_hex = DigestRecord::mint(DigestDomain::Sha256Jcs, input.pre.digest).hex();
    let refusal = replay(case_with(input, child_change("parent"), |wire, _| {
        wire.byte_provision.retain(|(_, hex, _)| *hex != pre_hex);
    }))
    .unwrap_err();
    let ReplayRefusal::Admission(AdmissionFailure::Incomplete(record)) = &refusal else {
        panic!("expected an incomplete admission, got {refusal:?}");
    };
    assert_eq!(record.code.as_str(), "unavailable_observation");
    assert_eq!(refusal.code(), record.code);
}

/// TC-515 step 4 (FR-116-AC-4): invocation bytes edited under their own
/// digest refuse `stale_dependency`/`content-mismatch`.
#[trace("TC-515", "FR-116-AC-4")]
#[test]
fn edited_invocation_bytes_refuse_content_mismatch() {
    let input = forbidden_parent_change();
    let selected_digest = input.invocation.digest;
    let invocation_hex = DigestRecord::mint(DigestDomain::Sha256Jcs, selected_digest).hex();
    let mut edited_digest = [0_u8; 32];
    let refusal = replay(case_with(input, child_change("parent"), |wire, _| {
        let entry = wire
            .byte_provision
            .iter_mut()
            .find(|(_, hex, _)| *hex == invocation_hex)
            .expect("the invocation is provided");
        entry.2 = String::from_utf8(entry.2.clone())
            .expect("UTF-8")
            .replace("\"boolean\":true", "\"boolean\":false")
            .into_bytes();
        edited_digest = document_digest(&entry.2);
    }))
    .unwrap_err();
    let ReplayRefusal::Request(ReplayRequestRefusal::ContentMismatch {
        selected,
        recomputed,
    }) = &refusal
    else {
        panic!("expected a content mismatch, got {refusal:?}");
    };
    assert_eq!(
        selected,
        &format!(
            "{:?}",
            DigestRecord::mint(DigestDomain::Sha256Jcs, selected_digest)
        )
    );
    assert_eq!(
        recomputed,
        &format!(
            "{:?}",
            DigestRecord::mint(DigestDomain::Sha256Jcs, edited_digest)
        )
    );
    assert_eq!(refusal.code(), qsl_foundation::Code::StaleDependency);
    assert!(refusal
        .to_string()
        .starts_with("stale_dependency/content-mismatch"));
}

/// TC-515 step 5 (FR-116-AC-5): replaying step 1's envelope twice gives
/// equal results. `FrameCounterexample: FamilyPayload`, and the envelope
/// carries it as its generic parameter.
#[trace("TC-515", "FR-116-AC-5")]
#[test]
fn replaying_one_envelope_twice_gives_equal_results() {
    fn family_payload<P: FamilyPayload>(envelope: &WitnessEnvelope<P>) -> &P {
        envelope.family_payload()
    }
    let first = case(forbidden_parent_change(), child_change("parent"));
    assert_eq!(
        family_payload(&first.envelope).change,
        child_change("parent")
    );
    let second = case(forbidden_parent_change(), child_change("parent"));
    assert_eq!(replay(first).unwrap(), replay(second).unwrap());
}

/// FR-116 Outputs ("on the envelope's arm"): an `Input`-arm envelope
/// settles the `Input` arm, reproduced without a witness.
#[trace("TC-515", "FR-116-AC-1")]
#[test]
fn an_input_arm_envelope_settles_the_input_arm() {
    let case = case(forbidden_parent_change(), child_change("parent"));
    let payload = case.envelope.family_payload().clone();
    let package_id = package_digest(case.unit.compiled.emitted.package_id());
    let envelope = envelope(
        &case.unit.bytes,
        package_id,
        ReplaySource::Input(Vec::new()),
        payload,
    );
    let result = replay_frame(case.wire, &envelope, crate::ReplayLimits::default())
        .expect("the replay settles");
    let ReplayResult::Input(arm) = result.result() else {
        panic!("expected the input arm, got {:?}", result.result());
    };
    assert_eq!(
        arm.settlement(),
        crate::InputSettlement::ReproducedWithoutWitness
    );
}

/// FR-116 with FR-115: the violation `run_clause`'s `Frame` selection
/// reports for forbidden-parent-change, decoded into a payload
/// ([`ClaimedChange::of_witness`]), replays to the same verdict and the
/// same frame witness.
#[trace("TC-515", "FR-116-AC-1")]
#[test]
fn a_check_time_frame_violation_replays_to_the_same_witness() {
    let input = forbidden_parent_change();
    let mut clause_request = config_version_request(ClauseRunSelection::Frame {
        operation: OperationName {
            model: identifier("Config"),
            object: identifier("ConfigVersion"),
            operation: identifier("attemptUpdate"),
        },
        invocation: input.invocation.clone(),
    });
    for (reference, bytes) in &input.documents {
        clause_request
            .snapshots
            .insert(reference.digest, bytes.clone());
        clause_request
            .invocations
            .insert(reference.digest, bytes.clone());
    }
    let report = run_clause(clause_request).expect("a well-formed request reports");
    let ClauseDisposition::FrameViolation(witness) = &report.disposition else {
        panic!("expected FrameViolation, got {:?}", report.disposition);
    };
    let claimed = ClaimedChange::of_witness(witness);
    // The decode names the object by the witness's population and key.
    assert_eq!(claimed, child_change("parent"));

    let result = replay(case(input, claimed.clone())).expect("the replay settles");
    assert_eq!(
        witness_arm(&result).settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(result.claimed(), &claimed);
    assert_eq!(result.found(), Some(witness.as_ref()));
}

/// An invocation taking `child.versionNumber` from 0 to -1, outside
/// `Int[0, 1000]`, with the frame respected.
fn wrapping_debit() -> Invocation {
    invocation(
        &[object("root", 1, None), object("child", 0, Some("root"))],
        &[object("root", 1, None), object("child", -1, Some("root"))],
        |_| {},
    )
}

/// `request` with every document of `input` provided.
fn provide(mut request: ClauseRunRequest, input: &Invocation) -> ClauseRunRequest {
    for (reference, bytes) in &input.documents {
        request.snapshots.insert(reference.digest, bytes.clone());
        request.invocations.insert(reference.digest, bytes.clone());
    }
    request
}

fn assert_refused_invalid_value(disposition: &ClauseDisposition) {
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = disposition else {
        panic!("expected a refused admission, got {disposition:?}");
    };
    assert_eq!(record.code.as_str(), "invalid_runtime_input");
    assert_eq!(record.cause, "invalid-value");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("child")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("versionNumber")
    );
}

/// FR-106-AC-13: a `Frame` run admits its post snapshot as an input, so the
/// wrapping debit still refuses `invalid_runtime_input`/`invalid-value`.
#[trace("TC-465", "FR-106-AC-13")]
#[test]
fn a_frame_run_still_refuses_an_out_of_range_post_state_value() {
    let input = wrapping_debit();
    let request = provide(
        config_version_request(ClauseRunSelection::Frame {
            operation: OperationName {
                model: identifier("Config"),
                object: identifier("ConfigVersion"),
                operation: identifier("attemptUpdate"),
            },
            invocation: input.invocation.clone(),
        }),
        &input,
    );
    let report = run_clause(request).expect("a well-formed request reports");
    assert_refused_invalid_value(&report.disposition);
}

/// FR-106-AC-13: `run_clause`'s `Clause` selection runs under the caller's
/// `ObservationLimits`, whose default refuses the wrapping debit's post
/// snapshot.
#[trace("TC-465", "FR-106-AC-13")]
#[test]
fn run_clause_still_refuses_an_out_of_range_post_state_value() {
    let input = wrapping_debit();
    let request = provide(
        config_version_request(ClauseRunSelection::Clause(ClauseSelection {
            name: "VersionUnchanged".to_owned(),
            input: ClauseSelectionInput::Invocation {
                invocation: input.invocation.clone(),
            },
        })),
        &input,
    );
    let report = run_clause(request).expect("a well-formed request reports");
    assert_refused_invalid_value(&report.disposition);
}

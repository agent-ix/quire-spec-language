// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-517 (FR-122): `crate::replay_state_clause` over FR-108's ConfigVersion
//! unit, and over TC-466 step 3's `probe` unit for the precondition. Each
//! envelope is built by hand, its clause node and `claim` occurrence taken
//! from the compiled package, with the unit's source, the domain package and
//! the observation documents in the byte provision.

use super::frame::object;
use super::frame_replay::{
    config_version_unit, package_digest, packet, request, source_digest, unit_for, wire,
    witness_source, Unit, IDENTITY,
};
use super::*;
use crate::{
    replay_state_clause, ClauseIdentityMismatch, DisagreementCause, EvaluatedValue, FamilyPayload,
    InputSettlement, ObservationForm, ProofCategory, ReplayRefusal, ReplayRequestRefusal,
    ReplayRequestWire, ReplayResult, ReplaySource, SnapshotValue, StateClauseCounterexample,
    StateClauseKind, StateClauseReplayResult, Verdict, WitnessEnvelope, WitnessSettlement,
};
use qsl_foundation::digest::{DigestDomain, DigestRecord, WireNodeId};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::Identifier;

fn name(text: &str) -> Identifier {
    Identifier::new(text).expect("an identifier")
}

/// `clause`'s node and `claim` occurrence in `unit`'s compiled package.
fn clause_identity(unit: &Unit, clause: &str) -> (WireNodeId, OccurrenceKey) {
    let clause = unit
        .compiled
        .package
        .graph()
        .state_clause(clause)
        .expect("the clause is declared");
    let node = wire(clause.identity());
    (node, OccurrenceKey::new(node, clause.claim().clone()))
}

/// `occurrence` at the next ordinal of its role.
fn next_ordinal(occurrence: &OccurrenceKey) -> OccurrenceKey {
    let origin = occurrence.origin();
    OccurrenceKey::new(
        occurrence.node(),
        quire_exact::Origin::new(origin.role().clone(), origin.ordinal() + 1),
    )
}

/// An observation's documents: what the payload names and the byte
/// provision holds.
struct Documents {
    observation: ClauseSelectionInput,
    /// The documents admission reads, in read order.
    read: Vec<DocumentRef>,
    bytes: Vec<(DocumentRef, Vec<u8>)>,
}

/// An `attemptUpdate` invocation with self `child` over `pre_objects` and
/// `post_objects`.
fn invocation(pre_objects: &[serde_json::Value], post_objects: &[serde_json::Value]) -> Documents {
    let (pre, pre_bytes) = document_ref_and_bytes("clause-pre", |label| {
        super::frame::snapshot(label, "pre", pre_objects)
    });
    let (post, post_bytes) = document_ref_and_bytes("clause-post", |label| {
        super::frame::snapshot(label, "post", post_objects)
    });
    let (invocation, invocation_bytes) = document_ref_and_bytes("clause-invocation", |label| {
        config_version_invocation_bytes(
            label,
            &config_version_model_digest_hex(),
            &pre,
            &post,
            "child",
        )
    });
    Documents {
        observation: ClauseSelectionInput::Invocation {
            invocation: invocation.clone(),
        },
        read: vec![invocation.clone(), pre.clone(), post.clone()],
        bytes: vec![
            (invocation, invocation_bytes),
            (pre, pre_bytes),
            (post, post_bytes),
        ],
    }
}

/// changed-version: `child.versionNumber` 2 -> 3, inside the frame.
fn changed_version() -> Documents {
    invocation(
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 3, Some("root"))],
    )
}

/// unchanged-version: `child.versionNumber` stays 2.
fn unchanged_version() -> Documents {
    invocation(
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 2, Some("root"))],
    )
}

/// forbidden-parent-change: the post sets `child.parent` absent, outside
/// `attemptUpdate`'s frame (`modifies [versionNumber]`).
fn forbidden_parent_change() -> Documents {
    invocation(
        &[object("root", 1, None), object("child", 2, Some("root"))],
        &[object("root", 1, None), object("child", 2, None)],
    )
}

/// A current snapshot, anchor `handler validate`, self `child`, holding
/// `root` at `root_version` and `child` at `child_version` naming `root`,
/// with `config_history` marked `complete`.
fn current(identity: &str, root_version: i64, child_version: i64, complete: bool) -> Documents {
    let (snapshot, bytes) = document_ref_and_bytes(identity, |label| {
        let bytes = config_version_snapshot(
            label,
            &config_version_model_digest_hex(),
            root_version,
            Some(child_version),
        );
        let mut document: serde_json::Value =
            serde_json::from_slice(&bytes).expect("the fixture is JSON");
        document["populations"][0]["complete"] = json!(complete);
        document.to_string().into_bytes()
    });
    let ClauseSelectionInput::Current {
        anchor,
        self_object,
        ..
    } = config_version_current_selection("ParentOrder", snapshot.clone(), "child").input
    else {
        unreachable!("a current selection")
    };
    Documents {
        observation: ClauseSelectionInput::Current {
            snapshot: snapshot.clone(),
            anchor,
            self_object,
        },
        read: vec![snapshot.clone()],
        bytes: vec![(snapshot, bytes)],
    }
}

/// healthy-parent: `root` 1, `child` 2 -- `ParentOrder` holds for `child`.
fn healthy_parent() -> Documents {
    current("healthy-parent", 1, 2, true)
}

/// violating-parent: `root` 3, `child` 2 -- `ParentOrder` fails for
/// `child`.
fn violating_parent() -> Documents {
    current("violating-parent", 3, 2, true)
}

/// incomplete-population: healthy-parent with `config_history` marked
/// `complete: false`.
fn incomplete_population() -> Documents {
    current("incomplete-population", 1, 2, false)
}

/// TC-466 step 3's `probe` unit: one precondition, `ReachesTarget`.
fn probe_unit() -> Unit {
    let (unit, _) = config_version_step3_unit_and_packages();
    unit_for(unit, config_version_step3_domain_document())
}

/// The chain `a -> b -> c` as `probe`'s pre state, with self `a` and
/// `target` naming `target`.
fn pre_call(target: &str) -> Documents {
    let (snapshot, bytes) = document_ref_and_bytes("chain-pre", |label| {
        config_version_step3_snapshot(
            label,
            &config_version_step3_model_digest_hex(),
            "pre",
            &config_version_step3_chain_objects(),
        )
    });
    Documents {
        observation: ClauseSelectionInput::PreCall {
            snapshot: snapshot.clone(),
            self_object: self_a(),
            parameters: BTreeMap::from([(name("target"), reference(target))]),
        },
        read: vec![snapshot.clone()],
        bytes: vec![(snapshot, bytes)],
    }
}

fn self_a() -> SelectedObject {
    SelectedObject {
        population: config_version_population_identity(),
        key: "a".to_owned(),
    }
}

fn reference(key: &str) -> SnapshotValue {
    SnapshotValue::Reference(SelectedObject {
        population: config_version_population_identity(),
        key: key.to_owned(),
    })
}

/// One replay case: the request over `unit` with `documents` in the byte
/// provision, and the envelope for `clause` over `documents`' observation,
/// its clause node and occurrence key the compiled clause's.
struct Case {
    unit: Unit,
    documents: Documents,
    wire: ReplayRequestWire,
    envelope: WitnessEnvelope<StateClauseCounterexample>,
}

fn case(unit: Unit, clause: &str, documents: Documents) -> Case {
    case_with(unit, clause, documents, witness_source(), |_, _| {})
}

/// [`case`] on `source`'s arm, with `adjust` applied to the request and to
/// the envelope's clause node and occurrence key.
fn case_with(
    unit: Unit,
    clause: &str,
    documents: Documents,
    source: ReplaySource,
    adjust: impl FnOnce(&mut ReplayRequestWire, &mut (WireNodeId, OccurrenceKey)),
) -> Case {
    let package_id = package_digest(unit.compiled.emitted.package_id());
    let mut identity = clause_identity(&unit, clause);
    let mut wire = request(
        &unit.bytes,
        &unit.domain_document,
        package_id,
        &documents.bytes,
    );
    adjust(&mut wire, &mut identity);
    let payload = StateClauseCounterexample {
        clause: name(clause),
        observation: documents.observation.clone(),
    };
    let (clause_node, occurrence_key) = identity;
    let envelope = WitnessEnvelope::reconstruct(packet(
        &unit.bytes,
        package_id,
        source,
        clause_node,
        occurrence_key,
        payload,
    ))
    .expect("a complete packet reconstructs");
    Case {
        unit,
        documents,
        wire,
        envelope,
    }
}

fn replay(case: Case) -> Result<StateClauseReplayResult, ReplayRefusal> {
    replay_state_clause(case.wire, &case.envelope)
}

fn witness_arm(result: &StateClauseReplayResult) -> &crate::WitnessArmResult {
    match result.result() {
        ReplayResult::Witness(arm) => arm,
        ReplayResult::Input(arm) => panic!("expected the witness arm, got {arm:?}"),
    }
}

/// Removes `reference`'s bytes from the request's byte provision.
fn without(wire: &mut ReplayRequestWire, reference: &DocumentRef) {
    let hex = DigestRecord::mint(DigestDomain::Sha256Jcs, reference.digest).hex();
    wire.byte_provision.retain(|(_, entry, _)| *entry != hex);
}

/// Replays `case` and checks it reproduced with an evaluated witness, whose
/// FR-351 record decides by `false` at index 0, with an empty value path and
/// no trace position, and that the result keeps the source digest, the
/// `package_id`, the clause, the envelope's identities and every document
/// admission read.
fn assert_reproduces_keeping_identities(case: Case, clause: &str) -> StateClauseReplayResult {
    let unit_bytes = case.unit.bytes.clone();
    let package_id = case.unit.compiled.emitted.package_id();
    let (node, occurrence) = (
        case.envelope.clause_node(),
        case.envelope.occurrence_key().clone(),
    );
    assert_eq!(
        (node, occurrence.clone()),
        clause_identity(&case.unit, clause)
    );
    let read = case.documents.read.clone();
    let result = replay(case).expect("the replay settles");
    let arm = witness_arm(&result);
    assert_eq!(
        arm.settlement(),
        WitnessSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(arm.category(), ProofCategory::Violation);
    assert_eq!(arm.value(), Some(EvaluatedValue::Boolean(false)));
    let record = arm.record().expect("a reproduced witness has its record");
    assert_eq!(record.deciding_element, EvaluatedValue::Boolean(false));
    assert_eq!(record.index, 0);
    assert!(record.value_path.is_empty());
    assert_eq!(record.trace_position, None);
    assert_eq!(result.source().digest(), source_digest(&unit_bytes));
    assert_eq!(result.source().identity(), IDENTITY);
    assert_eq!(result.package_id(), package_id);
    assert_eq!(result.clause().as_str(), clause);
    assert_eq!(result.clause_node(), node);
    assert_eq!(result.occurrence_key(), &occurrence);
    assert_eq!(result.documents(), &read[..]);
    result
}

/// TC-517 step 1 (FR-122-AC-1): `VersionUnchanged` over changed-version on
/// a `Witness`-arm envelope reproduces with an evaluated witness, keeping
/// the invocation and both snapshots.
#[trace("TC-517", "FR-122-AC-1")]
#[test]
fn a_violated_postcondition_reproduces_keeping_its_identities() {
    let result = assert_reproduces_keeping_identities(
        case(config_version_unit(), "VersionUnchanged", changed_version()),
        "VersionUnchanged",
    );
    assert_eq!(result.documents().len(), 3);
}

/// TC-517 step 1 (FR-122-AC-1): the same payload on an `Input`-arm envelope
/// reproduces without a witness, holding `false` and no FR-351 record (the
/// `Input` arm has no record member).
#[trace("TC-517", "FR-122-AC-1")]
#[test]
fn an_input_arm_envelope_reproduces_without_a_witness() {
    let documents = changed_version();
    let read = documents.read.clone();
    let result = replay(case_with(
        config_version_unit(),
        "VersionUnchanged",
        documents,
        ReplaySource::Input(Vec::new()),
        |_, _| {},
    ))
    .expect("the replay settles");
    let ReplayResult::Input(arm) = result.result() else {
        panic!("expected the input arm, got {:?}", result.result());
    };
    assert_eq!(arm.settlement(), InputSettlement::ReproducedWithoutWitness);
    assert_eq!(arm.value(), Some(EvaluatedValue::Boolean(false)));
    assert_eq!(result.documents(), &read[..]);
}

/// TC-517 step 1 (FR-122-AC-1): `ParentOrder` over violating-parent's
/// current snapshot (anchor `handler validate`, self `child`) reproduces,
/// keeping the one snapshot.
#[trace("TC-517", "FR-122-AC-1")]
#[test]
fn a_violated_invariant_reproduces_keeping_its_snapshot() {
    let result = assert_reproduces_keeping_identities(
        case(config_version_unit(), "ParentOrder", violating_parent()),
        "ParentOrder",
    );
    assert_eq!(result.documents().len(), 1);
}

/// `result`'s witness arm settled `inconclusive`, `Verdicts`, `violation`
/// proved and `success` replayed, with the evaluated `true` and no FR-351
/// record.
fn assert_inconclusive_by_verdicts(result: &StateClauseReplayResult) {
    let arm = witness_arm(result);
    assert_eq!(arm.settlement(), WitnessSettlement::Inconclusive);
    assert_eq!(
        arm.disagreement(),
        Some(DisagreementCause::Verdicts {
            proved: Verdict::from_category(ProofCategory::Violation),
            replayed: Verdict::from_category(ProofCategory::Success),
        })
    );
    assert_eq!(arm.value(), Some(EvaluatedValue::Boolean(true)));
    assert!(
        arm.record().is_none(),
        "a disagreement carries no decisive record"
    );
}

/// TC-517 step 2 (FR-122-AC-2): `VersionUnchanged` over unchanged-version
/// holds: `inconclusive`, `Verdicts`.
#[trace("TC-517", "FR-122-AC-2")]
#[test]
fn a_holding_postcondition_is_inconclusive_by_verdicts() {
    let result = replay(case(
        config_version_unit(),
        "VersionUnchanged",
        unchanged_version(),
    ))
    .expect("the replay settles");
    assert_inconclusive_by_verdicts(&result);
}

/// TC-517 step 2 (FR-122-AC-2): `ParentOrder` over healthy-parent holds:
/// `inconclusive`, `Verdicts`.
#[trace("TC-517", "FR-122-AC-2")]
#[test]
fn a_holding_invariant_is_inconclusive_by_verdicts() {
    let result = replay(case(config_version_unit(), "ParentOrder", healthy_parent()))
        .expect("the replay settles");
    assert_inconclusive_by_verdicts(&result);
}

/// TC-517 step 2 (FR-122-AC-2): `ParentOrder` over violating-parent with the
/// request's evaluation budget at zero completes no value: `inconclusive`,
/// `NoValue`.
#[trace("TC-517", "FR-122-AC-2")]
#[test]
fn an_exhausted_evaluation_budget_is_inconclusive_with_no_value() {
    let result = replay(case_with(
        config_version_unit(),
        "ParentOrder",
        violating_parent(),
        witness_source(),
        |wire, _| wire.accounting_limits = default_accounting(0),
    ))
    .expect("the replay settles");
    let arm = witness_arm(&result);
    assert_eq!(arm.settlement(), WitnessSettlement::Inconclusive);
    assert_eq!(
        arm.disagreement(),
        Some(DisagreementCause::NoValue {
            proved: Verdict::from_category(ProofCategory::Violation),
            replayed: Verdict::from_category(ProofCategory::Incomplete),
        })
    );
    assert_eq!(arm.value(), None);
    assert!(arm.record().is_none());
}

/// The refusal a stale identity gives, over changed-version with `adjust`
/// applied to the envelope's identities; with its invocation removed from
/// the provision when `drop_invocation`.
fn stale_identity(
    drop_invocation: bool,
    adjust: impl FnOnce(&mut (WireNodeId, OccurrenceKey)),
) -> ReplayRefusal {
    let documents = changed_version();
    let invocation = documents.read[0].clone();
    replay(case_with(
        config_version_unit(),
        "VersionUnchanged",
        documents,
        witness_source(),
        |wire, identity| {
            adjust(identity);
            if drop_invocation {
                without(wire, &invocation);
            }
        },
    ))
    .unwrap_err()
}

fn assert_revision_mismatch(refusal: &ReplayRefusal) {
    assert_eq!(refusal.code(), qsl_foundation::Code::StaleDependency);
    assert!(
        refusal
            .to_string()
            .starts_with("stale_dependency/revision-mismatch"),
        "{refusal}"
    );
}

/// TC-517 step 3 (FR-122-AC-3): an envelope whose `clause_node` is
/// `ParentOrder`'s node refuses `stale_dependency`/`revision-mismatch`
/// naming both nodes, and so does one whose occurrence key is also at
/// ordinal 1 (the node goes first); each again with the invocation absent
/// from the provision, so no document was admitted.
#[trace("TC-517", "FR-122-AC-3")]
#[test]
fn a_stale_clause_node_refuses_naming_both_nodes_before_admission() {
    let unit = config_version_unit();
    let (parent_order, _) = clause_identity(&unit, "ParentOrder");
    let (version_unchanged, _) = clause_identity(&unit, "VersionUnchanged");
    assert_ne!(parent_order, version_unchanged);
    for drop_invocation in [false, true] {
        for bump_occurrence in [false, true] {
            let refusal = stale_identity(drop_invocation, |(node, occurrence)| {
                *node = parent_order;
                if bump_occurrence {
                    *occurrence = next_ordinal(occurrence);
                }
            });
            assert!(
                matches!(
                    &refusal,
                    ReplayRefusal::ClauseIdentity(mismatch)
                        if **mismatch == ClauseIdentityMismatch::Node {
                            envelope: parent_order,
                            recompiled: version_unchanged,
                        }
                ),
                "{refusal:?}"
            );
            assert_revision_mismatch(&refusal);
        }
    }
}

/// TC-517 step 3 (FR-122-AC-3): an envelope whose occurrence key alone is
/// at ordinal 1 refuses naming both occurrences, with and without the
/// invocation in the provision.
#[trace("TC-517", "FR-122-AC-3")]
#[test]
fn a_stale_occurrence_refuses_naming_both_occurrences_before_admission() {
    let (_, recompiled) = clause_identity(&config_version_unit(), "VersionUnchanged");
    let stale = next_ordinal(&recompiled);
    for drop_invocation in [false, true] {
        let refusal = stale_identity(drop_invocation, |(_, occurrence)| {
            *occurrence = next_ordinal(occurrence);
        });
        assert!(
            matches!(
                &refusal,
                ReplayRefusal::ClauseIdentity(mismatch)
                    if **mismatch == ClauseIdentityMismatch::Occurrence {
                        envelope: stale.clone(),
                        recompiled: recompiled.clone(),
                    }
            ),
            "{refusal:?}"
        );
        assert_revision_mismatch(&refusal);
    }
}

/// TC-517 step 3 (FR-122-AC-3): a source edit that changes the `package_id`
/// refuses by FR-098's stale `package_id` rule.
#[trace("TC-517", "FR-122-AC-3")]
#[test]
fn a_source_edit_refuses_by_the_stale_package_rule() {
    let unit = config_version_unit();
    let original = package_digest(unit.compiled.emitted.package_id());
    let (clause_node, occurrence_key) = clause_identity(&unit, "VersionUnchanged");
    let mut edited = unit.bytes.clone();
    edited.extend_from_slice(
        b"function extra using v(a: Config::ConfigVersion): Integer pure { 2 }\n",
    );
    let documents = changed_version();
    let wire = request(&edited, &unit.domain_document, original, &documents.bytes);
    let envelope = WitnessEnvelope::reconstruct(packet(
        &edited,
        original,
        witness_source(),
        clause_node,
        occurrence_key,
        StateClauseCounterexample {
            clause: name("VersionUnchanged"),
            observation: documents.observation,
        },
    ))
    .unwrap();
    let refusal = replay_state_clause(wire, &envelope).unwrap_err();
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

/// TC-517 step 3 (FR-122-AC-3): a `clause` naming `Absent`, and one naming
/// the function `sameIdentity`, each refuse `missing_declaration`/
/// `missing-name`.
#[trace("TC-517", "FR-122-AC-3")]
#[test]
fn a_clause_naming_no_state_clause_refuses_missing_name() {
    for missing in ["Absent", "sameIdentity"] {
        let mut case = case(config_version_unit(), "VersionUnchanged", changed_version());
        let payload = StateClauseCounterexample {
            clause: name(missing),
            observation: case.documents.observation.clone(),
        };
        let package_id = package_digest(case.unit.compiled.emitted.package_id());
        case.envelope = WitnessEnvelope::reconstruct(packet(
            &case.unit.bytes,
            package_id,
            witness_source(),
            case.envelope.clause_node(),
            case.envelope.occurrence_key().clone(),
            payload,
        ))
        .unwrap();
        let refusal = replay(case).unwrap_err();
        assert!(
            matches!(&refusal, ReplayRefusal::UnknownClause { clause, .. }
                if clause.as_str() == missing),
            "{refusal:?}"
        );
        assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
        assert!(refusal
            .to_string()
            .starts_with("missing_declaration/missing-name"));
    }
}

/// TC-517 step 4 (FR-122-AC-4): `ReachesTarget` over the chain's pre state
/// with `self` `a` and `target` `a` (`a` does not reach itself) reproduces,
/// holding the one pre snapshot, no post snapshot, and `self` `a` and
/// `target` `a` as the values it reproduced.
#[trace("TC-517", "FR-122-AC-4")]
#[test]
fn a_violated_precondition_reproduces_over_its_pre_call_state() {
    let result = assert_reproduces_keeping_identities(
        case(probe_unit(), "ReachesTarget", pre_call("a")),
        "ReachesTarget",
    );
    let [snapshot] = result.documents() else {
        panic!("one document: {:?}", result.documents());
    };
    assert_eq!(snapshot.identity, "chain-pre");
    let ClauseSelectionInput::PreCall {
        self_object,
        parameters,
        ..
    } = result.observation()
    else {
        panic!("a pre-call observation: {:?}", result.observation());
    };
    assert_eq!(self_object, &self_a());
    assert_eq!(
        parameters,
        &BTreeMap::from([(name("target"), reference("a"))])
    );
}

/// TC-517 step 4 (FR-122-AC-4): with `target` `c` the precondition holds:
/// `inconclusive`, `Verdicts`.
#[trace("TC-517", "FR-122-AC-4")]
#[test]
fn a_holding_precondition_is_inconclusive_by_verdicts() {
    let result =
        replay(case(probe_unit(), "ReachesTarget", pre_call("c"))).expect("the replay settles");
    assert_inconclusive_by_verdicts(&result);
}

/// Replays `clause` of `unit` over `documents`, once with its documents in
/// the provision and once without, and checks both refuse
/// `wrong_snapshot`/`wrong-observation` naming `kind` and `form`.
fn assert_wrong_observation(
    unit: fn() -> Unit,
    clause: &str,
    documents: fn() -> Documents,
    kind: StateClauseKind,
    form: ObservationForm,
) {
    for drop_documents in [false, true] {
        let supplied = documents();
        let read = supplied.read.clone();
        let refusal = replay(case_with(
            unit(),
            clause,
            supplied,
            witness_source(),
            |wire, _| {
                if drop_documents {
                    for reference in &read {
                        without(wire, reference);
                    }
                }
            },
        ))
        .unwrap_err();
        assert!(
            matches!(&refusal, ReplayRefusal::WrongObservation { kind: k, form: f }
                if *k == kind && *f == form),
            "{refusal:?}"
        );
        assert_eq!(refusal.code(), qsl_foundation::Code::WrongSnapshot);
        assert!(refusal
            .to_string()
            .starts_with("wrong_snapshot/wrong-observation"));
    }
}

/// The `probe` invocation over the chain, self `a`, `target` `c`.
fn probe_invocation() -> Documents {
    let documents = probe_documents(&config_version_step3_chain_objects(), "a", "c");
    let mut bytes: Vec<(DocumentRef, Vec<u8>)> = Vec::new();
    let mut read = vec![documents.invocation.clone()];
    for (digest, document) in documents.invocations.iter().chain(&documents.snapshots) {
        let reference = DocumentRef {
            digest: *digest,
            ..documents.invocation.clone()
        };
        if *digest != documents.invocation.digest {
            read.push(reference.clone());
        }
        bytes.push((reference, document.clone()));
    }
    Documents {
        observation: ClauseSelectionInput::Invocation {
            invocation: documents.invocation,
        },
        read,
        bytes,
    }
}

/// A pre-call observation for `VersionUnchanged` over changed-version's
/// pre snapshot, self `child`.
fn version_unchanged_pre_call() -> Documents {
    let changed = changed_version();
    let (pre, pre_bytes) = changed.bytes[1].clone();
    Documents {
        observation: ClauseSelectionInput::PreCall {
            snapshot: pre.clone(),
            self_object: SelectedObject {
                population: config_version_population_identity(),
                key: "child".to_owned(),
            },
            parameters: BTreeMap::new(),
        },
        read: vec![pre.clone()],
        bytes: vec![(pre, pre_bytes)],
    }
}

/// TC-517 step 4 (FR-122-AC-4): an observation form that is not the
/// clause kind's refuses `wrong_snapshot`/`wrong-observation` before any
/// admission: `ReachesTarget` with an `Invocation`, `VersionUnchanged` with
/// a `PreCall`, and `VersionUnchanged` with a `Current` over healthy-parent
/// -- each the same with its documents absent from the provision.
#[trace("TC-517", "FR-122-AC-4")]
#[test]
fn an_observation_form_other_than_the_clause_kinds_refuses_before_admission() {
    assert_wrong_observation(
        probe_unit,
        "ReachesTarget",
        probe_invocation,
        StateClauseKind::Precondition,
        ObservationForm::Invocation,
    );
    assert_wrong_observation(
        config_version_unit,
        "VersionUnchanged",
        version_unchanged_pre_call,
        StateClauseKind::Postcondition,
        ObservationForm::PreCall,
    );
    assert_wrong_observation(
        config_version_unit,
        "VersionUnchanged",
        healthy_parent,
        StateClauseKind::Postcondition,
        ObservationForm::Current,
    );
}

/// TC-517 step 5 (FR-122-AC-5): `VersionUnchanged` over
/// forbidden-parent-change refuses with FR-106's `frame_violation`/
/// `unauthorized-change` record naming `child` and `parent`.
#[trace("TC-517", "FR-122-AC-5")]
#[test]
fn a_frame_violating_invocation_refuses_with_the_admission_record() {
    let refusal = replay(case(
        config_version_unit(),
        "VersionUnchanged",
        forbidden_parent_change(),
    ))
    .unwrap_err();
    let ReplayRefusal::Admission(AdmissionFailure::Refused(record)) = &refusal else {
        panic!("expected a refused admission, got {refusal:?}");
    };
    assert_eq!(record.code.as_str(), "frame_violation");
    assert_eq!(record.cause, "unauthorized-change");
    assert_eq!(
        record.fields.get("object").map(String::as_str),
        Some("child")
    );
    assert_eq!(
        record.fields.get("field").map(String::as_str),
        Some("parent")
    );
}

/// TC-517 step 5 (FR-122-AC-5): changed-version with its pre snapshot
/// absent from the provision refuses with FR-106's
/// `unavailable_observation` record.
#[trace("TC-517", "FR-122-AC-5")]
#[test]
fn an_absent_pre_snapshot_refuses_with_the_admission_record() {
    let documents = changed_version();
    let pre = documents.read[1].clone();
    let refusal = replay(case_with(
        config_version_unit(),
        "VersionUnchanged",
        documents,
        witness_source(),
        |wire, _| without(wire, &pre),
    ))
    .unwrap_err();
    let ReplayRefusal::Admission(AdmissionFailure::Incomplete(record)) = &refusal else {
        panic!("expected an incomplete admission, got {refusal:?}");
    };
    assert_eq!(record.code.as_str(), "unavailable_observation");
    assert_eq!(record.cause, "missing-required-artifact");
    assert_eq!(refusal.code(), record.code);
}

/// TC-517 step 5 (FR-122-AC-5): changed-version with its invocation bytes
/// edited under the same digest refuses `stale_dependency`/
/// `byte-digest-mismatch`.
#[trace("TC-517", "FR-122-AC-5")]
#[test]
fn edited_invocation_bytes_refuse_byte_digest_mismatch() {
    let documents = changed_version();
    let invocation = DigestRecord::mint(DigestDomain::Sha256Jcs, documents.read[0].digest).hex();
    let refusal = replay(case_with(
        config_version_unit(),
        "VersionUnchanged",
        documents,
        witness_source(),
        |wire, _| {
            let entry = wire
                .byte_provision
                .iter_mut()
                .find(|(_, hex, _)| *hex == invocation)
                .expect("the invocation is provided");
            entry.2 = String::from_utf8(entry.2.clone())
                .expect("UTF-8")
                .replace("\"boolean\":true", "\"boolean\":false")
                .into_bytes();
        },
    ))
    .unwrap_err();
    assert!(
        matches!(
            refusal,
            ReplayRefusal::Request(ReplayRequestRefusal::ByteDigestMismatch(_))
        ),
        "{refusal:?}"
    );
    assert!(refusal
        .to_string()
        .starts_with("stale_dependency/byte-digest-mismatch"));
}

/// TC-517 step 5 (FR-122-AC-5): `ParentOrder` over incomplete-population's
/// snapshot refuses with FR-106's `Incomplete` `incomplete_population`/
/// `incomplete-scope` record.
#[trace("TC-517", "FR-122-AC-5")]
#[test]
fn an_incomplete_population_refuses_with_the_incomplete_record() {
    let refusal = replay(case(
        config_version_unit(),
        "ParentOrder",
        incomplete_population(),
    ))
    .unwrap_err();
    let ReplayRefusal::Admission(AdmissionFailure::Incomplete(record)) = &refusal else {
        panic!("expected an incomplete admission, got {refusal:?}");
    };
    assert_eq!(record.code.as_str(), "incomplete_population");
    assert_eq!(record.cause, "incomplete-scope");
}

/// TC-517 step 6 (FR-122-AC-6): replaying step 1's `VersionUnchanged`
/// envelope twice gives equal results. `StateClauseCounterexample:
/// FamilyPayload`, the envelope carries it as its generic parameter, and a
/// payload's observation is one of the three forms.
#[trace("TC-517", "FR-122-AC-6")]
#[test]
fn replaying_one_envelope_twice_gives_equal_results() {
    fn family_payload<P: FamilyPayload>(envelope: &WitnessEnvelope<P>) -> &P {
        envelope.family_payload()
    }
    let first = case(config_version_unit(), "VersionUnchanged", changed_version());
    assert_eq!(
        family_payload(&first.envelope).observation.form(),
        ObservationForm::Invocation
    );
    let second = case(config_version_unit(), "VersionUnchanged", changed_version());
    assert_eq!(replay(first).unwrap(), replay(second).unwrap());
    let forms: Vec<ObservationForm> = [
        healthy_parent().observation,
        pre_call("a").observation,
        changed_version().observation,
    ]
    .iter()
    .map(ClauseSelectionInput::form)
    .collect();
    assert_eq!(
        forms,
        [
            ObservationForm::Current,
            ObservationForm::PreCall,
            ObservationForm::Invocation
        ]
    );
}

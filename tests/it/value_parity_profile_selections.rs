// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-484-AC-11 (TC-1252 VP-11): a unit whose lock selects a temporal and a
//! protocol profile still replays a function-value parity claim with
//! `profile_selections` `[]`: lock rows bind through `package_id` and are
//! never read or mapped into the request.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_foundation::SourceIdentity;
use qsl_replay::{
    call_site, replay_value_parity, CanonicalAssignment, DependencyInput, EnvelopeSource,
    Identifier, NoPayload, ProfileSelection, QualifiedName, ReplayLimits, ReplayRequest,
    ReplayRequestWire, RequestTargetWire, ScalarOutcome, StateEnvironment, TracePosition,
    ValueClaim, ValueParityResult, WitnessEnvelope, WitnessValue,
};
use qsl_replay::{ReplayTargetMembers, WitnessPacket};
use quire_exact::{Integer, ScalarLimits, Value};
use serde_json::json;

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

fn source_identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "test:value-parity-uc", "git", "r1")
}

/// TC-1252's unit `UC`.
fn uc() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         model Config = \"example/config-version\";\n\
         temporal ReachesTwo using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin {{ always eventually holds(c.versionNumber = 2) }}\n\
         protocol Flow using v over (input: Config::ConfigVersion) on origin {{\n\
         role R on Config::ConfigVersion;\n\
         run sequence Main {{ }}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}\n\
         function inc using v(x: Int[0, 9]): Integer pure {{ x + 1 }}\n"
    )
}

fn inc() -> QualifiedName {
    QualifiedName::new(vec![Identifier::new("inc").unwrap()]).unwrap()
}

fn integer(value: i64) -> ScalarOutcome {
    ScalarOutcome::Value(Value::Integer(Integer::from(value)))
}

/// FR-484-AC-11: the lock holds the two rows, the packet and the request
/// carry `[]`, the request decodes and replays, and each claim holds `UC`'s
/// `package_id`.
#[trace("TC-1252", "FR-484-AC-11")]
#[test]
fn a_unit_whose_lock_selects_profiles_replays_with_an_empty_selection_list() {
    let document = crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes();
    let source = uc();
    let located = call_site(
        source_identity(),
        "uc.native",
        source.as_bytes(),
        [document],
        &DependencyInput::default(),
        &inc(),
    )
    .expect("UC compiles and inc is located");

    let package: serde_json::Value = serde_json::from_slice(&located.package).unwrap();
    let mut rows = package["lock"]["profile_selections"]
        .as_array()
        .expect("the lock lists its profile selections")
        .clone();
    rows.sort_by_key(|row| row["role"].as_str().unwrap_or_default().to_owned());
    assert_eq!(
        rows,
        vec![
            json!({"definition": {"authority": "agent-ix", "identity": "quire.protocol.finite-global/v1"}, "role": "protocol_profile"}),
            json!({"definition": {"authority": "agent-ix", "identity": "quire.temporal.infinite-trace/v1"}, "role": "temporal_profile"}),
        ]
    );

    let site = &located.site;
    let parity = site.value_parity.as_ref().expect("inc is a value function");
    let source_digest = DigestRecord::mint(
        DigestDomain::SourceBytesV1,
        qsl_foundation::digest::ByteDigest::of(source.as_bytes()).as_bytes(),
    );
    let source_wire = (
        "agent-ix".to_owned(),
        "test:value-parity-uc".to_owned(),
        "git".to_owned(),
        "r1".to_owned(),
        Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
        source_digest.hex(),
    );
    let bindings = vec![CanonicalAssignment {
        parameter: site.parameters[0].1,
        value: WitnessValue::Integer(3),
    }];

    // The packet's profile_selections is the empty list, whatever the lock.
    let packet = WitnessPacket {
        obligation_identity: Some(*parity.obligation_identity.as_bytes()),
        occurrence_key: Some(site.declaration.clone()),
        clause_node: Some(site.function),
        replay_target: ReplayTargetMembers::function(inc()),
        package_id: Some((
            Some(located.package_id.domain().as_str().to_owned()),
            located.package_id.hex(),
        )),
        source_digests: Some(vec![source_wire.clone()]),
        profile_selections: Some(Vec::<ProfileSelection>::new()),
        run_limits: Some(UNLIMITED),
        declared_domains: Some(Vec::new()),
        backend: Some("generator-under-test".to_owned()),
        trace_position: Some(None::<TracePosition>),
        source: Some(EnvelopeSource::Input(bindings.clone())),
        family_payload: Some(NoPayload),
    };
    let envelope = WitnessEnvelope::reconstruct(packet, ReplayLimits::default()).expect("admitted");
    assert!(envelope.profile_selections().is_empty());

    let wire = ReplayRequestWire {
        profile_selections: envelope.profile_selections().to_vec(),
        package_id: (
            Some(envelope.package_id().domain().as_str().to_owned()),
            envelope.package_id().hex(),
        ),
        source_digests: envelope
            .source_digests()
            .iter()
            .map(qsl_replay::RawSourceRef::to_wire)
            .collect(),
        dependencies: Vec::new(),
        replay_target: RequestTargetWire::function(inc(), envelope.occurrence_key().clone()),
        source: envelope.source().clone().into(),
        obligation_identity: *envelope.obligation_identity().as_bytes(),
        backend: envelope.backend().as_str().to_owned(),
        state_environment: StateEnvironment::new(vec![]),
        accounting_limits: envelope.run_limits(),
        stage_limits: BTreeMap::new(),
        supplied_limits: None,
        object_environment: None,
        declared_domains: Vec::new(),
        byte_provision: vec![
            (
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest.hex(),
                source.as_bytes().to_vec(),
            ),
        ],
        documents: vec![document.to_vec()],
    };
    assert!(
        ReplayRequest::decode(wire.clone(), ReplayLimits::default()).is_ok(),
        "the request decodes: no unsupported-selection refusal"
    );
    for (generated, agrees) in [(4, true), (5, false)] {
        let report = replay_value_parity(
            wire.clone(),
            ValueClaim {
                generated: integer(generated),
                content_identity: DigestRecord::mint(DigestDomain::SourceBytesV1, [9; 32]),
            },
            ReplayLimits::default(),
        );
        match (report.result(), agrees) {
            (ValueParityResult::Agrees { .. }, true)
            | (ValueParityResult::Diverged { .. }, false) => {}
            (other, _) => panic!("generated {generated}: {other:?}"),
        }
        assert_eq!(report.claim().package_id, wire.package_id);
        assert_eq!(report.claim().package_id.1, located.package_id.hex());
    }
}

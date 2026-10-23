// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-484-AC-1 to FR-484-AC-10 (TC-1252): the obligation identity of a
//! function-value parity claim, minted by `call_site`, recomputed by
//! `replay_value_parity` and bound with every retained input.

use super::*;
use crate::witness::{NoPayload, WitnessEnvelope, WitnessPacket};
use crate::{ProfileSelection, TracePosition};
use qsl_semantics::value::supplied::SuppliedLimits;
use quire_canonical::Limits;
use quire_exact::{IntegerInterval, NodeKey, Origin, Role};
use quire_semantic_value::supplied_stop::SuppliedLimitKind;

const NODE: [u8; 32] = [0x22; 32];
const P0: &str = "ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda";
const T0: &str = "652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477";

fn from_hex(hex: &str) -> WireNodeId {
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * index..2 * index + 2], 16).unwrap();
    }
    WireNodeId::from_digest(bytes)
}

fn argument(
    parameter: WireNodeId,
    position: usize,
    semantic_type: WireNodeId,
) -> ValueParityArgument {
    ValueParityArgument {
        parameter,
        position,
        semantic_type,
    }
}

/// The spec's first argument: `x: Int[0, 9]` (QSpec FR-322 P4 typed by T4).
fn first() -> ValueParityArgument {
    argument(from_hex(P0), 0, from_hex(T0))
}

fn second() -> ValueParityArgument {
    argument(
        WireNodeId::from_digest([0x33; 32]),
        1,
        WireNodeId::from_digest([0x44; 32]),
    )
}

fn v2() -> ValueParityPreimage {
    ValueParityPreimage {
        node: WireNodeId::from_digest(NODE),
        arguments: vec![first(), second()],
    }
}

fn bytes_of<T: quire_canonical::Encode>(value: &T) -> String {
    String::from_utf8(quire_canonical::to_vec(value, Limits::new(u64::MAX)).unwrap()).unwrap()
}

fn sha_hex(text: &str) -> String {
    ByteDigest::of(text.as_bytes())
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn identity_hex(preimage: &ValueParityPreimage) -> String {
    value_parity_obligation(preimage).unwrap().to_string()
}

/// FR-484-AC-1: the 516-byte canonical text, its digest, and the first
/// argument alone.
#[trace("TC-1252", "FR-484-AC-1")]
#[test]
fn tc_1252_the_preimage_encodes_to_the_specified_bytes_and_digest() {
    let text = concat!(
        r#"{"arguments":[{"parameter":"ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda","position":0,"type":"652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477"},"#,
        r#"{"parameter":"3333333333333333333333333333333333333333333333333333333333333333","position":1,"type":"4444444444444444444444444444444444444444444444444444444444444444"}],"#,
        r#""node":"2222222222222222222222222222222222222222222222222222222222222222","#,
        r#""occurrence_key":{"ordinal":0,"role":"declaration"},"subject_kind":"function-value-parity"}"#
    );
    assert_eq!(text.len(), 516);
    assert_eq!(bytes_of(&v2()), text);
    assert_eq!(
        identity_hex(&v2()),
        "793909279b4897ab8f03f3c14547fc08861d353d979e471aa423c2bce9d8b92c"
    );
    let argument_text = format!(r#"{{"parameter":"{P0}","position":0,"type":"{T0}"}}"#);
    assert_eq!(bytes_of(&first()), argument_text);
    assert_eq!(
        sha_hex(&argument_text),
        "34f0d98f1306d5e3720eec3469a6181c73f809ac51c6fe608e97129e7f94270f"
    );
}

/// FR-484-AC-2: the empty list, the first argument alone, and the declared
/// order being part of the identity.
#[trace("TC-1252", "FR-484-AC-2")]
#[test]
fn tc_1252_the_declared_order_is_part_of_the_identity() {
    let with = |arguments| ValueParityPreimage {
        node: v2().node,
        arguments,
    };
    let empty = with(Vec::new());
    assert_eq!(
        bytes_of(&empty),
        r#"{"arguments":[],"node":"2222222222222222222222222222222222222222222222222222222222222222","occurrence_key":{"ordinal":0,"role":"declaration"},"subject_kind":"function-value-parity"}"#
    );
    assert_eq!(
        identity_hex(&empty),
        "908362fdf1c3d0bb20aadf2249e285964ad024f5fd994d6c01e88024839166b7"
    );
    assert_eq!(
        identity_hex(&with(vec![first()])),
        "3333f20dd244bd7043f410893713ed40ffc067b45cae410050898de89834cbc0"
    );
    let swapped = with(vec![
        argument(second().parameter, 0, second().semantic_type),
        argument(first().parameter, 1, first().semantic_type),
    ]);
    assert_eq!(
        identity_hex(&swapped),
        "f052b4583fc320a5a470209188b911e75fe02d55a28d079a5e614ac0f44e76fd"
    );
}

/// FR-484-AC-3: every member changes the digest, a position out of order
/// refuses, and the form never equals an operator-parity preimage.
#[trace("TC-1252", "FR-484-AC-3")]
#[test]
fn tc_1252_every_member_changes_the_digest_and_a_position_out_of_order_refuses() {
    let base = identity_hex(&v2());
    let other = WireNodeId::from_digest([0x55; 32]);
    let mut variants = Vec::new();
    let mut changed = v2();
    changed.node = other;
    variants.push(changed);
    let mut changed = v2();
    changed.arguments[1].parameter = other;
    variants.push(changed);
    let mut changed = v2();
    changed.arguments[1].semantic_type = other;
    variants.push(changed);
    let mut changed = v2();
    changed.arguments.pop();
    variants.push(changed);
    let mut changed = v2();
    changed.arguments.push(argument(other, 2, other));
    variants.push(changed);
    for variant in &variants {
        assert_ne!(identity_hex(variant), base);
    }
    let mut misplaced = v2();
    misplaced.arguments[1].position = 3;
    assert_eq!(
        value_parity_obligation(&misplaced),
        Err(IdentityEncodeError::PositionOrder {
            index: 1,
            position: 3
        })
    );
    let operator = ParityPreimage {
        node: v2().node,
        occurrence: Origin::new(Role::new("expression"), 0),
        obligation_kind: "operator_parity".to_owned(),
        arguments: vec![ParityArgument {
            identity: crate::OperandIdentity::GraphChild(other),
            domain: Domain::Range(
                IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).unwrap(),
            ),
        }],
    };
    assert_ne!(
        crate::parity_obligation(&operator).unwrap(),
        value_parity_obligation(&v2()).unwrap()
    );
}

fn unit() -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         function inc using v(x: Int[0, 9]): Integer pure {{ x + 1 }}\n\
         function add using v(x: Int[0, 9], y: Int[0, 9]): Integer pure {{ x + y }}\n\
         function small using v(x: Int[0, 9]): Boolean pure {{ x < 3 }}\n\
         function grow using v(n: Integer): Integer pure {{ n + 1 }}\n\
         function wide using v(m: Int[0, 2199023255552]): Integer pure {{ m + 1 }}\n"
    )
}

fn site(source: &str, function: &str) -> crate::CallSite<crate::FunctionSite> {
    crate::call_site(
        SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION),
        IDENTITY,
        source.as_bytes(),
        [],
        &crate::DependencyInput::default(),
        &name(&[function]),
    )
    .expect("the function is located")
}

/// A request for `function` built only from `call_site`'s retained inputs
/// (FR-484 "Retained claim inputs"), binding `value` to its first parameter.
fn retained_request(source: &str, function: &str, value: i64) -> ReplayRequestWire {
    let located = site(source, function);
    let compiled = spine(source, &BTreeMap::new());
    let mut wire = request(
        source.as_bytes(),
        compiled.emitted.package_id(),
        name(&[function]),
        input(located.site.parameters[0].1, value),
    );
    assert_eq!(
        wire.package_id.1,
        located.package_id.hex(),
        "the retained package reference is call_site's"
    );
    wire.replay_target =
        crate::RequestTargetWire::function(name(&[function]), located.site.declaration.clone());
    if let Some(parity) = &located.site.value_parity {
        wire.obligation_identity = *parity.obligation_identity.as_bytes();
    }
    wire
}

fn function_node(source: &str, function: &str) -> NodeKey {
    let compiled = spine(source, &BTreeMap::new());
    compiled
        .package
        .graph()
        .callable(function)
        .unwrap()
        .identity
}

/// FR-484-AC-4: `call_site` mints the identity of a non-`Boolean` function
/// from the package's own node ids, and none for a predicate.
#[trace("TC-1252", "FR-484-AC-4")]
#[test]
fn tc_1252_call_site_mints_the_identity_of_a_value_function_only() {
    let source = unit();
    let compiled = spine(&source, &BTreeMap::new());
    let semantic = compiled.package.graph().semantic_graph();
    for function in ["inc", "add"] {
        let located = site(&source, function).site;
        let parity = located
            .value_parity
            .expect("a value function has an identity");
        assert_eq!(parity.preimage.node, located.function);
        assert_eq!(parity.preimage.arguments.len(), located.parameters.len());
        let mut text = String::from("{\"arguments\":[");
        for (position, (_, parameter)) in located.parameters.iter().enumerate() {
            let key = semantic.resolve_wire(*parameter).expect("a parameter node");
            let declared = semantic
                .node(key)
                .and_then(|node| node.semantic_type())
                .expect("a typed parameter");
            let declared = WireNodeId::from_digest(*declared.as_bytes());
            assert_eq!(
                parity.preimage.arguments[position],
                argument(*parameter, position, declared)
            );
            if position > 0 {
                text.push(',');
            }
            text.push_str(&format!(
                "{{\"parameter\":\"{parameter}\",\"position\":{position},\"type\":\"{declared}\"}}"
            ));
        }
        text.push_str(&format!(
            "],\"node\":\"{}\",\"occurrence_key\":{{\"ordinal\":0,\"role\":\"declaration\"}},\"subject_kind\":\"function-value-parity\"}}",
            located.function
        ));
        assert_eq!(
            parity.obligation_identity.to_string(),
            sha_hex(&text),
            "{function}"
        );
    }
    let inc = site(&source, "inc").site.value_parity.unwrap();
    let add = site(&source, "add").site.value_parity.unwrap();
    assert_ne!(inc.obligation_identity, add.obligation_identity);
    assert_eq!(site(&source, "small").site.value_parity, None);
}

/// FR-484-AC-5: a request built only from the retained inputs agrees and
/// diverges, and each report's claim holds every retained input.
#[trace("TC-1252", "FR-484-AC-5")]
#[test]
fn tc_1252_a_request_from_the_retained_inputs_replays_and_carries_them() {
    let source = unit();
    let located = site(&source, "inc").site;
    for (generated, kind) in [(4, "agrees"), (5, "diverged")] {
        let wire = retained_request(&source, "inc", 3);
        let claim = value_claim(integer(generated));
        let report = replay_value_parity(wire.clone(), claim.clone(), ReplayLimits::default());
        assert_eq!(value_kind(report.result()), kind);
        let held = report.claim();
        assert_eq!(
            held.obligation.as_bytes(),
            located
                .value_parity
                .as_ref()
                .unwrap()
                .obligation_identity
                .as_bytes()
        );
        assert_eq!(held.occurrence_key, Some(located.declaration.clone()));
        assert_eq!(held.function, Some(name(&["inc"])));
        assert_eq!(held.package_id, wire.package_id);
        assert_eq!(held.source, wire.source);
        assert_eq!(held.limits, wire.accounting_limits);
        assert_eq!(held.supplied, SuppliedLimits::default());
        assert!(held.generated.same_as(&integer(generated)));
        assert_eq!(held.content_identity, claim.content_identity);
        assert_eq!(held, &crate::ValueIdentity::sent(&wire, &claim));
    }
}

fn refusal_of(report: &ValueParityReport) -> &ReplayRefusal {
    let ValueParityResult::Refused(refusal) = report.result() else {
        panic!("expected a refusal: {:?}", report.result());
    };
    refusal
}

/// FR-484-AC-6: a wrong identity refuses `Obligation` after the occurrence
/// check and before the argument join.
#[trace("TC-1252", "FR-484-AC-6")]
#[test]
fn tc_1252_a_wrong_identity_refuses_obligation_before_the_argument_join() {
    let source = unit();
    let inc = site(&source, "inc").site.value_parity.unwrap();
    let add = site(&source, "add").site;
    let mut retyped = inc.preimage.clone();
    retyped.arguments[0].semantic_type = WireNodeId::from_digest([0x44; 32]);
    let retyped = value_parity_obligation(&retyped).unwrap();
    let add_identity = add.value_parity.as_ref().unwrap().obligation_identity;
    let stray = add.parameters[1].1;
    for (claimed, extra) in [
        (*add_identity.as_bytes(), false),
        ([0; 32], false),
        (*retyped.as_bytes(), false),
        (*add_identity.as_bytes(), true),
    ] {
        let mut wire = retained_request(&source, "inc", 3);
        wire.obligation_identity = claimed;
        if extra {
            let ReplaySource::Input(assignments) = &mut wire.source else {
                panic!("an input source");
            };
            assignments.push(CanonicalAssignment {
                parameter: stray,
                value: WitnessValue::Integer(1),
            });
        }
        let report = replay_generated(wire, integer(4), ReplayLimits::default());
        let ReplayRefusal::ScalarIdentity(mismatch) = refusal_of(&report) else {
            panic!("expected ScalarIdentity: {:?}", report.result());
        };
        let ScalarIdentityMismatch::Obligation {
            claimed: named,
            recomputed,
        } = &**mismatch
        else {
            panic!("expected the Obligation cause: {mismatch:?}");
        };
        assert_eq!(named.as_bytes(), &claimed);
        assert_eq!(recomputed, &inc.obligation_identity);
        assert_eq!(
            report.terminal_value(),
            TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency))
        );
        assert_eq!(report.claim().obligation.as_bytes(), &claimed);
    }
}

/// FR-484-AC-7: an occurrence key other than the function's declaration
/// refuses `ClaimOccurrence` before the identity check, with no call.
#[trace("TC-1252", "FR-484-AC-7")]
#[test]
fn tc_1252_a_foreign_occurrence_refuses_before_any_call() {
    let source = unit();
    let compiled = spine(&source, &BTreeMap::new());
    let inc_node = wire_id(function_node(&source, "inc"));
    // `inc`'s `x + 1` application: an `expression` occurrence found by its
    // region text (a record-less occurrence still counts).
    let graph = compiled.package.graph();
    let application = graph
        .occurrences()
        .filter(|(_, origin, _)| origin.role() == &Role::new("expression"))
        .find(|(_, _, location)| {
            graph.region(location).is_some_and(|region| {
                let (start, end) = (region.start() as usize, region.end() as usize);
                source.get(start..end) == Some("x + 1")
                    && source[..start]
                        .ends_with("function inc using v(x: Int[0, 9]): Integer pure { ")
            })
        })
        .map(|(node, origin, _)| OccurrenceKey::new(wire_id(node), origin))
        .expect("inc holds an `x + 1` expression occurrence");
    assert_ne!(application.node(), inc_node);
    let keys = [
        site(&source, "add").site.declaration,
        application,
        OccurrenceKey::new(inc_node, Origin::new(Role::new("declaration"), 1)),
    ];
    for key in keys {
        let mut wire = retained_request(&source, "inc", 3);
        // The identity is wrong too: the occurrence check comes first.
        wire.obligation_identity = [0; 32];
        wire.replay_target = crate::RequestTargetWire::function(name(&["inc"]), key.clone());
        let report = replay_generated(wire, integer(4), ReplayLimits::default());
        let ReplayRefusal::ClaimOccurrence {
            key: named,
            function,
        } = refusal_of(&report)
        else {
            panic!("expected ClaimOccurrence: {:?}", report.result());
        };
        assert_eq!(named, &key);
        assert_eq!(function, &name(&["inc"]));
        assert_eq!(refusal_of(&report).code(), Code::MissingDeclaration);
    }
    let foreign = site(&source, "add").site.declaration;
    let mut stale = retained_request(&source, "inc", 3);
    stale.package_id = (
        Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
        DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]).hex(),
    );
    stale.replay_target = crate::RequestTargetWire::function(name(&["inc"]), foreign.clone());
    let report = replay_generated(stale, integer(4), ReplayLimits::default());
    assert!(
        matches!(refusal_of(&report), ReplayRefusal::PackageIdMismatch { .. }),
        "{:?}",
        report.result()
    );
    let mut small = retained_request(&source, "small", 3);
    small.replay_target = crate::RequestTargetWire::function(name(&["small"]), foreign);
    let report = replay_generated(small, integer(4), ReplayLimits::default());
    assert!(
        matches!(refusal_of(&report), ReplayRefusal::NotAValueFunction { .. }),
        "{:?}",
        report.result()
    );
}

/// FR-484-AC-8: the generated-content identity is carried unchanged on every
/// outcome, binds into the claim, and is never refused; the occurrence key
/// binds too.
#[trace("TC-1252", "FR-484-AC-8")]
#[test]
fn tc_1252_the_content_identity_and_occurrence_bind_into_the_claim() {
    let source = unit();
    let content = |byte: u8| DigestRecord::mint(DigestDomain::SourceBytesV1, [byte; 32]);
    let mut wrong = retained_request(&source, "inc", 3);
    wrong.obligation_identity = [0; 32];
    let cases = [
        (retained_request(&source, "inc", 3), integer(4), "agrees"),
        (retained_request(&source, "inc", 3), integer(5), "diverged"),
        (
            retained_request(&source, "inc", 12),
            integer(13),
            "refused input",
        ),
        (wrong, integer(4), "refused"),
    ];
    for (wire, generated, kind) in cases {
        let replay = |byte| {
            replay_value_parity(
                wire.clone(),
                crate::ValueClaim {
                    generated: generated.clone(),
                    content_identity: content(byte),
                },
                ReplayLimits::default(),
            )
        };
        let (a, b) = (replay(1), replay(2));
        assert_eq!(value_kind(a.result()), kind);
        assert_eq!(value_kind(b.result()), kind);
        assert_eq!(a.claim().content_identity, content(1));
        assert_eq!(b.claim().content_identity, content(2));
        assert_ne!(a.claim(), b.claim(), "{kind}");
    }
    let base = retained_request(&source, "inc", 3);
    let mut moved = base.clone();
    moved.replay_target =
        crate::RequestTargetWire::function(name(&["inc"]), site(&source, "add").site.declaration);
    let one = replay_generated(base, integer(4), ReplayLimits::default());
    let other = replay_generated(moved, integer(4), ReplayLimits::default());
    assert_ne!(one.claim(), other.claim());
}

/// The lock of `call_site`'s package bytes selects no profile for this unit.
/// The request carries an empty list (pending FR-484 profile-selection ruling).
fn assert_lock_selects_no_profile(package: &[u8]) {
    let wire: serde_json::Value = serde_json::from_slice(package).expect("package JSON");
    assert_eq!(wire["lock"]["profile_selections"], serde_json::json!([]));
}

/// FR-484-AC-9: the input-arm packet filled from `call_site` reconstructs,
/// round-trips, and the request built from the admitted envelope replays.
#[trace("TC-1252", "FR-484-AC-9")]
#[test]
fn tc_1252_the_input_arm_packet_round_trips_and_replays() {
    let source = unit();
    for (function, value, generated, kind) in
        [("grow", 41, 42, "agrees"), ("inc", 3, 5, "diverged")]
    {
        let located = site(&source, function);
        assert_lock_selects_no_profile(&located.package);
        let wire = retained_request(&source, function, value);
        let identity = *located
            .site
            .value_parity
            .as_ref()
            .unwrap()
            .obligation_identity
            .as_bytes();
        let packet = WitnessPacket {
            obligation_identity: Some(identity),
            occurrence_key: Some(located.site.declaration.clone()),
            clause_node: Some(located.site.function),
            replay_target: crate::ReplayTargetMembers::function(name(&[function])),
            package_id: Some((
                Some(located.package_id.domain().as_str().to_owned()),
                located.package_id.hex(),
            )),
            source_digests: Some(wire.source_digests.clone()),
            profile_selections: Some(Vec::<ProfileSelection>::new()),
            run_limits: Some(wire.accounting_limits),
            declared_domains: Some(Vec::new()),
            backend: Some("generator-under-test".to_owned()),
            trace_position: Some(None::<TracePosition>),
            source: Some(wire.source.clone()),
            family_payload: Some(NoPayload),
        };
        let sent = packet.clone();
        let envelope =
            WitnessEnvelope::reconstruct(packet, ReplayLimits::default()).expect("admitted");
        assert!(envelope.declared_domains().is_empty());
        assert_eq!(envelope.trace_position(), None);
        // Every one of the thirteen members round-trips exactly.
        let back = WitnessEnvelope::reconstruct(envelope.to_packet(), ReplayLimits::default())
            .expect("the round trip is admitted")
            .to_packet();
        assert_eq!(back.obligation_identity, sent.obligation_identity);
        assert_eq!(back.occurrence_key, sent.occurrence_key);
        assert_eq!(back.clause_node, sent.clause_node);
        assert_eq!(back.replay_target.select(), sent.replay_target.select());
        assert_eq!(back.package_id, sent.package_id);
        assert_eq!(back.source_digests, sent.source_digests);
        assert_eq!(back.profile_selections, sent.profile_selections);
        assert_eq!(back.run_limits, sent.run_limits);
        assert_eq!(back.declared_domains, sent.declared_domains);
        assert_eq!(back.backend, sent.backend);
        assert_eq!(back.trace_position, sent.trace_position);
        assert_eq!(back.source, sent.source);
        assert_eq!(back.family_payload, sent.family_payload);

        // The request is built from the admitted envelope's members alone.
        let mut built = wire;
        built.obligation_identity = *envelope.obligation_identity().as_bytes();
        built.package_id = (
            Some(envelope.package_id().domain().as_str().to_owned()),
            envelope.package_id().hex(),
        );
        built.source_digests = envelope
            .source_digests()
            .iter()
            .map(crate::RawSourceRef::to_wire)
            .collect();
        built.profile_selections = envelope.profile_selections().to_vec();
        built.accounting_limits = envelope.run_limits();
        built.backend = envelope.backend().as_str().to_owned();
        built.source = envelope.source().clone();
        built.replay_target = crate::RequestTargetWire::from(envelope.replay_target().clone());
        built.replay_target.occurrence_key = Some(envelope.occurrence_key().clone());
        let report = replay_generated(built, integer(generated), ReplayLimits::default());
        assert_eq!(value_kind(report.result()), kind, "{function}");
    }
}

/// FR-484-AC-10: A, H and B are bound into the claim; a B below a bounded
/// operand's bit length stops before the call as `PreCallIncomplete`, and an
/// unbounded `Integer` has no value-in-domain comparison for B to limit.
#[trace("TC-1252", "FR-484-AC-10")]
#[test]
fn tc_1252_the_supplied_ceilings_bind_into_the_claim() {
    let source = unit();
    let m = 1_099_511_627_776_i64;
    let with = |function: &str, value: i64, limits: Option<SuppliedLimits>| {
        let mut wire = retained_request(&source, function, value);
        wire.supplied_limits = limits;
        replay_generated(wire, integer(value + 1), ReplayLimits::default())
    };
    let b = |bits| Some(SuppliedLimits::default().with_numeric_magnitude_bits(bits));
    let wide = with("wide", m, b(4096));
    assert_eq!(value_kind(wide.result()), "agrees");
    let starved = with("wide", m, b(40));
    let ValueParityResult::PreCallIncomplete(stop) = starved.result() else {
        panic!("expected a pre-call stop: {:?}", starved.result());
    };
    let limit = stop.limit().expect("a limit cause");
    assert_eq!(limit.limit_kind, SuppliedLimitKind::NumericMagnitudeBits);
    assert_eq!(limit.limit, 40);
    assert_eq!(
        starved.terminal_value(),
        TerminalValue::Incomplete(crate::IncompleteCause::ResourceExhausted)
    );
    let mut starved_but_b = starved.claim().clone();
    assert_ne!(starved_but_b.supplied, wide.claim().supplied);
    assert_eq!(
        starved_but_b.supplied.admission_work_units(),
        wide.claim().supplied.admission_work_units()
    );
    assert_eq!(
        starved_but_b.supplied.conversion_work_units(),
        wide.claim().supplied.conversion_work_units()
    );
    assert_eq!(starved_but_b.supplied.numeric_magnitude_bits(), 40);
    starved_but_b.supplied = wide.claim().supplied;
    assert_eq!(&starved_but_b, wide.claim(), "the claims differ only in B");
    let omitted = with("wide", m, None);
    assert_eq!(
        omitted.claim(),
        wide.claim(),
        "omitted B is the default 4096"
    );
    let grow = with("grow", m, b(40));
    assert_eq!(value_kind(grow.result()), "agrees");

    let a = Some(SuppliedLimits::default().with_admission_work_units(16_777_217));
    let h = Some(SuppliedLimits::default().with_conversion_work_units(16_777_217));
    let (a, h) = (with("wide", m, a), with("wide", m, h));
    assert_ne!(a.claim(), omitted.claim());
    assert_ne!(h.claim(), omitted.claim());
    assert_ne!(a.claim(), h.claim());
}

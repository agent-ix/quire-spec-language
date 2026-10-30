// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1173 step 9, the root-path half of FR-404-AC-9, which is not tagged: a
//! consumer runs a call, builds the member's request (FR-404-AC-2), and
//! settles a refused one (an index that is no member's) through the
//! `qsl_replay` root alone. The parameter node of the call is not reachable
//! from the root, so the built request is not replayed here (FR-060's T12-A
//! rule is unmet, see QSL-1006). This file names
//! no `qsl_replay::spine`, `qsl_semantics`, `qsl_foundation` or `quire_exact`
//! item, so it stops compiling if the facade needs one. The six
//! non-constructible cases are `compile_fail` doctests on `CallReplayContext`.

use std::collections::BTreeMap;

fn fixture_node(fill: u8) -> NodeKey {
    NodeKey::new(
        "fixture",
        vec![quire_exact::PathSegment::Text(fill.to_string())],
        0,
    )
    .expect("fixture node")
}

use qsl_replay::CanonicalAssignment;
use qsl_replay::ScalarLimits;
use qsl_replay::{
    run_with_replay_context, BackendId, Call, CallArgument, CallerLimits, Cancel, DependencyInput,
    DomainKey, DomainKind, FiniteBound, Integer, ObjectEnvironment, PluginReplayRefusal,
    ProofBound, ReplayRequestWire, RequestIndex, SourceIdentity, TerminalValue, Value, NodeKey,
    WitnessValue,
};
use quire_semantic_value::supplied_stop::SuppliedLimits;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n\
     function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n";

#[test]
fn a_consumer_runs_builds_and_settles_through_the_crate_root() {
    let call = Call {
        function: "inc".to_owned(),
        arguments: vec![CallArgument {
            parameter: "x".to_owned(),
            value: Value::Integer(Integer::from(3_i64)),
        }],
        accounting: ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: 1_000_000,
            result_units: u64::MAX,
        },
        supplied: SuppliedLimits::default(),
    };
    let result = run_with_replay_context(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        SOURCE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        CallerLimits::default(),
        &call,
        &ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .expect("the call runs");
    let (package_id, _, requirements, context) = result.into_parts();
    assert_eq!(context.package_id(), package_id);
    assert_eq!(requirements.members().len(), 1);
    let plugin = BackendId::new("acme");
    let node = fixture_node(7);
    let bounds = [ProofBound::new(
        DomainKey::Node { node, path: vec![] },
        Some(DomainKind::Integer),
        FiniteBound::integer_range(Integer::from(0_i64), Integer::from(9_i64)).unwrap(),
    )
    .unwrap()];
    let assignments = vec![CanonicalAssignment {
        parameter: node,
        value: WitnessValue::Integer(9),
    }];
    let member = &requirements.members()[0];
    let request: ReplayRequestWire = context
        .request(
            package_id,
            member.request_index(),
            &bounds,
            &plugin,
            assignments.clone(),
        )
        .expect("a member is an application item");
    assert_eq!(request.package_id.1, package_id.hex());
    assert_eq!(
        request.replay_target.occurrence_key.as_ref(),
        Some(member.occurrence())
    );

    let index = RequestIndex::new(99);
    let built: Result<ReplayRequestWire, PluginReplayRefusal> =
        context.request(package_id, index, &bounds, &plugin, assignments);
    let refusal = built.err().expect("no such item");
    assert_eq!(
        refusal,
        PluginReplayRefusal::NotACallItem {
            request_index: index
        }
    );
    let record = refusal.record(index, plugin.clone());
    assert_eq!(record.value(), &TerminalValue::Failed);
    assert_eq!(record.backend(), Some(&plugin));
}

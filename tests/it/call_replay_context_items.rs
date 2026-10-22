// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1173 (FR-404): the context names an item by the pairing key the run's
//! `package_id` and a member's request index, which is also the index of the
//! item `items_from_requirements` writes for that member (FR-075). These
//! need `qsl-route`, which only the root crate's tests hold.

use std::collections::BTreeMap;

use qsl_foundation::{BackendId, SourceIdentity};
use qsl_replay::spine::default_accounting;
use qsl_replay::{
    run_with_replay_context, Call, CallArgument, CallerLimits, DependencyInput, Integer,
    ObjectEnvironment, PluginReplayRefusal, SuppliedLimits, TerminalValue, Value,
};
use qsl_route::request::items_from_requirements;
use qsl_route::Registry;
use qsl_semantics::check::PackageDeclarations;
use quire_exact::Cancel;
use quire_semantic_value::checking::CheckingLimits;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function q using v(x: Int[0, 9]): Integer pure { x + 1 }\n";

/// A member's request, built from the item `items_from_requirements` writes
/// at the member's index and that item's real proof bounds, is refused
/// `NotReplayable` and settles `inconclusive`: a member is an application
/// record, which `PluginReplayContext::request` does not replay until
/// QSL-980. The replayed arms of FR-404-AC-2 and AC-3 are not asserted here.
#[test]
fn a_members_item_bounds_refuse_not_replayable_until_application_claims() {
    let call = Call {
        function: "q".to_owned(),
        arguments: vec![CallArgument {
            parameter: "x".to_owned(),
            value: Value::Integer(Integer::from(3_i64)),
        }],
        accounting: default_accounting(1_000_000),
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

    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        SOURCE.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &BTreeMap::new(),
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the model is admitted");
    let graph = PackageDeclarations::assemble(raw, unit, models, Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the unit checks");
    let items = items_from_requirements(graph.requirements(), &Registry::new(), None);

    let member = &result.requirements().members()[0];
    let item = &items[member.request_index().get()];
    assert_eq!(item.occurrence(), member.occurrence());
    let plugin = BackendId::new("acme");
    let refusal = result
        .context()
        .request(
            result.package_id(),
            member.request_index(),
            item.proof_bounds(),
            &plugin,
            Vec::new(),
        )
        .err()
        .expect("an application item is not replayable yet");
    assert_eq!(
        refusal,
        PluginReplayRefusal::NotReplayable {
            key: member.occurrence().clone()
        }
    );
    let record = refusal.record(member.request_index(), plugin);
    assert!(matches!(record.value(), TerminalValue::Inconclusive(_)));
}

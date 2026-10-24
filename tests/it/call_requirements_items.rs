// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1089 (FR-395-AC-2, FR-075-AC-17, FR-062-AC-14): the members
//! `run_with_requirements` returns pair with the items
//! `items_from_requirements` writes, and the check stage records an owner for
//! a value-validity record only. These need `qsl-route` and the ConfigVersion
//! domain package, which only the root crate's tests hold.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{default_accounting, SpineLimits};
use qsl_replay::{
    run_with_requirements, Call, CallArgument, DependencyInput, Integer, ObjectEnvironment,
    OccurrenceKey, SuppliedLimits, Value,
};
use qsl_route::request::items_from_requirements;
use qsl_route::Registry;
use qsl_semantics::check::{Capability, CheckedGraph, PackageDeclarations};
use quire_exact::Cancel;
use quire_semantic_value::checking::CheckingLimits;

const K: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n\
    function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }\n\
    function id using v(x: Int[0, 9]): Int[0, 9] pure { x }\n\
    function two using v(): Int[0, 10] pure { inc(1) }\n";

fn members(function: &str) -> Vec<(OccurrenceKey, usize)> {
    let call = Call {
        function: function.to_owned(),
        arguments: vec![CallArgument {
            parameter: "x".to_owned(),
            value: Value::Integer(Integer::from(3_i64)),
        }],
        accounting: default_accounting(1_000_000),
        supplied: SuppliedLimits::default(),
    };
    let result = run_with_requirements(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        K.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call,
        &ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .expect("the call runs");
    result
        .requirements()
        .members()
        .iter()
        .map(|member| (member.occurrence().clone(), member.request_index().get()))
        .collect()
}

fn check(text: &str, packages: &BTreeMap<[u8; 32], Vec<u8>>) -> CheckedGraph {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:call-requirements", "fixture", "fixture:1"),
        "program.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("the model is admitted");
    PackageDeclarations::assemble(raw, unit, models, Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the unit checks")
}

/// FR-395-AC-2 and FR-075-AC-17: over K, each member's request index is its
/// key's position in the package order, the item `items_from_requirements`
/// writes at that index has the member's key, and the members of `inc` and
/// `sq` are exactly the four items, each in one call.
#[trace("TC-1089", "FR-395-AC-2", "FR-075-AC-17")]
#[test]
fn members_pair_with_the_items_at_their_request_index() {
    let graph = check(K, &BTreeMap::new());
    let items = items_from_requirements(graph.requirements(), &Registry::new(), None);
    assert_eq!(items.len(), 4);
    let inc = members("inc");
    let sq = members("sq");
    let mut seen = Vec::new();
    for (key, index) in inc.iter().chain(&sq) {
        let item = items.get(*index).expect("an item at the index");
        assert_eq!(item.index().get(), *index);
        assert_eq!(item.occurrence(), key);
        seen.push(*index);
    }
    seen.sort_unstable();
    assert_eq!(seen, [0, 1, 2, 3]);
}

/// FR-062-AC-14 (TC-1089 step 6): a `value-validity` record has the function
/// as owner; an `operation-contract` record and a `temporal-satisfaction`
/// record have none.
#[trace("TC-1089", "FR-062-AC-14")]
#[test]
fn only_value_validity_records_have_an_owner() {
    let document = crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]);
    let digest = packages.keys().next().expect("one package");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         function inc using v(x: Int[0, 9]): Int[0, 10] pure {{ x + 1 }}\n\
         post Q using v on Config::ConfigVersion::attemptUpdate {{ true }}\n\
         temporal Reaches using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin {{ eventually holds(c.versionNumber = 2) }}\n",
        qsl_semantics::model::key::hex(digest)
    );
    let graph = check(&text, &packages);
    let mut kinds = Vec::new();
    for record in graph.requirements().values() {
        let kind = record.requirements().kind();
        let expected = (kind == Capability::ValueValidity).then_some("inc");
        assert_eq!(record.owner(), expected, "{kind:?}");
        kinds.push(kind);
    }
    kinds.sort_by_key(|kind| kind.to_wire());
    // A `post` clause writes a clause record and an operation-frame record.
    kinds.dedup();
    assert_eq!(
        kinds,
        [
            Capability::OperationContract,
            Capability::TemporalSatisfaction,
            Capability::ValueValidity
        ]
    );
}

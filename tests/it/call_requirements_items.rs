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
    OccurrenceKey, Value,
};
use qsl_route::request::items_from_requirements;
use qsl_route::Registry;
use qsl_semantics::check::{Capability, CheckedGraph, PackageDeclarations};
use quire_exact::Cancel;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::supplied_stop::SuppliedLimits;

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
        backend: quire_eval::ExecutionBackendChoice::Interpreter,
        prepare: qsl_eval::execution::PrepareLimits::default(),
    };
    let result = run_with_requirements(
        SourceIdentity::new("a", "u"),
        "fixture".to_owned(),
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

fn check(text: &str, packages: &qsl_semantics::model::intake::PackageInput) -> CheckedGraph {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:call-requirements"),
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
    PackageDeclarations::assemble(raw, "fixture".to_string(), unit, models, Vec::new())
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
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let text = String::from(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n\
         model Config = \"example/config-version\";\n\
         function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }\n\
         post Q using v on Config::ConfigVersion::attemptUpdate { true }\n\
         temporal Reaches using t over (c: Config::ConfigVersion) clock \"model-steps\" \
         on origin { eventually holds(c.versionNumber = 2) }\n",
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

/// TC-1308 row 5: the one checked predicate is called from a function, a
/// state invariant, a protocol check and a temporal clause; every emitted
/// call keeps its own source-map occurrence and names that same callee node.
#[trace("TC-1308", "FR-046-AC-1", "FR-513-AC-3")]
#[test]
fn predicate_calls_from_each_clause_family_share_one_checked_callee() {
    let packages = qsl_semantics::model::intake::package_input([
        crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes(),
    ])
    .expect("the ConfigVersion package is keyed by its identity");
    let unit = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.model.complete/v1\";\n",
        "profile t = \"quire.temporal.infinite-trace/v1\";\n",
        "profile p = \"quire.protocol.finite-global/v1\";\n",
        "model Config = \"example/config-version\";\n",
        "predicate Positive using v(a: Int[0, 1000], b: Int[0, 1000]): Boolean { a <= b }\n",
        "function FunctionCall using v(x: Int[0, 1000], y: Int[0, 1000]): Boolean pure { Positive(x, y) }\n",
        "invariant StateCall using v on Config::ConfigVersion at current { Positive(self.versionNumber, 1) }\n",
        "temporal TemporalCall using t over (c: Config::ConfigVersion) clock \"model-steps\" on origin { always holds(Positive(c.versionNumber, 1)) }\n",
        "protocol Flow using p over (view: Config::ConfigVersion) on origin { ",
        "capture saved: Int[0, 1000] = view.versionNumber; ",
        "role R on Config::ConfigVersion; run sequence Main { check ProtocolCall using v { Positive(7, saved) }; } ",
        "finish End as (outcome: Boolean) { true }; }\n",
    );
    let emitted = crate::support::front_end::emitted(
        SourceIdentity::new("agent-ix", "test:tc-1308-calls"),
        "predicate-calls.native",
        unit.as_bytes(),
        &packages,
        &DependencyInput::default(),
    )
    .expect("the four checked call contexts admit and emit");
    let bytes = emitted.package().bytes();
    let mut wire: serde_json::Value = serde_json::from_slice(bytes).expect("checked package JSON");
    qsl_semantics::check::wire_id::resolve_operand_slots(&mut wire)
        .expect("every operand slot names a row entry");
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic nodes");
    let predicates: Vec<_> = nodes
        .iter()
        .filter(|node| node["declaration"]["qualified_name"] == serde_json::json!(["Positive"]))
        .collect();
    assert_eq!(predicates.len(), 1, "one Positive declaration is emitted");
    let calls: Vec<_> = nodes
        .iter()
        .filter(|node| node["semantic_form"] == "call")
        .collect();
    assert_eq!(calls.len(), 4, "one call per checked clause family");
    let selected_callee = calls[0]["body"]["arguments"][0]["target"].clone();
    let selected_predicate = predicates
        .iter()
        .find(|predicate| predicate["node_id"] == selected_callee)
        .expect("the integer call selects one exact Positive overload");
    let mut authored_calls = std::collections::BTreeSet::new();
    let mut state_receiver = None;
    let mut temporal_receiver = None;
    for call in &calls {
        assert_eq!(call["body"]["operator"], "call");
        let arguments = call["body"]["arguments"]
            .as_array()
            .expect("call arguments");
        assert_eq!(arguments.len(), 3, "callee followed by two ordered args");
        assert_eq!(arguments[0]["target"], selected_predicate["node_id"]);
        assert!(
            call["dependencies"].as_array().is_some_and(|dependencies| {
                dependencies.contains(&selected_predicate["node_id"])
            }),
            "call does not name the unique Positive node: {call}"
        );
        let source_map = wire["source_map"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["node_id"] == call["node_id"])
            .expect("each call has a source-map row");
        let call_text = source_map["regions"]
            .as_array()
            .and_then(|regions| {
                regions.iter().find_map(|region| {
                    let (Some(start), Some(end)) = (
                        region["start"]
                            .as_u64()
                            .and_then(|value| usize::try_from(value).ok()),
                        region["end"]
                            .as_u64()
                            .and_then(|value| usize::try_from(value).ok()),
                    ) else {
                        return None;
                    };
                    unit.get(start..end)
                        .filter(|text| text.starts_with("Positive("))
                        .map(str::to_owned)
                })
            })
            .expect("call source-map row retains the authored call expression");
        match call_text.as_str() {
            "Positive(x, y)" => {
                assert!(arguments[1]["target"]["path"]
                    .as_array()
                    .is_some_and(|path| path.iter().any(|part| part == "x")));
                assert!(arguments[2]["target"]["path"]
                    .as_array()
                    .is_some_and(|path| path.iter().any(|part| part == "y")));
            }
            "Positive(self.versionNumber, 1)" => {
                assert_ne!(arguments[1]["target"], arguments[2]["target"]);
                assert!(arguments[1]["target"]["path"]
                    .as_array()
                    .is_some_and(|path| path.iter().any(|part| part == "quire.op.record.project")));
                state_receiver = Some(arguments[1]["target"].clone());
            }
            "Positive(c.versionNumber, 1)" => {
                assert_ne!(arguments[1]["target"], arguments[2]["target"]);
                assert!(arguments[1]["target"]["path"]
                    .as_array()
                    .is_some_and(|path| path.iter().any(|part| part == "quire.op.record.project")));
                temporal_receiver = Some(arguments[1]["target"].clone());
            }
            "Positive(7, saved)" => {
                assert_ne!(arguments[1]["target"], arguments[2]["target"]);
                assert!(arguments[2]["target"]["path"]
                    .as_array()
                    .is_some_and(|path| path.iter().any(|part| part == "saved")));
            }
            _ => panic!("unexpected call expression {call_text}"),
        }
        authored_calls.insert(call_text);
    }
    assert_eq!(
        authored_calls,
        [
            "Positive(x, y)",
            "Positive(self.versionNumber, 1)",
            "Positive(c.versionNumber, 1)",
            "Positive(7, saved)",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
        "the checked call terms preserve argument order and each context receiver"
    );
    assert_ne!(
        state_receiver.expect("state invariant receiver target"),
        temporal_receiver.expect("temporal receiver target"),
        "self and c bind to distinct checked model receiver identities"
    );
    assert!(
        has_string_leaf(&wire, "saved"),
        "the emitted protocol retains the capture binder referenced by its call"
    );
}

/// Whether any string leaf of `value` equals `needle` exactly.
fn has_string_leaf(value: &serde_json::Value, needle: &str) -> bool {
    match value {
        serde_json::Value::String(text) => text == needle,
        serde_json::Value::Array(items) => items.iter().any(|item| has_string_leaf(item, needle)),
        serde_json::Value::Object(map) => map.values().any(|item| has_string_leaf(item, needle)),
        _ => false,
    }
}

/// TC-1308 row 7: recursive predicate cycles without an accepted measure are
/// refused by the complete front-end check stage before a package is emitted.
#[trace("TC-1308", "FR-046-AC-2", "FR-513-AC-4")]
#[test]
fn recursive_predicate_cycles_without_measures_refuse_on_the_spine() {
    let unit = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "predicate R using v(x: Boolean): Boolean { R(x) }\n",
        "predicate A using v(x: Boolean): Boolean { B(x) }\n",
        "predicate B using v(x: Boolean): Boolean { A(x) }\n",
    );
    let refusal = crate::support::front_end::emitted(
        SourceIdentity::new("agent-ix", "test:tc-1308-recursion"),
        "recursive-predicates.native",
        unit.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect_err("both recursive SCCs require measures");
    let qsl_replay::spine::CompileRefusal::Check { refusals, .. } = *refusal else {
        panic!("recursive predicates must refuse at check, got {refusal:?}");
    };
    let cycles: Vec<_> = refusals
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            qsl_semantics::check::CheckCause::UnprovedDecrease {
                cycle,
                obligation: qsl_semantics::check::MeasureObligation::MissingMeasure,
            } => Some(cycle),
            _ => None,
        })
        .collect();
    assert!(
        cycles
            .iter()
            .any(|cycle| cycle.first() == cycle.last() && cycle.len() == 2),
        "self recursion must carry its typed missing-measure obligation: {refusals:?}"
    );
    assert!(
        cycles
            .iter()
            .any(|cycle| cycle.len() == 3 && cycle.first() == cycle.last()),
        "mutual recursion must carry its typed missing-measure obligation: {refusals:?}"
    );
}

/// TC-1308 row 8: the cycles of row 7, each predicate with a QSpec FR-146
/// integer measure that decreases on every recursive edge, admit and emit.
#[trace("TC-1308", "FR-046-AC-2", "FR-513-AC-4")]
#[test]
fn recursive_predicate_cycles_with_decreasing_measures_admit_on_the_spine() {
    let unit = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "predicate R using v(x: Int[0, 9]): Boolean decreases(x) { if x > 0 then R(x - 1) else true }\n",
        "predicate A using v(x: Int[0, 9]): Boolean decreases(x) { if x > 0 then B(x - 1) else true }\n",
        "predicate B using v(x: Int[0, 9]): Boolean decreases(x) { if x > 0 then A(x - 1) else false }\n",
    );
    crate::support::front_end::emitted(
        SourceIdentity::new("agent-ix", "test:tc-1308-measured-recursion"),
        "measured-predicates.native",
        unit.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("measured self and mutual predicate recursion admits");

    // A measure that does not decrease on the edge still refuses: the
    // measure is checked, not merely accepted. Both the self cycle and the
    // mutual cycle passing `x` unchanged refuse.
    for (identity, declarations) in [
        (
            "test:tc-1308-undecreasing-measure",
            "predicate R using v(x: Int[0, 9]): Boolean decreases(x) { R(x) }\n",
        ),
        (
            "test:tc-1308-undecreasing-mutual-measure",
            "predicate A using v(x: Int[0, 9]): Boolean decreases(x) { B(x) }\n\
             predicate B using v(x: Int[0, 9]): Boolean decreases(x) { A(x) }\n",
        ),
    ] {
        let unit = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n{declarations}"
        );
        let refusal = crate::support::front_end::emitted(
            SourceIdentity::new("agent-ix", identity),
            "undecreasing-predicate.native",
            unit.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
        )
        .expect_err("a measure that does not decrease refuses");
        let qsl_replay::spine::CompileRefusal::Check { refusals, .. } = *refusal else {
            panic!("an undecreasing measure refuses at check, got {refusal:?}");
        };
        assert!(
            refusals.iter().any(|refusal| matches!(
                &refusal.cause,
                qsl_semantics::check::CheckCause::UnprovedDecrease {
                    obligation: qsl_semantics::check::MeasureObligation::Decrease,
                    ..
                }
            )),
            "{identity}: the undecreasing edge carries the decrease obligation: {refusals:?}"
        );
    }
}

/// TC-1308 row 10: `D`'s divisor guarded by the QSpec FR-146 rational
/// nonzero fact `x != rational(0, 1)` admits, as do its callers.
#[trace("TC-1308", "FR-046-AC-2", "FR-513-AC-4")]
#[test]
fn guarded_predicate_division_admits_on_the_spine() {
    let unit = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "predicate D using v(x: Rational[-10, 10; 1, 5]): Boolean { x != rational(0, 1) and rational(10, 1) / x > rational(1, 1) }\n",
        "function Caller using v(x: Rational[-10, 10; 1, 5]): Boolean pure { D(x) }\n",
        "function Dependent using v(x: Rational[-10, 10; 1, 5]): Boolean pure { Caller(x) }\n",
    );
    crate::support::front_end::emitted(
        SourceIdentity::new("agent-ix", "test:tc-1308-guarded-division"),
        "guarded-division.native",
        unit.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("the guarded divisor carries its nonzero fact");

    // Only `x != rational(0, 1)` on its true outcome (or `x = rational(0, 1)`
    // on its false one) is a nonzero fact; any other comparison proves none.
    for (identity, body) in [
        (
            "test:tc-1308-equal-zero-guard",
            "x = rational(0, 1) and rational(10, 1) / x > rational(1, 1)",
        ),
        (
            "test:tc-1308-nonzero-literal-guard",
            "x != rational(1, 1) and rational(10, 1) / x > rational(1, 1)",
        ),
    ] {
        let unit = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n\
             predicate D using v(x: Rational[-10, 10; 1, 5]): Boolean {{ {body} }}\n"
        );
        let refusal = crate::support::front_end::emitted(
            SourceIdentity::new("agent-ix", identity),
            "guarded-division.native",
            unit.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
        )
        .expect_err("a guard that proves no nonzero fact refuses");
        assert!(
            refusal.causes().iter().any(|cause| {
                cause.code.as_str() == "undefined_expression"
                    && cause.cause == Some("unproved-nonzero")
            }),
            "{body}: {refusal:?}"
        );
    }
    let unit = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "predicate D using v(x: Rational[-10, 10; 1, 5]): Boolean { if x = rational(0, 1) then false else rational(10, 1) / x > rational(1, 1) }\n",
    );
    crate::support::front_end::emitted(
        SourceIdentity::new("agent-ix", "test:tc-1308-equal-zero-else"),
        "guarded-division.native",
        unit.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("the false outcome of `x = rational(0, 1)` is a nonzero fact");

    // The integer divisor in a rational-typed context (QSpec FR-147-AC-11):
    // `x != 0` is the QSpec FR-146 integer nonzero fact.
    let unit = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "function F using v(x: Int[-10, 10]): Rational[-10, 10; 1, 10] pure { if x != 0 then 10 / x else rational(0, 1) }\n",
        "predicate DI using v(x: Int[-10, 10]): Boolean { F(x) > rational(1, 1) }\n",
    );
    crate::support::front_end::emitted(
        SourceIdentity::new("agent-ix", "test:tc-1308-guarded-integer-division"),
        "guarded-division.native",
        unit.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
    )
    .expect("the guarded integer divisor carries its nonzero fact");
}

/// TC-1308 row 9/10: predicate call type errors and a partial divisor retain
/// their typed check refusal through the complete spine, including a
/// possibly-zero rational divisor.
#[trace("TC-1308", "FR-046-AC-2", "FR-513-AC-4", "FR-040-AC-4")]
#[test]
fn predicate_call_errors_and_partial_division_are_checked_on_the_spine() {
    let compile = |identity: &str, declarations: &str| {
        let unit = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile v = \"quire.value.complete/v1\";\n{declarations}"
        );
        let result = crate::support::front_end::emitted(
            SourceIdentity::new("agent-ix", identity),
            "predicate-errors.native",
            unit.as_bytes(),
            &BTreeMap::new(),
            &DependencyInput::default(),
        );
        (unit, result)
    };
    for (identity, call) in [
        ("test:tc-1308-wrong-arity", "Positive(1)"),
        ("test:tc-1308-wrong-type", "Positive(true, 1)"),
    ] {
        let (unit, result) = compile(
            identity,
            &format!(
                "predicate Positive using v(a: Int[0, 9], b: Int[0, 9]): Boolean {{ a <= b }}\n\
                 function Caller using v(): Boolean pure {{ {call} }}\n\
                 function Dependent using v(): Boolean pure {{ Caller() }}\n"
            ),
        );
        let refusal = result.expect_err("a malformed call to Positive refuses");
        assert!(
            refusal
                .causes()
                .iter()
                .any(|cause| cause.code.as_str() == "ill_typed"),
            "the call refusal remains ill_typed: {refusal:?}"
        );
        let qsl_replay::spine::CompileRefusal::Check {
            region: Some(region),
            ..
        } = refusal.as_ref()
        else {
            panic!("the call refusal must identify its original check span: {refusal:?}");
        };
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        let span = unit
            .get(start..end)
            .expect("check region indexes the authored source");
        let call_start = unit.find(call).expect("authored call is present");
        let call_end = call_start + call.len();
        assert!(
            start >= call_start && end <= call_end,
            "refusal span {span:?} is inside the original call {call:?}"
        );
    }

    let (unit, wrong_unit) = compile(
        "test:tc-1308-wrong-unit",
        "dimension Time;\n\
         unit s: Time = rational(1, 1);\n\
         unit ms: Time = rational(1, 1000) * s;\n\
         predicate Milliseconds using v(x: Quantity<ms>): Boolean { true }\n\
         function Caller using v(): Boolean pure { Milliseconds(3 s) }\n",
    );
    let refusal = wrong_unit.expect_err("a call with the wrong quantity unit refuses");
    assert!(
        refusal
            .causes()
            .iter()
            .any(|cause| cause.code.as_str() == "ill_typed"),
        "the unit mismatch remains ill_typed: {refusal:?}"
    );
    let qsl_replay::spine::CompileRefusal::Check {
        region: Some(region),
        ..
    } = refusal.as_ref()
    else {
        panic!("the unit mismatch identifies its source span: {refusal:?}");
    };
    let start = usize::try_from(region.start()).unwrap();
    let end = usize::try_from(region.end()).unwrap();
    assert_eq!(
        unit.get(start..end),
        Some("3 s"),
        "unit mismatch points at the original call argument"
    );

    let (unit, result) = compile(
        "test:tc-1308-unguarded-division",
        "predicate D using v(x: Rational[-10, 10; 1, 5]): Boolean { rational(10, 1) / x > rational(1, 1) }\n\
         function Caller using v(x: Rational[-10, 10; 1, 5]): Boolean pure { D(x) }\n\
         function Dependent using v(x: Rational[-10, 10; 1, 5]): Boolean pure { Caller(x) }\n",
    );
    let refusal = result.expect_err("an unguarded divisor that may be zero refuses");
    // The refused predicate `D` is reported once, at its own body; the
    // callers `Caller` and `Dependent` have no package to emit, so the unit
    // as a whole refuses and nothing is emitted for them.
    let qsl_replay::spine::CompileRefusal::Check { refusals, .. } = refusal.as_ref() else {
        panic!("the division refusal is a check refusal: {refusal:?}");
    };
    assert_eq!(
        refusals.len(),
        1,
        "one refusal, at the predicate: {refusals:?}"
    );
    assert!(
        matches!(
            &refusals[0].location.origin,
            quire_semantic_value::location::ExpressionOwner::Body { function, .. } if function == "D"
        ),
        "the refusal is located in D: {refusals:?}"
    );
    assert!(
        refusal.causes().iter().any(|cause| {
            cause.code.as_str() == "undefined_expression" && cause.cause == Some("unproved-nonzero")
        }),
        "the division refusal remains unproved-nonzero: {refusal:?}"
    );
    let qsl_replay::spine::CompileRefusal::Check {
        region: Some(region),
        ..
    } = refusal.as_ref()
    else {
        panic!("the division refusal must identify its authored span: {refusal:?}");
    };
    let start = usize::try_from(region.start()).unwrap();
    let end = usize::try_from(region.end()).unwrap();
    let span = unit
        .get(start..end)
        .expect("check region indexes the authored source");
    assert!(
        span.contains("rational(10, 1) / x"),
        "refusal span is {span:?}"
    );

    // The integer divisor: an `Int[..]` operand of a rational `/` whose
    // expected result type is rational (QSpec FR-147-AC-11) still needs its
    // nonzero proof (QSpec FR-146), and the predicate calling it refuses too.
    let (unit, result) = compile(
        "test:tc-1308-unguarded-integer-division",
        "function F using v(x: Int[-10, 10]): Rational[-10, 10; 1, 10] pure { 10 / x }\n\
         predicate DI using v(x: Int[-10, 10]): Boolean { F(x) > rational(1, 1) }\n",
    );
    let refusal = result.expect_err("an unguarded integer divisor that may be zero refuses");
    assert!(
        refusal.causes().iter().any(|cause| {
            cause.code.as_str() == "undefined_expression" && cause.cause == Some("unproved-nonzero")
        }),
        "the integer division refusal is unproved-nonzero: {refusal:?}"
    );
    let qsl_replay::spine::CompileRefusal::Check {
        region: Some(region),
        ..
    } = refusal.as_ref()
    else {
        panic!("the integer division refusal identifies its span: {refusal:?}");
    };
    let start = usize::try_from(region.start()).unwrap();
    let end = usize::try_from(region.end()).unwrap();
    assert!(
        unit.get(start..end)
            .is_some_and(|span| span.contains("10 / x")),
        "refusal span is {:?}",
        unit.get(start..end)
    );

    let (non_boolean_unit, non_boolean) = compile(
        "test:tc-1308-non-boolean-predicate",
        "predicate NonBoolean using v(): Int[0, 9] { 1 }\n",
    );
    let refusal = non_boolean.expect_err("a predicate with a non-Boolean result refuses");
    let qsl_replay::spine::CompileRefusal::Source(diagnostic) = refusal.as_ref() else {
        panic!("a non-Boolean predicate refuses at the parser: {refusal:?}");
    };
    assert_eq!(diagnostic.message, "expected `Boolean`");
    let region = diagnostic.region.as_ref().expect("the refusal is located");
    let start = usize::try_from(region.start()).unwrap();
    let end = usize::try_from(region.end()).unwrap();
    assert_eq!(
        non_boolean_unit.get(start..end),
        Some("Int"),
        "the refusal points at the non-Boolean result type"
    );
}

/// TC-1308 rows 11 and 16: query records retain their checked declaration,
/// binder/type/unit selections, and replay of the empty identities is
/// executed below (FR-046-AC-4, FR-513-AC-5).
#[trace("TC-1308", "FR-046-AC-4", "FR-513-AC-5")]
#[test]
fn empty_collection_queries_return_their_defined_identity_on_the_spine() {
    let unit = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "model Config = \"example/config-version\";\n",
        "dimension Length; unit meter: Length = rational(1, 1);\n",
        "function querySize using v(s: Sequence<Int[0, 9]>[0, 3]): Integer pure { size(s) }\n",
        "function queryContains using v(s: Sequence<Int[0, 9]>[0, 3]): Boolean pure { contains(s, 1) }\n",
        "function queryForall using v(s: Sequence<Int[0, 9]>[0, 3]): Boolean pure { forall(x in s: x = 1) }\n",
        "function queryExists using v(s: Sequence<Int[0, 9]>[0, 3]): Boolean pure { exists(x in s: x = 1) }\n",
        "function queryFilter using v(s: Sequence<Int[0, 9]>[0, 3]): Sequence<Int[0, 9]>[0, 3] pure { filter(x in s: x = 1) }\n",
        "function queryMap using v(s: Sequence<Int[0, 9]>[0, 3]): Sequence<Int[0, 9]>[0, 3] pure { map(x in s: x) }\n",
        "function queryCount using v(s: Sequence<Int[0, 9]>[0, 3]): Integer pure { count<Config::VersionNumber>(x in s: x = 1) }\n",
        "function querySum using v(s: Sequence<Int[0, 9]>[0, 3]): Integer pure { sum<Config::VersionNumber>(x in s: x) }\n",
        "function queryUnitMap using v(s: Sequence<Quantity<meter>>[0, 3]): Sequence<Quantity<meter>>[0, 3] pure { map(x in s: x) }\n",
    );
    let packages = qsl_semantics::model::intake::package_input([
        crate::support::config_version_spine::spine::DOMAIN_PACKAGE.as_bytes(),
    ])
    .expect("ConfigVersion domain package");
    let sequence_type = quire_exact::CollectionType::new(
        quire_exact::CollectionKind::Sequence,
        quire_exact::ValueType::Int(
            quire_exact::IntegerInterval::new(0_i64.into(), 9_i64.into()).unwrap(),
        ),
        Some(quire_exact::CardinalityBound::new(0, 3).unwrap()),
    );
    let empty = quire_exact::from_admitted(sequence_type, Vec::new());
    let source = SourceIdentity::new("agent-ix", "test:tc-1308-empty-query");
    let cancel = Cancel::new();
    let limits = qsl_replay::spine::SpineLimits::default();
    let failure = |failure| {
        qsl_replay::spine::refusal_or_fault(failure).expect("no internal front-end fault")
    };
    let parsed = qsl_replay::spine::parse(
        &qsl_replay::spine::ParseRequest {
            source: &source,
            path: "empty-queries.native",
            bytes: unit.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(failure)
    .expect("the source parses")
    .into_value();
    let models = qsl_replay::spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(failure)
        .expect("the model selects")
        .into_value();
    let checked = qsl_replay::spine::check(
        &parsed,
        crate::TEST_PACKAGE_NAME.to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(failure)
    .expect("the checked package admits all query functions")
    .into_value();

    let emitted = qsl_package::emit_checked(checked.package(), qsl_package::EmitLimits::fixture())
        .expect("query package emits");
    let wire: serde_json::Value =
        serde_json::from_slice(emitted.package().bytes()).expect("emitted query package JSON");
    let nodes = wire["semantic_graph"]["nodes"]
        .as_array()
        .expect("emitted semantic nodes");
    let query_names = [
        "querySize",
        "queryContains",
        "queryForall",
        "queryExists",
        "queryFilter",
        "queryMap",
        "queryCount",
        "querySum",
        "queryUnitMap",
    ];
    let query_declarations: Vec<_> = nodes
        .iter()
        .filter(|node| {
            node["declaration"]["qualified_name"]
                .as_array()
                .and_then(|name| name.first())
                .and_then(serde_json::Value::as_str)
                .is_some_and(|name| query_names.contains(&name))
        })
        .collect();
    assert_eq!(query_declarations.len(), query_names.len());
    let declaration_of = |name: &str| {
        query_declarations
            .iter()
            .find(|node| node["declaration"]["qualified_name"][0] == name)
            .copied()
            .unwrap_or_else(|| panic!("{name} declaration is emitted"))
    };
    let node_ids: std::collections::BTreeSet<String> = query_declarations
        .iter()
        .map(|declaration| declaration["node_id"].to_string())
        .collect();
    assert_eq!(
        node_ids.len(),
        query_names.len(),
        "every query keeps its own node identity"
    );
    assert_eq!(
        declaration_of("queryContains")["semantic_type"],
        declaration_of("queryForall")["semantic_type"],
        "both Boolean queries select the same result type"
    );
    assert_ne!(
        declaration_of("queryContains")["semantic_type"],
        declaration_of("querySize")["semantic_type"],
        "a Boolean query and an Integer query select different result types"
    );
    assert!(
        has_string_leaf(&wire, "meter"),
        "the selected unit `meter` survives emission as its own value"
    );
    assert!(
        has_string_leaf(declaration_of("queryMap"), "x"),
        "the map query's own declaration references its binder `x`"
    );

    for (function, expected) in [
        ("querySize", ExpectedValue::Integer(0)),
        ("queryContains", ExpectedValue::Boolean(false)),
        ("queryForall", ExpectedValue::Boolean(true)),
        ("queryExists", ExpectedValue::Boolean(false)),
        ("queryFilter", ExpectedValue::EmptySequence),
        ("queryMap", ExpectedValue::EmptySequence),
        ("queryCount", ExpectedValue::Integer(0)),
        ("querySum", ExpectedValue::Integer(0)),
    ] {
        let mut meter = quire_exact::Meter::new(default_accounting(1_000_000));
        let evaluation = qsl_eval::value::CheckedPackageEvaluation::call(
            checked.package(),
            &qsl_eval::value::QualifiedName::unqualified(function).unwrap(),
            vec![empty.clone()],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .unwrap_or_else(|refusal| panic!("{function} refuses: {refusal:?}"));
        let quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(actual)) =
            evaluation.outcome
        else {
            panic!("{function} did not complete: {:?}", evaluation.outcome);
        };
        match (actual, expected) {
            (quire_exact::Value::Boolean(actual), ExpectedValue::Boolean(expected)) => {
                assert_eq!(actual, expected, "{function}");
            }
            (quire_exact::Value::Integer(actual), ExpectedValue::Integer(expected)) => {
                assert_eq!(actual, expected.into(), "{function}");
            }
            (quire_exact::Value::Collection(actual), ExpectedValue::EmptySequence) => {
                assert!(actual.elements().is_empty(), "{function}: {actual:?}")
            }
            (actual, expected) => {
                panic!("{function}: unexpected result {actual:?}; expected {expected:?}")
            }
        }
    }
}

/// TC-1308 row 15: a nested filter that expands a predicate charges work and
/// retained output on the checked package, and a fresh meter reproduces the
/// completed immutable result.
#[trace("TC-1308", "FR-046-AC-7", "FR-513-AC-6")]
#[test]
fn nested_predicate_query_stops_at_work_and_result_boundaries_on_the_spine() {
    use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
    use quire_exact::{
        CollectionKind, CollectionType, Integer, IntegerInterval, LimitKind, Meter, Value,
        ValueType,
    };

    let unit = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }\n",
        "function nested using v(groups: Sequence<Sequence<Int[0, 9]>[0, 3]>[0, 3]): Sequence<Sequence<Int[0, 9]>[0, 3]>[0, 3] pure { map(group in groups: filter(item in group: Positive(item))) }\n",
    );
    let source = SourceIdentity::new("agent-ix", "test:tc-1308-query-budgets");
    let cancel = Cancel::new();
    let limits = qsl_replay::spine::SpineLimits::default();
    let packages = BTreeMap::new();
    let failure = |failure| {
        qsl_replay::spine::refusal_or_fault(failure).expect("no internal front-end fault")
    };
    let parsed = qsl_replay::spine::parse(
        &qsl_replay::spine::ParseRequest {
            source: &source,
            path: "query-budgets.native",
            bytes: unit.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(failure)
    .expect("source parses")
    .into_value();
    let models = qsl_replay::spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(failure)
        .expect("no models are selected")
        .into_value();
    let checked = qsl_replay::spine::check(
        &parsed,
        crate::TEST_PACKAGE_NAME.to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(failure)
    .expect("nested query checks")
    .into_value();

    let inner_type = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Int(IntegerInterval::new(0_i64.into(), 9_i64.into()).unwrap()),
        Some(quire_exact::CardinalityBound::new(0, 3).unwrap()),
    );
    let outer_type = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::collection(inner_type.clone()),
        Some(quire_exact::CardinalityBound::new(0, 3).unwrap()),
    );
    let input = quire_exact::from_admitted(
        outer_type,
        vec![
            quire_exact::from_admitted(
                inner_type.clone(),
                vec![
                    Value::Integer(Integer::from(1_i64)),
                    Value::Integer(Integer::from(0_i64)),
                    Value::Integer(Integer::from(2_i64)),
                ],
            ),
            quire_exact::from_admitted(inner_type, vec![Value::Integer(Integer::from(3_i64))]),
        ],
    );
    let original_input = format!("{input:?}");
    let accounting = |work_units, result_units| quire_exact::ScalarLimits {
        work_units,
        result_units,
        ..default_accounting(u64::MAX)
    };
    let evaluate = |work_units, result_units| {
        let mut meter = Meter::new(accounting(work_units, result_units));
        let evaluation = checked
            .package()
            .call(
                &QualifiedName::unqualified("nested").unwrap(),
                vec![input.clone()],
                &ObjectEnvironment::default(),
                &mut meter,
            )
            .expect("nested query evaluation has no call refusal");
        (evaluation.outcome, meter)
    };
    let (quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(expected)), measured) =
        evaluate(u64::MAX, u64::MAX)
    else {
        panic!("unlimited nested query completes");
    };
    let work = measured.consumed(LimitKind::WorkUnits);
    let results = measured.consumed(LimitKind::ResultUnits);
    assert!(
        work > 0 && results > 0,
        "measured work={work}, results={results}"
    );

    for bound in [work, work - 1, 0] {
        let (outcome, _) = evaluate(bound, u64::MAX);
        if bound == work {
            assert!(matches!(
                outcome,
                quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(_))
            ));
        } else {
            let quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Incomplete(incomplete)) =
                outcome
            else {
                panic!("work bound {bound} should stop incompletely: {outcome:?}");
            };
            assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
        }
    }
    for bound in [results, results - 1, 0] {
        let (outcome, _) = evaluate(u64::MAX, bound);
        if bound == results {
            assert!(matches!(
                outcome,
                quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(_))
            ));
        } else {
            let quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Incomplete(incomplete)) =
                outcome
            else {
                panic!("result bound {bound} should stop incompletely: {outcome:?}");
            };
            assert_eq!(incomplete.limit_kind, LimitKind::ResultUnits);
        }
    }
    let (quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(retried)), _) =
        evaluate(u64::MAX, u64::MAX)
    else {
        panic!("fresh meter retry completes");
    };
    assert_eq!(format!("{retried:?}"), format!("{expected:?}"));
    assert_eq!(format!("{input:?}"), original_input);
}

/// TC-1308 row 14: call admission rejects wrong element kinds before query
/// execution, while maximum-length and nested sequences retain duplicates
/// and occurrence order.
#[trace("TC-1308", "FR-046-AC-4", "FR-513-AC-5")]
#[test]
fn query_inputs_reject_before_evaluation_and_keep_nested_occurrence_order() {
    use num_traits::ToPrimitive;
    use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
    use quire_exact::{
        CollectionKind, CollectionType, Integer, IntegerInterval, Meter, Value, ValueType,
    };

    let unit = concat!(
        "language \"ix:native\" edition \"1-draft\";\n",
        "profile v = \"quire.value.complete/v1\";\n",
        "function preserve using v(s: Sequence<Int[0, 9]>[0, 3]): Sequence<Int[0, 9]>[0, 3] pure { map(item in s: item) }\n",
        "function preserveNested using v(s: Sequence<Sequence<Int[0, 9]>[0, 3]>[0, 3]): Sequence<Sequence<Int[0, 9]>[0, 3]>[0, 3] pure { map(group in s: map(item in group: item)) }\n",
    );
    let source = SourceIdentity::new("agent-ix", "test:tc-1308-query-inputs");
    let cancel = Cancel::new();
    let limits = qsl_replay::spine::SpineLimits::default();
    let packages = BTreeMap::new();
    let failure = |failure| {
        qsl_replay::spine::refusal_or_fault(failure).expect("no internal front-end fault")
    };
    let parsed = qsl_replay::spine::parse(
        &qsl_replay::spine::ParseRequest {
            source: &source,
            path: "query-inputs.native",
            bytes: unit.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(failure)
    .expect("source parses")
    .into_value();
    let models = qsl_replay::spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(failure)
        .expect("no models are selected")
        .into_value();
    let checked = qsl_replay::spine::check(
        &parsed,
        crate::TEST_PACKAGE_NAME.to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(failure)
    .expect("query input functions check")
    .into_value();

    let int_type = ValueType::Int(IntegerInterval::new(0_i64.into(), 9_i64.into()).unwrap());
    let sequence_type = CollectionType::new(
        CollectionKind::Sequence,
        int_type.clone(),
        Some(quire_exact::CardinalityBound::new(0, 3).unwrap()),
    );
    let input = quire_exact::from_admitted(
        sequence_type.clone(),
        [1_i64, 1, 2]
            .into_iter()
            .map(|value| Value::Integer(Integer::from(value)))
            .collect(),
    );
    let evaluate = |function: &str, value: Value| {
        let mut meter = Meter::new(default_accounting(1_000_000));
        CheckedPackageEvaluation::call(
            checked.package(),
            &QualifiedName::unqualified(function).unwrap(),
            vec![value],
            &ObjectEnvironment::default(),
            &mut meter,
        )
    };
    let outcome = evaluate("preserve", input).expect("maximum length with duplicates admits");
    let quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Collection(
        values,
    ))) = outcome.outcome
    else {
        panic!("the maximum sequence completes: {:?}", outcome.outcome);
    };
    let integer_values = |values: &[Value]| {
        values
            .iter()
            .map(|value| match value {
                Value::Integer(value) => value.as_big().to_i64().expect("small test integer"),
                other => panic!("integer query occurrence, got {other:?}"),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(integer_values(values.elements()), [1, 1, 2]);

    let wrong_element_kind = quire_exact::from_admitted(
        CollectionType::new(
            CollectionKind::Sequence,
            ValueType::Boolean,
            Some(quire_exact::CardinalityBound::new(0, 3).unwrap()),
        ),
        vec![Value::Boolean(false)],
    );
    assert!(matches!(
        evaluate("preserve", wrong_element_kind),
        Err(quire_eval::CallFailure::Input(
            quire_semantic_value::call::InputRefusal::WrongValueKind { parameter: 0 }
        ))
    ));

    let wider_sequence_type = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Int(IntegerInterval::new(0_i64.into(), 10_i64.into()).unwrap()),
        Some(quire_exact::CardinalityBound::new(0, 3).unwrap()),
    );
    let invalid_last_value = quire_exact::from_admitted(
        wider_sequence_type,
        [1_i64, 1, 10]
            .into_iter()
            .map(|value| Value::Integer(Integer::from(value)))
            .collect(),
    );
    assert!(
        !ValueType::collection(sequence_type).admits(&invalid_last_value),
        "the final occurrence is outside the declared scalar bound"
    );
    assert!(
        matches!(
            evaluate("preserve", invalid_last_value),
            Err(quire_eval::CallFailure::Input(
                quire_semantic_value::call::InputRefusal::WrongValueKind { parameter: 0 }
            ))
        ),
        "the invalid final element is refused before any query result"
    );

    let inner_type = CollectionType::new(
        CollectionKind::Sequence,
        int_type,
        Some(quire_exact::CardinalityBound::new(0, 3).unwrap()),
    );
    let outer_type = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::collection(inner_type.clone()),
        Some(quire_exact::CardinalityBound::new(0, 3).unwrap()),
    );
    let nested_input = quire_exact::from_admitted(
        outer_type,
        vec![
            quire_exact::from_admitted(
                inner_type.clone(),
                vec![
                    Value::Integer(Integer::from(1_i64)),
                    Value::Integer(Integer::from(1_i64)),
                ],
            ),
            quire_exact::from_admitted(inner_type, vec![Value::Integer(Integer::from(2_i64))]),
        ],
    );
    let outcome = evaluate("preserveNested", nested_input).expect("nested input admits");
    let quire_eval::FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Collection(
        groups,
    ))) = outcome.outcome
    else {
        panic!("nested query completes: {:?}", outcome.outcome);
    };
    let group_values: Vec<Vec<i64>> = groups
        .elements()
        .iter()
        .map(|group| match group {
            Value::Collection(values) => integer_values(values.elements()),
            other => panic!("nested collection, got {other:?}"),
        })
        .collect();
    assert_eq!(group_values, [vec![1, 1], vec![2]]);
}

#[derive(Clone, Copy, Debug)]
enum ExpectedValue {
    Boolean(bool),
    Integer(i64),
    EmptySequence,
}

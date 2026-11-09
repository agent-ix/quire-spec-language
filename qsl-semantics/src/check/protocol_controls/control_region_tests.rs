//! QSL-1678: S3 checking charges a protocol only for its own control region.
//!
//! The composed binder once proved this with a read-count ceiling over 200
//! protocols. The checked route proves it with the `s3.nodes` and
//! `s3.work_units` it spends: a protocol's charge is a function of that
//! protocol alone, so the same protocol costs the same beside any neighbour
//! and a package of `k` equal protocols costs `k` times one of them. A check
//! that rescanned the package's other controls per protocol would make the
//! cost of a protocol depend on its neighbours and the total grow
//! quadratically.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use quire_semantic_value::checking::CheckingLimits;

use crate::check::{
    CheckCause, CheckRefusal, CheckedGraph, CheckingLimitKind, PackageDeclarations,
};

mod fixture {
    use crate as semantics;
    include!("../../../tests/fixtures/protocol_obligations.rs");
}
use fixture::{obligation_model, HEADER};

/// A unit cannot be empty, so every package opens with the protocol `Base`
/// of no checks; then one protocol per `(name, check count)`, each running
/// `count` checks in one sequence and finishing.
fn package(protocols: &[(&str, usize)]) -> String {
    let mut text = HEADER.to_owned();
    for (name, checks) in [("Base", 0)].iter().chain(protocols) {
        let body: String = (0..*checks)
            .map(|index| format!("check C{index} using v {{ true }}; "))
            .collect();
        text.push_str(&format!(
            "protocol {name} using p over (view: M::Node) on origin {{ \
             role Service on M::Node; run sequence Main {{ {body} }} \
             finish Closed as (closed: M::Receipt) {{ true }}; }}\n"
        ));
    }
    text
}

fn check(text: &str, limits: CheckingLimits) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        vec![obligation_model()],
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("{text}: the assembler refuses: {refusal:?}"))
    .check(limits, quire_semantic_value::checking::CheckMode::Linked)
}

/// Builds the limits that cap one dimension at a value.
type Cap = fn(u64) -> CheckingLimits;

const NODES: Cap = CheckingLimits::new;
const WORK: Cap = |units| CheckingLimits::default().with_work_budget(units);

/// The smallest value of the limit `make` builds under which `text` checks,
/// and the proof that one below it refuses on `kind` at that bound.
fn exact(text: &str, make: Cap, kind: CheckingLimitKind) -> u64 {
    let (mut refused, mut admitted) = (None, 1_u64 << 20);
    check(text, make(admitted)).expect("the generous limit admits the package");
    if check(text, make(0)).is_ok() {
        admitted = 0;
    }
    while refused.map_or(admitted > 0, |refused| admitted - refused > 1) {
        let middle = refused.map_or(admitted / 2, |refused| refused + (admitted - refused) / 2);
        if check(text, make(middle)).is_ok() {
            admitted = middle;
        } else {
            refused = Some(middle);
        }
    }
    if let Some(below) = admitted.checked_sub(1) {
        let refusals = check(text, make(below)).expect_err("one below the exact limit refuses");
        let [refusal] = refusals.as_slice() else {
            panic!("one limit refusal, got {refusals:?}");
        };
        assert!(
            matches!(&refusal.cause, CheckCause::ResourceExhausted(limit)
                if limit.kind == kind && limit.limit == below),
            "{refusal:?}"
        );
    }
    admitted
}

const KINDS: [(Cap, CheckingLimitKind); 2] = [
    (NODES, CheckingLimitKind::Nodes),
    (WORK, CheckingLimitKind::WorkBudget),
];

/// A protocol's charge does not depend on the controls of the
/// protocols beside it: a 60-check neighbour adds nothing to a 3-check
/// protocol's cost, and the 60-check protocol costs the same with or without
/// it.
#[trace("TC-1172", "FR-218-AC-36")]
#[test]
fn a_protocol_is_charged_for_its_own_controls_whatever_its_neighbours() {
    for (make, kind) in KINDS {
        let cost = |protocols: &[(&str, usize)]| exact(&package(protocols), make, kind);
        let (empty, light, heavy) = (cost(&[]), cost(&[("Light", 3)]), cost(&[("Heavy", 60)]));
        assert!(light > empty && heavy > light, "{kind:?}");
        assert_eq!(
            cost(&[("Heavy", 60), ("Light", 3)]),
            heavy + (light - empty),
            "{kind:?}: the light protocol's charge moves with its neighbour"
        );
        assert_eq!(
            cost(&[("Light", 3), ("Heavy", 60)]),
            heavy + (light - empty),
            "{kind:?}: the order of declarations changes a charge"
        );
    }
}

/// `k` equal protocols cost `k` times one protocol above the empty unit's
/// charge: linear, not quadratic, in the package's control count.
#[trace("TC-1172", "FR-218-AC-36")]
#[test]
fn a_package_of_equal_protocols_is_charged_linearly() {
    let many = |count: usize| {
        let names: Vec<String> = (0..count).map(|index| format!("Flow{index}")).collect();
        let protocols: Vec<(&str, usize)> = names.iter().map(|name| (name.as_str(), 20)).collect();
        package(&protocols)
    };
    for (make, kind) in KINDS {
        let empty = exact(&package(&[]), make, kind);
        let one = exact(&many(1), make, kind) - empty;
        for count in [2, 8] {
            assert_eq!(
                exact(&many(count), make, kind) - empty,
                one * count as u64,
                "{kind:?}: {count} protocols"
            );
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Qualified bounded-event keys, real monitor debt and terminal closure.

use ix_trace_rs::trace;
use qsl_eval::automaton::{
    translate_body, BoundedState, BoundedTranslation, ContextBinding, Letter, Monitor,
    QualifiedBoundedMonitor, StateContext,
};
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{Origin, Role, Value};

use crate::temporal_trace::{fixture, EVENT};

fn context(clause: &qsl_semantics::check::CheckedTemporalClause, ordinal: u64) -> StateContext {
    StateContext::clause(
        clause,
        OccurrenceKey::new(
            crate::support::ids::node_key([0xab]),
            Origin::new(Role::new("temporal"), ordinal),
        ),
        Vec::new(),
    )
}

#[trace("TC-636", "FR-201-AC-13", "FR-338-AC-6")]
#[test]
fn qualified_bounded_monitor_keys_follow_real_debt_and_ignore_allocation_order() {
    let fixture = fixture(
        EVENT,
        "c: Counter",
        "on origin",
        "eventually[0,2] holds(true)",
    );
    let context = context(&fixture.clause, 0);
    let mut first = QualifiedBoundedMonitor::new(fixture.clause.formula(), &context, 100).unwrap();
    let mut second = QualifiedBoundedMonitor::new(fixture.clause.formula(), &context, 100).unwrap();
    let origin = first.start(false).unwrap();
    let accepted = second.start(true).unwrap();
    let reordered_origin = second.start(false).unwrap();
    assert_ne!(
        origin, reordered_origin,
        "fixture must change local allocation order"
    );
    assert_eq!(
        first.key(origin).unwrap(),
        second.key(reordered_origin).unwrap()
    );
    assert_eq!(
        first.state(origin).unwrap(),
        BoundedState::Open { remaining: 2 }
    );
    assert_eq!(first.rank(origin).unwrap(), 2);
    let loss = first.step(origin, false).unwrap();
    assert_eq!(first.rank(loss).unwrap(), 1);
    assert_ne!(first.key(origin).unwrap(), first.key(loss).unwrap());
    let delivered = first.step(loss, true).unwrap();
    assert_eq!(first.state(delivered).unwrap(), BoundedState::Accept);
    assert_eq!(first.rank(delivered).unwrap(), 0);
    assert_eq!(
        first.key(delivered).unwrap(),
        first.decision_key(true).unwrap()
    );
    assert_eq!(first.key(delivered).unwrap(), second.key(accepted).unwrap());
    let owner = BoundedTranslation::clause(&fixture.clause, &context).unwrap();
    assert_eq!(
        first.key(origin).unwrap(),
        owner.key(BoundedState::Open { remaining: 2 }).unwrap()
    );
    assert_eq!(
        owner.admit(first.key(loss).unwrap().as_bytes()).unwrap().1,
        BoundedState::Open { remaining: 1 }
    );
}

#[trace("TC-636", "FR-201-AC-13")]
#[test]
fn qualified_bounded_monitor_terminal_false_extension_and_zero_horizon() {
    for horizon in [0, 2] {
        let fixture = fixture(
            EVENT,
            "c: Counter",
            "on origin",
            &format!("eventually[0,{horizon}] holds(true)"),
        );
        let context = context(&fixture.clause, 0);
        let mut qualified =
            QualifiedBoundedMonitor::new(fixture.clause.formula(), &context, 100).unwrap();
        let pending = qualified.start(false).unwrap();
        let satisfied = qualified.start(true).unwrap();
        assert_eq!(qualified.terminal(pending).unwrap(), BoundedState::Reject);
        assert_eq!(qualified.terminal(satisfied).unwrap(), BoundedState::Accept);
        if horizon == 0 {
            assert_eq!(qualified.state(pending).unwrap(), BoundedState::Reject);
            assert_eq!(qualified.state(satisfied).unwrap(), BoundedState::Accept);
            assert_eq!(
                qualified.key(pending).unwrap(),
                qualified.decision_key(false).unwrap()
            );
        }
        let automaton =
            translate_body(fixture.clause.formula(), fixture.clause.profile(), 100).unwrap();
        let mut real = Monitor::bad_prefix(automaton, 100).unwrap();
        let initial = real.initial().unwrap();
        let pending = real.step(initial, Letter::default()).unwrap();
        let satisfied = real.step(initial, Letter::from_bits(1)).unwrap();
        assert!(real.false_extension_rejects(pending).unwrap());
        assert!(!real.false_extension_rejects(satisfied).unwrap());
    }
}

#[trace("TC-636", "FR-201-AC-14", "FR-338-AC-6")]
#[test]
fn qualified_bounded_monitor_preserves_context_and_refuses_other_forms() {
    let fixture = fixture(
        EVENT,
        "c: Counter",
        "on origin",
        "eventually[0,2] holds(true)",
    );
    let own = context(&fixture.clause, 0);
    let foreign = context(&fixture.clause, 1);
    let mut first = QualifiedBoundedMonitor::new(fixture.clause.formula(), &own, 100).unwrap();
    let mut second = QualifiedBoundedMonitor::new(fixture.clause.formula(), &foreign, 100).unwrap();
    let first_state = first.start(false).unwrap();
    let second_state = second.start(false).unwrap();
    let first_key = first.key(first_state).unwrap();
    assert_ne!(first_key, second.key(second_state).unwrap());
    let other_owner = BoundedTranslation::clause(&fixture.clause, &foreign).unwrap();
    assert_eq!(
        other_owner.admit(first_key.as_bytes()),
        Err(qsl_eval::automaton::KeyRefusal::ForeignContext)
    );
    let mut bound = own.clone();
    bound.binding.push(ContextBinding::value(
        crate::support::ids::node_key([0xcd]),
        Value::Boolean(true),
    ));
    let mut bound_monitor =
        QualifiedBoundedMonitor::new(fixture.clause.formula(), &bound, 100).unwrap();
    let bound_state = bound_monitor.start(false).unwrap();
    assert_ne!(first_key, bound_monitor.key(bound_state).unwrap());
    let bound_owner = BoundedTranslation::clause(&fixture.clause, &bound).unwrap();
    assert_eq!(
        bound_owner.admit(first_key.as_bytes()),
        Err(qsl_eval::automaton::KeyRefusal::ForeignContext)
    );
    let fixture =
        crate::temporal_trace::fixture(EVENT, "c: Counter", "on origin", "always[0,2] holds(true)");
    assert!(QualifiedBoundedMonitor::new(
        fixture.clause.formula(),
        &context(&fixture.clause, 0),
        100
    )
    .is_err());
}

#[trace("TC-636", "FR-201-AC-13", "FR-201-AC-14")]
#[test]
fn qualified_bounded_monitor_retains_the_producers_reached_handles() {
    let fixture = fixture(
        EVENT,
        "c: Counter",
        "on origin",
        "eventually[0,2] holds(true)",
    );
    let automaton =
        translate_body(fixture.clause.formula(), fixture.clause.profile(), 100).unwrap();
    let mut monitor = Monitor::bad_prefix(automaton, 100).unwrap();
    let initial = monitor.initial().unwrap();
    let origin = monitor.step(initial, Letter::default()).unwrap();
    let loss = monitor.step(origin, Letter::default()).unwrap();
    let qualified = QualifiedBoundedMonitor::from_monitor(
        fixture.clause.formula(),
        &context(&fixture.clause, 0),
        monitor,
        100,
    )
    .unwrap();
    assert_eq!(qualified.rank(origin).unwrap(), 2);
    assert_eq!(qualified.rank(loss).unwrap(), 1);
    assert_ne!(qualified.key(origin).unwrap(), qualified.key(loss).unwrap());
    let other = crate::temporal_trace::fixture(
        EVENT,
        "c: Counter",
        "on origin",
        "eventually[0,3] holds(true)",
    );
    let automaton = translate_body(other.clause.formula(), other.clause.profile(), 100).unwrap();
    assert!(QualifiedBoundedMonitor::from_monitor(
        fixture.clause.formula(),
        &context(&fixture.clause, 0),
        Monitor::bad_prefix(automaton, 100).unwrap(),
        100,
    )
    .is_err());
}

#[trace("TC-636", "FR-201-AC-13")]
#[test]
fn qualified_bounded_monitor_closes_a_full_u64_horizon_without_stutter_states() {
    let fixture = fixture(
        EVENT,
        "c: Counter",
        "on origin",
        &format!("eventually[0,{}] holds(true)", u64::MAX),
    );
    let automaton =
        translate_body(fixture.clause.formula(), fixture.clause.profile(), 100).unwrap();
    let mut monitor = Monitor::bad_prefix(automaton, 100).unwrap();
    let initial = monitor.initial().unwrap();
    let origin = monitor.step(initial, Letter::default()).unwrap();
    let count = monitor.state_count();
    assert_eq!(
        monitor.bounded_eventually_terminal(origin).unwrap(),
        Some(BoundedState::Reject)
    );
    assert_eq!(
        monitor.state_count(),
        count,
        "terminal closure creates no stutter states"
    );
}

#[trace("TC-525", "FR-338-AC-6", "FR-201-AC-14")]
#[test]
fn over_instance_keys_and_ordinary_reference_arguments_have_distinct_context_forms() {
    use crate::support::ids::{effective_id, node_key, universe_id};
    use quire_exact::{ObjectId, ObjectReference};
    let fixture = fixture(
        EVENT,
        "c: Counter",
        "on origin",
        "eventually[0,2] holds(true)",
    );
    let parameter = node_key([0xcd]);
    let reference = ObjectReference::new(
        universe_id([1]),
        effective_id([2]),
        ObjectId::new("c").unwrap(),
    );
    let mut instance = context(&fixture.clause, 0);
    instance.binding.push(ContextBinding::instance(
        parameter.clone(),
        reference.object().as_str(),
    ));
    let mut ordinary = context(&fixture.clause, 0);
    ordinary.binding.push(ContextBinding::value(
        parameter,
        Value::Reference(reference),
    ));
    let instance = BoundedTranslation::clause(&fixture.clause, &instance).unwrap();
    let ordinary = BoundedTranslation::clause(&fixture.clause, &ordinary).unwrap();
    let instance_key = instance.key(instance.start(false)).unwrap();
    let ordinary_key = ordinary.key(ordinary.start(false)).unwrap();
    assert_ne!(instance_key, ordinary_key);
    let instance_value: serde_json::Value =
        serde_json::from_slice(instance_key.as_bytes()).unwrap();
    let ordinary_value: serde_json::Value =
        serde_json::from_slice(ordinary_key.as_bytes()).unwrap();
    assert_eq!(
        instance_value[1][3][0][1],
        serde_json::json!(["instance", "c"])
    );
    assert_eq!(ordinary_value[1][3][0][1]["type"], "reference");
    assert_eq!(ordinary_value[1][3][0][1]["identity"], "c");
}

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
    bound.binding.push(ContextBinding {
        parameter: crate::support::ids::node_key([0xcd]),
        value: Value::Boolean(true),
    });
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

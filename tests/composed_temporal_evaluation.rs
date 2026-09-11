// SPDX-License-Identifier: AGPL-3.0-only
//! TC-122: bounded temporal truth under each selected profile.
//!
//! FR-043. Expected truths, bases and premises come from the authored source and
//! from the profile definitions under `resources/native-v1/proposals/`, never
//! from the evaluator's own output.

#[path = "support/temporal/mod.rs"]
mod setup;

use quire_spec_language::temporal::{self, Basis, Closure, Completeness, Truth};
use setup::{
    admitted, assessment, holds_leaf, origin, position, trace, EVENT_POSITION, FIXED_SAMPLE,
    TIMESTAMPED_WINDOW,
};

/// One closed-complete position at offset zero with every authored atom true. A
/// body spelt over a constant alone emits no atom and so values none.
fn one_true(
    package: &quire_spec_language::protocol_artifact::AdmittedPackage,
    at: usize,
) -> Vec<temporal::Position> {
    let valuations: Vec<(u32, bool)> = setup::holds_leaves(package, at)
        .into_iter()
        .map(|leaf| (leaf, true))
        .collect();
    vec![position(0, &valuations)]
}

/// FR-043-AC-1: `always[0,1] holds(p)` is false and `always[0,1] true` is true
/// on one closed-complete position, under both false-extension profiles. The
/// distinction survives a grouping node and an enclosing connective.
#[test]
fn closed_false_extension_separates_an_atom_from_a_constant() {
    for profile in [EVENT_POSITION, FIXED_SAMPLE] {
        for body in [
            "always[0,1] holds(view.ready)",
            "always[0,1] (holds(view.ready))",
            "true and always[0,1] holds(view.ready)",
        ] {
            admitted(&origin("Due", profile, body), |package, at| {
                let mut supplied = trace(profile, "orders");
                supplied.positions = one_true(package, at);
                let report =
                    temporal::evaluate(package, at, &supplied, temporal::Limits::default());
                let assessed = assessment(&report);
                assert_eq!(
                    assessed.truth,
                    Some(Truth::False),
                    "{profile} {body}: an atom is false at the out-of-scope offset",
                );
                assert_eq!(assessed.basis, Some(Basis::ClosedScope));
            });
        }
        for body in [
            "always[0,1] true",
            "always[0,1] (true)",
            "true and always[0,1] true",
        ] {
            admitted(&origin("Due", profile, body), |package, at| {
                let supplied = trace(profile, "orders");
                let report =
                    temporal::evaluate(package, at, &supplied, temporal::Limits::default());
                assert_eq!(
                    assessment(&report).truth,
                    Some(Truth::True),
                    "{profile} {body}: a constant keeps its authored value",
                );
            });
        }
    }
}

/// FR-043-AC-2: identical valuations and interval under the three profiles keep
/// three distinct meanings and three distinct result identities.
#[test]
fn three_profiles_keep_three_meanings_over_identical_valuations() {
    let body = "always[0,1] holds(view.ready)";
    let mut outcomes = Vec::new();
    for profile in [EVENT_POSITION, FIXED_SAMPLE, TIMESTAMPED_WINDOW] {
        admitted(&origin("Due", profile, body), |package, at| {
            let mut supplied = trace(profile, "orders");
            supplied.positions = one_true(package, at);
            supplied.watermark = 1;
            let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
            let assessed = assessment(&report);
            outcomes.push((
                assessed.truth,
                assessed.premises.profile,
                assessed.premises.clone(),
            ));
        });
    }
    assert_eq!(outcomes[0].0, Some(Truth::False), "event-position");
    assert_eq!(outcomes[1].0, Some(Truth::False), "fixed-sample");
    assert_eq!(
        outcomes[2].0,
        Some(Truth::True),
        "a complete finite window adds no synthetic atom after closure",
    );
    assert_ne!(outcomes[0].1, outcomes[1].1);
    assert_ne!(outcomes[1].1, outcomes[2].1);
    assert_ne!(outcomes[0].2, outcomes[2].2, "premises are distinct");
}

/// FR-043-AC-3: a mismatched profile identity, profile revision or clock binding
/// name refuses before any position is visited.
#[test]
fn a_mismatched_binding_refuses_before_any_position_is_visited() {
    /// Which dimension of the asserted binding this control mutates.
    fn mutate(supplied: &mut temporal::Trace, dimension: temporal::Dimension) {
        match dimension {
            temporal::Dimension::Profile => {
                supplied.clock.profile_identity = temporal::FIXED_SAMPLE.into();
            }
            temporal::Dimension::ProfileRevision => {
                supplied.clock.profile_revision = "1-draft.2".into();
            }
            temporal::Dimension::Clock => supplied.clock.name = "other".into(),
            other => panic!("no control for {other:?}"),
        }
    }

    let body = "always[0,1] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        for dimension in [
            temporal::Dimension::Profile,
            temporal::Dimension::ProfileRevision,
            temporal::Dimension::Clock,
        ] {
            let mut supplied = trace(EVENT_POSITION, "orders");
            supplied.positions = one_true(package, at);
            mutate(&mut supplied, dimension);
            let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
            let error = report.result().expect_err("a mismatched binding refuses");
            assert!(
                matches!(
                    error,
                    temporal::Error::Refused(temporal::Refusal::Binding { dimension: named, .. })
                        if *named == dimension
                ),
                "{dimension:?}: {error:?}",
            );
            assert_eq!(
                report.usage().positions,
                0,
                "{dimension:?}: the refusal precedes any position visit",
            );
        }
    });
}

/// FR-043-AC-6: the temporal Boolean connectives are pointwise. On one
/// closed-complete position with `p` true, `always[0,1] not holds(p)` is false
/// while `eventually[0,1] not holds(p)` is true.
///
/// This reading is the compiler's selection, not a restatement of an owned rule;
/// FR-043 records it for agent E's ruling.
#[test]
fn temporal_connectives_are_pointwise_not_untimed() {
    for (body, expected) in [
        ("always[0,1] not holds(view.ready)", Truth::False),
        ("eventually[0,1] not holds(view.ready)", Truth::True),
        ("always[0,1] not true", Truth::False),
    ] {
        admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
            let mut supplied = trace(EVENT_POSITION, "orders");
            supplied.positions = one_true(package, at);
            let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
            assert_eq!(assessment(&report).truth, Some(expected), "{body}");
        });
    }
}

/// FR-043-AC-7: an open scope returns pending with `unsettled` unless a decisive
/// witness or counterexample settles it; a missing fact inside the decision
/// support returns `unavailable` with no truth.
#[test]
fn open_scopes_settle_only_from_a_decisive_fact() {
    let body = "eventually[0,3] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        let leaf = holds_leaf(package, at);

        let mut witnessed = trace(EVENT_POSITION, "orders");
        witnessed.decision_scope = Closure::Open;
        witnessed.positions = vec![
            position(0, &[(leaf, false)]),
            position(1, &[(leaf, true)]),
            position(2, &[(leaf, false)]),
            position(3, &[(leaf, false)]),
        ];
        let report = temporal::evaluate(package, at, &witnessed, temporal::Limits::default());
        let assessed = assessment(&report);
        assert_eq!(assessed.truth, Some(Truth::True));
        assert_eq!(assessed.basis, Some(Basis::DecisiveWitness));
        assert_eq!(
            assessed.premises.decision_scope,
            Closure::Open,
            "settling does not close the scope",
        );

        let mut unsettled = trace(EVENT_POSITION, "orders");
        unsettled.decision_scope = Closure::Open;
        unsettled.positions = vec![position(0, &[(leaf, false)])];
        let report = temporal::evaluate(package, at, &unsettled, temporal::Limits::default());
        let assessed = assessment(&report);
        assert_eq!(assessed.truth, Some(Truth::Pending));
        assert_eq!(assessed.basis, Some(Basis::Unsettled));

        let mut unavailable = trace(EVENT_POSITION, "orders");
        unavailable.positions = vec![position(0, &[]), position(1, &[(leaf, true)])];
        let report = temporal::evaluate(package, at, &unavailable, temporal::Limits::default());
        let assessed = assessment(&report);
        assert_eq!(
            assessed.truth, None,
            "a missing required fact is not pending"
        );
        assert_eq!(assessed.basis, Some(Basis::Unavailable));
        assert!(assessed.incomplete.is_some());
    });
}

/// FR-043-AC-8: an incomplete input applies neither boundary rule, and an empty
/// finite window emits empty truth only under completeness through the upper
/// endpoint.
#[test]
fn an_incomplete_input_applies_no_boundary_rule() {
    let body = "always[0,1] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        let mut supplied = trace(EVENT_POSITION, "orders");
        supplied.positions = one_true(package, at);
        supplied.completeness = Completeness::Incomplete;
        let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        let assessed = assessment(&report);
        assert_eq!(
            assessed.truth,
            Some(Truth::Pending),
            "a scope labelled closed without complete observations extends nothing",
        );
    });

    for (watermark, completeness, expected) in [
        (1, Completeness::Complete, Some(Truth::True)),
        (0, Completeness::Complete, Some(Truth::Pending)),
        (1, Completeness::Incomplete, Some(Truth::Pending)),
    ] {
        admitted(&origin("Due", TIMESTAMPED_WINDOW, body), |package, at| {
            let mut supplied = trace(TIMESTAMPED_WINDOW, "orders");
            supplied.watermark = watermark;
            supplied.completeness = completeness;
            let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
            assert_eq!(
                assessment(&report).truth,
                expected,
                "empty universal window at watermark {watermark} under {completeness:?}",
            );
        });
    }
}

/// FR-043-AC-9: the same visible suffix yields a Boolean from an authoritative
/// origin and missing-history incomplete from a bare cutoff.
#[test]
fn a_history_cutoff_is_not_an_authoritative_origin() {
    let body = "historically[0,2] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        let leaf = holds_leaf(package, at);
        let positions = vec![position(0, &[(leaf, true)])];

        let mut authoritative = trace(EVENT_POSITION, "orders");
        authoritative.positions = positions.clone();
        let report = temporal::evaluate(package, at, &authoritative, temporal::Limits::default());
        assert_eq!(
            assessment(&report).truth,
            Some(Truth::False),
            "false extension applies past an authoritative origin",
        );

        let mut cutoff = trace(EVENT_POSITION, "orders");
        cutoff.positions = positions;
        cutoff.authoritative_origin = false;
        let report = temporal::evaluate(package, at, &cutoff, temporal::Limits::default());
        let assessed = assessment(&report);
        assert_eq!(assessed.truth, None);
        assert_eq!(
            assessed.incomplete.as_ref().map(|value| value.dimension),
            Some(temporal::Dimension::History),
        );
    });
}

/// FR-043-AC-13: identical inputs reproduce one result identity; a changed
/// premise produces a distinct one.
#[test]
fn a_changed_premise_produces_a_distinct_result_identity() {
    let body = "always[0,1] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        let mut supplied = trace(EVENT_POSITION, "orders");
        supplied.positions = one_true(package, at);
        let first = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        let second = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
        assert_eq!(
            assessment(&first).premises,
            assessment(&second).premises,
            "an unchanged configuration reproduces one identity",
        );

        let mut changed = supplied.clone();
        changed
            .clock
            .parameters
            .insert("sequence-authority".into(), "other".into());
        let report = temporal::evaluate(package, at, &changed, temporal::Limits::default());
        assert_ne!(
            assessment(&first).premises,
            assessment(&report).premises,
            "a changed declared clock parameter changes result identity",
        );

        let lowered = temporal::Limits {
            visits: 900_000,
            ..temporal::Limits::default()
        };
        let report = temporal::evaluate(package, at, &supplied, lowered);
        assert_ne!(
            assessment(&first).premises,
            assessment(&report).premises,
            "a changed ceiling changes result identity",
        );
    });
}

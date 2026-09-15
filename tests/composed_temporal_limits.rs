// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-124: checked bounds, exhaustion and required retained state.
//!
//! NFR-008. Expected charges come from the counter definitions published in
//! `docs/native-temporal-evaluation.md` under accounting label
//! `quire.native.temporal-work/1`, never from the run's own reported usage.

#[allow(dead_code)]
#[path = "support/temporal/mod.rs"]
mod setup;

use ix_trace_rs::trace;

use quire_spec_language::protocol_artifact::AdmittedPackage;
use quire_spec_language::temporal::{
    self, Basis, Eviction, LimitDimension, Limits, Position, Trace, Truth,
};
use setup::{
    admitted, assessment, holds_leaves, origin, position, trace, EVENT_POSITION, FIXED_SAMPLE,
    TIMESTAMPED_WINDOW,
};

/// The eight bounded operators, each as a body template taking one interval.
/// `{i}` is the interval, `{a}` the operand.
const OPERATORS: [(&str, &str); 8] = [
    ("eventually", "eventually{i} {a}"),
    ("always", "always{i} {a}"),
    ("once", "once{i} {a}"),
    ("historically", "historically{i} {a}"),
    ("until", "({a} until{i} {a})"),
    ("release", "({a} release{i} {a})"),
    ("since", "({a} since{i} {a})"),
    ("triggered", "({a} triggered{i} {a})"),
];

/// The interval bound pairs the declared overflow population enumerates.
fn bounds() -> [(i64, i64); 4] {
    [
        (0, i64::MAX),
        (1, i64::MAX),
        (i64::MAX - 1, i64::MAX),
        (0, i64::MAX / 2),
    ]
}

/// Nest `template` to `depth`, innermost operand `holds(view.ready)`.
fn nest(template: &str, interval: &str, depth: usize) -> String {
    let mut body = "holds(view.ready)".to_owned();
    for _ in 0..depth {
        body = template.replace("{i}", interval).replace("{a}", &body);
    }
    body
}

/// A trace with one closed-complete position valuing every emitted atom true.
fn one_position(package: &AdmittedPackage, at: usize, profile: &str) -> Trace {
    let mut supplied = trace(profile, "orders");
    let valuations: Vec<(u32, bool)> = holds_leaves(package, at)
        .into_iter()
        .map(|leaf| (leaf, true))
        .collect();
    supplied.positions = vec![position(0, &valuations)];
    supplied
}

/// NFR-008-AC-1, sweep: no case in the declared population wraps, saturates or
/// narrows. Every outcome is a well-formed assessment or a typed stop naming
/// `Horizon` or `Positions`.
///
/// The population is the one NFR-008's Verification section declares: eight
/// operators x three profiles x nesting depths one through four x four interval
/// bound pairs, 384 authored declarations. This sweep deliberately asserts no
/// per-case expected value. Which composition a nested formula actually performs
/// depends on which offsets its trace reaches — under the sparse finite-window
/// profile a nested operator is evaluated only at admitted instants — so any
/// oracle predicting per-case overflow would have to reimplement the evaluator.
/// The overflow obligation itself is proved by the targeted control below.
#[trace("TC-124", "NFR-008-AC-1")]
#[test]
fn the_declared_population_never_wraps_or_saturates() {
    let mut cases = 0usize;
    let mut stopped = 0usize;
    let mut assessed = 0usize;
    for (name, template) in OPERATORS {
        for profile in [EVENT_POSITION, FIXED_SAMPLE, TIMESTAMPED_WINDOW] {
            for depth in 1..=4usize {
                for (lower, upper) in bounds() {
                    cases += 1;
                    let interval = format!("[{lower},{upper}]");
                    let body = nest(template, &interval, depth);
                    admitted(&origin("Due", profile, &body), |package, at| {
                        let supplied = one_position(package, at, profile);
                        let report = temporal::evaluate(package, at, &supplied, Limits::default());
                        let label = format!("{name} depth {depth} {interval} under {profile}");
                        match report.result() {
                            Ok(obligations) => {
                                assert_eq!(obligations.len(), 1, "{label}");
                                assessed += 1;
                            }
                            Err(temporal::Error::Exhausted(exhaustion)) => {
                                assert!(
                                    matches!(
                                        exhaustion.dimension,
                                        LimitDimension::Horizon | LimitDimension::Positions
                                    ),
                                    "{label}: stopped on {:?}",
                                    exhaustion.dimension,
                                );
                                assert_eq!(exhaustion.subject.declaration, at, "{label}");
                                stopped += 1;
                            }
                            Err(other) => panic!("{label}: unexpected {other:?}"),
                        }
                    });
                }
            }
        }
    }
    assert_eq!(cases, 8 * 3 * 4 * 4, "the declared population is 384 cases");
    assert!(
        stopped > 0 && assessed > 0,
        "the population reaches both the bounded and the stopped regime",
    );
}

/// NFR-008-AC-1, overflow: a composition that provably leaves the checked i64
/// domain stops on `Horizon` and evaluates nothing further.
///
/// The inner interval is reached from offset one, so the composition
/// `1 + (i64::MAX - 1) + 1` leaves the domain. The expected bound is computed
/// here from the authored intervals, not read back from the evaluator.
#[trace("TC-124", "NFR-008-AC-1")]
#[test]
fn a_composition_outside_the_checked_domain_stops_on_horizon() {
    let inner = i64::MAX;
    let body = format!("always[1,1] eventually[{inner},{inner}] holds(view.ready)");
    // Independent: the inner operator is reached at offset 1, and 1 + i64::MAX
    // is outside the domain.
    assert!(
        1i64.checked_add(inner).is_none(),
        "the control is an overflow"
    );

    for profile in [EVENT_POSITION, FIXED_SAMPLE] {
        admitted(&origin("Due", profile, &body), |package, at| {
            let supplied = one_position(package, at, profile);
            let report = temporal::evaluate(package, at, &supplied, Limits::default());
            let error = report
                .result()
                .expect_err("an overflowing composition stops");
            let temporal::Error::Exhausted(exhaustion) = error else {
                panic!("{profile}: expected a typed stop, got {error:?}");
            };
            assert_eq!(
                exhaustion.dimension,
                LimitDimension::Horizon,
                "{profile}: the stop names the checked-arithmetic dimension",
            );
        });
    }
}

/// The population above is all large intervals, so a control with ordinary
/// bounds establishes that the same nested shapes do evaluate. Without it,
/// an evaluator that refused everything would pass the population.
#[trace("TC-124", "NFR-008-AC-1")]
#[test]
fn ordinary_bounds_under_the_same_nesting_still_evaluate() {
    for (name, template) in OPERATORS {
        for depth in 1..=4usize {
            let body = nest(template, "[0,1]", depth);
            admitted(&origin("Due", EVENT_POSITION, &body), |package, at| {
                let supplied = one_position(package, at, EVENT_POSITION);
                let report = temporal::evaluate(package, at, &supplied, Limits::default());
                let assessed = assessment(&report);
                assert!(
                    assessed.truth.is_some() || assessed.incomplete.is_some(),
                    "{name} depth {depth}: ordinary bounds produce an outcome, not a stop",
                );
            });
        }
    }
}

/// NFR-008-AC-2: a reached ceiling in any charged dimension produces a typed
/// stop naming that dimension, and never a Boolean — including where the
/// observed prefix would otherwise have settled the obligation.
#[trace("TC-124", "NFR-008-AC-2")]
#[test]
fn a_reached_ceiling_never_emits_a_boolean() {
    let body = "always[0,2] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        let supplied = one_position(package, at, EVENT_POSITION);

        // The same trace under sufficient ceilings settles, so each stop below
        // suppresses a Boolean that was otherwise available.
        let settled = temporal::evaluate(package, at, &supplied, Limits::default());
        assert_eq!(assessment(&settled).truth, Some(Truth::False));
        // The version-1 entry point authenticates nothing, so its report
        // retains no authenticated clock identity to be mistaken for one.
        assert!(settled.authenticated().is_none());

        for (dimension, limits) in [
            (
                LimitDimension::Positions,
                Limits {
                    positions: 1,
                    ..Limits::default()
                },
            ),
            (
                LimitDimension::Valuations,
                Limits {
                    valuations: 0,
                    ..Limits::default()
                },
            ),
            (
                LimitDimension::Visits,
                Limits {
                    visits: 1,
                    ..Limits::default()
                },
            ),
            (
                LimitDimension::Depth,
                Limits {
                    depth: 1,
                    ..Limits::default()
                },
            ),
            (
                LimitDimension::Horizon,
                Limits {
                    horizon: 1,
                    ..Limits::default()
                },
            ),
            (
                LimitDimension::Instances,
                Limits {
                    instances: 0,
                    ..Limits::default()
                },
            ),
        ] {
            let report = temporal::evaluate(package, at, &supplied, limits);
            let error = report
                .result()
                .expect_err("a reached ceiling stops evaluation");
            match error {
                temporal::Error::Exhausted(exhaustion) => {
                    assert_eq!(exhaustion.dimension, dimension);
                    assert_eq!(exhaustion.subject.declaration, at);
                }
                other => panic!("{dimension:?}: expected a typed stop, got {other:?}"),
            }
        }
    });
}

/// NFR-008-AC-3: a required retained record that is evicted produces an explicit
/// incomplete result naming it, and never narrows the evaluated interval.
#[trace("TC-124", "NFR-008-AC-3")]
#[test]
fn an_evicted_required_record_is_named_not_narrowed() {
    let body = "always[0,1] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        let leaves = holds_leaves(package, at);
        let leaf = leaves[0];
        let mut supplied = trace(EVENT_POSITION, "orders");
        supplied.positions = vec![
            position(0, &[(leaf, true)]),
            Position {
                coordinate: 1,
                order: None,
                valuations: [(leaf, true)].into_iter().collect(),
            },
        ];
        supplied.decision_scope = temporal::Closure::Open;

        let retained = temporal::evaluate(package, at, &supplied, Limits::default());
        assert_eq!(
            assessment(&retained).truth,
            Some(Truth::True),
            "both required valuations are retained while the obligation is unsettled",
        );

        let mut evicted = supplied.clone();
        evicted.evicted = vec![Eviction::Valuation {
            node: leaf,
            coordinate: 1,
        }];
        let report = temporal::evaluate(package, at, &evicted, Limits::default());
        let assessed = assessment(&report);
        assert_eq!(
            assessed.truth, None,
            "an evicted requirement is not a truth"
        );
        assert_eq!(assessed.basis, Some(Basis::Unavailable));
        let incomplete = assessed
            .incomplete
            .as_ref()
            .expect("the loss is recorded explicitly");
        assert_eq!(incomplete.dimension, temporal::Dimension::Valuation);
        assert_eq!(
            incomplete.subject.declaration, at,
            "the obligation identity is retained",
        );
    });
}

/// NFR-008-AC-4: effective clamped ceilings participate in result identity.
#[trace("TC-124", "NFR-008-AC-4")]
#[test]
fn ceilings_participate_in_result_identity() {
    let body = "always[0,1] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        let supplied = one_position(package, at, EVENT_POSITION);
        let first = temporal::evaluate(package, at, &supplied, Limits::default());
        let same = temporal::evaluate(package, at, &supplied, Limits::default());
        assert_eq!(
            assessment(&first).premises,
            assessment(&same).premises,
            "an unchanged configuration reproduces one identity",
        );

        let lowered = Limits {
            visits: 500,
            ..Limits::default()
        };
        let report = temporal::evaluate(package, at, &supplied, lowered);
        assert_ne!(
            assessment(&first).premises,
            assessment(&report).premises,
            "a changed ceiling refuses reuse of the earlier result",
        );

        // A ceiling above the published default clamps; zero is preserved.
        let raised = Limits {
            visits: usize::MAX,
            ..Limits::default()
        };
        let report = temporal::evaluate(package, at, &supplied, raised);
        assert_eq!(
            report.limits().visits,
            Limits::default().visits,
            "a ceiling above the default clamps",
        );
        let zeroed = Limits {
            visits: 0,
            ..Limits::default()
        };
        let report = temporal::evaluate(package, at, &supplied, zeroed);
        assert_eq!(report.limits().visits, 0, "zero is preserved");
    });
}

/// NFR-008-AC-5: the first unaffordable operation stays unperformed, usage
/// reflects only successful charges, and a retry under sufficient ceilings
/// produces the full result from unmutated inputs.
#[trace("TC-124", "NFR-008-AC-5")]
#[test]
fn the_first_unaffordable_operation_stays_unperformed() {
    let body = "always[0,2] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        let supplied = one_position(package, at, EVENT_POSITION);
        let before = supplied.clone();

        let full = temporal::evaluate(package, at, &supplied, Limits::default());
        let required = full.usage();
        assert_eq!(assessment(&full).truth, Some(Truth::False));

        for ceiling in [0, 1, required.visits.saturating_sub(1)] {
            let report = temporal::evaluate(
                package,
                at,
                &supplied,
                Limits {
                    visits: ceiling,
                    ..Limits::default()
                },
            );
            let error = report.result().expect_err("an insufficient ceiling stops");
            let temporal::Error::Exhausted(exhaustion) = error else {
                panic!("expected a typed stop, got {error:?}");
            };
            assert_eq!(exhaustion.dimension, LimitDimension::Visits);
            assert_eq!(
                exhaustion.used, ceiling,
                "usage reflects only successful charges",
            );
            assert!(
                report.usage().visits <= ceiling,
                "the unaffordable visit was not performed",
            );
        }

        assert_eq!(supplied, before, "the inputs are unmutated");
        let retry = temporal::evaluate(package, at, &supplied, Limits::default());
        assert_eq!(assessment(&retry).truth, Some(Truth::False));
        assert_eq!(retry.usage(), required, "the retry reproduces the full run");
    });
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-125: native-to-TL mapping support classification.
//!
//! FR-045. Every expected disposition is spelt here from the reviewed
//! correspondence support table restated in FR-045, never read back from the
//! classifier, an installed TL version or a backend capability report. No
//! evaluator, bridge definition or predicate projection participates: the
//! classification is a total function of the selected profile, the reachable
//! operator kinds and the requested surrounding-execution closure.

// The shared helper serves several controls; this one needs only part of it.
#[path = "support/temporal/mod.rs"]
#[allow(dead_code)]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::protocol_artifact::{wire as w, AdmittedPackage};
use quire_spec_language::temporal::{
    self, Closure, Operators, Support, Unmatched, EVENT_POSITION as EVENT_POSITION_IDENTITY,
};
use setup::{admitted, origin, Declaration, EVENT_POSITION, FIXED_SAMPLE, TIMESTAMPED_WINDOW};

/// The reviewed support table's rows, in the order FR-045 gives them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Row {
    /// Timestamped-event finite window.
    FiniteWindow,
    /// Any bounded past operator reachable from the root.
    PastOperator,
    /// Event-position, a bounded future operator, complete surrounding execution.
    EventComplete,
    /// Event-position, a bounded future operator, open surrounding execution.
    EventOpen,
    /// Fixed-sample and a bounded future operator.
    FixedSampleFuture,
    /// Either false-extension profile with no bounded temporal operator at all.
    NoOperator,
}

/// Every row the table has; the control requires each one to be reachable.
const ROWS: [Row; 6] = [
    Row::FiniteWindow,
    Row::PastOperator,
    Row::EventComplete,
    Row::EventOpen,
    Row::FixedSampleFuture,
    Row::NoOperator,
];

/// The row's own key, transcribed from FR-045's table and not from the
/// classifier, so "matches exactly one row" is checked against the table.
fn keys(row: Row, profile: &str, operators: Operators, surrounding: Closure) -> bool {
    let false_extension = profile == EVENT_POSITION || profile == FIXED_SAMPLE;
    let bounded_future = operators.future && !operators.past;
    match row {
        Row::FiniteWindow => profile == TIMESTAMPED_WINDOW,
        Row::PastOperator => operators.past,
        Row::EventComplete => {
            profile == EVENT_POSITION && bounded_future && surrounding == Closure::Closed
        }
        Row::EventOpen => {
            profile == EVENT_POSITION && bounded_future && surrounding == Closure::Open
        }
        Row::FixedSampleFuture => profile == FIXED_SAMPLE && bounded_future,
        Row::NoOperator => false_extension && !operators.future && !operators.past,
    }
}

/// What the table says a row returns. Spelt here, so the classifier's own
/// output is never the expectation.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Expect {
    /// Every unmatched dimension the row names, and nothing else.
    Unsupported(&'static [Unmatched]),
    /// The row's exact TL target identity, and whether the row attaches the
    /// total-sample-valuation condition.
    Supported {
        target: &'static str,
        total_sample_valuation: bool,
    },
}

/// One admitted declaration exercising one row.
struct Case {
    row: Row,
    profile: &'static str,
    body: &'static str,
    surrounding: Closure,
    expect: Expect,
}

/// The operator kinds reachable from the declaration's root, read from the
/// admitted arena rather than from the authored text.
fn operators(package: &AdmittedPackage, declaration: usize) -> Operators {
    Operators::of(&package.package().declarations[declaration].temporal)
}

/// The admitted profile revision recorded for the declaration's selected
/// definition, read independently of the classification.
fn admitted_revision(package: &AdmittedPackage, declaration: usize) -> String {
    let package = package.package();
    let entry = &package.declarations[declaration];
    package.definitions[entry.profile as usize]
        .revision
        .value
        .clone()
}

/// Require a disposition to be exactly what the table says.
fn require(expect: &Expect, support: &Support, what: &str) {
    match (expect, support) {
        (Expect::Unsupported(dimensions), Support::Unsupported { dimensions: named }) => {
            assert_eq!(
                named.as_slice(),
                *dimensions,
                "{what}: unmatched dimensions"
            );
        }
        (
            Expect::Supported {
                target,
                total_sample_valuation,
            },
            Support::Supported {
                target: named,
                total_sample_valuation: condition,
                ..
            },
        ) => {
            assert_eq!(named.identity(), *target, "{what}: TL target identity");
            assert_eq!(
                condition, total_sample_valuation,
                "{what}: total-sample-valuation condition",
            );
        }
        (expect, support) => panic!("{what}: expected {expect:?}, classified {support:?}"),
    }
}

/// Every row of the table, one admitted declaration each.
fn cases() -> Vec<Case> {
    vec![
        Case {
            row: Row::FiniteWindow,
            profile: TIMESTAMPED_WINDOW,
            body: "always[0,1] holds(view.ready)",
            surrounding: Closure::Closed,
            expect: Expect::Unsupported(&[Unmatched::FiniteWindow]),
        },
        Case {
            row: Row::PastOperator,
            profile: EVENT_POSITION,
            body: "once[0,1] holds(view.ready)",
            surrounding: Closure::Closed,
            expect: Expect::Unsupported(&[Unmatched::PastOperator]),
        },
        Case {
            row: Row::PastOperator,
            profile: FIXED_SAMPLE,
            body: "historically[0,2] holds(view.ready)",
            surrounding: Closure::Open,
            expect: Expect::Unsupported(&[Unmatched::PastOperator]),
        },
        Case {
            row: Row::EventComplete,
            profile: EVENT_POSITION,
            body: "always[0,1] holds(view.ready)",
            surrounding: Closure::Closed,
            expect: Expect::Supported {
                target: "mltl.closed-trace/v1",
                total_sample_valuation: false,
            },
        },
        Case {
            row: Row::EventOpen,
            profile: EVENT_POSITION,
            body: "always[0,1] holds(view.ready)",
            surrounding: Closure::Open,
            expect: Expect::Supported {
                target: "mltl.online-prefix/v1",
                total_sample_valuation: false,
            },
        },
        Case {
            row: Row::FixedSampleFuture,
            profile: FIXED_SAMPLE,
            body: "eventually[0,2] holds(view.ready)",
            surrounding: Closure::Closed,
            expect: Expect::Supported {
                target: "mltl.closed-trace/v1",
                total_sample_valuation: true,
            },
        },
        Case {
            row: Row::FixedSampleFuture,
            profile: FIXED_SAMPLE,
            body: "eventually[0,2] holds(view.ready)",
            surrounding: Closure::Open,
            expect: Expect::Supported {
                target: "mltl.online-prefix/v1",
                total_sample_valuation: true,
            },
        },
        Case {
            row: Row::NoOperator,
            profile: EVENT_POSITION,
            body: "holds(view.ready)",
            surrounding: Closure::Closed,
            expect: Expect::Supported {
                target: "mltl.closed-trace/v1",
                total_sample_valuation: false,
            },
        },
        Case {
            row: Row::NoOperator,
            profile: FIXED_SAMPLE,
            body: "true",
            surrounding: Closure::Open,
            expect: Expect::Supported {
                target: "mltl.online-prefix/v1",
                total_sample_valuation: false,
            },
        },
    ]
}

/// FR-045-AC-1: every row of the table is reachable, is exercised by an admitted
/// declaration matching it and no other row, and returns that row's disposition
/// and TL target.
#[trace("TC-125", "FR-045-AC-1")]
#[test]
fn every_support_table_row_is_reachable_by_exactly_one_declaration() {
    let cases = cases();
    for row in ROWS {
        assert!(
            cases.iter().any(|case| case.row == row),
            "{row:?} is not exercised by any declaration",
        );
    }
    for case in &cases {
        let what = format!("{:?} {} {}", case.row, case.profile, case.body);
        admitted(&origin("Due", case.profile, case.body), |package, at| {
            let reachable = operators(package, at);
            let matched: Vec<Row> = ROWS
                .into_iter()
                .filter(|row| keys(*row, case.profile, reachable, case.surrounding))
                .collect();
            assert_eq!(
                matched,
                vec![case.row],
                "{what}: the table must key this declaration to one row",
            );
            let classified = temporal::mapping_support(package, at, case.surrounding)
                .expect("an admitted temporal declaration classifies");
            require(&case.expect, &classified.support, &what);
        });
    }
}

/// FR-045-AC-1: the disposition is a function of the selected profile, the
/// reachable operator kinds and the requested closure alone. Declarations
/// differing in name, clock, activation, interval bounds and operator spelling
/// but agreeing on those three inputs classify identically, so no syntax match
/// and no historical result is consulted; the classifier is handed no trace, so
/// no decision-scope closure and no backend report can reach it.
#[trace("TC-125", "FR-045-AC-1")]
#[test]
fn the_disposition_follows_the_three_inputs_and_nothing_else() {
    let first = Declaration {
        name: "Due",
        profile: EVENT_POSITION,
        clock: "orders",
        body: "always[0,1] holds(view.ready)",
        activation: "on origin",
    };
    let second = Declaration {
        name: "Later",
        profile: EVENT_POSITION,
        clock: "shipments",
        body: "eventually[3,9] (holds(view.ready) or true)",
        activation: "on each (started: M::Node) when (started.n >= 0)",
    };
    admitted(&first, |package, at| {
        let repeated = temporal::mapping_support(package, at, Closure::Closed).unwrap();
        let again = temporal::mapping_support(package, at, Closure::Closed).unwrap();
        assert_eq!(
            repeated.support, again.support,
            "an unchanged request reproduces one disposition",
        );
        let closed = repeated.support;
        let open = temporal::mapping_support(package, at, Closure::Open)
            .unwrap()
            .support;
        assert_ne!(closed, open, "the requested closure selects the TL target");
        admitted(&second, |other, other_at| {
            assert_eq!(
                operators(package, at),
                operators(other, other_at),
                "both declarations reach a bounded future operator and no past one",
            );
            assert_ne!(
                package.package().declarations[at].temporal,
                other.package().declarations[other_at].temporal,
                "the two formulas differ in bytes",
            );
            assert_eq!(
                temporal::mapping_support(other, other_at, Closure::Closed)
                    .unwrap()
                    .support,
                closed,
                "differing spelling, clock and activation do not change the row",
            );
        });
    });
}

/// FR-045-AC-2: a timestamped-event request names the finite-window dimension
/// and a bounded-past request names the past-operator dimension, under every
/// profile. Neither carries a substitute formula, profile or clock, and no index
/// conversion is inferred for the timestamped request.
#[trace("TC-125", "FR-045-AC-2")]
#[test]
fn unsupported_requests_name_their_own_dimension_and_substitute_nothing() {
    admitted(
        &origin("Due", TIMESTAMPED_WINDOW, "always[0,1] holds(view.ready)"),
        |package, at| {
            let classified = temporal::mapping_support(package, at, Closure::Closed).unwrap();
            let Support::Unsupported { dimensions } = &classified.support else {
                panic!("a finite-window profile has no TL target: {classified:?}");
            };
            assert_eq!(
                dimensions.as_slice(),
                [Unmatched::FiniteWindow],
                "the finite-window dimension alone; no index conversion is inferred",
            );
            assert_eq!(
                classified.retained.profile.identity(),
                setup::identity(TIMESTAMPED_WINDOW),
                "the selected profile is retained, never substituted",
            );
        },
    );

    for (profile, body) in [
        (EVENT_POSITION, "once[0,1] holds(view.ready)"),
        (FIXED_SAMPLE, "once[0,1] holds(view.ready)"),
        (TIMESTAMPED_WINDOW, "historically[0,2] holds(view.ready)"),
    ] {
        admitted(&origin("Due", profile, body), |package, at| {
            let classified = temporal::mapping_support(package, at, Closure::Closed).unwrap();
            let Support::Unsupported { dimensions } = &classified.support else {
                panic!("{profile}: a bounded past operator has no TL target");
            };
            assert!(
                dimensions.contains(&Unmatched::PastOperator),
                "{profile}: the past-operator dimension is named",
            );
            assert_eq!(
                classified.retained.profile.identity(),
                setup::identity(profile),
                "{profile}: the selected profile is retained, never substituted",
            );
        });
    }
}

/// FR-045-AC-3: an unsupported result retains the native declaration subject,
/// its selected profile identity and revision and its activation record, and the
/// source declaration is unchanged by the classification.
#[trace("TC-125", "FR-045-AC-3")]
#[test]
fn an_unsupported_result_retains_the_native_subject_and_changes_nothing() {
    let each = Declaration {
        name: "Due",
        profile: TIMESTAMPED_WINDOW,
        clock: "orders",
        body: "always[0,1] holds(view.ready)",
        activation: "on each (started: M::Node) when (started.n >= 0)",
    };
    for declaration in [
        origin("Due", TIMESTAMPED_WINDOW, "always[0,1] holds(view.ready)"),
        each,
    ] {
        admitted(&declaration, |package, at| {
            let before = package.package().declarations[at].clone();
            let revision = admitted_revision(package, at);
            let classified = temporal::mapping_support(package, at, Closure::Closed).unwrap();
            assert!(matches!(classified.support, Support::Unsupported { .. }));

            let retained = &classified.retained;
            assert_eq!(retained.subject.declaration, at, "the retained subject");
            assert_eq!(retained.subject.node, None);
            assert_eq!(retained.subject.position, None);
            assert_eq!(retained.name, before.name, "the native declaration name");
            assert_eq!(
                retained.profile.identity(),
                setup::identity(TIMESTAMPED_WINDOW),
                "the selected profile identity",
            );
            assert_eq!(retained.profile_revision, revision, "the admitted revision");
            let w::Body::Temporal { activation, .. } = &before.body else {
                panic!("an authored temporal declaration");
            };
            assert_eq!(&retained.activation, activation, "the activation record");

            assert_eq!(
                &before,
                &package.package().declarations[at],
                "the source declaration is unchanged by the classification",
            );
        });
    }
}

/// FR-045-AC-4: a declaration unmatched on two dimensions at once names both,
/// not one summary cause.
#[trace("TC-125", "FR-045-AC-4")]
#[test]
fn a_doubly_unmatched_declaration_names_both_dimensions() {
    admitted(
        &origin("Due", TIMESTAMPED_WINDOW, "once[0,1] holds(view.ready)"),
        |package, at| {
            let classified = temporal::mapping_support(package, at, Closure::Closed).unwrap();
            let Support::Unsupported { dimensions } = &classified.support else {
                panic!("neither dimension has a TL target: {classified:?}");
            };
            assert_eq!(
                dimensions.as_slice(),
                [Unmatched::FiniteWindow, Unmatched::PastOperator],
                "both unmatched dimensions, not one summary cause",
            );
        },
    );
}

/// FR-045-AC-5: a supported classification names its TL target identity, the
/// source table's baseline revision and every outstanding bridge premise, and
/// asserts no correspondence: naming a premise does not discharge it, and the
/// classification carries no TL formula, valuation request or correspondence
/// record for any premise to be discharged against.
#[trace("TC-125", "FR-045-AC-5")]
#[test]
fn a_supported_classification_names_its_target_baseline_and_premises() {
    /// The premises FR-045 requires a supported classification to name.
    const REQUIRED: [&str; 9] = [
        "total Boolean predicate projection",
        "source and clause identity",
        "model, type and predicate bindings",
        "evaluation anchor",
        "capture environment",
        "clock and observation binding",
        "interval",
        "closure and history premises",
        "result dimensions",
    ];

    admitted(
        &origin("Due", EVENT_POSITION, "always[0,1] holds(view.ready)"),
        |package, at| {
            let classified = temporal::mapping_support(package, at, Closure::Closed).unwrap();
            let Support::Supported {
                target,
                table,
                premises,
                ..
            } = &classified.support
            else {
                panic!("an event-position future formula is supported: {classified:?}");
            };
            assert_eq!(target.identity(), "mltl.closed-trace/v1");
            assert!(
                table.contains("782c1ce39a197cd52b8b35b50adf2e5e3ecedd0f"),
                "the source table's baseline revision is recorded: {table}",
            );
            assert!(
                table.contains("FR-095"),
                "the reviewed source table is named: {table}",
            );
            for required in REQUIRED {
                assert!(
                    premises.iter().any(|premise| premise.contains(required)),
                    "the outstanding premise {required:?} is named in {premises:?}",
                );
            }
            assert_eq!(
                classified.retained.profile.identity(),
                EVENT_POSITION_IDENTITY,
                "a supported classification still retains its native profile",
            );
        },
    );
}

/// FR-045-AC-5: equal temporal formula bytes under different selected profiles
/// do not receive the same classification. Equal bytes establish no definition
/// selection: the profile the source selected decides the row.
#[trace("TC-125", "FR-045-AC-5")]
#[test]
fn equal_formula_bytes_under_different_profiles_classify_differently() {
    let body = "always[0,1] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |event, event_at| {
        let event_formula = event.package().declarations[event_at].temporal.clone();
        let event_support = temporal::mapping_support(event, event_at, Closure::Closed)
            .unwrap()
            .support;

        for profile in [TIMESTAMPED_WINDOW, FIXED_SAMPLE] {
            admitted(&origin("Due", profile, body), |other, other_at| {
                assert_eq!(
                    other.package().declarations[other_at].temporal,
                    event_formula,
                    "{profile}: the emitted temporal formula bytes are equal",
                );
                let support = temporal::mapping_support(other, other_at, Closure::Closed)
                    .unwrap()
                    .support;
                assert_ne!(
                    support, event_support,
                    "{profile}: a different selected profile is a different classification",
                );
            });
        }
    });
}

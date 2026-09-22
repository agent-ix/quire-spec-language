// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-122: bounded temporal truth under each selected profile.
//!
//! FR-043. Expected truths, bases and premises come from the authored source and
//! from the profile definitions under `resources/native-v1/proposals/`, never
//! from the evaluator's own output.

use crate::support::temporal as setup;

use ix_trace_rs::trace;
use quire_spec_language::protocol_artifact::{wire as w, AdmittedPackage};
use quire_spec_language::temporal::{self, Basis, Closure, Completeness, Execution, Truth};
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
#[trace("TC-122", "FR-043-AC-1")]
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
#[trace("TC-122", "FR-043-AC-2")]
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
#[trace("TC-122", "FR-043-AC-3")]
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
#[trace("TC-122", "FR-043-AC-6")]
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
#[trace("TC-122", "FR-043-AC-7")]
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
#[trace("TC-122", "FR-043-AC-8")]
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
#[trace("TC-122", "FR-043-AC-9")]
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
#[trace("TC-122", "FR-043-AC-13")]
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

/// The temporal arena leaf one handle reaches, through the grouping and
/// connective nodes the authored source placed above it.
fn leaf_under(nodes: &[w::Temporal], handle: &w::Handle) -> u32 {
    let mut index = handle.index;
    for _ in 0..=nodes.len() {
        match &nodes[index as usize].operation {
            w::TemporalOperation::Holds { .. } => return index,
            w::TemporalOperation::Group { value } | w::TemporalOperation::Unary { value, .. } => {
                index = value.index;
            }
            other => panic!("no holds leaf under {other:?}"),
        }
    }
    panic!("the emitted temporal graph is not closed")
}

/// The two `holds` leaves the declaration's one order-sensitive binary operator
/// relates, its left operand first. Two authored occurrences of one predicate
/// emit two distinct leaves, so the trace values each independently.
fn relation_leaves(package: &AdmittedPackage, at: usize) -> (u32, u32) {
    let nodes = &package.package().declarations[at].temporal;
    nodes
        .iter()
        .find_map(|node| match &node.operation {
            w::TemporalOperation::Binary {
                operator:
                    w::TemporalBinary::Until
                    | w::TemporalBinary::Release
                    | w::TemporalBinary::Since
                    | w::TemporalBinary::Triggered,
                left,
                right,
                ..
            } => Some((leaf_under(nodes, left), leaf_under(nodes, right))),
            _ => None,
        })
        .expect("one authored order-sensitive binary operator")
}

/// One position per row at coordinates `0..rows.len()`, valuing the single leaf.
fn valued(leaf: u32, rows: &[bool]) -> Vec<temporal::Position> {
    rows.iter()
        .enumerate()
        .map(|(index, held)| position(index as i64, &[(leaf, *held)]))
        .collect()
}

/// One position per row at coordinates `0..rows.len()`, valuing the left and
/// right leaves of the authored relation.
fn related(leaves: (u32, u32), rows: &[(bool, bool)]) -> Vec<temporal::Position> {
    rows.iter()
        .enumerate()
        .map(|(index, (left, right))| {
            position(index as i64, &[(leaves.0, *left), (leaves.1, *right)])
        })
        .collect()
}

/// FR-043-AC-4: the eight bounded operators evaluate over inclusive intervals,
/// including the zero-width `[k,k]` form and a nonzero lower bound. `once[1,1]`
/// is strong previous, so it is false at an authoritative origin and reads the
/// immediately preceding offset elsewhere. `p until[1,2] q` is true with `p`
/// false at the anchor and `q` true at offset one, because the left operand is
/// required only between the lower bound and the witness.
#[trace("TC-122", "FR-043-AC-4")]
#[test]
fn all_eight_bounded_operators_evaluate_over_inclusive_intervals() {
    /// Authored unary body, the leaf's valuation at positions 0..3, and the
    /// truth the authored source and profile definitions require.
    type Unary<'a> = (&'a str, &'a [bool], Truth);
    /// Authored binary body, the left and right leaf valuations at positions
    /// 0..3, and the required truth.
    type Binary<'a> = (&'a str, &'a [(bool, bool)], Truth);

    const UNARY: &[Unary<'_>] = &[
        // A zero-width interval at a nonzero lower bound reads offset two alone.
        (
            "eventually[2,2] holds(view.ready)",
            &[false, false, true, false],
            Truth::True,
        ),
        (
            "eventually[2,2] holds(view.ready)",
            &[false, true, false, false],
            Truth::False,
        ),
        // A nonzero lower bound excludes the anchor from the universal range.
        (
            "always[1,2] holds(view.ready)",
            &[false, true, true, false],
            Truth::True,
        ),
        (
            "always[1,2] holds(view.ready)",
            &[false, true, false, false],
            Truth::False,
        ),
        // `once[1,1]` is strong previous: inside `eventually[2,2]` it reads
        // offset one and neither offset two nor the anchor.
        (
            "eventually[2,2] once[1,1] holds(view.ready)",
            &[false, true, false, false],
            Truth::True,
        ),
        (
            "eventually[2,2] once[1,1] holds(view.ready)",
            &[true, false, true, false],
            Truth::False,
        ),
        // A strong previous at the authoritative origin has no previous
        // position, so it is false where a weak previous would be true.
        (
            "once[1,1] holds(view.ready)",
            &[true, true, true, true],
            Truth::False,
        ),
        (
            "eventually[2,2] historically[0,1] holds(view.ready)",
            &[false, true, true, false],
            Truth::True,
        ),
        (
            "eventually[2,2] historically[0,1] holds(view.ready)",
            &[false, false, true, false],
            Truth::False,
        ),
    ];

    const BINARY: &[Binary<'_>] = &[
        // `p until[1,2] q` with `p` false at the anchor and `q` true at offset
        // one: the left operand is required only from the lower bound to the
        // witness, so the anchor's false `p` is not consulted.
        (
            "holds(view.ready) until[1,2] holds(view.ready)",
            &[
                (false, false),
                (false, true),
                (false, false),
                (false, false),
            ],
            Truth::True,
        ),
        (
            "holds(view.ready) until[1,2] holds(view.ready)",
            &[
                (false, false),
                (false, false),
                (false, false),
                (false, false),
            ],
            Truth::False,
        ),
        // A zero-width binary interval admits the witness at offset two alone.
        (
            "holds(view.ready) until[2,2] holds(view.ready)",
            &[(true, false), (true, false), (true, true), (false, false)],
            Truth::True,
        ),
        (
            "holds(view.ready) until[2,2] holds(view.ready)",
            &[(true, false), (true, true), (true, false), (false, false)],
            Truth::False,
        ),
        // `q1 and (p1 or q2)`, the Boolean dual of `until` over the same range.
        (
            "holds(view.ready) release[1,2] holds(view.ready)",
            &[(false, false), (false, true), (false, true), (false, false)],
            Truth::True,
        ),
        (
            "holds(view.ready) release[1,2] holds(view.ready)",
            &[(false, false), (true, false), (false, true), (false, false)],
            Truth::False,
        ),
        // The past operators, anchored where their computed range lies inside
        // the admitted positions.
        (
            "eventually[2,2] (holds(view.ready) since[1,2] holds(view.ready))",
            &[
                (false, false),
                (false, true),
                (false, false),
                (false, false),
            ],
            Truth::True,
        ),
        (
            "eventually[2,2] (holds(view.ready) since[1,2] holds(view.ready))",
            &[
                (false, false),
                (false, false),
                (false, false),
                (false, false),
            ],
            Truth::False,
        ),
        (
            "eventually[2,2] (holds(view.ready) triggered[1,2] holds(view.ready))",
            &[(false, false), (true, true), (false, false), (false, false)],
            Truth::True,
        ),
        (
            "eventually[2,2] (holds(view.ready) triggered[1,2] holds(view.ready))",
            &[
                (false, false),
                (true, false),
                (false, false),
                (false, false),
            ],
            Truth::False,
        ),
    ];

    for profile in [EVENT_POSITION, FIXED_SAMPLE, TIMESTAMPED_WINDOW] {
        for (body, rows, expected) in UNARY {
            admitted(&origin("Due", profile, body), |package, at| {
                let mut supplied = trace(profile, "orders");
                supplied.positions = valued(holds_leaf(package, at), rows);
                supplied.watermark = 3;
                let report =
                    temporal::evaluate(package, at, &supplied, temporal::Limits::default());
                assert_eq!(
                    assessment(&report).truth,
                    Some(*expected),
                    "{profile} {body} over {rows:?}",
                );
            });
        }
        for (body, rows, expected) in BINARY {
            admitted(&origin("Due", profile, body), |package, at| {
                let mut supplied = trace(profile, "orders");
                supplied.positions = related(relation_leaves(package, at), rows);
                supplied.watermark = 3;
                let report =
                    temporal::evaluate(package, at, &supplied, temporal::Limits::default());
                assert_eq!(
                    assessment(&report).truth,
                    Some(*expected),
                    "{profile} {body} over {rows:?}",
                );
            });
        }
    }
}

/// FR-043-AC-4: `release` and `triggered` agree with the Boolean duals of
/// `until` and `since` on the same traces, over every valuation of the two
/// leaves at the three admitted positions.
#[trace("TC-122", "FR-043-AC-4")]
#[test]
fn release_and_triggered_agree_with_their_boolean_duals() {
    /// Every valuation of two leaves at three positions.
    const PATTERNS: u32 = 64;

    /// Evaluate one authored body against all `PATTERNS` traces, in one order.
    fn outcomes(body: &str) -> Vec<Option<Truth>> {
        let mut collected = Vec::new();
        admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
            let leaves = relation_leaves(package, at);
            for pattern in 0..PATTERNS {
                let rows: Vec<(bool, bool)> = (0..3)
                    .map(|index| {
                        (
                            pattern >> (2 * index) & 1 == 1,
                            pattern >> (2 * index + 1) & 1 == 1,
                        )
                    })
                    .collect();
                let mut supplied = trace(EVENT_POSITION, "orders");
                supplied.positions = related(leaves, &rows);
                supplied.watermark = 2;
                let report =
                    temporal::evaluate(package, at, &supplied, temporal::Limits::default());
                collected.push(assessment(&report).truth);
            }
        });
        assert!(
            collected.contains(&Some(Truth::True)) && collected.contains(&Some(Truth::False)),
            "{body}: both truths occur, so agreement is not agreement on one constant",
        );
        collected
    }

    assert_eq!(
        outcomes("holds(view.ready) release[1,2] holds(view.ready)"),
        outcomes("not (not holds(view.ready) until[1,2] not holds(view.ready))"),
        "release is the Boolean dual of until under one lower-bound convention",
    );
    assert_eq!(
        outcomes("eventually[2,2] (holds(view.ready) triggered[1,2] holds(view.ready))"),
        outcomes("eventually[2,2] not (not holds(view.ready) since[1,2] not holds(view.ready))"),
        "triggered is the Boolean dual of since under one lower-bound convention",
    );
}

/// FR-043-AC-5: `until`, `release`, `since` and `triggered` refuse when two
/// participating positions share a clock coordinate and either lacks an
/// admitted order key, or when the two keys come from different authorities.
/// One admitted order over both positions evaluates, and reversing the trace's
/// insertion order changes nothing.
#[trace("TC-122", "FR-043-AC-5")]
#[test]
fn order_sensitive_operators_refuse_without_one_admitted_order() {
    /// One admitted order key.
    fn key(authority: &str, key: i64) -> Option<temporal::OrderKey> {
        Some(temporal::OrderKey {
            authority: authority.into(),
            key,
        })
    }

    for (body, expected) in [
        (
            "holds(view.ready) until[0,2] holds(view.ready)",
            Truth::True,
        ),
        (
            "holds(view.ready) release[0,2] holds(view.ready)",
            Truth::False,
        ),
        (
            "holds(view.ready) since[0,2] holds(view.ready)",
            Truth::False,
        ),
        (
            "holds(view.ready) triggered[0,2] holds(view.ready)",
            Truth::False,
        ),
    ] {
        admitted(&origin("Due", TIMESTAMPED_WINDOW, body), |package, at| {
            let (left, right) = relation_leaves(package, at);
            // Two positions share clock coordinate zero; a third at coordinate
            // two carries the witness.
            let shared = |first: Option<temporal::OrderKey>, second: Option<temporal::OrderKey>| {
                let mut positions = vec![
                    position(0, &[(left, true), (right, false)]),
                    position(0, &[(left, true), (right, false)]),
                    position(2, &[(left, false), (right, true)]),
                ];
                positions[0].order = first;
                positions[1].order = second;
                positions
            };
            let supply = |positions: Vec<temporal::Position>| {
                let mut supplied = trace(TIMESTAMPED_WINDOW, "orders");
                supplied.positions = positions;
                supplied.watermark = 2;
                supplied
            };

            for (case, positions) in [
                ("neither position keyed", shared(None, None)),
                ("one position keyed", shared(key("causal", 1), None)),
                (
                    "two admitted order authorities",
                    shared(key("causal", 1), key("sequence", 2)),
                ),
            ] {
                let supplied = supply(positions);
                let report =
                    temporal::evaluate(package, at, &supplied, temporal::Limits::default());
                let error = report
                    .result()
                    .err()
                    .unwrap_or_else(|| panic!("{body} {case}: an equal coordinate refuses"));
                assert!(
                    matches!(
                        error,
                        temporal::Error::Refused(temporal::Refusal::Order { .. })
                    ),
                    "{body} {case}: {error:?}",
                );
            }

            // One admitted order authority covering both positions evaluates,
            // and the trace's insertion order is never consulted as one.
            let admitted_order = shared(key("causal", 1), key("causal", 2));
            let mut reversed = admitted_order.clone();
            reversed.reverse();
            let (forwards, backwards) = (supply(admitted_order), supply(reversed));
            let ordered = temporal::evaluate(package, at, &forwards, temporal::Limits::default());
            let assessed = assessment(&ordered);
            assert_eq!(
                assessed.truth,
                Some(expected),
                "{body}: one admitted order evaluates",
            );
            assert_eq!(assessed.basis, Some(Basis::ClosedScope), "{body}");

            let report = temporal::evaluate(package, at, &backwards, temporal::Limits::default());
            let reversed = assessment(&report);
            assert_eq!(reversed.truth, assessed.truth, "{body}: reversed insertion");
            assert_eq!(reversed.basis, assessed.basis, "{body}: reversed insertion");
            assert_eq!(
                reversed.premises, assessed.premises,
                "{body}: reversed insertion",
            );
            // Decision support locates positions in the caller's own vector, so
            // the two supports name the same admitted positions by the clock
            // coordinate each carries rather than by the caller's index.
            let named = |supplied: &temporal::Trace, support: &[usize]| -> Vec<i64> {
                support
                    .iter()
                    .map(|index| supplied.positions[*index].coordinate)
                    .collect()
            };
            assert_eq!(
                named(&backwards, &reversed.support),
                named(&forwards, &assessed.support),
                "{body}: reversing insertion order leaves the decision support unchanged",
            );
        });
    }
}

/// FR-043-AC-10: decision-scope closure, surrounding-execution closure,
/// assessment execution and input completeness stay four independently
/// represented dimensions. Every one of the sixteen combinations is reported as
/// supplied and carries its own result identity, and closing or completing one
/// axis never closes or completes another: only a closed decision scope over
/// complete input applies the closed-boundary rule.
///
/// The remaining half of this criterion — that every one-axis substitution of a
/// settlement basis, truth and closure combination is refused — has no control
/// here. The public API accepts no caller-supplied basis: the evaluator computes
/// the basis from the value and the two premises, so there is no substitution to
/// submit. Recorded as remaining work on compiler #38.
#[trace("TC-122", "FR-043-AC-10")]
#[test]
fn the_four_progress_axes_stay_independently_represented() {
    let body = "always[0,2] holds(view.ready)";
    admitted(&origin("Due", EVENT_POSITION, body), |package, at| {
        let leaf = holds_leaf(package, at);
        let mut identities = Vec::new();
        for decision in [Closure::Closed, Closure::Open] {
            for surrounding in [Closure::Closed, Closure::Open] {
                for execution in [Execution::Completed, Execution::Failed] {
                    for completeness in [Completeness::Complete, Completeness::Incomplete] {
                        let mut supplied = trace(EVENT_POSITION, "orders");
                        supplied.positions = vec![position(0, &[(leaf, true)])];
                        supplied.decision_scope = decision;
                        supplied.surrounding_execution = surrounding;
                        supplied.execution = execution;
                        supplied.completeness = completeness;
                        let report =
                            temporal::evaluate(package, at, &supplied, temporal::Limits::default());
                        let assessed = assessment(&report);
                        let axes = format!(
                            "decision {decision:?}, surrounding {surrounding:?}, \
                             execution {execution:?}, completeness {completeness:?}"
                        );
                        assert_eq!(assessed.premises.decision_scope, decision, "{axes}");
                        assert_eq!(
                            assessed.premises.surrounding_execution, surrounding,
                            "{axes}",
                        );
                        assert_eq!(assessed.premises.execution, execution, "{axes}");
                        assert_eq!(assessed.premises.completeness, completeness, "{axes}");

                        // The closed-boundary rule answers to the decision scope
                        // and input completeness alone. A closed surrounding
                        // execution does not close the decision scope, a closed
                        // decision scope does not complete the input, and a
                        // failed assessment execution is not a temporal truth.
                        let closed =
                            decision == Closure::Closed && completeness == Completeness::Complete;
                        let expected = if closed {
                            (Some(Truth::False), Some(Basis::ClosedScope))
                        } else {
                            (Some(Truth::Pending), Some(Basis::Unsettled))
                        };
                        assert_eq!((assessed.truth, assessed.basis), expected, "{axes}");
                        identities.push((axes, assessed.premises.clone()));
                    }
                }
            }
        }
        assert_eq!(identities.len(), 16);
        for (index, (axes, premises)) in identities.iter().enumerate() {
            for (other, other_premises) in identities.iter().skip(index + 1) {
                assert_ne!(
                    premises, other_premises,
                    "{axes} and {other} are four axes, not fewer",
                );
            }
        }
    });
}

/// FR-043-AC-11: a fixed-sample or timestamped watermark reaching an inclusive
/// deadline over complete valuations settles a silent obligation; a watermark
/// short of the deadline, or incomplete input at it, does not. An admitted
/// instant exactly at the deadline participates before settlement and one past
/// it does not. An event-position clock does not advance during silence.
#[trace("TC-122", "FR-043-AC-11")]
#[test]
fn progress_settles_a_silent_deadline_only_where_the_clock_advances() {
    let body = "eventually[0,2] holds(view.ready)";
    for (profile, watermark, completeness, truth, basis) in [
        (
            FIXED_SAMPLE,
            2,
            Completeness::Complete,
            Truth::False,
            Basis::DecisiveCounterexample,
        ),
        (
            FIXED_SAMPLE,
            1,
            Completeness::Complete,
            Truth::Pending,
            Basis::Unsettled,
        ),
        (
            FIXED_SAMPLE,
            2,
            Completeness::Incomplete,
            Truth::Pending,
            Basis::Unsettled,
        ),
        (
            TIMESTAMPED_WINDOW,
            2,
            Completeness::Complete,
            Truth::False,
            Basis::DecisiveCounterexample,
        ),
        (
            TIMESTAMPED_WINDOW,
            1,
            Completeness::Complete,
            Truth::Pending,
            Basis::Unsettled,
        ),
        // An event-position sequence does not advance during silence, so no
        // watermark settles its deadline.
        (
            EVENT_POSITION,
            99,
            Completeness::Complete,
            Truth::Pending,
            Basis::Unsettled,
        ),
    ] {
        admitted(&origin("Due", profile, body), |package, at| {
            let leaf = holds_leaf(package, at);
            let mut supplied = trace(profile, "orders");
            supplied.positions = vec![position(0, &[(leaf, false)])];
            supplied.decision_scope = Closure::Open;
            supplied.completeness = completeness;
            supplied.watermark = watermark;
            let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
            let assessed = assessment(&report);
            let case = format!("{profile} silent to watermark {watermark} under {completeness:?}");
            assert_eq!(assessed.truth, Some(truth), "{case}");
            assert_eq!(assessed.basis, Some(basis), "{case}");
        });
    }

    // An instant exactly at the inclusive upper endpoint participates; one tick
    // past it is outside the window and is not consulted.
    for (coordinate, truth, basis, participates) in [
        (2, Truth::True, Basis::DecisiveWitness, true),
        (3, Truth::False, Basis::DecisiveCounterexample, false),
    ] {
        admitted(&origin("Due", TIMESTAMPED_WINDOW, body), |package, at| {
            let leaf = holds_leaf(package, at);
            let mut supplied = trace(TIMESTAMPED_WINDOW, "orders");
            supplied.positions = vec![
                position(0, &[(leaf, false)]),
                position(coordinate, &[(leaf, true)]),
            ];
            supplied.decision_scope = Closure::Open;
            supplied.watermark = 2;
            let report = temporal::evaluate(package, at, &supplied, temporal::Limits::default());
            let assessed = assessment(&report);
            assert_eq!(assessed.truth, Some(truth), "instant at {coordinate}");
            assert_eq!(assessed.basis, Some(basis), "instant at {coordinate}");
            assert_eq!(
                assessed.support.contains(&1),
                participates,
                "instant at {coordinate} participates: {participates}",
            );
        });
    }
}

/// FR-043-AC-12: a regressing watermark and a completeness assertion revised in
/// conflict under one binding each return a typed contradiction refusal. The
/// retained progress is not rolled back, the retained closure is not restamped,
/// and re-evaluating the established progress reproduces the earlier result
/// rather than a rewritten one. Progress asserted under a foreign subject, clock
/// or profile is progress for another binding and settles nothing here.
#[trace("TC-122", "FR-043-AC-12")]
#[test]
fn contradicting_progress_refuses_and_rolls_nothing_back() {
    let body = "eventually[0,2] holds(view.ready)";
    admitted(&origin("Due", FIXED_SAMPLE, body), |package, at| {
        let leaf = holds_leaf(package, at);
        let silent = |watermark: i64, completeness: Completeness| {
            let mut supplied = trace(FIXED_SAMPLE, "orders");
            supplied.positions = vec![position(0, &[(leaf, false)])];
            supplied.decision_scope = Closure::Open;
            supplied.completeness = completeness;
            supplied.watermark = watermark;
            supplied
        };
        let binding = temporal::Binding {
            declaration: at,
            clock: "orders".into(),
            profile_identity: temporal::FIXED_SAMPLE.into(),
        };
        let established = temporal::Progress {
            watermark: 2,
            completeness: Completeness::Complete,
            decision_scope: Closure::Open,
        };

        let mut ledger = temporal::Ledger::new();
        let settled = temporal::evaluate_with_progress(
            package,
            at,
            &silent(2, Completeness::Complete),
            temporal::Limits::default(),
            &mut ledger,
        );
        let reached = assessment(&settled).clone();
        assert_eq!(reached.truth, Some(Truth::False), "the deadline settles");
        assert_eq!(reached.basis, Some(Basis::DecisiveCounterexample));

        let regressed = temporal::evaluate_with_progress(
            package,
            at,
            &silent(1, Completeness::Complete),
            temporal::Limits::default(),
            &mut ledger,
        );
        let error = regressed
            .result()
            .expect_err("a regressing watermark is a contradiction");
        assert!(
            matches!(
                error,
                temporal::Error::Refused(temporal::Refusal::Progress {
                    dimension: temporal::Dimension::Watermark,
                    ..
                })
            ),
            "{error:?}",
        );
        assert_eq!(
            ledger.progress(&binding),
            Some(established),
            "progress is not rolled back and closure is not restamped",
        );
        let again = temporal::evaluate_with_progress(
            package,
            at,
            &silent(2, Completeness::Complete),
            temporal::Limits::default(),
            &mut ledger,
        );
        assert_eq!(
            assessment(&again),
            &reached,
            "the earlier result is reproduced, not rewritten",
        );

        let mut revised = temporal::Ledger::new();
        temporal::evaluate_with_progress(
            package,
            at,
            &silent(2, Completeness::Complete),
            temporal::Limits::default(),
            &mut revised,
        );
        let conflict = temporal::evaluate_with_progress(
            package,
            at,
            &silent(2, Completeness::Incomplete),
            temporal::Limits::default(),
            &mut revised,
        );
        let error = conflict
            .result()
            .expect_err("a conflicting completeness revision is a contradiction");
        assert!(
            matches!(
                error,
                temporal::Error::Refused(temporal::Refusal::Progress {
                    dimension: temporal::Dimension::Completeness,
                    ..
                })
            ),
            "{error:?}",
        );
        assert_eq!(
            revised.progress(&binding),
            Some(established),
            "the retained completeness assertion is not revised",
        );

        // A foreign subject, clock or profile is a different binding.
        let mut foreign = temporal::Ledger::new();
        for other in [
            temporal::Binding {
                declaration: at + 1,
                ..binding.clone()
            },
            temporal::Binding {
                clock: "other".into(),
                ..binding.clone()
            },
            temporal::Binding {
                profile_identity: temporal::EVENT_POSITION.into(),
                ..binding.clone()
            },
        ] {
            foreign
                .record(
                    other,
                    temporal::Progress {
                        watermark: 5,
                        completeness: Completeness::Complete,
                        decision_scope: Closure::Closed,
                    },
                )
                .expect("foreign progress records under its own binding");
        }
        let report = temporal::evaluate_with_progress(
            package,
            at,
            &silent(0, Completeness::Complete),
            temporal::Limits::default(),
            &mut foreign,
        );
        let assessed = assessment(&report);
        assert_eq!(
            assessed.truth,
            Some(Truth::Pending),
            "foreign progress settles no deadline here",
        );
        assert_eq!(assessed.basis, Some(Basis::Unsettled));
        assert_eq!(assessed.premises.watermark, 0);
    });
}

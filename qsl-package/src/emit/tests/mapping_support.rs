// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-527 (TC-1322): the native-to-TL mapping support classification of a
//! checked temporal clause. Every package is emitted by the S4 emitter and
//! admitted through QSL's I2 read; each expectation is read by the test from
//! FR-045's table, from the view, or from the package's lock.

use quire_contract_model::{CheckedNodeId, CheckedPackageV2, CheckedSelectionRole};

use super::temporal_clause::{
    admit, compile, declared, declared_on, evidence, unit, EVENT, INFINITE,
};
use super::*;
use crate::checked_v2::V2ReadRefusal;
use crate::checked_views::{
    mapping_support, temporal_subject, Activation, Classification, Closure, MappingRefusal,
    Support, Target, Unmatched, ViewRefusal, OUTSTANDING_PREMISES,
};

const FIXED: &str = "quire.temporal.fixed-sample.false-extension/v1";
const TIMESTAMPED: &str = "quire.temporal.timestamped-event.finite-window/v1";
const TIMED: &str = "quire.temporal.timed/v1";

const P: &str = "holds(c.versionNumber = 1)";
const Q: &str = "holds(c.versionNumber = 2)";
const R: &str = "holds(c.versionNumber = 3)";

/// The body of one clause per FR-045 table row.
const PAST: &str = "once[0,1] holds(c.versionNumber = 1)";
const FUTURE: &str = "eventually[0,2] holds(c.versionNumber = 2)";
const MIXED: &str =
    "once[0,1] holds(c.versionNumber = 1) and eventually[0,2] holds(c.versionNumber = 2)";
const NONE: &str = "holds(c.versionNumber = 1)";

/// The clause ids of `package`, in graph order.
fn clauses(package: &CheckedPackageV2) -> Vec<CheckedNodeId> {
    use quire_contract_model::{CheckedNodeKind, TemporalForm};
    package
        .graph()
        .nodes
        .iter()
        .zip(package.node_kinds())
        .filter(|(_, kind)| **kind == CheckedNodeKind::Temporal(TemporalForm::TemporalClause))
        .map(|(node, _)| node.node_id.clone())
        .collect()
}

/// The package of the one named clause with `body` under `profile`.
fn package_of_named(profile: &str, name: &str, body: &str) -> std::sync::Arc<CheckedPackageV2> {
    admit(&compile(&unit(profile, &declared(name, body))))
}

fn package_of(profile: &str, body: &str) -> std::sync::Arc<CheckedPackageV2> {
    package_of_named(profile, "Due", body)
}

/// The classification of the one clause with `body` under `profile`.
fn classify(profile: &str, body: &str, surrounding: Closure) -> Classification {
    classify_named(profile, "Due", body, surrounding)
}

fn classify_named(profile: &str, name: &str, body: &str, surrounding: Closure) -> Classification {
    let package = package_of_named(profile, name, body);
    let [id] = clauses(&package)[..]
        .to_vec()
        .try_into()
        .expect("one clause");
    let view = temporal_subject(&package, &id).expect("the clause reads");
    let classified = mapping_support(&view, surrounding).expect("the clause classifies");
    assert_eq!(classified.retained.name.as_deref(), view.name());
    classified
}

fn supported(target: Target, total_sample_valuation: bool) -> Support {
    Support::Supported {
        target,
        total_sample_valuation,
        premises: OUTSTANDING_PREMISES,
    }
}

fn unsupported(dimensions: &[Unmatched]) -> Support {
    Support::Unsupported {
        dimensions: dimensions.to_vec(),
    }
}

/// FR-527-AC-1, AC-2, AC-4, AC-6, AC-7 (TC-1322 rows 1, 2, 4, 6, 7): one
/// clause per row of FR-045's table, each matching that row and no other,
/// returns that row's disposition and target; the accumulating rows name
/// every matched dimension; a clause with no bounded operator takes the last
/// row under each false-extension profile; the fixed-sample future row alone
/// sets `total_sample_valuation`.
#[trace(
    "FR-527-AC-1",
    "FR-527-AC-2",
    "FR-527-AC-4",
    "FR-527-AC-6",
    "FR-527-AC-7",
    "TC-1322"
)]
#[test]
fn each_row_of_the_support_table_returns_its_disposition() {
    use Closure::{Complete, Open};
    use Target::{ClosedTrace, OnlinePrefix, OriginCompleteHistory};
    use Unmatched::{FiniteWindow, InfiniteTrace, MixedDirection};
    let cases: [(&str, &str, Closure, Support); 24] = [
        // Row 1: infinite trace.
        (
            INFINITE,
            "always holds(c.versionNumber = 1)",
            Complete,
            unsupported(&[InfiniteTrace]),
        ),
        (
            INFINITE,
            "always holds(c.versionNumber = 1)",
            Open,
            unsupported(&[InfiniteTrace]),
        ),
        // Row 2: finite window.
        (
            TIMESTAMPED,
            "always[1,9] holds(c.versionNumber = 1)",
            Complete,
            unsupported(&[FiniteWindow]),
        ),
        (
            TIMESTAMPED,
            "always[1,9] holds(c.versionNumber = 1)",
            Open,
            unsupported(&[FiniteWindow]),
        ),
        // Row 3: mixed direction.
        (EVENT, MIXED, Complete, unsupported(&[MixedDirection])),
        (FIXED, MIXED, Open, unsupported(&[MixedDirection])),
        // Accumulation across the first three rows.
        (
            TIMESTAMPED,
            MIXED,
            Complete,
            unsupported(&[FiniteWindow, MixedDirection]),
        ),
        (
            INFINITE,
            MIXED,
            Open,
            unsupported(&[InfiniteTrace, MixedDirection]),
        ),
        // Row 4: past only, whatever the closure.
        (
            EVENT,
            PAST,
            Complete,
            supported(OriginCompleteHistory, false),
        ),
        (EVENT, PAST, Open, supported(OriginCompleteHistory, false)),
        (
            FIXED,
            PAST,
            Complete,
            supported(OriginCompleteHistory, false),
        ),
        (FIXED, PAST, Open, supported(OriginCompleteHistory, false)),
        // Row 5 and 6: event-position future by closure.
        (EVENT, FUTURE, Complete, supported(ClosedTrace, false)),
        (EVENT, FUTURE, Open, supported(OnlinePrefix, false)),
        // Row 7: fixed-sample future, total valuation under both closures.
        (FIXED, FUTURE, Complete, supported(ClosedTrace, true)),
        (FIXED, FUTURE, Open, supported(OnlinePrefix, true)),
        // Row 8: no bounded operator under each false-extension profile.
        (EVENT, NONE, Complete, supported(ClosedTrace, false)),
        (EVENT, NONE, Open, supported(OnlinePrefix, false)),
        (FIXED, NONE, Complete, supported(ClosedTrace, false)),
        (FIXED, NONE, Open, supported(OnlinePrefix, false)),
        // Future operators the unbounded profile also carries.
        (
            INFINITE,
            "eventually holds(c.versionNumber = 1)",
            Complete,
            unsupported(&[InfiniteTrace]),
        ),
        (TIMESTAMPED, FUTURE, Open, unsupported(&[FiniteWindow])),
        (TIMESTAMPED, PAST, Complete, unsupported(&[FiniteWindow])),
        (INFINITE, PAST, Open, unsupported(&[InfiniteTrace])),
    ];
    for (profile, body, surrounding, expected) in cases {
        let classified = classify(profile, body, surrounding);
        assert_eq!(
            classified.support, expected,
            "{profile} {body} {surrounding:?}"
        );
    }
}

/// The four past operators and the forms they take in a clause body.
const PAST_OPERATORS: [&str; 4] = [
    "once[0,1] holds(c.versionNumber = 1)",
    "historically[0,1] holds(c.versionNumber = 1)",
    "holds(c.versionNumber = 1) since[0,1] holds(c.versionNumber = 2)",
    "holds(c.versionNumber = 1) triggered[0,1] holds(c.versionNumber = 2)",
];

/// FR-527-AC-3 (TC-1322 row 3): each of `once`, `historically`, `since` and
/// `triggered` alone, under each false-extension profile and each closure,
/// is `Supported` with `mltl.origin-complete-history/v1`; `once[1,1]`, strong
/// previous, is among them.
#[trace("FR-527-AC-3", "TC-1322")]
#[test]
fn each_past_operator_alone_maps_to_the_origin_complete_history_target() {
    let strong_previous = "once[1,1] holds(c.versionNumber = 1)";
    for body in PAST_OPERATORS.into_iter().chain([strong_previous]) {
        for profile in [EVENT, FIXED] {
            for surrounding in [Closure::Complete, Closure::Open] {
                let classified = classify(profile, body, surrounding);
                assert_eq!(
                    classified.support,
                    supported(Target::OriginCompleteHistory, false),
                    "{profile} {body}"
                );
                let Support::Supported { target, .. } = classified.support else {
                    panic!("supported");
                };
                assert_eq!(target.identity(), "mltl.origin-complete-history/v1");
            }
        }
    }
}

/// Each way the connectives wrap `formula`.
fn wrapped(formula: &str) -> [String; 4] {
    [
        format!("not ({formula})"),
        format!("({formula}) and {R}"),
        format!("({formula}) or {R}"),
        format!("{R} implies ({formula})"),
    ]
}

/// FR-527-AC-5 (TC-1322 row 5): `once[0,1] holds(p)` classifies as past-only,
/// `eventually[0,2] holds(q)` as future-only and a formula holding both as
/// mixed-direction, and `not`, `and`, `or` and `implies` change none of the
/// three.
#[trace("FR-527-AC-5", "TC-1322")]
#[test]
fn the_connectives_carry_no_direction() {
    let past = format!("once[0,1] {P}");
    let future = format!("eventually[0,2] {Q}");
    let both = format!("({past}) and ({future})");
    let cases = [
        (past, supported(Target::OriginCompleteHistory, false)),
        (future, supported(Target::ClosedTrace, false)),
        (both, unsupported(&[Unmatched::MixedDirection])),
    ];
    for (formula, expected) in cases {
        assert_eq!(
            classify(EVENT, &formula, Closure::Complete).support,
            expected,
            "{formula}"
        );
        for body in wrapped(&formula) {
            assert_eq!(
                classify(EVENT, &body, Closure::Complete).support,
                expected,
                "{body}"
            );
        }
    }
    // A connective alone, with no temporal operator, is the last row.
    for body in wrapped(P) {
        assert_eq!(
            classify(EVENT, &body, Closure::Open).support,
            supported(Target::OnlinePrefix, false),
            "{body}"
        );
    }
}

/// FR-527-AC-8 (TC-1322 row 8): an unsupported result holds the clause's
/// `CheckedNodeId`, profile selection and activation equal to the values read
/// from the view, and classification leaves the package unchanged.
#[trace("FR-527-AC-8", "TC-1322")]
#[test]
fn an_unsupported_result_retains_the_native_subject() {
    let compiled = compile(&unit_each());
    let package = admit(&compiled);
    let before = (*package).clone();
    let ids = clauses(&package);
    assert_eq!(ids.len(), 1);
    let view = temporal_subject(&package, &ids[0]).expect("reads");
    let classified = mapping_support(&view, Closure::Complete).expect("classifies");
    assert_eq!(classified.support, unsupported(&[Unmatched::InfiniteTrace]));
    assert_eq!(classified.retained.declaration, *view.declaration());
    assert_eq!(classified.retained.declaration, ids[0]);
    assert_eq!(classified.retained.profile, *view.profile());
    assert_eq!(
        classified.retained.profile.definition.identity.as_ref(),
        INFINITE
    );
    assert_eq!(classified.retained.activation, *view.activation());
    assert!(matches!(
        classified.retained.activation,
        Activation::Each { guard: Some(_), .. }
    ));
    assert_eq!(*package, before);
}

fn unit_each() -> String {
    unit(
        INFINITE,
        &declared_on(
            "Due",
            "on each (q: Config::ConfigVersion) when (c.versionNumber >= 3)",
            "always holds(c.versionNumber = 1)",
        ),
    )
}

/// FR-527-AC-9 (TC-1322 row 9): a supported result names the exact TL target
/// and the eight outstanding premises of FR-045, and holds nothing that
/// claims a correspondence record.
#[trace("FR-527-AC-9", "TC-1322")]
#[test]
fn a_supported_result_names_its_target_and_the_eight_premises() {
    let identities = [
        (FUTURE, Closure::Complete, "mltl.closed-trace/v1"),
        (FUTURE, Closure::Open, "mltl.online-prefix/v1"),
        (PAST, Closure::Complete, "mltl.origin-complete-history/v1"),
    ];
    for (body, surrounding, identity) in identities {
        let classified = classify(EVENT, body, surrounding);
        let Support::Supported {
            target, premises, ..
        } = classified.support
        else {
            panic!("supported: {body}");
        };
        assert_eq!(target.identity(), identity);
        assert_eq!(
            premises,
            [
                "total Boolean predicate projection",
                "source and clause identity",
                "model, type and predicate bindings",
                "evaluation anchor and immutable capture environment",
                "clock and observation binding",
                "interval",
                "closure and history premises",
                "result dimensions the selected TL wire does not encode",
            ]
        );
        let shown = format!("{:?}", classified.support);
        assert!(!shown.to_lowercase().contains("correspondence"), "{shown}");
    }
}

/// FR-527-AC-10 (TC-1322 row 10): the same formula bytes under different
/// profiles receive different classifications.
#[trace("FR-527-AC-10", "TC-1322")]
#[test]
fn equal_formula_bytes_under_different_profiles_classify_differently() {
    let body = "always[0,5] holds(c.versionNumber = 1)";
    let by_profile: Vec<Support> = [EVENT, FIXED, TIMESTAMPED, INFINITE]
        .into_iter()
        .map(|profile| classify(profile, body, Closure::Complete).support)
        .collect();
    assert_eq!(
        by_profile,
        [
            supported(Target::ClosedTrace, false),
            supported(Target::ClosedTrace, true),
            unsupported(&[Unmatched::FiniteWindow]),
            unsupported(&[Unmatched::InfiniteTrace]),
        ]
    );
    for (index, first) in by_profile.iter().enumerate() {
        assert!(by_profile[index + 1..].iter().all(|other| other != first));
    }
}

/// FR-527-AC-12 (TC-1322 row 12): changing `surrounding` changes the target of
/// FR-045's future rows and nothing else.
#[trace("FR-527-AC-12", "TC-1322")]
#[test]
fn the_closure_changes_only_the_future_rows_target() {
    let rows = [
        (INFINITE, "always holds(c.versionNumber = 1)", false),
        (TIMESTAMPED, "always[1,9] holds(c.versionNumber = 1)", false),
        (EVENT, MIXED, false),
        (EVENT, PAST, false),
        (FIXED, PAST, false),
        (EVENT, FUTURE, true),
        (FIXED, FUTURE, true),
        (EVENT, NONE, true),
        (FIXED, NONE, true),
    ];
    for (profile, body, closure_selects) in rows {
        let complete = classify(profile, body, Closure::Complete);
        let open = classify(profile, body, Closure::Open);
        if !closure_selects {
            assert_eq!(complete, open, "{profile} {body}");
            continue;
        }
        assert_eq!(complete.retained, open.retained);
        assert_eq!(complete.selection(), open.selection());
        let (
            Support::Supported {
                target: closed,
                total_sample_valuation: closed_valuation,
                premises: closed_premises,
            },
            Support::Supported {
                target: online,
                total_sample_valuation: online_valuation,
                premises: online_premises,
            },
        ) = (complete.support, open.support)
        else {
            panic!("supported: {profile} {body}");
        };
        assert_eq!(
            (closed, online),
            (Target::ClosedTrace, Target::OnlinePrefix)
        );
        assert_eq!(closed_valuation, online_valuation);
        assert_eq!(closed_premises, online_premises);
    }
}

/// The lock's `profile_selections` row of `profile`, read by the test.
fn lock_row(package: &CheckedPackageV2, profile: &str) -> quire_contract_model::CheckedSelection {
    let rows: Vec<_> = package
        .lock()
        .profile_selections
        .iter()
        .filter(|row| {
            row.role == CheckedSelectionRole::TemporalProfile
                && row.definition.identity.as_ref() == profile
        })
        .collect();
    let [row] = rows[..] else {
        panic!("one lock row for {profile}: {rows:?}");
    };
    row.clone()
}

/// FR-527-AC-13 (TC-1322 row 13): every result's `selection` holds the
/// package identity, the clause and the definition identity and artifact equal
/// to the values read from the lock row, supported or not. The selection has
/// exactly those three private fields, so it holds no clock parameter map.
#[trace("FR-527-AC-13", "TC-1322")]
#[test]
fn every_result_retains_the_selection_of_the_lock_row() {
    let cases = [
        (EVENT, FUTURE),
        (FIXED, PAST),
        (TIMESTAMPED, "always[1,9] holds(c.versionNumber = 1)"),
        (INFINITE, "always holds(c.versionNumber = 1)"),
    ];
    for (profile, body) in cases {
        let package = package_of(profile, body);
        let [id] = clauses(&package)[..]
            .to_vec()
            .try_into()
            .expect("one clause");
        let view = temporal_subject(&package, &id).expect("reads");
        for surrounding in [Closure::Complete, Closure::Open] {
            let classified = mapping_support(&view, surrounding).expect("classifies");
            let selection = classified.selection();
            let row = lock_row(&package, profile);
            assert_eq!(selection.package(), package.name());
            assert_eq!(*selection.clause(), id);
            assert_eq!(selection.definition_identity(), profile);
            assert_eq!(
                selection.definition_identity(),
                row.definition.identity.as_ref()
            );
            assert_eq!(*selection.definition_artifact(), row.definition);
        }
    }
}

/// FR-527-AC-16, AC-17 (TC-1322 rows 16, 17): a view whose profile row names a
/// definition identity outside the five admitted temporal profiles returns
/// `NoProfileSelection` under both closures, and a node that is not a
/// temporal clause is refused by `temporal_subject` with `UnresolvedClause`.
#[trace("FR-527-AC-16", "FR-527-AC-17", "TC-1322")]
#[test]
fn a_profile_row_outside_the_five_admitted_profiles_is_refused() {
    let package = package_of(INFINITE, "always holds(c.versionNumber = 1)");
    let [id] = clauses(&package)[..]
        .to_vec()
        .try_into()
        .expect("one clause");
    let view = temporal_subject(&package, &id).expect("reads");
    for identity in [
        "quire.temporal.unknown/v1",
        "quire.temporal.infinite-trace/v2",
    ] {
        let foreign = quire_contract_model::CheckedSelection {
            role: CheckedSelectionRole::TemporalProfile,
            definition: quire_contract_model::CheckedArtifactRef {
                authority: "agent-ix".into(),
                identity: identity.into(),
            },
        };
        let foreign_view = view.clone().with_profile(&foreign);
        for surrounding in [Closure::Complete, Closure::Open] {
            assert_eq!(
                mapping_support(&foreign_view, surrounding).err(),
                Some(MappingRefusal::NoProfileSelection),
                "{identity}"
            );
        }
    }
    assert!(mapping_support(&view, Closure::Complete).is_ok());

    // A node that is no temporal clause: the clause's `over` parameter.
    assert_eq!(
        temporal_subject(&package, view.over()).err(),
        Some(ViewRefusal::UnresolvedClause)
    );
}

/// FR-527-AC-21 (TC-1322 row 21): the reader refuses a package whose lock has
/// no `profile_selections` row for a temporal clause's `temporal_profile`
/// law, and one whose row names another identity than the clause's law, each
/// with `operation_law_unselected` at the clause's `laws/0/definition`, so no
/// such package yields a `TemporalSubjectView`.
#[trace("FR-527-AC-21", "TC-1322")]
#[test]
fn the_reader_refuses_a_package_with_no_profile_selection_row() {
    let compiled = compile(&unit(
        INFINITE,
        &declared("Due", "always holds(c.versionNumber = 1)"),
    ));
    assert!(
        matches!(read_wire_outcome(&compiled.wire), Read::Verified { .. }),
        "the unmodified wire reads"
    );
    let mut stripped = compiled.wire.clone();
    stripped["lock"]["profile_selections"]
        .as_array_mut()
        .expect("rows")
        .retain(|row| row["definition"]["identity"] != INFINITE);
    let mut other = compiled.wire.clone();
    for row in other["lock"]["profile_selections"]
        .as_array_mut()
        .expect("rows")
    {
        if row["definition"]["identity"] == INFINITE {
            row["definition"]["identity"] = Value::from("quire.temporal.unknown/v1");
        }
    }
    for wire in [stripped, other] {
        let Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) = read_wire_outcome(&wire)
        else {
            panic!("the reader refuses the package");
        };
        assert_eq!(
            refusal.cause,
            Some(quire_contract_model::CheckedPackageRefusalCause::OperationLawUnselected)
        );
        let path = refusal.loci[0]
            .path
            .as_ref()
            .expect("a pointer")
            .to_string();
        assert!(path.ends_with("/laws/0/definition"), "{path}");
    }
}

/// The outcome of reading `wire`'s canonical bytes.
fn read_wire_outcome(wire: &Value) -> Read {
    let bytes = quire_canonical::to_vec(wire, crate::checked::EMIT_LIMITS).expect("canonical");
    read_v2(&bytes, library(), V2ReadLimits::fixture(), &evidence(wire))
}

/// FR-527-AC-19 (TC-1322 row 19): classifying one clause twice gives equal
/// results.
#[trace("FR-527-AC-19", "TC-1322")]
#[test]
fn classifying_a_clause_twice_gives_equal_results() {
    let package = package_of(FIXED, FUTURE);
    let [id] = clauses(&package)[..]
        .to_vec()
        .try_into()
        .expect("one clause");
    let view = temporal_subject(&package, &id).expect("reads");
    let first = mapping_support(&view, Closure::Open).expect("classifies");
    let second = mapping_support(&view, Closure::Open).expect("classifies");
    assert_eq!(first, second);
    assert_eq!(first.support, supported(Target::OnlinePrefix, true));
    let again = temporal_subject(&package, &id).expect("reads");
    assert_eq!(
        mapping_support(&again, Closure::Open).expect("classifies"),
        first
    );
}

/// FR-527-AC-18 (TC-1322 row 18): a unit selecting a temporal profile outside
/// the five admitted identities is refused at check with FR-110's
/// `UnsupportedSelection`, so no clause node exists to classify.
#[trace("FR-527-AC-18", "TC-1322")]
#[test]
fn a_profile_outside_the_admitted_identities_is_refused_at_check() {
    use qsl_semantics::check::{resolve_profiles, ProfileCause};
    for identity in [
        "quire.temporal.unknown/v1",
        "quire.temporal.event-position.false-extension/v2",
        "quire.temporal.infinite-trace/v2",
    ] {
        let source = unit(
            identity,
            &declared("Due", "always holds(c.versionNumber = 1)"),
        );
        let parsed = qsl_cst::parse(
            qsl_foundation::SourceIdentity::new("agent-ix", "test:profile"),
            "program.native",
            source.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 admits the unit");
        let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let refused = resolve_profiles(&built.selections().profiles)
            .expect_err("the profile is refused at check");
        let causes: Vec<_> = refused.iter().map(|refusal| refusal.cause).collect();
        assert_eq!(causes, [ProfileCause::UnsupportedSelection], "{identity}");
        assert_eq!(refused[0].selected, identity);
        assert_eq!(refused[0].cause(), "unsupported-selection");
    }
    // The four admitted identities, and the timed profile FR-234 admits, pass.
    for identity in [EVENT, FIXED, TIMESTAMPED, INFINITE, TIMED] {
        let source = unit(
            identity,
            &declared("Due", "always holds(c.versionNumber = 1)"),
        );
        let parsed = qsl_cst::parse(
            qsl_foundation::SourceIdentity::new("agent-ix", "test:profile"),
            "program.native",
            source.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 admits the unit");
        let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        assert_eq!(
            resolve_profiles(&built.selections().profiles),
            Ok(()),
            "{identity}"
        );
    }
}

/// The clauses of a timed hybrid package: `SlowEnough` selected under
/// `quire.temporal.timed/v1` over the `Braking` hybrid model, read by the I2
/// reader, classified under both closures.
fn timed_classifications(body: &str) -> Vec<Classification> {
    timed_classifications_named("SlowEnough", body)
}

fn timed_classifications_named(name: &str, body: &str) -> Vec<Classification> {
    let (emission, _wire) = super::hybrid::compile(&format!(
        "{}{}",
        super::hybrid::braking(super::hybrid::FLOW),
        super::hybrid::slow_enough_named(name, "t", body)
    ));
    let Read::Verified { admitted, .. } = read_v2(
        emission.package.bytes(),
        LibraryName::new("example").expect("a library name"),
        V2ReadLimits::fixture(),
        &read_evidence(&emission),
    ) else {
        panic!("the I2 read admits the timed package");
    };
    let [id] = clauses(&admitted)[..]
        .to_vec()
        .try_into()
        .expect("one clause");
    let view = temporal_subject(&admitted, &id).expect("reads");
    assert_eq!(view.profile().definition.identity.as_ref(), TIMED);
    [Closure::Complete, Closure::Open]
        .map(|surrounding| {
            let classified = mapping_support(&view, surrounding).expect("classifies");
            assert_eq!(classified.retained.declaration, id);
            assert_eq!(classified.retained.profile, *view.profile());
            assert_eq!(classified.retained.name.as_deref(), view.name());
            assert_eq!(classified.selection().definition_identity(), TIMED);
            classified
        })
        .to_vec()
}

/// FR-527-AC-22 (TC-1322 row 22): a clause the reader admitted under
/// `quire.temporal.timed/v1` returns `Unsupported` naming `TimedProfile` and
/// no TL target, under both closures.
#[trace("FR-527-AC-22", "TC-1322")]
#[test]
fn the_timed_profile_is_unsupported_on_its_own_dimension() {
    for classified in timed_classifications("always [0, 10] holds(h.v < 30)") {
        assert_eq!(classified.support, unsupported(&[Unmatched::TimedProfile]));
    }
}

/// FR-527-AC-23 (TC-1322 row 23): a timed clause holding a past and a future
/// operator returns `Unsupported` naming `TimedProfile` and `MixedDirection`.
#[trace("FR-527-AC-23", "TC-1322")]
#[test]
fn a_timed_clause_that_mixes_directions_names_both_dimensions() {
    let body = "once [0, 1] holds(h.v < 30) and eventually [0, 2] holds(h.v < 30)";
    for classified in timed_classifications(body) {
        assert_eq!(
            classified.support,
            unsupported(&[Unmatched::TimedProfile, Unmatched::MixedDirection])
        );
    }
}

/// FR-527-AC-24 (TC-1322 row 24): supported and unsupported results retain the
/// view's unique name; a shared declaration has no single retained name.
#[trace("FR-527-AC-24", "TC-1322")]
#[test]
fn a_result_retains_the_name_the_view_reads() {
    let supported = classify_named(
        INFINITE,
        "Gamma",
        "always holds(c.versionNumber = 1)",
        Closure::Complete,
    );
    assert_eq!(supported.retained.name.as_deref(), Some("Gamma"));

    let [unsupported, _] = timed_classifications_named("Delta", "always[0,10] holds(h.v < 30)")
        .try_into()
        .expect("both closures classify");
    assert_eq!(unsupported.retained.name.as_deref(), Some("Delta"));
    assert!(matches!(unsupported.support, Support::Unsupported { .. }));

    let body = "always holds(c.versionNumber = 1)";
    let shared = admit(&compile(&unit(
        INFINITE,
        &format!("{} {}", declared("Alpha", body), declared("Beta", body)),
    )));
    let [id] = clauses(&shared)[..]
        .to_vec()
        .try_into()
        .expect("one shared node");
    let view = temporal_subject(&shared, &id).expect("shared view");
    let classified = mapping_support(&view, Closure::Complete).expect("classifies");
    assert_eq!(classified.retained.name, None);
    assert_eq!(classified.retained.name.as_deref(), view.name());
}

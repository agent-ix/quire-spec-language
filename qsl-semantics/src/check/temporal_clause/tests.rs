// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-326 (TC-836): the `TemporalTrace` family's `check` over real
//! complete-V1 source: S1 (`qsl_cst::parse`), S2 (`qsl_forms::build_unit`),
//! the assembler, then [`TemporalTraceFamily::check`] directly.

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind, TemporalInterval, TemporalUpper};
use qsl_foundation::diagnostic::{Code, Locus, StageFailure};
use qsl_foundation::{SourceIdentity, Span};
use quire_exact::NodeKey;
use quire_semantic_value::checking::CheckingLimits;

use super::*;
use crate::check::PackageDeclarations;
use crate::family::{DiagnosticSink, ScopeStack, StageLimits};

const VALUE: &str = "quire.value.complete/v1";
const EVENT: &str = "quire.temporal.event-position.false-extension/v1";
const FIXED: &str = "quire.temporal.fixed-sample.false-extension/v1";
const INFINITE: &str = "quire.temporal.infinite-trace/v1";
const TIMESTAMPED: &str = "quire.temporal.timestamped-event.finite-window/v1";

/// A unit selecting the value profile and `temporal` profiles, declaring a
/// bounded `Counter` and an unbounded `Loose`, with one temporal clause
/// `C` over a `Counter` (or `over`) whose formula is `formula`.
fn source(temporal: &[&str], over: &str, formula: &str) -> String {
    let mut text =
        format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"{VALUE}\";\n");
    for (index, identity) in temporal.iter().enumerate() {
        text.push_str(&format!("profile t{index} = \"{identity}\";\n"));
    }
    let using = if temporal.is_empty() { "v" } else { "t0" };
    text.push_str(&format!(
        "record Counter {{ level: Int[0, 9]; }}\n\
         record Loose {{ n: Integer; }}\n\
         temporal C using {using} over (c: {over}) clock \"model-steps\" on origin \
         {{ {formula} }}\n"
    ));
    text
}

fn timestamped_source(formula: &str) -> String {
    source(&[TIMESTAMPED], "Int[0,9]", formula)
        .replace("over (c: Int[0,9])", "over (c: Quantity<ms>)")
        .replace(
            "temporal C using",
            "dimension Time;\nunit s : Time = rational(1, 1);\nunit ms : Time = rational(1, 1000) * s;\ndimension Length;\nunit m : Length = rational(1, 1);\ntemporal C using",
        )
}

fn assemble(text: &str) -> PackageDeclarations {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        crate::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the unit assembles")
}

type Outcome = Result<CheckedTemporalClause, StageFailure<CheckRefusal>>;

/// The unit's one clause, checked by the family.
fn check(package: &PackageDeclarations) -> Outcome {
    let units = crate::check::time_units::TimeUnits::new(
        &package.units,
        &package.nominal_spans,
        &package.models,
        &package.dimension_equivalences,
        &package.source,
        &package.package_name,
    );
    let scope = Scope::new(
        package.types.clone(),
        package.operations.clone(),
        package.enums.clone(),
        package.aliases.clone(),
        Vec::new(),
        None,
        Vec::new(),
    );
    let declarations = TemporalDeclarations {
        scope: &scope,
        selections: &package.temporal_selections,
        source: &package.source,
        package_name: &package.package_name,
        embedding: None,
        position_limit: 10_000,
        models: &package.models,
        protocols: &[],
        timed: TimedContext {
            units: &units,
            time_sources: &package.time_sources,
            rational_bits: CheckingLimits::default().rational_bits(),
            clock_units: &package.clock_units,
        },
    };
    let mut meter = quire_exact::Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let mut diagnostics = DiagnosticSink::default();
    let mut scopes = ScopeStack::default();
    let mut cx = CheckContext::new(
        &declarations,
        StageLimits {
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        },
        &mut meter,
        &mut diagnostics,
        &mut scopes,
    );
    let [clause] = package.temporal_clauses.as_slice() else {
        panic!("one clause: {:?}", package.temporal_clauses);
    };
    TemporalTraceFamily::check(clause, &mut cx).map(qsl_foundation::diagnostic::Staged::into_value)
}

fn checked(temporal: &[&str], formula: &str) -> (String, CheckedTemporalClause) {
    let text = source(temporal, "Counter", formula);
    match check(&assemble(&text)) {
        Ok(clause) => (text, clause),
        Err(failure) => panic!("{formula}: the clause checks, not {failure:?}"),
    }
}

/// The cause the clause refuses with.
fn refused(temporal: &[&str], formula: &str) -> (String, TemporalCause) {
    let text = source(temporal, "Counter", formula);
    match check(&assemble(&text)) {
        Err(StageFailure::Refused(CheckRefusal {
            cause: CheckCause::Temporal(cause),
            ..
        })) => (text, *cause),
        other => panic!("{formula}: a temporal refusal, not {other:?}"),
    }
}

/// The span of the first occurrence of `needle` in `text`.
fn first(text: &str, needle: &str) -> Span {
    let start = text.find(needle).expect("the needle is in the unit");
    Span {
        start,
        end: start + needle.len(),
    }
}

fn node(byte: u8) -> NodeKey {
    NodeKey::new(
        "fixture".to_owned(),
        vec![quire_exact::PathSegment::Text(format!("node-{byte}"))],
        0,
    )
    .expect("a nonempty structural fixture key")
}

/// FR-326-AC-1: under infinite-trace `eventually` alone checks with no
/// interval and no horizon, and its record is (`temporal-satisfaction`,
/// `Unbounded`) with exactly one infinite-trace-formula domain keyed by the
/// clause node; under event-position the same clause refuses
/// `unsupported_construct`/`expression-form` at `eventually`'s span.
#[trace("TC-836", "FR-326-AC-1")]
#[test]
fn a_bare_operator_is_unbounded_under_infinite_trace_and_refused_under_a_bounded_profile() {
    let (_, clause) = checked(&[INFINITE], "eventually holds(c.level = 3)");
    assert_eq!(clause.profile(), TemporalProfile::InfiniteTrace);
    assert_eq!(clause.horizon(), None);
    let [eventually] = clause.operators() else {
        panic!("one operator: {:?}", clause.operators());
    };
    assert_eq!(eventually.operator, TemporalOperator::Eventually);
    assert_eq!(eventually.interval, None);

    let [claim] = TemporalTraceFamily::requirements(&clause)
        .try_into()
        .expect("one claim");
    let record = claim
        .requirements(node(1), &[node(2)], &[], &[])
        .expect("the record is keyed");
    assert_eq!(record.kind(), Capability::TemporalSatisfaction);
    let ClaimExtent::Unbounded(domains) = record.extent() else {
        panic!("unbounded: {:?}", record.extent());
    };
    assert_eq!(
        domains.iter().collect::<Vec<_>>(),
        [(
            &DomainKey::Node {
                node: node(1),
                path: Vec::new()
            },
            DomainKind::InfiniteTrace
        )]
    );

    let (text, cause) = refused(&[EVENT], "eventually holds(c.level = 3)");
    assert_eq!(
        cause,
        TemporalCause::MissingInterval {
            span: first(&text, "eventually")
        }
    );
    assert_eq!(cause.code(), Code::UnsupportedConstruct);
    assert_eq!(cause.tag(), "expression-form");
}

/// FR-326-AC-2: event-position `always[0,2] eventually[1,3] holds(..)` has
/// horizon 5; `eventually[3,*]` refuses `unsupported_construct`/
/// `expression-form` at the interval there and checks as `Some{3, Open}`
/// under infinite-trace; `eventually[5,3]` refuses `ill_typed`/
/// `type-mismatch` at the interval under both.
#[trace("TC-836", "FR-326-AC-2")]
#[test]
fn intervals_are_admitted_by_profile_and_the_horizon_is_the_greatest_reach() {
    let (_, nested) = checked(&[EVENT], "always[0,2] eventually[1,3] holds(c.level = 3)");
    assert_eq!(nested.horizon(), Some(5));
    let intervals: Vec<_> = nested
        .operators()
        .iter()
        .map(|operator| operator.interval.clone())
        .collect();
    assert_eq!(
        intervals,
        [
            Some(CheckedInterval::Steps(
                TemporalInterval::new(0, TemporalUpper::Finite(2)).unwrap()
            )),
            Some(CheckedInterval::Steps(
                TemporalInterval::new(1, TemporalUpper::Finite(3)).unwrap()
            )),
        ]
    );

    let formula = "eventually[3,*] holds(c.level = 3)";
    let (text, cause) = refused(&[EVENT], formula);
    assert_eq!(
        cause,
        TemporalCause::OpenInterval {
            span: first(&text, "[3,*]")
        }
    );
    assert_eq!(cause.code(), Code::UnsupportedConstruct);
    assert_eq!(cause.tag(), "expression-form");
    let (_, open) = checked(&[INFINITE], formula);
    assert_eq!(
        open.operators()[0].interval,
        Some(CheckedInterval::Steps(
            TemporalInterval::new(3, TemporalUpper::Open).unwrap()
        ))
    );
    assert_eq!(open.horizon(), None);

    for profile in [EVENT, INFINITE] {
        let (text, cause) = refused(&[profile], "eventually[5,3] holds(c.level = 3)");
        assert_eq!(
            cause,
            TemporalCause::InvertedInterval {
                lower: 5,
                upper: 3,
                span: first(&text, "[5,3]")
            },
            "{profile}"
        );
        assert_eq!(cause.code(), Code::IllTyped);
        assert_eq!(cause.tag(), "type-mismatch");
    }
}

/// FR-326-AC-3: a horizon sum past `u64` refuses with a work-budget stage
/// limit located at the operator whose interval overflowed.
#[trace("TC-836", "FR-326-AC-3")]
#[test]
fn a_horizon_past_u64_is_a_work_budget_limit_at_the_overflowing_operator() {
    let text = source(
        &[EVENT],
        "Counter",
        "eventually[0,18446744073709551615] eventually[0,1] holds(c.level = 3)",
    );
    let package = assemble(&text);
    let Err(StageFailure::Limit(limit)) = check(&package) else {
        panic!("a stage limit");
    };
    assert_eq!(limit.setting(), Setting::S3WorkUnits);
    assert_eq!(
        limit.kind(),
        qsl_foundation::diagnostic::LimitKind::WorkBudget
    );
    assert_eq!(limit.actual(), u128::from(u64::MAX) + 1);
    let expected = crate::check::region::region(&package.source, None, first(&text, "eventually"));
    assert_eq!(
        limit.locus(),
        expected.map(Locus::Region).as_ref(),
        "located at the outer operator"
    );
}

/// FR-326-AC-4: a unit holding a temporal clause with no temporal profile
/// selection, or two, refuses `unknown_profile`/`unsupported-selection` at
/// the clause, naming the selections it found.
#[trace("TC-836", "FR-326-AC-4")]
#[test]
fn the_unit_selects_exactly_one_temporal_profile() {
    for (profiles, found) in [
        (&[][..], Vec::new()),
        (&[EVENT, INFINITE][..], vec![EVENT, INFINITE]),
    ] {
        let (text, cause) = refused(profiles, "eventually[0,1] holds(c.level = 3)");
        let start = text.find("temporal C").expect("the clause");
        let end = text.rfind('}').expect("the clause's close") + 1;
        assert_eq!(
            cause,
            TemporalCause::ProfileSelection {
                found,
                span: Span { start, end }
            }
        );
        assert_eq!(cause.code(), Code::UnknownProfile);
        assert_eq!(cause.tag(), "unsupported-selection");
    }
}

/// FR-326-AC-5: a bounded-profile clause over a type with no unbounded
/// position has the record (`temporal-satisfaction`, `Bounded`); the
/// interval `[0,2]` keys unequal under event-position and fixed-sample, and
/// under infinite-trace.
#[trace("TC-836", "FR-326-AC-5")]
#[test]
fn a_bounded_clause_over_a_bounded_type_is_bounded_and_keys_carry_the_profile() {
    let formula = "eventually[0,2] holds(c.level = 3)";
    let key = |profile| {
        let (_, clause) = checked(&[profile], formula);
        let [claim] = TemporalTraceFamily::requirements(&clause)
            .try_into()
            .expect("one claim");
        if profile != INFINITE {
            let record = claim
                .requirements(node(1), &[node(2)], &[], &[])
                .expect("keyed");
            assert_eq!(record.kind(), Capability::TemporalSatisfaction);
            assert_eq!(record.extent(), &ClaimExtent::Bounded);
        }
        let node = clause.operators()[0].node;
        clause.interval_key(node).expect("a finite interval keys")
    };
    let event = key(EVENT);
    let fixed = key(FIXED);
    let infinite = key(INFINITE);
    assert_eq!((event.lower(), event.upper()), (0, 2));
    assert_ne!(event, fixed);
    assert_ne!(event, infinite);
    assert_eq!(event, key(EVENT));
}

/// FR-326 "Requirement record": a type position with no finite bound in the
/// `over` parameter's type is an unbounded domain beside the formula's.
#[trace("FR-326")]
#[test]
fn the_over_parameters_unbounded_positions_join_the_record() {
    let text = source(&[INFINITE], "Loose", "eventually holds(c.n = 3)");
    let clause = check(&assemble(&text)).expect("the clause checks");
    let [claim] = TemporalTraceFamily::requirements(&clause)
        .try_into()
        .expect("one claim");
    let record = claim
        .requirements(node(1), &[node(2)], &[], &[])
        .expect("keyed");
    let ClaimExtent::Unbounded(domains) = record.extent() else {
        panic!("unbounded: {:?}", record.extent());
    };
    let mut kinds: Vec<_> = domains.iter().map(|(_, kind)| kind).collect();
    kinds.sort();
    assert_eq!(kinds, [DomainKind::Integer, DomainKind::InfiniteTrace]);
    assert_eq!(
        claim.requirements(node(1), &[], &[], &[]),
        Err(InternalFault::new(STAGE, "parameter-node-keyed"))
    );

    let bounded = source(&[EVENT], "Loose", "eventually[0,1] holds(c.n = 3)");
    let clause = check(&assemble(&bounded)).expect("the clause checks");
    let [claim] = TemporalTraceFamily::requirements(&clause)
        .try_into()
        .expect("one claim");
    let record = claim
        .requirements(node(1), &[node(2)], &[], &[])
        .expect("keyed");
    let ClaimExtent::Unbounded(domains) = record.extent() else {
        panic!("a bounded profile over an unbounded type is unbounded");
    };
    assert_eq!(domains.len(), 1);
}

/// FR-326: an `over` type that names nothing refuses at its type form.
#[trace("FR-326")]
#[test]
fn an_unresolved_over_type_refuses_at_the_type() {
    let text = source(&[EVENT], "Missing", "eventually[0,1] holds(true)");
    let Err(StageFailure::Refused(CheckRefusal {
        cause: CheckCause::Temporal(cause),
        ..
    })) = check(&assemble(&text))
    else {
        panic!("a temporal refusal");
    };
    assert_eq!(cause.code(), Code::MissingDeclaration);
    assert_eq!(cause.span(), first(&text, "Missing"));
}

/// The one temporal refusal `text`'s package check gives.
fn package_refusal(text: &str) -> TemporalCause {
    let refusals = assemble(text)
        .check(CheckingLimits::default())
        .expect_err("the clause is refused");
    let [CheckRefusal {
        cause: CheckCause::Temporal(cause),
        ..
    }] = refusals.as_slice()
    else {
        panic!("one temporal refusal for `{text}`: {refusals:?}");
    };
    (**cause).clone()
}

/// FR-326, FR-337: a clause that activates `on origin` or `on each`, with or
/// without a capture, checks and compiles into the package, keeping its
/// captures; a clause that does not check reports its own refusal.
#[trace("FR-326", "FR-337-AC-7")]
#[test]
fn the_package_check_refuses_a_clause_no_emission_writes() {
    let origin = source(&[INFINITE], "Counter", "eventually holds(c.level = 3)");
    let each = origin.replace("on origin", "on each (d: Counter) when (d.level = 1)");
    let captured = origin.replace(
        "{ eventually",
        "{ capture before: Int[0, 9] = c.level; eventually",
    );
    for (text, captures) in [
        (origin.clone(), 0),
        (each, 0),
        (captured.clone(), 1),
        (captured.replace("on origin", "on each (d: Counter)"), 1),
    ] {
        let graph = assemble(&text)
            .check(CheckingLimits::default())
            .expect("an emitted clause compiles");
        let [clause] = graph.temporal_clauses() else {
            panic!("one emitted clause");
        };
        assert_eq!(clause.clause().name(), "C");
        assert_eq!(clause.clause().captures().len(), captures);
    }

    let bounded = source(&[EVENT], "Counter", "eventually holds(c.level = 3)");
    assert!(matches!(
        package_refusal(&bounded),
        TemporalCause::MissingInterval { .. }
    ));
}

/// The package check of `text` admits its one temporal clause.
fn package_admits(text: &str) {
    let graph = assemble(text)
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("`{text}` checks: {refusals:?}"));
    assert_eq!(graph.temporal_clauses().len(), 1, "{text}");
}

/// FR-194-AC-12 (TC-629): a carried aggregate value is refused
/// `ill_typed`/`type-mismatch` at the value before any charge, its threshold
/// is still checked against `Quantity<u>` and reported after it, a carried
/// threshold is refused, and the value converted back to `Quantity<u>` checks.
#[trace("TC-629", "FR-194-AC-12")]
#[test]
fn a_carried_aggregate_value_or_threshold_is_refused_at_the_carried_expression() {
    use crate::check::lowering::{temporal_location, AggregatePart, TemporalSite};
    let carried = |formula: &str| {
        source(&[EVENT], "Quantity<m>", formula).replace(
            "temporal C using",
            "dimension Length;\nunit m : Length = rational(1, 1);\n\
             dimension Time;\nunit s : Time = rational(1, 1);\n\
             type Lc = Quantity<m, Int[-5, 5]>;\ntemporal C using",
        )
    };
    let at = |part| temporal_location(0, TemporalSite::Aggregate(0, part));
    let refused = |text: &str| {
        assemble(text)
            .check(CheckingLimits::default())
            .expect_err("the carried expression is refused at S3")
            .into_iter()
            .map(|refusal| {
                (
                    refusal.cause.code(),
                    refusal.cause.cause().map(str::to_owned),
                    refusal.location,
                )
            })
            .collect::<Vec<_>>()
    };
    for kind in ["sum of", "min of", "max of", "quantile 0.5 of"] {
        assert_eq!(
            refused(&carried(&format!(
                "{kind} convert<Lc>(c) over past[0,2] <= 5 m"
            ))),
            [(
                Code::IllTyped,
                Some("type-mismatch".to_owned()),
                at(AggregatePart::Value)
            )],
            "{kind}"
        );
        assert_eq!(
            refused(&carried(&format!(
                "{kind} convert<Lc>(c) over past[0,2] <= 5 s"
            ))),
            [
                (
                    Code::IllTyped,
                    Some("type-mismatch".to_owned()),
                    at(AggregatePart::Value)
                ),
                (
                    Code::IllTyped,
                    Some("unit-mismatch".to_owned()),
                    at(AggregatePart::Threshold)
                ),
            ],
            "{kind}"
        );
    }
    assert_eq!(
        refused(&carried("sum of c over past[0,2] <= convert<Lc>(5 m)")),
        [(
            Code::IllTyped,
            Some("type-mismatch".to_owned()),
            at(AggregatePart::Threshold)
        )]
    );
    package_admits(&carried(
        "sum of convert<Quantity<m>>(convert<Lc>(c)) over past[0,2] <= 5 m",
    ));
}

/// FR-194-AC-11 (TC-629): over an event field `Quantity<M::ms>` of a selected
/// domain package, `sum`, `min`, `max` and `quantile` keep the package unit,
/// a threshold in the package's `ps` converts into it, and a threshold in a
/// unit of the source's own `Time` is a unit mismatch at the threshold. (The
/// package units carry names the source does not, `pms` and `ps`: the
/// hand-built package model resolves an unqualified unit name to its own.)
#[trace("TC-629", "FR-194-AC-11")]
#[test]
fn a_window_aggregate_over_a_package_unit_keeps_that_unit() {
    use crate::check::lowering::{temporal_location, AggregatePart, TemporalSite};
    use crate::model::domain_package::{
        DomainPackage, DomainPackageRef, PackageDimensionRecord, PackageUnitRecord,
    };
    use crate::model::intake::SelectedModels;
    use quire_checked::key::DeclarationKey;

    let time = DeclarationKey::fixture("dimension/PTime");
    let (s, ms) = (
        DeclarationKey::fixture("unit/ps"),
        DeclarationKey::fixture("unit/pms"),
    );
    let mut model = DomainPackage::new(DomainPackageRef::fixture("rpc"), vec![]);
    model.dimensions.push(PackageDimensionRecord {
        key: time.clone(),
        name: "PTime".to_owned(),
        base: true,
        terms: vec![],
    });
    for (key, name, target, scale) in [(s.clone(), "ps", None, "1"), (ms, "pms", Some(s), "1/1000")]
    {
        model.units.push(PackageUnitRecord {
            key,
            name: name.to_owned(),
            dimension: time.clone(),
            target,
            scale: scale.to_owned(),
            offset: "0".to_owned(),
        });
    }
    let package = |formula: &str| {
        let text = source(&[EVENT], "E", formula)
            .replace("over (c: E)", "over (e: E)")
            .replace(
                "record Counter",
                "model M = \"test/orders\";\n\
             dimension Time;\nunit s : Time = rational(1, 1);\n\
             unit ms : Time = rational(1, 1000) * s;\n\
             record E { latency: Quantity<M::pms>; }\n\
record Counter",
            );
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let selected = SelectedModels::fixture(
            "M",
            Span { start: 0, end: 1 },
            model.clone(),
            crate::model::accounting::ModelNormalizationLimits::UNLIMITED,
        )
        .expect("the package model admits");
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "fixture".to_owned(),
            unit,
            selected,
            Vec::new(),
        )
        .expect("the unit assembles")
        .check(CheckingLimits::default())
    };
    for formula in [
        "sum of e.latency over past[0,2] <= 1 M::ps",
        "min of e.latency over past[0,2] <= 1 M::pms",
        "max of e.latency over past[0,2] <= 1 M::pms",
        "quantile 0.5 of e.latency over past[0,2] <= 1 M::pms",
        "sum of e.latency where holds(e.latency > 9 M::pms) over past[0,2] min_count 0 = 0 M::pms",
    ] {
        package(formula).unwrap_or_else(|refusals| panic!("{formula}: {refusals:?}"));
    }
    for threshold in ["10 ms", "10 s"] {
        let Err(refusals) = package(&format!("sum of e.latency over past[0,2] <= {threshold}"))
        else {
            panic!("`{threshold}`, of the source's own Time, is another dimension");
        };
        assert_eq!(refusals.len(), 1, "{threshold}: {refusals:?}");
        assert_eq!(refusals[0].cause.code(), Code::IllTyped, "{threshold}");
        assert_eq!(
            refusals[0].cause.cause(),
            Some("unit-mismatch"),
            "{threshold}"
        );
        assert_eq!(
            refusals[0].location,
            temporal_location(0, TemporalSite::Aggregate(0, AggregatePart::Threshold)),
            "{threshold}"
        );
    }
}

/// FR-194-AC-4 (TC-629): aggregate atoms and their expressions are checked
/// at S3, and a clause whose atoms check is admitted into the package.
#[trace("TC-629", "FR-194-AC-4", "FR-194-AC-7", "FR-194-AC-8", "FR-194-AC-10")]
#[test]
fn aggregate_atoms_are_checked_and_admitted() {
    package_admits(&source(&[EVENT], "Int[0,9]", "count over past[0,3] >= 1"));
    package_admits(&source(
        &[EVENT],
        "Int[0,9]",
        "count over past[0,3] min_count 0 >= 1",
    ));

    let invalid_quantile = source(&[EVENT], "Int[0,9]", "quantile 1 of c over past[0,3] <= 1");
    assert!(matches!(
        package_refusal(&invalid_quantile),
        TemporalCause::InvalidAggregateValue { .. }
    ));

    let zero_minimum = source(
        &[EVENT],
        "Int[0,9]",
        "max of c over past[0,3] min_count 0 <= 1",
    );
    assert!(matches!(
        package_refusal(&zero_minimum),
        TemporalCause::InvalidAggregateValue { .. }
    ));

    let mixed_window = source(&[EVENT], "Int[0,9]", "count over past[5,2 s] >= 1");
    let mixed_window_refusal = package_refusal(&mixed_window);
    assert_eq!(mixed_window_refusal.code(), Code::IllTyped);
    assert_eq!(mixed_window_refusal.tag(), "unit-mismatch");
    assert!(matches!(
        mixed_window_refusal,
        TemporalCause::AggregateWindowKindMismatch { .. }
    ));

    let inverted_window = source(&[EVENT], "Int[0,9]", "count over past[5,2] >= 1");
    assert!(matches!(
        package_refusal(&inverted_window),
        TemporalCause::InvalidAggregateWindow { .. }
    ));

    let several_faults = source(
        &[EVENT],
        "Int[0,9]",
        "quantile 1 of c over past[5,2 s] min_count 0 <= 1",
    );
    let TemporalCause::Multiple { causes } = package_refusal(&several_faults) else {
        panic!("all aggregate faults must be reported");
    };
    assert_eq!(
        causes
            .iter()
            .map(|cause| (cause.code(), cause.tag()))
            .collect::<Vec<_>>(),
        [
            (Code::InvalidPackage, "invalid-value"),
            (Code::IllTyped, "unit-mismatch"),
            (Code::InvalidPackage, "invalid-value"),
        ]
    );

    package_admits(&timestamped_source("count over past[0 s, 1000 ms] >= 1"));
    let inverted_duration = timestamped_source("count over past[2 s, 1000 ms] >= 1");
    assert!(matches!(
        package_refusal(&inverted_duration),
        TemporalCause::InvalidAggregateWindow { .. }
    ));
    // FR-194: a bound of another dimension is no duration, a unit
    // mismatch at its unit, whichever bound it is.
    // FR-194-AC-7: at the unit `m` of the first such bound.
    for (window, unit_at) in [("past[0 m, 1 m]", 7), ("past[0 s, 1 m]", 12)] {
        let text = timestamped_source(&format!("count over {window} >= 1"));
        let refusal = package_refusal(&text);
        assert_eq!(
            (refusal.code(), refusal.tag()),
            (Code::IllTyped, "unit-mismatch"),
            "{window}"
        );
        let start = first(&text, window).start + unit_at;
        assert_eq!(
            refusal,
            TemporalCause::AggregateWindowKindMismatch {
                span: Span {
                    start,
                    end: start + 1
                }
            },
            "{window}"
        );
    }

    // The threshold may use another unit of the value's dimension.
    package_admits(&timestamped_source(
        "sum of c over past[0 s, 1000 ms] <= 1000 ms",
    ));
    let wrong_threshold_dimension = timestamped_source("sum of c over past[0 s, 1000 ms] <= 1 m");
    // FR-194-AC-7: `5 m` against a time value is a unit mismatch at the
    // threshold.
    let refusals = assemble(&wrong_threshold_dimension)
        .check(CheckingLimits::default())
        .expect_err("a threshold of another dimension is refused at S3");
    assert_eq!(refusals[0].cause.code(), Code::IllTyped);
    assert_eq!(refusals[0].cause.cause(), Some("unit-mismatch"));
    assert_eq!(
        refusals[0].location,
        crate::check::lowering::temporal_location(
            0,
            crate::check::lowering::TemporalSite::Aggregate(
                0,
                crate::check::lowering::AggregatePart::Threshold
            )
        )
    );

    // An exact fraction `n/d` threshold is the rational literal `n/d`; on
    // `count` it is of another family.
    package_admits(&source(
        &[EVENT],
        "Int[0,9]",
        "fraction holds(c > 1) over past[0,3] >= 1/2",
    ));
    let rational_count = source(&[EVENT], "Int[0,9]", "count over past[0,3] >= 1/2");
    let refusals = assemble(&rational_count)
        .check(CheckingLimits::default())
        .expect_err("a rational count threshold is refused");
    assert_eq!(
        refusals[0].cause,
        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
    );

    let ill_typed = source(
        &[EVENT],
        "Int[0,9]",
        "count where holds(c) over past[0,3] >= 1",
    );
    let refusals = assemble(&ill_typed)
        .check(CheckingLimits::default())
        .expect_err("a non-Boolean aggregate filter is refused");
    assert_eq!(refusals[0].cause.code(), Code::IllTyped);
    assert!(matches!(refusals[0].cause, CheckCause::NonBooleanRoot));

    let mismatched_threshold = source(&[EVENT], "Int[0,9]", "sum of c over past[0,3] <= true");
    let refusals = assemble(&mismatched_threshold)
        .check(CheckingLimits::default())
        .expect_err("a threshold must match the aggregate value type");
    assert!(matches!(
        refusals[0].cause,
        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
    ));

    let nonnumeric_count_threshold = source(&[EVENT], "Int[0,9]", "count over past[0,3] >= true");
    let refusals = assemble(&nonnumeric_count_threshold)
        .check(CheckingLimits::default())
        .expect_err("a count threshold is dimensionless numeric");
    assert!(matches!(
        refusals[0].cause,
        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
    ));

    // QSpec FR-370's families: a value of family integer, rational or
    // quantity; a threshold of the aggregate's result family, an integer
    // literal standing for the rational of its value where a rational is
    // required.
    package_admits(&source(
        &[EVENT],
        "Int[0,9]",
        "fraction holds(c > 1) over past[0,3] >= 1",
    ));
    package_admits(&source(&[EVENT], "Int[0,9]", "count over past[0,3] >= c"));
    for formula in [
        "sum of (c > 1) over past[0,3] <= 1",
        "fraction holds(c > 1) over past[0,3] >= c",
    ] {
        let refusals = assemble(&source(&[EVENT], "Int[0,9]", formula))
            .check(CheckingLimits::default())
            .expect_err(formula);
        assert!(
            matches!(
                refusals[0].cause,
                CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
            ),
            "{formula}: {refusals:?}"
        );
    }

    let bounded = check(&assemble(&source(
        &[EVENT],
        "Int[0,9]",
        "always[0,2] count over past[0,3] >= 1",
    )))
    .expect("the family checks the formula before S4 emission");
    assert_eq!(bounded.horizon(), Some(2));
    assert!(matches!(
        bounded.formula().nodes().get(1),
        Some(FormulaNode::Aggregate { .. })
    ));
}

/// FR-337-AC-15: a `duration` window lowers to quantity literals of its
/// lower bound's time unit, both bounds converted exactly (`past[0 ms,
/// 300 s]` is 0 and 300000 of `ms`), bound as `window_kind`, `lower` and
/// `upper` of the aggregate argument in QSpec FR-370's order.
#[trace("TC-1267", "FR-337-AC-15")]
#[test]
fn a_duration_window_lowers_to_quantities_of_its_unit() {
    use crate::check::node_key::{BodyTerm, GroupMember, LeafTerm, LiteralValue, MemberTerm};
    let graph = assemble(&timestamped_source(
        "always[0,2] (quantile 0.95 of c over past[0 ms, 300 s] <= 5 ms)",
    ))
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("the clause checks: {refusals:?}"));
    let [clause] = graph.temporal_clauses() else {
        panic!("one clause");
    };
    let [unit] = clause.clause().duration_window_units().collect::<Vec<_>>()[..] else {
        panic!("one duration window");
    };
    let arguments = graph
        .semantic_graph()
        .nodes()
        .find_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity == "quire.op.temporal.aggregate" =>
            {
                Some(application.arguments.clone())
            }
            _ => None,
        })
        .expect("an aggregate node");
    let [MemberTerm::Group(argument)] = &arguments[..] else {
        panic!("one aggregate argument: {arguments:?}");
    };
    let members: Vec<(&str, Option<&LiteralValue>, Option<NodeKey>)> = argument
        .members
        .iter()
        .map(|member| match member {
            GroupMember::Binding(binding) => match &binding.value {
                LeafTerm::Literal { ty, value } => {
                    (binding.name.as_str(), Some(value), ty.admitted().cloned())
                }
                _ => (binding.name.as_str(), None, None),
            },
            GroupMember::Leaf(_) => panic!("a binding: {member:?}"),
        })
        .collect();
    assert_eq!(
        members.iter().map(|(name, ..)| *name).collect::<Vec<_>>(),
        [
            "kind",
            "q",
            "value",
            "window_kind",
            "lower",
            "upper",
            "min_count",
            "comparison",
            "threshold"
        ]
    );
    let quantity = |name: &str| {
        members
            .iter()
            .find(|(bound, ..)| *bound == name)
            .map(|(_, value, ty)| (value.cloned(), ty.clone()))
            .unwrap_or_else(|| panic!("a binding {name}"))
    };
    let ms = |value: i64| {
        (
            Some(LiteralValue::Quantity(quire_exact::Rational::from_integer(
                quire_exact::Integer::from(value),
            ))),
            Some(unit.clone()),
        )
    };
    assert_eq!(quantity("lower"), ms(0));
    assert_eq!(quantity("upper"), ms(300_000));
    assert_eq!(quantity("threshold"), ms(5));

    // A threshold in another unit of the value's dimension is converted
    // to the value's unit: a reference to its converted node.
    let graph = assemble(&timestamped_source(
        "always[0,2] (quantile 0.95 of c over past[0 ms, 300 s] <= 1 s)",
    ))
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("the clause checks: {refusals:?}"));
    let converted = graph
        .semantic_graph()
        .nodes()
        .find_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity == "quire.op.temporal.aggregate" =>
            {
                match application.arguments.first() {
                    Some(MemberTerm::Group(argument)) => argument.members.last().cloned(),
                    _ => None,
                }
            }
            _ => None,
        })
        .expect("an aggregate threshold");
    let GroupMember::Binding(threshold) = converted else {
        panic!("a threshold binding");
    };
    assert_eq!(threshold.name, "threshold");
    let LeafTerm::Reference { target } = &threshold.value else {
        panic!("a reference to the converted threshold: {threshold:?}");
    };
    let node = target
        .admitted()
        .and_then(|key| graph.semantic_graph().node(key.clone()))
        .expect("the converted threshold node");
    assert_eq!(
        node.semantic_type(),
        Some(unit.clone()),
        "the threshold is in the value's unit"
    );
}

/// FR-337-AC-15: `s3.rational_bits` bounds a `duration` bound's converted
/// value, the literal emitted, as well as its written magnitude: `past[0 ms,
/// 255 s]` converts `255 s` (8 bits written) to `255000` of `ms`, 18 bits,
/// so under a ceiling of 16 it refuses `LimitExceeded(S3RationalBits)` at
/// `255 s`, and under a ceiling of 18 it checks.
#[trace("TC-1267", "FR-337-AC-15")]
#[test]
fn a_converted_duration_bound_past_the_rational_bits_ceiling_is_a_limit() {
    let text = timestamped_source("count over past[0 ms, 255 s] >= 1");
    let within = |bits| CheckingLimits::default().with_rational_bits(bits);
    let refusals = assemble(&text)
        .check(within(16))
        .expect_err("the converted bound is past the ceiling");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
        panic!("a stage limit: {refusal:?}");
    };
    assert_eq!(limit.kind, crate::check::CheckingLimitKind::RationalBits);
    assert_eq!((limit.limit, limit.actual), (16, 18));
    let package = assemble(&text);
    let expected = crate::check::region::region(&package.source, None, first(&text, "255 s"));
    assert_eq!(limit.region, expected);
    assemble(&text)
        .check(within(18))
        .unwrap_or_else(|refusals| panic!("within the ceiling: {refusals:?}"));
}

/// FR-337-AC-7: a guard whose divisor has no S3 law checks, the guard is
/// kept beside the `holds` conditions, and an unguarded `on each` clause
/// keeps none.
#[trace("TC-1265", "FR-337-AC-7")]
#[test]
fn an_on_each_guard_is_kept_with_the_emitted_clause() {
    let origin = source(&[INFINITE], "Counter", "eventually holds(c.level = 3)");
    for (activation, guarded) in [
        ("on each (d: Counter) when (6 mod (2 - c.level) = 0)", true),
        ("on each (d: Counter) when (d.level = c.level)", true),
        ("on each (d: Counter)", false),
        ("on origin", false),
    ] {
        let text = origin
            .replace("on origin", activation)
            // A complete-value function in the unit admits integer `mod`.
            .replace(
                "record Counter",
                "function Noop using v(): Boolean pure { true }\nrecord Counter",
            );
        let graph = assemble(&text)
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{activation}: {refusals:?}"));
        let [clause] = graph.temporal_clauses() else {
            panic!("one emitted clause");
        };
        assert_eq!(clause.guard().is_some(), guarded, "{activation}");
        if let Some((_, slots)) = clause.guard() {
            assert!(slots >= 2, "{activation}: the guard binds `c` and `d`");
        }
    }
}

/// FR-325 and FR-337: S2 preserves indexed-read syntax, while an ordinary
/// temporal clause has no bound trace variable and refuses it at S3.
#[trace("FR-325", "FR-337")]
#[test]
fn ordinary_temporal_hyper_read_refuses_outside_a_hyper_clause() {
    let text = source(&[INFINITE], "Counter", "holds(c.level @ a = 3)");
    let package = assemble(&text);
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("an ordinary temporal clause cannot read a trace variable");
    assert!(
        refusals.iter().any(|refusal| {
            matches!(&refusal.cause, CheckCause::Hyper(cause)
            if cause.code() == Code::UnsupportedConstruct
                && cause.tag() == "expression-form")
        }),
        "{refusals:?}"
    );
}

/// FR-337: a `holds` condition is typed over the clause's `over` parameter
/// as a Boolean: a condition naming no binding refuses at its own source,
/// and a non-Boolean condition refuses as a non-Boolean root.
#[trace("FR-337")]
#[test]
fn a_holds_condition_is_typed_over_the_over_parameter() {
    let text = source(&[INFINITE], "Counter", "eventually holds(d.level = 3)");
    let package = assemble(&text);
    let regions = package.regions();
    let refusals = package
        .check(CheckingLimits::default())
        .expect_err("an unbound name is refused");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    assert!(
        matches!(&refusal.cause, CheckCause::MissingName(name) if name == "d"),
        "{refusal:?}"
    );
    let region = regions.refusal_region(refusal).expect("a located refusal");
    let at = text.find("d.level").expect("the read");
    assert_eq!(region.start(), u64::try_from(at).unwrap());

    let text = source(&[INFINITE], "Counter", "eventually holds(c.level)");
    let refusals = assemble(&text)
        .check(CheckingLimits::default())
        .expect_err("a non-Boolean condition is refused");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(refusal.cause, CheckCause::NonBooleanRoot, "{refusal:?}");
}

/// FR-233 and FR-326: a timed interval (FR-393) where the clause counts
/// steps, here under `model-steps` with infinite-trace and event-position,
/// refuses `ill_typed`/`type-mismatch` at the interval, rather than being
/// checked as a step interval or dropped.
#[trace("TC-1078", "TC-688", "FR-233-AC-2")]
#[test]
fn a_timed_interval_under_a_step_count_refuses_type_mismatch_at_the_interval() {
    let formula = "eventually[0 ms, 3 ms) holds(c.level = 3)";
    for profile in [INFINITE, EVENT] {
        let (text, cause) = refused(&[profile], formula);
        assert_eq!(
            cause,
            TemporalCause::TimedIntervalUnderStepCount {
                span: first(&text, "[0 ms, 3 ms)"),
            },
            "{profile}"
        );
        assert_eq!(cause.code(), Code::IllTyped);
        assert_eq!(cause.tag(), "type-mismatch");
    }
}

/// FR-326-AC-6: under the finite-window profile `eventually[0,5] holds(p)`
/// checks with the step interval `[0,5]`, held as integral closed ticks;
/// each well-formed timed interval, the unit-bearing, rational, open-ended
/// and `[3 ms, *]` forms, refuses `ill_typed`/`type-mismatch` at the
/// interval's span; a step interval `[3,*]` refuses `unsupported_construct`/
/// `expression-form`; the aggregate window `count over past[0 ms, 300 ms]
/// min_count 1 >= 1` is a duration and checks.
#[trace("TC-836", "FR-326-AC-6")]
#[test]
fn the_finite_window_profile_admits_only_integral_closed_tick_intervals() {
    let (_, clause) = checked(&[TIMESTAMPED], "eventually[0,5] holds(c.level = 3)");
    let intervals: Vec<_> = clause
        .operators()
        .iter()
        .filter_map(|operator| operator.interval.clone())
        .collect();
    assert_eq!(
        intervals,
        [CheckedInterval::Steps(
            TemporalInterval::new(0, TemporalUpper::Finite(5)).expect("an ordered interval")
        )]
    );
    assert_eq!(clause.horizon(), Some(5));

    for interval in [
        "[0 ms, 5 ms]",
        "[0 ms, 5 ms)",
        "[0.5 ms, 5 ms]",
        "(1 ms, 5 ms]",
        "[3 ms, *]",
    ] {
        let (text, cause) = refused(
            &[TIMESTAMPED],
            &format!("eventually{interval} holds(c.level = 3)"),
        );
        assert_eq!(
            cause,
            TemporalCause::TimedIntervalUnderStepCount {
                span: first(&text, interval),
            },
            "{interval}"
        );
        assert_eq!(
            (cause.code(), cause.tag()),
            (Code::IllTyped, "type-mismatch"),
            "{interval}"
        );
    }

    let (text, cause) = refused(&[TIMESTAMPED], "eventually[3,*] holds(c.level = 3)");
    assert_eq!(
        cause,
        TemporalCause::OpenInterval {
            span: first(&text, "[3,*]"),
        }
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::UnsupportedConstruct, "expression-form")
    );

    package_admits(&timestamped_source(
        "count over past[0 ms, 300 ms] min_count 1 >= 1",
    ));
}

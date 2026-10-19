// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-326 (TC-836): the `TemporalTrace` family's `check` over real
//! complete-V1 source: S1 (`qsl_cst::parse`), S2 (`qsl_forms::build_unit`),
//! the assembler, then [`TemporalTraceFamily::check`] directly.

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind, TemporalInterval, TemporalUpper};
use qsl_foundation::diagnostic::{Code, Locus, StageFailure};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::{SourceIdentity, Span};
use quire_semantic_value::checking::CheckingLimits;

use super::*;
use crate::check::PackageDeclarations;
use crate::family::{DiagnosticSink, ScopeStack, StageLimits};

const VALUE: &str = "quire.value.complete/v1";
const EVENT: &str = "quire.temporal.event-position.false-extension/v1";
const FIXED: &str = "quire.temporal.fixed-sample.false-extension/v1";
const INFINITE: &str = "quire.temporal.infinite-trace/v1";

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

fn assemble(text: &str) -> PackageDeclarations {
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        crate::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the unit assembles")
}

type Outcome = Result<CheckedTemporalClause, StageFailure<CheckRefusal>>;

/// The unit's one clause, checked by the family.
fn check(package: &PackageDeclarations) -> Outcome {
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
        embedding: None,
        position_limit: 10_000,
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

fn node(byte: u8) -> WireNodeId {
    WireNodeId::from_digest([byte; 32])
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
        .requirements(node(1), &[node(2)])
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
        .map(|operator| operator.interval)
        .collect();
    assert_eq!(
        intervals,
        [
            Some(TemporalInterval::new(0, TemporalUpper::Finite(2)).unwrap()),
            Some(TemporalInterval::new(1, TemporalUpper::Finite(3)).unwrap()),
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
        Some(TemporalInterval::new(3, TemporalUpper::Open).unwrap())
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
            let record = claim.requirements(node(1), &[node(2)]).expect("keyed");
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
    let record = claim.requirements(node(1), &[node(2)]).expect("keyed");
    let ClaimExtent::Unbounded(domains) = record.extent() else {
        panic!("unbounded: {:?}", record.extent());
    };
    let mut kinds: Vec<_> = domains.iter().map(|(_, kind)| kind).collect();
    kinds.sort();
    assert_eq!(kinds, [DomainKind::Integer, DomainKind::InfiniteTrace]);
    assert_eq!(
        claim.requirements(node(1), &[]),
        Err(InternalFault::new(STAGE, "parameter-node-keyed"))
    );

    let bounded = source(&[EVENT], "Loose", "eventually[0,1] holds(c.n = 3)");
    let clause = check(&assemble(&bounded)).expect("the clause checks");
    let [claim] = TemporalTraceFamily::requirements(&clause)
        .try_into()
        .expect("one claim");
    let record = claim.requirements(node(1), &[node(2)]).expect("keyed");
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
        panic!("one temporal refusal: {refusals:?}");
    };
    (**cause).clone()
}

/// FR-326, FR-337: a clause that activates `on origin` and declares no
/// capture checks and compiles into the package; one that activates `on
/// each`, or declares a capture, checks but no S4 emission writes it, so
/// `check` refuses it as `unsupported_construct`/`not-yet-implemented` at
/// the clause; a clause that does not check reports its own refusal.
#[trace("FR-326")]
#[test]
fn the_package_check_refuses_a_clause_no_emission_writes() {
    let graph = assemble(&source(
        &[INFINITE],
        "Counter",
        "eventually holds(c.level = 3)",
    ))
    .check(CheckingLimits::default())
    .expect("an `on origin` clause compiles");
    let [clause] = graph.temporal_clauses() else {
        panic!("one emitted clause");
    };
    assert_eq!(clause.clause().name(), "C");

    let origin = source(&[INFINITE], "Counter", "eventually holds(c.level = 3)");
    let each = origin.replace("on origin", "on each (d: Counter) when (d.level = 1)");
    let captured = origin.replace(
        "{ eventually",
        "{ capture before: Int[0, 9] = c.level; eventually",
    );
    for text in [each, captured] {
        let cause = package_refusal(&text);
        assert!(
            matches!(cause, TemporalCause::NotEmitted { .. }),
            "{cause:?}"
        );
        assert_eq!(cause.code(), Code::UnsupportedConstruct);
        assert_eq!(cause.tag(), "not-yet-implemented");
        assert_eq!(cause.span().start, text.find("temporal C").expect("clause"));
    }

    let bounded = source(&[EVENT], "Counter", "eventually holds(c.level = 3)");
    assert!(matches!(
        package_refusal(&bounded),
        TemporalCause::MissingInterval { .. }
    ));
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

/// FR-393 leaves a timed interval's meaning to S3 (FR-233, FR-234). Until
/// that check lands, a clause with one refuses `unsupported_construct`/
/// `not-yet-implemented` at the interval under every profile, rather than
/// checking it as a step interval or dropping it.
#[trace("TC-1078")]
#[test]
fn a_timed_interval_refuses_not_yet_implemented_at_the_interval() {
    let formula = "eventually[0 ms, 3 ms) holds(c.level = 3)";
    for profile in [INFINITE, EVENT] {
        let (text, cause) = refused(&[profile], formula);
        assert_eq!(
            cause,
            TemporalCause::TimedIntervalNotChecked {
                span: first(&text, "[0 ms, 3 ms)"),
            },
            "{profile}"
        );
        assert_eq!(cause.code(), Code::UnsupportedConstruct);
        assert_eq!(cause.tag(), "not-yet-implemented");
    }
}

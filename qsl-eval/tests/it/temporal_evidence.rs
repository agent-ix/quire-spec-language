// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-522 and FR-523 (TC-1317, TC-1318): the temporal evidence call's clock
//! binding, result identity and thirteen work counters.
//!
//! Clauses are real complete-V1 source taken through S1, S2 and the
//! `TemporalTrace` family's check. The checked package's rows (the lock's
//! profile selection and the declaration's `temporal_bindings` row) are the
//! rows a recipe package carries, built from their wire form.

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    CallMeter, ClauseSelection, ClauseStop, ClauseWork, EvidenceCounter, EvidenceStop,
    EvidenceUsage, IdentityBuilder, IdentityMember, MemberEncoder, MismatchedDimension, NodeReach,
    NodeShape, ReachFault, TemporalEvidenceLimits, TraceBinding, Visit,
};
use qsl_eval::temporal::{
    evaluate, AtomReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalTrace,
};
use qsl_package::{CheckedNodeId, CheckedSelection, ClockConfiguration, TemporalBinding};
use qsl_semantics::check::{FormulaNodeId, SCALAR_LIMITS_UNLIMITED};
use quire_exact::{ChargePoint, LimitKind, Meter, ScalarLimits};
use serde_json::{json, Value};

use crate::temporal_trace::{fixture, fixture_within};

const FIXED_PROFILE: &str = "quire.temporal.fixed-sample.false-extension/v1";
const EVENT_PROFILE: &str = "quire.temporal.event-position.false-extension/v1";
const TIMESTAMPED_PROFILE: &str = "quire.temporal.timestamped-event.finite-window/v1";

fn node(name: &str) -> CheckedNodeId {
    serde_json::from_value(json!({"package": "recipe", "path": [name], "ordinal": 0}))
        .expect("a node identity")
}

fn profile_row(identity: &str) -> CheckedSelection {
    serde_json::from_value(json!({
        "role": "temporal_profile",
        "definition": {"authority": "agent-ix", "identity": identity},
    }))
    .expect("a profile selection")
}

fn number(decimal: &str) -> Value {
    json!({"kind": "integer", "decimal": decimal})
}

fn fixed(epoch: &str, period: &str, unit: &str) -> Value {
    json!({"kind": "fixed_sample", "epoch": number(epoch), "period": number(period), "unit": unit})
}

fn event(authority: &str) -> Value {
    json!({"kind": "event_position", "sequence_authority": authority})
}

fn timestamped(unit: &str) -> Value {
    json!({"kind": "timestamped_event", "timestamp_unit": unit})
}

fn clock(value: Value) -> ClockConfiguration {
    serde_json::from_value(value).expect("a clock configuration")
}

fn binding_row(declaration: &str, name: &str, configuration: Value) -> TemporalBinding {
    serde_json::from_value(json!({
        "declaration": {"package": "recipe", "path": [declaration], "ordinal": 0},
        "occurrence": 0,
        "clock_name": name,
        "clock": configuration,
    }))
    .expect("a temporal binding row")
}

fn asserted(profile: &str, name: &str, configuration: Value) -> TraceBinding {
    TraceBinding {
        profile: profile.into(),
        clock_name: name.into(),
        parameters: clock(configuration),
    }
}

/// The recipe's three clock kinds: the lock's profile identity, the row and
/// the binding that agrees with both.
struct Recipe {
    declaration: CheckedNodeId,
    profile: CheckedSelection,
    row: TemporalBinding,
    agreeing: TraceBinding,
}

impl Recipe {
    fn selection(&self) -> ClauseSelection<'_> {
        ClauseSelection {
            declaration: &self.declaration,
            profile: &self.profile,
            clock: Some(&self.row),
        }
    }
}

fn fixed_sample() -> Recipe {
    Recipe {
        declaration: node("DueSample"),
        profile: profile_row(FIXED_PROFILE),
        row: binding_row("DueSample", "sample-clock", fixed("0", "10", "ms")),
        agreeing: asserted(FIXED_PROFILE, "sample-clock", fixed("0", "10", "ms")),
    }
}

fn event_position() -> Recipe {
    Recipe {
        declaration: node("Due"),
        profile: profile_row(EVENT_PROFILE),
        row: binding_row("Due", "order-clock", event("order-events")),
        agreeing: asserted(EVENT_PROFILE, "order-clock", event("order-events")),
    }
}

fn timestamped_event() -> Recipe {
    Recipe {
        declaration: node("DueTimestamp"),
        profile: profile_row(TIMESTAMPED_PROFILE),
        row: binding_row("DueTimestamp", "stamp-clock", timestamped("ms")),
        agreeing: asserted(TIMESTAMPED_PROFILE, "stamp-clock", timestamped("ms")),
    }
}

fn refused(recipe: &Recipe, binding: &TraceBinding) -> MismatchedDimension {
    binding
        .authenticate(&recipe.selection())
        .expect_err("the binding differs from the row")
        .dimension
}

/// FR-522-AC-1: another profile identity is refused at stage `admit` with
/// `invalid_runtime_input`/`invalid-value`, `Profile`, both identities and
/// no authenticated binding.
#[trace("FR-522-AC-1", "TC-1317")]
#[test]
fn another_profile_is_refused_naming_both_identities() {
    let recipe = fixed_sample();
    let mut binding = recipe.agreeing.clone();
    binding.profile = EVENT_PROFILE.into();
    let mismatch = binding
        .authenticate(&recipe.selection())
        .expect_err("another profile");
    assert_eq!(mismatch.dimension, MismatchedDimension::Profile);
    assert_eq!(mismatch.asserted, EVENT_PROFILE);
    assert_eq!(mismatch.expected.as_deref(), Some(FIXED_PROFILE));
    assert_eq!(mismatch.stage(), "admit");
    let code = mismatch.catalog_code();
    assert_eq!(
        (code.code(), code.cause()),
        ("invalid_runtime_input", "invalid-value")
    );
}

/// FR-522-AC-2, AC-3, AC-6: a differing clock name, another clock
/// alternative, a differing sequence authority and a differing timestamp
/// unit are each refused with their own dimension.
#[trace("FR-522-AC-2", "FR-522-AC-3", "FR-522-AC-6", "TC-1317")]
#[test]
fn a_clock_name_an_alternative_and_a_member_each_name_their_dimension() {
    let recipe = event_position();
    let mut binding = recipe.agreeing.clone();
    binding.clock_name = "another-clock".into();
    assert_eq!(refused(&recipe, &binding), MismatchedDimension::Clock);

    let recipe = fixed_sample();
    let mut binding = recipe.agreeing.clone();
    binding.parameters = clock(event("order-events"));
    let mismatch = binding
        .authenticate(&recipe.selection())
        .expect_err("another alternative");
    assert_eq!(mismatch.dimension, MismatchedDimension::ClockParameters);
    assert_eq!(mismatch.asserted, "event_position");
    assert_eq!(mismatch.expected.as_deref(), Some("fixed_sample"));

    let recipe = event_position();
    let mut binding = recipe.agreeing.clone();
    binding.parameters = clock(event("another-authority"));
    assert_eq!(
        refused(&recipe, &binding),
        MismatchedDimension::SequenceAuthority
    );

    let recipe = timestamped_event();
    let mut binding = recipe.agreeing.clone();
    binding.parameters = clock(timestamped("s"));
    assert_eq!(
        refused(&recipe, &binding),
        MismatchedDimension::TimestampUnit
    );
}

/// FR-522-AC-4, AC-5: a fixed-sample request that differs in `epoch`,
/// `period` and `unit` alone is refused with `Epoch`, `SamplePeriod` and
/// `ClockUnit`, and one that differs in `epoch` and `unit` is refused with
/// `Epoch`.
#[trace("FR-522-AC-4", "FR-522-AC-5", "TC-1317")]
#[test]
fn a_fixed_sample_member_names_the_first_differing_dimension() {
    let recipe = fixed_sample();
    let with = |parameters: Value| {
        let mut binding = recipe.agreeing.clone();
        binding.parameters = clock(parameters);
        binding
    };
    assert_eq!(
        refused(&recipe, &with(fixed("1", "10", "ms"))),
        MismatchedDimension::Epoch
    );
    assert_eq!(
        refused(&recipe, &with(fixed("0", "1", "ms"))),
        MismatchedDimension::SamplePeriod
    );
    assert_eq!(
        refused(&recipe, &with(fixed("0", "10", "us"))),
        MismatchedDimension::ClockUnit
    );
    assert_eq!(
        refused(&recipe, &with(fixed("1", "10", "us"))),
        MismatchedDimension::Epoch
    );
    // The period named in the use case: 1 ms asserted against 10 ms.
    let mismatch = with(fixed("0", "1", "ms"))
        .authenticate(&recipe.selection())
        .expect_err("another period");
    assert_eq!(mismatch.asserted, "1");
    assert_eq!(mismatch.expected.as_deref(), Some("10"));
}

/// FR-522-AC-7, AC-8: the dimensions are tested in the order `Profile`,
/// `Clock`, `ClockParameters`: a wrong profile and clock name report
/// `Profile`, and a wrong clock name and parameter report `Clock`.
#[trace("FR-522-AC-7", "FR-522-AC-8", "TC-1317")]
#[test]
fn the_first_differing_dimension_in_the_fixed_order_is_reported() {
    let recipe = fixed_sample();
    let mut binding = recipe.agreeing.clone();
    binding.profile = EVENT_PROFILE.into();
    binding.clock_name = "another-clock".into();
    assert_eq!(refused(&recipe, &binding), MismatchedDimension::Profile);

    let mut binding = recipe.agreeing.clone();
    binding.clock_name = "another-clock".into();
    binding.parameters = clock(fixed("0", "1", "ms"));
    assert_eq!(refused(&recipe, &binding), MismatchedDimension::Clock);

    let mut binding = recipe.agreeing.clone();
    binding.parameters = clock(event("order-events"));
    assert_eq!(
        refused(&recipe, &binding),
        MismatchedDimension::ClockParameters
    );
}

/// A binding that agrees is authenticated, and the retained binding holds
/// the package, the declaration, the profile definition, the clock name and
/// the clock configuration of the rows. No report carries it yet, so
/// FR-522-AC-9's report half is not claimed here.
#[trace("TC-1317")]
#[test]
fn an_agreeing_binding_retains_the_rows_it_agreed_with() {
    for recipe in [fixed_sample(), event_position(), timestamped_event()] {
        let authenticated = recipe
            .agreeing
            .authenticate(&recipe.selection())
            .expect("the binding agrees");
        assert_eq!(&*authenticated.package, "recipe");
        assert_eq!(authenticated.declaration, recipe.declaration);
        assert_eq!(authenticated.profile, recipe.profile.definition);
        assert_eq!(authenticated.clock_name, recipe.row.clock_name);
        assert_eq!(authenticated.clock, recipe.row.clock);
    }
}

/// FR-522-AC-10, AC-11: a refused binding is decided before the meter is
/// charged and before any stamp or position is read, so the meter's charge
/// log is empty and a trace with no positions reports the clock, not an
/// empty trace.
#[trace("FR-522-AC-10", "FR-522-AC-11", "TC-1317")]
#[test]
fn a_refused_binding_charges_nothing_and_precedes_the_trace() {
    let recipe = timestamped_event();
    let mut binding = recipe.agreeing.clone();
    binding.clock_name = "another-clock".into();
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let call = CallMeter::new(TemporalEvidenceLimits::default(), &mut meter);
    // Authentication takes the selection and the binding alone: no meter,
    // no stamp and no position, so no trace condition can decide first.
    let mismatch = binding
        .authenticate(&recipe.selection())
        .expect_err("another clock name");
    assert_eq!(mismatch.dimension, MismatchedDimension::Clock);
    assert_eq!(call.usage(), EvidenceUsage::default());
    drop(call);
    assert!(meter.admitted_charges().is_empty());
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 0);
}

/// FR-522-AC-12: a clause whose declaration has no `temporal_bindings` row
/// refuses a binding with `Clock`, and no other configuration is
/// substituted.
#[trace("FR-522-AC-12", "TC-1317")]
#[test]
fn a_clause_without_a_clock_row_refuses_any_binding() {
    let recipe = event_position();
    let selection = ClauseSelection {
        clock: None,
        ..recipe.selection()
    };
    let mismatch = recipe
        .agreeing
        .authenticate(&selection)
        .expect_err("no clock row");
    assert_eq!(mismatch.dimension, MismatchedDimension::Clock);
    assert_eq!(mismatch.asserted, "order-clock");
    assert_eq!(mismatch.expected, None);
}

/// The members of one request that the identity covers, with the owners'
/// members encoded as opaque bytes.
#[derive(Clone)]
struct Request {
    recipe_row: TemporalBinding,
    profile: &'static str,
    cover: Vec<u8>,
    stamp: u64,
    valuation: bool,
    order_key: u64,
    watermark: u64,
    through: u64,
    scope_closed: bool,
    execution_closed: bool,
    lower_boundary: u8,
    retained: u64,
    ledger_entry: u64,
    ceilings: TemporalEvidenceLimits,
    meter_ceiling: u64,
}

impl Request {
    fn fixed_sample() -> Self {
        Self {
            recipe_row: binding_row("DueSample", "sample-clock", fixed("0", "10", "ms")),
            profile: FIXED_PROFILE,
            cover: MemberEncoder::new().flag(false).finish(),
            stamp: 7,
            valuation: true,
            order_key: 3,
            watermark: 100,
            through: 100,
            scope_closed: true,
            execution_closed: true,
            lower_boundary: 0,
            retained: 1,
            ledger_entry: 1,
            ceilings: TemporalEvidenceLimits::default(),
            meter_ceiling: 1_000_000,
        }
    }

    fn identity(&self) -> IdentityBuilder {
        let declaration = node("DueSample");
        let profile = profile_row(self.profile);
        let selection = ClauseSelection {
            declaration: &declaration,
            profile: &profile,
            clock: Some(&self.recipe_row),
        };
        let binding = TraceBinding {
            profile: self.profile.into(),
            clock_name: self.recipe_row.clock_name.clone(),
            parameters: self.recipe_row.clock.clone(),
        };
        let authenticated = binding
            .authenticate(&selection)
            .expect("the request agrees with its own row");
        IdentityBuilder::new()
            .clause(&declaration)
            .expect("the clause encodes")
            .authenticated(&authenticated)
            .expect("the binding encodes")
            .member(IdentityMember::Cover, self.cover.clone())
            .member(
                IdentityMember::Trace,
                MemberEncoder::new()
                    .u64(self.stamp)
                    .flag(self.valuation)
                    .u64(self.order_key)
                    .finish(),
            )
            .member(
                IdentityMember::Progress,
                MemberEncoder::new()
                    .u64(self.watermark)
                    .u64(self.through)
                    .flag(self.scope_closed)
                    .flag(self.execution_closed)
                    .u64(u64::from(self.lower_boundary))
                    .finish(),
            )
            .member(
                IdentityMember::RetainedInstances,
                MemberEncoder::new().u64(self.retained).finish(),
            )
            .member(
                IdentityMember::ProgressLedger,
                MemberEncoder::new().u64(self.ledger_entry).finish(),
            )
            .ceilings(&self.ceilings)
            .meter_ceiling(self.meter_ceiling)
    }
}

/// The identity builder gives one request one identity, however often it is
/// built, and the three clauses' requests have pairwise distinct identities.
#[trace("TC-1317")]
#[test]
fn a_request_has_one_identity_and_three_clauses_have_three() {
    let first = Request::fixed_sample().identity().result_identity();
    assert_eq!(first, Request::fixed_sample().identity().result_identity());
    let identities: Vec<_> = [fixed_sample(), event_position(), timestamped_event()]
        .iter()
        .map(|recipe| {
            let authenticated = recipe
                .agreeing
                .authenticate(&recipe.selection())
                .expect("agrees");
            IdentityBuilder::new()
                .clause(&recipe.declaration)
                .expect("encodes")
                .authenticated(&authenticated)
                .expect("encodes")
                .result_identity()
        })
        .collect();
    for (index, left) in identities.iter().enumerate() {
        for right in identities.iter().skip(index + 1) {
            assert_ne!(left, right);
        }
    }
}

fn changed(edit: impl FnOnce(&mut Request)) -> IdentityBuilder {
    let mut request = Request::fixed_sample();
    edit(&mut request);
    request.identity()
}

/// Changing exactly one member of the identity list gives a distinct
/// identity, each member varied alone, and the earlier identity is unchanged
/// by building the changed one. The trace, progress and ledger members are
/// bytes this test chooses, since no evaluator produces them yet, so this
/// proves the builder only and claims none of FR-522-AC-15 to AC-19, AC-22
/// or AC-23.
#[trace("TC-1317")]
#[test]
fn each_member_of_the_identity_list_changes_the_identity_alone() {
    let base = Request::fixed_sample().identity().result_identity();
    let mut variants: Vec<(String, IdentityBuilder)> = vec![
        (
            "epoch".into(),
            changed(|r| {
                r.recipe_row = binding_row("DueSample", "sample-clock", fixed("1", "10", "ms"))
            }),
        ),
        (
            "period".into(),
            changed(|r| {
                r.recipe_row = binding_row("DueSample", "sample-clock", fixed("0", "20", "ms"))
            }),
        ),
        (
            "unit".into(),
            changed(|r| {
                r.recipe_row = binding_row("DueSample", "sample-clock", fixed("0", "10", "us"))
            }),
        ),
        (
            "profile".into(),
            changed(|r| r.profile = "another-profile/v1"),
        ),
        (
            "clock name".into(),
            changed(|r| {
                r.recipe_row = binding_row("DueSample", "other-clock", fixed("0", "10", "ms"))
            }),
        ),
        (
            "cover".into(),
            changed(|r| r.cover = MemberEncoder::new().flag(true).u64(2).finish()),
        ),
        ("stamp".into(), changed(|r| r.stamp += 1)),
        ("valuation".into(), changed(|r| r.valuation = false)),
        ("order key".into(), changed(|r| r.order_key += 1)),
        ("watermark".into(), changed(|r| r.watermark += 1)),
        ("through".into(), changed(|r| r.through += 1)),
        ("scope closure".into(), changed(|r| r.scope_closed = false)),
        (
            "surrounding closure".into(),
            changed(|r| r.execution_closed = false),
        ),
        ("lower boundary".into(), changed(|r| r.lower_boundary = 1)),
        ("retained instance".into(), changed(|r| r.retained += 1)),
        ("progress ledger".into(), changed(|r| r.ledger_entry += 1)),
        ("meter ceiling".into(), changed(|r| r.meter_ceiling -= 1)),
    ];
    for counter in EvidenceCounter::ALL {
        variants.push((
            format!("ceiling {}", counter.name()),
            changed(|r| {
                r.ceilings = r
                    .ceilings
                    .with_ceiling(counter, r.ceilings.ceiling(counter) - 1);
            }),
        ));
    }
    assert_eq!(variants.len(), 30);
    for (name, builder) in &variants {
        assert_ne!(builder.result_identity(), base, "{name}");
    }
    for (index, (name, left)) in variants.iter().enumerate() {
        for (other, right) in variants.iter().skip(index + 1) {
            assert_ne!(
                left.result_identity(),
                right.result_identity(),
                "{name} / {other}"
            );
        }
    }
    assert_eq!(Request::fixed_sample().identity().result_identity(), base);
}

/// For an event-position request, the sequence authority of the row, the
/// watermark and the scope closure each alone give a distinct identity in
/// the builder. The watermark and scope are test-chosen, so FR-522-AC-19 is
/// not claimed until an evaluator supplies them.
#[trace("TC-1317")]
#[test]
fn an_event_position_identity_depends_on_authority_watermark_and_scope() {
    let identity = |authority: &str, watermark: u64, closed: bool| {
        let recipe = Recipe {
            row: binding_row("Due", "order-clock", event(authority)),
            agreeing: asserted(EVENT_PROFILE, "order-clock", event(authority)),
            ..event_position()
        };
        let authenticated = recipe
            .agreeing
            .authenticate(&recipe.selection())
            .expect("agrees");
        IdentityBuilder::new()
            .clause(&recipe.declaration)
            .expect("encodes")
            .authenticated(&authenticated)
            .expect("encodes")
            .member(
                IdentityMember::Progress,
                MemberEncoder::new().u64(watermark).flag(closed).finish(),
            )
            .result_identity()
    };
    let base = identity("order-events", 5, true);
    assert_ne!(identity("other-events", 5, true), base);
    assert_ne!(identity("order-events", 6, true), base);
    assert_ne!(identity("order-events", 5, false), base);
}

/// FR-522-AC-21: two requests that differ only in the relation, the evidence
/// ledger or the progress ledger have equal `input_identity` values, and
/// differ in `result_identity` where the member is part of it.
#[trace("FR-522-AC-21", "TC-1317")]
#[test]
fn the_input_identity_leaves_out_the_relation_and_the_ledgers() {
    let base = Request::fixed_sample().identity();
    let relation = base.clone().member(
        IdentityMember::Relation,
        MemberEncoder::new().u64(1).finish(),
    );
    let ledger = changed(|r| r.retained += 1);
    let progress = changed(|r| r.ledger_entry += 1);
    for other in [&relation, &ledger, &progress] {
        assert_eq!(other.input_identity(), base.input_identity());
        assert_ne!(other.result_identity(), base.result_identity());
    }
    let evidence = changed(|r| r.stamp += 1);
    assert_ne!(evidence.input_identity(), base.input_identity());
}

// FR-523: the counters.

fn clause_with_five_tick_reach() -> crate::temporal_trace::Fixture {
    fixture(
        EVENT_PROFILE,
        "c: Counter",
        "on origin",
        "eventually[0, 5] holds(c.level = 3)",
    )
}

/// The charge sequence FR-501-AC-12 gives `eventually[0,5] holds(p)` `on
/// origin` with three eligible positions, plus the retained members of the
/// other counters, charged through one call.
fn exercise(call: &mut CallMeter<'_>) -> Result<(), EvidenceStop> {
    let fixture = clause_with_five_tick_reach();
    call.admit_clause(fixture.clause.formula(), None, [0])
        .map_err(|stop| match stop {
            ClauseStop::Limit(reached) => EvidenceStop::Limit(reached),
            ClauseStop::Unreadable(fault) => panic!("a bounded formula: {fault}"),
        })?;
    call.assess(b"t1".to_vec());
    call.charge(&[(EvidenceCounter::Instances, 1)])
        .map_err(EvidenceStop::Limit)?;
    call.visit(Visit::Step)?; // the origin resolved
    call.visit(Visit::Step)?; // `eventually` at position 0
    for _ in 0..3 {
        call.visit(Visit::Lookup)?;
        call.visit(Visit::Scan)?;
    }
    call.charge(&[
        (EvidenceCounter::Support, 3),
        (EvidenceCounter::Lineage, 1),
        (EvidenceCounter::RequestBytes, 10),
        (EvidenceCounter::ResultBytes, 20),
        (EvidenceCounter::MemberBytes, 7),
    ])
    .map_err(EvidenceStop::Limit)?;
    Ok(())
}

fn work_limit(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        work_units,
        ..SCALAR_LIMITS_UNLIMITED
    }
}

fn run(
    limits: TemporalEvidenceLimits,
    meter: &mut Meter,
) -> (
    Result<(), EvidenceStop>,
    qsl_eval::temporal::evidence::EvidenceUsage,
) {
    let mut call = CallMeter::new(limits, meter);
    let result = exercise(&mut call);
    (result, call.usage())
}

fn measured() -> qsl_eval::temporal::evidence::EvidenceUsage {
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let (result, usage) = run(TemporalEvidenceLimits::default(), &mut meter);
    result.expect("the default ceilings admit the call");
    usage
}

/// The scripted call of `exercise` measures `formula_nodes` 2, `instances` 1,
/// `positions` 3, `valuations` 3, `captures` 0, `evaluation_steps` 8 and
/// `visited` 8, and reports all thirteen counters (AC-18). The script charges these
/// numbers itself, so FR-523-AC-3 (the usage of a real evaluation) is not
/// claimed.
#[trace("FR-523-AC-18", "TC-1318")]
#[test]
fn the_usage_of_an_origin_clause_over_three_positions_is_measured_per_counter() {
    let usage = measured();
    let expected = [
        (EvidenceCounter::FormulaNodes, 2),
        (EvidenceCounter::Instances, 1),
        (EvidenceCounter::Positions, 3),
        (EvidenceCounter::Valuations, 3),
        (EvidenceCounter::Captures, 0),
        (EvidenceCounter::Support, 3),
        (EvidenceCounter::HistorySpan, 5),
        (EvidenceCounter::EvaluationSteps, 8),
        (EvidenceCounter::Lineage, 1),
        (EvidenceCounter::Visited, 8),
        (EvidenceCounter::RequestBytes, 10),
        (EvidenceCounter::ResultBytes, 20),
        (EvidenceCounter::MemberBytes, 7),
    ];
    assert_eq!(usage.entries(), expected);
}

/// FR-523-AC-4, AC-5: each counter completes with its ceiling at exactly its
/// measured usage and returns `Incomplete` naming the counter and its
/// setting, with no truth, one below it. The counters whose measured usage
/// is zero (`captures`, which the script never charges) cannot go one below
/// here; `captures_are_charged_by_a_capture_and_an_evicted_record_spends_no_capture`
/// covers it. The script's charges are its own, so this proves the ceiling
/// machinery per counter and not the counts of a real evaluation.
#[trace("FR-523-AC-4", "FR-523-AC-5", "TC-1318")]
#[test]
fn each_counter_completes_at_its_usage_and_stops_one_below() {
    let usage = measured();
    for counter in EvidenceCounter::ALL {
        let used = usage.get(counter);
        if used == 0 {
            continue;
        }
        let at = TemporalEvidenceLimits::default().with_ceiling(counter, used);
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let (result, again) = run(at, &mut meter);
        assert_eq!(result, Ok(()), "{} at its usage", counter.name());
        assert_eq!(again, usage);

        let below = TemporalEvidenceLimits::default().with_ceiling(counter, used - 1);
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let (result, _) = run(below, &mut meter);
        let Err(EvidenceStop::Limit(reached)) = result else {
            panic!(
                "{} one below its usage must stop: {result:?}",
                counter.name()
            );
        };
        assert_eq!(reached.counter, counter);
        assert_eq!(reached.setting, counter.setting());
        assert_eq!(reached.limit, counter.limit_kind());
        assert_eq!(reached.value, used - 1);
        assert_eq!(
            reached.setting.name(),
            format!("temporal_evidence.{}", counter.name())
        );
    }
}

/// FR-523-AC-7, AC-9: a ceiling of zero is used as zero, so `positions` 0
/// stops at the first scan, and a refused charge leaves the counter at its
/// last successful value.
#[trace("FR-523-AC-7", "FR-523-AC-9", "TC-1318")]
#[test]
fn a_zero_ceiling_stops_at_the_first_charge_and_raises_nothing() {
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let limits = TemporalEvidenceLimits::default().with_ceiling(EvidenceCounter::Positions, 0);
    let mut call = CallMeter::new(limits, &mut meter);
    let Err(EvidenceStop::Limit(reached)) = exercise(&mut call) else {
        panic!("positions 0 must stop the first scan");
    };
    assert_eq!(reached.counter, EvidenceCounter::Positions);
    assert_eq!(reached.value, 0);
    let usage = call.usage();
    assert_eq!(usage.get(EvidenceCounter::Positions), 0);
    // The three lookups before the scan were performed; the refused scan
    // raised neither `evaluation_steps` nor `visited`.
    assert_eq!(usage.get(EvidenceCounter::Valuations), 1);
    assert_eq!(usage.get(EvidenceCounter::EvaluationSteps), 3);
    assert_eq!(usage.get(EvidenceCounter::Visited), 2 + 1);
}

/// FR-523-AC-8: a ceiling raised above its default is used as given; the
/// default of `positions` is 1,000,000 and 2,000,000 charges are admitted.
#[trace("FR-523-AC-8", "TC-1318")]
#[test]
fn a_ceiling_above_its_default_is_not_clamped() {
    let default = TemporalEvidenceLimits::default();
    assert_eq!(default.positions, 1_000_000);
    let raised = default.with_ceiling(EvidenceCounter::Positions, 2_000_000);
    assert_eq!(raised.ceiling(EvidenceCounter::Positions), 2_000_000);
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let mut call = CallMeter::new(raised, &mut meter);
    call.charge(&[(EvidenceCounter::Positions, 2_000_000)])
        .expect("2,000,000 positions are within the raised ceiling");
    assert_eq!(call.usage().get(EvidenceCounter::Positions), 2_000_000);
    let reached = call
        .charge(&[(EvidenceCounter::Positions, 1)])
        .expect_err("the raised ceiling is reached");
    assert_eq!(reached.value, 2_000_000);
}

/// The charge log of a scripted call stopped one below its usage equals the
/// log of the successful prefix of the full call; a retry under sufficient
/// ceilings gives the result of a call that never stopped. The script is not
/// an evaluation, so FR-523-AC-10, AC-11 and AC-19 are not claimed.
#[trace("TC-1318")]
#[test]
fn a_stopped_call_logs_the_prefix_and_a_retry_equals_an_unstopped_call() {
    let mut full = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let (result, usage) = run(TemporalEvidenceLimits::default(), &mut full);
    assert_eq!(result, Ok(()));
    let full_log = full.admitted_charges().to_vec();
    assert_eq!(full_log.len(), 8);
    assert!(full_log
        .iter()
        .all(|charge| charge.point == ChargePoint::TemporalVisit));

    let below = TemporalEvidenceLimits::default().with_ceiling(
        EvidenceCounter::EvaluationSteps,
        usage.get(EvidenceCounter::EvaluationSteps) - 1,
    );
    let mut stopped = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let (result, _) = run(below, &mut stopped);
    assert!(matches!(result, Err(EvidenceStop::Limit(_))));
    assert_eq!(stopped.admitted_charges(), &full_log[..7]);

    let mut retry = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let (result, again) = run(TemporalEvidenceLimits::default(), &mut retry);
    assert_eq!(result, Ok(()));
    assert_eq!(again, usage);
    assert_eq!(retry.admitted_charges(), full_log.as_slice());
}

/// FR-523-AC-12, AC-13: a clause whose `history_span` or `formula_nodes`
/// exceeds its ceiling stops the call before any instance is assessed: the
/// stop names no instance and the meter is untouched.
#[trace("FR-523-AC-12", "FR-523-AC-13", "TC-1318")]
#[test]
fn a_clause_level_counter_stops_before_any_instance_is_assessed() {
    let fixture = clause_with_five_tick_reach();
    for (counter, ceiling) in [
        (EvidenceCounter::HistorySpan, 4),
        (EvidenceCounter::FormulaNodes, 1),
    ] {
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let limits = TemporalEvidenceLimits::default().with_ceiling(counter, ceiling);
        let mut call = CallMeter::new(limits, &mut meter);
        let Err(ClauseStop::Limit(reached)) =
            call.admit_clause(fixture.clause.formula(), None, [0])
        else {
            panic!("{} must stop the clause", counter.name());
        };
        assert_eq!(reached.counter, counter);
        assert_eq!(reached.value, ceiling);
        assert_eq!(reached.instance, None);
        drop(call);
        assert!(meter.admitted_charges().is_empty());
    }
}

fn shape(first: Option<usize>, second: Option<usize>, reach: NodeReach) -> NodeShape {
    NodeShape {
        operands: [first, second],
        reach,
    }
}

/// FR-523-AC-6: `formula_nodes` counts the distinct nodes reachable from the
/// root, and a node two parents share counts once. The checker emits one
/// arena node per source occurrence, so the shared node is built from the
/// node shapes a checked package's graph holds (FR-515, FR-337), where equal
/// terms are one node.
#[trace("FR-523-AC-6", "TC-1318")]
#[test]
fn formula_nodes_counts_distinct_reachable_nodes() {
    let atom = shape(None, None, NodeReach::None);
    // `p and p` where both operands are the one atom node.
    let shared = [shape(Some(1), Some(1), NodeReach::None), atom];
    assert_eq!(ClauseWork::of_shapes(&shared, 0).unwrap().formula_nodes, 2);
    // `p and q` with two atom nodes.
    let distinct = [shape(Some(1), Some(2), NodeReach::None), atom, atom];
    assert_eq!(
        ClauseWork::of_shapes(&distinct, 0).unwrap().formula_nodes,
        3
    );
    // Nodes the root does not reach are not counted, and a node reached by
    // two paths of different depth counts once with the greater reach.
    let deep = [
        shape(Some(1), Some(2), NodeReach::Future(2)),
        shape(Some(2), None, NodeReach::Past(7)),
        atom,
        atom,
    ];
    let work = ClauseWork::of_shapes(&deep, 0).unwrap();
    assert_eq!(work.formula_nodes, 3);
    assert_eq!(work.history_span, 9);
    assert_eq!(work.future_reach, 2);
    // A cycle and an operand outside the formula are malformed.
    let cycle = [
        shape(Some(1), None, NodeReach::None),
        shape(Some(0), None, NodeReach::None),
    ];
    assert_eq!(ClauseWork::of_shapes(&cycle, 0), Err(ReachFault::Malformed));
    assert_eq!(
        ClauseWork::of_shapes(&[shape(Some(5), None, NodeReach::None)], 0),
        Err(ReachFault::Malformed)
    );
    // A reach sum past u64 is an overflow, not a wrapped sum.
    let overflow = [
        shape(Some(1), None, NodeReach::Future(u64::MAX)),
        shape(Some(2), None, NodeReach::Future(1)),
        atom,
    ];
    assert_eq!(
        ClauseWork::of_shapes(&overflow, 0),
        Err(ReachFault::Overflow)
    );
    // The checker's own arena: one node per source occurrence.
    let built = fixture(
        EVENT_PROFILE,
        "c: Counter",
        "on origin",
        "eventually[0, 5] (holds(c.level = 1) and holds(c.level = 2))",
    );
    let work = ClauseWork::of(built.clause.formula()).expect("bounded");
    assert_eq!(work.formula_nodes, 4);
    assert_eq!(work.history_span, 5);
    assert_eq!(work.future_reach, 5);
}

/// Two scripted calls share one meter whose work-unit ceiling is
/// the first call's usage plus one: the first completes and the second
/// stops at the `temporal.visit` charge point naming its instance. With
/// `evaluation_steps` at 1000 and the meter one below the visits, the stop
/// is at `temporal.visit`; with the meter unbounded and `evaluation_steps`
/// one below it names `evaluation_steps`.
#[trace("TC-1318")]
#[test]
fn calls_sharing_a_meter_accumulate_and_the_counter_is_charged_first() {
    let steps = measured().get(EvidenceCounter::EvaluationSteps);
    let mut shared = Meter::new(work_limit(steps + 1));
    let (first, first_usage) = run(TemporalEvidenceLimits::default(), &mut shared);
    assert_eq!(first, Ok(()));
    let (second, _) = run(TemporalEvidenceLimits::default(), &mut shared);
    let Err(EvidenceStop::Meter {
        incomplete,
        instance,
    }) = second
    else {
        panic!("the shared meter must stop the second call: {second:?}");
    };
    assert_eq!(incomplete.charge_point, ChargePoint::TemporalVisit);
    assert_eq!(instance.as_deref(), Some(b"t1".as_slice()));
    // The first call kept its measured usage, and replayed on a fresh meter
    // it gives the same usage (AC-19).
    let mut fresh = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let (replayed, replayed_usage) = run(TemporalEvidenceLimits::default(), &mut fresh);
    assert_eq!(replayed, Ok(()));
    assert_eq!(replayed_usage, first_usage);

    let thousand =
        TemporalEvidenceLimits::default().with_ceiling(EvidenceCounter::EvaluationSteps, 1000);
    let mut meter = Meter::new(work_limit(steps - 1));
    let (result, _) = run(thousand, &mut meter);
    assert!(matches!(
        result,
        Err(EvidenceStop::Meter { ref incomplete, .. }) if incomplete.charge_point == ChargePoint::TemporalVisit
    ));
    let below =
        TemporalEvidenceLimits::default().with_ceiling(EvidenceCounter::EvaluationSteps, steps - 1);
    let mut unbounded = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let (result, _) = run(below, &mut unbounded);
    let Err(EvidenceStop::Limit(reached)) = result else {
        panic!("the counter must stop first: {result:?}");
    };
    assert_eq!(reached.counter, EvidenceCounter::EvaluationSteps);
}

/// FR-523-AC-9: a charge the meter denies leaves every counter at its last
/// successful value, as a charge a counter refuses does. The counters are
/// raised only once the meter has admitted the unit.
#[trace("FR-523-AC-9", "TC-1318")]
#[test]
fn a_meter_denial_leaves_every_counter_at_its_last_value() {
    let mut meter = Meter::new(work_limit(1));
    let mut call = CallMeter::new(TemporalEvidenceLimits::default(), &mut meter);
    call.assess(b"t1".to_vec());
    call.visit(Visit::Scan).expect("the meter admits one visit");
    let admitted = call.usage();
    assert_eq!(admitted.get(EvidenceCounter::Positions), 1);
    assert_eq!(admitted.get(EvidenceCounter::EvaluationSteps), 1);
    assert_eq!(admitted.get(EvidenceCounter::Visited), 1);
    let Err(EvidenceStop::Meter {
        incomplete,
        instance,
    }) = call.visit(Visit::Scan)
    else {
        panic!("the meter must deny the second visit");
    };
    assert_eq!(incomplete.charge_point, ChargePoint::TemporalVisit);
    assert_eq!(instance.as_deref(), Some(b"t1".as_slice()));
    assert_eq!(call.usage(), admitted);
    assert_eq!(call.meter().admitted_charges().len(), 1);
}

/// FR-523-AC-4, AC-5: `captures` is spent by a capture established and
/// retained, and by nothing else; its ceiling admits exactly that many
/// captures and the next stops naming `captures`. An evicted record consulted
/// spends `evaluation_steps` and `visited` and never `captures`.
#[trace("FR-523-AC-4", "FR-523-AC-5", "TC-1318")]
#[test]
fn captures_are_charged_by_a_capture_and_an_evicted_record_spends_no_capture() {
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let limits = TemporalEvidenceLimits::default().with_ceiling(EvidenceCounter::Captures, 2);
    let mut call = CallMeter::new(limits, &mut meter);
    call.visit(Visit::Capture).expect("the first capture");
    call.visit(Visit::Capture).expect("the second capture");
    let usage = call.usage();
    assert_eq!(usage.get(EvidenceCounter::Captures), 2);
    assert_eq!(usage.get(EvidenceCounter::EvaluationSteps), 2);
    assert_eq!(usage.get(EvidenceCounter::Visited), 2);
    assert_eq!(usage.get(EvidenceCounter::Positions), 0);
    assert_eq!(usage.get(EvidenceCounter::Valuations), 0);
    let Err(EvidenceStop::Limit(reached)) = call.visit(Visit::Capture) else {
        panic!("a third capture is over the ceiling of two");
    };
    assert_eq!(reached.counter, EvidenceCounter::Captures);
    assert_eq!(reached.value, 2);
    assert_eq!(reached.setting.name(), "temporal_evidence.captures");
    assert_eq!(call.usage(), usage);

    call.visit(Visit::Evicted).expect("an evicted record");
    let after = call.usage();
    assert_eq!(after.get(EvidenceCounter::Captures), 2);
    assert_eq!(after.get(EvidenceCounter::EvaluationSteps), 3);
    assert_eq!(after.get(EvidenceCounter::Visited), 3);

    for counter in [EvidenceCounter::EvaluationSteps, EvidenceCounter::Visited] {
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let limits = TemporalEvidenceLimits::default().with_ceiling(counter, 0);
        let mut call = CallMeter::new(limits, &mut meter);
        let Err(EvidenceStop::Limit(reached)) = call.visit(Visit::Evicted) else {
            panic!(
                "an evicted record is over a ceiling of zero on {}",
                counter.name()
            );
        };
        assert_eq!(reached.counter, counter);
    }
}

/// FR-523-AC-16: a stamp whose `t0 + Hf + m` exceeds `u64::MAX` stops naming
/// `history_span` before any position is visited, a stamp whose sum is
/// `u64::MAX` is admitted, and the margin `2u` is part of the sum.
#[trace("FR-523-AC-16", "TC-1318")]
#[test]
fn a_tick_sum_that_overflows_u64_stops_at_history_span() {
    let fixture = clause_with_five_tick_reach();
    let formula = fixture.clause.formula();
    let admit = |uncertainty: Option<u64>, anchors: Vec<u64>| {
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut call = CallMeter::new(TemporalEvidenceLimits::default(), &mut meter);
        let result = call.admit_clause(formula, uncertainty, anchors);
        drop(call);
        assert!(meter.admitted_charges().is_empty());
        result
    };
    assert!(admit(None, vec![u64::MAX - 5]).is_ok());
    let Err(ClauseStop::Limit(reached)) = admit(None, vec![0, u64::MAX - 4]) else {
        panic!("the sum exceeds u64");
    };
    assert_eq!(reached.counter, EvidenceCounter::HistorySpan);
    assert_eq!(
        reached.limit,
        qsl_foundation::diagnostic::LimitKind::TickCount
    );
    // Under a counter cover of uncertainty 2 the margin is 4.
    assert!(admit(Some(2), vec![u64::MAX - 9]).is_ok());
    assert!(matches!(
        admit(Some(2), vec![u64::MAX - 8]),
        Err(ClauseStop::Limit(_))
    ));
    // A margin that does not fit is the same stop.
    assert!(matches!(
        admit(Some(u64::MAX), vec![0]),
        Err(ClauseStop::Limit(_))
    ));
}

struct AlwaysTrue;

impl TemporalLetters for AlwaysTrue {
    fn atom(
        &mut self,
        _atom: FormulaNodeId,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        Ok(AtomReading::True)
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        Ok(AtomReading::True)
    }
}

/// FR-523-AC-17: a formula nested 100,000 deep is measured and evaluated
/// with `formula_nodes` and `evaluation_steps` raised to fit it, on a thread
/// with a 512 KiB stack.
#[trace("FR-523-AC-17", "TC-1318")]
#[test]
fn a_formula_nested_one_hundred_thousand_deep_evaluates_on_a_small_stack() {
    const DEPTH: usize = 100_000;
    let body = format!("{}holds(c.level = 3)", "not ".repeat(DEPTH));
    let built = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            let limits = qsl_cst::Limits::default()
                .with_tokens(DEPTH * 4)
                .with_nodes(DEPTH * 4)
                .with_work_units(usize::MAX);
            fixture_within(EVENT_PROFILE, "c: Counter", "on origin", &body, limits)
        })
        .expect("a build thread")
        .join()
        .expect("the deep clause checks");
    let outcome = std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(move || {
            let limits = TemporalEvidenceLimits::default()
                .with_ceiling(EvidenceCounter::FormulaNodes, DEPTH as u64 + 1)
                .with_ceiling(EvidenceCounter::EvaluationSteps, DEPTH as u64 + 1)
                .with_ceiling(EvidenceCounter::Visited, DEPTH as u64 + 1);
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let mut call = CallMeter::new(limits, &mut meter);
            let work = call
                .admit_clause(built.clause.formula(), None, [0])
                .expect("the raised ceilings admit the clause");
            assert_eq!(work.formula_nodes, DEPTH as u64 + 1);
            assert_eq!(work.history_span, 0);
            let usage = call.usage();
            assert_eq!(usage.get(EvidenceCounter::FormulaNodes), DEPTH as u64 + 1);
            let outcome = evaluate(
                &built.clause,
                TemporalTrace::Finite { len: 1 },
                &mut AlwaysTrue,
                &mut meter,
            )
            .expect("the trace is readable");
            drop(built);
            outcome
        })
        .expect("an evaluation thread")
        .join()
        .expect("evaluation does not overflow a 512 KiB stack");
    // An even number of negations over a true atom is true.
    assert_eq!(outcome, TemporalOutcome::Holds);
}

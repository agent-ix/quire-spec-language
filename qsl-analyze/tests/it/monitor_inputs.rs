// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-283 (TC-765, TC-766): the `Samples` and `Typed` trace shapes, the
//! `sources` and `temporal_selections` inputs, and `max_window_values`,
//! over one compiled unit holding the `Counter` model, a `Reading` record
//! trace subject and their temporal clauses.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use ix_trace_rs::trace;
use qsl_analyze::monitor::{
    monitor, MonitorClause, MonitorLimits, MonitorOutcome, MonitorRecord, MonitorRefusal,
    MonitorRequest, MonitorTrace, MonitorVerdict, TRACE_FORMAT,
};
use qsl_foundation::diagnostic::StageFailure;
use qsl_foundation::{Source, SourceIdentity};
use qsl_package::{ClockConfiguration, TemporalClockSelection};
use qsl_replay::spine::{
    self, run_package_temporal, CallOutcome, CallRefusal, ClauseDisposition, DependencyInput,
    OverObject, PackageTemporalRequest, ParseRequest, ProgressAssertion, ProgressClosure,
    ProgressCompleteness, SpineLimits, StateClauseExecutionPoints, TemporalIndeterminateReason,
    TemporalVerdict, TimestampedMissingValuation, TimestampedObservation, TimestampedWindow,
    TraceBinding, TraceInput,
};
use qsl_replay::ItemLabel;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::observation::{AdmissionFailure, DocumentRef, ObservationLimits};
use quire_exact::{Cancel, CancelCause, FieldValue, Value, ValueType};
use quire_semantic_value::catalog::Category;
use serde_json::json;

use crate::counter::{document, Documents, UNLIMITED};

const PATH: &str = "monitor-inputs.native";

/// The header both units share: the `Counter` model and, as the only
/// temporal profile, `profile`.
fn header(profile: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t = \"{profile}\";\n\
         model M = \"test/counter\";\n\
         record Reading {{ level: Int[0, 9]; }}\n\
         type Zero = Int[0, 0];\n\
         function ints using v(xs: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 3] pure {{ xs }}\n"
    )
}

/// The event-position unit: a model clause, FR-327-AC-6's refusing clause
/// and the `Reading` trace-subject clauses.
fn event_unit() -> String {
    header("quire.temporal.event-position.false-extension/v1")
        + "temporal Small using t over (c: M::Counter) clock \"model-steps\" on origin { always[0,2] holds(c.value < 2) }\n\
           temporal Refuse using t over (c: M::Counter) clock \"model-steps\" on origin { always[0,1] holds(count<Zero>(m in ints(sequence[c.value]): m > 0) = 0) }\n\
           temporal Gate using t over (r: Reading) clock \"model-steps\" on origin { always[0,2] (count over past[0, 9] min_count 2 >= 1) }\n\
           temporal Dense using t over (r: Reading) clock \"model-steps\" on origin { always[1,1] (count over past[0, 9] min_count 1 >= 1) }\n\
           temporal Wide using t over (r: Reading) clock \"model-steps\" on origin { always[0,0] (count where holds(r.level > 9) over past[0, 0] min_count 0 >= 1) }\n\
           temporal Long using t over (r: Reading) clock \"model-steps\" on origin { always[0,19999] (count where holds(r.level >= 0) over past[0, 0] min_count 0 >= 1) }\n"
}

/// The finite-window unit: FR-501-AC-4's clause.
fn window_unit() -> String {
    header("quire.temporal.timestamped-event.finite-window/v1")
        + "temporal Window using t over (c: M::Counter) clock \"ms\" on origin { eventually[0,5] holds(c.value > 0) }\n\
           temporal Window2 using t over (c: M::Counter) clock \"ms\" on origin { eventually[0,5] holds(c.value > 1) }\n"
}

/// FR-501-AC-7's `until` clause under the finite-window profile.
fn until_unit() -> String {
    header("quire.temporal.timestamped-event.finite-window/v1")
        + "temporal Until using t over (c: M::Counter) clock \"ms\" on origin { holds(c.value > 0) until[0,1] holds(c.value > 1) }\n"
}

fn identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "monitor-inputs")
}

/// The unit compiled: its package, its sources and the clock selections of
/// its clock-bearing temporal declarations.
struct Compiled {
    text: String,
    unit: spine::CheckedUnit,
    sources: Vec<Source>,
    selections: Vec<TemporalClockSelection>,
}

fn selections(parsed: &qsl_cst::ParsedSource) -> Vec<TemporalClockSelection> {
    use qsl_forms::DeclarationForm;
    let unit = qsl_forms::build_unit(parsed).expect("the unit forms");
    let profiles = &unit.selections().profiles;
    unit.forms()
        .iter()
        .filter_map(|form| {
            let DeclarationForm::Temporal(clause) = form.form() else {
                return None;
            };
            let selected = profiles
                .iter()
                .find(|profile| profile.alias == clause.profile.alias)
                .expect("the temporal profile is selected");
            let clock = match selected.identity.as_str() {
                "quire.temporal.event-position.false-extension/v1" => {
                    ClockConfiguration::EventPosition {
                        sequence_authority: "counter-event-sequence".into(),
                    }
                }
                "quire.temporal.timestamped-event.finite-window/v1" => {
                    ClockConfiguration::TimestampedEvent {
                        timestamp_unit: "ms".into(),
                    }
                }
                _ => return None,
            };
            Some(TemporalClockSelection::new(
                parsed.source().reference().clone(),
                form.span(),
                clock,
            ))
        })
        .collect()
}

fn compile(documents: &Documents, text: String) -> Compiled {
    compile_with(documents, text, &DependencyInput::default())
}

fn compile_with(documents: &Documents, text: String, dependencies: &DependencyInput) -> Compiled {
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let identity = identity();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: PATH,
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("the unit parses")
    .into_value();
    let models = spine::select(&parsed, &documents.packages, limits.model, &cancel)
        .expect("the model selects")
        .into_value();
    let cst = qsl_cst::parse(identity, PATH, text.as_bytes(), qsl_cst::Limits::default())
        .expect("the unit parses");
    let selections = selections(&cst);
    let checked = spine::check_with_temporal_selections_and_execution_points(
        &parsed,
        "fixture".to_owned(),
        &StateClauseExecutionPoints::new(),
        &selections,
        &models,
        dependencies,
        limits,
        &cancel,
    )
    .expect("the unit checks")
    .into_value();
    let mut sources = vec![checked.source().clone()];
    sources.extend(checked.libraries().iter().cloned());
    Compiled {
        text,
        unit: checked,
        sources,
        selections,
    }
}

struct World {
    event: Compiled,
    window: Compiled,
    until: Compiled,
    documents: Documents,
}

fn world() -> World {
    let documents = Documents::new(document(false));
    World {
        event: compile(&documents, event_unit()),
        window: compile(&documents, window_unit()),
        until: compile(&documents, until_unit()),
        documents,
    }
}

fn over(key: &str) -> OverObject {
    OverObject {
        population: "ix://test/counter/counters".to_owned(),
        key: key.to_owned(),
    }
}

fn model_clause(name: &str) -> MonitorClause {
    MonitorClause {
        name: name.to_owned(),
        over: Some(over("c1")),
    }
}

fn record_clause(name: &str) -> MonitorClause {
    MonitorClause {
        name: name.to_owned(),
        over: None,
    }
}

fn limits(max_window_values: u64) -> MonitorLimits {
    MonitorLimits {
        observation: ObservationLimits::default(),
        normalization: ModelNormalizationLimits::default(),
        accounting: UNLIMITED,
        max_window_values,
    }
}

impl World {
    fn run(
        &self,
        unit: &Compiled,
        clauses: Vec<MonitorClause>,
        trace: MonitorTrace<'_>,
        sources: &[Source],
        selections: &[TemporalClockSelection],
        limits: &MonitorLimits,
    ) -> Result<MonitorOutcome, StageFailure<MonitorRefusal>> {
        let request = MonitorRequest {
            clauses,
            trace,
            snapshots: &self.documents.snapshots,
            packages: &self.documents.packages,
            sources,
            temporal_selections: selections,
        };
        monitor(unit.unit.package(), &request, limits, &Cancel::new())
            .map(|staged| staged.into_value())
    }

    fn samples(
        &self,
        clauses: Vec<MonitorClause>,
        levels: &[i64],
        max: u64,
    ) -> Result<MonitorOutcome, StageFailure<MonitorRefusal>> {
        let subject = self.reading();
        let ValueType::Composite(key) = &subject else {
            panic!("Reading is a record");
        };
        let samples: Vec<Value> = levels
            .iter()
            .map(|level| {
                quire_exact::from_admitted_slots(
                    key.clone(),
                    vec![FieldValue::Present(Value::Integer((*level).into()))].into_boxed_slice(),
                )
            })
            .collect();
        self.run(
            &self.event,
            clauses,
            MonitorTrace::Samples {
                subject: subject.clone(),
                samples: &samples,
            },
            &self.event.sources,
            &[],
            &limits(max),
        )
    }

    fn reading(&self) -> ValueType {
        let types = self.event.unit.package().graph().scope().types();
        ValueType::Composite(
            types
                .composites()
                .find(|declaration| declaration.name() == "Reading")
                .expect("Reading is declared")
                .key(),
        )
    }

    fn refs(&mut self, tag: &str, values: &[i64]) -> Vec<DocumentRef> {
        values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                self.documents
                    .start(&format!("{tag}-{index}"), &[("c1", *value)])
            })
            .collect()
    }

    fn document_trace(&mut self, tag: &str, values: &[i64]) -> Vec<u8> {
        let prefix: Vec<_> = self
            .refs(tag, values)
            .iter()
            .map(|reference| {
                json!({"identity": {"authority": reference.authority,
                    "identity": reference.identity}})
            })
            .collect();
        json!({"format": TRACE_FORMAT, "prefix": prefix})
            .to_string()
            .into_bytes()
    }
}

fn refused_clause(
    result: Result<MonitorOutcome, StageFailure<MonitorRefusal>>,
) -> (
    String,
    Box<qsl_semantics::model::observation::AdmissionRecord>,
) {
    let Err(StageFailure::Refused(MonitorRefusal::ClauseRefused { clause, failure })) = result
    else {
        panic!("a clause refusal, got {result:?}");
    };
    let AdmissionFailure::Refused(record) = *failure else {
        panic!("a refusal record");
    };
    (clause, record)
}

fn only(outcome: &MonitorOutcome) -> &MonitorRecord {
    let [record] = outcome.records.as_slice() else {
        panic!("one record, got {}", outcome.records.len());
    };
    record
}

/// FR-283-AC-11: over a `Samples` trace, a window aggregate short of data is
/// `InsufficientData` naming its atom by source text, and a violated
/// aggregate carries a `decisive` and no `counterexample`.
#[trace("TC-765", "FR-283-AC-11")]
#[test]
fn samples_report_insufficient_data_and_a_decisive_aggregate() {
    let world = world();
    let outcome = world
        .samples(vec![record_clause("Gate")], &[0, 1, 2], 1 << 20)
        .expect("the request completes");
    let record = only(&outcome);
    assert_eq!(record.verdict, MonitorVerdict::InsufficientData);
    assert_eq!(record.category, Category::Undefined);
    assert_eq!(record.report().disposition.exit_code(), 10);
    let ClauseDisposition::Temporal(TemporalVerdict::Undefined {
        at,
        cause:
            spine::TemporalUndefinedCause::InsufficientData {
                count, min_count, ..
            },
    }) = &record.report().disposition
    else {
        panic!("insufficient data, got {:?}", record.report().disposition);
    };
    assert_eq!((*at, *count, *min_count), (0, 1, 2));
    assert_eq!(
        record.atom(),
        Some("count over past[0, 9] min_count 2 >= 1"),
        "named by the source text of the aggregate"
    );
    assert_eq!(
        record.item(0).expect("an item").result(),
        ItemLabel::Undefined
    );

    let outcome = world
        .samples(vec![record_clause("Wide")], &[0, 1, 2], 1 << 20)
        .expect("the request completes");
    let record = only(&outcome);
    assert_eq!(record.verdict, MonitorVerdict::Violated);
    assert!(record.counterexample().is_none());
    assert!(record.report().witness.is_none());
    let decisive = record.report().decisive.as_ref().expect("a decisive");
    assert_eq!(decisive.position, Some(0));
    assert!(decisive.aggregate.is_some());
}

/// FR-283-AC-11: an `over` over `Samples`, and a model-subject clause over
/// `Samples`, refuse the whole request `invalid_runtime_input`/`invalid-value`.
#[trace("TC-765", "FR-283-AC-11")]
#[test]
fn samples_refuse_an_over_and_a_model_subject_clause() {
    let world = world();
    let with_over = MonitorClause {
        name: "Gate".to_owned(),
        over: Some(over("c1")),
    };
    let (clause, record) = refused_clause(world.samples(vec![with_over], &[0, 1, 2], 1 << 20));
    assert_eq!(clause, "Gate");
    assert_eq!(record.catalog.code(), "invalid_runtime_input");
    assert_eq!(record.catalog.cause(), "invalid-value");

    let (clause, record) = refused_clause(world.samples(
        vec![record_clause("Gate"), record_clause("Small")],
        &[0, 1, 2],
        1 << 20,
    ));
    assert_eq!(clause, "Small");
    assert_eq!(record.catalog.cause(), "invalid-value");
}

/// FR-283-AC-12: `max_window_values` is the caller's window limit.
#[trace("TC-766", "FR-283-AC-12")]
#[test]
fn max_window_values_is_the_callers_window_limit() {
    let world = world();
    let limited = world
        .samples(vec![record_clause("Dense")], &[0, 1, 2], 1)
        .expect("the request completes");
    let record = only(&limited);
    assert_eq!(record.verdict, MonitorVerdict::WindowLimitReached);
    assert_eq!(record.category, Category::Incomplete);
    assert_eq!(record.report().disposition.exit_code(), 22);
    let ClauseDisposition::Temporal(TemporalVerdict::WindowLimitReached {
        limit,
        value,
        setting,
    }) = &record.report().disposition
    else {
        panic!("a window limit, got {:?}", record.report().disposition);
    };
    assert_eq!(
        (*limit, *value, *setting),
        ("max_window_values", 1, "monitor.max_window_values")
    );

    let wide = world
        .samples(vec![record_clause("Dense")], &[0, 1, 2], 2)
        .expect("the request completes");
    assert_eq!(only(&wide).verdict, MonitorVerdict::Tested);
}

/// FR-283-AC-12: a refused `holds` condition renders its locus from
/// `sources`; no matching source, and a source too short for the span,
/// refuse the request with `SourceNotSupplied`.
#[trace("TC-766", "FR-283-AC-12")]
#[test]
fn a_refused_condition_needs_the_source_it_is_cited_at() {
    let mut world = world();
    let bytes = world.document_trace("r", &[0, 1]);
    let run = |world: &World, sources: &[Source]| {
        world.run(
            &world.event,
            vec![model_clause("Refuse")],
            MonitorTrace::Document(&bytes),
            sources,
            &[],
            &limits(1 << 20),
        )
    };

    let outcome = run(&world, &world.event.sources).expect("the request completes");
    let record = only(&outcome);
    assert_eq!(record.verdict, MonitorVerdict::ConditionRefused);
    assert_eq!(record.category, Category::Refusal);
    assert_eq!(record.report().disposition.exit_code(), 20);
    assert_eq!(record.report().refused_at, Some(1));
    let ClauseDisposition::Evaluate(CallOutcome::Refused(CallRefusal::Record {
        code,
        fields,
        locus,
        ..
    })) = &record.report().disposition
    else {
        panic!("a refused condition, got {:?}", record.report().disposition);
    };
    assert_eq!(
        (code.code(), code.cause()),
        ("integer_out_of_domain", "outside-domain")
    );
    assert_eq!(
        fields.get("expected").map(String::as_str),
        Some("Int[0, 0]")
    );
    let locus = locus.as_ref().expect("a locus");
    assert_eq!(locus.file, PATH);
    let condition = "count<Zero>(m in ints(sequence[c.value]): m > 0)";
    let start = world
        .event
        .text
        .find(condition)
        .expect("the condition is in the unit");
    assert_eq!(locus.span.start.byte, start);
    assert_eq!(locus.span.end.byte, start + condition.len());

    let Err(StageFailure::Refused(MonitorRefusal::SourceNotSupplied { clause, source })) =
        run(&world, &[])
    else {
        panic!("a source is not supplied");
    };
    assert_eq!(clause, "Refuse");
    assert_eq!(source, identity());

    let short = Source::read_typed(
        identity(),
        PATH,
        &world.event.text.as_bytes()[..40],
        1 << 20,
    )
    .unwrap_or_else(|_| panic!("a short source admits"));
    let Err(StageFailure::Refused(MonitorRefusal::SourceNotSupplied { clause, source })) =
        run(&world, &[short])
    else {
        panic!("a short source does not hold the span");
    };
    assert_eq!((clause.as_str(), source), ("Refuse", identity()));
}

/// FR-283-AC-8: a trace document is bounded by `observation.objects` and
/// `observation.values` as it is by `observation.input_bytes` (FND-005), and
/// its `identity` object is closed (FND-011).
#[trace("TC-766", "FR-283-AC-8")]
#[test]
fn a_trace_document_is_bounded_and_its_identity_object_is_closed() {
    let mut world = world();
    let bytes = world.document_trace("b", &[0, 1]);
    let run = |world: &World, bytes: &[u8], limits: &MonitorLimits| {
        world.run(
            &world.event,
            vec![model_clause("Small")],
            MonitorTrace::Document(bytes),
            &[],
            &[],
            limits,
        )
    };
    let document_defect = |result| {
        let Err(StageFailure::Refused(MonitorRefusal::TraceDocument { record, pointer })) = result
        else {
            panic!("a trace document defect, got {result:?}");
        };
        (record, pointer)
    };

    // The document is one root object, a `prefix` array, two reference
    // objects and their two identity objects: five objects and eleven
    // value forms (root, format, prefix, and per element: the element, its
    // identity object and two labels).
    let mut bounded = limits(1 << 20);
    bounded.observation.objects_per_document = 4;
    let (record, pointer) = document_defect(run(&world, &bytes, &bounded));
    assert_eq!(record.catalog.code(), "stage_limit_exceeded");
    assert_eq!(record.catalog.cause(), "node-count-exceeded");
    assert_eq!(record.fields["setting"], "observation.objects");
    assert_eq!(record.fields["bound"], "4");
    assert_eq!(record.fields["actual"], "5");
    assert_eq!(pointer, "");

    let mut bounded = limits(1 << 20);
    bounded.observation.values_per_document = 10;
    let (record, _) = document_defect(run(&world, &bytes, &bounded));
    assert_eq!(record.catalog.cause(), "node-count-exceeded");
    assert_eq!(record.fields["setting"], "observation.values");
    assert_eq!(record.fields["bound"], "10");
    assert_eq!(record.fields["actual"], "11");

    let mut exact = limits(1 << 20);
    exact.observation.objects_per_document = 5;
    exact.observation.values_per_document = 11;
    assert!(run(&world, &bytes, &exact).is_ok());

    let closed = json!({"format": TRACE_FORMAT, "prefix": [
        {"identity": {"authority": "test", "identity": "b-0", "extra": 1}}]})
    .to_string();
    let (record, pointer) = document_defect(run(&world, closed.as_bytes(), &limits(1 << 20)));
    assert_eq!(record.catalog.code(), "invalid_runtime_input");
    assert_eq!(record.catalog.cause(), "unknown-member");
    assert_eq!(record.fields["field"], "extra");
    assert_eq!(pointer, "/prefix/0/identity");
}

/// FR-276-AC-1: `monitor` over a handle already cancelled with `Requested`
/// returns `StageFailure::Cancelled(Requested)` and no output, on every trace
/// shape; and the evaluation meter denies a charge once the handle is
/// cancelled (TC-757).
#[trace("TC-757", "FR-276-AC-1")]
#[test]
fn a_handle_cancelled_before_the_call_returns_cancelled() {
    let mut world = world();
    let refs = world.refs("k", &[0, 1, 0]);
    let trace = TraceInput::Finite(refs);
    let name = "Small";
    let object = over("c1");
    let run = |cancel: &Cancel| {
        run_package_temporal(&PackageTemporalRequest {
            package: world.event.unit.package(),
            packages: &world.documents.packages,
            model_limits: ModelNormalizationLimits::default(),
            snapshots: &world.documents.snapshots,
            observation_limits: ObservationLimits::default(),
            accounting: UNLIMITED,
            name,
            trace: &trace,
            over: &object,
            sources: &world.event.sources,
            authentication: None,
            cancel,
        })
    };
    let live = run(&Cancel::new());
    assert!(matches!(
        live.disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Holds)
    ));
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let stopped = run(&cancel);
    let ClauseDisposition::Temporal(TemporalVerdict::Incomplete(incomplete)) = &stopped.disposition
    else {
        panic!("a denied charge, got {:?}", stopped.disposition);
    };
    assert_eq!(incomplete.limit_kind.as_str(), "work_units");

    let bytes = world.document_trace("k2", &[0, 1, 0]);
    let request = MonitorRequest {
        clauses: vec![model_clause("Small")],
        trace: MonitorTrace::Document(&bytes),
        snapshots: &world.documents.snapshots,
        packages: &world.documents.packages,
        sources: &world.event.sources,
        temporal_selections: &[],
    };
    assert!(matches!(
        monitor(
            world.event.unit.package(),
            &request,
            &limits(1 << 20),
            &cancel
        ),
        Err(StageFailure::Cancelled(CancelCause::Requested))
    ));

    let subject = world.reading();
    let ValueType::Composite(key) = &subject else {
        panic!("Reading is a record");
    };
    let samples = vec![reading_sample(key, 1)];
    let request = MonitorRequest {
        clauses: vec![record_clause("Gate")],
        trace: MonitorTrace::Samples {
            subject: subject.clone(),
            samples: &samples,
        },
        snapshots: &world.documents.snapshots,
        packages: &world.documents.packages,
        sources: &world.event.sources,
        temporal_selections: &[],
    };
    assert!(matches!(
        monitor(
            world.event.unit.package(),
            &request,
            &limits(1 << 20),
            &cancel
        ),
        Err(StageFailure::Cancelled(CancelCause::Requested))
    ));
}

fn reading_sample(key: &quire_exact::NodeKey, level: i64) -> Value {
    quire_exact::from_admitted_slots(
        key.clone(),
        vec![FieldValue::Present(Value::Integer(level.into()))].into_boxed_slice(),
    )
}

/// TC-757: a handle cancelled by its own charge observer DURING a `Samples`
/// evaluation stops it at a meter charge: `monitor` returns
/// `Cancelled(Requested)` at exactly the charge that cancelled it, so the poll
/// is inside the evaluation and not only before or after it.
#[trace("TC-757")]
#[test]
fn a_handle_cancelled_during_a_samples_evaluation_stops_it() {
    let world = world();
    let subject = world.reading();
    let ValueType::Composite(key) = &subject else {
        panic!("Reading is a record");
    };
    let samples: Vec<Value> = (0..20_000).map(|_| reading_sample(key, 1)).collect();
    let run = |cancel: &Cancel| {
        let request = MonitorRequest {
            clauses: vec![record_clause("Long")],
            trace: MonitorTrace::Samples {
                subject: subject.clone(),
                samples: &samples,
            },
            snapshots: &world.documents.snapshots,
            packages: &world.documents.packages,
            sources: &world.event.sources,
            temporal_selections: &[],
        };
        monitor(
            world.event.unit.package(),
            &request,
            &limits(1 << 20),
            cancel,
        )
    };
    // A handle that counts every meter charge made under it and cancels at
    // the `at`-th, so the stop point is a charge count, never a duration.
    let counting = |at: Option<u64>| {
        let charges = Arc::new(AtomicU64::new(0));
        let slot: Arc<OnceLock<Cancel>> = Arc::new(OnceLock::new());
        let (count, handle) = (Arc::clone(&charges), Arc::clone(&slot));
        let cancel = Cancel::observing(move || {
            let seen = count.fetch_add(1, Ordering::SeqCst) + 1;
            if Some(seen) == at {
                if let Some(cancel) = handle.get() {
                    cancel.cancel(CancelCause::Requested);
                }
            }
        });
        slot.set(cancel.clone()).expect("the slot is set once");
        (cancel, charges)
    };
    let (live, total) = counting(None);
    let completed = run(&live)
        .expect("the uncounted request completes")
        .into_value();
    assert_eq!(only(&completed).verdict, MonitorVerdict::Tested);
    let total = total.load(Ordering::SeqCst);
    assert!(
        total > 100,
        "the evaluation charges the meter {total} times"
    );

    let at = total / 2;
    let (cancel, charges) = counting(Some(at));
    let result = run(&cancel);
    assert!(
        matches!(result, Err(StageFailure::Cancelled(CancelCause::Requested))),
        "{result:?}"
    );
    assert_eq!(
        charges.load(Ordering::SeqCst),
        at,
        "the evaluation stopped at the charge that saw the cancellation"
    );
}

fn window(
    refs: Vec<DocumentRef>,
    stamps: &[u64],
    watermark: u64,
    binding: TraceBinding,
) -> TraceInput {
    TraceInput::Timestamped {
        observations: refs
            .into_iter()
            .zip(stamps)
            .map(|(document, stamp)| TimestampedObservation {
                document,
                stamp: *stamp,
                order_key: None,
            })
            .collect(),
        progress: ProgressAssertion {
            watermark,
            completeness: ProgressCompleteness::Complete { through: 5 },
            scope: ProgressClosure::Closed,
            execution: ProgressClosure::Closed,
        },
        window: Box::new(TimestampedWindow {
            binding: Some(binding),
            authoritative_origin: true,
            cover: None,
            missing_valuations: vec![TimestampedMissingValuation {
                position: 1,
                atom: 1,
            }],
        }),
    }
}

fn binding(clock_name: &str) -> TraceBinding {
    TraceBinding {
        profile: "quire.temporal.timestamped-event.finite-window/v1".to_owned(),
        clock_name: clock_name.to_owned(),
        parameters: ClockConfiguration::TimestampedEvent {
            timestamp_unit: "ms".into(),
        },
    }
}

/// FR-283-AC-15: a `Typed` `Timestamped` trace is authenticated against the
/// emission of `temporal_selections`; its unresolved atom reads
/// `Indeterminate` naming the atom by source text, needs `sources`, and a
/// closed window of 3 is `Pending`.
#[trace("TC-765", "FR-283-AC-15")]
#[test]
fn a_timestamped_trace_is_authenticated_and_reads_indeterminate() {
    let mut world = world();
    let refs = world.refs("t", &[0, 2, 0]);
    let trace = window(refs.clone(), &[0, 2, 5], 5, binding("ms"));
    let run = |trace: &TraceInput, sources: &[Source], selections: &[TemporalClockSelection]| {
        world.run(
            &world.window,
            vec![model_clause("Window")],
            MonitorTrace::Typed(trace),
            sources,
            selections,
            &limits(1 << 20),
        )
    };

    let outcome = run(&trace, &world.window.sources, &world.window.selections)
        .expect("the request completes");
    let record = only(&outcome);
    assert_eq!(record.verdict, MonitorVerdict::Indeterminate);
    assert_eq!(record.category, Category::Indeterminate);
    assert_eq!(record.report().disposition.exit_code(), 11);
    let ClauseDisposition::Temporal(TemporalVerdict::Indeterminate { at, reason }) =
        &record.report().disposition
    else {
        panic!("indeterminate, got {:?}", record.report().disposition);
    };
    assert_eq!(*at, 0);
    assert_eq!(
        *reason,
        TemporalIndeterminateReason::MissingValuation {
            position: 1,
            atom: "holds(c.value > 0)".to_owned(),
            instance: None,
            also_held_by: Vec::new(),
        }
    );
    assert_eq!(record.atom(), Some("holds(c.value > 0)"));

    let Err(StageFailure::Refused(MonitorRefusal::SourceNotSupplied { clause, .. })) =
        run(&trace, &[], &world.window.selections)
    else {
        panic!("the atom needs its source");
    };
    assert_eq!(clause, "Window");

    let early = window(refs.clone(), &[0, 2, 5], 3, binding("ms"));
    let pending = run(&early, &world.window.sources, &world.window.selections)
        .expect("the request completes");
    assert_eq!(only(&pending).verdict, MonitorVerdict::Pending);
    assert_eq!(only(&pending).report().disposition.exit_code(), 0);
}

/// FR-283-AC-15: a binding that asserts another clock refuses the whole
/// request with `ClauseRefused` before any position is admitted; a missing
/// clock selection is `missing_declaration`/`missing-selection`; the clause
/// over a `Finite` trace is `invalid-value`.
#[trace("TC-765", "FR-283-AC-15")]
#[test]
fn a_binding_is_authenticated_before_any_position_is_admitted() {
    let mut world = world();
    let mut refs = world.refs("a", &[0, 2, 0]);
    refs[1] = DocumentRef {
        authority: "test".to_owned(),
        identity: "never-provisioned".to_owned(),
    };
    let wrong = window(refs.clone(), &[0, 2, 5], 5, binding("us"));
    let (clause, record) = refused_clause(world.run(
        &world.window,
        vec![model_clause("Window")],
        MonitorTrace::Typed(&wrong),
        &world.window.sources,
        &world.window.selections,
        &limits(1 << 20),
    ));
    assert_eq!(clause, "Window");
    assert_eq!(record.catalog.code(), "invalid_runtime_input");
    assert_eq!(record.catalog.cause(), "invalid-value");
    assert_eq!(record.fields["mismatched_dimension"], "Clock");

    // The agreeing binding reaches admission, which names position 1.
    let agreeing = window(refs, &[0, 2, 5], 5, binding("ms"));
    let result = world.run(
        &world.window,
        vec![model_clause("Window")],
        MonitorTrace::Typed(&agreeing),
        &world.window.sources,
        &world.window.selections,
        &limits(1 << 20),
    );
    assert!(
        matches!(
            result,
            Err(StageFailure::Refused(MonitorRefusal::Admission {
                position: 1,
                ..
            }))
        ),
        "{result:?}"
    );

    let (clause, record) = refused_clause(world.run(
        &world.window,
        vec![model_clause("Window")],
        MonitorTrace::Typed(&agreeing),
        &world.window.sources,
        &[],
        &limits(1 << 20),
    ));
    assert_eq!(clause, "Window");
    assert_eq!(record.catalog.code(), "missing_declaration");
    assert_eq!(record.catalog.cause(), "missing-selection");

    let finite = world.document_trace("f", &[0, 2, 0]);
    let (clause, record) = refused_clause(world.run(
        &world.window,
        vec![model_clause("Window")],
        MonitorTrace::Document(&finite),
        &world.window.sources,
        &[],
        &limits(1 << 20),
    ));
    assert_eq!(clause, "Window");
    assert_eq!(record.catalog.cause(), "invalid-value");
}

/// FR-283-AC-13: a record-trace-subject clause over a `Document` trace, and a
/// model-subject clause with no `over`, are refused before FR-330 admits them.
#[trace("TC-766", "FR-283-AC-13")]
#[test]
fn a_clause_that_does_not_fit_the_trace_shape_is_refused() {
    let mut world = world();
    let bytes = world.document_trace("s", &[0, 1]);
    let (clause, record) = refused_clause(world.run(
        &world.event,
        vec![record_clause("Gate")],
        MonitorTrace::Document(&bytes),
        &world.event.sources,
        &[],
        &limits(1 << 20),
    ));
    assert_eq!(clause, "Gate");
    assert_eq!(record.catalog.cause(), "invalid-value");

    let (clause, record) = refused_clause(world.run(
        &world.event,
        vec![record_clause("Small")],
        MonitorTrace::Document(&bytes),
        &world.event.sources,
        &[],
        &limits(1 << 20),
    ));
    assert_eq!(clause, "Small");
    assert_eq!(record.catalog.cause(), "wrong-role-mapping");

    let result = world.run(
        &world.event,
        vec![model_clause("Absent")],
        MonitorTrace::Document(&bytes),
        &world.event.sources,
        &[],
        &limits(1 << 20),
    );
    assert!(
        matches!(&result, Err(StageFailure::Refused(MonitorRefusal::UnknownClause { name })) if name == "Absent"),
        "{result:?}"
    );
}

/// The refusal record a request defect is reported as, with its clause.
fn request_defect(
    result: Result<MonitorOutcome, StageFailure<MonitorRefusal>>,
) -> (
    String,
    String,
    String,
    std::collections::BTreeMap<&'static str, String>,
) {
    let (clause, record) = refused_clause(result);
    (
        clause,
        record.catalog.code().to_owned(),
        record.catalog.cause().to_owned(),
        record.fields.clone(),
    )
}

fn window_run(
    world: &World,
    trace: &TraceInput,
    selections: &[TemporalClockSelection],
    packages: &qsl_semantics::model::intake::PackageInput,
) -> Result<MonitorOutcome, StageFailure<MonitorRefusal>> {
    let request = MonitorRequest {
        clauses: vec![model_clause("Window")],
        trace: MonitorTrace::Typed(trace),
        snapshots: &world.documents.snapshots,
        packages,
        sources: &world.window.sources,
        temporal_selections: selections,
    };
    monitor(
        world.window.unit.package(),
        &request,
        &limits(1 << 20),
        &Cancel::new(),
    )
    .map(|staged| staged.into_value())
}

/// FR-283 Behavior (FND-001): the defects of the request's
/// `temporal_selections` are `ClauseRefused` request refusals (exit class
/// 20), never `StageFailure::Fault`.
#[trace("TC-765", "FR-283-AC-15")]
#[test]
fn defective_clock_selections_are_request_refusals() {
    let mut world = world();
    let refs = world.refs("s", &[0, 2, 0]);
    let trace = window(refs, &[0, 2, 5], 5, binding("ms"));
    let all = world.window.selections.clone();
    let packages = world.documents.packages.clone();
    let fields = |member: &str| std::collections::BTreeMap::from([("member", member.to_owned())]);

    // A declaration no selected clause owns has no selection.
    let missing = window_run(&world, &trace, &all[..1], &packages);
    assert_eq!(
        request_defect(missing),
        (
            "Window".to_owned(),
            "missing_declaration".to_owned(),
            "missing-selection".to_owned(),
            fields("temporal_selections")
        )
    );

    // A repeated selection.
    let mut twice = all.clone();
    twice.push(all[0].clone());
    // A selection at a span no declaration holds.
    let mut surplus = all.clone();
    surplus.push(TemporalClockSelection::new(
        all[0].source.clone(),
        qsl_foundation::Span { start: 0, end: 1 },
        ClockConfiguration::TimestampedEvent {
            timestamp_unit: "ms".into(),
        },
    ));
    // A configuration of another profile.
    let mut mismatched = all.clone();
    mismatched[0] = TemporalClockSelection::new(
        all[0].source.clone(),
        all[0].declaration_span,
        ClockConfiguration::EventPosition {
            sequence_authority: "counter-event-sequence".into(),
        },
    );
    // A timestamp unit other than the one the trace asserts.
    let mut wrong_unit = all.clone();
    wrong_unit[0] = TemporalClockSelection::new(
        all[0].source.clone(),
        all[0].declaration_span,
        ClockConfiguration::TimestampedEvent {
            timestamp_unit: "us".into(),
        },
    );
    for (name, selections) in [
        ("duplicate", twice),
        ("surplus", surplus),
        ("mismatched", mismatched),
    ] {
        assert_eq!(
            request_defect(window_run(&world, &trace, &selections, &packages)),
            (
                "Window".to_owned(),
                "invalid_runtime_input".to_owned(),
                "invalid-value".to_owned(),
                fields("temporal_selections")
            ),
            "{name}"
        );
    }
    assert_eq!(
        request_defect(window_run(&world, &trace, &wrong_unit, &packages)),
        (
            "Window".to_owned(),
            "invalid_runtime_input".to_owned(),
            "invalid-value".to_owned(),
            std::collections::BTreeMap::from([
                ("asserted", "ms".to_owned()),
                ("expected", "us".to_owned()),
                ("mismatched_dimension", "TimestampUnit".to_owned()),
            ])
        )
    );
}

/// FR-283 Behavior (FND-001): domain package documents the strict read does
/// not admit are a `ClauseRefused` request refusal, never a `Fault`.
#[trace("TC-765", "FR-283-AC-15")]
#[test]
fn domain_packages_the_strict_read_refuses_are_request_refusals() {
    let mut world = world();
    let refs = world.refs("p", &[0, 2, 0]);
    let trace = window(refs, &[0, 2, 5], 5, binding("ms"));
    let selections = world.window.selections.clone();
    let identity = world
        .documents
        .packages
        .keys()
        .next()
        .expect("one package")
        .clone();

    // Each supplied-document defect of a unit with no library dependency
    // reaches the strict read: the closure of a package with no
    // `dependency_selections` is empty, so `Closure` has no input here.
    let missing = qsl_semantics::model::intake::PackageInput::default();
    let mut truncated = world.documents.packages.clone();
    truncated
        .get_mut(&identity)
        .expect("the package is supplied")
        .truncate(1);
    let mut another = world.documents.packages.clone();
    another.insert(identity, document(true));
    for (name, packages) in [
        ("missing", missing),
        ("truncated", truncated),
        ("another document", another),
    ] {
        assert_eq!(
            request_defect(window_run(&world, &trace, &selections, &packages)),
            (
                "Window".to_owned(),
                "invalid_runtime_input".to_owned(),
                "invalid-value".to_owned(),
                std::collections::BTreeMap::from([
                    ("member", "packages".to_owned()),
                    ("stage", "strict-read".to_owned()),
                ])
            ),
            "{name}"
        );
    }
}

/// FR-522 (FND-003): a forged profile, clock parameters or timestamp unit is
/// refused with its mismatched dimension before any position is admitted,
/// though the trace names an unprovisioned snapshot.
#[trace("TC-765", "FR-283-AC-15")]
#[test]
fn a_forged_binding_dimension_is_refused_before_admission() {
    let mut world = world();
    let mut refs = world.refs("d", &[0, 2, 0]);
    refs[1] = DocumentRef {
        authority: "test".to_owned(),
        identity: "never-provisioned".to_owned(),
    };
    let selections = world.window.selections.clone();
    let packages = world.documents.packages.clone();
    let forged = |change: &dyn Fn(&mut TraceBinding)| {
        let mut forged = binding("ms");
        change(&mut forged);
        window(refs.clone(), &[0, 2, 5], 5, forged)
    };
    let cases: Vec<(&str, TraceInput, &str)> = vec![
        (
            "profile",
            forged(&|binding| {
                binding.profile = "quire.temporal.event-position.false-extension/v1".to_owned();
            }),
            "Profile",
        ),
        (
            "parameters",
            forged(&|binding| {
                binding.parameters = ClockConfiguration::EventPosition {
                    sequence_authority: "ms".into(),
                };
            }),
            "ClockParameters",
        ),
        (
            "unit",
            forged(&|binding| {
                binding.parameters = ClockConfiguration::TimestampedEvent {
                    timestamp_unit: "us".into(),
                };
            }),
            "TimestampUnit",
        ),
    ];
    for (name, trace, dimension) in cases {
        let (clause, code, cause, fields) =
            request_defect(window_run(&world, &trace, &selections, &packages));
        assert_eq!(
            (clause.as_str(), code.as_str(), cause.as_str()),
            ("Window", "invalid_runtime_input", "invalid-value"),
            "{name}"
        );
        assert_eq!(fields["mismatched_dimension"], dimension, "{name}");
    }
}

/// FR-283 Behavior: a `Progressed` trace goes through `monitor`, its binding
/// authenticated against the emission before any position is admitted.
#[trace("TC-765", "FR-283-AC-15")]
#[test]
fn a_progressed_trace_is_authenticated_through_monitor() {
    let mut world = world();
    let refs = world.refs("g", &[0, 1, 0]);
    let progressed = |refs: Vec<DocumentRef>, clock: &str| TraceInput::Progressed {
        trace: refs,
        progress: ProgressAssertion {
            watermark: 2,
            completeness: ProgressCompleteness::Complete { through: 2 },
            scope: ProgressClosure::Closed,
            execution: ProgressClosure::Closed,
        },
        lower_boundary: spine::ProgressLowerBoundary::AuthoritativeOrigin,
        binding: TraceBinding {
            profile: "quire.temporal.event-position.false-extension/v1".to_owned(),
            clock_name: clock.to_owned(),
            parameters: ClockConfiguration::EventPosition {
                sequence_authority: "counter-event-sequence".into(),
            },
        },
    };
    let run = |trace: &TraceInput| {
        world.run(
            &world.event,
            vec![model_clause("Small")],
            MonitorTrace::Typed(trace),
            &world.event.sources,
            &world.event.selections,
            &limits(1 << 20),
        )
    };
    let outcome = run(&progressed(refs.clone(), "model-steps")).expect("the request completes");
    let record = only(&outcome);
    assert!(
        record.report().authenticated_temporal_selection.is_some(),
        "the binding was authenticated"
    );

    let mut unprovisioned = refs;
    unprovisioned[1] = DocumentRef {
        authority: "test".to_owned(),
        identity: "never-provisioned".to_owned(),
    };
    let (clause, code, cause, fields) = request_defect(run(&progressed(unprovisioned, "other")));
    assert_eq!(
        (clause.as_str(), code.as_str(), cause.as_str()),
        ("Small", "invalid_runtime_input", "invalid-value")
    );
    assert_eq!(fields["mismatched_dimension"], "Clock");
}

/// FND-004: an atom is a `holds` or aggregate node; a formula node of
/// another kind, or none, is never named by its node index.
#[trace("TC-765", "FR-283-AC-15")]
#[test]
fn only_a_holds_or_aggregate_atom_is_named() {
    let world = world();
    let package = world.window.unit.package();
    let sources = &world.window.sources;
    assert_eq!(
        spine::name_clause_atom(package, sources, "Window", 1).expect("the holds atom"),
        "holds(c.value > 0)"
    );
    for (index, expected) in [
        (0, ("temporal", "named-atom-is-a-holds-or-aggregate-node")),
        (99, ("monitor", "named-atom-is-a-formula-node")),
    ] {
        let Err(refusal) = spine::name_clause_atom(package, sources, "Window", index) else {
            panic!("node {index} is no atom");
        };
        let spine::RunRefusal::Fault(fault) = *refusal else {
            panic!("an internal failure for node {index}");
        };
        assert_eq!((fault.stage(), fault.invariant()), expected, "node {index}");
    }
}

/// FR-283 "Trace document" order (FND-005): the reader and kind refusals
/// precede the object and value limits.
#[trace("TC-766", "FR-283-AC-8")]
#[test]
fn the_kind_refusal_precedes_the_document_limits() {
    let world = world();
    let mut tight = limits(1 << 20);
    tight.observation.values_per_document = 1;
    tight.observation.objects_per_document = 0;
    let result = world.run(
        &world.event,
        vec![model_clause("Small")],
        MonitorTrace::Document(b"[1,2,3]"),
        &[],
        &[],
        &tight,
    );
    let Err(StageFailure::Refused(MonitorRefusal::TraceDocument { record, pointer })) = result
    else {
        panic!("a trace document defect, got {result:?}");
    };
    assert_eq!(
        (
            record.catalog.code(),
            record.catalog.cause(),
            pointer.as_str()
        ),
        ("invalid_runtime_input", "wrong-value-kind", "")
    );
}

/// A `monitor` over `clauses` of `unit` and its `Typed` `trace`.
fn run_clauses(
    world: &World,
    unit: &Compiled,
    clauses: &[&str],
    trace: &TraceInput,
    selections: &[TemporalClockSelection],
    packages: &qsl_semantics::model::intake::PackageInput,
) -> Result<MonitorOutcome, StageFailure<MonitorRefusal>> {
    let request = MonitorRequest {
        clauses: clauses.iter().map(|name| model_clause(name)).collect(),
        trace: MonitorTrace::Typed(trace),
        snapshots: &world.documents.snapshots,
        packages,
        sources: &unit.sources,
        temporal_selections: selections,
    };
    monitor(
        unit.unit.package(),
        &request,
        &limits(1 << 20),
        &Cancel::new(),
    )
    .map(|staged| staged.into_value())
}

/// FR-283-AC-16: a missing selection and an empty `packages` each name the
/// clause the rule picks (the owner of the missing selection when selected,
/// otherwise the first selected clause), with no record.
#[trace("TC-765", "FR-283-AC-16")]
#[test]
fn request_defects_name_the_owner_or_the_first_clause() {
    let mut world = world();
    let refs = world.refs("n", &[0, 2, 0]);
    let trace = window(refs, &[0, 2, 5], 5, binding("ms"));
    let all = world.window.selections.clone();
    assert_eq!(all.len(), 2, "Window and Window2 each bear a clock");
    let packages = world.documents.packages.clone();

    // `Window2`'s selection is the one missing, so it is named although
    // `Window` is selected first.
    let missing = run_clauses(
        &world,
        &world.window,
        &["Window", "Window2"],
        &trace,
        &all[..1],
        &packages,
    );
    assert_eq!(
        request_defect(missing),
        (
            "Window2".to_owned(),
            "missing_declaration".to_owned(),
            "missing-selection".to_owned(),
            std::collections::BTreeMap::from([("member", "temporal_selections".to_owned())])
        )
    );

    // Empty `packages` fails the strict read, naming the first selected.
    let empty = qsl_semantics::model::intake::PackageInput::default();
    let strict = run_clauses(
        &world,
        &world.window,
        &["Window2", "Window"],
        &trace,
        &all,
        &empty,
    );
    assert_eq!(
        request_defect(strict),
        (
            "Window2".to_owned(),
            "invalid_runtime_input".to_owned(),
            "invalid-value".to_owned(),
            std::collections::BTreeMap::from([
                ("member", "packages".to_owned()),
                ("stage", "strict-read".to_owned()),
            ])
        )
    );
}

/// FR-283-AC-16: a package checked against a library dependency whose own
/// domain package document `packages` omits is refused at the `closure`
/// stage.
#[trace("TC-765", "FR-283-AC-16")]
#[test]
fn a_missing_library_domain_package_is_refused_at_the_closure_stage() {
    let mut world = world();
    let library = DependencyInput::new([qsl_replay::spine::SuppliedLibrary {
        identity: "test/lib".to_owned(),
        source: SourceIdentity::new("agent-ix", "lib"),
        path: "lib.native".to_owned(),
        bytes: b"language \"ix:native\" edition \"1-draft\";\n\
                 profile v = \"quire.value.complete/v1\";\n\
                 model L = \"test/counter\";\n\
                 function ok using v(): Boolean pure { true }\n"
            .to_vec(),
    }])
    .expect("an admissible dependency input");
    let text = window_unit().replacen("model M", "import \"test/lib\" as g;\nmodel M", 1);
    let unit = compile_with(&world.documents, text, &library);
    let refs = world.refs("c", &[0, 2, 0]);
    let trace = window(refs, &[0, 2, 5], 5, binding("ms"));
    let selections = unit.selections.clone();
    let empty = qsl_semantics::model::intake::PackageInput::default();
    let result = run_clauses(&world, &unit, &["Window"], &trace, &selections, &empty);
    assert_eq!(
        request_defect(result),
        (
            "Window".to_owned(),
            "invalid_runtime_input".to_owned(),
            "invalid-value".to_owned(),
            std::collections::BTreeMap::from([
                ("member", "packages".to_owned()),
                ("stage", "closure".to_owned()),
            ])
        )
    );
}

/// FR-283-AC-13: over FR-501-AC-7's trace (stamps 0, 1, 1, no order keys) as a
/// `Typed` `Timestamped` trace with an agreeing `binding`, `until` reports a
/// `TraceRefused` record `invalid_runtime_input`/`invalid-value` at position 2,
/// category refusal, exit 20.
#[trace("TC-766", "FR-283-AC-13")]
#[test]
fn a_tied_stamp_without_order_keys_is_a_trace_refused_record() {
    let mut world = world();
    let refs = world.refs("u", &[0, 1, 2]);
    let mut trace = window(refs, &[0, 1, 1], 5, binding("ms"));
    if let TraceInput::Timestamped { window, .. } = &mut trace {
        window.missing_valuations.clear();
    }
    let outcome = world
        .run(
            &world.until,
            vec![model_clause("Until")],
            MonitorTrace::Typed(&trace),
            &world.until.sources,
            &world.until.selections,
            &limits(1 << 20),
        )
        .expect("the request completes");
    let record = only(&outcome);
    assert_eq!(record.verdict, MonitorVerdict::TraceRefused);
    assert_eq!(record.category, Category::Refusal);
    assert_eq!(record.report().disposition.exit_code(), 20);
    let ClauseDisposition::TemporalRefusal(refusal) = &record.report().disposition else {
        panic!("a trace refusal, got {:?}", record.report().disposition);
    };
    assert!(
        matches!(
            refusal,
            spine::TemporalTraceRefusal::SameStampOrderUnavailable { position: 2 }
        ),
        "{refusal:?}"
    );
    let code = refusal.catalog_code();
    assert_eq!(code.code(), "invalid_runtime_input");
    assert_eq!(code.cause(), "invalid-value");
}

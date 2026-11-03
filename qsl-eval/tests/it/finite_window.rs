// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-501 (TC-1295): a temporal clause under the timestamped-event
//! finite-window profile evaluated over a timestamped trace.
//!
//! Each clause is real source taken through S1, S2, the assembler and the
//! `TemporalTrace` family's check. The `holds` conditions are not checked at
//! S3, so the letter reads each atom by name (`holds(p)`) from the
//! valuations the test authors per position; the expected values come from
//! the authored stamps and from QSpec's vectors, never from the evaluator.

use std::collections::HashMap;

use ix_trace_rs::trace;
use qsl_eval::temporal::{
    evaluate, evaluate_finite_window, Assessment, AtomReading, Basis, ClockCover, Closure,
    Completeness, Containment, IndeterminateReason, LetterStop, LowerBoundary, OrderKey,
    TemporalLetters, TemporalOutcome, TemporalPosition, TemporalRefusal, TemporalTrace,
    TimestampedObservation, TimestampedTrace, Valuation, Watermark, WindowClock, WindowProgress,
    WindowVerdict,
};
use qsl_semantics::check::{FormulaNode, FormulaNodeId, SCALAR_LIMITS_UNLIMITED};
use quire_exact::{ChargePoint, LimitKind, Meter, ScalarLimits};

use crate::temporal_trace::{fixture, Fixture, EVENT};

const FINITE_WINDOW: &str = "quire.temporal.timestamped-event.finite-window/v1";

fn clause(activation: &str, body: &str) -> Fixture {
    fixture(FINITE_WINDOW, "c: Counter", activation, body)
}

fn origin(body: &str) -> Fixture {
    clause("on origin", body)
}

/// A valuation spelled as the vectors do: `T`, `F`, `absent`, `conflict`.
fn valuation(word: &str) -> Valuation {
    match word {
        "T" => Valuation::True,
        "F" => Valuation::False,
        "absent" => Valuation::Missing,
        "conflict" => Valuation::Conflicting,
        other => panic!("unknown valuation {other}"),
    }
}

/// One supplied position: stamp, the valuation of each named atom, and an
/// order key.
#[derive(Clone)]
struct Row {
    stamp: u64,
    atoms: Vec<(&'static str, &'static str)>,
    key: Option<(&'static str, u64)>,
}

fn p(stamp: u64, value: &'static str) -> Row {
    Row {
        stamp,
        atoms: vec![("p", value)],
        key: None,
    }
}

/// A request: the rows as supplied, and the progress.
#[derive(Clone)]
struct Request {
    rows: Vec<Row>,
    watermark: Watermark,
    complete: Completeness,
    scope: Closure,
    execution: Closure,
    lower: LowerBoundary,
    cover: ClockCover,
    unit: &'static str,
}

fn request(rows: Vec<Row>, watermark: u64, complete: Option<u64>) -> Request {
    Request {
        rows,
        watermark: Watermark::Inclusive(watermark),
        complete: complete.map_or(Completeness::Incomplete, |through| Completeness::Complete {
            through,
        }),
        scope: Closure::Closed,
        execution: Closure::Closed,
        lower: LowerBoundary::AuthoritativeOrigin,
        cover: ClockCover::Exact,
        unit: "ms",
    }
}

impl Request {
    fn open(mut self) -> Self {
        self.scope = Closure::Open;
        self
    }

    fn cutoff(mut self) -> Self {
        self.lower = LowerBoundary::HistoryCutoff;
        self
    }

    fn counter(mut self, uncertainty: u64) -> Self {
        self.cover = ClockCover::Counter { uncertainty };
        self
    }

    fn admit(&self, fixture: &Fixture) -> Result<TimestampedTrace, TemporalRefusal> {
        TimestampedTrace::admit(
            fixture.clause.name(),
            WindowClock {
                unit: self.unit.to_owned(),
                cover: self.cover,
            },
            self.rows
                .iter()
                .enumerate()
                .map(|(index, row)| TimestampedObservation {
                    stamp: row.stamp,
                    order_key: row.key.map(|(authority, key)| OrderKey {
                        authority: authority.to_owned(),
                        key,
                    }),
                    observation: format!("{index}:{:?}", row.atoms),
                })
                .collect(),
            WindowProgress {
                watermark: self.watermark,
                completeness: self.complete,
                scope: self.scope,
                execution: self.execution,
                lower_boundary: self.lower,
            },
        )
    }
}

/// The letter: reads `holds(<name>)` by name from the valuations of the
/// supplied position that the admitted position names.
struct Letters<'a> {
    fixture: &'a Fixture,
    by_admitted: Vec<HashMap<&'static str, &'static str>>,
}

impl<'a> Letters<'a> {
    fn new(fixture: &'a Fixture, request: &Request, trace: &TimestampedTrace) -> Self {
        let by_admitted = (0..trace.len())
            .map(|position| {
                let supplied = trace
                    .supplied_index(TemporalPosition::new(position as u64))
                    .expect("a supplied index");
                request.rows[supplied].atoms.iter().copied().collect()
            })
            .collect();
        Self {
            fixture,
            by_admitted,
        }
    }

    fn name(&self, atom: FormulaNodeId) -> &str {
        let Some(FormulaNode::Holds { span }) =
            self.fixture.clause.formula().nodes().get(atom.index())
        else {
            panic!("the letter reads only holds atoms");
        };
        self.fixture.text[span.start..span.end]
            .strip_prefix("holds(")
            .and_then(|rest| rest.strip_suffix(')'))
            .expect("a holds atom")
    }
}

impl TemporalLetters for Letters<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        match self.valuation(atom, position, _meter)? {
            Valuation::True => Ok(AtomReading::True),
            Valuation::False => Ok(AtomReading::False),
            other => panic!("the control reads definite valuations, not {other:?}"),
        }
    }

    fn valuation(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<Valuation, LetterStop> {
        let name = self.name(atom).to_owned();
        let word = self
            .by_admitted
            .get(position.index() as usize)
            .and_then(|atoms| atoms.get(name.as_str()))
            .copied()
            .unwrap_or("absent");
        Ok(valuation(word))
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        panic!("these clauses have no `when` condition");
    }
}

fn limits(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        work_units,
        ..SCALAR_LIMITS_UNLIMITED
    }
}

/// The report and the work consumed.
fn run_with(
    fixture: &Fixture,
    request: &Request,
    budget: ScalarLimits,
) -> (
    Result<qsl_eval::temporal::WindowEvaluation, TemporalRefusal>,
    u64,
) {
    let trace = request.admit(fixture).expect("the trace admits");
    let mut letters = Letters::new(fixture, request, &trace);
    let mut meter = Meter::new(budget);
    let result = evaluate_finite_window(&fixture.clause, &trace, &mut letters, &mut meter);
    (result, meter.consumed(LimitKind::WorkUnits))
}

fn verdict(fixture: &Fixture, request: &Request) -> WindowVerdict {
    run_with(fixture, request, SCALAR_LIMITS_UNLIMITED)
        .0
        .expect("the trace is readable")
        .verdict
}

fn at(index: u64) -> TemporalPosition {
    TemporalPosition::new(index)
}

fn violated(index: u64) -> WindowVerdict {
    WindowVerdict::Violated { at: at(index) }
}

fn pending(index: u64, deadline: u64) -> WindowVerdict {
    WindowVerdict::Pending {
        at: at(index),
        deadline,
    }
}

fn indeterminate(index: u64, reason: IndeterminateReason) -> WindowVerdict {
    WindowVerdict::Indeterminate {
        at: at(index),
        reason,
    }
}

fn rows(stamps: &[u64], values: &[&'static str]) -> Vec<Row> {
    stamps.iter().zip(values).map(|(s, v)| p(*s, v)).collect()
}

fn atom_id(fixture: &Fixture) -> FormulaNodeId {
    let index = fixture
        .clause
        .formula()
        .nodes()
        .iter()
        .position(|node| matches!(node, FormulaNode::Holds { .. }))
        .expect("an atom");
    fixture.clause.formula().node_id(index).expect("a node id")
}

/// FR-501-AC-1 (TC-1295 rows 1 to 3): only positions whose stamp distance
/// lies in the closed interval take part; a position stamped at the bound
/// does, one past it does not.
#[trace("TC-1295", "FR-501-AC-1")]
#[test]
fn the_interval_selects_positions_by_stamp_distance() {
    let stamps = [0, 2, 5];
    let always = origin("always[0,5] holds(p)");
    assert_eq!(
        verdict(
            &always,
            &request(rows(&stamps, &["T", "F", "T"]), 5, Some(5))
        ),
        violated(0)
    );
    assert_eq!(
        verdict(
            &always,
            &request(rows(&stamps, &["T", "T", "T"]), 5, Some(5))
        ),
        WindowVerdict::Holds
    );
    let short = origin("eventually[0,1] holds(p)");
    assert_eq!(
        verdict(
            &short,
            &request(rows(&stamps, &["F", "T", "F"]), 5, Some(5))
        ),
        violated(0)
    );
    let wide = origin("eventually[0,2] holds(p)");
    assert_eq!(
        verdict(&wide, &request(rows(&stamps, &["F", "T", "F"]), 5, Some(5))),
        WindowVerdict::Holds
    );
    // A position stamped 6 is outside [0, 5]; one stamped 5 is inside.
    let beyond = request(rows(&[0, 2, 5, 6], &["T", "T", "T", "F"]), 6, Some(6));
    assert_eq!(verdict(&always, &beyond), WindowVerdict::Holds);
}

/// FR-501-AC-2 (rows 4 and 5): no synthetic false atom after closure, where
/// event-position false extension gives false; empty windows of an
/// established activation settle.
#[trace("TC-1295", "FR-501-AC-2")]
#[test]
fn an_established_window_adds_no_synthetic_atom() {
    let single = origin("always[0,1] holds(p)");
    assert_eq!(
        verdict(&single, &request(vec![p(0, "T")], 1, Some(1))),
        WindowVerdict::Holds
    );
    // The control: event-position false extension reads the missing second
    // position as false.
    let control = fixture(EVENT, "c: Counter", "on origin", "always[0,1] holds(p)");
    let request_control = request(vec![p(0, "T")], 1, Some(1));
    let trace = request_control.admit(&control).expect("admits");
    let mut letters = Letters::new(&control, &request_control, &trace);
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let outcome = evaluate(
        &control.clause,
        TemporalTrace::Finite { len: 1 },
        &mut letters,
        &mut meter,
    )
    .expect("readable");
    assert_eq!(outcome, TemporalOutcome::Violated { at: at(0) });
    let later = request(vec![p(0, "T")], 3, Some(3));
    assert_eq!(
        verdict(&origin("eventually[2,3] holds(p)"), &later),
        violated(0)
    );
    assert_eq!(
        verdict(&origin("always[2,3] holds(p)"), &later),
        WindowVerdict::Holds
    );
}

/// FR-501-AC-3 (rows 6 and 7): pending until progress closes the window;
/// the same positions with later progress settle; a decisive witness or
/// counterexample settles an open window.
#[trace("TC-1295", "FR-501-AC-3")]
#[test]
fn an_open_window_is_pending_until_progress_or_a_decisive_event() {
    let eventually = origin("eventually[0,5] holds(p)");
    let two = rows(&[0, 1], &["F", "F"]);
    assert_eq!(
        verdict(&eventually, &request(two.clone(), 1, Some(1)).open()),
        pending(0, 5)
    );
    assert_eq!(
        verdict(&eventually, &request(two, 5, Some(5)).open()),
        violated(0)
    );
    assert_eq!(
        verdict(
            &eventually,
            &request(rows(&[0, 1], &["F", "T"]), 1, Some(1)).open()
        ),
        WindowVerdict::Holds
    );
    assert_eq!(
        verdict(
            &origin("always[0,5] holds(p)"),
            &request(rows(&[0, 1], &["T", "F"]), 1, Some(1)).open()
        ),
        violated(0)
    );
}

/// FR-501-AC-4 (rows 8 to 11): a missing or conflicting valuation is never
/// read as false.
#[trace("TC-1295", "FR-501-AC-4")]
#[test]
fn missing_and_conflicting_valuations_are_never_false() {
    let eventually = origin("eventually[0,5] holds(p)");
    let atom = atom_id(&eventually);
    let stamps = [0, 2, 5];
    assert_eq!(
        verdict(
            &eventually,
            &request(rows(&stamps, &["F", "absent", "F"]), 5, Some(5))
        ),
        indeterminate(
            0,
            IndeterminateReason::MissingValuation {
                position: at(1),
                atom
            }
        )
    );
    assert_eq!(
        verdict(
            &eventually,
            &request(rows(&stamps, &["F", "absent", "T"]), 5, Some(5))
        ),
        WindowVerdict::Holds
    );
    assert_eq!(
        verdict(
            &eventually,
            &request(rows(&stamps, &["F", "conflict", "F"]), 5, Some(5))
        ),
        indeterminate(
            0,
            IndeterminateReason::ConflictingValuation {
                position: at(1),
                atom
            }
        )
    );
    assert_eq!(
        verdict(
            &eventually,
            &request(rows(&stamps, &["F", "absent", "F"]), 3, Some(3))
        ),
        pending(0, 5)
    );
}

/// FR-501-AC-5 (rows 12 and 13): an incomplete input never admits the
/// empty-window truth.
#[trace("TC-1295", "FR-501-AC-5")]
#[test]
fn an_incomplete_input_is_indeterminate_never_closed() {
    let always = origin("always[0,5] holds(p)");
    let incomplete = IndeterminateReason::IncompleteInput;
    assert_eq!(
        verdict(&always, &request(vec![p(0, "T")], 5, None)),
        indeterminate(0, incomplete.clone())
    );
    assert_eq!(
        verdict(&always, &request(vec![p(0, "T")], 5, Some(5))),
        WindowVerdict::Holds
    );
    let empty = origin("eventually[2,3] holds(p)");
    assert_eq!(
        verdict(&empty, &request(vec![p(0, "T")], 3, None)),
        indeterminate(0, incomplete.clone())
    );
    assert_eq!(
        verdict(&empty, &request(vec![p(0, "T")], 3, Some(2))),
        indeterminate(0, incomplete)
    );
}

/// FR-501-AC-6 (rows 14 to 16): the inclusive deadline participates, silence
/// settles a window, and a half-open watermark without a mapping is the
/// inclusive watermark one below its end.
#[trace("TC-1295", "FR-501-AC-6")]
#[test]
fn the_inclusive_deadline_participates_and_silence_settles() {
    let clause = origin("eventually[0,3] holds(q)");
    let q = |stamp, value| Row {
        stamp,
        atoms: vec![("q", value)],
        key: None,
    };
    assert_eq!(
        verdict(&clause, &request(vec![q(0, "F"), q(3, "T")], 3, Some(3))),
        WindowVerdict::Holds
    );
    assert_eq!(
        verdict(&clause, &request(vec![q(0, "F"), q(4, "T")], 4, Some(4))),
        violated(0)
    );
    let silent = origin("eventually[0,3] holds(p)");
    assert_eq!(
        verdict(&silent, &request(vec![p(0, "F")], 3, Some(3))),
        violated(0)
    );
    let mut half_open = request(vec![p(0, "F")], 3, Some(3));
    half_open.watermark = Watermark::HalfOpen { end: 3 };
    assert_eq!(verdict(&silent, &half_open), pending(0, 3));
    let report = run_with(&silent, &half_open, SCALAR_LIMITS_UNLIMITED)
        .0
        .expect("readable");
    assert_eq!(report.premises.watermark, 2);
}

/// FR-501-AC-7 (rows 17 and 18): equal stamps need comparable order keys at
/// an order-sensitive operator, refused at the later position.
#[trace("TC-1295", "FR-501-AC-7")]
#[test]
fn equal_stamps_need_comparable_order_keys() {
    let plain = request(
        vec![
            Row {
                stamp: 0,
                atoms: vec![("p", "T"), ("q", "F")],
                key: None,
            },
            Row {
                stamp: 1,
                atoms: vec![("p", "T"), ("q", "F")],
                key: None,
            },
            Row {
                stamp: 1,
                atoms: vec![("p", "T"), ("q", "F")],
                key: None,
            },
        ],
        1,
        Some(1),
    );
    let refused = |body: &str, activation: &str| {
        let fixture = clause(activation, body);
        run_with(&fixture, &plain, SCALAR_LIMITS_UNLIMITED).0
    };
    for (body, activation) in [
        ("holds(p) until[0,1] holds(q)", "on origin"),
        ("holds(p) release[0,1] holds(q)", "on origin"),
        ("holds(p) since[0,1] holds(q)", "on each (q: Counter)"),
        ("holds(p) triggered[0,1] holds(q)", "on each (q: Counter)"),
    ] {
        assert_eq!(
            refused(body, activation).err(),
            Some(TemporalRefusal::UnorderedTie { position: at(2) }),
            "{body}"
        );
    }
    assert!(refused("eventually[0,1] holds(p)", "on origin").is_ok());
    // Keys from one authority order the run, whatever the supply order.
    let keyed_rows = |first: u64, second: u64, second_authority: &'static str| {
        let atoms = vec![("p", "T"), ("q", "F")];
        let row = |stamp, key| Row {
            stamp,
            atoms: atoms.clone(),
            key,
        };
        request(
            vec![
                row(0, None),
                row(1, Some(("A", first))),
                row(1, Some((second_authority, second))),
            ],
            1,
            Some(1),
        )
    };
    let until = clause("on origin", "holds(p) until[0,1] holds(q)");
    let one_authority = keyed_rows(1, 2, "A");
    let forward = run_with(&until, &one_authority, SCALAR_LIMITS_UNLIMITED)
        .0
        .expect("evaluates");
    let swapped = keyed_rows(2, 1, "A");
    let reordered = run_with(&until, &swapped, SCALAR_LIMITS_UNLIMITED)
        .0
        .expect("evaluates");
    assert_eq!(forward.verdict, reordered.verdict);
    assert_eq!(
        swapped.admit(&until).expect("admits").supplied_index(at(1)),
        Some(2),
        "the key order, not the supply order, is the admitted order"
    );
    let two_authorities = keyed_rows(1, 2, "B");
    assert_eq!(
        run_with(&until, &two_authorities, SCALAR_LIMITS_UNLIMITED)
            .0
            .err(),
        Some(TemporalRefusal::UnorderedTie { position: at(2) })
    );
}

/// FR-501-AC-8 (row 19): a past operator reaching before a history cutoff is
/// indeterminate unless history already decides; an authoritative origin
/// gives a definite verdict.
#[trace("TC-1295", "FR-501-AC-8")]
#[test]
fn a_history_cutoff_is_missing_history_unless_decided() {
    let once = clause("on each (q: Counter)", "once[0,4] holds(p)");
    let none = request(rows(&[3, 4], &["F", "F"]), 4, Some(4));
    assert_eq!(
        verdict(&once, &none.clone().cutoff()),
        indeterminate(0, IndeterminateReason::MissingHistory)
    );
    assert_eq!(verdict(&once, &none), violated(0));
    let witnessed = request(rows(&[3, 4], &["T", "F"]), 4, Some(4));
    assert_eq!(
        verdict(&once, &witnessed.clone().cutoff()),
        WindowVerdict::Holds
    );
    assert_eq!(verdict(&once, &witnessed), WindowVerdict::Holds);
}

/// A counter-cover trace of AC-9: `req` at tick 0 and `ack` at `ack`, with
/// progress and completeness through `through`.
fn handshake(ack: u64, through: u64, uncertainty: Option<u64>) -> Request {
    let row = |stamp, req, ack| Row {
        stamp,
        atoms: vec![("req", req), ("ack", ack)],
        key: None,
    };
    let base = request(
        vec![row(0, "T", "F"), row(ack, "F", "T")],
        through,
        Some(through),
    );
    match uncertainty {
        Some(u) => base.counter(u),
        None => base,
    }
}

/// FR-501-AC-9 (rows 20 and 21): a counter cover that straddles the bound is
/// indeterminate; the exact cover is not; the comparison functions agree
/// with QSpec's tick-cover vectors.
#[trace("TC-1295", "FR-501-AC-9")]
#[test]
fn a_counter_cover_straddling_the_bound_is_indeterminate() {
    let clause = origin("holds(req) implies eventually[0,3000] holds(ack)");
    let straddles = IndeterminateReason::StraddlesBound;
    for (ack, counter, exact) in [
        (2998, WindowVerdict::Holds, WindowVerdict::Holds),
        (
            3000,
            indeterminate(0, straddles.clone()),
            WindowVerdict::Holds,
        ),
        (3001, violated(0), violated(0)),
    ] {
        assert_eq!(
            verdict(&clause, &handshake(ack, 3001, Some(0))),
            counter,
            "counter u=0 ack {ack}"
        );
        assert_eq!(
            verdict(&clause, &handshake(ack, 3001, None)),
            exact,
            "exact ack {ack}"
        );
    }
    // TCV-06 to TCV-08 with uncertainty 1.
    for (ack, expected) in [
        (2997, WindowVerdict::Holds),
        (2998, indeterminate(0, straddles.clone())),
        (3003, violated(0)),
    ] {
        assert_eq!(
            verdict(&clause, &handshake(ack, 3003, Some(1))),
            expected,
            "counter u=1 ack {ack}"
        );
    }
}

/// The comparison functions on QSpec's published vectors (TCV-01 to
/// TCV-15), whose expected values are the file's.
#[trace("TC-1295", "FR-501-AC-9")]
#[test]
fn the_comparison_functions_agree_with_the_tick_cover_vectors() {
    let Some(qspec) = std::env::var_os("QSPEC_DIR") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let path =
        std::path::Path::new(&qspec).join("proposals/checked-package-v2/tick-cover-vectors.json");
    let text = std::fs::read_to_string(&path).expect("the vectors file reads");
    let document: serde_json::Value = serde_json::from_str(&text).expect("the vectors parse");
    let cases = document["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 15);
    for case in cases {
        let id = case["id"].as_str().expect("id");
        let cover = match case["uncertainty"].as_u64().expect("uncertainty") {
            0 => ClockCover::Counter { uncertainty: 0 },
            uncertainty => ClockCover::Counter { uncertainty },
        };
        match case["kind"].as_str().expect("kind") {
            "within_bound" => {
                let got = cover.within_bound(
                    case["bound"].as_i64().expect("bound"),
                    case["request_tick"].as_u64().expect("request"),
                    case["ack_tick"].as_u64().expect("ack"),
                );
                let want = match case["expected"].as_str().expect("expected") {
                    "true" => Containment::True,
                    "false" => Containment::False,
                    "indeterminate" => Containment::Indeterminate,
                    other => panic!("{id}: {other}"),
                };
                assert_eq!(got, want, "{id}");
            }
            "definitely_before" => {
                let got = cover.definitely_before(
                    case["a_tick"].as_u64().expect("a"),
                    case["b_tick"].as_u64().expect("b"),
                );
                assert_eq!(got, case["expected"].as_bool().expect("expected"), "{id}");
            }
            other => panic!("{id}: {other}"),
        }
    }
}

/// FR-501-AC-10 (row 22): a definitely false activation outranks a later
/// pending one; an unresolved earlier one is returned when none is false.
#[trace("TC-1295", "FR-501-AC-10")]
#[test]
fn a_false_activation_outranks_a_pending_one() {
    let each = clause("on each (q: Counter)", "eventually[0,2] holds(p)");
    assert_eq!(
        verdict(&each, &request(rows(&[0, 10], &["F", "F"]), 10, Some(10))),
        violated(0)
    );
    assert_eq!(
        verdict(&each, &request(rows(&[0, 10], &["T", "F"]), 10, Some(10))),
        pending(1, 12)
    );
}

/// FR-501-AC-12 (row 23, 26): seven charges; a meter of six returns
/// `Incomplete` at `temporal.visit`; equal inputs give equal results.
#[trace("TC-1295", "FR-501-AC-12")]
#[test]
fn work_is_charged_per_visit_and_scan() {
    let clause = origin("eventually[0,5] holds(p)");
    let request = request(rows(&[0, 2, 5], &["F", "T", "F"]), 5, Some(5));
    let (done, charged) = run_with(&clause, &request, limits(7));
    assert_eq!(done.expect("readable").verdict, WindowVerdict::Holds);
    assert_eq!(charged, 7);
    let (denied, _) = run_with(&clause, &request, limits(6));
    match denied.expect("readable").verdict {
        WindowVerdict::Incomplete(incomplete) => {
            assert_eq!(incomplete.charge_point, ChargePoint::TemporalVisit);
        }
        other => panic!("expected Incomplete, not {other:?}"),
    }
    let first = run_with(&clause, &request, limits(7)).0.expect("readable");
    let second = run_with(&clause, &request, limits(7)).0.expect("readable");
    assert_eq!(first, second);
}

/// FR-501-AC-12: a sibling activation settled before the denied charge keeps
/// its verdict.
#[trace("TC-1295", "FR-501-AC-12")]
#[test]
fn a_settled_sibling_keeps_its_verdict_when_a_charge_is_denied() {
    let each = clause("on each (q: Counter)", "eventually[0,0] holds(p)");
    let request = request(rows(&[0, 1], &["F", "T"]), 1, Some(1));
    let (full, charged) = run_with(&each, &request, SCALAR_LIMITS_UNLIMITED);
    assert_eq!(full.expect("readable").verdict, violated(0));
    // One charge short of the whole run: the first activation is settled.
    let (short, _) = run_with(&each, &request, limits(charged - 1));
    assert_eq!(short.expect("readable").verdict, violated(0));
    // With no sibling settled, a denied charge is Incomplete.
    let alone = origin("eventually[0,0] holds(p)");
    let (denied, _) = run_with(
        &alone,
        &self::request(vec![p(0, "T")], 0, Some(0)),
        limits(1),
    );
    assert!(matches!(
        denied.expect("readable").verdict,
        WindowVerdict::Incomplete(_)
    ));
}

/// FR-501-AC-13 (row 27): the margin 2u holds a counter-cover activation
/// pending until 3002; the exact cover settles at 3000.
#[trace("TC-1295", "FR-501-AC-13")]
#[test]
fn the_counter_margin_delays_establishment() {
    let clause = origin("holds(req) implies eventually[0,3000] holds(ack)");
    let only_req = |watermark: u64, through: u64, u: Option<u64>| {
        let base = request(
            vec![Row {
                stamp: 0,
                atoms: vec![("req", "T"), ("ack", "F")],
                key: None,
            }],
            watermark,
            Some(through),
        );
        u.map_or(base.clone(), |u| base.counter(u))
    };
    assert_eq!(
        verdict(&clause, &only_req(3001, 3002, Some(1))),
        pending(0, 3002)
    );
    assert_eq!(
        verdict(&clause, &only_req(3002, 3002, Some(1))),
        violated(0)
    );
    assert_eq!(verdict(&clause, &only_req(3000, 3000, None)), violated(0));
}

/// FR-501-AC-14 (row 28): basis, verdict and decision support are separate
/// fields.
#[trace("TC-1295", "FR-501-AC-14")]
#[test]
fn the_report_gives_basis_and_decision_support() {
    let always = origin("always[0,5] holds(p)");
    let atom = atom_id(&always);
    let stamps = [0, 2, 5];
    let closed = request(rows(&stamps, &["T", "F", "T"]), 5, Some(5));
    let report = run_with(&always, &closed, SCALAR_LIMITS_UNLIMITED)
        .0
        .expect("r");
    assert_eq!(report.verdict, violated(0));
    assert_eq!(report.basis, Basis::ClosedScope);
    assert_eq!(report.support.len(), 1);
    assert_eq!(
        (report.support[0].position, report.support[0].atom),
        (at(1), atom)
    );
    let open = run_with(&always, &closed.clone().open(), SCALAR_LIMITS_UNLIMITED)
        .0
        .expect("r");
    assert_eq!(open.verdict, violated(0));
    assert_eq!(open.basis, Basis::DecisiveCounterexample);
    assert_eq!(open.support, report.support);
    let eventually = origin("eventually[0,5] holds(p)");
    let witnessed = request(rows(&stamps, &["F", "F", "T"]), 5, Some(5));
    let report = run_with(&eventually, &witnessed, SCALAR_LIMITS_UNLIMITED)
        .0
        .expect("r");
    assert_eq!(report.verdict, WindowVerdict::Holds);
    assert_eq!(report.basis, Basis::ClosedScope);
    assert_eq!(report.support.len(), 1);
    assert_eq!(report.support[0].position, at(2));
    let pending_report = run_with(
        &eventually,
        &request(rows(&[0, 1], &["F", "F"]), 1, Some(1)).open(),
        SCALAR_LIMITS_UNLIMITED,
    )
    .0
    .expect("r");
    assert_eq!(pending_report.basis, Basis::Unsettled);
    let open_witness = run_with(
        &eventually,
        &request(rows(&[0, 1], &["F", "T"]), 1, Some(1)),
        SCALAR_LIMITS_UNLIMITED,
    )
    .0
    .expect("r");
    assert_eq!(open_witness.basis, Basis::DecisiveWitness);
    let unknown = run_with(
        &eventually,
        &request(rows(&stamps, &["F", "absent", "F"]), 5, Some(5)),
        SCALAR_LIMITS_UNLIMITED,
    )
    .0
    .expect("r");
    assert_eq!(unknown.basis, Basis::Unavailable);
    // Every valuation of the window would decide it: the missing one either
    // way, and a false one flipped to true.
    let positions: Vec<TemporalPosition> =
        unknown.support.iter().map(|pair| pair.position).collect();
    assert_eq!(positions, vec![at(0), at(1), at(2)]);
}

/// FR-501-AC-15 (row 29): scope, execution, assessment and completeness stay
/// four independent fields.
#[trace("TC-1295", "FR-501-AC-15")]
#[test]
fn the_four_dimensions_stay_independent() {
    let always = origin("always[0,5] holds(p)");
    let base = request(rows(&[0, 2, 5], &["T", "F", "T"]), 5, Some(5));
    let closed = run_with(&always, &base, SCALAR_LIMITS_UNLIMITED)
        .0
        .expect("r");
    let open = run_with(&always, &base.clone().open(), SCALAR_LIMITS_UNLIMITED)
        .0
        .expect("r");
    assert_eq!(open.verdict, closed.verdict);
    assert_eq!(open.support, closed.support);
    assert_ne!(open.basis, closed.basis);
    assert_eq!(open.premises.scope, Closure::Open);
    assert_eq!(
        (open.premises.execution, open.premises.completeness),
        (closed.premises.execution, closed.premises.completeness)
    );
    let mut surrounding = base.clone();
    surrounding.execution = Closure::Open;
    let moved = run_with(&always, &surrounding, SCALAR_LIMITS_UNLIMITED)
        .0
        .expect("r");
    assert_eq!(moved.premises.execution, Closure::Open);
    assert_eq!(moved.premises.scope, closed.premises.scope);
    assert_eq!(
        (moved.verdict.clone(), moved.basis, moved.support.clone()),
        (closed.verdict.clone(), closed.basis, closed.support.clone())
    );
    assert_eq!(closed.premises.assessment, Assessment::Completed);
    let (stopped, _) = run_with(&always, &base, limits(1));
    assert_eq!(stopped.expect("r").premises.assessment, Assessment::Failed);
    let incomplete = run_with(
        &always,
        &request(rows(&[0, 2, 5], &["T", "T", "T"]), 5, None),
        SCALAR_LIMITS_UNLIMITED,
    )
    .0
    .expect("r");
    assert_eq!(
        incomplete.verdict,
        indeterminate(0, IndeterminateReason::IncompleteInput)
    );
    assert_eq!(incomplete.premises.scope, Closure::Closed);
    assert_eq!(incomplete.premises.execution, Closure::Closed);
}

/// FR-501-AC-16 (row 30): one result identity per input; each input changed
/// alone changes it.
#[trace("TC-1295", "FR-501-AC-16")]
#[test]
fn the_result_identity_follows_every_input() {
    let always = origin("always[0,5] holds(p)");
    let base = request(rows(&[0, 2, 5], &["T", "F", "T"]), 5, Some(5));
    let identity = |request: &Request, budget: ScalarLimits| {
        run_with(&always, request, budget).0.expect("r").identity
    };
    let original = identity(&base, SCALAR_LIMITS_UNLIMITED);
    assert_eq!(original, identity(&base, SCALAR_LIMITS_UNLIMITED));
    let mut changes: Vec<(&str, Request, ScalarLimits)> = Vec::new();
    let mut unit = base.clone();
    unit.unit = "us";
    changes.push(("unit", unit, SCALAR_LIMITS_UNLIMITED));
    changes.push(("cover", base.clone().counter(0), SCALAR_LIMITS_UNLIMITED));
    let mut stamp = base.clone();
    stamp.rows[1].stamp = 3;
    changes.push(("stamp", stamp, SCALAR_LIMITS_UNLIMITED));
    let mut value = base.clone();
    value.rows[0].atoms = vec![("p", "F")];
    changes.push(("valuation", value, SCALAR_LIMITS_UNLIMITED));
    let mut key = base.clone();
    key.rows[2].key = Some(("A", 1));
    changes.push(("order key", key, SCALAR_LIMITS_UNLIMITED));
    let mut watermark = base.clone();
    watermark.watermark = Watermark::Inclusive(6);
    changes.push(("watermark", watermark, SCALAR_LIMITS_UNLIMITED));
    let mut through = base.clone();
    through.complete = Completeness::Complete { through: 6 };
    changes.push(("through", through, SCALAR_LIMITS_UNLIMITED));
    changes.push((
        "lower boundary",
        base.clone().cutoff(),
        SCALAR_LIMITS_UNLIMITED,
    ));
    changes.push(("ceiling", base.clone(), limits(1_000)));
    let mut seen = vec![original.clone()];
    for (name, changed, budget) in changes {
        let changed = identity(&changed, budget);
        assert!(
            !seen.contains(&changed),
            "{name} did not change the identity"
        );
        seen.push(changed);
    }
    assert_eq!(original, identity(&base, SCALAR_LIMITS_UNLIMITED));
}

/// FR-501-AC-17 (row 31): completeness may run ahead of the watermark; an
/// empty or decreasing trace is refused before any position is visited.
#[trace("TC-1295", "FR-501-AC-17")]
#[test]
fn completeness_may_run_ahead_and_a_malformed_trace_is_refused() {
    let eventually = origin("eventually[0,5] holds(p)");
    let falses = rows(&[0, 2, 5], &["F", "F", "F"]);
    assert_eq!(
        verdict(&eventually, &request(falses.clone(), 5, Some(9))),
        violated(0)
    );
    assert_eq!(
        verdict(&eventually, &request(falses, 3, Some(9))),
        pending(0, 5)
    );
    let empty = request(Vec::new(), 5, Some(5)).admit(&eventually);
    assert_eq!(empty.err(), Some(TemporalRefusal::EmptyTrace));
    let decreasing = request(rows(&[0, 5, 2], &["F", "F", "F"]), 5, Some(5)).admit(&eventually);
    assert_eq!(
        decreasing.err(),
        Some(TemporalRefusal::DecreasingStamp { position: at(2) })
    );
}

/// FR-501 "Verdicts": a clause under another profile is refused.
#[trace("TC-1295", "FR-501-AC-1")]
#[test]
fn a_clause_under_another_profile_is_refused() {
    let control = fixture(EVENT, "c: Counter", "on origin", "always[0,1] holds(p)");
    let request = request(vec![p(0, "T")], 1, Some(1));
    let trace = request.admit(&control).expect("admits");
    let mut letters = Letters::new(&control, &request, &trace);
    let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
    let refused = evaluate_finite_window(&control.clause, &trace, &mut letters, &mut meter);
    assert!(matches!(
        refused,
        Err(TemporalRefusal::NotFiniteWindowProfile { .. })
    ));
}

/// FR-501 "Verdicts": nested operators lift over the possibility sets of
/// the positions they read, and an atom that is undefined at an eligible
/// position is returned as FR-327 states.
#[trace("TC-1295", "FR-501-AC-4")]
#[test]
fn a_nested_clause_lifts_over_missing_valuations() {
    let nested = origin("always[0,2] eventually[0,2] holds(p)");
    // p is true at stamp 1 and 2, missing at 0: each inner window has a
    // true witness, so the missing valuation changes nothing.
    assert_eq!(
        verdict(
            &nested,
            &request(
                rows(&[0, 1, 2, 3, 4], &["absent", "T", "T", "T", "T"]),
                4,
                Some(4)
            )
        ),
        WindowVerdict::Holds
    );
    // The inner window of the anchor at stamp 2 is {2, 3, 4}: all false,
    // so the outer always is false at the origin.
    assert_eq!(
        verdict(
            &nested,
            &request(
                rows(&[0, 1, 2, 3, 4], &["T", "T", "F", "F", "F"]),
                4,
                Some(4)
            )
        ),
        violated(0)
    );
}

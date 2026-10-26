// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230: S3 checks a timed model's clocks, guards, resets, time
//! invariants and urgency. The fixture is ADR-026 §11's `Rpc` call, as a
//! domain package object type `Call` whose clocks are package fields typed
//! `ix://quire/native/Clock`, with the guards and resets written as the
//! unit's own state clauses.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    CheckCause, CheckRefusal, CheckedGraph, CheckedNode, ClockOp, NodeKind, StatePredicate,
    TemporalCause, TimeSourceKind, TimedConstruct, TimedModel,
};
use qsl_semantics::model::intake::meaning;
use quire_exact::{Integer, Rational};
use serde_json::{json, Value};

use crate::model_operations::{
    admit_and_assemble_with_profiles, config_unit_with_profiles, operation,
    version_number_value_type, wire_construct, wire_envelope, wire_field, wire_type,
};

const PACKAGE: &str = "example/config-version";
const TEMPORAL: &str = "profile t = \"quire.temporal.complete/v1\";\n";
const UNITS: &str = "dimension Time;\nunit s : Time = rational(1, 1);\n\
                     unit ms : Time = rational(1, 1000) * s;\n";

fn call(member: &str) -> String {
    format!("ix://{PACKAGE}/Call/{member}")
}

/// A field of `Call`; a clock is typed `Clock` and, as FCD admits it,
/// non-unique.
fn field(name: &str, type_ref: &str) -> Value {
    let mut field = wire_field(&call(name), name, type_ref, "required", 1);
    if type_ref == "ix://quire/native/Clock" {
        field["multiplicity"]["unique"] = json!(false);
    }
    field
}

/// An operation of `Call` writing `modifies`.
fn call_operation(name: &str, modifies: &[&str]) -> Value {
    let mut operation = operation(
        name,
        json!([]),
        None,
        json!({
            "modifies": modifies.iter().map(|field| call(field)).collect::<Vec<_>>(),
            "creates": [],
            "deletes": [],
        }),
    );
    operation["identity"] = json!(call(name));
    operation
}

/// `Call` with Boolean `waiting`, `inflight`, `late` and `ready`, the bound
/// integer `n`, the clocks `clocks`, and the operations `send` (writing
/// `waiting`, `inflight`, `late` and `x`), `reply`, `timeout` and `bump`
/// (writing `n` and `x`), over the population `calls`.
fn rpc_document(clocks: &[&str]) -> Vec<u8> {
    rpc_document_with(clocks, false)
}

/// [`rpc_document`], with a record value type `Probe` holding a Boolean
/// `x` when `probe` holds.
fn rpc_document_with(clocks: &[&str], probe: bool) -> Vec<u8> {
    let (number_construct, number_type, number) = version_number_value_type();
    let boolean = "ix://quire/native/Boolean";
    let mut fields = vec![
        field("waiting", boolean),
        field("inflight", boolean),
        field("late", boolean),
        field("ready", boolean),
        field("n", &number),
    ];
    fields.extend(
        clocks
            .iter()
            .map(|clock| field(clock, "ix://quire/native/Clock")),
    );
    let call_type = format!("ix://{PACKAGE}/Call");
    let mut types = json!([
        number_type,
        wire_type(
            &call_type,
            "object_type",
            json!({
                "supertypes": [],
                "fields": fields,
                "operations": [
                    call_operation("send", &["waiting", "inflight", "late", "x"]),
                    call_operation("reply", &["inflight", "late", "waiting"]),
                    call_operation("timeout", &["waiting"]),
                    call_operation("bump", &["n", "x"]),
                ],
                "relationships": [],
            }),
        ),
    ]);
    if probe {
        let probe_type = format!("ix://{PACKAGE}/Probe");
        let x = format!("{probe_type}/x");
        types.as_array_mut().expect("types").push(wire_type(
            &probe_type,
            "value_record",
            json!({"fields": [wire_field(&x, "x", boolean, "required", 1)]}),
        ));
    }
    let population = format!("ix://{PACKAGE}/calls");
    let populations = json!([{
        "identity": population.clone(),
        "displayName": population.clone(),
        "kind": {"module": PACKAGE, "name": "population"},
        "members": [call_type],
        "extent": "closed",
        "origin": {
            "generated": {
                "generatorIdentity": population.clone(),
                "generatorVersion": "1.0.0",
                "inputIdentities": [population],
            }
        },
    }]);
    let mut constructs = json!([
        wire_construct("object_type", meaning::OBJECT_TYPE, json!({})),
        wire_construct("population", meaning::POPULATION, json!({})),
        number_construct,
    ]);
    if probe {
        constructs
            .as_array_mut()
            .expect("constructs")
            .push(wire_construct(
                "value_record",
                meaning::RECORD_VALUE_TYPE,
                json!({}),
            ));
    }
    wire_envelope(constructs, types, populations)
        .to_string()
        .into_bytes()
}

/// ADR-026 §11's `Rpc` members, in the unit's own spelling, under `time`.
fn rpc(time: &str) -> String {
    format!(
        "{UNITS}{time}\n\
         pre SendIdle using v on Config::Call::send {{ not self.waiting }}\n\
         post Send using v on Config::Call::send {{ \
             self.waiting and self.inflight and not self.late and self.x = 0 ms }}\n\
         pre Reply using v on Config::Call::reply {{ self.inflight and self.x >= 1 ms }}\n\
         pre Timeout using v on Config::Call::timeout {{ self.waiting and self.x >= 3 ms }}\n\
         time invariant ServerDeadline when {{ self.inflight }} {{ self.x <= 3 ms }}\n\
         time invariant ClientDeadline when {{ self.waiting }} {{ self.x <= 3 ms }}\n"
    )
}

fn check(document: &[u8], body: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    admit_and_assemble_with_profiles(document, TEMPORAL, body)
        .expect("S2 builds every timed form")
        .with_unwritten_timed_members()
        .check(quire_semantic_value::checking::CheckingLimits::default())
}

fn timed(graph: &CheckedGraph) -> &TimedModel {
    graph.timed_model("Config").expect("a timed model")
}

fn exact(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("a rational")
}

/// The one timed refusal of `body` over `document`, with the text it is
/// located at.
fn refused(document: &[u8], body: &str) -> (TemporalCause, String) {
    let refusals = check(document, body).expect_err("the timed member is refused");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal expected: {refusals:?}");
    };
    let CheckCause::Temporal(cause) = &refusal.cause else {
        panic!("a timed refusal expected: {refusal:?}");
    };
    let (text, _) = config_unit_with_profiles(document, TEMPORAL, body);
    let span = cause.span();
    ((**cause).clone(), text[span.start..span.end].to_owned())
}

/// The model attributes `node` reads, by name, in ascending order.
fn attributes(node: CheckedNode<'_>) -> Vec<String> {
    let mut names = Vec::new();
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if let NodeKind::Attribute { field, .. } = node.kind() {
            names.push(field.name.clone());
        }
        pending.extend(node.children());
    }
    names.sort();
    names.dedup();
    names
}

/// Part of FR-230-AC-1 and of FR-230-AC-8 (TC-685): `Rpc` under `time dense
/// unit ms` checks, from a unit whose header selects the temporal layer, to
/// a dense source, the one clock `Call.x`, the two time invariants bounding
/// `x` by 3, `send`'s reset of `x` to 0 and `reply`'s guard with its one
/// convex part `x >= 1` beside the data predicate `self.inflight`, which no
/// longer reads the clock. Not covered: AC-1's S2 forms and the `s` unit's
/// package identity (S4); AC-8's E3 resolution and AC-7's refusals for the
/// unit with no header.
#[trace("TC-685")]
#[test]
fn rpc_checks_to_its_clocks_guards_resets_and_invariants() {
    let graph = check(&rpc_document(&["x"]), &rpc("time Config dense unit ms;"))
        .expect("Rpc checks under the timed facet");
    assert!(matches!(
        graph.time_sources()[0].source,
        TimeSourceKind::Dense
    ));
    assert_eq!(graph.time_sources()[0].unit, "ms");
    let model = timed(&graph);
    let names: Vec<&str> = model.clocks().iter().map(|clock| clock.name()).collect();
    assert_eq!(names, ["x"]);
    let x = model.clocks()[0].clone();

    let invariants: Vec<_> = model
        .invariants()
        .iter()
        .map(|invariant| {
            let [bound] = invariant.bounds() else {
                panic!("one bound");
            };
            (
                invariant.name(),
                model.clock(bound.clock()).expect("a clock").name(),
                bound.op(),
                bound.bound().clone(),
            )
        })
        .collect();
    assert_eq!(
        invariants,
        [
            ("ServerDeadline", "x", ClockOp::LessOrEqual, exact(3, 1)),
            ("ClientDeadline", "x", ClockOp::LessOrEqual, exact(3, 1)),
        ]
    );

    let [send] = model.resets() else {
        panic!("one postcondition resets a clock");
    };
    assert_eq!(send.clause(), "Send");
    assert_eq!(send.operation().declaration.name(), "send");
    let [reset] = send.resets() else {
        panic!("send resets one clock");
    };
    assert_eq!(model.clock(reset.clock()), Some(&x));
    assert_eq!(reset.value(), &exact(0, 1));

    let guards: Vec<_> = model
        .guards()
        .iter()
        .map(|guard| (guard.clause(), guard.operation().declaration.name()))
        .collect();
    assert_eq!(guards, [("Reply", "reply"), ("Timeout", "timeout")]);
    let [part] = model.guards()[0].parts() else {
        panic!("reply's guard has one convex part");
    };
    assert_eq!(
        attributes(part.data().expect("the part keeps `self.inflight`").body()),
        ["inflight"]
    );
    let [constraint] = part.clocks() else {
        panic!("one clock constraint");
    };
    assert_eq!(
        (constraint.op(), constraint.bound()),
        (ClockOp::GreaterOrEqual, &exact(1, 1))
    );
    // The data reading of the clause no longer reads the clock.
    assert_eq!(
        attributes(graph.state_clause("Reply").expect("Reply checks").body()),
        ["inflight"]
    );
}

/// Part of FR-230-AC-1 (TC-685): under `time dense unit s`, `reply`'s bound
/// `1 ms` converts to `1/1000`. Not covered: the `s` unit's package identity
/// differing from `ms`'s, which S4 emits.
#[trace("TC-685")]
#[test]
fn a_bound_converts_exactly_to_the_model_unit() {
    let graph = check(&rpc_document(&["x"]), &rpc("time Config dense unit s;"))
        .expect("Rpc checks in seconds");
    let model = timed(&graph);
    let [part] = model.guards()[0].parts() else {
        panic!("one convex part");
    };
    assert_eq!(part.clocks()[0].bound(), &exact(1, 1000));
    assert_eq!(model.resets()[0].resets()[0].value(), &exact(0, 1));
}

/// FR-230-AC-4's S3 half (TC-685 step 4): `(x < 1 or x > 2) and
/// self.ready` is two convex parts, `x < 1` and `x > 2`, each with the data
/// predicate `self.ready`. The S2 forms are `qsl-forms`' `time_forms` test
/// and the 6,400 clock places with the `10^40 ms` constant `qsl-eval`'s
/// `timed_subject` test.
#[trace("TC-685", "FR-230-AC-4")]
#[test]
fn a_disjunctive_guard_splits_into_convex_parts() {
    let body = format!(
        "{UNITS}time Config dense unit ms;\n\
         pre Bump using v on Config::Call::bump {{ \
             (self.x < 1 ms or self.x > 2 ms) and self.ready }}\n"
    );
    let graph = check(&rpc_document(&["x"]), &body).expect("the guard checks");
    let [guard] = timed(&graph).guards() else {
        panic!("one guard");
    };
    let parts: Vec<_> = guard
        .parts()
        .iter()
        .map(|part| {
            let [constraint] = part.clocks() else {
                panic!("one constraint per part");
            };
            (
                attributes(part.data().expect("a data predicate").body()),
                constraint.op(),
                constraint.bound().clone(),
            )
        })
        .collect();
    assert_eq!(
        parts,
        [
            (vec!["ready".to_owned()], ClockOp::Less, exact(1, 1)),
            (vec!["ready".to_owned()], ClockOp::Greater, exact(2, 1)),
        ]
    );
}

/// FR-230-AC-4 (TC-685 step 4): 64 package clock fields are 64 clocks of
/// `Call`, and a `10^40 ms` constant checks exactly. The 6,400 places over
/// 100 objects are `qsl-eval`'s `timed_subject` test.
#[trace("TC-685", "FR-230-AC-4")]
#[test]
fn many_clocks_and_a_huge_constant_check() {
    let names: Vec<String> = (0..64).map(|index| format!("c{index:02}")).collect();
    let mut clocks: Vec<&str> = names.iter().map(String::as_str).collect();
    clocks.push("x");
    let huge = format!("1{} ms", "0".repeat(40));
    let body = format!(
        "{UNITS}time Config dense unit ms;\n\
         pre Bump using v on Config::Call::bump {{ self.x <= {huge} }}\n"
    );
    let graph = check(&rpc_document(&clocks), &body).expect("the constant checks");
    let model = timed(&graph);
    assert_eq!(model.clocks().len(), 65);
    let bound = model.guards()[0].parts()[0].clocks()[0].bound().clone();
    let expected = format!("1{}", "0".repeat(40));
    assert_eq!(bound.numerator().to_string(), expected);
}

/// Part of FR-230-AC-2 (TC-685): each ill-formed clock use refuses at its
/// span, a clock read through a path other than `self` and a clock write
/// nested under `implies` among them. Not covered: the duplicate-source,
/// `unit m` and untimed-`urgent when` vectors (other TC-685 tests), and the
/// `time-invariant-shape` vector the grammar does not admit (STD-493).
#[trace("TC-685")]
#[test]
fn ill_formed_clock_uses_refuse_at_their_span() {
    let document = rpc_document(&["x", "y"]);
    let dense = format!("{UNITS}time Config dense unit ms;\n");
    for (clause, tag, at) in [
        (
            "post P using v on Config::Call::bump { self.n = self.x }",
            "clock-read-outside-constraint",
            "self.x",
        ),
        (
            "pre P using v on Config::Call::bump { self.x <= self.y }",
            "clock-comparison",
            "self.x <= self.y",
        ),
        (
            "pre P using v on Config::Call::bump { self.x >= -1 ms }",
            "negative-clock-constant",
            "-1 ms",
        ),
        (
            "post P using v on Config::Call::bump { self.x = self.x + 1 ms }",
            "clock-write",
            "self.x = self.x + 1 ms",
        ),
        (
            "post P using v on Config::Call::bump { self.ready implies self.x = 0 ms }",
            "clock-write",
            "self.x = 0 ms",
        ),
        (
            "pre P using v on Config::Call::bump { \
             forall(o in allInstances<Config::Call>(Config::calls) : o.x <= 1 ms) }",
            "clock-read-outside-constraint",
            "o.x",
        ),
    ] {
        let (cause, text) = refused(&document, &format!("{dense}{clause}\n"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::InvalidTimedModel, tag, at),
            "{clause}"
        );
    }
}

/// FR-230: a clock read through a binder over `allInstances<Call>` is
/// found on `Call` itself, though another type, the record `Probe`,
/// declares a Boolean field of the same name `x`.
#[trace("TC-685")]
#[test]
fn a_clock_read_is_found_on_its_operands_type() {
    let document = rpc_document_with(&["x"], true);
    let (cause, text) = refused(
        &document,
        &format!(
            "{UNITS}time Config dense unit ms;\n\
             pre P using v on Config::Call::bump {{ \
             forall(o in allInstances<Config::Call>(Config::calls) : o.x <= 1 ms) }}\n"
        ),
    );
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (
            Code::InvalidTimedModel,
            "clock-read-outside-constraint",
            "o.x"
        )
    );
}

/// Part of FR-230-AC-2 (TC-685): a clock field under a tick source refuses
/// `clock-without-dense-time` at the time declaration. Not covered: the
/// field's own span, which a package field does not have (STD-493).
#[trace("TC-685")]
#[test]
fn a_clock_under_tick_time_refuses() {
    let body = format!("{UNITS}time Config tick Config::Call::bump period 1 ms unit ms;\n");
    let (cause, text) = refused(&rpc_document(&["x"]), &body);
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (
            Code::InvalidTimedModel,
            "clock-without-dense-time",
            "time Config tick Config::Call::bump period 1 ms unit ms;"
        )
    );
}

/// Part of FR-230-AC-2 (TC-685): `urgent` on an operation whose guard reads
/// a clock refuses `urgent-guard-reads-clock` at the declaration; on one
/// whose guard reads none it checks.
#[trace("TC-685")]
#[test]
fn urgency_on_a_clock_guarded_operation_refuses() {
    let document = rpc_document(&["x"]);
    let urgent = "urgent Prompt Config::Call::reply;";
    let (cause, text) = refused(
        &document,
        &format!("{}{urgent}\n", rpc("time Config dense unit ms;")),
    );
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (Code::InvalidTimedModel, "urgent-guard-reads-clock", urgent)
    );
    let graph = check(
        &document,
        &format!(
            "{}urgent Eager Config::Call::send;\nurgent Busy when {{ self.inflight }};\n",
            rpc("time Config dense unit ms;")
        ),
    )
    .expect("urgency on an unguarded operation and on a predicate checks");
    assert_eq!(timed(&graph).urgency().len(), 2);
}

/// FR-230 "Clocks and resets": a package clock field of a model with no
/// `time` member that no clause reads refuses `clock-without-dense-time` at
/// the model's declaration, rather than dropping out of the package.
#[trace("TC-685")]
#[test]
fn an_unread_clock_of_an_untimed_model_refuses_at_the_model() {
    let (cause, text) = refused(&rpc_document(&["x"]), UNITS);
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::InvalidTimedModel, "clock-without-dense-time")
    );
    assert!(text.starts_with("model Config"), "{text}");
}

/// The declared names of the checked package's semantic nodes.
fn declared_names(graph: &CheckedGraph) -> Vec<String> {
    graph
        .semantic_graph()
        .nodes()
        .filter_map(|node| {
            let preimage: Value = serde_json::from_slice(node.preimage()).ok()?;
            let names = preimage["declaration"]["qualified_name"]
                .as_array()?
                .clone();
            Some(
                names
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("::"),
            )
        })
        .collect()
}

/// One constraint of a convex part: its clock's name, comparison and bound.
type Bound = (String, ClockOp, Rational);

/// Each convex part of `guard` as its data literals' model attributes, each
/// constraint as its clock's name, comparison and bound.
fn parts(model: &TimedModel, guard: usize) -> Vec<(Vec<String>, Vec<Bound>)> {
    model.guards()[guard]
        .parts()
        .iter()
        .map(|part| {
            (
                part.data()
                    .map(|data| attributes(data.body()))
                    .unwrap_or_default(),
                part.clocks()
                    .iter()
                    .map(|constraint| {
                        (
                            model
                                .clock(constraint.clock())
                                .expect("a clock")
                                .name()
                                .to_owned(),
                            constraint.op(),
                            constraint.bound().clone(),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

const WITHIN: &str = "predicate near using v(c: Clock, b: Quantity<ms>): Boolean { c <= b }\n";

/// Part of FR-230-AC-9 (TC-685 step 11): the clock-reading predicate
/// `near(c: Clock, b: Quantity<ms>)` expands where a precondition calls it:
/// `near(self.x, 3 ms)` is the one constraint `x <= 3` on `Call.x`, and
/// `near(self.x, 3 ms) and near(self.y, 1 ms)` two constraints; the
/// checked package holds no node for `near`. Not covered: the time
/// invariant body `near(self.x, 3 ms)`, which the time invariant grammar
/// (a list of clock bounds) does not admit, and AC-9's name `within`, a
/// reserved word (STD-493).
#[trace("TC-685")]
#[test]
fn a_clock_reading_predicate_expands_in_a_precondition() {
    let body = format!(
        "{UNITS}time Config dense unit ms;\n{WITHIN}\
         pre Near using v on Config::Call::bump {{ near(self.x, 3 ms) }}\n\
         pre Both using v on Config::Call::send {{ near(self.x, 3 ms) and near(self.y, 1 ms) }}\n"
    );
    let graph = check(&rpc_document(&["x", "y"]), &body).expect("the calls expand");
    let model = timed(&graph);
    assert_eq!(
        parts(model, 0),
        [(
            Vec::new(),
            vec![("x".to_owned(), ClockOp::LessOrEqual, exact(3, 1))]
        )]
    );
    assert_eq!(
        parts(model, 1),
        [(
            Vec::new(),
            vec![
                ("x".to_owned(), ClockOp::LessOrEqual, exact(3, 1)),
                ("y".to_owned(), ClockOp::LessOrEqual, exact(1, 1)),
            ]
        )]
    );
    assert!(
        !declared_names(&graph).iter().any(|name| name == "near"),
        "{:?}",
        declared_names(&graph)
    );
}

/// Part of FR-230-AC-9's refusals (TC-685 step 11), each at its span:
/// `near(self.x, self.n)` refuses `clock-bound-not-constant` at the
/// argument; `near(5, 3 ms)` `ill_typed`/`type-mismatch` at `5`; a body
/// `self.n = c` for a clock parameter `c` and `n: Int`
/// `clock-read-outside-constraint` at the clock read; `near(self.x, 3 ms)`
/// in an `invariant` clause `clock-read-outside-constraint` at the call. Not
/// covered: a body that resets `c`, which a predicate body cannot write
/// (STD-493).
#[trace("TC-685")]
#[test]
fn clock_reading_misuses_refuse_at_their_span() {
    let document = rpc_document(&["x", "y"]);
    let dense = format!("{UNITS}time Config dense unit ms;\n{WITHIN}");
    for (declarations, code, tag, at) in [
        (
            "pre P using v on Config::Call::bump { near(self.x, self.n) }",
            Code::InvalidTimedModel,
            "clock-bound-not-constant",
            "self.n",
        ),
        (
            "pre P using v on Config::Call::bump { near(5, 3 ms) }",
            Code::IllTyped,
            "type-mismatch",
            "5",
        ),
        (
            "predicate copies using v(c: Clock): Boolean { self.n = c }",
            Code::InvalidTimedModel,
            "clock-read-outside-constraint",
            "c",
        ),
        (
            "invariant I using v on Config::Call at current { near(self.x, 3 ms) }",
            Code::InvalidTimedModel,
            "clock-read-outside-constraint",
            "near(self.x, 3 ms)",
        ),
    ] {
        let (cause, text) = refused(&document, &format!("{dense}{declarations}\n"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (code, tag, at),
            "{declarations}"
        );
    }
}

/// Part of FR-230-AC-10 (TC-685 step 12): with `function pick(a: Clock, b:
/// Clock, left: Boolean): Clock { if left then a else b }`, the guard
/// `pick(self.x, self.y, self.ready) >= 1 ms` checks to two convex parts,
/// `x >= 1` with the data predicate `self.ready` and `y >= 1` with
/// `not self.ready`; `function g(): Clock { 0 }` refuses `ill_typed`/
/// `type-mismatch` at the body. Not covered: `peer_clock(o: Call): Clock {
/// o.peer.x }`, a place reached through a reference, which the clock-place
/// slice builds with FR-230-AC-24.
#[trace("TC-685")]
#[test]
fn a_clock_result_function_selects_its_place_by_data() {
    let document = rpc_document(&["x", "y"]);
    let dense = format!("{UNITS}time Config dense unit ms;\n");
    let body = format!(
        "{dense}function pick using v(a: Clock, b: Clock, left: Boolean): Clock pure {{ \
             if left then a else b }}\n\
         pre Picked using v on Config::Call::bump {{ pick(self.x, self.y, self.ready) >= 1 ms }}\n"
    );
    let graph = check(&document, &body).expect("the place expression expands");
    assert_eq!(
        shown_parts(timed(&graph), 0),
        [
            (
                Some("ready".to_owned()),
                vec![("x".to_owned(), ClockOp::GreaterOrEqual, exact(1, 1))]
            ),
            (
                Some("not ready".to_owned()),
                vec![("y".to_owned(), ClockOp::GreaterOrEqual, exact(1, 1))]
            ),
        ]
    );
    let (cause, text) = refused(
        &document,
        &format!("{dense}function g using v(): Clock pure {{ 0 }}\n"),
    );
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (Code::IllTyped, "type-mismatch", "0")
    );
}

/// Part of FR-230-AC-14 (TC-685 step 16): in a dense model's unit, a
/// function `f(c: Clock)`, a function `g(): Clock { self.x }`, a record with
/// a `Clock` field, `type Alias = Clock;`, `Option<Clock>` and
/// `Sequence<Clock>` raise no refusal of their own; the record's checked node
/// lists its data field only, and none of the others has a node. Not
/// covered: the package fields `t: Timer` and `ts: Sequence<Timer>` under a
/// tick source or no time source, which need package records with clock
/// fields.
#[trace("TC-685")]
#[test]
fn clock_typed_declarations_raise_no_refusal_of_their_own() {
    let body = format!(
        "{UNITS}time Config dense unit ms;\n\
         predicate f using v(c: Clock): Boolean {{ true }}\n\
         function g using v(): Clock pure {{ self.x }}\n\
         record Timer {{ x: Clock; n: Integer; }}\n\
         type Alias = Clock;\n\
         type Maybe = Option<Clock>;\n\
         type Clocks = Sequence<Clock>;\n"
    );
    let graph = check(&rpc_document(&["x"]), &body).expect("the declarations check");
    let names = declared_names(&graph);
    assert!(names.iter().any(|name| name == "Timer"), "{names:?}");
    for absent in ["f", "g", "Alias", "Maybe", "Clocks"] {
        assert!(
            !names.iter().any(|name| name == absent),
            "{absent}: {names:?}"
        );
    }
    let timer = graph
        .semantic_graph()
        .nodes()
        .find_map(|node| {
            let preimage: Value = serde_json::from_slice(node.preimage()).ok()?;
            (preimage["declaration"]["qualified_name"] == json!(["Timer"])).then_some(preimage)
        })
        .expect("the Timer record node");
    assert_eq!(
        timer["body"]["members"].as_array().map(Vec::len),
        Some(1),
        "Timer lists its data field n only: {timer}"
    );
}

/// `node`, a data predicate, written out: each attribute by name, each
/// connective, comparison and arithmetic operator by its variant.
fn shown(node: CheckedNode<'_>) -> String {
    let children = node.children();
    let child = |index: usize| children.get(index).copied().map_or_else(String::new, shown);
    match node.kind() {
        NodeKind::Not(_) => format!("not {}", child(0)),
        NodeKind::Connective(connective, ..) => {
            format!("({} {connective:?} {})", child(0), child(1))
        }
        NodeKind::Order(operator, ..) => format!("({} {operator:?} {})", child(0), child(1)),
        NodeKind::Arithmetic(operator, ..) => {
            format!("({} {operator:?} {})", child(0), child(1))
        }
        NodeKind::Attribute { field, .. } => field.name.clone(),
        NodeKind::Coerce(..) => child(0),
        NodeKind::Literal(literal) => format!("{literal:?}")
            .chars()
            .filter(char::is_ascii_digit)
            .collect(),
        other => format!("{other:?}")
            .chars()
            .take_while(|c| c.is_alphanumeric())
            .collect(),
    }
}

/// Each convex part of `guard` with its data predicate written out.
fn shown_parts(model: &TimedModel, guard: usize) -> Vec<(Option<String>, Vec<Bound>)> {
    model.guards()[guard]
        .parts()
        .iter()
        .map(|part| {
            (
                part.data().map(|data| shown(data.body())),
                part.clocks()
                    .iter()
                    .map(|constraint| {
                        (
                            model
                                .clock(constraint.clock())
                                .expect("a clock")
                                .name()
                                .to_owned(),
                            constraint.op(),
                            constraint.bound().clone(),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

/// FR-230 "Clock-reading declarations" and "Parameters" (TC-685): a
/// clock-reading body expands at its call with each data predicate read at
/// the call site, its parameters replaced by their arguments: `n > 0` of
/// `p(self.x, self.n)` is `self.n > 0` beside `x <= 3`; a nested call's data
/// argument `n + 1` is `self.n + 1`, and `self.ready` in a body is the
/// caller's; an `if` over a data comparison selects `x` under `self.n > 2`
/// and `y` under its negation; and the place path `o.y` of an object
/// parameter `o` called with `self` is the receiver's clock `y`.
#[trace("TC-685")]
#[test]
fn clock_reading_bodies_read_their_data_at_the_call() {
    let body = format!(
        "{UNITS}time Config dense unit ms;\n\
         predicate p using v(c: Clock, n: Integer): Boolean {{ n > 0 and c <= 3 ms }}\n\
         predicate q using v(c: Clock, n: Integer): Boolean {{ p(c, n + 1) or self.ready }}\n\
         function sel using v(a: Clock, b: Clock, n: Integer): Clock pure {{ \
             if n > 2 then a else b }}\n\
         function own using v(o: Config::Call): Clock pure {{ o.y }}\n\
         pre P using v on Config::Call::bump {{ p(self.x, self.n) }}\n\
         pre Q using v on Config::Call::send {{ q(self.x, self.n) }}\n\
         pre S using v on Config::Call::reply {{ sel(self.x, self.y, self.n) >= 1 ms }}\n\
         pre O using v on Config::Call::timeout {{ own(self) < 2 ms }}\n"
    );
    let graph = check(&rpc_document(&["x", "y"]), &body).expect("the calls expand");
    let model = timed(&graph);
    let bound = |clock: &str, op, value| vec![(clock.to_owned(), op, exact(value, 1))];
    assert_eq!(
        shown_parts(model, 0),
        [(
            Some("(n Greater 0)".to_owned()),
            bound("x", ClockOp::LessOrEqual, 3)
        )]
    );
    assert_eq!(
        shown_parts(model, 1),
        [
            (
                Some("((n Add 1) Greater 0)".to_owned()),
                bound("x", ClockOp::LessOrEqual, 3)
            ),
            (Some("ready".to_owned()), Vec::new()),
        ]
    );
    assert_eq!(
        shown_parts(model, 2),
        [
            (
                Some("(n Greater 2)".to_owned()),
                bound("x", ClockOp::GreaterOrEqual, 1)
            ),
            (
                Some("not (n Greater 2)".to_owned()),
                bound("y", ClockOp::GreaterOrEqual, 1)
            ),
        ]
    );
    assert_eq!(
        shown_parts(model, 3),
        [(None, bound("y", ClockOp::Less, 2))]
    );
}

/// FR-230 "Clock-reading declarations" (TC-685): a call of a clock-reading
/// declaration outside an operation precondition, a time invariant body or
/// another clock-reading body refuses `invalid_timed_model`/
/// `clock-read-outside-constraint`: in a value function, a temporal clause,
/// an operation body and a state clause of a model with no `time` member.
#[trace("TC-685")]
#[test]
fn a_clock_reading_call_elsewhere_reads_a_clock_outside_a_constraint() {
    let g = "function g using v(): Clock pure { self.x }\n";
    let timed = format!("{UNITS}time Config dense unit ms;\n{g}");
    let profiles = format!("{TEMPORAL}profile tt = \"quire.temporal.timed/v1\";\n");
    for (document, body) in [
        (
            rpc_document(&["x"]),
            format!("{timed}function f using v(n: Integer): Boolean pure {{ g() >= 1 ms }}\n"),
        ),
        (
            rpc_document(&["x"]),
            format!(
                "{timed}temporal T using tt over (c: Config::Call) clock \"model-time\" \
                 on origin {{ always holds(g() >= 1 ms) }}\n"
            ),
        ),
        (
            rpc_document(&["x"]),
            format!("{timed}body B using v on Config::Call::bump() {{ g() >= 1 ms }}\n"),
        ),
        (
            crate::model_operations::config_version_document(
                crate::model_operations::attempt_update_modifies_version_number(),
                Vec::new(),
                Vec::new(),
                json!([]),
            ),
            format!(
                "{UNITS}predicate p using v(c: Clock): Boolean {{ c <= 3 ms }}\n\
                 invariant I using v on Config::ConfigVersion at current {{ \
                 p(self.versionNumber) }}\n"
            ),
        ),
    ] {
        let refusals = admit_and_assemble_with_profiles(&document, &profiles, &body)
            .expect("S2 builds every form")
            .with_unwritten_timed_members()
            .check(quire_semantic_value::checking::CheckingLimits::default())
            .expect_err("the call is refused");
        let [refusal] = refusals.as_slice() else {
            panic!("one refusal expected: {refusals:?}");
        };
        assert_eq!(
            (
                refusal.cause.code(),
                refusal.cause.cause().map(|cause| cause.to_string())
            ),
            (
                Code::InvalidTimedModel,
                Some("clock-read-outside-constraint".to_owned())
            ),
            "{body}"
        );
    }
}

/// FR-230 "Parameters" (TC-685): an `if` whose condition reads a clock is no
/// data condition, so a place it selects refuses `clock-read-outside-
/// constraint` at the condition; a clock-reading declaration whose
/// expansion reaches its own call, which FR-230 gives no expansion
/// (STD-493), is not checked.
#[trace("TC-685")]
#[test]
fn a_place_selected_by_a_clock_or_a_recursive_expansion_refuses() {
    let document = rpc_document(&["x", "y"]);
    let dense = format!(
        "{UNITS}time Config dense unit ms;\n\
         predicate near using v(c: Clock): Boolean {{ c <= 3 ms }}\n"
    );
    let (cause, text) = refused(
        &document,
        &format!(
            "{dense}function sel using v(a: Clock, b: Clock): Clock pure {{ \
                 if near(a) then a else b }}\n\
             pre S using v on Config::Call::reply {{ sel(self.x, self.y) >= 1 ms }}\n"
        ),
    );
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (
            Code::InvalidTimedModel,
            "clock-read-outside-constraint",
            "near(a)"
        )
    );
    let refusals = check(
        &document,
        &format!(
            "{dense}predicate loop using v(c: Clock): Boolean {{ loop(c) }}\n\
             pre L using v on Config::Call::reply {{ loop(self.x) }}\n"
        ),
    )
    .expect_err("the expansion does not end");
    assert!(
        matches!(
            refusals.as_slice(),
            [CheckRefusal {
                cause: CheckCause::TimedNotChecked {
                    construct: TimedConstruct::RecursiveClockReading,
                    ..
                },
                ..
            }]
        ),
        "{refusals:?}"
    );
}

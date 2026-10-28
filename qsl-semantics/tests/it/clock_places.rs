// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Places": a clock below an object's field, through a package
//! record, a clock alias or a union case, read and reset by its path.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{ClockOp, CollectionShape, PlaceStep, SlotRef};
use quire_semantic_value::declaration::CompositeShape;
use serde_json::{json, Value};

use crate::model_operations::{version_number_value_type, wire_construct, wire_field, wire_type};
use crate::timed_model::{
    call, check, exact, field, refused, rpc_document_with, shown_parts, timed, PACKAGE, UNITS,
};
use qsl_semantics::model::intake::meaning;

/// `ix://example/config-version/<name>`.
fn package(name: &str) -> String {
    format!("ix://{PACKAGE}/{name}")
}

/// [`rpc_document_with`]`(&["x"])` with a package record `Timer { x: Clock;
/// n: Int }`, the `Call` fields `t: Timer` and `u: Timer`, written by `send`
/// and `bump`, and `edit` applied last.
fn timer_document(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    let (_, _, number) = version_number_value_type();
    let timer = package("Timer");
    let mut clock = wire_field(
        &format!("{timer}/x"),
        "x",
        "ix://quire/native/Clock",
        "required",
        1,
    );
    clock["multiplicity"]["unique"] = json!(false);
    let types = document["types"].as_array_mut().expect("types");
    types.push(wire_type(
        &timer,
        "value_record",
        json!({"fields": [clock, wire_field(&format!("{timer}/n"), "n", &number, "required", 1)]}),
    ));
    let fields = types[1]["fields"].as_array_mut().expect("Call fields");
    fields.push(field("t", &timer));
    fields.push(field("u", &timer));
    for operation in types[1]["operations"].as_array_mut().expect("operations") {
        if matches!(operation["name"].as_str(), Some("send" | "bump")) {
            let modifies = operation["frame"]["modifies"]
                .as_array_mut()
                .expect("frame");
            modifies.push(json!(call("t")));
            modifies.push(json!(call("u")));
        }
    }
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(wire_construct(
            "value_record",
            meaning::RECORD_VALUE_TYPE,
            json!({}),
        ));
    edit(&mut document);
    serde_json::to_vec(&document).expect("JSON")
}

const DENSE: &str = "time Config dense unit ms;\n";

/// Part of FR-230-AC-11 (TC-685 step 13): with the package record `Timer {
/// x: Clock; n: Int }` held by the `Call` fields `t` and `u`, the model's
/// clock places are `Call.t.x`, `Call.u.x` and `Call.x`, each under its
/// object field; `self.t.x >= 1 ms` in a precondition is a guard on `t.x`;
/// `self.t = Timer { x: 0 ms, n: 3 }` in a postcondition resets `t.x` to
/// 0 and keeps the write of `n`; `self.t.n = 4` and `self.t.n > 1` are
/// data reads and writes; `self.u = self.t` refuses
/// `clock-read-outside-constraint` at `self.t`, and a `Timer` literal in a
/// `let` refuses `clock-write` at the literal. The checked `Timer` record
/// lists `n` only. The three objects' clocks are `qsl-eval`'s. Not
/// covered: the tuple field `p: (Clock, Int)`, which a package cannot
/// declare (STD-493 item 16).
#[trace("TC-685")]
#[test]
fn a_package_record_holds_its_clock_as_a_place_of_its_object() {
    let document = timer_document(|_| {});
    let body = format!(
        "{UNITS}{DENSE}pre P using v on Config::Call::reply {{ self.t.x >= 1 ms }}\n\
         post Q using v on Config::Call::send {{ self.t = Config::Timer {{ x: 0 ms, n: 3 }} }}\n\
         post N using v on Config::Call::bump {{ self.t.n = 4 }}\n\
         pre D using v on Config::Call::bump {{ self.t.n > 1 }}\n"
    );
    let graph = check(&document, &body).expect("the record's clock is a place");
    let model = timed(&graph);
    let places: Vec<(&str, &[PlaceStep], &str)> = model
        .clocks()
        .iter()
        .map(|clock| (clock.name(), clock.path(), clock.field().node.as_str()))
        .collect();
    let t = package("Call/t");
    let u = package("Call/u");
    let x = package("Call/x");
    assert_eq!(
        places,
        [
            (
                "t.x",
                &[
                    PlaceStep::Field("t".to_owned()),
                    PlaceStep::Field("x".to_owned())
                ][..],
                t.as_str()
            ),
            (
                "u.x",
                &[
                    PlaceStep::Field("u".to_owned()),
                    PlaceStep::Field("x".to_owned())
                ][..],
                u.as_str()
            ),
            ("x", &[PlaceStep::Field("x".to_owned())][..], x.as_str()),
        ]
    );
    assert_eq!(
        shown_parts(model, 0),
        [(
            None,
            vec![("t.x".to_owned(), ClockOp::GreaterOrEqual, exact(1, 1))]
        )]
    );
    let [resets] = model.resets() else {
        panic!("one postcondition resets: {:?}", model.resets());
    };
    let reset: Vec<_> = resets
        .resets()
        .iter()
        .map(|reset| {
            (
                model.clock(reset.clock()).expect("a clock").name(),
                reset.value().clone(),
            )
        })
        .collect();
    assert_eq!(reset, [("t.x", exact(0, 1))]);
    // The data component `n: 3` stays a write of the postcondition: the
    // literal `3` is in the checked body, the clock constant `0 ms` is not.
    let mut literals: Vec<String> = Vec::new();
    let mut pending = vec![graph.state_clause("Q").expect("Q").body()];
    while let Some(node) = pending.pop() {
        if matches!(node.kind(), qsl_semantics::check::NodeKind::Literal(_)) {
            literals.push(crate::timed_model::shown(node));
        }
        pending.extend(node.children());
    }
    assert_eq!(literals, ["3"]);
    let timer = graph
        .scope()
        .types()
        .composites()
        .find(|composite| composite.name().ends_with("Timer"))
        .expect("the checked Timer record");
    let CompositeShape::Record(fields) = timer.shape() else {
        panic!("Timer is a record: {timer:?}");
    };
    let names: Vec<&str> = fields.iter().map(|field| field.name()).collect();
    assert_eq!(names, ["n"], "Timer lists its data field only");

    for (clause, code, tag, at) in [
        (
            "post Q using v on Config::Call::send { self.u = self.t }",
            Code::InvalidTimedModel,
            "clock-read-outside-constraint",
            "self.t",
        ),
        (
            "post Q using v on Config::Call::send { \
             let r = Config::Timer { x: 0 ms, n: 1 } in self.t.n = r.n }",
            Code::InvalidTimedModel,
            "clock-write",
            "Config::Timer { x: 0 ms, n: 1 }",
        ),
    ] {
        let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{clause}\n"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (code, tag, at),
            "{clause}"
        );
    }
}

/// Remove `Call`'s own clock field `x` from `document` and from every frame,
/// so its only clocks are the records'.
fn without_own_clock(document: &mut Value) {
    let x = json!(call("x"));
    let call_type = &mut document["types"][1];
    call_type["fields"]
        .as_array_mut()
        .expect("Call fields")
        .retain(|field| field["name"] != "x");
    for operation in call_type["operations"].as_array_mut().expect("operations") {
        operation["frame"]["modifies"]
            .as_array_mut()
            .expect("frame")
            .retain(|field| field != &x);
    }
}

/// Part of FR-230-AC-14 (TC-685 step 16): a `Call` field `t: Timer` whose
/// record holds a clock refuses `invalid_timed_model`/
/// `clock-without-dense-time` under a `time tick` source, at the time
/// declaration, and in a model with no `time` member, at the model's
/// declaration. `ts: Sequence<Timer>` is
/// `a_record_clock_under_an_optional_or_collection_field`'s. Not covered:
/// the field's own span, which a package field does not carry (STD-493).
#[trace("TC-685")]
#[test]
fn a_record_clock_without_dense_time_refuses() {
    let document = timer_document(without_own_clock);
    let tick = "time Config tick Config::Call::bump period 1 ms unit ms;";
    let (cause, text) = refused(&document, &format!("{UNITS}{tick}\n"));
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (Code::InvalidTimedModel, "clock-without-dense-time", tick)
    );
    let (cause, text) = refused(&document, UNITS);
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::InvalidTimedModel, "clock-without-dense-time")
    );
    assert!(text.starts_with("model Config"), "{text}");
}

/// FR-230 "Parameters" (TC-685): a place path from a record parameter is a
/// place expression. `function inner(r: Config::Timer): Clock { r.x }`
/// called as `inner(self.t) >= 1 ms` constrains `t.x`; `predicate fresh(r:
/// Config::Timer): Boolean { r.n > 0 and r.x <= 3 ms }` called with
/// `self.u` reads `self.u.n > 0` and constrains `u.x`; an argument that is
/// no place of the record type refuses `ill_typed`/`type-mismatch` at it,
/// and the record read whole in a body refuses
/// `clock-read-outside-constraint`.
#[trace("TC-685")]
#[test]
fn a_record_parameter_reads_its_clock_by_path() {
    let document = timer_document(|_| {});
    let declarations = "function inner using v(r: Config::Timer): Clock pure { r.x }\n\
         predicate fresh using v(r: Config::Timer): Boolean { r.n > 0 and r.x <= 3 ms }\n";
    let body = format!(
        "{UNITS}{DENSE}{declarations}\
         pre P using v on Config::Call::reply {{ inner(self.t) >= 1 ms }}\n\
         pre F using v on Config::Call::bump {{ fresh(self.u) }}\n"
    );
    let graph = check(&document, &body).expect("the record parameter's paths are places");
    let model = timed(&graph);
    assert_eq!(
        shown_parts(model, 0),
        [(
            None,
            vec![("t.x".to_owned(), ClockOp::GreaterOrEqual, exact(1, 1))]
        )]
    );
    assert_eq!(
        shown_parts(model, 1),
        [(
            Some("(u.n Greater 0)".to_owned()),
            vec![("u.x".to_owned(), ClockOp::LessOrEqual, exact(3, 1))]
        )]
    );
    for (clause, tag, at) in [
        (
            "pre P using v on Config::Call::reply { inner(self.n) >= 1 ms }",
            "type-mismatch",
            "self.n",
        ),
        (
            "predicate same using v(r: Config::Timer): Boolean { r = r }",
            "clock-read-outside-constraint",
            "r",
        ),
    ] {
        let (cause, text) = refused(
            &document,
            &format!("{UNITS}{DENSE}{declarations}{clause}\n"),
        );
        assert_eq!((cause.tag(), text.as_str()), (tag, at), "{clause}");
    }
}

/// Part of FR-230-AC-14 (TC-685 step 16) and FR-230 "Records, tuples and
/// unions": a `Timer` held by an optional field (`o?: Timer`) under a dense
/// source is a guarded place S3 does not check yet (STD-493 item 17),
/// refused at the time declaration; under a tick source and with no time
/// source a collection field (`ts: Timer` of multiplicity `[0, 3]`, the
/// package form of `ts: Sequence<Timer>`) refuses
/// `clock-without-dense-time` as a single field does.
#[trace("TC-685")]
#[test]
fn a_record_clock_under_an_optional_or_collection_field() {
    let with = |name: &'static str, presence: &'static str, upper: u64| {
        timer_document(move |document: &mut Value| {
            let mut held = field(name, &package("Timer"));
            held["presence"] = json!(presence);
            held["multiplicity"]["lower"] = json!(if upper == 1 { 1 } else { 0 });
            held["multiplicity"]["upper"] = json!(upper);
            document["types"][1]["fields"]
                .as_array_mut()
                .expect("Call fields")
                .push(held);
        })
    };
    let refusals =
        check(&with("o", "optional", 1), &format!("{UNITS}{DENSE}")).expect_err("not checked");
    assert!(
        matches!(
            refusals.as_slice(),
            [qsl_semantics::check::CheckRefusal {
                cause: qsl_semantics::check::CheckCause::TimedNotChecked {
                    construct: qsl_semantics::check::TimedConstruct::Clock,
                    ..
                },
                ..
            }]
        ),
        "{refusals:?}"
    );
    let collection = {
        let mut document: Value = serde_json::from_slice(&with("ts", "required", 3)).expect("JSON");
        without_own_clock(&mut document);
        serde_json::to_vec(&document).expect("JSON")
    };
    let tick = "time Config tick Config::Call::bump period 1 ms unit ms;";
    let (cause, text) = refused(&collection, &format!("{UNITS}{tick}\n"));
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (Code::InvalidTimedModel, "clock-without-dense-time", tick)
    );
    let (cause, _) = refused(&collection, UNITS);
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::InvalidTimedModel, "clock-without-dense-time")
    );
}

/// FR-230 "Clock constraints and guards" and "Records, tuples and unions"
/// (TC-685): `self.peer.x`, a clock of another object read through a
/// reference field, is a clock read no constraint at a call site takes:
/// `clock-read-outside-constraint` at the read, in a guard and in a write.
/// A `Timer` literal that leaves out its clock component writes `t.x` no
/// constant: `clock-write` at the literal.
#[trace("TC-685")]
#[test]
fn a_referenced_clock_or_an_omitted_component_refuses() {
    let document = timer_document(|document: &mut Value| {
        document["types"][1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(field("peer", &package("Call")));
    });
    for (clause, tag, at) in [
        (
            "pre P using v on Config::Call::reply { self.peer.x >= 1 ms }",
            "clock-read-outside-constraint",
            "self.peer.x",
        ),
        (
            "post Q using v on Config::Call::send { self.t.n = 1 and self.peer.x = self.peer.x }",
            "clock-read-outside-constraint",
            "self.peer.x",
        ),
        (
            "post Q using v on Config::Call::send { self.t = Config::Timer { n: 3 } }",
            "clock-write",
            "Config::Timer { n: 3 }",
        ),
    ] {
        let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{clause}\n"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::InvalidTimedModel, tag, at),
            "{clause}"
        );
    }
}

/// Part of FR-230-AC-10 (TC-685 step 12): with `peer: Reference<Call>` (a
/// required field typed by the object type `Call`), `function
/// peer_clock(o: Call): Clock { o.peer.x }` called as `peer_clock(self) <=
/// 2 ms` checks to one convex part constraining the clock `x` of the object
/// `self.peer` refers to, reached through the field `peer`; each state's
/// `self.peer` selects that object (`qsl-eval`'s
/// `a_peer_clock_is_the_clock_of_the_referenced_object` reads the parts
/// over a universe). `own_clock(self.peer)` of `function own_clock(o:
/// Call): Clock { o.x }` reads the same place.
#[trace("TC-685")]
#[test]
fn a_clock_through_a_reference_field_is_the_referenced_objects() {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(field("peer", &package("Call")));
    let document = serde_json::to_vec(&document).expect("JSON");
    let declaration = "function peer_clock using v(o: Config::Call): Clock pure { o.peer.x }\n";
    let body = format!(
        "{UNITS}{DENSE}{declaration}\
         pre Near using v on Config::Call::reply {{ peer_clock(self) <= 2 ms }}\n"
    );
    let graph = check(&document, &body).expect("the peer's clock is a place");
    let model = timed(&graph);
    let [part] = model.guards()[0].parts() else {
        panic!("one convex part: {:?}", model.guards());
    };
    assert!(part.data().is_none());
    let [constraint] = part.clocks() else {
        panic!("one clock constraint: {:?}", part.clocks());
    };
    assert_eq!(
        (
            model.clock(constraint.clock()).expect("a clock").name(),
            constraint.via(),
            constraint.op(),
            constraint.bound().clone()
        ),
        (
            "x",
            &[PlaceStep::Field("peer".to_owned())][..],
            ClockOp::LessOrEqual,
            exact(2, 1)
        )
    );
    // Passed the referenced object itself, the body's own clock is the
    // peer's.
    let own = "function own_clock using v(o: Config::Call): Clock pure { o.x }\n";
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}{own}\
             pre Near using v on Config::Call::reply {{ own_clock(self.peer) <= 2 ms }}\n"
        ),
    )
    .expect("`self.peer` is an object argument");
    let model = timed(&graph);
    let vias: Vec<&[PlaceStep]> = model.guards()[0].parts()[0]
        .clocks()
        .iter()
        .map(|constraint| constraint.via())
        .collect();
    assert_eq!(vias, [&[PlaceStep::Field("peer".to_owned())][..]]);
}

/// Part of FR-230-AC-10 (TC-685), SR-1651 GA FND-002: `not (peer_clock(self)
/// <= 2 ms)` checks to the negated constraint `x > 2` of the object
/// `self.peer` refers to, its reference field carried into the negation;
/// a clock read through an optional reference (`value(o.backup).x`,
/// `backup?: Call`) or a multi-valued one (`o.peers.x`, `peers: Call [0,
/// 2]`) is no place: the walk stops at the field, and the read has no
/// data type (`ill_typed`/`type-mismatch` at the read).
#[trace("TC-685")]
#[test]
fn a_negated_peer_clock_keeps_its_reference_and_other_references_refuse() {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    let fields = document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields");
    fields.push(field("peer", &package("Call")));
    let mut backup = field("backup", &package("Call"));
    backup["presence"] = json!("optional");
    fields.push(backup);
    let mut peers = field("peers", &package("Call"));
    peers["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": false, "unique": true});
    fields.push(peers);
    let document = serde_json::to_vec(&document).expect("JSON");
    let declaration = "function peer_clock using v(o: Config::Call): Clock pure { o.peer.x }\n";
    let body = format!(
        "{UNITS}{DENSE}{declaration}\
         pre Far using v on Config::Call::reply {{ not (peer_clock(self) <= 2 ms) }}\n"
    );
    let graph = check(&document, &body).expect("a negated peer clock checks");
    let model = timed(&graph);
    let constraints: Vec<_> = model.guards()[0]
        .parts()
        .iter()
        .flat_map(|part| part.clocks())
        .map(|constraint| {
            (
                constraint.via(),
                constraint.op(),
                constraint.bound().clone(),
            )
        })
        .collect();
    assert_eq!(
        constraints,
        [(
            &[PlaceStep::Field("peer".to_owned())][..],
            ClockOp::Greater,
            exact(2, 1)
        )]
    );
    for reader in ["value(o.backup).x", "o.peers.x"] {
        let body = format!(
            "{UNITS}{DENSE}function other_clock using v(o: Config::Call): Clock pure {{ {reader} }}\n\
             pre Near using v on Config::Call::reply {{ other_clock(self) <= 2 ms }}\n"
        );
        let (cause, text) = refused(&document, &body);
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::IllTyped, "type-mismatch", reader),
        );
    }
}

/// A [`timer_document`] whose `Call` also holds `ts: Timer` of
/// multiplicity `[0, upper]` (`None`: unbounded), `ordered` and `unique` as
/// given: the package form of `Sequence<Timer>`, `OrderedSet<Timer>`,
/// `Set<Timer>` or `Bag<Timer>`.
fn timer_collection(upper: Option<u64>, ordered: bool, unique: bool) -> Vec<u8> {
    timer_document(move |document: &mut Value| {
        let mut held = field("ts", &package("Timer"));
        held["multiplicity"] = json!({"lower": 0, "ordered": ordered, "unique": unique});
        if let Some(upper) = upper {
            held["multiplicity"]["upper"] = json!(upper);
        }
        document["types"][1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(held);
        // `send` and `bump` also write `ts`.
        for operation in document["types"][1]["operations"]
            .as_array_mut()
            .expect("operations")
        {
            if matches!(operation["name"].as_str(), Some("send" | "bump")) {
                operation["frame"]["modifies"]
                    .as_array_mut()
                    .expect("frame")
                    .push(json!(call("ts")));
            }
        }
    })
}

/// Part of FR-230-AC-19 and FR-230-AC-23 (TC-685), the record parts: a
/// `Timer` collection field `ts` under a dense source checks; its clock is
/// the place `ts.[slot].x`, its path the field `ts`, a `Slot` step of the
/// field's shape, then `x`, under the object field `ts`. The static clock
/// count of the place is the declared maximum, `3` for `[0, 3]`, and none
/// for `[0, *]`; the static clock dimension of one `Call` is `1 + 1 + 1`
/// (`t.x`, `u.x`, `x`) plus `3`, or plus nothing when `ts` is unbounded. Not covered: a `Map<Text, Timer>` field and its key rule,
/// which the domain package cannot declare (it has no map type), and the
/// `Clock`-element collections (IR-1036).
#[trace("TC-685")]
#[test]
fn a_record_collection_holds_one_clock_place_per_slot() {
    for (upper, ordered, unique) in [
        (Some(3), true, false),
        (Some(3), true, true),
        (Some(3), false, true),
        (Some(3), false, false),
        (None, true, false),
    ] {
        let document = timer_collection(upper, ordered, unique);
        let graph = check(&document, &format!("{UNITS}{DENSE}")).expect("a collection checks");
        let model = timed(&graph);
        let place = model
            .clocks()
            .iter()
            .find(|clock| clock.name() == "ts.[slot].x")
            .unwrap_or_else(|| panic!("the slot place: {:?}", model.clocks()));
        assert_eq!(
            place.path(),
            [
                PlaceStep::Field("ts".to_owned()),
                PlaceStep::Slot(CollectionShape {
                    ordered,
                    unique,
                    min: 0,
                    max: upper,
                }),
                PlaceStep::Field("x".to_owned()),
            ]
        );
        assert_eq!(place.field().node, package("Call/ts"));
        assert_eq!(place.bound(), upper);
        assert_eq!(place.slots().count(), 1);
        let single = model
            .clocks()
            .iter()
            .find(|clock| clock.name() == "t.x")
            .expect("t.x");
        assert_eq!(single.bound(), Some(1));
        // `t.x`, `u.x` and `x`, one each, and the slots of `ts`.
        assert_eq!(model.dimension(place.owner()), Some(3 + upper.unwrap_or(0)));
    }
}

/// Part of FR-230-AC-21 (TC-685), the record parts of "Element identity":
/// over a `Timer` collection `ts`, `size(self.ts)`, a data field of an
/// element read through a traversal (`size(filter(e in self.ts: e.n > 1))`)
/// are data reads and check; an element's clock read in a traversal body
/// (`filter` and `count` over `e.x`), an element read whole (`e`), `=`
/// and `!=` over the collection or over `filter` results, `contains` and
/// `convert` compare or read clocks and refuse
/// `clock-read-outside-constraint` at the read.
#[trace("TC-685")]
#[test]
fn a_record_collection_reads_its_data_and_refuses_value_comparison() {
    let document = timer_collection(Some(3), true, false);
    let body = format!(
        "{UNITS}{DENSE}pre D using v on Config::Call::bump {{ size(self.ts) < 3 }}\n\
         pre E using v on Config::Call::reply {{ size(filter(e in self.ts: e.n > 1)) = 0 }}\n"
    );
    check(&document, &body).expect("data reads of a clock collection check");
    for (clause, at) in [
        (
            "pre E using v on Config::Call::reply { size(filter(e in self.ts: e.x >= 1 ms)) = 0 }",
            "e.x",
        ),
        (
            "pre E using v on Config::Call::reply { count<Config::VersionNumber>(e in self.ts: e.x >= 1 ms) = 0 }",
            "e.x",
        ),
        (
            "pre E using v on Config::Call::reply { exists(e in self.ts: e = self.t) }",
            "e",
        ),
        (
            "pre E using v on Config::Call::reply { self.ts = self.ts }",
            "self.ts",
        ),
        (
            "pre E using v on Config::Call::reply { contains(self.ts, self.t) }",
            "self.ts",
        ),
        (
            "pre E using v on Config::Call::reply { self.ts != self.ts }",
            "self.ts",
        ),
        (
            "pre E using v on Config::Call::reply { filter(e in self.ts: e.n > 1) = filter(e in self.ts: e.n > 2) }",
            "filter(e in self.ts: e.n > 1)",
        ),
        (
            "pre E using v on Config::Call::reply { size(convert<Set<Config::Timer>[0, 3]>(self.ts)) > 0 }",
            // The conversion's target, a collection of the clock-bearing
            // record, is where the comparison of elements is named.
            "Set<Config::Timer>[0, 3]",
        ),
    ] {
        let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{clause}\n"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::InvalidTimedModel, "clock-read-outside-constraint", at),
            "{clause}"
        );
    }
}

/// The index of a held slot.
fn held(slot: SlotRef) -> u64 {
    match slot {
        SlotRef::Held(slot) => slot,
        SlotRef::Family => panic!("a held slot expected"),
    }
}

/// The guard parts of the first guard of `model`: for each, the elements
/// its data predicate reads (field, slot) and its constraints (clock name,
/// slots, op, bound).
#[allow(clippy::type_complexity)]
fn slot_parts(
    model: &qsl_semantics::check::TimedModel,
) -> Vec<(
    bool,
    Vec<(String, u64)>,
    Vec<(String, Vec<u64>, ClockOp, String)>,
)> {
    model.guards()[0]
        .parts()
        .iter()
        .map(|part| {
            (
                part.data().is_some(),
                part.elements()
                    .iter()
                    .map(|element| (element.path().join("."), held(element.slot())))
                    .collect(),
                part.clocks()
                    .iter()
                    .map(|constraint| {
                        (
                            model
                                .clock(constraint.clock())
                                .expect("a clock")
                                .name()
                                .to_owned(),
                            constraint.slots().iter().copied().map(held).collect(),
                            constraint.op(),
                            format!("{:?}", constraint.bound()),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

/// A family's template part: the slots of the elements it reads, and each
/// constraint's slots and operator.
type Template = (Vec<SlotRef>, Vec<(Vec<SlotRef>, ClockOp)>);

/// Part of FR-230-AC-21 (TC-685), "Quantified reads" in a precondition:
/// over `ts: Sequence<Timer>[0, 3]`, the guard `forall(e in self.ts: e.x <=
/// 3 ms)` records one entry for each size 0 to 3, each with the data
/// predicate `size(self.ts) = s` and `x <= 3` on each slot below `s` (none
/// for size 0); `exists(e in self.ts: e.n = 2 and e.x >= 1 ms)` records one
/// convex part for each slot `j`, with the data predicate "holds slot `j`"
/// and `n = 2` of the element at `j`, and `x >= 1` on slot `j`. Over a
/// collection with no declared maximum the guard is one part conjoining the
/// family of the entries indexed by size: the body read once at the
/// template slot, every slot enabling it for a `forall` (`each`), some slot
/// for an `exists` or a negated `forall` (SR-1659002 FND-001).
#[trace("TC-685")]
#[test]
fn a_quantified_guard_records_one_entry_per_size_or_slot() {
    let document = timer_collection(Some(3), true, false);
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}pre Near using v on Config::Call::reply {{ \
             forall(e in self.ts: e.x <= 3 ms) }}\n"
        ),
    )
    .expect("a forall guard checks");
    let model = timed(&graph);
    let bound = format!("{:?}", exact(3, 1));
    let x = |slot: u64| {
        (
            "ts.[slot].x".to_owned(),
            vec![slot],
            ClockOp::LessOrEqual,
            bound.clone(),
        )
    };
    assert_eq!(
        slot_parts(model),
        [
            (true, vec![], vec![]),
            (true, vec![], vec![x(0)]),
            (true, vec![], vec![x(0), x(1)]),
            (true, vec![], vec![x(0), x(1), x(2)]),
        ]
    );
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}pre Near using v on Config::Call::reply {{ \
             exists(e in self.ts: e.n = 2 and e.x >= 1 ms) }}\n"
        ),
    )
    .expect("an exists guard checks");
    let model = timed(&graph);
    let one = format!("{:?}", exact(1, 1));
    let at = |slot: u64| {
        (
            true,
            vec![("ts".to_owned(), slot)],
            vec![(
                "ts.[slot].x".to_owned(),
                vec![slot],
                ClockOp::GreaterOrEqual,
                one.clone(),
            )],
        )
    };
    assert_eq!(slot_parts(model), [at(0), at(1), at(2)]);
    let unbounded = timer_collection(None, true, false);
    let family = |guard: &str| {
        let graph = check(
            &unbounded,
            &format!("{UNITS}{DENSE}pre Near using v on Config::Call::reply {{ {guard} }}\n"),
        )
        .unwrap_or_else(|refusals| panic!("{guard}: {refusals:?}"));
        let model = timed(&graph);
        let [part] = model.guards()[0].parts() else {
            panic!("one part: {:?}", model.guards()[0].parts());
        };
        let [family] = part.families() else {
            panic!("one family: {part:?}");
        };
        let templates: Vec<Template> = family
            .parts()
            .iter()
            .map(|template| {
                (
                    template
                        .elements()
                        .iter()
                        .map(|element| element.slot())
                        .collect(),
                    template
                        .clocks()
                        .iter()
                        .map(|constraint| (constraint.slots().to_vec(), constraint.op()))
                        .collect(),
                )
            })
            .collect();
        (family.path().join("."), family.each(), templates)
    };
    let open = SlotRef::Family;
    assert_eq!(
        family("forall(e in self.ts: e.x <= 3 ms)"),
        (
            "ts".to_owned(),
            true,
            vec![(vec![], vec![(vec![open], ClockOp::LessOrEqual)])]
        )
    );
    assert_eq!(
        family("not forall(e in self.ts: e.x <= 3 ms)"),
        (
            "ts".to_owned(),
            false,
            vec![(vec![], vec![(vec![open], ClockOp::Greater)])]
        )
    );
    assert_eq!(
        family("exists(e in self.ts: e.n = 2 and e.x >= 1 ms)"),
        (
            "ts".to_owned(),
            false,
            vec![(vec![open], vec![(vec![open], ClockOp::GreaterOrEqual)])]
        )
    );
}

/// Part of FR-230-AC-21 (TC-685), "Resets by selection": `forall(e in
/// self.ts: e.n > 1 implies e.x = 0 ms)` in `send`'s postcondition, whose
/// frame names `ts`, records one reset of `ts.[slot].x` to 0 with the
/// selection's data predicate `e.n > 1`; `forall(e in self.ts: e.x = 2 ms)`
/// records one with no predicate, every held slot. An `exists` reset, a
/// premise reading a clock and a reset under `reply`, whose frame does not
/// name `ts`, refuse.
#[trace("TC-685")]
#[test]
fn a_forall_reset_selects_the_slots_it_writes() {
    let document = timer_collection(Some(3), true, false);
    for (clause, value, predicate) in [
        (
            "forall(e in self.ts: e.n > 1 implies e.x = 0 ms)",
            exact(0, 1),
            true,
        ),
        ("forall(e in self.ts: e.x = 2 ms)", exact(2, 1), false),
    ] {
        let graph = check(
            &document,
            &format!("{UNITS}{DENSE}post R using v on Config::Call::send {{ {clause} }}\n"),
        )
        .unwrap_or_else(|refusals| panic!("{clause}: {refusals:?}"));
        let model = timed(&graph);
        let [resets] = model.resets() else {
            panic!("one postcondition resets: {:?}", model.resets());
        };
        let [reset] = resets.resets() else {
            panic!("one reset: {:?}", resets.resets());
        };
        let selection = reset.selection().expect("a reset by selection");
        assert_eq!(
            (
                model.clock(reset.clock()).expect("a clock").name(),
                reset.value().clone(),
                selection.path().join("."),
                selection.predicate().is_some(),
            ),
            ("ts.[slot].x", value, "ts".to_owned(), predicate),
            "{clause}"
        );
    }
    for (clause, tag, at) in [
        (
            "post R using v on Config::Call::send { exists(e in self.ts: e.x = 0 ms) }",
            "clock-write",
            "exists(e in self.ts: e.x = 0 ms)",
        ),
        (
            "post R using v on Config::Call::send { forall(e in self.ts: e.x > 1 ms implies e.x = 0 ms) }",
            "clock-read-outside-constraint",
            "e.x",
        ),
        (
            "post R using v on Config::Call::reply { forall(e in self.ts: e.x = 0 ms) }",
            "clock-write",
            "forall(e in self.ts: e.x = 0 ms)",
        ),
    ] {
        let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{clause}\n"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::InvalidTimedModel, tag, at),
            "{clause}"
        );
    }
}

/// [`timer_collection`]`(Some(3), true, false)` whose `Call` also holds
/// `st: Timer` of multiplicity `[0, 2]`, unordered and unique: the package
/// form of `Set<Timer>`, written by `send`.
fn timer_sequence_and_set() -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(&timer_collection(Some(3), true, false)).expect("JSON");
    let mut held = field("st", &package("Timer"));
    held["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": false, "unique": true});
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(held);
    for operation in document["types"][1]["operations"]
        .as_array_mut()
        .expect("operations")
    {
        if operation["name"] == "send" {
            operation["frame"]["modifies"]
                .as_array_mut()
                .expect("frame")
                .push(json!(call("st")));
        }
    }
    serde_json::to_vec(&document).expect("JSON")
}

/// Part of FR-230-AC-20 (TC-685), "Collection-valued writes":
/// - `self.ts = filter(e in pre(self.ts): e.n > 1)` writes each surviving
///   element carried from its slot of the pre-state; a bare `self.ts` on
///   the right is recorded as read in the post-state (FR-104, STD-493 item
///   28);
/// - `self.ts = map(e in pre(self.ts): e)` carries the element its binder
///   `e` names;
/// - `self.ts = sequence[Timer { x: 0 ms, n: 9 }]` writes a fresh element
///   with `x = 0`, and the checked body keeps the data `9` and drops the
///   clock constant;
/// - `self.st = set[Timer { x: 0 ms, n: 1 }, Timer { x: 5 ms, n: 1 }]`
///   records both occurrences: which are equal is decided on their values
///   when the write executes (SR-1662001 FND-003).
///
/// These refuse:
/// - `clock-write` at the occurrence for a component reading a clock
///   (`Timer { x: self.x, n: 1 }`);
/// - `clock-write` when `ts` is written in an invariant, under a `let`,
///   or by `reply`, whose frame does not name `ts`;
/// - `conflicting-clock-write` at the later write when one postcondition
///   writes `ts` twice, or writes it and resets a clock of it.
///
/// Not covered: `Timer { x: self.ts[0].x, n: 1 }`, whose selection has no
/// S2 form (STD-493 item 24), and AC-20's `flatten(sequence[sequence[..],
/// self.ts])`: a collection literal under `flatten` has no expected type,
/// so the value typer refuses it `AmbiguousLiteral`.
#[trace("TC-685")]
#[test]
fn a_collection_write_records_the_provenance_of_each_occurrence() {
    use qsl_semantics::check::Written;

    let document = timer_sequence_and_set();
    let written = |clause: &str| -> (Vec<(String, Written)>, Vec<String>) {
        let graph = check(
            &document,
            &format!("{UNITS}{DENSE}post Q using v on Config::Call::send {{ {clause} }}\n"),
        )
        .unwrap_or_else(|refusals| panic!("{clause}: {refusals:?}"));
        let model = timed(&graph);
        let writes = model
            .resets()
            .iter()
            .flat_map(|resets| resets.writes())
            .map(|write| (write.field().to_owned(), write.written().clone()))
            .collect();
        let mut literals = Vec::new();
        let mut pending = vec![graph.state_clause("Q").expect("Q").body()];
        while let Some(node) = pending.pop() {
            if matches!(node.kind(), qsl_semantics::check::NodeKind::Literal(_)) {
                literals.push(crate::timed_model::shown(node));
            }
            pending.extend(node.children());
        }
        literals.sort();
        (writes, literals)
    };
    let base = check(&document, &format!("{UNITS}{DENSE}")).expect("the model checks");
    let clock = |name: &str| {
        timed(&base)
            .clock_fields()
            .find(|(_, clock)| clock.name() == name)
            .map(|(id, _)| id)
            .unwrap_or_else(|| panic!("{name}"))
    };
    let field = |pre: bool| Written::Field {
        field: "ts".to_owned(),
        pre,
    };
    assert_eq!(
        written("self.ts = filter(e in pre(self.ts): e.n > 1)").0,
        [("ts".to_owned(), Written::Filter(Box::new(field(true))))]
    );
    assert_eq!(
        written("self.ts = filter(e in self.ts: e.n > 1)").0,
        [("ts".to_owned(), Written::Filter(Box::new(field(false))))]
    );
    assert_eq!(
        written("self.ts = map(e in pre(self.ts): e)").0,
        [(
            "ts".to_owned(),
            Written::Map {
                source: Box::new(field(true)),
                body: Box::new(Written::Element("e".to_owned())),
            }
        )]
    );
    let (writes, literals) = written("self.ts = sequence[Config::Timer { x: 0 ms, n: 9 }]");
    assert_eq!(
        writes,
        [(
            "ts".to_owned(),
            Written::Literal {
                kind: quire_exact::CollectionKind::Sequence,
                occurrences: vec![Written::Fresh(vec![(clock("ts.[slot].x"), exact(0, 1))])],
            }
        )]
    );
    assert_eq!(literals, ["9"], "the data component stays a data write");
    assert_eq!(
        written("self.st = set[Config::Timer { x: 0 ms, n: 1 }, Config::Timer { x: 5 ms, n: 1 }]")
            .0,
        [(
            "st".to_owned(),
            Written::Literal {
                kind: quire_exact::CollectionKind::Set,
                occurrences: vec![
                    Written::Fresh(vec![(clock("st.[slot].x"), exact(0, 1))]),
                    Written::Fresh(vec![(clock("st.[slot].x"), exact(5, 1))]),
                ],
            }
        )]
    );
    for (clause, at) in [
        (
            "post Q using v on Config::Call::send { self.ts = sequence[Config::Timer { x: self.x, n: 1 }] }",
            "Config::Timer { x: self.x, n: 1 }",
        ),
        (
            "post Q using v on Config::Call::send { let s = self.ts in self.ts = s }",
            "self.ts = s",
        ),
        (
            "post Q using v on Config::Call::reply { self.ts = self.ts }",
            "self.ts = self.ts",
        ),
        (
            "invariant I using v on Config::Call at current { self.ts = self.ts }",
            "self.ts = self.ts",
        ),
    ] {
        let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{clause}\n"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::InvalidTimedModel, "clock-write", at),
            "{clause}"
        );
    }
    for (clause, at) in [
        (
            "self.ts = filter(e in pre(self.ts): e.n > 1) and self.ts = pre(self.ts)",
            "self.ts = pre(self.ts)",
        ),
        (
            "self.ts = pre(self.ts) and forall(e in self.ts: e.x = 0 ms)",
            "forall(e in self.ts: e.x = 0 ms)",
        ),
    ] {
        let (cause, text) = refused(
            &document,
            &format!("{UNITS}{DENSE}post Q using v on Config::Call::send {{ {clause} }}\n"),
        );
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::InvalidTimedModel, "conflicting-clock-write", at),
            "{clause}"
        );
    }
}

/// Part of FR-230-AC-19 (TC-685), the record part of a nested collection:
/// with a record `Holder { ts: Timer [0, 2] }` held by `Call.hs` of
/// multiplicity `[0, 3]`, the place `hs.[slot].ts.[slot].x` has two slot
/// steps and the static bound `3 * 2`.
#[trace("TC-685")]
#[test]
fn a_collection_in_a_collection_bounds_its_place_by_the_product() {
    let document = timer_document(|document: &mut Value| {
        let holder = package("Holder");
        let mut ts = wire_field(
            &format!("{holder}/ts"),
            "ts",
            &package("Timer"),
            "required",
            0,
        );
        ts["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
        let types = document["types"].as_array_mut().expect("types");
        types.push(wire_type(&holder, "value_record", json!({"fields": [ts]})));
        let mut hs = field("hs", &holder);
        hs["multiplicity"] = json!({"lower": 0, "upper": 3, "ordered": true, "unique": false});
        types[1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(hs);
    });
    let graph = check(&document, &format!("{UNITS}{DENSE}")).expect("a nested collection checks");
    let model = timed(&graph);
    let place = model
        .clocks()
        .iter()
        .find(|clock| clock.name() == "hs.[slot].ts.[slot].x")
        .unwrap_or_else(|| panic!("the nested place: {:?}", model.clocks()));
    assert_eq!(place.slots().count(), 2);
    assert_eq!(place.bound(), Some(6));
}

/// Part of FR-230-AC-23 and FR-230-AC-16 (TC-685), the sequence halves: a
/// package `sequence` type `Timers` over the record `Timer` types `Call.q`,
/// whose clock place is `q.[slot].x` under an unbounded sequence slot
/// (`max: None`, no static bound), and `Call.qs: Timers` of multiplicity
/// `[0, 2]` holds `qs.[slot].[slot].x`. A sequence type `Ticks` whose
/// `items` is `ix://quire/native/Clock` types `Call.ks` as the clock
/// collection `ks.[slot]` (FR-230-AC-16). A sequence type `Flags` whose
/// `items` is `ix://quire/native/Boolean` types `Call.ds` as a sequence of
/// `Boolean`. Not covered: the map halves, which quire-exact's value types
/// cannot hold yet (no map value type, IR-1046).
#[trace("TC-685")]
#[test]
fn a_package_sequence_type_is_an_unbounded_clock_collection() {
    let document = timer_document(|document: &mut Value| {
        let sequence = |name: &str, items: &str| {
            let mut node = wire_type(&package(name), "unused", json!({"items": items}));
            node["kind"] = json!("sequence");
            node
        };
        let types = document["types"].as_array_mut().expect("types");
        types.push(sequence("Timers", &package("Timer")));
        types.push(sequence("Flags", "ix://quire/native/Boolean"));
        types.push(sequence("Ticks", "ix://quire/native/Clock"));
        let mut qs = field("qs", &package("Timers"));
        qs["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
        let fields = types[1]["fields"].as_array_mut().expect("Call fields");
        fields.push(field("q", &package("Timers")));
        fields.push(qs);
        fields.push(field("ds", &package("Flags")));
        fields.push(field("ks", &package("Ticks")));
    });
    let graph = check(&document, &format!("{UNITS}{DENSE}"))
        .unwrap_or_else(|refusals| panic!("a sequence type checks: {refusals:?}"));
    let model = timed(&graph);
    let unbounded = PlaceStep::Slot(CollectionShape {
        ordered: true,
        unique: false,
        min: 0,
        max: None,
    });
    let q = model
        .clocks()
        .iter()
        .find(|clock| clock.name() == "q.[slot].x")
        .unwrap_or_else(|| panic!("q's place: {:?}", model.clocks()));
    assert_eq!(
        q.path(),
        [
            PlaceStep::Field("q".to_owned()),
            unbounded.clone(),
            PlaceStep::Field("x".to_owned()),
        ]
    );
    assert_eq!(q.bound(), None);
    let qs = model
        .clocks()
        .iter()
        .find(|clock| clock.name() == "qs.[slot].[slot].x")
        .unwrap_or_else(|| panic!("qs's place: {:?}", model.clocks()));
    assert_eq!(qs.slots().count(), 2);
    let ks = model
        .clocks()
        .iter()
        .find(|clock| clock.name() == "ks.[slot]")
        .unwrap_or_else(|| panic!("ks's place: {:?}", model.clocks()));
    assert_eq!(
        ks.path(),
        [PlaceStep::Field("ks".to_owned()), unbounded.clone()]
    );
    let ds = graph
        .scope()
        .types()
        .attribute(q.owner(), "ds")
        .expect("Call.ds")
        .field()
        .value_type()
        .clone();
    let quire_exact::ValueType::Collection(ds) = &ds else {
        panic!("ds is a collection: {ds:?}");
    };
    assert_eq!(ds.kind(), quire_exact::CollectionKind::Sequence);
    assert!(
        matches!(ds.element(), quire_exact::ValueType::Boolean),
        "{ds:?}"
    );
}

/// Part of FR-230-AC-16 and FR-230-AC-17 (TC-685), the alias halves: an
/// alias `Stopwatch` of `ix://quire/native/Clock` types `Call.s` as the
/// clock `s`; an alias `D` of the unit `ix://quire/unit/ms` types `Call.d`
/// as `Quantity<ms>`, and a sequence type `Ds` whose `items` is `D` types
/// `Call.ds` as a sequence of `Quantity<ms>`. Not covered here: the map
/// halves (quire-exact has no map value type); the parameter, result and
/// ghost-unit refusals are
/// [`a_unit_alias_or_sequence_types_each_position_and_refuses_a_ghost_unit`].
#[trace("TC-685")]
#[test]
fn an_alias_of_a_clock_or_a_unit_types_its_members() {
    let document = timer_document(|document: &mut Value| {
        let core = |name: &str, kind: &str, extra: Value| {
            let mut node = wire_type(&package(name), "unused", extra);
            node["kind"] = json!(kind);
            node
        };
        let types = document["types"].as_array_mut().expect("types");
        types.push(core(
            "Stopwatch",
            "alias",
            json!({"target": "ix://quire/native/Clock"}),
        ));
        types.push(core("D", "alias", json!({"target": "ix://quire/unit/ms"})));
        types.push(core("Ds", "sequence", json!({"items": package("D")})));
        let fields = types[1]["fields"].as_array_mut().expect("Call fields");
        fields.push(field("s", &package("Stopwatch")));
        fields.push(field("d", &package("D")));
        fields.push(field("ds", &package("Ds")));
    });
    let graph = check(&document, &format!("{UNITS}{DENSE}"))
        .unwrap_or_else(|refusals| panic!("the aliases check: {refusals:?}"));
    let model = timed(&graph);
    let s = model
        .clocks()
        .iter()
        .find(|clock| clock.name() == "s")
        .unwrap_or_else(|| panic!("s's place: {:?}", model.clocks()));
    let typed = |name: &str| {
        graph
            .scope()
            .types()
            .attribute(s.owner(), name)
            .unwrap_or_else(|| panic!("Call.{name}"))
            .field()
            .value_type()
            .clone()
    };
    assert!(
        matches!(typed("d"), quire_exact::ValueType::Quantity(_)),
        "{:?}",
        typed("d")
    );
    let ds = typed("ds");
    let quire_exact::ValueType::Collection(ds) = &ds else {
        panic!("ds is a collection: {ds:?}");
    };
    assert_eq!(ds.kind(), quire_exact::CollectionKind::Sequence);
    assert!(
        matches!(ds.element(), quire_exact::ValueType::Quantity(_)),
        "{ds:?}"
    );
}

/// Part of FR-230-AC-21 (TC-685), "Quantified reads" over a collection
/// below a record field (SR-1659002 FND-003): with a record `Holder { ts:
/// Timer [0, 2] }` held by `Call.h`, the guard `exists(e in self.h.ts: e.n
/// = 2 and e.x >= 1 ms)` records one part for each slot of `h.ts`, each
/// reading the element at the path `h.ts` and constraining the slot's
/// `h.ts.[slot].x`.
#[trace("TC-685")]
#[test]
fn a_quantified_guard_reads_a_collection_below_a_record_field() {
    let document = timer_document(|document: &mut Value| {
        let holder = package("Holder");
        let mut ts = wire_field(
            &format!("{holder}/ts"),
            "ts",
            &package("Timer"),
            "required",
            0,
        );
        ts["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
        let types = document["types"].as_array_mut().expect("types");
        types.push(wire_type(&holder, "value_record", json!({"fields": [ts]})));
        types[1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(field("h", &holder));
    });
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}pre Near using v on Config::Call::reply {{ \
             exists(e in self.h.ts: e.n = 2 and e.x >= 1 ms) }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("the guard checks: {refusals:?}"));
    let model = timed(&graph);
    let one = format!("{:?}", exact(1, 1));
    let at = |slot: u64| {
        (
            true,
            vec![("h.ts".to_owned(), slot)],
            vec![(
                "h.ts.[slot].x".to_owned(),
                vec![slot],
                ClockOp::GreaterOrEqual,
                one.clone(),
            )],
        )
    };
    assert_eq!(slot_parts(model), [at(0), at(1)]);
}

/// Part of FR-230-AC-21 (TC-685), "Quantified reads" in a clock-reading
/// declaration (SR-1659002 FND-003): `predicate near(ts: Sequence<Timer>[0,
/// 3])` with the body `forall(e in ts: e.x <= 3 ms)`, called as
/// `near(self.ts)` in a precondition, records the guard of the `forall`
/// over `self.ts` itself, one entry for each size 0 to 3; with the body
/// `exists(e in ts: e.n = 2 and e.x >= 1 ms)` it records one part for each
/// slot, reading the element at the slot.
#[trace("TC-685")]
#[test]
fn a_quantified_read_expands_in_a_clock_reading_declaration() {
    let document = timer_collection(Some(3), true, false);
    let read = |body: &str| {
        let graph = check(
            &document,
            &format!(
                "{UNITS}{DENSE}predicate near using v(ts: Sequence<Config::Timer>[0, 3]): \
                 Boolean {{ {body} }}\n\
                 pre Near using v on Config::Call::reply {{ near(self.ts) }}\n"
            ),
        )
        .unwrap_or_else(|refusals| panic!("{body}: {refusals:?}"));
        slot_parts(timed(&graph))
    };
    let bound = format!("{:?}", exact(3, 1));
    let x = |slot: u64| {
        (
            "ts.[slot].x".to_owned(),
            vec![slot],
            ClockOp::LessOrEqual,
            bound.clone(),
        )
    };
    assert_eq!(
        read("forall(e in ts: e.x <= 3 ms)"),
        [
            (true, vec![], vec![]),
            (true, vec![], vec![x(0)]),
            (true, vec![], vec![x(0), x(1)]),
            (true, vec![], vec![x(0), x(1), x(2)]),
        ]
    );
    let one = format!("{:?}", exact(1, 1));
    let at = |slot: u64| {
        (
            true,
            vec![("ts".to_owned(), slot)],
            vec![(
                "ts.[slot].x".to_owned(),
                vec![slot],
                ClockOp::GreaterOrEqual,
                one.clone(),
            )],
        )
    };
    assert_eq!(
        read("exists(e in ts: e.n = 2 and e.x >= 1 ms)"),
        [at(0), at(1), at(2)]
    );
}

/// A `constraints[]` entry of `owner` bounding `applies` with `keyword`
/// and `value`.
fn clock_constraint(owner: &str, keyword: &str, value: &str, applies: &str) -> Value {
    let identity = package(&format!("{owner}/{keyword}"));
    json!({
        "identity": identity.clone(),
        "keyword": keyword,
        "operands": {"value": value},
        "appliesTo": applies,
        "diagnosticCode": package("BOUND"),
        "origin": {"generated": {"generatorIdentity": identity.clone(),
            "generatorVersion": "1.0.0", "inputIdentities": [identity]}},
    })
}

/// Part of FR-230-AC-25 (TC-685): a constraint `max` `"3"` whose
/// `appliesTo` is the alias `Timeout` of `Clock` checks as the time
/// invariant `x <= 3` on the place typed `Timeout` (`w`) and on no other
/// clock; the same constraint on `Call` with `appliesTo`
/// `ix://quire/native/Clock` bounds every clock place of `Call` by 3; a
/// `min` `"1"` constraint refuses `invalid_timed_model`/
/// `time-invariant-shape`; `minLength` and a negative `max` operand refuse
/// `invalid_model_binding`/`malformed-declaration`. Not covered: the
/// presence guard of an `Option<Timeout>` place, which waits on
/// `Option<Clock>` places (580n), and the `time tick` and untimed
/// refusals.
#[trace("TC-685")]
#[test]
fn a_max_constraint_on_a_clock_alias_bounds_its_places() {
    let document = |constraints: Vec<Value>, on_call: Vec<Value>| {
        timer_document(move |document: &mut Value| {
            let mut timeout = wire_type(
                &package("Timeout"),
                "unused",
                json!({"target": "ix://quire/native/Clock", "constraints": constraints}),
            );
            timeout["kind"] = json!("alias");
            let types = document["types"].as_array_mut().expect("types");
            types.push(timeout);
            types[1]["fields"]
                .as_array_mut()
                .expect("Call fields")
                .push(field("w", &package("Timeout")));
            types[1]["constraints"] = json!(on_call);
        })
    };
    let bounded = |constraints, on_call| {
        let graph = check(&document(constraints, on_call), &format!("{UNITS}{DENSE}"))
            .unwrap_or_else(|refusals| panic!("the bound checks: {refusals:?}"));
        let model = timed(&graph);
        model
            .invariants()
            .iter()
            .map(|invariant| {
                let mut clocks: Vec<(String, String)> = invariant
                    .bounds()
                    .iter()
                    .map(|bound| {
                        (
                            model
                                .clock(bound.clock())
                                .expect("a clock")
                                .name()
                                .to_owned(),
                            format!("{:?} {:?}", bound.op(), bound.bound()),
                        )
                    })
                    .collect();
                clocks.sort();
                (invariant.name().to_owned(), clocks)
            })
            .collect::<Vec<_>>()
    };
    let at_most = format!("{:?} {:?}", ClockOp::LessOrEqual, exact(3, 1));
    assert_eq!(
        bounded(
            vec![clock_constraint("Timeout", "max", "3", &package("Timeout"))],
            vec![]
        ),
        [(
            package("Timeout/max"),
            vec![("w".to_owned(), at_most.clone())]
        )]
    );
    let every: Vec<(String, String)> = ["t.x", "u.x", "w", "x"]
        .into_iter()
        .map(|name| (name.to_owned(), at_most.clone()))
        .collect();
    assert_eq!(
        bounded(
            vec![],
            vec![clock_constraint(
                "Call",
                "max",
                "3",
                "ix://quire/native/Clock"
            )]
        ),
        [(package("Call/max"), every)]
    );
    let refusals = check(
        &document(
            vec![clock_constraint("Timeout", "min", "1", &package("Timeout"))],
            vec![],
        ),
        &format!("{UNITS}{DENSE}"),
    )
    .expect_err("a min bound is refused");
    assert!(
        matches!(
            refusals.as_slice(),
            [qsl_semantics::check::CheckRefusal {
                cause: qsl_semantics::check::CheckCause::Temporal(cause),
                ..
            }] if cause.code() == Code::InvalidTimedModel && cause.tag() == "time-invariant-shape"
        ),
        "{refusals:?}"
    );
    for (keyword, value) in [("minLength", "3"), ("max", "-1")] {
        let malformed = std::panic::catch_unwind(|| {
            check(
                &document(
                    vec![clock_constraint(
                        "Timeout",
                        keyword,
                        value,
                        &package("Timeout"),
                    )],
                    vec![],
                ),
                &format!("{UNITS}{DENSE}"),
            )
        });
        let Err(panicked) = malformed else {
            panic!("{keyword} {value}: intake refuses the constraint");
        };
        let message = panicked
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default();
        assert!(
            message.contains("InvalidModelBinding") && message.contains("constraints"),
            "{keyword} {value}: {message}"
        );
    }
}

/// Part of FR-230-AC-17 (TC-685): the unit `ms` named by its package path
/// `ix://quire/unit/<package>/ms` as the `target` of an alias `D` types the
/// field `Call.d`, the parameter `bump(p: D)` and the result of `bump` as
/// `Quantity<ms>`, and a sequence type `Ms` whose `items` is that unit
/// types `Call.ms` as a sequence of `Quantity<ms>`. The same alias `target`
/// or sequence `items` naming `ix://quire/unit/ghost` refuses
/// `missing_declaration`/`missing-name` naming the type definition and its
/// path `TypeDefinition.target` or `TypeDefinition.items`, once, whether
/// or not a member is typed by it. Not covered: the map half (IR-1046,
/// quire-exact has no map value type).
#[trace("TC-685")]
#[test]
fn a_unit_alias_or_sequence_types_each_position_and_refuses_a_ghost_unit() {
    let document = |unit: &str, used: bool| {
        let unit = unit.to_owned();
        timer_document(move |document: &mut Value| {
            let core = |name: &str, kind: &str, extra: Value| {
                let mut node = wire_type(&package(name), "unused", extra);
                node["kind"] = json!(kind);
                node
            };
            let types = document["types"].as_array_mut().expect("types");
            types.push(core("D", "alias", json!({"target": unit})));
            types.push(core("Ms", "sequence", json!({"items": unit})));
            if !used {
                return;
            }
            let fields = types[1]["fields"].as_array_mut().expect("Call fields");
            fields.push(field("d", &package("D")));
            fields.push(field("ms", &package("Ms")));
            for operation in types[1]["operations"].as_array_mut().expect("operations") {
                if operation["name"] == "bump" {
                    let mut parameter = wire_field(
                        &format!("{}/p", call("bump")),
                        "p",
                        &package("D"),
                        "required",
                        1,
                    );
                    parameter["multiplicity"]["unique"] = json!(true);
                    operation["params"] = json!([parameter]);
                    operation["returns"] = json!({
                        "typeRef": package("D"),
                        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                        "nullable": false,
                    });
                }
            }
        })
    };
    let graph = check(
        &document(&format!("ix://quire/unit/{PACKAGE}/ms"), true),
        &format!("{UNITS}{DENSE}"),
    )
    .unwrap_or_else(|refusals| panic!("the unit types check: {refusals:?}"));
    let model = timed(&graph);
    let owner = model
        .clocks()
        .iter()
        .find(|clock| clock.name() == "x")
        .expect("Call.x")
        .owner();
    let typed = |name: &str| {
        graph
            .scope()
            .types()
            .attribute(owner, name)
            .unwrap_or_else(|| panic!("Call.{name}"))
            .field()
            .value_type()
            .clone()
    };
    let quantity = typed("d");
    assert!(
        matches!(quantity, quire_exact::ValueType::Quantity(_)),
        "{quantity:?}"
    );
    let ms = typed("ms");
    let quire_exact::ValueType::Collection(ms) = &ms else {
        panic!("ms is a collection: {ms:?}");
    };
    assert_eq!(ms.kind(), quire_exact::CollectionKind::Sequence);
    assert_eq!(ms.element(), &quantity);
    let bump = graph
        .scope()
        .operations()
        .declared_by(owner)
        .iter()
        .find(|operation| operation.name() == "bump")
        .expect("bump");
    assert_eq!(bump.parameters(), &[("p".to_owned(), quantity.clone())]);
    assert_eq!(bump.result(), Some(&quantity));

    for used in [true, false] {
        let refusal = crate::model_operations::admit_and_assemble_with_profiles(
            &document("ix://quire/unit/ghost", used),
            crate::timed_model::TEMPORAL,
            &format!("{UNITS}{DENSE}"),
        )
        .expect_err("a ghost unit refuses");
        let mut named: Vec<(Code, String, String)> = refusal
            .errors
            .iter()
            .map(|error| {
                let qsl_semantics::check::AssemblyCause::UnresolvedModelQuantityType {
                    name,
                    member_identity,
                    member_path,
                } = &error.cause
                else {
                    panic!("an unresolved unit: {error:?}");
                };
                assert_eq!(name, "ghost");
                (
                    error.cause.code(),
                    member_identity.clone(),
                    member_path.clone(),
                )
            })
            .collect();
        named.sort_by(|left, right| (&left.1, &left.2).cmp(&(&right.1, &right.2)));
        assert_eq!(
            named,
            [
                (
                    Code::MissingDeclaration,
                    package("D"),
                    "TypeDefinition.target".to_owned()
                ),
                (
                    Code::MissingDeclaration,
                    package("Ms"),
                    "TypeDefinition.items".to_owned()
                ),
            ],
            "used: {used}"
        );
    }
}

/// Part of FR-230-AC-25 (TC-685), SR-1681002 GA FND-001 and SR-1681001 CR
/// FND-004: a `max` `"3"` with `appliesTo` `ix://quire/native/Clock` on an
/// object type with no clock field refuses
/// `invalid_timed_model`/`clock-without-dense-time` under `time tick` and
/// in an untimed unit, and checks under `time dense`; a keyword `format`
/// and an operand `"abc"` refuse `invalid_model_binding`/
/// `malformed-declaration` at the constraint; a `max` on the alias `Short`
/// of the alias `Timeout` of `Clock` bounds the place typed `Short` and not
/// the one typed `Timeout`, and one on `Timeout` bounds both.
#[trace("TC-685")]
#[test]
fn a_clock_bound_needs_dense_time_and_reaches_through_aliases() {
    let unclocked = |constraints: Vec<Value>| {
        let mut document: Value =
            serde_json::from_slice(&rpc_document_with(&[], false)).expect("JSON");
        document["types"][1]["constraints"] = json!(constraints);
        for operation in document["types"][1]["operations"]
            .as_array_mut()
            .expect("operations")
        {
            operation["frame"]["modifies"]
                .as_array_mut()
                .expect("a frame")
                .retain(|entry| entry != &json!(call("x")));
        }
        serde_json::to_vec(&document).expect("JSON")
    };
    let on_clock = || {
        vec![clock_constraint(
            "Call",
            "max",
            "3",
            "ix://quire/native/Clock",
        )]
    };
    let tick = "time Config tick Config::Call::bump period 1 ms unit ms;\n";
    for time in [tick, ""] {
        let (cause, _) = refused(&unclocked(on_clock()), &format!("{UNITS}{time}"));
        assert!(
            matches!(
                cause,
                qsl_semantics::check::TemporalCause::ClockWithoutDenseTime { .. }
            ),
            "{time:?}: {cause:?}"
        );
        assert_eq!(cause.tag(), "clock-without-dense-time");
    }
    check(&unclocked(on_clock()), &format!("{UNITS}{DENSE}"))
        .unwrap_or_else(|refusals| panic!("a dense bound checks: {refusals:?}"));
    for (keyword, value) in [("format", "3"), ("max", "abc")] {
        let malformed = std::panic::catch_unwind(|| {
            check(
                &unclocked(vec![clock_constraint(
                    "Call",
                    keyword,
                    value,
                    "ix://quire/native/Clock",
                )]),
                &format!("{UNITS}{DENSE}"),
            )
        });
        let Err(panicked) = malformed else {
            panic!("{keyword} {value}: intake refuses the constraint");
        };
        let message = panicked
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_default();
        assert!(
            message.contains("InvalidModelBinding") && message.contains("constraints"),
            "{keyword} {value}: {message}"
        );
    }

    let chained = |on_timeout: Vec<Value>, on_short: Vec<Value>| {
        timer_document(move |document: &mut Value| {
            let alias = |name: &str, target: &str, constraints: Vec<Value>| {
                let mut node = wire_type(
                    &package(name),
                    "unused",
                    json!({"target": target, "constraints": constraints}),
                );
                node["kind"] = json!("alias");
                node
            };
            let types = document["types"].as_array_mut().expect("types");
            types.push(alias("Timeout", "ix://quire/native/Clock", on_timeout));
            types.push(alias("Short", &package("Timeout"), on_short));
            let fields = types[1]["fields"].as_array_mut().expect("Call fields");
            fields.push(field("w", &package("Timeout")));
            fields.push(field("s", &package("Short")));
        })
    };
    let bounded = |document: Vec<u8>| {
        let graph = check(&document, &format!("{UNITS}{DENSE}"))
            .unwrap_or_else(|refusals| panic!("the bound checks: {refusals:?}"));
        let model = timed(&graph);
        let mut names: Vec<String> = model
            .invariants()
            .iter()
            .flat_map(|invariant| invariant.bounds())
            .map(|bound| {
                model
                    .clock(bound.clock())
                    .expect("a clock")
                    .name()
                    .to_owned()
            })
            .collect();
        names.sort();
        names
    };
    assert_eq!(
        bounded(chained(
            vec![],
            vec![clock_constraint("Short", "max", "3", &package("Short"))]
        )),
        ["s"]
    );
    assert_eq!(
        bounded(chained(
            vec![clock_constraint("Timeout", "max", "3", &package("Timeout"))],
            vec![]
        )),
        ["s", "w"]
    );
}

/// [`rpc_document_with`]`(&["x", "y"])` whose `bump` takes `c` typed
/// `param` and returns `returns`, its frame naming `frame`, with a clock
/// alias `Stopwatch`.
fn clock_parameter_document(param: &str, returns: Option<&str>, frame: &[&str]) -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x", "y"], false)).expect("JSON");
    let types = document["types"].as_array_mut().expect("types");
    let mut stopwatch = wire_type(
        &package("Stopwatch"),
        "unused",
        json!({"target": "ix://quire/native/Clock"}),
    );
    stopwatch["kind"] = json!("alias");
    types.push(stopwatch);
    for operation in types[1]["operations"].as_array_mut().expect("operations") {
        if operation["name"] != "bump" {
            continue;
        }
        let identity = call("bump/c");
        operation["params"] = json!([{
            "identity": identity, "name": "c", "typeRef": param, "presence": "required",
            "nullable": false, "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
            "origin": {"generated": {"generatorIdentity": identity, "generatorVersion": "1.0.0",
                "inputIdentities": [identity]}},
        }]);
        if let Some(returns) = returns {
            operation["returns"] = json!({"typeRef": returns, "nullable": false,
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false}});
        }
        operation["frame"]["modifies"] =
            json!(frame.iter().map(|field| call(field)).collect::<Vec<_>>());
    }
    serde_json::to_vec(&document).expect("JSON")
}

/// FR-230-AC-13 and FR-230-AC-16 (TC-685): a domain-package operation
/// `bump(c: Clock): Clock` checks, its `Clock` parameter and result no data
/// member of the checked operation. With the postcondition `c = 0 ms` and a
/// frame naming `Call.x` the parameter ranges over the places of `Call.x`
/// and the reset is recorded once, for `x`, tagged with the parameter; the
/// same postcondition with an empty frame refuses
/// `invalid_timed_model`/`clock-write` at the reset. With the precondition
/// `c >= 1 ms` and no reset it ranges over every clock field, `x` and `y`,
/// one constraint for each, each tagged with the parameter. A parameter
/// typed by the alias `Stopwatch` of `Clock` is a `Clock` parameter too, and
/// `c` read as data (`c = c`) is a clock comparison, never a data read.
#[trace("TC-685")]
#[test]
fn a_clock_parameter_ranges_over_the_places_it_may_write() {
    let clock = "ix://quire/native/Clock";
    let post = format!("{UNITS}{DENSE}post Bumped using v on Config::Call::bump {{ c = 0 ms }}\n");
    let graph = check(&clock_parameter_document(clock, Some(clock), &["x"]), &post)
        .unwrap_or_else(|refusals| panic!("bump(c: Clock): Clock checks: {refusals:?}"));
    let model = timed(&graph);
    let names = |clock: qsl_semantics::check::ClockId| {
        model.clock(clock).expect("a clock").name().to_owned()
    };
    let [range] = model.clock_parameters() else {
        panic!("one clock parameter: {:?}", model.clock_parameters());
    };
    assert_eq!(range.position(), 0);
    assert_eq!(
        range
            .range()
            .iter()
            .map(|clock| names(*clock))
            .collect::<Vec<_>>(),
        ["x"]
    );
    let resets: Vec<(String, Option<usize>)> = model
        .resets()
        .iter()
        .flat_map(|resets| resets.resets())
        .map(|reset| (names(reset.clock()), reset.parameter()))
        .collect();
    assert_eq!(resets, [("x".to_owned(), Some(0))]);

    let (cause, text) = refused(&clock_parameter_document(clock, None, &[]), &post);
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (Code::InvalidTimedModel, "clock-write", "c = 0 ms")
    );

    let pre = format!("{UNITS}{DENSE}pre Late using v on Config::Call::bump {{ c >= 1 ms }}\n");
    for param in [clock.to_owned(), package("Stopwatch")] {
        let graph = check(&clock_parameter_document(&param, None, &[]), &pre)
            .unwrap_or_else(|refusals| panic!("{param}: {refusals:?}"));
        let model = timed(&graph);
        let mut constrained: Vec<(String, Option<usize>)> = model
            .guards()
            .iter()
            .flat_map(|guard| guard.parts())
            .flat_map(|part| part.clocks())
            .map(|constraint| {
                (
                    model
                        .clock(constraint.clock())
                        .expect("a clock")
                        .name()
                        .to_owned(),
                    constraint.parameter(),
                )
            })
            .collect();
        constrained.sort();
        assert_eq!(
            constrained,
            [("x".to_owned(), Some(0)), ("y".to_owned(), Some(0))],
            "{param}"
        );
    }
    let (cause, _) = refused(
        &clock_parameter_document(clock, None, &[]),
        &format!("{UNITS}{DENSE}pre Same using v on Config::Call::bump {{ c = c }}\n"),
    );
    assert_eq!(cause.tag(), "clock-comparison");
}

/// [`rpc_document_with`]`(&["x"])` with the optional clock field `v`
/// (`Option<Clock>`), which `bump` writes.
fn option_clock_document() -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    let mut v = field("v", "ix://quire/native/Clock");
    v["presence"] = json!("optional");
    let call_type = &mut document["types"][1];
    call_type["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(v);
    for operation in call_type["operations"].as_array_mut().expect("operations") {
        if operation["name"] == "bump" {
            operation["frame"]["modifies"]
                .as_array_mut()
                .expect("frame")
                .push(json!(call("v")));
        }
    }
    serde_json::to_vec(&document).expect("JSON")
}

/// FR-230-AC-12 and FR-230-AC-39 (TC-685), S3's part: over `v:
/// Option<Clock>`, the place `v` holds a presence bit. The guard
/// `present(self.v) and value(self.v) >= 1 ms` checks to one convex part
/// `v >= 1` requiring `v` present, with no data predicate; `not
/// present(self.v)` is a part requiring it absent, never a clock
/// constraint. `self.v = 0 ms` records a reset of `v`, which makes it
/// present, and `when present(self.v) { value(self.v) <= 3 ms }` records
/// the bound with the presence it requires. `value(self.v) >= 1 ms`
/// without `present(self.v)`, in a guard or a time invariant, refuses
/// `undefined_expression`/`unproved-presence`.
#[trace("TC-685")]
#[test]
fn an_option_clock_reads_its_presence_bit() {
    let document = option_clock_document();
    let body = format!(
        "{UNITS}{DENSE}\
         pre Armed using v on Config::Call::bump {{ present(self.v) and value(self.v) >= 1 ms }}\n\
         pre Idle using v on Config::Call::reply {{ not present(self.v) }}\n\
         post Arm using v on Config::Call::bump {{ self.v = 0 ms }}\n\
         time invariant Bound when {{ present(self.v) }} {{ value(self.v) <= 3 ms }}\n"
    );
    let graph = check(&document, &body).expect("an Option<Clock> place checks");
    let model = timed(&graph);
    let (v, field) = model
        .clock_fields()
        .find(|(_, field)| field.name() == "v")
        .expect("v is a place");
    assert!(field.presence_bit());
    let parts: Vec<_> = model
        .guards()
        .iter()
        .map(|guard| {
            let [part] = guard.parts() else {
                panic!("one part: {:?}", guard.parts());
            };
            (
                part.presences().to_vec(),
                part.clocks()
                    .iter()
                    .map(|constraint| (constraint.clock(), constraint.op()))
                    .collect::<Vec<_>>(),
                part.data().is_none(),
            )
        })
        .collect();
    assert_eq!(
        parts,
        [
            (vec![(v, true)], vec![(v, ClockOp::GreaterOrEqual)], true),
            (vec![(v, false)], vec![], true),
        ]
    );
    let resets: Vec<_> = model
        .resets()
        .iter()
        .flat_map(|resets| resets.resets())
        .map(|reset| (reset.clock(), reset.value().clone()))
        .collect();
    assert_eq!(resets, [(v, exact(0, 1))]);
    let [invariant] = model.invariants() else {
        panic!("one invariant");
    };
    let [alternative] = invariant.alternatives() else {
        panic!("one alternative");
    };
    assert_eq!(alternative.presences(), [(v, true)]);
    assert_eq!(invariant.bounds()[0].clock(), v);

    // FR-230-AC-12: the refusal is located at the read.
    for (clause, read) in [
        (
            "pre Late using v on Config::Call::bump { value(self.v) >= 1 ms }\n",
            "value(self.v) >= 1 ms",
        ),
        (
            "time invariant Loose when { true } { value(self.v) <= 3 ms }\n",
            "value(self.v) <= 3 ms",
        ),
        (
            "time invariant Either when { present(self.v) or self.late } { value(self.v) <= 3 ms }\n",
            "value(self.v) <= 3 ms",
        ),
    ] {
        let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{clause}"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::UndefinedExpression, "unproved-presence", read),
            "{clause}"
        );
    }
}

/// The STD-493 ruling on FR-230 "Presence bit" (TC-685): `absent` is an
/// option literal typed by the `Option<T>` its context expects. It checks
/// as an equality operand on either side of an `Option<Integer>` and as a
/// function body whose result is `Option<Integer>`; where an `Integer` is
/// expected it refuses `ill_typed`/`type-mismatch`, and with no expected
/// type (`absent = absent`) it refuses `ill_typed` as an ambiguous literal.
#[trace("TC-685")]
#[test]
fn absent_takes_the_option_type_its_context_expects() {
    use qsl_semantics::check::CheckCause;
    use quire_exact::IllTypedCause;

    let document = rpc_document_with(&["x"], false);
    for function in [
        "function f using v(o: Option<Integer>): Boolean pure { o = absent }\n",
        "function f using v(o: Option<Integer>): Boolean pure { absent != o }\n",
        "function f using v(): Option<Integer> pure { absent }\n",
    ] {
        if let Err(refusals) = check(&document, &format!("{UNITS}{DENSE}{function}")) {
            panic!("{function}: {refusals:?}");
        }
    }
    for (function, expected) in [
        (
            "function f using v(): Integer pure { absent }\n",
            IllTypedCause::TypeMismatch,
        ),
        (
            "function f using v(): Boolean pure { absent = absent }\n",
            IllTypedCause::AmbiguousLiteral,
        ),
    ] {
        let refusals = check(&document, &format!("{UNITS}{DENSE}{function}"))
            .expect_err("absent needs an expected option type");
        assert!(
            refusals
                .iter()
                .any(|refusal| refusal.cause == CheckCause::IllTyped(expected)),
            "{function}: {refusals:?}"
        );
    }
}

/// FR-230-AC-12 (TC-685), the drop: `self.v = absent` over `v:
/// Option<Clock>` records a drop of `v` and no reset; with a frame not
/// naming `v`, the drop and a reset `self.v = 0 ms` each refuse
/// `invalid_timed_model`/`clock-write`.
#[trace("TC-685")]
#[test]
fn absent_drops_an_option_clock() {
    let document = option_clock_document();
    let body =
        format!("{UNITS}{DENSE}post Disarm using v on Config::Call::bump {{ self.v = absent }}\n");
    let graph = check(&document, &body).expect("the drop checks");
    let model = timed(&graph);
    let (v, _) = model
        .clock_fields()
        .find(|(_, field)| field.name() == "v")
        .expect("v is a place");
    let [resets] = model.resets() else {
        panic!("one postcondition");
    };
    assert_eq!(resets.drops(), [v]);
    assert!(resets.resets().is_empty());

    for write in ["self.v = absent", "self.v = 0 ms"] {
        let reply =
            format!("{UNITS}{DENSE}post Outside using v on Config::Call::reply {{ {write} }}\n");
        let (cause, text) = refused(&document, &reply);
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::InvalidTimedModel, "clock-write", write)
        );
    }
}

/// FR-230 "Presence bit" (TC-685), part of FR-230-AC-12: a postcondition
/// writes one `Option<Clock>` place at most once. A reset and a drop, in
/// either order, or two resets give the place no one presence and value,
/// and refuse `invalid_timed_model`/`conflicting-clock-write` at the second
/// write.
#[trace("TC-685")]
#[test]
fn two_writes_of_an_option_clock_conflict() {
    let document = option_clock_document();
    for (writes, second) in [
        ("self.v = 0 ms and self.v = absent", "self.v = absent"),
        ("self.v = absent and self.v = 0 ms", "self.v = 0 ms"),
        ("self.v = 0 ms and self.v = 1 ms", "self.v = 1 ms"),
    ] {
        let body =
            format!("{UNITS}{DENSE}post Both using v on Config::Call::bump {{ {writes} }}\n");
        let (cause, text) = refused(&document, &body);
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::InvalidTimedModel, "conflicting-clock-write", second),
            "{writes}"
        );
    }
}

/// FR-230-AC-39 (TC-685), S3's part: `present(self.v)` is a `Boolean` data
/// predicate over the presence bit beyond a guard's top-level conjunct.
/// `self.v != absent` and `self.v = absent` in a guard are its presence and
/// absence; `not present(self.v) or self.late` in a time invariant's `when`
/// is two alternatives, one requiring `v` absent and one reading `late`;
/// `urgent U when { present(self.v) and self.late }` requires `v` present.
/// A presence read this slice does not read yet (under `or` in a
/// postcondition, a quantifier body, another object's place) refuses
/// `unsupported_construct`/`not-yet-implemented` at the read, never as a
/// clock read. STD-526 (QSL FR-230-AC-51) rules the last two in; the
/// stacked QSL-580t slice builds them.
#[trace("TC-685")]
#[test]
fn present_is_a_data_predicate_beyond_a_guard_conjunct() {
    let document = option_clock_document();
    let body = format!(
        "{UNITS}{DENSE}\
         pre Armed using v on Config::Call::bump {{ self.v != absent and value(self.v) >= 1 ms }}\n\
         pre Idle using v on Config::Call::reply {{ self.v = absent }}\n\
         time invariant Idle when {{ not present(self.v) or self.late }} {{ self.x <= 5 ms }}\n\
         urgent Due when {{ present(self.v) and self.late }};\n"
    );
    let graph = check(&document, &body).expect("presence reads check");
    let model = timed(&graph);
    let (v, _) = model
        .clock_fields()
        .find(|(_, field)| field.name() == "v")
        .expect("v is a place");
    let guards: Vec<_> = model
        .guards()
        .iter()
        .map(|guard| {
            guard
                .parts()
                .iter()
                .map(|part| part.presences().to_vec())
                .collect::<Vec<_>>()
        })
        .collect();
    assert_eq!(guards, [vec![vec![(v, true)]], vec![vec![(v, false)]]]);
    let [invariant] = model.invariants() else {
        panic!("one invariant");
    };
    let alternatives: Vec<_> = invariant
        .alternatives()
        .iter()
        .map(|alternative| alternative.presences().to_vec())
        .collect();
    assert_eq!(alternatives, [vec![(v, false)], vec![]]);
    let [qsl_semantics::check::Urgency::When { alternatives, .. }] = model.urgency() else {
        panic!("one urgent when");
    };
    let urgent: Vec<_> = alternatives
        .iter()
        .map(|alternative| alternative.presences().to_vec())
        .collect();
    assert_eq!(urgent, [vec![(v, true)]]);

    for (clause, read) in [
        (
            "post Either using v on Config::Call::bump { present(self.v) or self.late }\n",
            "present(self.v)",
        ),
        (
            "time invariant All when { forall(c in allInstances<Config::Call>(Config::calls) : present(c.v)) } { self.x <= 5 ms }\n",
            "present(c.v)",
        ),
    ] {
        let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{clause}"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::UnsupportedConstruct, "not-yet-implemented", read),
            "{clause}"
        );
    }
}

/// The STD-493 `absent` ruling (TC-685), lowering: `absent` lowers to QSpec
/// FR-322's `none` literal, a `value`/`literal` node of kind `none` whose
/// type is the `Option<Integer>` it was typed by, the function's result.
#[trace("TC-685")]
#[test]
fn absent_lowers_to_the_none_literal_of_its_option() {
    let document = rpc_document_with(&["x"], false);
    let graph = check(
        &document,
        &format!("{UNITS}{DENSE}function f using v(): Option<Integer> pure {{ absent }}\n"),
    )
    .expect("absent checks");
    let nodes: Vec<Value> = graph
        .semantic_graph()
        .nodes()
        .map(|node| serde_json::from_slice(node.preimage()).expect("a JSON preimage"))
        .collect();
    let [none] = nodes
        .iter()
        .filter(|node| node["body"]["term"] == "literal" && node["body"]["value_kind"] == "none")
        .collect::<Vec<_>>()[..]
    else {
        panic!("one none literal");
    };
    assert_eq!(
        (
            &none["node_tag"],
            &none["semantic_form"],
            &none["body"]["value"]
        ),
        (&json!("value"), &json!("literal"), &Value::Null)
    );
    // Typed by the function's result type, `Option<Integer>`.
    let function = nodes
        .iter()
        .find(|node| node["node_tag"] == "function")
        .expect("the function node");
    assert_eq!(none["body"]["type"], function["semantic_type"]);
    assert!(nodes
        .iter()
        .any(|node| node["semantic_form"] == "option" && node["node_tag"] == "composite_type"));
}

/// [`rpc_document_with`]`(&["x"])` with the variant type `Mode { Idle,
/// Armed(Clock) }` held by the `Call` field `state`, which `bump` writes.
fn union_clock_document() -> Vec<u8> {
    union_clock_document_with(false)
}

/// [`union_clock_document`], with the second clock case `Waiting(Clock)`
/// when `waiting` holds.
fn union_clock_document_with(waiting: bool) -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    let mode = package("Mode");
    let case = |name: &str, line: u64, payload: Option<&str>| {
        let mut case = json!({
            "identity": package(&format!("variant/Mode-{name}")), "name": name,
            "origin": {"source": {"sourceIdentity": package("spec"), "path": "spec.qspec",
                "startLine": line, "startColumn": 3}},
        });
        if let Some(payload) = payload {
            case["payloadType"] = json!(payload);
        }
        case
    };
    let mut variant = wire_type(
        &mode,
        "mode",
        json!({"variants": [case("Idle", 10, None), case("Armed", 11, Some("ix://quire/native/Clock"))]}),
    );
    if waiting {
        variant["variants"]
            .as_array_mut()
            .expect("variants")
            .push(case("Waiting", 12, Some("ix://quire/native/Clock")));
    }
    variant["displayName"] = json!("Mode");
    document["types"]
        .as_array_mut()
        .expect("types")
        .push(variant);
    let mut construct = wire_construct(
        "mode",
        meaning::VARIANT_TYPE,
        json!({"variants": "required"}),
    );
    construct["construct"]["shape"] = json!("enumeration");
    document["constructs"]
        .as_array_mut()
        .expect("constructs")
        .push(construct);
    let call_type = &mut document["types"][1];
    call_type["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(field("state", &mode));
    for operation in call_type["operations"].as_array_mut().expect("operations") {
        if operation["name"] == "bump" {
            operation["frame"]["modifies"]
                .as_array_mut()
                .expect("frame")
                .push(json!(call("state")));
        }
    }
    serde_json::to_vec(&document).expect("JSON")
}

/// FR-230-AC-12 (TC-685), a union case's clock: over `state: Mode` with
/// `Mode { Idle, Armed(Clock) }`, the case `Armed(Clock)` is the place
/// `Call.state.Armed`, guarded by the case, with no presence bit.
/// `present(self.state.Armed) and
/// value(self.state.Armed) >= 1 ms` is one part `state.Armed >= 1`
/// requiring the case, `self.state = Config::Mode::Armed(0 ms)` resets the
/// place, which selects the case, and `self.state = Config::Mode::Idle`
/// drops it; `value(self.state.Armed) >= 1 ms` without the presence
/// refuses `undefined_expression`/`unproved-presence`.
#[trace("TC-685")]
#[test]
fn a_union_case_clock_is_a_guarded_place() {
    let document = union_clock_document();
    let body = format!(
        "{UNITS}{DENSE}\
         pre Armed using v on Config::Call::reply {{ present(self.state.Armed) and value(self.state.Armed) >= 1 ms }}\n\
         post Arm using v on Config::Call::bump {{ self.state = Config::Mode::Armed(0 ms) }}\n"
    );
    let graph = check(&document, &body).expect("a union case's clock checks");
    let model = timed(&graph);
    let (armed, field) = model
        .clock_fields()
        .find(|(_, field)| field.name() == "state.Armed")
        .expect("state.Armed is a place");
    assert_eq!(
        field.path(),
        [
            PlaceStep::Field("state".to_owned()),
            PlaceStep::Union("Armed".to_owned())
        ]
    );
    assert!(!field.presence_bit());
    let [part] = model.guards()[0].parts() else {
        panic!("one part");
    };
    assert_eq!(part.presences(), [(armed, true)]);
    assert_eq!(part.clocks()[0].clock(), armed);
    let resets: Vec<_> = model
        .resets()
        .iter()
        .flat_map(|resets| resets.resets())
        .map(|reset| (reset.clock(), reset.value().clone()))
        .collect();
    assert_eq!(resets, [(armed, exact(0, 1))]);

    let idle = format!(
        "{UNITS}{DENSE}post Disarm using v on Config::Call::bump {{ self.state = Config::Mode::Idle }}\n"
    );
    let graph = check(&document, &idle).expect("another case checks");
    let [resets] = timed(&graph).resets() else {
        panic!("one postcondition");
    };
    assert_eq!(resets.drops(), [armed]);
    assert!(resets.resets().is_empty());

    let unproved = format!(
        "{UNITS}{DENSE}pre Late using v on Config::Call::reply {{ value(self.state.Armed) >= 1 ms }}\n"
    );
    let (cause, text) = refused(&document, &unproved);
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (
            Code::UndefinedExpression,
            "unproved-presence",
            "value(self.state.Armed) >= 1 ms"
        )
    );
}

/// FR-230 "Presence bit" (TC-685), part of FR-230-AC-39: a top-level
/// postcondition conjunct `present(self.v)`, `self.v != absent` or `not
/// (self.v = absent)` requires `v` present in the post-state, and `not
/// present(self.v)` or `not (self.v != absent)` requires it absent; each is recorded on the postcondition's resets, with no reset or
/// drop.
#[trace("TC-685")]
#[test]
fn a_postcondition_requires_a_presence_of_its_post_state() {
    let document = option_clock_document();
    for (conjunct, present) in [
        ("present(self.v)", true),
        ("self.v != absent", true),
        ("not (self.v = absent)", true),
        ("not present(self.v)", false),
        ("not (self.v != absent)", false),
    ] {
        let body =
            format!("{UNITS}{DENSE}post Kept using v on Config::Call::bump {{ {conjunct} }}\n");
        let graph =
            check(&document, &body).unwrap_or_else(|refusals| panic!("{conjunct}: {refusals:?}"));
        let model = timed(&graph);
        let (v, _) = model
            .clock_fields()
            .find(|(_, field)| field.name() == "v")
            .expect("v is a place");
        let [resets] = model.resets() else {
            panic!("{conjunct}: one postcondition");
        };
        assert_eq!(resets.requires(), [(v, present)], "{conjunct}");
        assert!(resets.resets().is_empty() && resets.drops().is_empty());
    }
}

/// FR-230 "Records, tuples and unions" (TC-685), part of FR-230-AC-12:
/// writing a clock-bearing union field. With a second clock case
/// `Waiting(Clock)`, `self.state = Config::Mode::Armed(0 ms)` resets
/// `Armed` and drops `Waiting`. A postcondition writes the field at most
/// once: `Armed(0 ms) and Armed(1 ms)` and `Armed(0 ms) and Idle` refuse
/// `invalid_timed_model`/`conflicting-clock-write` at the second write. A
/// write outside the frame refuses `clock-write`, and a whole read of the
/// field, in a guard or through `pre`, refuses as a clock read outside a
/// constraint.
#[trace("TC-685")]
#[test]
fn a_union_field_is_written_once_and_never_read_whole() {
    let document = union_clock_document_with(true);
    let graph = check(
        &document,
        &format!("{UNITS}{DENSE}post Arm using v on Config::Call::bump {{ self.state = Config::Mode::Armed(0 ms) }}\n"),
    )
    .expect("the switch checks");
    let model = timed(&graph);
    let place = |name: &str| {
        model
            .clock_fields()
            .find(|(_, field)| field.name() == name)
            .map(|(clock, _)| clock)
            .unwrap_or_else(|| panic!("{name} is a place"))
    };
    let [resets] = model.resets() else {
        panic!("one postcondition");
    };
    let reset: Vec<_> = resets
        .resets()
        .iter()
        .map(|reset| (reset.clock(), reset.value().clone()))
        .collect();
    assert_eq!(reset, [(place("state.Armed"), exact(0, 1))]);
    assert_eq!(resets.drops(), [place("state.Waiting")]);

    for (clause, tag, at) in [
        (
            "post Twice using v on Config::Call::bump { self.state = Config::Mode::Armed(0 ms) and self.state = Config::Mode::Armed(1 ms) }",
            "conflicting-clock-write",
            "self.state = Config::Mode::Armed(1 ms)",
        ),
        (
            "post Switch using v on Config::Call::bump { self.state = Config::Mode::Armed(0 ms) and self.state = Config::Mode::Idle }",
            "conflicting-clock-write",
            "self.state = Config::Mode::Idle",
        ),
        (
            "post Outside using v on Config::Call::reply { self.state = Config::Mode::Armed(0 ms) }",
            "clock-write",
            "self.state = Config::Mode::Armed(0 ms)",
        ),
        (
            "pre Whole using v on Config::Call::bump { self.state = Config::Mode::Idle }",
            "clock-read-outside-constraint",
            "self.state",
        ),
        (
            "post Kept using v on Config::Call::bump { self.state = pre(self.state) }",
            "clock-read-outside-constraint",
            "self.state",
        ),
    ] {
        let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{clause}\n"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::InvalidTimedModel, tag, at),
            "{clause}"
        );
    }
}

/// QSpec FR-415-AC-21, the vectors FP-11 and FP-12 (TC-685): a
/// postcondition that requires `present(p)` of a place it does not write,
/// where the operation's precondition establishes `p` absent, has no
/// successor, decided statically: `invalid_timed_model`/
/// `clock-presence-unwritten` at the requirement, for `v: Option<Clock>`
/// and for the union case `state.Armed` alike. With the write (`self.v = 5
/// ms`, the constructor `Armed(5 ms)`) it checks.
#[trace("TC-685")]
#[test]
fn a_presence_the_precondition_denies_and_nothing_writes_refuses() {
    for (document, place, write) in [
        (option_clock_document(), "self.v", "self.v = 5 ms"),
        (
            union_clock_document_with(false),
            "self.state.Armed",
            "self.state = Config::Mode::Armed(5 ms)",
        ),
    ] {
        let pre = format!("pre Off using v on Config::Call::bump {{ not present({place}) }}\n");
        let (cause, text) = refused(
            &document,
            &format!(
                "{UNITS}{DENSE}{pre}post On using v on Config::Call::bump {{ present({place}) }}\n"
            ),
        );
        let requirement = format!("present({place})");
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (
                Code::InvalidTimedModel,
                "clock-presence-unwritten",
                requirement.as_str()
            )
        );
        check(
            &document,
            &format!(
                "{UNITS}{DENSE}{pre}post On using v on Config::Call::bump {{ {write} and present({place}) }}\n"
            ),
        )
        .unwrap_or_else(|refusals| panic!("{place} written: {refusals:?}"));
    }
}

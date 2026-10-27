// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Places": a clock below an object's field, through a package
//! record, a clock alias or a union case, read and reset by its path.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{ClockOp, CollectionShape, PlaceStep};
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
        assert_eq!(model.dimension(place.owner()), 3 + upper.unwrap_or(0));
    }
}

/// Part of FR-230-AC-21 (TC-685), the record parts of "Element identity":
/// over a `Timer` collection `ts`, `size(self.ts)`, a data field of an
/// element read through a traversal (`size(filter(e in self.ts: e.n > 1))`)
/// are data reads and check; an element's clock read in a traversal body
/// (`filter` and `count` over `e.x`), an element read whole (`e`), `=` over the collection and
/// `contains` compare or read clocks and refuse
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
    ] {
        let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{clause}\n"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::InvalidTimedModel, "clock-read-outside-constraint", at),
            "{clause}"
        );
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
                    .map(|element| (element.field().to_owned(), element.slot()))
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
                            constraint.slots().to_vec(),
                            constraint.op(),
                            format!("{:?}", constraint.bound()),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

/// Part of FR-230-AC-21 (TC-685), "Quantified reads" in a precondition:
/// over `ts: Sequence<Timer>[0, 3]`, the guard `forall(e in self.ts: e.x <=
/// 3 ms)` records one entry for each size 0 to 3, each with the data
/// predicate `size(self.ts) = s` and `x <= 3` on each slot below `s` (none
/// for size 0); `exists(e in self.ts: e.n = 2 and e.x >= 1 ms)` records one
/// convex part for each slot `j`, with the data predicate "holds slot `j`"
/// and `n = 2` of the element at `j`, and `x >= 1` on slot `j`. A
/// collection with no declared maximum refuses as not checked.
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
    let refusals = check(
        &unbounded,
        &format!(
            "{UNITS}{DENSE}pre Near using v on Config::Call::reply {{ \
             forall(e in self.ts: e.x <= 3 ms) }}\n"
        ),
    )
    .expect_err("an unbounded quantified guard is not checked");
    assert!(
        matches!(
            refusals.as_slice(),
            [qsl_semantics::check::CheckRefusal {
                cause: qsl_semantics::check::CheckCause::TimedNotChecked { .. },
                ..
            }]
        ),
        "{refusals:?}"
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
                selection.field(),
                selection.predicate().is_some(),
            ),
            ("ts.[slot].x", value, "ts", predicate),
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

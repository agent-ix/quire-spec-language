// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Places": a clock below an object's field, through a package
//! record or a clock alias, read and reset by its path.

use filament_semantic_value::checking::CheckMode;
use filament_semantic_value::declaration::CompositeShape;
use fsl_foundation::diagnostic::Code;
use fsl_semantics::check::{
    BindingValue, BodyTerm, CheckedGraph, ClockOp, CollectionShape, LeafTerm, LiteralValue,
    MemberTerm, NodeRef, NodeTag, PlaceStep, SlotRef,
};
use ix_trace_rs::trace;
use serde_json::{json, Value};

use crate::model_operations::{version_number_value_type, wire_construct, wire_field, wire_type};
use crate::timed_model::{
    call, check, exact, field, refused, rpc_document_with, shown_parts, timed, PACKAGE, UNITS,
};
use fsl_semantics::model::intake::meaning;

use crate::ring_queue;

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
        "ix://filament/native/Clock",
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
/// lists `n` only. The three objects' clocks are `fsl-eval`'s.
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
        if matches!(node.kind(), filament_checked::node::NodeKind::Literal(_)) {
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
            [fsl_semantics::check::CheckRefusal {
                cause: fsl_semantics::check::CheckCause::TimedNotChecked {
                    construct: fsl_semantics::check::TimedConstruct::Clock,
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
/// `self.peer` selects that object (`fsl-eval`'s
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
                    map: false,
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
        assert_eq!(
            model.dimension(&place.owner()),
            Some(3 + upper.unwrap_or(0))
        );
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
    model: &fsl_semantics::check::TimedModel,
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
#[trace("TC-685", "FR-230-AC-55", "FSpec-FR-415-AC-24")]
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

/// FR-230-AC-56 / FR-415-AC-24: a universal quantified invariant keeps one
/// family selector, which bounds each live collection slot and has no
/// instance to constrain for an empty collection.
#[trace("TC-685", "FR-230-AC-56", "FSpec-FR-415-AC-24")]
#[test]
fn a_quantified_time_invariant_bounds_the_live_slot_family() {
    let document = timer_collection(Some(3), true, false);
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}time invariant All when {{ true }} {{ \
             forall(e in self.ts: e.x <= 9 ms) }}\n"
        ),
    )
    .expect("a universal quantified invariant checks");
    let model = timed(&graph);
    let [invariant] = model.invariants() else {
        panic!("one invariant: {:?}", model.invariants());
    };
    let [binding] = invariant.binders() else {
        panic!("one slot binder: {:?}", invariant.binders());
    };
    assert_eq!(binding.name(), "e");
    let [bound] = invariant.bounds() else {
        panic!("one live-slot bound: {:?}", invariant.bounds());
    };
    assert_eq!(bound.slots(), [SlotRef::Family]);
    assert_eq!(bound.binding(), Some(binding));
    assert_eq!(bound.op(), ClockOp::LessOrEqual);
    assert_eq!(bound.bound(), &exact(9, 1));
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
    use fsl_semantics::check::Written;

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
            if matches!(node.kind(), filament_checked::node::NodeKind::Literal(_)) {
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
                kind: filament_exact::CollectionKind::Sequence,
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
                kind: filament_exact::CollectionKind::Set,
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
/// `items` is `ix://filament/native/Clock` types `Call.ks` as the clock
/// collection `ks.[slot]` (FR-230-AC-16). A sequence type `Flags` whose
/// `items` is `ix://filament/native/Boolean` types `Call.ds` as a sequence of
/// `Boolean`. Not covered: the map halves, which filament-exact's value types
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
        types.push(sequence("Flags", "ix://filament/native/Boolean"));
        types.push(sequence("Ticks", "ix://filament/native/Clock"));
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
        map: false,
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
    let filament_exact::ValueType::Collection(ds) = &ds else {
        panic!("ds is a collection: {ds:?}");
    };
    assert_eq!(ds.kind(), filament_exact::CollectionKind::Sequence);
    assert!(
        matches!(ds.element(), filament_exact::ValueType::Boolean),
        "{ds:?}"
    );
}

/// FR-230-AC-23, the FSL-side shape: `Call.q` typed `ix://filament/native/Clock`
/// with multiplicity `{0, 3, true, false}` is a clock collection `q.[slot]`
/// of at most three slots, `Call.r` typed `Timer` with `{0, 2, true, false}`
/// holds `r.[slot].x`, a `Clock` sequence type `Ticks` types `Call.ks`, and
/// an operation parameter `p: Timer` and result `Timer`, each of multiplicity
/// `{0, 2, true, false}`, check. The static dimension of one `Call` counts `q`'s three slots and
/// `r`'s two beside the record clocks, and a collection maximum of `4`
/// instead of `3` changes the place bound and dimension. The parameter and
/// result have the type of `Call.r` (a `Sequence` of `Timer`). A map type
/// `TickMap` whose `values` is `ix://filament/native/Clock` is declared beside
/// `Ticks` and is admitted. Not covered: a member typed by `TickMap` (a clock
/// under a `Map` value has no place id; the Map ruling is QSL-1169), package identity changing with the collection
/// maximum, which the checked graph does not expose here (QSL-1030 follow-up
/// for the spec lane/IR owner of identity projection); a `Clock`-typed
/// collection parameter or result, which `model_operation` in
/// `check/assemble.rs` refuses as `UnsupportedModelMember`.
#[trace("FR-230-AC-23")]
#[test]
fn a_clock_collection_field_parameter_and_result_check_with_their_bounds() {
    let document = |maximum: u64| {
        timer_document(move |document: &mut Value| {
            let sequence = {
                let mut node = wire_type(
                    &package("Ticks"),
                    "unused",
                    json!({"items": "ix://filament/native/Clock"}),
                );
                node["kind"] = json!("sequence");
                node
            };
            let types = document["types"].as_array_mut().expect("types");
            types.push(sequence);
            let mut map = wire_type(
                &package("TickMap"),
                "unused",
                json!({"keys": "ix://filament/native/Text", "values": "ix://filament/native/Clock", "bound": {"lower": 0}}),
            );
            map["kind"] = json!("map");
            types.push(map);
            let mut q = field("q", "ix://filament/native/Clock");
            q["multiplicity"] =
                json!({"lower": 0, "upper": maximum, "ordered": true, "unique": false});
            let mut r = field("r", &package("Timer"));
            r["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
            let fields = types[1]["fields"].as_array_mut().expect("Call fields");
            fields.push(q);
            fields.push(r);
            fields.push(field("ks", &package("Ticks")));
            for operation in types[1]["operations"].as_array_mut().expect("operations") {
                if operation["name"] == "bump" {
                    let mut parameter = wire_field(
                        &format!("{}/p", call("bump")),
                        "p",
                        &package("Timer"),
                        "required",
                        1,
                    );
                    parameter["multiplicity"] =
                        json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
                    operation["params"] = json!([parameter]);
                    operation["returns"] = json!({
                        "typeRef": package("Timer"),
                        "multiplicity": {"lower": 0, "upper": 2, "ordered": true, "unique": false},
                        "nullable": false,
                    });
                }
            }
        })
    };
    let dimension_of = |maximum: u64| {
        let graph = check(&document(maximum), &format!("{UNITS}{DENSE}"))
            .unwrap_or_else(|refusals| panic!("the collection shape checks: {refusals:?}"));
        let model = timed(&graph);
        let place = |name: &str| {
            model
                .clocks()
                .iter()
                .find(|clock| clock.name() == name)
                .unwrap_or_else(|| panic!("{name}: {:?}", model.clocks()))
        };
        let q = place("q.[slot]");
        assert_eq!(
            q.path(),
            [
                PlaceStep::Field("q".to_owned()),
                PlaceStep::Slot(CollectionShape {
                    ordered: true,
                    unique: false,
                    min: 0,
                    max: Some(maximum),
                    map: false,
                }),
            ]
        );
        assert_eq!(q.bound(), Some(maximum));
        assert_eq!(place("r.[slot].x").bound(), Some(2));
        assert_eq!(place("ks.[slot]").bound(), None);
        let owner = q.owner();
        let r = graph
            .scope()
            .types()
            .attribute(owner.clone(), "r")
            .expect("Call.r")
            .field()
            .value_type()
            .clone();
        let filament_exact::ValueType::Collection(shape) = &r else {
            panic!("r is a collection: {r:?}");
        };
        assert_eq!(shape.kind(), filament_exact::CollectionKind::Sequence);
        let bump = graph
            .scope()
            .operations()
            .declared_by(owner.clone())
            .iter()
            .find(|operation| operation.name() == "bump")
            .expect("bump");
        assert_eq!(bump.parameters(), &[("p".to_owned(), r.clone())]);
        assert_eq!(bump.result(), Some(&r));
        model.dimension(&owner)
    };
    // `x`, `t.x`, `u.x`, then `q`'s three slots and `r`'s two.
    assert_eq!(dimension_of(3), Some(3 + 3 + 2));
    assert_eq!(dimension_of(4), Some(3 + 4 + 2));
}

/// Part of FR-230-AC-16 and FR-230-AC-17 (TC-685), the alias halves: an
/// alias `Stopwatch` of `ix://filament/native/Clock` types `Call.s` as the
/// clock `s`; an alias `D` of the unit `ix://filament/unit/ms` types `Call.d`
/// as `Quantity<ms>`, and a sequence type `Ds` whose `items` is `D` types
/// `Call.ds` as a sequence of `Quantity<ms>`. Not covered here: the map
/// halves (filament-exact has no map value type); the parameter, result and
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
            json!({"target": "ix://filament/native/Clock"}),
        ));
        types.push(core(
            "D",
            "alias",
            json!({"target": "ix://filament/unit/ms"}),
        ));
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
        matches!(typed("d"), filament_exact::ValueType::Quantity(_)),
        "{:?}",
        typed("d")
    );
    let ds = typed("ds");
    let filament_exact::ValueType::Collection(ds) = &ds else {
        panic!("ds is a collection: {ds:?}");
    };
    assert_eq!(ds.kind(), filament_exact::CollectionKind::Sequence);
    assert!(
        matches!(ds.element(), filament_exact::ValueType::Quantity(_)),
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
/// `ix://filament/native/Clock` bounds every clock place of `Call` by 3; a
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
                json!({"target": "ix://filament/native/Clock", "constraints": constraints}),
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
                "ix://filament/native/Clock"
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
            [fsl_semantics::check::CheckRefusal {
                cause: fsl_semantics::check::CheckCause::Temporal(cause),
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
/// `ix://filament/unit/<package>/ms` as the `target` of an alias `D` types the
/// field `Call.d`, the parameter `bump(p: D)` and the result of `bump` as
/// `Quantity<ms>`, and a sequence type `Ms` whose `items` is that unit
/// types `Call.ms` as a sequence of `Quantity<ms>`. The same alias `target`
/// or sequence `items` naming `ix://filament/unit/ghost` refuses
/// `missing_declaration`/`missing-name` naming the type definition and its
/// path `TypeDefinition.target` or `TypeDefinition.items`, once, whether
/// or not a member is typed by it. Not covered: the map half (IR-1046,
/// filament-exact has no map value type).
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
        &document(&format!("ix://filament/unit/{PACKAGE}/ms"), true),
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
            .attribute(owner.clone(), name)
            .unwrap_or_else(|| panic!("Call.{name}"))
            .field()
            .value_type()
            .clone()
    };
    let quantity = typed("d");
    assert!(
        matches!(quantity, filament_exact::ValueType::Quantity(_)),
        "{quantity:?}"
    );
    let ms = typed("ms");
    let filament_exact::ValueType::Collection(ms) = &ms else {
        panic!("ms is a collection: {ms:?}");
    };
    assert_eq!(ms.kind(), filament_exact::CollectionKind::Sequence);
    assert_eq!(ms.element(), &quantity);
    let bump = graph
        .scope()
        .operations()
        .declared_by(owner.clone())
        .iter()
        .find(|operation| operation.name() == "bump")
        .expect("bump");
    assert_eq!(bump.parameters(), &[("p".to_owned(), quantity.clone())]);
    assert_eq!(bump.result(), Some(&quantity));

    for used in [true, false] {
        let refusal = crate::model_operations::admit_and_assemble_with_profiles(
            &document("ix://filament/unit/ghost", used),
            crate::timed_model::TEMPORAL,
            &format!("{UNITS}{DENSE}"),
        )
        .expect_err("a ghost unit refuses");
        let mut named: Vec<(Code, String, String)> = refusal
            .errors
            .iter()
            .map(|error| {
                let fsl_semantics::check::AssemblyCause::UnresolvedModelQuantityType {
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
/// FND-004: a `max` `"3"` with `appliesTo` `ix://filament/native/Clock` on an
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
            "ix://filament/native/Clock",
        )]
    };
    let tick = "time Config tick Config::Call::bump period 1 ms unit ms;\n";
    for time in [tick, ""] {
        let (cause, _) = refused(&unclocked(on_clock()), &format!("{UNITS}{time}"));
        assert!(
            matches!(
                cause,
                fsl_semantics::check::TemporalCause::ClockWithoutDenseTime { .. }
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
                    "ix://filament/native/Clock",
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
            types.push(alias("Timeout", "ix://filament/native/Clock", on_timeout));
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
        json!({"target": "ix://filament/native/Clock"}),
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
    let clock = "ix://filament/native/Clock";
    let post = format!("{UNITS}{DENSE}post Bumped using v on Config::Call::bump {{ c = 0 ms }}\n");
    let graph = check(&clock_parameter_document(clock, Some(clock), &["x"]), &post)
        .unwrap_or_else(|refusals| panic!("bump(c: Clock): Clock checks: {refusals:?}"));
    let model = timed(&graph);
    let names = |clock: fsl_semantics::check::ClockId| {
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
    let mut v = field("v", "ix://filament/native/Clock");
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

/// The `Call` model with a self-reference used to exercise a clock place
/// whose owner is read through another object reference.
fn option_clock_peer_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&option_clock_document()).expect("JSON");
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(field("peer", &package("Call")));
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

/// In a population binder, the presence read's owner is the visited
/// `Reference<Call>` itself. Keep the binder term as the second operand of
/// the clock place's `presence_read`; lowering an object dereference here
/// would give each place two encodings.
#[trace("TC-685", "FR-829-AC-3", "FR-1040-AC-4", "FSpec-FR-322-AC-455")]
#[test]
fn a_population_clock_presence_read_uses_the_visited_reference() {
    let document = option_clock_document();
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}time invariant PopulationPresence when {{ \
             exists(c in Config::calls : present(c.v)) }} {{ self.x <= 1 ms }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("population presence read checks: {refusals:?}"));
    let semantic_graph = graph.semantic_graph();
    let reads: Vec<_> = semantic_graph
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.option.present" =>
            {
                Some(application)
            }
            _ => None,
        })
        .collect();
    let [read] = reads.as_slice() else {
        panic!("one population presence_read: {reads:?}");
    };
    let [MemberTerm::Leaf(LeafTerm::Reference { target: place }), MemberTerm::Leaf(LeafTerm::Reference { target: owner })] =
        read.arguments.as_slice()
    else {
        panic!("presence_read has the template then the owner term: {read:?}");
    };
    let place = semantic_graph
        .node(
            place
                .admitted()
                .expect("place template is admitted")
                .clone(),
        )
        .expect("place template node exists");
    assert_eq!(place.semantic_form(), "clock_place");
    let owner = semantic_graph
        .node(owner.admitted().expect("owner term is admitted").clone())
        .expect("owner node exists");
    assert_eq!(owner.node_tag(), NodeTag::Value);
    assert_eq!(owner.semantic_form(), "parameter");
    let BodyTerm::Aggregate(owner_body) = owner.body() else {
        panic!("the owner parameter carries its binder name");
    };
    let Some(MemberTerm::Binding(name)) = owner_body.members.first() else {
        panic!("the owner parameter begins with a name binding");
    };
    assert_eq!(name.name, "name");
    let BindingValue::Leaf(LeafTerm::Literal {
        value: LiteralValue::Text(owner_name),
        ..
    }) = &name.value
    else {
        panic!("the owner parameter's name is text");
    };
    assert_eq!(owner_name, "c");
}

/// The arguments of every application of `identity` in `graph`.
fn applications_of(graph: &CheckedGraph, identity: &str) -> Vec<Vec<MemberTerm>> {
    graph
        .semantic_graph()
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application) if application.operation.identity() == identity => {
                Some(application.arguments.clone())
            }
            _ => None,
        })
        .collect()
}

/// The semantic form of the node a leaf reference `member` names.
fn form_of(graph: &CheckedGraph, member: &MemberTerm) -> String {
    let MemberTerm::Leaf(LeafTerm::Reference { target }) = member else {
        panic!("expected a node reference: {member:?}");
    };
    graph
        .semantic_graph()
        .node(target.admitted().expect("admitted reference").clone())
        .expect("referenced node exists")
        .semantic_form()
        .to_owned()
}

/// The name of the `parameter` node a leaf reference `member` names.
fn parameter_name(graph: &CheckedGraph, member: &MemberTerm) -> String {
    assert_eq!(form_of(graph, member), "parameter", "{member:?}");
    let MemberTerm::Leaf(LeafTerm::Reference { target }) = member else {
        panic!("expected a node reference: {member:?}");
    };
    let node = graph
        .semantic_graph()
        .node(target.admitted().expect("admitted reference").clone())
        .expect("parameter node exists");
    let BodyTerm::Aggregate(body) = node.body() else {
        panic!("a parameter is an aggregate: {node:?}");
    };
    let Some(MemberTerm::Binding(name)) = body.members.first() else {
        panic!("a parameter begins with its name: {node:?}");
    };
    let BindingValue::Leaf(LeafTerm::Literal {
        value: LiteralValue::Text(name),
        ..
    }) = &name.value
    else {
        panic!("a parameter name is text: {node:?}");
    };
    name.clone()
}

/// FR-1040-AC-4: `count` and `sum` over a population lower each presence read
/// of `c.v` to a `presence_read` over the place template whose owner term is
/// the binder `c` itself.
#[trace("TC-3084", "FR-1040-AC-4")]
#[test]
fn population_presence_in_count_and_sum_lowers_with_the_binder_as_owner() {
    let document = option_clock_document();
    for predicate in [
        "count(c in Config::calls where present(c.v)) >= 0",
        "sum(c in Config::calls: if present(c.v) then 1 else 0) >= 0",
    ] {
        let graph = check(
            &document,
            &format!(
                "{UNITS}{DENSE}time invariant Crowd when {{ {predicate} }} \
                 {{ self.x <= 1 ms }}\n"
            ),
        )
        .unwrap_or_else(|refusals| panic!("{predicate}: {refusals:?}"));
        let reads = applications_of(&graph, "filament.op.option.present");
        let [read] = reads.as_slice() else {
            panic!("{predicate}: one presence read: {reads:?}");
        };
        let [place, owner] = read.as_slice() else {
            panic!("{predicate}: the template then the owner: {read:?}");
        };
        assert_eq!(form_of(&graph, place), "clock_place", "{predicate}");
        assert_eq!(parameter_name(&graph, owner), "c", "{predicate}");
    }
}

/// FR-1040-AC-4: a presence read in a state clause checks, with a binder
/// owner, whether or not the clause also constrains a clock.
#[trace("TC-3084", "FR-1040-AC-4")]
#[test]
fn a_state_clause_can_read_population_clock_presence() {
    let document = option_clock_document();
    check(
        &document,
        &format!(
            "{UNITS}{DENSE}pre Named using v on Config::Call::reply {{ \
             count(c in Config::calls where present(c.v)) >= 0 }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("state-clause population presence checks: {refusals:?}"));
}

/// FR-1040-AC-2: `present(self.peer.v)` in a guard that also constrains a
/// clock lowers to a `presence_read` over the template whose owner term is the
/// `peer` field read itself, with no `deref`.
#[trace("TC-3084", "FR-1040-AC-2")]
#[test]
fn a_state_clause_can_read_clock_presence_through_a_reference() {
    let document = option_clock_peer_document();
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}pre Named using v on Config::Call::reply {{ \
             present(self.peer.v) and self.x >= 1 ms }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("reference-owner presence checks: {refusals:?}"));
    let reads = applications_of(&graph, "filament.op.option.present");
    let [read] = reads.as_slice() else {
        panic!("one reference-owner presence read: {reads:?}");
    };
    let [place, owner] = read.as_slice() else {
        panic!("the presence read carries the template then the owner: {read:?}");
    };
    assert_eq!(form_of(&graph, place), "clock_place");
    let MemberTerm::Leaf(LeafTerm::Reference { target }) = owner else {
        panic!("the owner is a node reference: {owner:?}");
    };
    let owner = graph
        .semantic_graph()
        .node(target.admitted().expect("owner is admitted").clone())
        .expect("owner term exists");
    let BodyTerm::Application(owner_read) = owner.body() else {
        panic!("the owner is the peer field read itself: {owner:?}");
    };
    assert_eq!(
        owner_read.operation.identity(),
        "filament.op.record.project"
    );
    let Some(fsl_semantics::value::Member::Field { name, .. }) = owner_read.operation.member()
    else {
        panic!("the owner read selects a field: {owner_read:?}");
    };
    assert_eq!(name.as_str(), "peer");
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
    use filament_exact::IllTypedCause;
    use fsl_semantics::check::CheckCause;

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

/// [`option_clock_document`] with `v` an `Option<Integer>` instead.
fn option_integer_document() -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(&option_clock_document()).expect("JSON");
    let fields = document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields");
    let v = fields
        .iter_mut()
        .find(|field| field["name"] == "v")
        .expect("the optional field v");
    assert_eq!(v["typeRef"], "ix://filament/native/Clock");
    v["typeRef"] = json!("ix://filament/native/Integer");
    serde_json::to_vec(&document).expect("JSON")
}

/// FR-143-AC-18 (TC-188): `self.v = absent` is admitted as the `none` of the
/// written place's own option, for `v: Option<Clock>` and for
/// `v: Option<Integer>`.
#[trace("FSpec-TC-188", "FSpec-FR-143-AC-18")]
#[test]
fn the_absent_write_is_the_none_of_an_option_clock_and_an_option_integer() {
    let body =
        format!("{UNITS}{DENSE}post Disarm using v on Config::Call::bump {{ self.v = absent }}\n");
    check(&option_clock_document(), &body).expect("the Option<Clock> write checks");
    check(&option_integer_document(), &body).expect("the Option<Integer> write checks");
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
/// A presence read of `self` this slice does not read (under `or` in a
/// postcondition) refuses `unsupported_construct`/`not-yet-implemented` at
/// the read, never as a clock read.
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
    let [fsl_semantics::check::Urgency::When { alternatives, .. }] = model.urgency() else {
        panic!("one urgent when");
    };
    let urgent: Vec<_> = alternatives
        .iter()
        .map(|alternative| alternative.presences().to_vec())
        .collect();
    assert_eq!(urgent, [vec![(v, true)]]);

    let either = "post Either using v on Config::Call::bump { present(self.v) or self.late }\n";
    let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{either}"));
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (
            Code::UnsupportedConstruct,
            "not-yet-implemented",
            "present(self.v)"
        )
    );
}

/// The STD-493 `absent` ruling (TC-685), lowering: `absent` lowers to FSpec
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
    let nodes: Vec<_> = graph.semantic_graph().nodes().collect();
    let [none] = nodes
        .iter()
        .filter(|node| {
            matches!(
                node.body(),
                BodyTerm::Leaf(LeafTerm::Literal {
                    value: LiteralValue::None,
                    ..
                })
            )
        })
        .collect::<Vec<_>>()[..]
    else {
        panic!("one none literal");
    };
    assert_eq!(
        (none.node_tag(), none.semantic_form()),
        (NodeTag::Value, "literal")
    );
    // Typed by the function's result type, `Option<Integer>`.
    let function = nodes
        .iter()
        .find(|node| node.node_tag() == NodeTag::Function)
        .expect("the function node");
    let BodyTerm::Leaf(LeafTerm::Literal { ty, .. }) = none.body() else {
        unreachable!("filtered to a literal");
    };
    assert_eq!(
        Some(ty),
        function.semantic_type().map(NodeRef::Admitted).as_ref()
    );
    assert!(nodes
        .iter()
        .any(|node| node.semantic_form() == "option" && node.node_tag() == NodeTag::CompositeType));
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

/// FSpec FR-322-AC-155 and FSL FR-230 "Presence is derived" (TC-685 step
/// 34), checking: with `peer: Reference<Call>`, `present(self.peer.v)` of
/// the object `self.peer` refers to is a data read of that object's place
/// presence, never a clock read, so a time invariant whose `when` reads it
/// checks (FSpec PR-01, an owner other than `self`). An owner that is no
/// object, `present(self.late.v)` over the `Boolean` `late`, refuses
/// `ill_typed`/`type-mismatch` (PR-05).
#[trace("TC-685")]
#[test]
fn another_objects_presence_is_a_data_read() {
    let mut document: Value = serde_json::from_slice(&option_clock_document()).expect("JSON");
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(field("peer", &package("Call")));
    let document = serde_json::to_vec(&document).expect("JSON");
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}time invariant Peer when {{ present(self.peer.v) }} {{ self.x <= 5 ms }}\n"
        ),
    )
    .expect("another object's presence checks");
    let [invariant] = timed(&graph).invariants() else {
        panic!("one invariant");
    };
    let [alternative] = invariant.alternatives() else {
        panic!("one alternative");
    };
    assert!(alternative.presences().is_empty());
    let (cause, _) = refused(
        &document,
        &format!(
            "{UNITS}{DENSE}time invariant Odd when {{ present(self.late.v) }} {{ self.x <= 5 ms }}\n"
        ),
    );
    assert_eq!(
        (cause.code(), cause.tag()),
        (Code::IllTyped, "type-mismatch")
    );
}

/// FR-230-AC-19 and AC-41 (TC-685), S3's part: `q: Sequence<Option<Clock>>[0,
/// 2]` (a presence-optional `Clock` field of multiplicity `[0, 2]`) checks
/// under a dense source to the place `q.[slot]`, guarded by the element's
/// presence bit, whose `resolved_path` rows are those of FSpec's presence
/// place `q`: `Sequence<Option<ix://filament/native/Clock>>` bounded `[0, 2]`
/// and its slot `Option<ix://filament/native/Clock>`. An unbounded one, whose
/// bits the timed system cannot enumerate, is still not checked.
#[trace("TC-685")]
#[test]
fn a_sequence_of_option_clocks_is_a_place_with_a_bit() {
    let with = |upper: Option<u64>| {
        let mut document: Value =
            serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
        let mut q = field("q", "ix://filament/native/Clock");
        q["presence"] = json!("optional");
        q["multiplicity"] = json!({"lower": 0, "ordered": true, "unique": false});
        if let Some(upper) = upper {
            q["multiplicity"]["upper"] = json!(upper);
        }
        document["types"][1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(q);
        serde_json::to_vec(&document).expect("JSON")
    };
    let graph = check(&with(Some(2)), &format!("{UNITS}{DENSE}"))
        .unwrap_or_else(|refusals| panic!("q checks: {refusals:?}"));
    let model = timed(&graph);
    let q = model
        .clocks()
        .iter()
        .find(|clock| clock.name() == "q.[slot]")
        .unwrap_or_else(|| panic!("q's place: {:?}", model.clocks()));
    assert!(q.presence_bit());
    let rows: Vec<(&str, &str, &str, &str, &str, &str)> = q
        .resolved_path()
        .iter()
        .map(|row| {
            (
                row.declaring_type.as_str(),
                row.member_identity.as_str(),
                row.member_type.as_str(),
                row.collection_kind.as_str(),
                row.minimum.as_str(),
                row.maximum.as_str(),
            )
        })
        .collect();
    let call = package("Call");
    let q_identity = format!("{call}/q");
    assert_eq!(
        rows,
        [
            (
                call.as_str(),
                q_identity.as_str(),
                "Sequence<Option<ix://filament/native/Clock>>",
                "sequence",
                "0",
                "2"
            ),
            (
                "Option<ix://filament/native/Clock>",
                "Sequence<Option<ix://filament/native/Clock>>[slot]",
                "Option<ix://filament/native/Clock>",
                "none",
                "",
                ""
            ),
        ]
    );
    let refusals = check(&with(None), &format!("{UNITS}{DENSE}")).expect_err("not checked");
    assert!(
        matches!(
            refusals.as_slice(),
            [fsl_semantics::check::CheckRefusal {
                cause: fsl_semantics::check::CheckCause::TimedNotChecked { .. },
                ..
            }]
        ),
        "{refusals:?}"
    );
}

/// `document` with `Call`'s `peer: Reference<Call>` and `peers:
/// Sequence<Reference<Call>>[0, 2]`.
fn with_peers(document: &[u8]) -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(document).expect("JSON");
    let mut peers = field("peers", &package("Call"));
    peers["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
    let fields = document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields");
    fields.push(field("peer", &package("Call")));
    fields.push(peers);
    serde_json::to_vec(&document).expect("JSON")
}

/// FR-230-AC-51 (TC-685 step 34), SR-1754001 FND-002 and SR-1754002
/// FND-001..003: another object's presence is a data predicate of a guard's
/// convex part. `pre G { present(self.peer.v) }` is one part whose data
/// reads it, beside the clock constraint in `present(self.peer.v) and
/// self.x >= 1 ms`; `urgent U when { present(self.peer.v) }` checks alike, and so does the
/// quantified `forall(c in self.peers: present(c.v))`. In a postcondition,
/// which this slice does not read it in, it refuses
/// `unsupported_construct`/`not-yet-implemented` at the read, never as a
/// type mismatch.
#[trace("TC-685")]
#[test]
fn another_objects_presence_is_a_guard_data_predicate() {
    let document = with_peers(&option_clock_document());
    for (guard, clocks) in [
        ("present(self.peer.v)", 0),
        ("present(self.peer.v) and self.x >= 1 ms", 1),
        ("forall(c in self.peers : present(c.v))", 0),
    ] {
        let graph = check(
            &document,
            &format!("{UNITS}{DENSE}pre G using v on Config::Call::bump {{ {guard} }}\n"),
        )
        .unwrap_or_else(|refusals| panic!("{guard}: {refusals:?}"));
        let model = timed(&graph);
        let [guarded] = model.guards() else {
            panic!("{guard}: one guard");
        };
        let [part] = guarded.parts() else {
            panic!("{guard}: one part");
        };
        assert!(part.data().is_some(), "{guard}");
        assert!(part.presences().is_empty(), "{guard}");
        assert_eq!(part.clocks().len(), clocks, "{guard}");
    }
    check(
        &document,
        &format!("{UNITS}{DENSE}urgent U when {{ present(self.peer.v) }};\n"),
    )
    .unwrap_or_else(|refusals| panic!("urgency: {refusals:?}"));
    let (cause, text) = refused(
        &document,
        &format!("{UNITS}{DENSE}post Q using v on Config::Call::bump {{ present(self.peer.v) }}\n"),
    );
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (
            Code::UnsupportedConstruct,
            "not-yet-implemented",
            "present(self.peer.v)"
        )
    );
}

/// FSpec FR-415-AC-21 (TC-685): an Option<Clock> place required present
/// without a write has no successor when its precondition establishes it
/// absent; writing the clock makes it present. Requirements are not writes.
#[trace("TC-685")]
#[test]
fn an_absent_option_clock_required_present_without_a_write_refuses() {
    let document = option_clock_document();
    let pre = "pre Off using v on Config::Call::bump { not present(self.v) }\n".to_owned();
    let (cause, text) = refused(
        &document,
        &format!(
            "{UNITS}{DENSE}{pre}post On using v on Config::Call::bump {{ present(self.v) }}\n"
        ),
    );
    assert_eq!(
        (cause.code(), cause.tag(), text.as_str()),
        (
            Code::InvalidTimedModel,
            "clock-presence-unwritten",
            "present(self.v)"
        )
    );
    check(
        &document,
        &format!(
            "{UNITS}{DENSE}{pre}post On using v on Config::Call::bump {{ self.v = 5 ms and present(self.v) }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("the written option clock: {refusals:?}"));
}

/// FSpec FR-415-AC-20 and STD-526 PR-03 (TC-685 step 34), S3's part: over
/// `q: Sequence<Option<Clock>>[0, 2]`, `forall(e in self.q : present(e))`
/// and `exists(e in self.q : present(e))` are data predicates of the
/// elements' presences, never clock reads: a time invariant's `when`, an
/// `urgent when` and a guard (one convex part whose data reads it) check,
/// and so does the quantifier over another object's `q`
/// (`self.peer.q`). `present(self.q)` with no element is no presence of a
/// place and refuses.
#[trace("TC-685")]
#[test]
fn quantified_presence_over_an_option_sequence_is_a_data_predicate() {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    let mut q = field("q", "ix://filament/native/Clock");
    q["presence"] = json!("optional");
    q["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
    let fields = document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields");
    fields.push(q);
    fields.push(field("peer", &package("Call")));
    // A model whose clocks no operation writes is not emitted whole, so
    // `bump` and `send` write `q`.
    for operation in document["types"][1]["operations"]
        .as_array_mut()
        .expect("operations")
    {
        if matches!(operation["name"].as_str(), Some("send" | "bump")) {
            operation["frame"]["modifies"]
                .as_array_mut()
                .expect("frame")
                .push(json!(call("q")));
        }
    }
    let document = serde_json::to_vec(&document).expect("JSON");
    for body in [
        "time invariant All when { forall(e in self.q : present(e)) } { self.x <= 5 ms }\n",
        "urgent U when { exists(e in self.q : present(e)) };\n",
        "time invariant Peer when { forall(e in self.peer.q : present(e)) } { self.x <= 5 ms }\n",
    ] {
        check(&document, &format!("{UNITS}{DENSE}{body}"))
            .unwrap_or_else(|refusals| panic!("{body}: {refusals:?}"));
    }
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}pre G using v on Config::Call::bump {{ forall(e in self.q : present(e)) and self.x >= 1 ms }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("the guard: {refusals:?}"));
    let model = timed(&graph);
    let [part] = model.guards()[0].parts() else {
        panic!("one part");
    };
    assert!(part.data().is_some());
    assert_eq!(part.clocks().len(), 1);
    let slot_graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}time invariant P when {{ \
             all(c in Config::calls where exists(e in c.q : present(e))) }} {{ self.x <= 5 ms }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("slot presence lowers: {refusals:?}"));
    let semantic_graph = slot_graph.semantic_graph();
    let slots: Vec<_> = semantic_graph
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.option.slots" =>
            {
                Some((node, application))
            }
            _ => None,
        })
        .collect();
    let [(slots_node, slots_app)] = slots.as_slice() else {
        panic!(
            "one option.slots domain: {:?} {:?}",
            timed(&slot_graph).invariants(),
            semantic_graph
                .nodes()
                .map(|node| (
                    node.semantic_form(),
                    format!("{:?}", node.body())
                        .chars()
                        .take(160)
                        .collect::<String>()
                ))
                .collect::<Vec<_>>()
        );
    };
    assert_eq!(slots_node.semantic_form(), "presence_read");
    let reference_key = |member: &MemberTerm| {
        let MemberTerm::Leaf(LeafTerm::Reference { target }) = member else {
            panic!("expected a direct reference operand: {member:?}");
        };
        target
            .admitted()
            .expect("lowered references have admitted keys")
            .clone()
    };
    let [slot_place, slot_owner] = slots_app.arguments.as_slice() else {
        panic!("option.slots takes place then owner: {slots_app:?}");
    };
    let slot_place_key = reference_key(slot_place);
    let slot_owner_key = reference_key(slot_owner);
    let place_node = semantic_graph
        .node(slot_place_key.clone())
        .expect("slot domain place exists");
    assert_eq!(place_node.semantic_form(), "clock_place");
    let owner_node = semantic_graph
        .node(slot_owner_key.clone())
        .expect("slot domain owner exists");
    assert_eq!(owner_node.semantic_form(), "parameter");
    let BodyTerm::Aggregate(owner_body) = owner_node.body() else {
        panic!("the population owner is a parameter: {owner_node:?}");
    };
    let Some(MemberTerm::Binding(owner_name)) = owner_body.members.first() else {
        panic!("the owner parameter binds its name: {owner_node:?}");
    };
    let BindingValue::Leaf(LeafTerm::Literal {
        value: LiteralValue::Text(owner_name),
        ..
    }) = &owner_name.value
    else {
        panic!("the owner parameter name is text: {owner_node:?}");
    };
    assert_eq!(owner_name, "c");
    let presence_reads: Vec<_> = semantic_graph
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.option.present" =>
            {
                Some((node, application))
            }
            _ => None,
        })
        .collect();
    let [(presence_node, presence)] = presence_reads.as_slice() else {
        panic!("one slot presence read: {presence_reads:?}");
    };
    let [presence_place, presence_owner, presence_selector] = presence.arguments.as_slice() else {
        panic!("presence read takes place, owner, and selector: {presence:?}");
    };
    assert_eq!(reference_key(presence_place), slot_place_key);
    assert_eq!(reference_key(presence_owner), slot_owner_key);
    let selector_key = reference_key(presence_selector);
    let selector = semantic_graph
        .node(selector_key)
        .expect("presence selector parameter exists");
    assert_eq!(selector.semantic_form(), "parameter");
    let BodyTerm::Aggregate(selector_body) = selector.body() else {
        panic!("the slot selector is a parameter: {selector:?}");
    };
    let Some(MemberTerm::Binding(selector_name)) = selector_body.members.first() else {
        panic!("the selector parameter binds its name: {selector:?}");
    };
    let BindingValue::Leaf(LeafTerm::Literal {
        value: LiteralValue::Text(selector_name),
        ..
    }) = &selector_name.value
    else {
        panic!("the selector parameter name is text: {selector:?}");
    };
    assert_eq!(selector_name, "slot");

    let quantifier = |identity: &str| {
        let found: Vec<_> = semantic_graph
            .nodes()
            .filter_map(|node| match node.body() {
                BodyTerm::Application(application)
                    if application.operation.identity() == identity =>
                {
                    Some((node, application))
                }
                _ => None,
            })
            .collect();
        let [(node, application)] = found.as_slice() else {
            panic!("one {identity} quantifier: {found:?}");
        };
        (*node, *application)
    };
    let (outer_forall, outer_application) = quantifier("filament.op.collection.forall");
    let (inner_exists, inner_application) = quantifier("filament.op.collection.exists");
    assert_eq!(outer_forall.semantic_form(), "quantify");
    assert_eq!(inner_exists.semantic_form(), "quantify");
    let [_, outer_body] = outer_application.arguments.as_slice() else {
        panic!("population forall takes source and body: {outer_application:?}");
    };
    let MemberTerm::Binding(outer_binding) = outer_body else {
        panic!("population forall binds its body: {outer_body:?}");
    };
    let BindingValue::Leaf(LeafTerm::Reference { target: outer_body }) = &outer_binding.value
    else {
        panic!("population forall body is the inner quantifier: {outer_binding:?}");
    };
    assert_eq!(outer_binding.name, "c");
    assert_eq!(outer_body.admitted().cloned(), Some(inner_exists.key()));
    let [inner_domain, inner_body] = inner_application.arguments.as_slice() else {
        panic!("slot exists takes domain and body: {inner_application:?}");
    };
    assert_eq!(reference_key(inner_domain), slots_node.key());
    let MemberTerm::Binding(inner_binding) = inner_body else {
        panic!("slot exists binds its body: {inner_body:?}");
    };
    let BindingValue::Leaf(LeafTerm::Reference { target: inner_body }) = &inner_binding.value
    else {
        panic!("slot exists body is the presence read: {inner_binding:?}");
    };
    assert_eq!(inner_binding.name, "slot");
    assert_eq!(inner_body.admitted().cloned(), Some(presence_node.key()));
    assert!(check(
        &document,
        &format!("{UNITS}{DENSE}urgent U when {{ present(self.q) }};\n"),
    )
    .is_err());
}

/// SR-1763001 FND-003 (TC-685, STD-526 PR-03): over `q:
/// Sequence<Option<Clock>>[0, 2]`, a quantifier whose body reads
/// `present(e)` other than as the whole body (`not present(e)`, `present(e)
/// and value(e) >= 1 ms`) reads no clock outside a constraint, and this
/// slice does not read it yet: it refuses
/// `unsupported_construct`/`not-yet-implemented` at the quantifier, in a
/// guard, in a time invariant's `when` and in a postcondition alike, where
/// it is a requirement and never a write (FSpec FR-415-AC-21).
#[trace("TC-685")]
#[test]
fn another_quantified_presence_body_is_not_yet_implemented() {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    let mut q = field("q", "ix://filament/native/Clock");
    q["presence"] = json!("optional");
    q["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(q);
    let document = serde_json::to_vec(&document).expect("JSON");
    for (form, query) in [
        (
            "pre G using v on Config::Call::bump {{ {query} }}\n",
            "exists(e in self.q : not present(e))",
        ),
        (
            "pre G using v on Config::Call::bump {{ {query} }}\n",
            "forall(e in self.q : present(e) and value(e) >= 1 ms)",
        ),
        (
            "time invariant All when {{ {query} }} {{ self.x <= 5 ms }}\n",
            "exists(e in self.q : not present(e))",
        ),
        (
            "post P using v on Config::Call::bump {{ {query} }}\n",
            "exists(e in self.q : not present(e))",
        ),
    ] {
        let body = form
            .replace("{{", "{")
            .replace("}}", "}")
            .replace("{query}", query);
        let (cause, text) = refused(&document, &format!("{UNITS}{DENSE}{body}"));
        assert_eq!(
            (cause.code(), cause.tag(), text.as_str()),
            (Code::UnsupportedConstruct, "not-yet-implemented", query),
            "{body}"
        );
    }
}

/// SR-1743001 CR FND-002 (TC-685, FSpec FR-415-AC-21): a reset of a
/// `Clock` parameter whose range holds the place writes it for the instance
/// binding the parameter there. Over `bump(c: Clock)` with a frame naming
/// `v: Option<Clock>` and `x`, `pre { not present(self.v) }` and `post { c
/// = 0 ms and present(self.v) }` check: the instance binding `c` to `v`
/// writes `v`, so no `clock-presence-unwritten` is decided statically.
#[trace("TC-685")]
#[test]
fn a_clock_parameter_reset_ranging_over_the_place_writes_it() {
    let clock = "ix://filament/native/Clock";
    let mut document: Value =
        serde_json::from_slice(&clock_parameter_document(clock, None, &["v", "x"])).expect("JSON");
    let mut v = field("v", clock);
    v["presence"] = json!("optional");
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(v);
    let document = serde_json::to_vec(&document).expect("JSON");
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}\
             pre Off using v on Config::Call::bump {{ not present(self.v) }}\n\
             post On using v on Config::Call::bump {{ c = 0 ms and present(self.v) }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("c ranges over v: {refusals:?}"));
    let model = timed(&graph);
    let [range] = model.clock_parameters() else {
        panic!("one clock parameter: {:?}", model.clock_parameters());
    };
    let mut names: Vec<&str> = range
        .range()
        .iter()
        .map(|clock| model.clock(*clock).expect("a clock").name())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["v", "x"]);
}

/// A clock-place reference lowers to the place template and its owner; a
/// field that is no place still refuses `ill_typed`/`type-mismatch` at `ref`.
#[trace("TC-685")]
#[test]
fn a_ref_names_a_clock_place_of_its_owner() {
    use fsl_semantics::check::CheckCause;
    let document = rpc_document_with(&["x"], false);
    let body = |term: &str| {
        format!(
            "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
             pre Named using v on Config::Call::reply {{ accept({term}) }}\n"
        )
    };
    let graph = check(&document, &body("ref(self.x)"))
        .unwrap_or_else(|refusals| panic!("clock-place references check: {refusals:?}"));
    let reads: Vec<_> = graph
        .semantic_graph()
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.clock_reference.ref" =>
            {
                Some(application)
            }
            _ => None,
        })
        .collect();
    assert_eq!(reads.len(), 1, "one shared place-reference node");
    let [MemberTerm::Leaf(LeafTerm::Reference { target: place }), MemberTerm::Leaf(LeafTerm::Reference { target: owner })] =
        reads[0].arguments.as_slice()
    else {
        panic!(
            "place reference carries template then owner: {:?}",
            reads[0]
        );
    };
    let place = graph
        .semantic_graph()
        .node(place.admitted().expect("place key is admitted").clone())
        .expect("place template exists");
    assert_eq!(place.semantic_form(), "clock_place");
    let owner = graph
        .semantic_graph()
        .node(owner.admitted().expect("owner key is admitted").clone())
        .expect("owner node exists");
    assert_eq!(owner.semantic_form(), "parameter");

    let refusal = |term: &str| {
        let body = format!(
            "{UNITS}{DENSE}pre Named using v on Config::Call::reply {{ {term} = {term} }}\n"
        );
        let refusals = check(&document, &body).expect_err("the ref is refused");
        let [refusal] = refusals.as_slice() else {
            panic!("one refusal expected: {refusals:?}");
        };
        refusal.clone()
    };
    // The comparison is the clause body's root, and the left `ref` its first
    // operand: the refusal is located at the `ref`.
    let at_ref = |refusal: &fsl_semantics::check::CheckRefusal| {
        assert_eq!(refusal.location.path(), vec![0], "{refusal:?}");
    };
    let missing = refusal("ref(self.nothing)");
    assert!(
        matches!(
            &missing.cause,
            CheckCause::IllTyped(filament_exact::IllTypedCause::TypeMismatch)
        ),
        "{missing:?}"
    );
    at_ref(&missing);
    for term in ["ref(self.nothing)", "ref(self.nothing.x)"] {
        let named = refusal(term);
        assert!(
            matches!(
                &named.cause,
                CheckCause::IllTyped(filament_exact::IllTypedCause::TypeMismatch)
            ),
            "{term}: {named:?}"
        );
        at_ref(&named);
    }
}

/// FR-1040-AC-7: `Reference<Clock>` is a first-class pure function type, and
/// its arguments lower through the native clock-reference scalar type.
#[trace("TC-3084", "FR-1040-AC-7")]
#[test]
fn clock_references_are_function_values() {
    let document = timer_sequence_and_map();
    let body = format!(
        "{UNITS}{DENSE}function same using v(a: Reference<Clock>, b: Reference<Clock>): Boolean pure {{ a = b }}\n\
         pre Named using v on Config::Call::reply {{ size(self.ts) >= 2 and \
         same(ref(self.ts[0].x), ref(self.ts[1].x)) }}\n"
    );
    let graph = check(&document, &body)
        .unwrap_or_else(|refusals| panic!("clock references flow through functions: {refusals:?}"));
    let semantic = graph.semantic_graph();
    let place_references = semantic
        .nodes()
        .filter(|node| {
            matches!(
                node.body(),
                BodyTerm::Application(application)
                    if application.operation.identity() == "filament.op.clock_reference.ref"
            )
        })
        .count();
    assert_eq!(place_references, 2);
    assert!(semantic.nodes().any(|node| {
        node.node_tag() == NodeTag::ScalarType && node.semantic_form() == "clock_reference"
    }));
}

/// FR-1040-AC-2: after `self.peer`, the reference field itself is the place
/// owner; the remaining `x` path resolves against its target object type.
#[trace("TC-3084", "FR-1040-AC-2")]
#[test]
fn a_ref_crosses_an_object_reference_field() {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(field("peer", &package("Call")));
    let document = serde_json::to_vec(&document).expect("JSON");
    let body = format!(
        "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
         pre Named using v on Config::Call::reply {{ accept(ref(self.peer.x)) }}\n"
    );
    let graph = check(&document, &body)
        .unwrap_or_else(|refusals| panic!("reference-owner place checks: {refusals:?}"));
    let reads: Vec<_> = graph
        .semantic_graph()
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.clock_reference.ref" =>
            {
                Some(application)
            }
            _ => None,
        })
        .collect();
    assert_eq!(reads.len(), 1, "equal place-reference terms share one node");
    let [MemberTerm::Leaf(LeafTerm::Reference { target: place }), MemberTerm::Leaf(LeafTerm::Reference { target: owner })] =
        reads[0].arguments.as_slice()
    else {
        panic!(
            "place reference carries template then owner: {:?}",
            reads[0]
        );
    };
    assert_eq!(
        graph
            .semantic_graph()
            .node(place.admitted().expect("place is admitted").clone())
            .expect("place template exists")
            .semantic_form(),
        "clock_place"
    );
    let owner = graph
        .semantic_graph()
        .node(owner.admitted().expect("owner is admitted").clone())
        .expect("owner term exists");
    let BodyTerm::Application(owner_read) = owner.body() else {
        panic!("the owner is the peer field read itself: {owner:?}");
    };
    assert_eq!(
        owner_read.operation.identity(),
        "filament.op.record.project"
    );
    let Some(fsl_semantics::value::Member::Field { name, .. }) = owner_read.operation.member()
    else {
        panic!("owner read selects a field: {owner_read:?}");
    };
    assert_eq!(name.as_str(), "peer");

    let mut wrong_receiver: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    wrong_receiver["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(field("peer", "ix://filament/native/Boolean"));
    let wrong_receiver = serde_json::to_vec(&wrong_receiver).expect("JSON");
    let source = format!(
        "{UNITS}{DENSE}pre Named using v on Config::Call::reply {{ \
         ref(self.peer.x) = ref(self.peer.x) }}\n"
    );
    let refusals = check(&wrong_receiver, &source)
        .expect_err("a non-reference peer cannot own the target's clock place");
    let [refusal] = refusals.as_slice() else {
        panic!("one wrong receiver refusal: {refusals:?}");
    };
    assert!(matches!(
        &refusal.cause,
        fsl_semantics::check::CheckCause::IllTyped(filament_exact::IllTypedCause::TypeMismatch)
    ));
    assert_eq!(refusal.location.path(), vec![0], "refusal is at the ref");
}

/// `Reference<Clock>` uses the native `clock_reference` scalar type in every
/// source type position.
#[trace("TC-685")]
#[test]
fn a_reference_to_clock_is_admitted_in_every_type_position() {
    let document = rpc_document_with(&["x"], false);
    for (position, declaration) in [
        (
            "parameter",
            "function f using v(r: Reference<Clock>): Boolean pure { true }",
        ),
        (
            "result",
            "function f using v(r: Reference<Clock>): Reference<Clock> pure { r }",
        ),
        ("tuple component", "tuple Pair(Reference<Clock>, Boolean);"),
        (
            "union payload",
            "union Holder { Empty, Has(Reference<Clock>) }",
        ),
        (
            "optional",
            "function f using v(r: Option<Reference<Clock>>): Boolean pure { true }",
        ),
    ] {
        let body = format!("{UNITS}{DENSE}{declaration}\n");
        let checked = match crate::model_operations::admit_and_assemble_with_profiles(
            &document,
            crate::timed_model::TEMPORAL,
            &body,
        ) {
            Err(error) => panic!("{position}: Reference<Clock> intake: {error:?}"),
            Ok(declarations) => declarations
                .check(
                    CheckMode::Linked,
                    filament_semantic_value::checking::CheckingLimits::default(),
                    &crate::reward_evaluator::ZeroRewardEvaluator,
                )
                .unwrap_or_else(|refusals| {
                    panic!("{position}: Reference<Clock> checks: {refusals:?}")
                }),
        };
        assert!(checked.semantic_graph().nodes().any(|node| {
            node.node_tag() == NodeTag::ScalarType && node.semantic_form() == "clock_reference"
        }));
    }
}

/// A `Call` with `ts: Sequence<Timer>[0, 3]` and `m`, a package map type from
/// `Text` to `Timer`.
fn timer_sequence_and_map() -> Vec<u8> {
    timer_document(|document: &mut Value| {
        let types = document["types"].as_array_mut().expect("types");
        let mut map = wire_type(
            &package("TimerMap"),
            "unused",
            json!({"keys": "ix://filament/native/Text", "values": package("Timer"), "bound": {"lower": 0}}),
        );
        map["kind"] = json!("map");
        types.push(map);
        let mut ts = field("ts", &package("Timer"));
        ts["multiplicity"] = json!({"lower": 0, "upper": 3, "ordered": true, "unique": false});
        let fields = types[1]["fields"].as_array_mut().expect("Call fields");
        fields.push(ts);
        fields.push(field("m", &package("TimerMap")));
    })
}

/// A `Call` with `q: Sequence<Clock>[0, 2]` for direct slot-reference
/// definedness checks.
fn timer_clock_sequence() -> Vec<u8> {
    timer_document(|document: &mut Value| {
        let mut q = field("q", "ix://filament/native/Clock");
        q["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
        document["types"][1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(q);
        for operation in document["types"][1]["operations"]
            .as_array_mut()
            .expect("operations")
        {
            if matches!(operation["name"].as_str(), Some("send" | "bump")) {
                operation["frame"]["modifies"]
                    .as_array_mut()
                    .expect("frame")
                    .push(json!(call("q")));
            }
        }
    })
}

/// A `Call` with `mo: Map<Text, Clock>[0, 2]` to exercise the
/// map-key domain of a slot-presence quantifier. Map membership itself is
/// the optional presence of each clock place.
fn timer_map_clock_document() -> Vec<u8> {
    timer_document(|document: &mut Value| {
        let map = package("ClockMap");
        let mut map_type = wire_type(
            &map,
            "unused",
            json!({
                "keys": "ix://filament/native/Text",
                "values": "ix://filament/native/Clock",
                "bound": {"lower": 0, "upper": 2},
            }),
        );
        map_type["kind"] = json!("map");
        let types = document["types"].as_array_mut().expect("types");
        types.push(map_type);
        types[1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(field("mo", &map));
    })
}

/// The selectors of a `ref`'s place (FR-230 "Selection"): a `Sequence` slot
/// takes an `Integer` index and requires the selected slot to be live; a place
/// under a `Map` value has no place id.
#[trace("TC-685")]
#[test]
fn a_ref_selects_a_slot_by_index_and_no_map_place_has_an_id() {
    use filament_exact::IllTypedCause;
    use fsl_semantics::check::CheckCause;
    let document = timer_sequence_and_map();
    let refusal = |term: &str| {
        let body = format!(
            "{UNITS}{DENSE}pre Named using v on Config::Call::reply {{ {term} = {term} }}\n"
        );
        let refusals = check(&document, &body).expect_err("the ref is refused");
        let [refusal] = refusals.as_slice() else {
            panic!("one refusal expected: {refusals:?}");
        };
        refusal.clone()
    };
    let body = format!(
        "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
         pre Named using v on Config::Call::reply {{ size(self.ts) >= 1 and accept(ref(self.ts[0].x)) }}\n"
    );
    check(&document, &body)
        .unwrap_or_else(|refusals| panic!("sequence place reference checks: {refusals:?}"));
    let text_index = refusal("ref(self.ts[\"a\"].x)");
    assert_eq!(
        text_index.cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
    assert_eq!(text_index.location.path(), vec![0, 0, 0, 1]);
    let keyed = refusal("ref(self.m[\"a\"].x)");
    assert_eq!(
        keyed.cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
    assert_eq!(keyed.location.path(), vec![0]);
    // A longer path that names no place is a type mismatch even when it
    // passes through a reference field or sequence selector.
    for term in ["ref(self.ts[0])", "ref(self.r.ts[0].x)", "ref(self.r.x)"] {
        let longer = refusal(term);
        assert!(
            matches!(
                &longer.cause,
                CheckCause::IllTyped(IllTypedCause::TypeMismatch)
            ),
            "{term}: {longer:?}"
        );
        assert_eq!(longer.location.path(), vec![0], "{term}");
    }
}

/// FR-1040-AC-5: a clock-place reference through a possibly empty sequence
/// requires the selected slot to be present, located at its selector.
#[trace("TC-3084", "FR-1040-AC-1", "FR-1040-AC-3", "FR-1040-AC-5")]
#[test]
fn a_ref_sequence_selector_requires_a_presence_proof() {
    use fsl_semantics::check::{CheckCause, Obligation};
    let document = timer_sequence_and_map();
    let body = format!(
        "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
         pre Named using v on Config::Call::reply {{ accept(ref(self.ts[0].x)) }}\n"
    );
    let refusals = check(&document, &body).expect_err("the selected sequence slot may be absent");
    let [refusal] = refusals.as_slice() else {
        panic!("one selector refusal: {refusals:?}");
    };
    assert!(
        matches!(refusal.cause, CheckCause::Unproved(Obligation::Presence)),
        "{refusal:?}"
    );
    assert_eq!(refusal.location.path(), vec![0, 0, 0, 1], "{refusal:?}");

    let guarded = body.replace(
        "accept(ref(self.ts[0].x))",
        "size(self.ts) >= 1 and accept(ref(self.ts[0].x))",
    );
    check(&document, &guarded).expect("the size premise proves slot zero is live");

    let direct_sequence = timer_clock_sequence();
    let population = format!(
        "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
         pre Named using v on Config::Call::reply {{ forall(c in Config::calls : \
         size(c.q) >= 1 implies accept(ref(c.q[0]))) }}\n"
    );
    check(&direct_sequence, &population).unwrap_or_else(|refusals| {
        panic!("the population size premise proves its slot: {refusals:?}")
    });
    let unguarded_population = population.replace("size(c.q) >= 1 implies ", "");
    let refusals = check(&direct_sequence, &unguarded_population)
        .expect_err("the population slot has no size proof");
    assert!(
        refusals
            .iter()
            .any(|refusal| matches!(refusal.cause, CheckCause::Unproved(Obligation::Presence))),
        "the population selector carries its presence obligation: {refusals:?}"
    );

    let optional = option_clock_document();
    let optional_reference = format!(
        "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
         pre Named using v on Config::Call::reply {{ accept(ref(self.v)) }}\n"
    );
    check(&optional, &optional_reference)
        .expect("a reference to the optional clock place needs no presence proof");

    let direct_sequence = timer_clock_sequence();
    let direct = format!(
        "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
         pre Named using v on Config::Call::reply {{ accept(ref(self.q[0])) }}\n"
    );
    let refusals = check(&direct_sequence, &direct)
        .expect_err("the direct selected sequence slot may be absent");
    let [refusal] = refusals.as_slice() else {
        panic!("one direct selector refusal: {refusals:?}");
    };
    assert!(
        matches!(refusal.cause, CheckCause::Unproved(Obligation::Presence)),
        "{refusal:?}"
    );
    let positive = direct.replace(
        "accept(ref(self.q[0]))",
        "size(self.q) >= 1 and accept(ref(self.q[0]))",
    );
    check(&direct_sequence, &positive)
        .expect("the size premise proves the direct sequence slot is live");
}

/// `document` with the operation `pick` of `Call`, taking `parameters`
/// (name, type reference) and writing nothing.
fn with_pick(document: &[u8], parameters: &[(&str, &str)]) -> Vec<u8> {
    let mut document: Value = serde_json::from_slice(document).expect("JSON");
    let mut pick = crate::timed_model::call_operation("pick", &[]);
    pick["params"] = Value::Array(
        parameters
            .iter()
            .map(|(name, type_ref)| {
                let mut parameter =
                    crate::model_operations::operation_parameter("pick", name, type_ref);
                let identity = call(&format!("pick/{name}"));
                parameter["origin"]["generated"]["generatorIdentity"] = json!(identity);
                parameter["origin"]["generated"]["inputIdentities"] = json!([identity]);
                parameter["identity"] = json!(identity);
                parameter
            })
            .collect(),
    );
    document["types"][1]["operations"]
        .as_array_mut()
        .expect("operations")
        .push(pick);
    serde_json::to_vec(&document).expect("JSON")
}

/// [`timer_clock_sequence`] with `pick(i)` of a bounded `VersionNumber`.
fn timer_clock_sequence_with_pick() -> Vec<u8> {
    let (_, _, number) = version_number_value_type();
    with_pick(&timer_clock_sequence(), &[("i", &number)])
}

/// FR-1040-AC-5, population half: a clock reference selected by an operation
/// parameter through a population binder needs `i < size(c.q)` in the same
/// `where`, and refuses `unproved-presence` at the selector without it.
#[trace("TC-3084", "FR-1040-AC-5")]
#[test]
fn a_population_ref_selected_by_a_parameter_requires_the_size_premise() {
    use fsl_semantics::check::{CheckCause, Obligation};
    let document = timer_clock_sequence_with_pick();
    let body = |premise: &str| {
        format!(
            "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
             pre P using v on Config::Call::pick {{ count(c in Config::calls where \
             {premise}accept(ref(c.q[i]))) >= 0 }}\n"
        )
    };
    check(&document, &body("i < size(c.q) and "))
        .unwrap_or_else(|refusals| panic!("the size premise proves the slot: {refusals:?}"));
    let refusals = check(&document, &body("")).expect_err("the slot may be absent");
    let [refusal] = refusals.as_slice() else {
        panic!("one selector refusal: {refusals:?}");
    };
    assert!(
        matches!(refusal.cause, CheckCause::Unproved(Obligation::Presence)),
        "{refusal:?}"
    );
    assert_eq!(
        refusal.location.path().last(),
        Some(&1),
        "located at the selector `i`, the index operand of the selection: {refusal:?}"
    );
}

/// A declared type is checked whole: a set of maps is no admitted collection
/// element, whatever its leaves are.
#[trace("TC-685")]
#[test]
fn a_set_of_maps_is_refused_as_a_declared_type() {
    use filament_exact::IllTypedCause;
    use fsl_semantics::check::CheckCause;
    let body = format!(
        "{UNITS}{DENSE}function f using v(s: Set<Map<Text[0, 8; nfc], Float64>[0, 2]>[0, 2]): Boolean pure {{ true }}\n"
    );
    let refusals = check(&timer_document(|_| {}), &body).expect_err("Set<Map<..>> is refused");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::IllTyped(IllTypedCause::OperatorIneligible)
        )),
        "{refusals:?}"
    );
}

/// FR-1040-AC-3: a population `count` and `only` over `ref(c.x)` check in a
/// precondition and in a state invariant, and lower with the binder `c`
/// itself as the owner term of the `clock_reference.ref` application.
#[trace("TC-3084", "FR-1040-AC-3")]
#[test]
fn a_population_ref_lowers_with_the_binder_as_owner() {
    let document = timer_sequence_and_map();
    let predicates = [
        "count(c in Config::calls where ref(c.x) = ref(self.x)) >= 0",
        "only(c in Config::calls where c.n = 2: ref(c.x)) = ref(self.x)",
    ];
    let clauses = [
        "pre P using v on Config::Call::reply { PRED }\n",
        "invariant I using v on Config::Call at current { PRED }\n",
    ];
    for clause in clauses {
        for predicate in predicates {
            let body = format!("{UNITS}{DENSE}{}", clause.replace("PRED", predicate));
            let graph = check(&document, &body)
                .unwrap_or_else(|refusals| panic!("{predicate}: {refusals:?}"));
            let refs = applications_of(&graph, "filament.op.clock_reference.ref");
            assert!(
                refs.iter().any(|arguments| {
                    matches!(arguments.as_slice(), [_, owner, ..]
                        if form_of(&graph, owner) == "parameter"
                            && parameter_name(&graph, owner) == "c")
                }),
                "{predicate}: a ref owned by the binder c in {refs:?}"
            );
        }
    }
}

/// FR-1040-AC-3 (refinement-row form), FR-230 "Clock references": in a
/// refinement row over `calls`, `only(c in C::calls where c.n = 1: ref(c.x))`
/// and `count(c in C::calls where ref(c.x) = ref(self.x)) >= 0` check and lower
/// with the binder `c`, typed `Reference<Call>`, as the owner term of the
/// `clock_reference.ref` application, whose place is the `clock_place` node
/// the timed model lists for `Call.x`.
#[trace("TC-3084", "FR-1040-AC-3")]
#[test]
fn a_refinement_row_ref_lowers_with_the_binder_as_owner() {
    use ring_queue::{check_clock_refinement, clock_refinement};
    for row in [
        "busy = only(c in C::calls where c.n = 1: ref(c.x)) = ref(self.x);",
        "busy = count(c in C::calls where ref(c.x) = ref(self.x)) >= 0;",
    ] {
        let graph = check_clock_refinement(&clock_refinement(row))
            .unwrap_or_else(|refusals| {
                panic!(
                    "{row}: {:?}",
                    refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
                )
            })
            .graph;
        let timed = graph.timed_model("C").expect("the timed model");
        assert_eq!(
            timed.clocks().len(),
            1,
            "{row}: the member's one clock place"
        );
        let refs = applications_of(&graph, "filament.op.clock_reference.ref");
        let owned_by_binder = refs.iter().any(|arguments| {
            let [place, owner, ..] = arguments.as_slice() else {
                return false;
            };
            let MemberTerm::Leaf(LeafTerm::Reference { target }) = place else {
                return false;
            };
            let semantic = graph.semantic_graph();
            let binder_type = match owner {
                MemberTerm::Leaf(LeafTerm::Reference { target }) => semantic
                    .node(target.admitted().expect("admitted owner").clone())
                    .and_then(|node| node.semantic_type())
                    .and_then(|ty| semantic.node(ty)),
                _ => None,
            };
            form_of(&graph, place) == "clock_place"
                && target
                    .admitted()
                    .is_some_and(|key| timed.place_nodes().contains(key))
                && form_of(&graph, owner) == "parameter"
                && parameter_name(&graph, owner) == "c"
                && binder_type.is_some_and(|ty| {
                    if ty.node_tag() != NodeTag::CompositeType || ty.semantic_form() != "reference"
                    {
                        return false;
                    }
                    let BodyTerm::Aggregate(body) = ty.body() else {
                        return false;
                    };
                    let [MemberTerm::Leaf(LeafTerm::Reference { target })] =
                        body.members.as_slice()
                    else {
                        return false;
                    };
                    target.admitted().is_some_and(|key| {
                        graph.resolve_declaration(key.clone())
                            == Some(&ring_queue::key(ring_queue::CLOCKS, "Call"))
                    })
                })
        });
        assert!(
            owned_by_binder,
            "{row}: a ref of the timed model's place owned by the binder c in {refs:?}"
        );
    }
}

/// FR-1040-AC-6 (refinement-row form), FR-430 "Timed refinement": in a
/// refinement row, `sum(c in C::calls: c.x) >= 0` refuses
/// `unsupported_construct`/`expression-form` at `c.x`.
#[trace("TC-3084", "FR-1040-AC-6")]
#[test]
fn a_refinement_row_clock_read_is_refused() {
    use fsl_semantics::check::{CheckCause, RefinementCause};
    use ring_queue::{check_clock_refinement, clock_refinement};
    let row = "busy = sum(c in C::calls: c.x) >= 0;";
    let Err(refusals) = check_clock_refinement(&clock_refinement(row)) else {
        panic!("the row reads a clock");
    };
    let clock_reads: Vec<_> = refusals
        .iter()
        .filter(|found| {
            matches!(&found.refusal.cause, CheckCause::Refinement(refinement)
                if matches!(refinement.cause, RefinementCause::ClockRead))
        })
        .collect();
    let [found] = clock_reads.as_slice() else {
        panic!(
            "one clock-read refusal: {:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        );
    };
    assert_eq!(
        (found.refusal.cause.code(), found.refusal.cause.cause()),
        (Code::UnsupportedConstruct, Some("expression-form"))
    );
    assert_eq!(found.at, "c.x");
}

/// FR-1040-AC-6: the negative population forms. A clock read as the summand
/// or the `only` result is outside a constraint; a reference summed, a
/// selection through a map value, and an ordering of two references are
/// ill-typed.
#[trace("TC-3084", "FR-1040-AC-6")]
#[test]
fn population_clock_reads_and_ill_typed_refs_are_refused() {
    use filament_exact::IllTypedCause;
    use fsl_semantics::check::{CheckCause, TemporalCause};
    let document = with_pick(&timer_sequence_and_map(), &[]);
    let refusals_of = |clause: &str| {
        let body = format!(
            "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
             pre P using v on Config::Call::pick {{ {clause} }}\n"
        );
        check(&document, &body).expect_err(clause)
    };
    for clause in [
        "sum(c in Config::calls: c.x) >= 0",
        "only(c in Config::calls: c.x) >= 1 ms",
    ] {
        let refusals = refusals_of(clause);
        assert!(
            refusals.iter().any(|refusal| matches!(
                &refusal.cause,
                CheckCause::Temporal(cause)
                    if matches!(**cause, TemporalCause::ClockReadOutsideConstraint { .. })
            )),
            "{clause}: {refusals:?}"
        );
    }
    for (clause, cause) in [
        (
            "sum(c in Config::calls: ref(c.x)) >= 0",
            IllTypedCause::TypeMismatch,
        ),
        (
            "count(c in Config::calls where accept(ref(c.m[\"a\"].x))) >= 0",
            IllTypedCause::TypeMismatch,
        ),
        (
            "count(c in Config::calls where ref(c.x) < ref(c.t.x)) >= 0",
            IllTypedCause::OperatorIneligible,
        ),
    ] {
        let refusals = refusals_of(clause);
        assert!(
            refusals.iter().any(
                |refusal| matches!(&refusal.cause, CheckCause::IllTyped(found) if *found == cause)
            ),
            "{clause}: {refusals:?}"
        );
    }
}

/// A clock read inside a `ref` selector is no place path: `c.x` there is read
/// as a field of `c`, which a clock is not, so the typer refuses it as an
/// unknown field before any clock constraint is looked for.
#[trace("TC-3084", "FR-1040-AC-6")]
#[test]
fn a_clock_read_in_a_ref_selector_is_an_unknown_field() {
    use fsl_semantics::check::CheckCause;
    let document = timer_clock_sequence();
    let body = format!(
        "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
         pre P using v on Config::Call::reply {{ count(c in Config::calls where \
         accept(ref(c.q[if c.x >= 1 ms then 0 else 1]))) >= 0 }}\n"
    );
    let refusals = check(&document, &body).expect_err("the selector reads a clock");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::StateModel(model) = &refusal.cause else {
        panic!("a state-model refusal: {refusal:?}");
    };
    assert!(
        matches!(&model.cause, CheckCause::UnknownField { owner, field, .. }
            if owner.ends_with("Call") && field.ends_with("x")),
        "{refusal:?}"
    );
    assert_eq!(refusal.location.path().len(), 7, "{refusal:?}");
}

/// FR-125-AC-17: a value-family function reads no field through a reference,
/// neither as an intermediate field of a `ref` path nor as the clock
/// collection under `size`.
#[trace("TC-1031", "FR-125-AC-17")]
#[test]
fn a_function_body_reads_no_reference_field_through_ref_or_size() {
    use filament_exact::IllTypedCause;
    use fsl_semantics::check::CheckCause;
    let mut peer: Value = serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    peer["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(field("peer", &package("Call")));
    let peer = serde_json::to_vec(&peer).expect("JSON");
    for (document, function) in [
        (
            peer,
            "function g using v(o: Reference<Config::Call>): Reference<Clock> pure { ref(o.peer.x) }",
        ),
        (
            timer_clock_sequence(),
            "function h using v(o: Reference<Config::Call>): Integer pure { size(o.q) }",
        ),
    ] {
        let refusals = check(&document, &format!("{UNITS}{DENSE}{function}\n"))
            .expect_err("a bare field read through a reference");
        assert!(
            refusals.iter().any(|refusal| matches!(
                refusal.cause,
                CheckCause::IllTyped(IllTypedCause::TypeMismatch)
            )),
            "{function}: {refusals:?}"
        );
    }
}

/// `ref` names its place by the template of that place: with the places `x`,
/// `ts[..].x` and `m[..].x` in one model, each reference carries the template
/// of its own path.
#[trace("TC-3084", "FR-1040-AC-1")]
#[test]
fn a_ref_names_the_template_of_its_own_place() {
    let document = timer_sequence_and_map();
    let body = format!(
        "{UNITS}{DENSE}function same using v(a: Reference<Clock>, b: Reference<Clock>): Boolean pure {{ a = b }}\n\
         pre Named using v on Config::Call::reply {{ size(self.ts) >= 1 and \
         same(ref(self.ts[0].x), ref(self.x)) }}\n"
    );
    let graph = check(&document, &body).unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let refs = applications_of(&graph, "filament.op.clock_reference.ref");
    assert_eq!(refs.len(), 2, "{refs:?}");
    for arguments in refs {
        let MemberTerm::Leaf(LeafTerm::Reference { target }) = &arguments[0] else {
            panic!("the template comes first: {arguments:?}");
        };
        let place = graph
            .semantic_graph()
            .node(target.admitted().expect("admitted template").clone())
            .expect("template node exists");
        let BodyTerm::Aggregate(body) = place.body() else {
            panic!("a clock place is an aggregate: {place:?}");
        };
        let path_steps = body
            .members
            .iter()
            .find_map(|member| match member {
                MemberTerm::Binding(binding) if binding.name == "path" => match &binding.value {
                    BindingValue::Tuple(tuple) => Some(tuple.members.len()),
                    _ => None,
                },
                _ => None,
            })
            .expect("the place carries its path");
        // `ts[..].x` is the field, its slot and the clock; `x` the clock alone,
        // and only a selected place carries a slot selector argument.
        match arguments.len() {
            3 => assert_eq!(path_steps, 3, "{arguments:?}"),
            2 => assert_eq!(path_steps, 1, "{arguments:?}"),
            other => panic!("{other} arguments: {arguments:?}"),
        }
    }
}

/// `size` of a clock collection is the size of its slot domain: the
/// `option.slots` application over the place template and the owner, not a
/// record projection.
#[trace("TC-3084", "FR-1040-AC-5")]
#[test]
fn size_of_a_clock_collection_lowers_to_its_slot_domain() {
    let document = timer_clock_sequence();
    let body = format!(
        "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
         pre Named using v on Config::Call::reply {{ size(self.q) >= 1 and \
         accept(ref(self.q[0])) }}\n"
    );
    let graph = check(&document, &body).unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let domains = applications_of(&graph, "filament.op.option.slots");
    let [domain] = domains.as_slice() else {
        panic!("one slot domain: {domains:?}");
    };
    let [place, _owner] = domain.as_slice() else {
        panic!("the template then the owner: {domain:?}");
    };
    assert_eq!(form_of(&graph, place), "clock_place");
    assert!(
        applications_of(&graph, "filament.op.record.project")
            .iter()
            .all(|arguments| arguments.iter().all(|member| !matches!(
                member,
                MemberTerm::Leaf(LeafTerm::Reference { target })
                    if graph
                        .semantic_graph()
                        .node(target.admitted().expect("admitted").clone())
                        .is_some_and(|node| node.semantic_form() == "clock_place")
            ))),
        "the clock collection is projected as no record field"
    );
}

/// FR-953-AC-2: two quantifiers over `self.q` share one slot-domain node,
/// and one over `r` adds a second.
#[trace("TC-2663", "FR-953-AC-2")]
#[test]
fn quantifiers_over_one_collection_share_a_slot_domain() {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    for name in ["q", "r"] {
        let mut place = field(name, "ix://filament/native/Clock");
        place["presence"] = json!("optional");
        place["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
        document["types"][1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(place);
    }
    for operation in document["types"][1]["operations"]
        .as_array_mut()
        .expect("operations")
    {
        if matches!(operation["name"].as_str(), Some("send" | "bump")) {
            for name in ["q", "r"] {
                operation["frame"]["modifies"]
                    .as_array_mut()
                    .expect("frame")
                    .push(json!(call(name)));
            }
        }
    }
    let document = serde_json::to_vec(&document).expect("JSON");
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}time invariant Shared when {{ forall(e in self.q : present(e)) and \
             exists(e in self.q : present(e)) and forall(e in self.r : present(e)) }} \
             {{ self.x <= 5 ms }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let domains = applications_of(&graph, "filament.op.option.slots");
    assert_eq!(domains.len(), 2, "{domains:?}");
}

/// FR-1040-AC-4: `exists` over the population of a `forall` over the slots
/// lowers its slot domain at the binder.
#[trace("TC-3084", "FR-1040-AC-4")]
#[test]
fn a_population_exists_over_slot_forall_lowers_at_the_binder() {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    let mut q = field("q", "ix://filament/native/Clock");
    q["presence"] = json!("optional");
    q["multiplicity"] = json!({"lower": 0, "upper": 2, "ordered": true, "unique": false});
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(q);
    for operation in document["types"][1]["operations"]
        .as_array_mut()
        .expect("operations")
    {
        if matches!(operation["name"].as_str(), Some("send" | "bump")) {
            operation["frame"]["modifies"]
                .as_array_mut()
                .expect("frame")
                .push(json!(call("q")));
        }
    }
    let document = serde_json::to_vec(&document).expect("JSON");
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}time invariant Crowd when {{ \
             exists(c in Config::calls : forall(e in c.q : present(e))) }} {{ self.x <= 5 ms }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let domains = applications_of(&graph, "filament.op.option.slots");
    let [domain] = domains.as_slice() else {
        panic!("one slot domain: {domains:?}");
    };
    assert_eq!(parameter_name(&graph, &domain[1]), "c");
}

/// FR-1040-AC-1: `ref(self.x)` in a postcondition checks, and its
/// `clock_reference.ref` application is typed by the native `clock_reference`
/// scalar.
#[trace("TC-3084", "FR-1040-AC-1")]
#[test]
fn a_ref_in_a_postcondition_is_typed_by_the_clock_reference_scalar() {
    let document = timer_clock_sequence();
    let body = format!(
        "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
         post W using v on Config::Call::bump {{ accept(ref(self.x)) }}\n"
    );
    let graph = check(&document, &body).unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let semantic = graph.semantic_graph();
    let typed: Vec<_> = semantic
        .nodes()
        .filter(|node| {
            matches!(
                node.body(),
                BodyTerm::Application(application)
                    if application.operation.identity() == "filament.op.clock_reference.ref"
            )
        })
        .collect();
    let [reference] = typed.as_slice() else {
        panic!("one place reference: {typed:?}");
    };
    let result = semantic
        .node(reference.semantic_type().expect("a typed application"))
        .expect("result type node");
    assert_eq!(result.node_tag(), NodeTag::ScalarType);
    assert_eq!(result.semantic_form(), "clock_reference");
}

/// FR-1040-AC-2: through an `Option<Reference<Call>>` field the place path is
/// refused `type-mismatch` at the `ref`; `value(self.peer)` under
/// `present(self.peer)` is the owner; and a selection below `self.peer`
/// checks under its size premise.
#[trace("TC-3084", "FR-1040-AC-2")]
#[test]
fn a_ref_through_an_optional_reference_needs_value() {
    use filament_exact::IllTypedCause;
    use fsl_semantics::check::CheckCause;
    let mut document: Value = serde_json::from_slice(&timer_sequence_and_map()).expect("JSON");
    let mut peer = field("peer", &package("Call"));
    peer["presence"] = json!("optional");
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(peer);
    let document = serde_json::to_vec(&document).expect("JSON");
    let clause = |predicate: &str| {
        format!(
            "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
             pre Named using v on Config::Call::reply {{ {predicate} }}\n"
        )
    };
    let refusals = check(&document, &clause("accept(ref(self.peer.x))"))
        .expect_err("an optional reference has no place");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
    assert_eq!(refusal.location.path(), vec![0], "{refusal:?}");
    check(
        &document,
        &clause("present(self.peer) and accept(ref(value(self.peer).x))"),
    )
    .unwrap_or_else(|refusals| panic!("value(self.peer) under present: {refusals:?}"));
}

/// FR-1040-AC-2: a selection below an object reference checks under the
/// size premise read through that reference.
#[trace("TC-3084", "FR-1040-AC-2")]
#[test]
fn a_ref_selects_a_slot_below_an_object_reference() {
    let mut document: Value = serde_json::from_slice(&timer_sequence_and_map()).expect("JSON");
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("Call fields")
        .push(field("peer", &package("Call")));
    let document = serde_json::to_vec(&document).expect("JSON");
    let body = format!(
        "{UNITS}{DENSE}function accept using v(r: Reference<Clock>): Boolean pure {{ true }}\n\
         pre Named using v on Config::Call::reply {{ size(self.peer.ts) >= 1 and \
         accept(ref(self.peer.ts[0].x)) }}\n"
    );
    check(&document, &body).unwrap_or_else(|refusals| panic!("{refusals:?}"));
}

/// FR-1040-AC-7: the parameters of a function over `Reference<Clock>` are
/// typed by the `clock_reference` scalar, and a body made ill-typed is refused
/// once, at the body, not at the call.
#[trace("TC-3084", "FR-1040-AC-7")]
#[test]
fn a_clock_reference_function_has_typed_parameters_and_refuses_at_its_body() {
    let document = timer_sequence_and_map();
    let call = "pre Named using v on Config::Call::reply {{ size(self.ts) >= 2 and \
                same(ref(self.ts[0].x), ref(self.ts[1].x)) }}\n";
    let body = |expression: &str| {
        format!(
            "{UNITS}{DENSE}function same using v(a: Reference<Clock>, b: Reference<Clock>): \
             Boolean pure {{ {expression} }}\n{}",
            call.replace("{{", "{").replace("}}", "}")
        )
    };
    let graph = check(&document, &body("a = b")).unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let semantic = graph.semantic_graph();
    let typed_parameters = semantic
        .nodes()
        .filter(|node| node.semantic_form() == "parameter")
        .filter(|node| {
            node.semantic_type()
                .and_then(|ty| semantic.node(ty))
                .is_some_and(|ty| ty.semantic_form() == "clock_reference")
        })
        .count();
    assert!(
        typed_parameters >= 2,
        "both parameters are typed by clock_reference"
    );
    let refusals = check(&document, &body("a = 1")).expect_err("the body is ill-typed");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(
        format!("{:?}", refusals[0].location).contains("Body { function: \"same\""),
        "refused at the function body: {refusals:?}"
    );
}

/// TC-3084's clock model with a canonical reference definition and a data
/// field writable independently of the clock it names.
fn clock_reference_document(target: &str) -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x", "y"], false)).expect("JSON");
    let mut reference = wire_type(&package("ClockRef"), "unused", json!({"target": target}));
    reference["kind"] = json!("reference");
    document["types"]
        .as_array_mut()
        .expect("types")
        .push(reference);
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("fields")
        .push(field("r", &package("ClockRef")));
    document["types"][1]["operations"]
        .as_array_mut()
        .expect("operations")
        .push(crate::timed_model::call_operation("point", &["r"]));
    serde_json::to_vec(&document).expect("JSON")
}

/// A clock reference is a data write, and the same field can be read in a
/// population condition without reading any clock value.
#[trace("TC-3084", "FR-1040-AC-8", "FR-1040-AC-3")]
#[test]
fn a_clock_reference_field_is_read_and_written_as_data() {
    let document = clock_reference_document("ix://filament/native/Clock");
    let graph = check(&document, &format!("{UNITS}{DENSE}\
        post Point using v on Config::Call::point {{ self.r = ref(self.x) }}\n\
        invariant Count using v on Config::Call at current {{ count(c in Config::calls where ref(c.x) = self.r) >= 0 }}\n"))
        .unwrap_or_else(|refusals| panic!("data read/write and frame: {refusals:?}"));
    assert_eq!(
        timed(&graph).clocks().len(),
        2,
        "r contributes no clock place"
    );
    assert!(
        timed(&graph)
            .resets()
            .iter()
            .all(|resets| resets.resets().is_empty()),
        "point resets no clock"
    );
    assert_eq!(
        applications_of(&graph, "filament.op.clock_reference.ref").len(),
        2
    );
    assert_eq!(
        applications_of(&graph, "filament.op.clock_reference.eq").len(),
        1
    );
}

/// Alias and native-target value definitions keep the reference data leaf;
/// field presence and multiplicity keep their ordinary wrappers.
#[trace("TC-3084", "FR-1040-AC-8", "FSpec-FR-154-AC-53")]
#[test]
fn clock_reference_targets_and_wrappers_preserve_data_types() {
    for value in [false, true] {
        let mut document: Value =
            serde_json::from_slice(&clock_reference_document(&package("Watch"))).expect("JSON");
        let mut watch = wire_type(
            &package("Watch"),
            "clock_value_type",
            json!({"target": "ix://filament/native/Clock"}),
        );
        if !value {
            watch["kind"] = json!("alias");
        }
        document["types"].as_array_mut().expect("types").push(watch);
        if value {
            document["constructs"]
                .as_array_mut()
                .expect("constructs")
                .push(wire_construct(
                    "clock_value_type",
                    meaning::VALUE_TYPE,
                    json!({}),
                ));
        }
        let mut optional = field("optional", &package("ClockRef"));
        optional["presence"] = json!("optional");
        let mut sequence = field("refs", &package("ClockRef"));
        sequence["multiplicity"] =
            json!({"lower": 0, "upper": 3, "ordered": true, "unique": false});
        document["types"][1]["fields"]
            .as_array_mut()
            .expect("fields")
            .extend([optional, sequence]);
        let graph = check(&serde_json::to_vec(&document).expect("JSON"), &format!("{UNITS}{DENSE}\
            invariant Data using v on Config::Call at current {{ present(self.optional) implies value(self.optional) = self.r }}\n\
            invariant Refs using v on Config::Call at current {{ size(self.refs) <= 3 }}\n"))
            .unwrap_or_else(|refusals| panic!("value target {value}: {refusals:?}"));
        assert_eq!(timed(&graph).clocks().len(), 2);
        assert_eq!(
            applications_of(&graph, "filament.op.clock_reference.eq").len(),
            1
        );
    }
}

/// Direct Clock stays a place; using it as a data assignment is a clock write.
#[trace("TC-3084", "FR-1040-AC-8")]
#[test]
fn a_direct_clock_field_keeps_its_clock_write_refusal() {
    let mut document: Value =
        serde_json::from_slice(&clock_reference_document("ix://filament/native/Clock"))
            .expect("JSON");
    let r = document["types"][1]["fields"]
        .as_array_mut()
        .expect("fields")
        .iter_mut()
        .find(|field| field["name"] == "r")
        .expect("r");
    r["typeRef"] = json!("ix://filament/native/Clock");
    r["multiplicity"]["unique"] = json!(false);
    let body = format!(
        "{UNITS}{DENSE}post Point using v on Config::Call::point {{ self.r = ref(self.x) }}\n"
    );
    let refusals = check(&serde_json::to_vec(&document).expect("JSON"), &body)
        .expect_err("a clock is not a data field");
    assert!(
        refusals
            .iter()
            .any(|refusal| refusal.cause.code() == Code::InvalidTimedModel
                && refusal.cause.cause() == Some("clock-write")),
        "{refusals:?}"
    );
}

/// The native set has no ClockReference pseudo-name.
#[trace("TC-3084", "FR-1040-AC-8", "FSpec-FR-154-AC-53")]
#[test]
fn a_clock_reference_native_pseudo_name_is_refused_at_the_field() {
    let mut document: Value =
        serde_json::from_slice(&clock_reference_document("ix://filament/native/Clock"))
            .expect("JSON");
    document["types"][1]["fields"]
        .as_array_mut()
        .expect("fields")
        .iter_mut()
        .find(|field| field["name"] == "r")
        .expect("r")["typeRef"] = json!("ix://filament/native/ClockReference");
    let refusal = crate::model_operations::admit(&serde_json::to_vec(&document).expect("JSON"))
        .expect_err("no native pseudo-name");
    assert_eq!(refusal.cause.code(), Code::InvalidModelBinding);
    assert_eq!(refusal.cause.cause(), Some("malformed-declaration"));
    assert!(
        format!("{refusal:?}").contains("Field.typeRef"),
        "{refusal:?}"
    );
}

/// Native-target constraints preserve their keyword locus and catalog code.
#[trace("TC-3084", "FR-1040-AC-8", "FSpec-FR-154-AC-51")]
#[test]
fn native_target_constraint_refusals_name_the_keyword() {
    for (native, keyword) in [("Clock", "maxLength"), ("Integer", "min")] {
        let watch = package("Watch");
        let mut document: Value =
            serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
        let constraint = json!({
            "identity": format!("{watch}/constraints/{keyword}"), "keyword": keyword,
            "operands": {"value": 5}, "appliesTo": watch,
            "diagnosticCode": "watch.bound", "origin": {"generated": {
                "generatorIdentity": watch, "generatorVersion": "1.0.0", "inputIdentities": [watch],
            }},
        });
        document["types"].as_array_mut().expect("types").push(wire_type(&watch, "clock_value_type", json!({"target": format!("ix://filament/native/{native}"), "constraints": [constraint]})));
        document["constructs"]
            .as_array_mut()
            .expect("constructs")
            .push(wire_construct(
                "clock_value_type",
                meaning::VALUE_TYPE,
                json!({}),
            ));
        let refusal = crate::model_operations::admit(&serde_json::to_vec(&document).expect("JSON"))
            .expect_err("invalid keyword");
        assert_eq!(
            refusal.cause.code(),
            Code::InvalidModelBinding,
            "{native}: {refusal:?}"
        );
        assert_eq!(refusal.cause.cause(), Some("malformed-declaration"));
        assert!(
            format!("{refusal:?}").contains("Constraint.keyword"),
            "{native}: {refusal:?}"
        );
    }
}

/// A model unit writes the canonical reference definition instead of a
/// NativeClockReference pseudo-name or a direct clock field.
#[trace("TC-3084", "FR-1040-AC-8")]
#[test]
fn a_model_unit_emits_a_clock_reference_definition() {
    let source = "language \"ix:native\" edition \"1-draft\"; domain \"acme/rpc\"; object Call { x: Clock; r: Reference<Clock>; }";
    let parsed = fsl_cst::parse(
        fsl_foundation::SourceIdentity::new("test", "clock-reference"),
        "model.fsl",
        source.as_bytes(),
        fsl_cst::Limits::default(),
    )
    .expect("source parses");
    let compiled = fsl_semantics::model::intake::compile_model_unit(
        &parsed,
        fsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("model compiles");
    let document: Value = serde_json::from_slice(compiled.bytes()).expect("JSON");
    let reference = document["types"]
        .as_array()
        .expect("types")
        .iter()
        .find(|ty| ty["identity"] == "ix://acme/rpc/Reference_Call_r")
        .expect("canonical reference definition");
    assert_eq!(reference["kind"], "reference");
    assert_eq!(reference["target"], "ix://filament/native/Clock");
    let call = document["types"]
        .as_array()
        .expect("types")
        .iter()
        .find(|ty| ty["identity"] == "ix://acme/rpc/Call")
        .expect("Call");
    let r = call["fields"]
        .as_array()
        .expect("fields")
        .iter()
        .find(|field| field["name"] == "r")
        .expect("r");
    assert_eq!(r["typeRef"], "ix://acme/rpc/Reference_Call_r");
    crate::model_operations::admit(compiled.bytes()).expect("canonical output admits");
}

/// FR-1040-AC-6: a call to a clock-reading function over a population
/// binder's clock is outside a constraint.
#[trace("TC-3084", "FR-1040-AC-6")]
#[test]
fn a_clock_reading_call_over_a_population_clock_is_refused() {
    use fsl_semantics::check::{CheckCause, TemporalCause};
    let body = format!(
        "{UNITS}{DENSE}predicate late using v(t: Clock): Boolean {{ t >= 1 ms }}\n\
         pre P using v on Config::Call::reply {{ count(c in Config::calls where late(c.x)) >= 0 }}\n"
    );
    let refusals = check(&timer_sequence_and_map(), &body).expect_err("the clock is read");
    assert!(
        refusals.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::Temporal(cause)
                if matches!(**cause, TemporalCause::ClockReadOutsideConstraint { .. })
        )),
        "{refusals:?}"
    );
}

/// FR-953: a map slot-presence quantifier ranges over the map's key type.
#[trace("TC-3084", "FR-953-AC-1")]
#[test]
fn quantified_presence_over_a_clock_map_uses_its_key_domain() {
    let document = timer_map_clock_document();
    let graph = check(
        &document,
        &format!(
            "{UNITS}{DENSE}time invariant P when {{ \
             exists(k in self.mo : present(k)) }} {{ self.x <= 5 ms }}\n"
        ),
    )
    .unwrap_or_else(|refusals| panic!("Map<Text, Clock> slot presence: {refusals:?}"));
    let semantic_graph = graph.semantic_graph();
    let slots: Vec<_> = semantic_graph
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.option.slots" =>
            {
                Some((node, application))
            }
            _ => None,
        })
        .collect();
    let [(slots_node, slots_application)] = slots.as_slice() else {
        panic!("one map slot domain: {slots:?}");
    };
    let slots_type = semantic_graph
        .node(
            slots_node
                .semantic_type()
                .expect("typed option.slots domain"),
        )
        .expect("option.slots domain type exists");
    assert_eq!(slots_type.semantic_form(), "collection_bounds");
    let set_type = semantic_graph
        .node(slots_type.semantic_type().expect("bounded Set<Text> base"))
        .expect("Set<Text> type exists");
    assert_eq!(set_type.semantic_form(), "set");
    let BodyTerm::Aggregate(set_body) = set_type.body() else {
        panic!("the map slot domain is Set<Text>: {set_type:?}");
    };
    let [MemberTerm::Leaf(LeafTerm::Reference { target })] = set_body.members.as_slice() else {
        panic!("Set<Text> has its Text element type: {set_body:?}");
    };
    let text_type = semantic_graph
        .node(target.admitted().expect("admitted Text type").clone())
        .expect("Text type exists");
    assert_eq!(text_type.semantic_form(), "text");

    let presence: Vec<_> = semantic_graph
        .nodes()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(application)
                if application.operation.identity() == "filament.op.option.present" =>
            {
                Some(application)
            }
            _ => None,
        })
        .collect();
    let [presence] = presence.as_slice() else {
        panic!("one slot presence application: {presence:?}");
    };
    let [_, _, selector] = presence.arguments.as_slice() else {
        panic!("option.present has a selector argument: {presence:?}");
    };
    let MemberTerm::Leaf(LeafTerm::Reference { target }) = selector else {
        panic!("option.present selector is a parameter reference: {selector:?}");
    };
    let selector = semantic_graph
        .node(
            target
                .admitted()
                .expect("admitted selector parameter")
                .clone(),
        )
        .expect("selector parameter exists");
    assert_eq!(selector.semantic_form(), "parameter");
    let selector_type = semantic_graph
        .node(selector.semantic_type().expect("typed map selector"))
        .expect("map selector type exists");
    assert_eq!(selector_type.semantic_form(), "text");
}

/// Native clock presence belongs to the map slot, so wrapping its value in
/// `Option<Clock>` is an invalid model declaration at the option's `inner`.
#[trace("TC-3084", "FR-953-AC-1")]
#[test]
fn an_option_clock_map_value_is_refused_at_intake() {
    let mut document: Value = serde_json::from_slice(&timer_map_clock_document()).expect("JSON");
    let option = package("OptionalClock");
    let mut option_type = wire_type(
        &option,
        "option",
        json!({"inner": "ix://filament/native/Clock"}),
    );
    option_type["kind"] = json!("option");
    document["types"]
        .as_array_mut()
        .expect("types")
        .push(option_type);
    let map = document["types"]
        .as_array_mut()
        .expect("types")
        .iter_mut()
        .find(|ty| ty["kind"] == "map")
        .expect("clock map");
    map["values"] = json!(option);
    let document = serde_json::to_vec(&document).expect("JSON");

    let refusal = crate::model_operations::admit(&document)
        .expect_err("Option<Clock> is not a valid map value type");
    assert_eq!(
        refusal.cause.code(),
        Code::InvalidModelBinding,
        "{refusal:?}"
    );
    assert_eq!(
        refusal.cause.cause(),
        Some("malformed-declaration"),
        "{refusal:?}"
    );
    let detail = format!("{refusal:?}");
    assert!(detail.contains("INVALID_OPTION_INNER"), "{detail}");
    assert!(detail.contains("TypeDefinition.inner"), "{detail}");
}

/// A selector outside a `ref` is part of a clock place expression that is not
/// checked yet: clock reads and resets in a timed clause refuse
/// `not-yet-implemented` at the selector, and a data read there is refused as
/// not a timed place. In a function body a clock selector refuses
/// `unsupported-feature`, while a bounded ordinary data read remains valid.
#[trace("TC-685", "FR-1040-AC-6")]
#[test]
fn a_selector_outside_a_ref_is_not_checked_yet() {
    use fsl_semantics::check::{CheckCause, TemporalCause};
    let document = timer_sequence_and_map();
    for clause in [
        "pre Read using v on Config::Call::reply { self.ts[0].x >= 1 ms }",
        "post Reset using v on Config::Call::send { self.ts[1].x = 0 ms }",
        "post Keyed using v on Config::Call::send { self.m[\"k\"].x = 0 ms }",
        "pre Data using v on Config::Call::reply { \
         size(self.ts) >= 1 and self.ts[0].n = 1 }",
    ] {
        let body = format!("{UNITS}{DENSE}{clause}\n");
        let refusals = check(&document, &body).expect_err("the selector is refused");
        let [refusal] = refusals.as_slice() else {
            panic!("{clause}: one refusal expected: {refusals:?}");
        };
        let CheckCause::Temporal(cause) = &refusal.cause else {
            panic!("{clause}: {refusal:?}");
        };
        let TemporalCause::SelectionNotChecked { span } = **cause else {
            panic!("{clause}: {cause:?}");
        };
        let (text, _) = crate::model_operations::config_unit_with_profiles(
            &document,
            crate::timed_model::TEMPORAL,
            &body,
        );
        assert!(
            text[span.start..span.end].contains('['),
            "{clause}: {}",
            &text[span.start..span.end]
        );
    }

    // A function body reading a clock place through a collection selector is
    // refused at the selector until place selection is implemented there.
    let body = format!(
        "{UNITS}{DENSE}function f using v(o: Reference<Config::Call>): Boolean pure {{ o.ts[0].x >= 1 ms }}\n"
    );
    let refusals = check(&document, &body).expect_err("the selector is refused");
    assert!(
        refusals
            .iter()
            .any(|refusal| matches!(refusal.cause, CheckCause::UnsupportedFeature { .. })),
        "{refusals:?}"
    );

    let document = timer_document(|document: &mut Value| {
        let mut ticks = wire_type(
            &package("Ticks"),
            "unused",
            json!({"items": "ix://filament/native/Clock"}),
        );
        ticks["kind"] = json!("sequence");
        document["types"].as_array_mut().expect("types").push(ticks);
        document["types"][1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(field("ks", &package("Ticks")));
    });
    let body = format!(
        "{UNITS}{DENSE}function f using v(o: Reference<Config::Call>): Clock pure {{ o.ks[0] }}\n"
    );
    let refusals =
        check(&document, &body).expect_err("an indexed Clock value is outside a constraint");
    assert!(
        refusals.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::Temporal(cause)
                if matches!(**cause, TemporalCause::NotAPlaceExpression { .. })
        )),
        "{refusals:?}"
    );
}

/// [`rpc_document_with`]`(&["x"])` with the tuple `Slot(Clock, Integer)` and
/// the variant type `Phase { Armed(Clock), Idle }`, held by the `Call` fields
/// `s` and `p`, both written by `bump`.
fn clock_tuple_and_union_document() -> Vec<u8> {
    let mut document: Value =
        serde_json::from_slice(&rpc_document_with(&["x"], false)).expect("JSON");
    let (slot, phase) = (package("Slot"), package("Phase"));
    let origin = |identity: &str| {
        json!({"source": {"sourceIdentity": identity, "path": "clock-members.fspec",
            "startLine": 1, "startColumn": 1}})
    };
    let types = document["types"].as_array_mut().expect("types");
    types.push(wire_type(
        &slot,
        "tuple_type",
        json!({"positions": [
            {"identity": format!("{slot}/p0"), "typeRef": "ix://filament/native/Clock",
             "origin": origin(&format!("{slot}/p0"))},
            {"identity": format!("{slot}/p1"), "typeRef": "ix://filament/native/Integer",
             "origin": origin(&format!("{slot}/p1"))},
        ]}),
    ));
    types.push(wire_type(
        &phase,
        "variant_type",
        json!({"variants": [
            {"identity": package("variant/Phase-Armed"), "name": "Armed",
             "payloadType": "ix://filament/native/Clock",
             "origin": origin(&package("variant/Phase-Armed"))},
            {"identity": package("variant/Phase-Idle"), "name": "Idle",
             "origin": origin(&package("variant/Phase-Idle"))},
        ]}),
    ));
    let call_type = &mut types[1];
    let fields = call_type["fields"].as_array_mut().expect("Call fields");
    fields.push(field("s", &slot));
    fields.push(field("p", &phase));
    for operation in call_type["operations"].as_array_mut().expect("operations") {
        if operation["name"] == "bump" {
            let modifies = operation["frame"]["modifies"]
                .as_array_mut()
                .expect("frame");
            modifies.push(json!(call("s")));
            modifies.push(json!(call("p")));
        }
    }
    let constructs = document["constructs"].as_array_mut().expect("constructs");
    for (name, meaning, shape, members) in [
        (
            "tuple_type",
            meaning::TUPLE_VALUE_TYPE,
            "tuple",
            json!({"positions": "required"}),
        ),
        (
            "variant_type",
            meaning::VARIANT_TYPE,
            "enumeration",
            json!({"variants": "required"}),
        ),
    ] {
        let mut construct = wire_construct(name, meaning, members);
        construct["construct"]["shape"] = json!(shape);
        constructs.push(construct);
    }
    serde_json::to_vec(&document).expect("JSON")
}

/// FR-230-AC-63 and FSpec FR-415-AC-28: a `Clock` tuple position is an
/// unguarded clock place and a `Clock` union case payload a place guarded by
/// its case; both are read only in clock constraints and written by resets,
/// and `=` over a clock-bearing union reads a clock.
#[trace("TC-1860", "FR-230-AC-63")]
#[trace("FSpec-TC-1361", "FSpec-FR-415-AC-28")]
#[trace("FSpec-TC-1361", "FSpec-FR-322-AC-399")]
#[test]
fn a_clock_tuple_position_and_union_case_are_clock_places() {
    let document = clock_tuple_and_union_document();
    let body = format!(
        "{UNITS}{DENSE}\
         pre Armed using v on Config::Call::bump {{ present(self.p.Armed) and value(self.p.Armed) >= 1 ms }}\n\
         pre Slotted using v on Config::Call::reply {{ self.s[0] >= 2 ms }}\n\
         pre Data using v on Config::Call::reply {{ self.s[1] >= 0 }}\n\
         post Arm using v on Config::Call::bump {{ self.p = Config::Phase::Armed(0 ms) }}\n"
    );
    let graph = check(&document, &body).expect("clock tuple and union places check");
    let model = timed(&graph);
    let names: Vec<_> = model
        .clock_fields()
        .map(|(_, field)| field.name().to_owned())
        .collect();
    assert!(names.iter().any(|name| name == "s.0"), "{names:?}");
    assert!(names.iter().any(|name| name == "p.Armed"), "{names:?}");
    assert!(model.clock_fields().all(|(_, field)| !field.presence_bit()));
    assert_eq!(
        model
            .resets()
            .iter()
            .flat_map(|resets| resets.resets())
            .count(),
        1
    );

    let presence = |clause: &str| {
        check(&document, &format!("{UNITS}{DENSE}{clause}"))
            .expect_err("refused")
            .iter()
            .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        presence("pre Late using v on Config::Call::bump { value(self.p.Armed) >= 1 ms }\n"),
        [(Code::UndefinedExpression, Some("unproved-presence"))]
    );
    assert_eq!(
        presence("invariant Same using v on Config::Call at current { self.p = self.p }\n"),
        [(
            Code::InvalidTimedModel,
            Some("clock-read-outside-constraint")
        )]
    );
    assert_eq!(
        presence("invariant Same using v on Config::Call at current { self.s = self.s }\n"),
        [(
            Code::InvalidTimedModel,
            Some("clock-read-outside-constraint")
        )]
    );
}

/// FR-056-AC-64: a value type bound by `target` to
/// `ix://filament/native/Clock` is an alias of the clock type, so a field
/// typed by it is a clock place and its `max` constraint bounds that place.
#[trace("TC-3031", "FR-056-AC-64")]
#[test]
fn a_value_type_bound_to_clock_by_target_types_a_clock_place() {
    let document = timer_document(|document: &mut Value| {
        let watch = package("Watch");
        let constraint = json!({
            "identity": format!("{watch}/constraints/max"),
            "keyword": "max",
            "operands": {"value": 5},
            "appliesTo": watch.clone(),
            "diagnosticCode": "clock.max",
            "origin": {"generated": {
                "generatorIdentity": watch.clone(),
                "generatorVersion": "1.0.0",
                "inputIdentities": [watch.clone()],
            }},
        });
        let types = document["types"].as_array_mut().expect("types");
        types.push(wire_type(
            &watch,
            "clock_value_type",
            json!({"target": "ix://filament/native/Clock", "constraints": [constraint]}),
        ));
        types[1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(field("w", &watch));
        document["constructs"]
            .as_array_mut()
            .expect("constructs")
            .push(wire_construct(
                "clock_value_type",
                meaning::VALUE_TYPE,
                json!({}),
            ));
    });
    let graph = check(&document, &format!("{UNITS}{DENSE}"))
        .unwrap_or_else(|refusals| panic!("a clock-bound value type checks: {refusals:?}"));
    let model = timed(&graph);
    let w = model
        .clocks()
        .iter()
        .find(|clock| clock.name() == "w")
        .unwrap_or_else(|| panic!("w's place: {:?}", model.clocks()));
    assert!(
        model.invariants().iter().any(|invariant| invariant
            .bounds()
            .iter()
            .any(|bound| model.clock(bound.clock()).map(|c| c.name()) == Some(w.name()))),
        "Watch's max bounds w: {:?}",
        model.invariants()
    );
}

/// FR-488-AC-29 (TC-3082 step 3): a clock guard's quantifier binder in the
/// reserved slot form, `forall(slot_sx_sy in self.ts: ...)` over a
/// `Sequence<Timer>` field, refuses `ambiguous_declaration`/`ambiguous-name`
/// naming the identifier and the tag `slot`, within the guard's quantifier.
#[trace("TC-3082", "FR-488-AC-29")]
#[test]
fn a_clock_guard_binder_in_the_reserved_slot_form_is_refused() {
    let document = timer_document(|document: &mut Value| {
        let mut held = field("ts", &package("Timer"));
        held["multiplicity"]["lower"] = json!(0);
        held["multiplicity"]["upper"] = json!(3);
        document["types"][1]["fields"]
            .as_array_mut()
            .expect("Call fields")
            .push(held);
    });
    let guard = "forall(slot_sx_sy in self.ts: slot_sx_sy.x < 5 ms)";
    let body = format!("{UNITS}{DENSE}pre F using v on Config::Call::send {{ {guard} }}\n");
    let refusals = check(&document, &body).expect_err("the reserved binder is refused");
    let reserved: Vec<_> = refusals
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            fsl_semantics::check::CheckCause::ReservedSlotName(cause) => Some(cause),
            _ => None,
        })
        .collect();
    let [cause] = reserved.as_slice() else {
        panic!("one reserved-name refusal: {refusals:?}");
    };
    assert_eq!(
        (cause.name.as_str(), cause.tag.to_string().as_str()),
        ("slot_sx_sy", "slot")
    );
    let (text, _) = crate::model_operations::config_unit_with_profiles(
        &document,
        crate::timed_model::TEMPORAL,
        &body,
    );
    let span = cause.span.expect("the guard's own span");
    assert_eq!(&text[span.start..span.end], guard);
    let refusal = refusals
        .iter()
        .find(|refusal| {
            matches!(
                refusal.cause,
                fsl_semantics::check::CheckCause::ReservedSlotName(_)
            )
        })
        .expect("the reserved-name refusal");
    assert_eq!(
        (refusal.cause.code(), refusal.cause.cause()),
        (Code::AmbiguousDeclaration, Some("ambiguous-name"))
    );
}

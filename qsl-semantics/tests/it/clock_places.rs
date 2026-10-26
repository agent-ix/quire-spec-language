// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Places": a clock below an object's field, through a package
//! record, a clock alias or a union case, read and reset by its path.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{ClockOp, PlaceStep};
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
/// declaration. Not covered: the field's own span, which a package field
/// does not carry (STD-493), and `ts: Sequence<Timer>`, the clock
/// collections' slice.
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

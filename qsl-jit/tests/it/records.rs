// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1825, FR-560, FR-561: record values run through the kernel on arena
//! handles, built, read and compared by the code the interpreter runs, and
//! return the interpreter's value, outcome and charges.

use qsl_eval::value::CheckedPackageEvaluation;
use quire_eval::FamilyOutcome;
use quire_exact::{Cancel, Integer, Meter, Outcome, Value};
use quire_semantic_value::supplied_stop::{SuppliedLimits, SuppliedRun};

use crate::common::{
    int, interpreted, jitted, name, package_of, prepare, unsupported_names, UNLIMITED,
};
use ix_trace_rs::trace;

const SOURCE: &str = "\
 record P { x: Int[0, 9]; y: Boolean; }\n\
 record W { a: Integer; o: Int[0, 9]?; }\n\
 function mk using v(a: Int[0, 9], b: Boolean): P pure { P { x: a, y: b } }\n\
 function getx using v(p: P): Int[0, 9] pure { p.x }\n\
 function gety using v(p: P): Boolean pure { p.y }\n\
 function roundtrip using v(a: Int[0, 9], b: Boolean): Int[0, 9] pure { (P { x: a, y: b }).x }\n\
 function same using v(p: P, q: P): Boolean pure { p = q }\n\
 function differ using v(p: P, q: P): Boolean pure { p != q }\n\
 function pick using v(c: Boolean, p: P, q: P): P pure { if c then p else q }\n\
 function flip using v(p: P): P pure { P { x: p.x, y: not p.y } }\n\
 function viacall using v(a: Int[0, 9]): Int[0, 9] pure { getx(mk(a, true)) }\n\
 function bigfield using v(a: Integer): Integer pure { (W { a: a * a }).a }\n\
 function optional using v(a: Integer): Boolean pure { W { a: a } = W { a: a } }\n\
 function nested using v(a: Int[0, 9], b: Boolean): Boolean pure { (P { x: (P { x: a, y: b }).x, y: (P { x: a, y: b }).y }).y }\n";

const FUNCTIONS: [&str; 12] = [
    "mk",
    "getx",
    "gety",
    "roundtrip",
    "same",
    "differ",
    "pick",
    "flip",
    "viacall",
    "bigfield",
    "optional",
    "nested",
];

/// The record `mk(a, b)` evaluates to, taken from the interpreter.
fn record(package: &qsl_package::CheckedPackage, a: i64, b: bool) -> Value {
    let mut meter = Meter::new(UNLIMITED);
    let mut run = SuppliedRun::new(SuppliedLimits::default(), Cancel::new()).expect("a run");
    let evaluation = package
        .call_supplied(
            &name("mk"),
            vec![int(a), Value::Boolean(b)],
            &Default::default(),
            &mut meter,
            &mut run,
        )
        .expect("mk evaluates");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("mk gave {other:?}"),
    }
}

fn arguments_of(package: &qsl_package::CheckedPackage, function: &str) -> Vec<Vec<Value>> {
    let records: Vec<Value> = [(0, false), (3, true), (9, true), (9, false)]
        .iter()
        .map(|(a, b)| record(package, *a, *b))
        .collect();
    let digits = || (0..10).map(int);
    match function {
        "mk" | "roundtrip" | "nested" => digits()
            .flat_map(|a| [true, false].map(|b| vec![a.clone(), Value::Boolean(b)]))
            .collect(),
        "getx" | "gety" | "flip" => records.iter().map(|p| vec![p.clone()]).collect(),
        "same" | "differ" => records
            .iter()
            .flat_map(|p| records.iter().map(|q| vec![p.clone(), q.clone()]))
            .collect(),
        "pick" => [true, false]
            .iter()
            .flat_map(|c| {
                records
                    .iter()
                    .map(move |p| vec![Value::Boolean(*c), p.clone(), p.clone()])
            })
            .collect(),
        "viacall" => digits().map(|a| vec![a]).collect(),
        _ => [
            Value::Integer(Integer::from(0_i64)),
            Value::Integer(Integer::from(7_i64)),
            Value::Integer(Integer::from(1_i128 << 70)),
        ]
        .into_iter()
        .map(|a| vec![a])
        .collect(),
    }
}

/// Every function is prepared, and each call returns the interpreter's
/// outcome and charges, building records, reading fields (including a
/// record with an absent optional field) and comparing records.
#[trace("TC-1385")]
#[test]
fn every_function_agrees_with_the_interpreter_on_value_and_charges() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &FUNCTIONS);
    assert_eq!(unsupported_names(&prepared), Vec::<String>::new());
    let mut compared = 0;
    for function in FUNCTIONS {
        for arguments in arguments_of(&package, function) {
            let want = interpreted(&package, function, &arguments, UNLIMITED);
            let got = jitted(&prepared, function, &arguments, UNLIMITED);
            assert_eq!(got.result, want.result, "{function} {arguments:?}");
            assert_eq!(
                got.charges, want.charges,
                "{function} {arguments:?} charges"
            );
            compared += 1;
        }
    }
    assert!(compared > 100, "{compared}");
}

/// With `work_units` at each value below the interpreter's spend, the JIT
/// stops at the same charge point.
#[trace("TC-1385")]
#[test]
fn a_denied_charge_stops_the_jit_where_it_stops_the_interpreter() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &FUNCTIONS);
    for function in FUNCTIONS {
        let arguments = arguments_of(&package, function).pop().expect("a tuple");
        for below in 0..24_u64 {
            let mut limits = UNLIMITED;
            limits.work_units = below;
            let want = interpreted(&package, function, &arguments, limits);
            let got = jitted(&prepared, function, &arguments, limits);
            assert_eq!(got.result, want.result, "{function} work_units={below}");
            assert_eq!(got.charges, want.charges, "{function} work_units={below}");
        }
    }
}

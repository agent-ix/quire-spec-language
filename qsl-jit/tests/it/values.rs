// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1825, FR-560, FR-561: `Rational`, `Decimal` and `Text` values run
//! through the kernel on arena handles and return the interpreter's value,
//! outcome, charges and loss records.

use quire_exact::{
    admit_text, Decimal, Integer, Meter, Rational, Text, TextPayload, TextProfile, TextType, Value,
};

use crate::common::{int, interpreted, jitted, package_of, prepare, unsupported_names, UNLIMITED};
use ix_trace_rs::trace;

const SOURCE: &str = "\
 type Q = Rational[-10000000000000000000000000, 10000000000000000000000000; 1, 100000];\n\
 type QP = Rational[1, 10000000000000000000000000; 1, 100000];\n\
 type S = Rational[-10000000000000000000000000000000, 10000000000000000000000000000000; 1, 10000000000];\n\
 type M = Rational[-1000000000000000000000000000000000000000000000000000, 1000000000000000000000000000000000000000000000000000; 1, 10000000000];\n\
 type D = Rational[-10000000000000000000000000000000, 10000000000000000000000000000000; 1, 10000000000000000000000000000000];\n\
 function ratio using v(a: Int[0, 9], b: Int[1, 9]): Rational[0, 9; 1, 9] pure { a / b }\n\
 function wide_ratio using v(a: Int[-1000000000000000000000000000000, 1000000000000000000000000000000], b: Int[1, 1000000]): Rational[-1000000000000000000000000000000, 1000000000000000000000000000000; 1, 1000000] pure { a / b }\n\
 function radd using v(p: Q, q: Q): S pure { p + q }\n\
 function rsub using v(p: Q, q: Q): S pure { p - q }\n\
 function rmul using v(p: Q, q: Q): M pure { p * q }\n\
 function rquot using v(p: Q, q: QP): D pure { p / q }\n\
 function rneg using v(p: Q): Q pure { -p }\n\
 function rless using v(p: Q, q: Q): Boolean pure { p < q }\n\
 function rleast using v(p: Q, q: Q): Boolean pure { p >= q }\n\
 function rsame using v(p: Q, q: Q): Boolean pure { p = q }\n\
 function rpick using v(c: Boolean, p: Q, q: Q): Q pure { if c then p else q }\n\
 function rcall using v(p: Q): S pure { radd(p, rneg(p)) }\n\
 function dsum using v(a: Decimal[0, 2000000; 0, 3; exact], b: Decimal[0, 2000000; 0, 3; exact]): Decimal[0, 2000000; 0, 3; exact] pure { a + b }\n\
 function dneg using v(a: Decimal[-2000000, 2000000; 0, 3; exact]): Decimal[-2000000, 2000000; 0, 3; exact] pure { -a }\n\
 function dless using v(a: Decimal[0, 2000000; 0, 3; exact], b: Decimal[0, 2000000; 0, 3; exact]): Boolean pure { a <= b }\n\
 function dsame using v(a: Decimal[0, 2000000; 0, 3; exact], b: Decimal[0, 2000000; 0, 3; exact]): Boolean pure { a = b }\n\
 function dround using v(q: Rational[0, 2; 1, 3]): Decimal[0, 100; 2, 2; nearest-even] pure { convert<Decimal[0, 100; 2, 2; nearest-even]>(q) }\n\
 function tless using v(a: Text[0, 16; nfd], b: Text[0, 16; nfd]): Boolean pure { a < b }\n\
 function tsame using v(a: Text[0, 16; nfd], b: Text[0, 16; nfd]): Boolean pure { a = b }\n\
 function tdiffer using v(a: Text[0, 16; nfd], b: Text[0, 16; nfd]): Boolean pure { a != b }\n\
 function tpick using v(c: Boolean, a: Text[0, 16; nfd], b: Text[0, 16; nfd]): Text[0, 16; nfd] pure { if c then a else b }\n";

const FUNCTIONS: [&str; 21] = [
    "ratio",
    "wide_ratio",
    "radd",
    "rsub",
    "rmul",
    "rquot",
    "rneg",
    "rless",
    "rleast",
    "rsame",
    "rpick",
    "rcall",
    "dsum",
    "dneg",
    "dless",
    "dsame",
    "dround",
    "tless",
    "tsame",
    "tdiffer",
    "tpick",
];

fn rational(numerator: i64, denominator: i64) -> Value {
    Value::Rational(
        Rational::new(Integer::from(numerator), Integer::from(denominator))
            .expect("a nonzero denominator"),
    )
}

fn decimal(coefficient: i64, scale: u32) -> Value {
    Value::Decimal(Decimal::new(Integer::from(coefficient), scale))
}

fn text(payload: &str) -> Value {
    let text: Text = admit_text(
        &TextPayload::from_utf8(payload.as_bytes()).expect("valid UTF-8"),
        &TextType::new(0, 16, TextProfile::Nfd).expect("a text type"),
        &mut Meter::new(UNLIMITED),
    )
    .completed()
    .expect("the text admits");
    Value::Text(text)
}

fn pairs(values: &[Value]) -> Vec<Vec<Value>> {
    values
        .iter()
        .flat_map(|a| values.iter().map(|b| vec![a.clone(), b.clone()]))
        .collect()
}

fn rationals() -> Vec<Value> {
    vec![
        rational(0, 1),
        rational(1, 3),
        rational(-7, 2),
        rational(5, 5),
        Value::Rational(Rational::from_integer(Integer::from(1_i128 << 80))),
    ]
}

fn positives() -> Vec<Value> {
    vec![rational(1, 3), rational(7, 2), rational(5, 5)]
}

fn decimals() -> Vec<Value> {
    vec![
        decimal(0, 0),
        decimal(123, 0),
        decimal(5, 3),
        decimal(1_999_999, 3),
    ]
}

fn texts() -> Vec<Value> {
    vec![
        text(""),
        text("a"),
        text("b"),
        text("\u{e9}\u{e9}"),
        text("e\u{301}e\u{301}"),
    ]
}

fn arguments_of(function: &str) -> Vec<Vec<Value>> {
    let with_flag = |tuples: Vec<Vec<Value>>| {
        [true, false]
            .into_iter()
            .flat_map(|flag| {
                tuples.clone().into_iter().map(move |mut tuple| {
                    tuple.insert(0, Value::Boolean(flag));
                    tuple
                })
            })
            .collect()
    };
    match function {
        "ratio" => (0..10)
            .flat_map(|a| (1..10).map(move |b| vec![int(a), int(b)]))
            .collect(),
        "wide_ratio" => [
            Value::Integer(Integer::from(-3_i64)),
            Value::Integer(Integer::from(1_i128 << 70)),
        ]
        .iter()
        .flat_map(|a| {
            [int(1), int(7)]
                .into_iter()
                .map(move |b| vec![a.clone(), b])
        })
        .collect(),
        "rquot" => rationals()
            .iter()
            .flat_map(|p| positives().into_iter().map(move |q| vec![p.clone(), q]))
            .collect(),
        "rneg" | "rcall" => rationals().into_iter().map(|p| vec![p]).collect(),
        "dneg" => decimals().into_iter().map(|p| vec![p]).collect(),
        "dround" => vec![
            vec![rational(2, 3)],
            vec![rational(1, 3)],
            vec![rational(0, 1)],
        ],
        "dsum" | "dless" | "dsame" => pairs(&decimals()),
        "tless" | "tsame" | "tdiffer" => pairs(&texts()),
        "tpick" => with_flag(pairs(&texts())),
        "rpick" => with_flag(pairs(&rationals())),
        _ => pairs(&rationals()),
    }
}

/// Every function is prepared, and each call returns the interpreter's
/// outcome, charges and loss records.
#[trace("TC-1385")]
#[test]
fn every_function_agrees_with_the_interpreter_on_value_charges_and_losses() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &FUNCTIONS);
    assert_eq!(unsupported_names(&prepared), Vec::<String>::new());
    let mut compared = 0;
    for function in FUNCTIONS {
        for arguments in arguments_of(function) {
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
    assert!(compared > 300, "{compared}");
}

/// A rounding conversion records its loss at the conversion, as the
/// interpreter does.
#[trace("TC-1385")]
#[test]
fn a_rounded_conversion_returns_the_interpreters_loss_record() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &["dround"]);
    let got = jitted(&prepared, "dround", &[rational(2, 3)], UNLIMITED);
    assert!(got.result.contains("Decimal"), "{}", got.result);
    assert!(
        got.result.contains("losses: [LocatedLoss"),
        "{}",
        got.result
    );
}

/// With `work_units` at each value below the interpreter's spend, the JIT
/// stops at the same charge point.
#[trace("TC-1385")]
#[test]
fn a_denied_charge_stops_the_jit_where_it_stops_the_interpreter() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &FUNCTIONS);
    for function in FUNCTIONS {
        let arguments = arguments_of(function).pop().expect("a tuple");
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

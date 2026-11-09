// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1825, FR-561, FR-562: an `Integer` operation whose result or operand
//! does not fit a machine word runs through the kernel on a value handle and
//! returns the interpreter's value, outcome and charges.

use quire_exact::{Integer, Value};

use crate::common::{
    int, interpreted, jitted, package_with_division, prepare, unsupported_names, UNLIMITED,
};

const SOURCE: &str = "\
 function mulwide using v(a: Int[0, 4000000000000000000], b: Int[0, 4000000000000000000]): Integer pure { a * b }\n\
 function addbig using v(a: Integer, b: Integer): Integer pure { a + b }\n\
 function subbig using v(a: Integer, b: Integer): Integer pure { a - b }\n\
 function mulbig using v(a: Integer, b: Integer): Integer pure { a * b }\n\
 function negbig using v(a: Integer): Integer pure { -a }\n\
 function divbig using v(a: Integer, b: Integer): Integer pure { a div b }\n\
 function rembig using v(a: Integer, b: Integer): Integer pure { a rem b }\n\
 function modbig using v(a: Integer, b: Integer): Integer pure { a mod b }\n\
 function lessbig using v(a: Integer, b: Integer): Boolean pure { a < b }\n\
 function atleast using v(a: Integer, b: Int[0, 9]): Boolean pure { a >= b }\n\
 function samebig using v(a: Integer, b: Integer): Boolean pure { a = b }\n\
 function differbig using v(a: Integer, b: Integer): Boolean pure { a != b }\n\
 function mixed using v(a: Integer, b: Int[0, 9]): Integer pure { a * b + 1 }\n\
 function joined using v(p: Boolean, a: Integer, b: Int[0, 9]): Integer pure { if p then a else b }\n\
 function huge using v(): Integer pure { 170141183460469231731687303715884105727 * 3 }\n\
 function called using v(a: Integer): Integer pure { addbig(a, a) * mulwide(2, 3) }\n\
 function product_of_calls using v(a: Int[0, 4000000000000000000]): Integer pure { mulwide(a, a) * mulwide(a, a) }\n\
 function lowmod using v(a: Integer): Boolean pure { a mod 10 < 5 }\n\
 function letwide using v(a: Integer): Integer pure { let s = a * a in s + s }\n";

const FUNCTIONS: [&str; 19] = [
    "mulwide",
    "addbig",
    "subbig",
    "mulbig",
    "negbig",
    "divbig",
    "rembig",
    "modbig",
    "lessbig",
    "atleast",
    "samebig",
    "differbig",
    "mixed",
    "joined",
    "huge",
    "called",
    "product_of_calls",
    "lowmod",
    "letwide",
];

fn big(shift: u32, offset: i64) -> Value {
    Value::Integer(Integer::from((1_i128 << shift) + i128::from(offset)))
}

fn negative(shift: u32) -> Value {
    Value::Integer(Integer::from(-(1_i128 << shift)))
}

fn samples() -> Vec<Value> {
    vec![
        int(0),
        int(1),
        int(-1),
        int(7),
        int(i64::MAX),
        int(i64::MIN),
        big(64, 0),
        big(100, 3),
        negative(70),
    ]
}

fn arguments_of(function: &str) -> Vec<Vec<Value>> {
    let pairs = |left: Vec<Value>, right: Vec<Value>| {
        left.iter()
            .flat_map(|a| right.iter().map(|b| vec![a.clone(), b.clone()]))
            .collect::<Vec<_>>()
    };
    let digits = || (0..10).map(int).collect::<Vec<_>>();
    match function {
        "mulwide" => pairs(
            vec![int(0), int(3), int(4_000_000_000_000_000_000)],
            vec![int(0), int(2), int(4_000_000_000_000_000_000)],
        ),
        "mixed" | "atleast" => pairs(samples(), digits()),
        "joined" => [true, false]
            .iter()
            .flat_map(|p| {
                pairs(samples(), digits())
                    .into_iter()
                    .map(move |mut tuple| {
                        tuple.insert(0, Value::Boolean(*p));
                        tuple
                    })
            })
            .collect(),
        "huge" => vec![vec![]],
        "negbig" | "called" | "lowmod" | "letwide" => {
            samples().into_iter().map(|a| vec![a]).collect()
        }
        "product_of_calls" => [0, 3, 4_000_000_000_000_000_000]
            .into_iter()
            .map(|a| vec![int(a)])
            .collect(),
        _ => pairs(samples(), samples()),
    }
}

/// The JIT prepares every function, and each call returns the interpreter's
/// outcome with the interpreter's charges, over operands on both sides of the
/// machine word (a zero divisor, `Undefined`, is among them).
#[test]
fn every_function_agrees_with_the_interpreter_on_value_and_charges() {
    let package = package_with_division(SOURCE);
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
    assert!(compared > 500, "{compared}");
}

/// With `work_units` at each value below the interpreter's spend, the JIT
/// stops at the same charge point with the same `Incomplete`.
#[test]
fn a_denied_charge_stops_the_jit_where_it_stops_the_interpreter() {
    let package = package_with_division(SOURCE);
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

/// The integer-bit limit is the kernel's: a product wider than the limit
/// refuses on the handle path as it does in the interpreter.
#[test]
fn a_result_wider_than_the_integer_bit_limit_is_the_kernels_refusal() {
    let package = package_with_division(SOURCE);
    let prepared = prepare(&package, &["mulbig"]);
    let mut limits = UNLIMITED;
    limits.integer_bits = 100;
    let arguments = [big(80, 0), big(80, 0)];
    let want = interpreted(&package, "mulbig", &arguments, limits);
    let got = jitted(&prepared, "mulbig", &arguments, limits);
    assert_eq!(got.result, want.result);
    assert_eq!(got.charges, want.charges);
}

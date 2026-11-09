// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-562, FR-561: the JIT returns the interpreter's outcome and meters the
//! same charges at the same points, at each parameter's range ends.

use ix_trace_rs::trace;

use crate::common::{int, interpreted, jitted, package_of, prepare, UNLIMITED};

const SOURCE: &str = "\
 function add using v(a: Int[0, 100], b: Int[0, 100]): Integer pure { a + b }\n\
 function mixed using v(a: Int[-50, 50], b: Int[-50, 50]): Integer pure { -(a * b) - a }\n\
 function wide using v(a: Int[-3037000499, 3037000499], b: Int[-3037000499, 3037000499]): Integer pure { a * b }\n\
 function modulo using v(a: Int[-50, 50], b: Int[1, 7]): Boolean pure { (a * 3 - b) mod 5 >= 2 }\n\
 function wrap using v(a: Int[0, 9], b: Int[0, 9]): Integer pure { let s = a + b in if s > 9 then s - 9 else s }\n\
 function flag using v(p: Boolean, a: Int[0, 9]): Integer pure { if p then a else 0 - a }\n\
 function literal using v(): Integer pure { 7 * 6 }\n";

/// Range-end argument tuples for a function of two bounded integers.
fn ends(low: i64, high: i64) -> Vec<Vec<quire_exact::Value>> {
    let points = [low, low + 1, 0.clamp(low, high), high - 1, high];
    points
        .iter()
        .flat_map(|a| points.iter().map(|b| vec![int(*a), int(*b)]))
        .collect()
}

fn cases() -> Vec<(&'static str, Vec<Vec<quire_exact::Value>>)> {
    vec![
        ("add", ends(0, 100)),
        ("mixed", ends(-50, 50)),
        ("wide", ends(-3_037_000_499, 3_037_000_499)),
        (
            "modulo",
            [-50, -7, 0, 7, 50]
                .iter()
                .flat_map(|a| (1..=7).map(move |b| vec![int(*a), int(b)]))
                .collect(),
        ),
        ("wrap", ends(0, 9)),
        (
            "flag",
            [true, false]
                .iter()
                .flat_map(|p| {
                    [0, 4, 9]
                        .iter()
                        .map(move |a| vec![quire_exact::Value::Boolean(*p), int(*a)])
                })
                .collect(),
        ),
        ("literal", vec![vec![]]),
    ]
}

#[trace("TC-1385")]
#[test]
fn every_supported_function_returns_the_interpreters_value_and_charges() {
    let package = package_of(SOURCE);
    let names: Vec<_> = cases().iter().map(|(n, _)| *n).collect();
    let prepared = prepare(&package, &names);
    let unsupported = crate::common::unsupported_names(&prepared);
    assert!(unsupported.is_empty(), "unsupported: {unsupported:?}");
    for (function, tuples) in cases() {
        for arguments in tuples {
            let want = interpreted(&package, function, &arguments, UNLIMITED);
            let got = jitted(&prepared, function, &arguments, UNLIMITED);
            assert_eq!(got.result, want.result, "{function} {arguments:?}");
            assert_eq!(
                got.charges, want.charges,
                "{function} {arguments:?} charges"
            );
            assert!(!want.charges.is_empty() || function == "literal");
        }
    }
}

/// With `work_units` below the interpreter's spend, the JIT returns the
/// interpreter's `Incomplete` at the same charge point (swept over 0..8).
#[trace("TC-1385")]
#[test]
fn one_work_unit_below_the_spend_gives_the_interpreters_incomplete() {
    let package = package_of(SOURCE);
    let names: Vec<_> = cases().iter().map(|(n, _)| *n).collect();
    let prepared = prepare(&package, &names);
    let mut checked = 0;
    for (function, tuples) in cases() {
        let arguments = tuples.last().expect("a tuple").clone();
        // Deny each charge in turn: `work_units` below the interpreter's
        // spend stops it at successive charge points.
        for below in 0..8_u64 {
            let mut limits = UNLIMITED;
            limits.work_units = below;
            let want = interpreted(&package, function, &arguments, limits);
            let got = jitted(&prepared, function, &arguments, limits);
            assert_eq!(got.result, want.result, "{function} work_units={below}");
            assert_eq!(got.charges, want.charges, "{function} work_units={below}");
            checked += 1;
        }
    }
    assert!(checked > 0);
}

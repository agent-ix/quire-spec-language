// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-561, FR-562: through a recording kernel table, an `Integer` operation
//! whose checked bounds prove the result fits records no kernel value
//! operation and charges as the kernel call it replaces; every other value
//! operation records one; a status a kernel entry returns is passed up.

use ix_trace_rs::trace;
use qsl_jit::{KernelCall, KernelRecorder};
use quire_eval::{CallFailure, FamilyOutcome};
use quire_exact::{AdmittedCharge, Cancel, CancelCause, Integer, Meter, Outcome, Value};
use quire_semantic_value::supplied_stop::{SuppliedLimits, SuppliedRun};

use crate::common::{
    int, interpreted, name, package_kernel, package_with_division, prepare, UNLIMITED,
};

const SOURCE: &str = "\
 type Q = Rational[-10, 10; 1, 10];\n\
 type QS = Rational[-1000, 1000; 1, 100];\n\
 function add using v(a: Integer, b: Integer): Integer pure { a + b }\n\
 function q using v(a: Integer, b: Integer): Integer pure { a div b }\n\
 function f using v(a: Int[0, 4611686018427387903], b: Int[0, 4611686018427387904]): Integer pure { a + b }\n\
 function g using v(a: Int[0, 4611686018427387904], b: Int[0, 4611686018427387904]): Integer pure { a + b }\n\
 function m using v(a: Int[-3037000499, 3037000499], b: Int[-3037000499, 3037000499]): Integer pure { a * b }\n\
 function m2 using v(a: Int[-3037000500, 3037000500], b: Int[-3037000500, 3037000500]): Integer pure { a * b }\n\
 function n using v(a: Int[-9223372036854775808, 0], b: Int[-1, -1]): Integer pure { a div b }\n\
 function d using v(a: Int[0, 9], b: Int[1, 9]): Integer pure { a div b }\n\
 function d0 using v(a: Int[0, 9], b: Int[0, 9]): Integer pure { a div b }\n\
 function dr using v(a: Int[-9, 9], b: Int[1, 9]): Integer pure { a rem b }\n\
 function dm using v(a: Int[-9, 9], b: Int[-9, -1]): Integer pure { a mod b }\n\
 function neg using v(a: Int[-9, 9]): Integer pure { -a }\n\
 function sub using v(a: Int[-9, 9], b: Int[-9, 9]): Integer pure { a - b }\n\
 function le using v(a: Int[-9, 9], b: Int[-9, 9]): Boolean pure { a <= b }\n\
 function gt using v(a: Int[-9, 9], b: Int[-9, 9]): Boolean pure { a > b }\n\
 function lt using v(a: Int[-9, 9], b: Int[-9, 9]): Boolean pure { a < b }\n\
 function ge using v(a: Int[-9, 9], b: Int[-9, 9]): Boolean pure { a >= b }\n\
 function same using v(a: Int[-9, 9], b: Int[-9, 9]): Boolean pure { a = b }\n\
 function differ using v(a: Int[-9, 9], b: Int[-9, 9]): Boolean pure { a != b }\n\
 function r using v(a: Q, b: Q): QS pure { a + b }\n\
 function t using v(a: Text[0, 16; nfd], b: Text[0, 16; nfd]): Boolean pure { a = b }\n\
 function u using v(a: Integer, b: Integer): Integer pure { a + b }\n";

/// One recorded call: its rendered result, charges and the kernel calls.
struct Recorded {
    result: String,
    charges: Vec<AdmittedCharge>,
    calls: Vec<KernelCall>,
    outcome: Result<Option<Value>, String>,
}

fn recorded(
    prepared: &qsl_jit::PreparedJit<'_>,
    function: &str,
    arguments: &[Value],
    limits: quire_exact::ScalarLimits,
    fault_on: Option<&'static str>,
    cancel: &Cancel,
) -> Recorded {
    let mut meter = Meter::new(limits);
    let mut run = SuppliedRun::new(SuppliedLimits::default(), cancel.clone()).expect("a run");
    let mut recorder = KernelRecorder {
        calls: Vec::new(),
        fault_on,
    };
    let result = prepared.call_recorded(
        &name(function),
        arguments,
        &Default::default(),
        &mut meter,
        &mut run,
        cancel,
        &mut recorder,
    );
    let outcome = match &result {
        Ok(evaluation) => match &evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => Ok(Some(value.clone())),
            _ => Ok(None),
        },
        Err(failure) => Err(format!("{failure:?}")),
    };
    Recorded {
        result: format!("{result:?}"),
        charges: meter.admitted_charges().to_vec(),
        calls: recorder.calls,
        outcome,
    }
}

fn run(prepared: &qsl_jit::PreparedJit<'_>, function: &str, arguments: &[Value]) -> Recorded {
    recorded(
        prepared,
        function,
        arguments,
        UNLIMITED,
        None,
        &Cancel::new(),
    )
}

fn names(recorded: &Recorded) -> Vec<&'static str> {
    recorded.calls.iter().map(|call| call.operation).collect()
}

fn wide(shift: u32) -> Value {
    Value::Integer(Integer::from(1_i128 << shift))
}

/// The ends and middle of a parameter whose range is `[low, high]`.
fn ends(low: i64, high: i64) -> Vec<Value> {
    vec![int(low), int(high), int(low / 2 + high / 2)]
}

/// Every call of `function` over `tuples` returns the interpreter's outcome.
fn values_agree(
    package: &qsl_package::CheckedPackage,
    prepared: &qsl_jit::PreparedJit<'_>,
    function: &str,
    tuples: &[Vec<Value>],
) {
    for arguments in tuples {
        let want = interpreted(package, function, arguments, UNLIMITED);
        let got = run(prepared, function, arguments);
        assert_eq!(got.result, want.result, "{function} {arguments:?}");
    }
}

/// Every call of `function` over `tuples` returns the interpreter's value and
/// charges, and under `work_units` one below each prefix of the spend the
/// interpreter's stop (FR-562-AC-4).
fn agrees(
    package: &qsl_package::CheckedPackage,
    prepared: &qsl_jit::PreparedJit<'_>,
    function: &str,
    tuples: &[Vec<Value>],
) {
    for arguments in tuples {
        let want = interpreted(package, function, arguments, UNLIMITED);
        let got = run(prepared, function, arguments);
        assert_eq!(got.result, want.result, "{function} {arguments:?}");
        assert_eq!(got.charges, want.charges, "{function} {arguments:?}");
        let spend = want.charges.len() as u64 + 2;
        for below in 0..spend {
            let mut limits = UNLIMITED;
            limits.work_units = below;
            let want = interpreted(package, function, arguments, limits);
            let got = recorded(prepared, function, arguments, limits, None, &Cancel::new());
            assert_eq!(got.result, want.result, "{function} {arguments:?} {below}");
            assert_eq!(
                got.charges, want.charges,
                "{function} {arguments:?} {below}"
            );
        }
    }
}

#[trace("TC-1382", "FR-561-AC-1")]
#[test]
fn an_unbounded_add_records_one_kernel_call_and_the_interpreters_charges() {
    let package = package_with_division(SOURCE);
    let prepared = prepare(&package, &["add", "q"]);
    let arguments = [wide(100), int(1)];
    let got = run(&prepared, "add", &arguments);
    assert_eq!(names(&got), ["add"]);
    let operands: Vec<Value> = got.calls[0].operands.clone();
    assert!(matches!(
        (&operands[0], &operands[1]),
        (Value::Integer(a), Value::Integer(b))
            if *a == Integer::from(1_i128 << 100) && *b == Integer::from(1_i64)
    ));
    assert!(matches!(
        &got.outcome,
        Ok(Some(Value::Integer(sum))) if *sum == Integer::from((1_i128 << 100) + 1)
    ));
    let want = interpreted(&package, "add", &arguments, UNLIMITED);
    assert_eq!(got.result, want.result);
    assert_eq!(got.charges, want.charges);
}

#[trace("TC-1382", "FR-561-AC-2")]
#[test]
fn a_kernel_status_is_passed_up_unchanged() {
    let package = package_with_division(SOURCE);
    let prepared = prepare(&package, &["q"]);
    let zero = [int(7), int(0)];
    // A zero divisor is the interpreter's `Undefined`, at the `div`.
    let want = interpreted(&package, "q", &zero, UNLIMITED);
    let got = run(&prepared, "q", &zero);
    assert_eq!(got.result, want.result);
    assert_eq!(names(&got), ["div"]);
    // One below the spend: the interpreter's `Incomplete`.
    let spend = interpreted(&package, "q", &[int(7), int(2)], UNLIMITED)
        .charges
        .len() as u64;
    for below in 0..spend {
        let mut limits = UNLIMITED;
        limits.work_units = below;
        let want = interpreted(&package, "q", &[int(7), int(2)], limits);
        let got = recorded(
            &prepared,
            "q",
            &[int(7), int(2)],
            limits,
            None,
            &Cancel::new(),
        );
        assert_eq!(got.result, want.result, "{below}");
    }
    // A `Fault` from the `div` entry is a fault and no value.
    let faulted = recorded(
        &prepared,
        "q",
        &[int(7), int(2)],
        UNLIMITED,
        Some("div"),
        &Cancel::new(),
    );
    assert!(matches!(faulted.outcome, Err(ref failure) if failure.starts_with("Fault")));
    // A cancelled handle returns `Cancelled(Requested)` before the call.
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let cancelled = recorded(&prepared, "q", &[int(7), int(2)], UNLIMITED, None, &cancel);
    assert!(matches!(
        cancelled.outcome,
        Err(ref failure) if *failure == format!("{:?}", CallFailure::Cancelled(CancelCause::Requested))
    ));
    assert!(cancelled.calls.is_empty());
}

#[trace("TC-1384", "FR-562-AC-1")]
#[test]
fn an_add_takes_the_fast_path_exactly_where_the_sum_fits_64_bits() {
    let package = package_with_division(SOURCE);
    let prepared = prepare(&package, &["f", "g"]);
    let f: Vec<Vec<Value>> = ends(0, 4_611_686_018_427_387_903)
        .into_iter()
        .flat_map(|a| {
            ends(0, 4_611_686_018_427_387_903)
                .into_iter()
                .map(move |b| vec![a.clone(), b])
        })
        .collect();
    let g: Vec<Vec<Value>> = ends(0, 4_611_686_018_427_387_904)
        .into_iter()
        .flat_map(|a| {
            ends(0, 4_611_686_018_427_387_904)
                .into_iter()
                .map(move |b| vec![a.clone(), b])
        })
        .collect();
    for arguments in &f {
        assert_eq!(names(&run(&prepared, "f", arguments)), Vec::<&str>::new());
    }
    for arguments in &g {
        assert_eq!(names(&run(&prepared, "g", arguments)), ["add"]);
    }
    values_agree(&package, &prepared, "f", &f);
    values_agree(&package, &prepared, "g", &g);
}

#[trace("TC-1384", "FR-562-AC-2", "FR-562-AC-3")]
#[test]
fn a_multiply_and_a_division_take_the_fast_path_where_the_bounds_prove_it() {
    let package = package_with_division(SOURCE);
    let prepared = prepare(&package, &["m", "m2", "n", "d", "d0", "dr", "dm"]);
    let pairs = |low: i64, high: i64| -> Vec<Vec<Value>> {
        ends(low, high)
            .into_iter()
            .flat_map(|a| ends(low, high).into_iter().map(move |b| vec![a.clone(), b]))
            .collect()
    };
    let m = pairs(-3_037_000_499, 3_037_000_499);
    let m2 = pairs(-3_037_000_500, 3_037_000_500);
    for arguments in &m {
        assert!(names(&run(&prepared, "m", arguments)).is_empty());
    }
    for arguments in &m2 {
        assert_eq!(names(&run(&prepared, "m2", arguments)), ["mul"]);
    }
    let n: Vec<Vec<Value>> = ends(i64::MIN, 0)
        .into_iter()
        .map(|a| vec![a, int(-1)])
        .collect();
    for arguments in &n {
        assert_eq!(names(&run(&prepared, "n", arguments)), ["div"]);
    }
    let d: Vec<Vec<Value>> = (0..10)
        .flat_map(|a| (1..10).map(move |b| vec![int(a), int(b)]))
        .collect();
    for arguments in &d {
        assert!(names(&run(&prepared, "d", arguments)).is_empty());
    }
    let d0: Vec<Vec<Value>> = (0..10)
        .flat_map(|a| (0..10).map(move |b| vec![int(a), int(b)]))
        .collect();
    for arguments in &d0 {
        assert_eq!(names(&run(&prepared, "d0", arguments)), ["div"]);
    }
    let signed: Vec<Vec<Value>> = (-9..10)
        .flat_map(|a| (1..10).map(move |b| vec![int(a), int(b)]))
        .collect();
    let negative: Vec<Vec<Value>> = (-9..10)
        .flat_map(|a| (-9..0).map(move |b| vec![int(a), int(b)]))
        .collect();
    for arguments in &signed {
        assert!(names(&run(&prepared, "dr", arguments)).is_empty());
    }
    for arguments in &negative {
        assert!(names(&run(&prepared, "dm", arguments)).is_empty());
    }
    for (function, tuples) in [
        ("m", &m),
        ("m2", &m2),
        ("n", &n),
        ("d", &d),
        ("d0", &d0),
        ("dr", &signed),
        ("dm", &negative),
    ] {
        values_agree(&package, &prepared, function, tuples);
    }
}

#[trace("TC-1384")]
#[test]
fn a_negation_a_subtraction_and_the_comparisons_take_the_fast_path() {
    let package = package_with_division(SOURCE);
    let prepared = prepare(
        &package,
        &["neg", "sub", "lt", "le", "gt", "ge", "same", "differ"],
    );
    let ones: Vec<Vec<Value>> = (-9..10).map(|a| vec![int(a)]).collect();
    let twos: Vec<Vec<Value>> = (-9..10)
        .flat_map(|a| (-9..10).map(move |b| vec![int(a), int(b)]))
        .collect();
    for (function, tuples) in [
        ("neg", &ones),
        ("sub", &twos),
        ("lt", &twos),
        ("le", &twos),
        ("gt", &twos),
        ("ge", &twos),
        ("same", &twos),
        ("differ", &twos),
    ] {
        for arguments in tuples.iter().step_by(7) {
            assert!(names(&run(&prepared, function, arguments)).is_empty());
        }
        values_agree(&package, &prepared, function, tuples);
    }
}

#[trace("TC-1384", "FR-562-AC-5")]
#[test]
fn a_value_that_fits_a_word_still_calls_the_kernel_for_a_rational_a_text_or_an_unbounded_sum() {
    let package = package_with_division(SOURCE);
    let prepared = prepare(&package, &["r", "t", "u"]);
    let rational = |n: i64, d: i64| {
        Value::Rational(
            quire_exact::Rational::new(Integer::from(n), Integer::from(d)).expect("a ratio"),
        )
    };
    let text = |payload: &str| {
        Value::Text(
            quire_exact::admit_text(
                &quire_exact::TextPayload::from_utf8(payload.as_bytes()).expect("UTF-8"),
                &quire_exact::TextType::new(0, 16, quire_exact::TextProfile::Nfd).expect("a type"),
                &mut Meter::new(UNLIMITED),
            )
            .completed()
            .expect("the text admits"),
        )
    };
    assert_eq!(
        names(&run(&prepared, "r", &[rational(1, 1), rational(2, 1)])),
        ["add"]
    );
    assert_eq!(names(&run(&prepared, "t", &[text("a"), text("a")])), ["eq"]);
    assert_eq!(names(&run(&prepared, "u", &[int(1), int(2)])), ["add"]);
}

/// The argument tuples at the ends and middle of each parameter's range.
fn tuples_of(function: &str) -> Vec<Vec<Value>> {
    let pairs = |low: i64, high: i64| -> Vec<Vec<Value>> {
        ends(low, high)
            .into_iter()
            .flat_map(|a| ends(low, high).into_iter().map(move |b| vec![a.clone(), b]))
            .collect()
    };
    let digits = |low: i64, high: i64| -> Vec<Vec<Value>> {
        (low..=high)
            .flat_map(|a| (low..=high).map(move |b| vec![int(a), int(b)]))
            .collect()
    };
    match function {
        "f" => ends(0, 4_611_686_018_427_387_903)
            .into_iter()
            .flat_map(|a| {
                ends(0, 4_611_686_018_427_387_904)
                    .into_iter()
                    .map(move |b| vec![a.clone(), b])
            })
            .collect(),
        "neg" => (-9..=9).map(|a| vec![int(a)]).collect(),
        "d" => (0..=9)
            .flat_map(|a| (1..=9).map(move |b| vec![int(a), int(b)]))
            .collect(),
        "dr" => (-9..=9)
            .flat_map(|a| (1..=9).map(move |b| vec![int(a), int(b)]))
            .collect(),
        "dm" => (-9..=9)
            .flat_map(|a| (-9..=-1).map(move |b| vec![int(a), int(b)]))
            .collect(),
        "sub" | "lt" | "le" | "gt" | "ge" | "same" | "differ" => digits(-9, 9),
        "g" => pairs(0, 4_611_686_018_427_387_904),
        "m" => pairs(-3_037_000_499, 3_037_000_499),
        "m2" => pairs(-3_037_000_500, 3_037_000_500),
        "n" => ends(i64::MIN, 0)
            .into_iter()
            .map(|a| vec![a, int(-1)])
            .collect(),
        _ => digits(0, 9),
    }
}

/// For each function of FR-562 AC-1 to AC-3, and for the other fast-path
/// operations, the charge sequence of every
/// call equals the interpreter's, and one work unit below it the stop is the
/// interpreter's, at the same charge point.
#[trace("TC-1385", "FR-562-AC-4")]
#[test]
fn every_fast_path_function_charges_as_the_kernel_call_it_replaces() {
    let package = package_with_division(SOURCE);
    let functions = [
        "f", "g", "m", "m2", "n", "d", "d0", "dr", "dm", "neg", "sub", "lt", "le", "gt", "ge",
        "same", "differ",
    ];
    let prepared = prepare(&package, &functions);
    for function in functions {
        agrees(&package, &prepared, function, &tuples_of(function));
    }
}

/// Division under each admitted law: every sign combination of the ends,
/// zero where admitted, `MIN`, `MAX` and `-1`, through functions the bounds
/// put on the machine path and functions that reach the kernel, return the
/// interpreter's value and charges. `a rem -1` and `a mod -1` over `MIN` are
/// on the machine path and must not trap.
#[trace("TC-1385")]
#[test]
fn division_under_every_law_agrees_with_the_interpreter() {
    use qsl_semantics::value::DefinitionRole;
    const MIN: i64 = i64::MIN;
    const MAX: i64 = i64::MAX;
    let source = format!(
        "\
 function dq using v(a: Int[-9, 9], b: Int[-9, -1]): Integer pure {{ a div b }}\n\
 function dp using v(a: Int[-9, 9], b: Int[1, 9]): Integer pure {{ a div b }}\n\
 function rq using v(a: Int[-9, 9], b: Int[-9, -1]): Integer pure {{ a rem b }}\n\
 function rp using v(a: Int[-9, 9], b: Int[1, 9]): Integer pure {{ a rem b }}\n\
 function mq using v(a: Int[-9, 9], b: Int[-9, -1]): Integer pure {{ a mod b }}\n\
 function mp using v(a: Int[-9, 9], b: Int[1, 9]): Integer pure {{ a mod b }}\n\
 function dwide using v(a: Int[{MIN}, {MAX}], b: Int[2, {MAX}]): Integer pure {{ a div b }}\n\
 function dwiden using v(a: Int[{MIN}, {MAX}], b: Int[{MIN}, -2]): Integer pure {{ a div b }}\n\
 function rwide using v(a: Int[{MIN}, {MAX}], b: Int[{MIN}, -1]): Integer pure {{ a rem b }}\n\
 function mwide using v(a: Int[{MIN}, {MAX}], b: Int[{MIN}, -1]): Integer pure {{ a mod b }}\n\
 function rwidep using v(a: Int[{MIN}, {MAX}], b: Int[1, {MAX}]): Integer pure {{ a rem b }}\n\
 function mwidep using v(a: Int[{MIN}, {MAX}], b: Int[1, {MAX}]): Integer pure {{ a mod b }}\n\
 function rone using v(a: Int[{MIN}, 0], b: Int[-1, -1]): Integer pure {{ a rem b }}\n\
 function mone using v(a: Int[{MIN}, 0], b: Int[-1, -1]): Integer pure {{ a mod b }}\n\
 function done using v(a: Int[{MIN}, 0], b: Int[-1, -1]): Integer pure {{ a div b }}\n\
 function dzero using v(a: Int[-9, 9], b: Int[-9, 9]): Integer pure {{ a div b }}\n"
    );
    let small = |low: i64, high: i64| -> Vec<Vec<Value>> {
        (-9..=9)
            .flat_map(|a| (low..=high).map(move |b| vec![int(a), int(b)]))
            .collect()
    };
    let edges = |a: &[i64], b: &[i64]| -> Vec<Vec<Value>> {
        a.iter()
            .flat_map(|a| b.iter().map(move |b| vec![int(*a), int(*b)]))
            .collect()
    };
    let cases: Vec<(&str, Vec<Vec<Value>>)> = vec![
        ("dq", small(-9, -1)),
        ("dp", small(1, 9)),
        ("rq", small(-9, -1)),
        ("rp", small(1, 9)),
        ("mq", small(-9, -1)),
        ("mp", small(1, 9)),
        ("dwide", edges(&[MIN, -1, 0, 1, MAX], &[2, 3, MAX])),
        ("dwiden", edges(&[MIN, -1, 0, 1, MAX], &[MIN, -3, -2])),
        ("rwide", edges(&[MIN, -1, 0, 1, MAX], &[MIN, -3, -1])),
        ("mwide", edges(&[MIN, -1, 0, 1, MAX], &[MIN, -3, -1])),
        ("rwidep", edges(&[MIN, -1, 0, 1, MAX], &[1, 3, MAX])),
        ("mwidep", edges(&[MIN, -1, 0, 1, MAX], &[1, 3, MAX])),
        ("rone", edges(&[MIN, MIN + 1, -7, 0], &[-1])),
        ("mone", edges(&[MIN, MIN + 1, -7, 0], &[-1])),
        ("done", edges(&[MIN, MIN + 1, -7, 0], &[-1])),
        ("dzero", small(-9, 9)),
    ];
    let functions: Vec<&str> = cases.iter().map(|(function, _)| *function).collect();
    for law in [
        DefinitionRole::IntegerDivisionFloor,
        DefinitionRole::IntegerDivisionTruncating,
        DefinitionRole::IntegerDivisionEuclidean,
    ] {
        let package = crate::common::package_with_law(&source, law);
        let prepared = prepare(&package, &functions);
        for (function, tuples) in &cases {
            for arguments in tuples {
                let want = interpreted(&package, function, arguments, UNLIMITED);
                let got = run(&prepared, function, arguments);
                assert_eq!(got.result, want.result, "{law:?} {function} {arguments:?}");
                assert_eq!(
                    got.charges, want.charges,
                    "{law:?} {function} {arguments:?}"
                );
            }
            // The operations the bounds put on the machine path record no
            // kernel call; the others (a possible zero divisor, `MIN / -1`)
            // record one.
            let kernel = matches!(*function, "done" | "dzero");
            let first = &tuples[0];
            assert_eq!(
                run(&prepared, function, first).calls.is_empty(),
                !kernel,
                "{law:?} {function}"
            );
        }
    }
}

/// Checked under `CheckMode::Kernel` the result bound of `s` is a run-time
/// obligation: the addition records no kernel call, and a sum outside the
/// bound returns the interpreter's outcome, cause and location.
#[trace("TC-1384", "FR-562-AC-6")]
#[test]
fn a_result_bound_checked_under_kernel_mode_is_the_interpreters_outcome() {
    let package = package_kernel(
        " function s using v(a: Int[0, 9], b: Int[0, 9]): Int[0, 9] pure { a + b }\n",
    );
    let prepared = prepare(&package, &["s"]);
    for arguments in [[int(5), int(5)], [int(4), int(5)]] {
        let got = run(&prepared, "s", &arguments);
        assert!(names(&got).is_empty(), "{arguments:?}");
        let want = interpreted(&package, "s", &arguments, UNLIMITED);
        assert_eq!(got.result, want.result, "{arguments:?}");
        assert_eq!(got.charges, want.charges, "{arguments:?}");
    }
    let over = run(&prepared, "s", &[int(5), int(5)]);
    assert!(matches!(over.outcome, Ok(None)), "{}", over.result);
    let within = run(&prepared, "s", &[int(4), int(5)]);
    assert!(matches!(
        within.outcome,
        Ok(Some(Value::Integer(ref sum))) if *sum == Integer::from(9_i64)
    ));
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1825, FR-560, FR-561, FR-562: collection values run through the kernel
//! on arena handles, built, queried, folded and sequenced by the shared
//! iteration the interpreter drives, and return the interpreter's value,
//! outcome, location, losses and charges.

use qsl_eval::value::CheckedPackageEvaluation;
use qsl_package::CheckedPackage;
use quire_eval::FamilyOutcome;
use quire_exact::{admit_text, Cancel, Meter, Outcome, TextPayload, TextProfile, TextType, Value};
use quire_semantic_value::supplied_stop::{SuppliedLimits, SuppliedRun};

use crate::common::{
    int, interpreted, jitted, name, package_of, prepare, unsupported_names, UNLIMITED,
};
use ix_trace_rs::trace;

const SOURCE: &str = "\
 type Total = Integer;\n\
 type Small = Int[0, 9];\n\
 type Flag = Boolean;\n\
 record P { x: Int[0, 9]; y: Boolean; }\n\
 function digits using v(): Sequence<Int[0, 9]>[0, 6] pure { sequence[3, 1, 4, 1, 5] }\n\
 function nothing using v(): Sequence<Int[0, 9]>[0, 6] pure { sequence[] }\n\
 function wide using v(): Sequence<Integer>[0, 6] pure { sequence[100000000000000000000, 3, 100000000000000000000] }\n\
 function flags using v(): Sequence<Boolean>[0, 6] pure { sequence[true, false, true] }\n\
 function names using v(a: Text[0, 16; nfd], b: Text[0, 16; nfd]): Sequence<Text[0, 16; nfd]>[0, 6] pure { sequence[a, b] }\n\
 function one using v(): Sequence<Int[0, 9]>[0, 1] pure { sequence[7] }\n\
 function oneless using v(): Sequence<Int[0, 9]>[0, 1] pure { sequence[] }\n\
 function widenz using v(): Sequence<Integer>[1, 6] pure { sequence[100000000000000000000, 3] }\n\
 function nest using v(): Sequence<Sequence<Int[0, 9]>[0, 3]>[0, 3] pure { sequence[sequence[1, 2], sequence[], sequence[3]] }\n\
 function lit using v(a: Int[0, 9], b: Int[0, 9]): Sequence<Int[0, 9]>[0, 4] pure { sequence[a, b, a] }\n\
 function lits using v(a: Int[0, 9]): Set<Int[0, 9]>[0, 4] pure { set[a, 1, a] }\n\
 function bagof using v(a: Int[0, 9]): Bag<Int[0, 9]>[0, 4] pure { bag[a, 1, a] }\n\
 function ordset using v(a: Int[0, 9]): OrderedSet<Int[0, 9]>[0, 4] pure { orderedSet[a, 1, a] }\n\
 function toolong using v(a: Int[0, 9]): Sequence<Int[0, 9]>[0, 2] pure { sequence[a, 1, a] }\n\
 function sz using v(s: Sequence<Int[0, 9]>[0, 6]): Integer pure { size(s) }\n\
 function has using v(s: Sequence<Int[0, 9]>[0, 6], x: Int[0, 9]): Boolean pure { contains(s, x) }\n\
 function hasbag using v(b: Bag<Int[0, 9]>[0, 6], x: Int[0, 9]): Boolean pure { contains(b, x) }\n\
 function hastext using v(s: Sequence<Text[0, 16; nfd]>[0, 6], x: Text[0, 16; nfd]): Boolean pure { contains(s, x) }\n\
 function mapped using v(s: Sequence<Int[0, 9]>[0, 6]): Sequence<Integer>[0, 6] pure { map(x in s: x * x) }\n\
 function kept using v(s: Sequence<Int[0, 9]>[0, 6]): Sequence<Int[0, 9]>[0, 6] pure { filter(x in s: x > 3) }\n\
 function counted using v(s: Sequence<Int[0, 9]>[0, 6]): Total pure { count<Total>(x in s: x > 3) }\n\
 function countbits using v(s: Sequence<Boolean>[0, 6]): Total pure { count<Total>(x in s: x) }\n\
 function keptbits using v(s: Sequence<Boolean>[0, 6]): Sequence<Boolean>[0, 6] pure { filter(x in s: x) }\n\
 function summed using v(s: Sequence<Int[0, 9]>[0, 6]): Total pure { sum<Total>(x in s: x) }\n\
 function summedone using v(s: Sequence<Int[0, 9]>[0, 1]): Small pure { sum<Small>(x in s: x) }\n\
 function summedwide using v(s: Sequence<Integer>[0, 6]): Total pure { sum<Total>(x in s: x) }\n\
 function folded using v(s: Sequence<Int[0, 9]>[0, 6]): Total pure { fold<Total>(acc, x in s: acc * 3 + x, identity: 1) }\n\
 function reduced using v(s: Sequence<Integer>[1, 6]): Total pure { reduce<Total>(acc, x in s: acc + x) }\n\
 function foldbool using v(s: Sequence<Boolean>[0, 6]): Boolean pure { fold<Flag>(acc, x in s: acc and x, identity: true) }\n\
 function flat using v(s: Sequence<Sequence<Int[0, 9]>[0, 3]>[0, 3]): Sequence<Int[0, 9]>[0, 9] pure { flatten(s) }\n\
 function conv using v(s: Sequence<Int[0, 9]>[0, 6]): Set<Int[0, 9]>[0, 6] pure { convert<Set<Int[0, 9]>[0, 6]>(s) }\n\
 function convbag using v(s: Sequence<Int[0, 9]>[0, 6]): Bag<Int[0, 9]>[0, 6] pure { convert<Bag<Int[0, 9]>[0, 6]>(s) }\n\
 function dropped using v(s: Sequence<Int[0, 9]>[0, 6]): Sequence<Int[0, 9]>[0, 5] pure { dropFirst(s) }\n\
 function removed using v(s: Sequence<Int[0, 9]>[0, 6], x: Int[0, 9]): Sequence<Int[0, 9]>[0, 6] pure { removeOne(s, x) }\n\
 function same using v(s: Sequence<Int[0, 9]>[0, 6], t: Sequence<Int[0, 9]>[0, 6]): Boolean pure { s = t }\n\
 function pick using v(c: Boolean, s: Sequence<Int[0, 9]>[0, 6], t: Sequence<Int[0, 9]>[0, 6]): Sequence<Int[0, 9]>[0, 6] pure { if c then s else t }\n\
 function square using v(x: Int[0, 9]): Integer pure { x * x }\n\
 function viacall using v(s: Sequence<Int[0, 9]>[0, 6]): Sequence<Integer>[0, 6] pure { map(x in s: square(x)) }\n\
 function inner using v(s: Sequence<Int[0, 9]>[0, 6]): Sequence<Integer>[0, 6] pure { map(x in s: sum<Total>(y in s: x * y)) }\n\
 function mapped_text using v(s: Sequence<Text[0, 16; nfd]>[0, 6]): Sequence<Text[0, 16; nfd]>[0, 6] pure { map(x in s: x) }\n\
 function mapped_bits using v(s: Sequence<Boolean>[0, 6]): Sequence<Boolean>[0, 6] pure { map(x in s: not x) }\n\
 function widemap using v(s: Sequence<Integer>[0, 6]): Sequence<Integer>[0, 6] pure { map(x in s: x * x) }\n\
 function lastdigit using v(s: Sequence<Int[0, 9]>[0, 6]): Total pure { sum<Total>(x in dropFirst(s): x) }\n\
 function built using v(a: Int[0, 9]): Total pure { sum<Total>(x in lit(a, 3): x) }\n\
 function buildrec using v(a: Int[0, 9]): Sequence<P>[0, 2] pure { sequence[P { x: a, y: true }, P { x: 1, y: false }] }\n\
 function nowide using v(): Sequence<Integer>[0, 6] pure { sequence[] }\n\
 function recmap using v(s: Sequence<P>[0, 2]): Sequence<Int[0, 9]>[0, 2] pure { map(p in s: p.x) }\n";

const FUNCTIONS: [&str; 42] = [
    "lit",
    "lits",
    "bagof",
    "ordset",
    "toolong",
    "sz",
    "has",
    "hasbag",
    "hastext",
    "mapped",
    "kept",
    "counted",
    "countbits",
    "keptbits",
    "summed",
    "summedone",
    "summedwide",
    "folded",
    "reduced",
    "foldbool",
    "flat",
    "conv",
    "convbag",
    "dropped",
    "removed",
    "same",
    "pick",
    "viacall",
    "inner",
    "mapped_text",
    "mapped_bits",
    "widemap",
    "lastdigit",
    "built",
    "buildrec",
    "recmap",
    "digits",
    "nothing",
    "wide",
    "flags",
    "nest",
    "square",
];

/// What the interpreter evaluates the no-argument `function` to.
fn value_of(package: &CheckedPackage, function: &str) -> Value {
    value_with(package, function, Vec::new())
}

fn value_with(package: &CheckedPackage, function: &str, arguments: Vec<Value>) -> Value {
    let mut meter = Meter::new(UNLIMITED);
    let mut run = SuppliedRun::new(SuppliedLimits::default(), Cancel::new()).expect("a run");
    let evaluation = package
        .call_supplied(
            &name(function),
            arguments,
            &Default::default(),
            &mut meter,
            &mut run,
        )
        .expect("the helper evaluates");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("{function} gave {other:?}"),
    }
}

fn text(payload: &str) -> Value {
    Value::Text(
        admit_text(
            &TextPayload::from_utf8(payload.as_bytes()).expect("valid UTF-8"),
            &TextType::new(0, 16, TextProfile::Nfd).expect("a text type"),
            &mut Meter::new(UNLIMITED),
        )
        .completed()
        .expect("the text admits"),
    )
}

/// The argument tuples a function is compared on.
fn arguments_of(package: &CheckedPackage, function: &str) -> Vec<Vec<Value>> {
    let digits = value_of(package, "digits");
    let nothing = value_of(package, "nothing");
    let wide = value_of(package, "wide");
    let bits = value_of(package, "flags");
    let names = value_with(package, "names", vec![text("ab"), text("c")]);
    let nest = value_of(package, "nest");
    let small = || (0..10).map(int);
    let ints = || [digits.clone(), nothing.clone()];
    match function {
        "lit" => small()
            .flat_map(|a| [int(0), int(9)].map(|b| vec![a.clone(), b]))
            .collect(),
        "lits" | "bagof" | "ordset" | "toolong" | "built" | "buildrec" => {
            small().map(|a| vec![a]).collect()
        }
        "summedone" => vec![
            vec![value_of(package, "one")],
            vec![value_of(package, "oneless")],
        ],
        "reduced" => vec![vec![value_of(package, "widenz")]],
        "sz" | "mapped" | "kept" | "counted" | "summed" | "folded" | "conv" | "convbag"
        | "dropped" | "viacall" | "inner" | "lastdigit" => {
            ints().into_iter().map(|s| vec![s]).collect()
        }
        "has" | "removed" => ints()
            .into_iter()
            .flat_map(|s| [int(1), int(2), int(5)].map(|x| vec![s.clone(), x]))
            .collect(),
        "hasbag" => vec![
            vec![value_with(package, "bagof", vec![int(1)]), int(1)],
            vec![value_with(package, "bagof", vec![int(1)]), int(2)],
        ],
        "hastext" => vec![
            vec![names.clone(), text("ab")],
            vec![names.clone(), text("zz")],
        ],
        "countbits" | "keptbits" | "foldbool" | "mapped_bits" => vec![vec![bits.clone()]],
        "summedwide" | "widemap" => vec![vec![wide.clone()], vec![value_of(package, "nowide")]],
        "flat" => vec![vec![nest.clone()]],
        "same" => vec![
            vec![digits.clone(), digits.clone()],
            vec![digits.clone(), nothing.clone()],
        ],
        "pick" => [true, false]
            .iter()
            .map(|c| vec![Value::Boolean(*c), digits.clone(), nothing.clone()])
            .collect(),
        "mapped_text" => vec![vec![names.clone()]],
        "recmap" => vec![vec![value_of_rec(package)]],
        "square" => small().map(|a| vec![a]).collect(),
        _ => vec![Vec::new()],
    }
}

fn value_of_rec(package: &CheckedPackage) -> Value {
    let mut meter = Meter::new(UNLIMITED);
    let mut run = SuppliedRun::new(SuppliedLimits::default(), Cancel::new()).expect("a run");
    let evaluation = package
        .call_supplied(
            &name("buildrec"),
            vec![int(4)],
            &Default::default(),
            &mut meter,
            &mut run,
        )
        .expect("buildrec evaluates");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("buildrec gave {other:?}"),
    }
}

/// Every function is prepared, and each call returns the interpreter's
/// outcome and charges, over the construction, query, fold, sequence and
/// conversion forms and the element types they carry.
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
    assert!(compared > 150, "{compared}");
}

/// With `work_units` at each value below the interpreter's spend, the JIT
/// stops at the same charge point and reports the same stop.
#[trace("TC-1385")]
#[test]
fn a_denied_charge_stops_the_jit_where_it_stops_the_interpreter() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &FUNCTIONS);
    for function in FUNCTIONS {
        let arguments = arguments_of(&package, function).remove(0);
        for below in 0..48_u64 {
            let mut limits = UNLIMITED;
            limits.work_units = below;
            let want = interpreted(&package, function, &arguments, limits);
            let got = jitted(&prepared, function, &arguments, limits);
            assert_eq!(got.result, want.result, "{function} work_units={below}");
            assert_eq!(got.charges, want.charges, "{function} work_units={below}");
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1825: the call arena holds the live handles of the frames being run,
//! not every value the call ever made. A callee's intermediates are dropped
//! when it returns, as the interpreter drops them, so the arena is bounded by
//! the size of the program, not by the number of calls it makes.

use crate::common::{interpreted, jitted, package_of, prepare, UNLIMITED};

/// `l11` makes 2^11 calls of `l0`, each building two `Integer` values wider
/// than a machine word, from a program of twelve small functions.
const SOURCE: &str = "\
 function l0 using v(): Integer pure { 170141183460469231731687303715884105727 * 3 }\n\
  function l1 using v(): Integer pure { l0() + l0() }\n\
  function l2 using v(): Integer pure { l1() + l1() }\n\
  function l3 using v(): Integer pure { l2() + l2() }\n\
  function l4 using v(): Integer pure { l3() + l3() }\n\
  function l5 using v(): Integer pure { l4() + l4() }\n\
  function l6 using v(): Integer pure { l5() + l5() }\n\
  function l7 using v(): Integer pure { l6() + l6() }\n\
  function l8 using v(): Integer pure { l7() + l7() }\n\
  function l9 using v(): Integer pure { l8() + l8() }\n\
  function l10 using v(): Integer pure { l9() + l9() }\n\
  function l11 using v(): Integer pure { l10() + l10() }\n\
";

#[test]
fn the_arena_stays_bounded_by_the_program_not_by_the_number_of_calls() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &["l11"]);
    let want = interpreted(&package, "l11", &[], UNLIMITED);
    let got = jitted(&prepared, "l11", &[], UNLIMITED);
    assert_eq!(got.result, want.result);
    assert_eq!(got.charges, want.charges);
    // Without reclamation the arena would hold about 3 * 2^11 values.
    let peak = prepared.arena_peak_for_test();
    assert!(peak <= 3 * 12, "{peak}");
}

const LOOPS: &str = "\
 type Total = Integer;\n\
 function bigs using v(s: Sequence<Integer>[0, 400]): Total pure { sum<Total>(x in map(y in s: y * y * y): x + x) }\n\
 function nested using v(s: Sequence<Integer>[0, 400]): Total pure { sum<Total>(x in s: sum<Total>(y in dropFirst(s): x * y * 170141183460469231731687303715884105727)) }\n\
 function folded using v(s: Sequence<Integer>[0, 400]): Total pure { fold<Total>(acc, x in s: acc * x + 170141183460469231731687303715884105727, identity: 1) }\n";

/// A sequence of `count` `Integer`s wider than a machine word.
fn integers(count: usize) -> quire_exact::Value {
    use quire_exact::{
        form, CardinalityBound, CollectionKind, CollectionType, Integer, Meter, Outcome, Value,
        ValueType,
    };
    let elements = (0..count)
        .map(|n| Value::Integer(Integer::from((1_i128 << 70) + n as i128)))
        .collect();
    let kind = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Integer,
        Some(CardinalityBound::at_most(400)),
    );
    match form(&kind, elements, &mut Meter::new(UNLIMITED)) {
        Outcome::Completed(value) => value,
        other => panic!("the sequence forms: {other:?}"),
    }
}

/// A loop runs its body once per element, but cuts the arena back to its
/// start before each visit, so the peak does not grow with the element
/// count.
#[test]
fn a_loop_keeps_the_arena_bounded_whatever_the_element_count() {
    let package = package_of(LOOPS);
    let prepared = prepare(&package, &["bigs", "nested", "folded"]);
    for function in ["bigs", "nested", "folded"] {
        let mut peaks = Vec::new();
        for count in [2_usize, 40, 160] {
            let arguments = [integers(count)];
            let want = interpreted(&package, function, &arguments, UNLIMITED);
            let got = jitted(&prepared, function, &arguments, UNLIMITED);
            assert_eq!(got.result, want.result, "{function} {count}");
            assert_eq!(got.charges, want.charges, "{function} {count} charges");
            peaks.push(prepared.arena_peak_for_test());
        }
        assert_eq!(peaks[0], peaks[1], "{function} {peaks:?}");
        assert_eq!(peaks[1], peaks[2], "{function} {peaks:?}");
    }
}

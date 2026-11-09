// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1825, FR-495, FR-562-AC-6: under the package-level
//! `CheckMode::Kernel` check each unproved definedness obligation of a
//! function body is left to the kernel evaluation, while the same unit
//! refuses under `CheckMode::Linked`.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use quire_semantic_value::checking::CheckMode;

use crate::common::refusal_causes;

/// A unit whose single function has an unproved obligation, with the
/// `unproved-*` cause `Linked` refuses it with.
const OBLIGATIONS: [(&str, &str, &str); 5] = [
    (
        "decimal nonzero",
        " type D = Decimal[-1000, 1000; 0, 2; nearest-even];\n\
         function f using v(a: D, b: D): D pure { a / b }\n",
        "unproved-nonzero",
    ),
    (
        "presence",
        " record W { o: Int[0, 9]?; }\n\
         function f using v(w: W): Int[0, 9] pure { value(w.o) }\n",
        "unproved-presence",
    ),
    (
        "rational range",
        " type Q = Rational[-10, 10; 1, 10];\n\
         function f using v(a: Q, b: Q): Q pure { a * b }\n",
        "unproved-range",
    ),
    (
        "rational nonzero",
        " type Q = Rational[-10, 10; 1, 10];\n\
         function f using v(a: Q, b: Q): Q pure { a / b }\n",
        "unproved-nonzero",
    ),
    (
        "non-empty reduction",
        " type N = Int[0, 9];\n\
         function f using v(s: Sequence<N>[0, 3]): N pure { reduce<N>(acc, x in s: if acc > x then acc else x) }\n",
        "unproved-range",
    ),
];

/// Each obligation `Linked` refuses as `undefined_expression` is not a
/// refusal under `Kernel`.
#[trace("TC-1284", "FR-495-AC-3")]
#[test]
fn each_unproved_obligation_is_left_to_the_kernel_under_kernel_mode() {
    for (label, source, cause) in OBLIGATIONS {
        assert_eq!(
            refusal_causes(source, CheckMode::Linked),
            [(Code::UndefinedExpression, Some(cause))],
            "{label} refuses under Linked"
        );
        assert_eq!(
            refusal_causes(source, CheckMode::Kernel),
            [],
            "{label} checks under Kernel"
        );
    }
}

/// The termination measure is not a run-time obligation: its unproved
/// obligation refuses under `Kernel` as it does under `Linked`.
#[trace("TC-1284", "FR-495-AC-3")]
#[test]
fn an_unproved_measure_obligation_still_refuses_under_kernel_mode() {
    let source = " record W { o: Int[0, 9]?; }\n\
         function f using v(w: W): Int[0, 9] pure decreases(value(w.o)) { 0 }\n";
    let refused = [(Code::UndefinedExpression, Some("unproved-presence"))];
    assert_eq!(refusal_causes(source, CheckMode::Linked), refused);
    assert_eq!(refusal_causes(source, CheckMode::Kernel), refused);
}

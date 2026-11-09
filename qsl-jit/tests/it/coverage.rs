// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: the coverage the JIT publishes and the disposition of every entry
//! outside it.

use ix_trace_rs::trace;
use qsl_eval::execution::{CoverageOperation, CoverageType, PreparedEntries};
use quire_eval::{CallFailure, ExecutionBackendChoice, UnsupportedCause, UntranslatableConstruct};
use quire_exact::{Integer, IntegerInterval, ValueType};

use crate::common::{int, jitted_call, package_of, prepare, unsupported_names};

/// FR-560-AC-1: the seven floor forms, the floor value types and every
/// `Value`-family operator over them, and none of the constructs the floor
/// excludes.
#[trace("TC-1380", "FR-560-AC-1")]
#[test]
fn the_published_coverage_lists_the_floor_forms_types_and_operations() {
    let coverage = qsl_jit::coverage();
    for form in [
        "literal",
        "operator",
        "let",
        "if",
        "call",
        "record",
        "collection",
    ] {
        assert!(coverage.lists_form(form), "{form}");
    }
    for excluded in [
        "forall", "exists", "present", "deref", "field", "reaches", "self", "case",
    ] {
        assert!(!coverage.lists_form(excluded), "{excluded}");
    }
    for listed in [
        ValueType::Boolean,
        ValueType::Integer,
        ValueType::Int(
            IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64))
                .expect("a non-empty interval"),
        ),
    ] {
        assert!(coverage.lists_type(&listed), "{listed:?}");
    }
    assert!(!coverage.lists_type(&ValueType::Timestamp));
    let classes = [
        CoverageType::Boolean,
        CoverageType::BoundedInteger,
        CoverageType::UnboundedInteger,
        CoverageType::Rational,
        CoverageType::Decimal,
        CoverageType::Text,
        CoverageType::Record,
        CoverageType::Collection,
    ];
    for class in classes {
        assert!(coverage.value_types().contains(&class), "{class:?}");
    }
    let operators: [(CoverageType, &[&'static str]); 8] = [
        (CoverageType::BoundedInteger, &INTEGER),
        (CoverageType::UnboundedInteger, &INTEGER),
        (
            CoverageType::Boolean,
            &["and", "or", "implies", "not", "=", "!="],
        ),
        (CoverageType::Rational, &NUMERIC),
        (CoverageType::Decimal, &NUMERIC),
        (CoverageType::Text, &["<", "<=", ">", ">=", "=", "!="]),
        (CoverageType::Record, &["=", "!="]),
        (
            CoverageType::Collection,
            &[
                "=",
                "!=",
                "size",
                "contains",
                "flatten",
                "convert",
                "drop-first",
                "remove-one",
            ],
        ),
    ];
    for (operand, names) in operators {
        for operator in names {
            let operation = CoverageOperation { operator, operand };
            assert!(coverage.lists_operation(&operation), "{operation:?}");
        }
        let present = CoverageOperation {
            operator: "present",
            operand,
        };
        assert!(!coverage.lists_operation(&present), "{present:?}");
    }
}

const INTEGER: [&str; 15] = [
    "+", "-", "*", "negate", "div", "rem", "mod", "/", "<", "<=", ">", ">=", "coerce", "=", "!=",
];

const NUMERIC: [&str; 12] = [
    "+", "-", "*", "/", "negate", "<", "<=", ">", ">=", "=", "!=", "convert",
];

/// FR-560-AC-2: `add`, `half` and `name` are prepared, none listed
/// unsupported.
#[trace("TC-1380", "FR-560-AC-2")]
#[test]
fn integer_rational_and_text_entries_of_the_floor_are_not_listed_unsupported() {
    let package = package_of(
        "\
 function add using v(a: Integer, b: Integer): Integer pure { a + b }\n\
 function half using v(a: Int[0, 9]): Rational[0, 9; 1, 2] pure { a / 2 }\n\
 function name using v(t: Text[0, 16; nfd]): Text[0, 16; nfd] pure { t }\n",
    );
    let prepared = prepare(&package, &["add", "half", "name"]);
    assert_eq!(unsupported_names(&prepared), Vec::<String>::new());
}

const SOURCE: &str = "\
 function ok using v(a: Int[0, 9]): Integer pure { a + 1 }\n\
 function stamped using v(t: Timestamp): Boolean pure { true }\n\
 function text using v(t: Text[0, 4; nfd]): Boolean pure { true }\n\
 function wide using v(a: Int[0, 9223372036854775807], b: Int[0, 9223372036854775807]): Integer pure { a + b }\n";

#[trace("TC-1381")]
#[test]
fn a_parameter_of_an_unlisted_type_is_listed_with_that_type_and_the_rest_are_prepared() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &["ok", "stamped", "ok"]);
    assert_eq!(unsupported_names(&prepared), vec!["stamped".to_owned()]);
    let (_, entry) = &prepared.unsupported()[0];
    assert_eq!(entry.backend, ExecutionBackendChoice::Jit);
    let UnsupportedCause::Untranslatable { construct, .. } = &entry.cause else {
        panic!("expected Untranslatable, got {:?}", entry.cause);
    };
    assert_eq!(
        *construct,
        UntranslatableConstruct::ValueType(Box::new(ValueType::Timestamp))
    );
    let called = jitted_call(&prepared, "stamped", &[]);
    assert!(matches!(called, Err(CallFailure::Unsupported(_))));
    assert!(jitted_call(&prepared, "ok", &[int(4)]).is_ok());
}

/// QSL-1825: a sum whose result needs more than 64 bits is no longer listed
/// unsupported; the kernel computes it on a handle.
#[test]
fn an_operation_whose_result_does_not_fit_64_bits_is_prepared_on_a_handle() {
    let package = package_of(SOURCE);
    let prepared = prepare(&package, &["wide"]);
    assert_eq!(unsupported_names(&prepared), Vec::<String>::new());
}

/// FR-560-AC-2: over the 10,000 generated units at the default seed, with
/// every function selected, the JIT lists unsupported only entries whose
/// first outside construct is one the table names, and no entry whose
/// constructs all lie in the coverage.
#[trace("TC-1380", "FR-560-AC-2")]
#[test]
fn no_generated_entry_inside_the_coverage_is_listed_unsupported() {
    for (unit, package) in crate::common::generated_units() {
        let functions: Vec<&str> = unit.functions.iter().map(|(n, _)| n.as_str()).collect();
        let prepared = prepare(package, &functions);
        assert!(
            prepared.unsupported().is_empty(),
            "unit {}: {:?}",
            unit.index,
            prepared.unsupported()
        );
    }
}

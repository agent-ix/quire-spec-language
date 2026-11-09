// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-560: the coverage the JIT publishes and the disposition of every entry
//! outside it.

use ix_trace_rs::trace;
use qsl_eval::execution::PreparedEntries;
use quire_eval::{CallFailure, ExecutionBackendChoice, UnsupportedCause, UntranslatableConstruct};
use quire_exact::ValueType;

use crate::common::{int, jitted_call, package_of, prepare, unsupported_names};

#[trace("TC-1380")]
#[test]
fn the_published_coverage_lists_the_implemented_scalar_subset() {
    let coverage = qsl_jit::coverage();
    for form in ["literal", "operator", "let", "if"] {
        assert!(coverage.lists_form(form), "{form}");
    }
    assert!(coverage.lists_type(&ValueType::Boolean));
    assert!(coverage.lists_type(&ValueType::Integer));
    assert!(!coverage.lists_type(&ValueType::Timestamp));
    assert!(!coverage.lists_form("case"));
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

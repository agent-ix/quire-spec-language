// SPDX-License-Identifier: AGPL-3.0-or-later
//! Domain-package intake refuses an integer with no exact double as
//! `noncanonical_wire`/`inexact-integer`.

use ix_trace_rs::trace;

use qsl_semantics::model::intake::{admit, package_input};
use qsl_semantics::model::normalize::{ModelRefusal, ModelRefusalCause};

/// Admit `raw`: refused with the refusal, or `None` when `raw` admits.
fn refusal_of(raw: &str) -> Option<ModelRefusal> {
    let packages = package_input([raw.as_bytes()]).map_err(Some);
    let packages = match packages {
        Ok(packages) => packages,
        Err(refusal) => return refusal,
    };
    admit(
        "test/golden",
        &packages,
        qsl_foundation::IntakeLimits::default(),
    )
    .err()
}

/// `raw` refuses `noncanonical_wire` with cause `inexact-integer` at `/n`.
fn assert_inexact_integer_at_n(raw: &str, what: &str) {
    let refusal = refusal_of(raw).unwrap_or_else(|| panic!("{what} admitted"));
    assert_eq!(
        refusal.code,
        qsl_foundation::diagnostic::Code::NoncanonicalWire,
        "{what}"
    );
    assert!(
        matches!(
            &refusal.cause,
            ModelRefusalCause::NoncanonicalNumber { inexact, document_pointer }
                if inexact.as_str() == "inexact-integer" && document_pointer.as_str() == "/n"
        ),
        "{what}: {:?}",
        refusal.cause
    );
}

fn big_integer_document(literal: &str) -> String {
    format!(r#"{{"package":{{"identity":"test/golden","version":"1"}},"n":{literal}}}"#)
}

/// An integer outside ±2^53 has no exact double, so intake refuses it as
/// `noncanonical_wire`/`inexact-integer`, whatever its spelling.
#[trace("TC-145", "FR-056-AC-13")]
#[test]
fn big_integers_are_refused_as_inexact() {
    for literal in [
        "18446744073709551615",
        "18446744073709551616",
        "123456789012345678901234567890",
        "-9007199254740993",
    ] {
        assert_inexact_integer_at_n(&big_integer_document(literal), literal);
    }
}

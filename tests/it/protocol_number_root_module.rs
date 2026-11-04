// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-511: the exact protocol number codec is a root-level public module.

use ix_trace_rs::trace;
use quire_spec_language::number::{
    ExactInteger, ExactRational, NumberComponent, NumberError, NumberWire, ProtocolNumber,
    NUMERIC_PROFILE,
};

/// FR-511-AC-1 / TC-1306: the root import surface remains a compatibility
/// re-export of qsl-package's codec types and profile.
#[trace("FR-511-AC-1", "TC-1306")]
#[test]
fn root_number_module_reexports_the_spine_codec() {
    assert_eq!(NUMERIC_PROFILE, "quire.protocol.numeric/1");
    assert_eq!(NUMERIC_PROFILE, qsl_package::number::NUMERIC_PROFILE);
    let value = ProtocolNumber::Integer(ExactInteger::new(7));
    let spine_value: qsl_package::number::ProtocolNumber = value;
    assert_eq!(
        spine_value,
        qsl_package::number::ProtocolNumber::Integer(qsl_package::number::ExactInteger::new(7),)
    );
    assert_eq!(ExactRational::new(1, 3).unwrap().denominator(), 3);
    assert_eq!(
        ExactInteger::from_decimal("01"),
        Err(NumberError::NonCanonicalDecimal {
            component: NumberComponent::Decimal,
        })
    );
    let _: Option<NumberWire> = None;
}

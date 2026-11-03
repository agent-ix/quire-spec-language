// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-511: the exact protocol number codec is a root-level public module.

use ix_trace_rs::trace;
use quire_spec_language::number::{
    ExactInteger, ExactRational, NumberComponent, NumberError, NumberWire, ProtocolNumber,
    NUMERIC_PROFILE,
};

/// FR-511-AC-1..4 / TC-1306: the root import surface and retained codec contract.
#[trace("FR-511-AC-1", "FR-511-AC-2", "FR-511-AC-3", "FR-511-AC-4", "TC-1306")]
#[test]
fn root_number_module_publishes_the_complete_codec_surface() {
    assert_eq!(NUMERIC_PROFILE, "quire.protocol.numeric/1");
    assert_eq!(
        ProtocolNumber::Integer(ExactInteger::new(7)),
        ProtocolNumber::Integer(ExactInteger::new(7))
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

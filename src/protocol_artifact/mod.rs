// SPDX-License-Identifier: AGPL-3.0-only
//! Components of the compiled protocol artifact contract.
//!
//! Numeric admission preserves exact wire values. It does not establish model
//! admission, source compilation, or admission of a complete protocol artifact.

mod number;

pub use number::{
    ExactInteger, ExactRational, NumberComponent, NumberError, NumberWire, ProtocolNumber,
    NUMERIC_PROFILE,
};

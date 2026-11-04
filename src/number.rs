// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compatibility path for the exact protocol-number codec owned by qsl-package.
//!
//! Kept while consumers still import `quire_spec_language::number`; new spine
//! consumers use [`qsl_package::number`]. This module is a re-export only, so
//! the implementation and profile string have one source of truth.

pub use qsl_package::number::{
    ExactInteger, ExactRational, NumberComponent, NumberError, NumberWire, ProtocolNumber,
    NUMERIC_PROFILE,
};

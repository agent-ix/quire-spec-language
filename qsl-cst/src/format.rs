// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-255: the formatter's caller-selected output-byte setting.

use qsl_foundation::source::MAX_SOURCE_BYTES;
use qsl_foundation::{Setting, SettingLimits};

/// The published default formatter output-byte ceiling.
pub const OUTPUT_BYTE_CEILING: usize = MAX_SOURCE_BYTES;

/// Caller-selected limits for formatting, including every emitted byte.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatLimits {
    /// Inclusive output length, including comments, indentation and final LF.
    pub output_bytes: usize,
}

impl Default for FormatLimits {
    fn default() -> Self {
        Self {
            output_bytes: OUTPUT_BYTE_CEILING,
        }
    }
}

impl FormatLimits {
    const OUTPUT_SETTING: Setting = Setting::FormatOutputBytes;

    /// Replace the output-byte bound with the supplied value, including zero.
    pub fn with_output_bytes(mut self, output_bytes: usize) -> Self {
        self.output_bytes = output_bytes;
        self
    }
}

impl SettingLimits for FormatLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![(
            Self::OUTPUT_SETTING,
            u64::try_from(self.output_bytes).unwrap_or(u64::MAX),
        )]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        if setting != Self::OUTPUT_SETTING {
            return false;
        }
        self.output_bytes = usize::try_from(bound).unwrap_or(usize::MAX);
        true
    }
}

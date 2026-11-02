// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-260 Behavior 5 and FR-506: the limits a domain package document is read
//! under.

use crate::setting::{Setting, SettingLimits};
use alloc::{vec, vec::Vec};

/// The published default of `intake.input_bytes`: 64 MiB (FR-260 B5).
pub const DEFAULT_INTAKE_INPUT_BYTES: u64 = 67_108_864;

/// The published default of `model.sequence_maximum`: 10,000 items (FR-506).
pub const DEFAULT_SEQUENCE_MAXIMUM: u32 = 10_000;

/// The caller's limits on one domain package document: its bytes and its
/// largest declared collection maximum. Each bound is used as given; the
/// collection maximum takes at most `u32::MAX`, the width of a declared
/// maximum.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IntakeLimits {
    /// The most bytes one package document may hold (`intake.input_bytes`).
    pub input_bytes: u64,
    /// The largest collection maximum a package may declare
    /// (`model.sequence_maximum`, FR-506). Zero is a real limit.
    pub sequence_maximum: u32,
}

impl Default for IntakeLimits {
    fn default() -> Self {
        Self {
            input_bytes: DEFAULT_INTAKE_INPUT_BYTES,
            sequence_maximum: DEFAULT_SEQUENCE_MAXIMUM,
        }
    }
}

impl IntakeLimits {
    /// These limits with `intake.input_bytes` set to `bound`.
    #[must_use]
    pub const fn with_input_bytes(mut self, bound: u64) -> Self {
        self.input_bytes = bound;
        self
    }

    /// These limits with `model.sequence_maximum` set to `bound`.
    #[must_use]
    pub const fn with_sequence_maximum(mut self, bound: u32) -> Self {
        self.sequence_maximum = bound;
        self
    }
}

/// FR-255: the one mapping from each field to its setting.
impl SettingLimits for IntakeLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![
            (Setting::IntakeInputBytes, self.input_bytes),
            (
                Setting::ModelSequenceMaximum,
                u64::from(self.sequence_maximum),
            ),
        ]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        match setting {
            Setting::IntakeInputBytes => self.input_bytes = bound,
            // A bound over `u32::MAX` is refused by the settings operation
            // before it reaches a limits type; a request entry above it
            // saturates, as no declared maximum exceeds `u32::MAX`.
            Setting::ModelSequenceMaximum => {
                self.sequence_maximum = u32::try_from(bound).unwrap_or(u32::MAX);
            }
            _ => return false,
        }
        true
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-260 Behavior 5: the byte limit a domain package document is read under.

use crate::setting::{Setting, SettingLimits};

/// The published default of `intake.input_bytes`: 64 MiB (FR-260 B5).
pub const DEFAULT_INTAKE_INPUT_BYTES: u64 = 67_108_864;

/// The caller's limit on one domain package document's bytes. The bound is
/// used as given, with no ceiling.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct IntakeLimits {
    /// The most bytes one package document may hold (`intake.input_bytes`).
    pub input_bytes: u64,
}

impl Default for IntakeLimits {
    fn default() -> Self {
        Self {
            input_bytes: DEFAULT_INTAKE_INPUT_BYTES,
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
}

/// FR-255: the one mapping from each field to its setting.
impl SettingLimits for IntakeLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![(Setting::IntakeInputBytes, self.input_bytes)]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        match setting {
            Setting::IntakeInputBytes => self.input_bytes = bound,
            _ => return false,
        }
        true
    }
}

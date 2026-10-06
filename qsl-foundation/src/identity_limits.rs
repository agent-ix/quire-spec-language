// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-259 Behavior 3: the byte limit every identity is encoded under.

use crate::setting::{Setting, SettingLimits};

/// The published default of `identity.input_bytes`: 16 MiB (FR-259 B3).
pub const DEFAULT_IDENTITY_INPUT_BYTES: u64 = 16_777_216;

/// The caller's limit on one identity's canonical bytes. It is a byte limit
/// only: nesting depth costs bytes and is never a limit of its own, and the
/// bound is used as given, with no ceiling. A site whose stage applies its
/// own byte budget passes that budget instead of this one.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityLimits {
    /// The most canonical bytes one identity may encode to
    /// (`identity.input_bytes`).
    pub input_bytes: u64,
}

impl Default for IdentityLimits {
    fn default() -> Self {
        Self {
            input_bytes: DEFAULT_IDENTITY_INPUT_BYTES,
        }
    }
}

impl IdentityLimits {
    /// These limits with `identity.input_bytes` set to `bound`.
    #[must_use]
    pub const fn with_input_bytes(mut self, bound: u64) -> Self {
        self.input_bytes = bound;
        self
    }
}

/// FR-255: the one mapping from each field to its setting.
impl SettingLimits for IdentityLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![(Setting::IdentityInputBytes, self.input_bytes)]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        match setting {
            Setting::IdentityInputBytes => self.input_bytes = bound,
            _ => return false,
        }
        true
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! The configured reader bound every #231 envelope's decoder enforces
//! before admitting an oversized encoding (FR-069-AC-4, FR-070-AC-7,
//! FR-071-AC-7, FR-072-AC-5), as the `replay.input_bytes` setting (FR-255).
//!
//! Every reader in this crate checks its input's encoded byte length
//! against a [`ReplayLimits`] before parsing any other member, and refuses
//! with [`BoundExceeded`] rather than returning a truncated or
//! partially-populated value. One limits value serves all four envelopes: the
//! four share one "configured reader bound" per their acceptance criteria.
//! The caller sets it with [`ReplayLimits::with_input_bytes`] or through the
//! settings operation; a request's own `stage_limits` does not carry it.

use qsl_foundation::diagnostic::LimitExceeded;
use qsl_foundation::{Setting, SettingLimits};

/// The published default of `replay.input_bytes`: 1 MiB, well above any
/// legitimate proof result, witness, replay request or replay result, and
/// small enough that an adversarial or malformed oversized input is cheap to
/// reject before any further parsing.
pub const DEFAULT_REPLAY_INPUT_BYTES: u64 = 1 << 20;

/// The default bound as a byte count, for tests that build an encoding one
/// byte over it.
#[cfg(test)]
pub(crate) const DEFAULT_INPUT_BYTES: usize = DEFAULT_REPLAY_INPUT_BYTES as usize;

/// The replay readers' limits: `replay.input_bytes`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReplayLimits {
    /// The most encoded bytes any replay request or envelope may hold.
    pub input_bytes: u64,
}

impl Default for ReplayLimits {
    fn default() -> Self {
        Self {
            input_bytes: DEFAULT_REPLAY_INPUT_BYTES,
        }
    }
}

impl ReplayLimits {
    /// Sets [`Self::input_bytes`] and nothing else.
    #[must_use]
    pub fn with_input_bytes(mut self, input_bytes: u64) -> Self {
        self.input_bytes = input_bytes;
        self
    }
}

impl SettingLimits for ReplayLimits {
    fn bounds(&self) -> Vec<(Setting, u64)> {
        vec![(Setting::ReplayInputBytes, self.input_bytes)]
    }

    fn set_bound(&mut self, setting: Setting, bound: u64) -> bool {
        match setting {
            Setting::ReplayInputBytes => self.input_bytes = bound,
            _ => return false,
        }
        true
    }
}

/// A #231 reader refused an encoding exceeding its [`ReplayLimits`]:
/// `stage_limit_exceeded/input-bytes-exceeded` naming `replay.input_bytes`, through
/// [`qsl_foundation::diagnostic::CatalogCoded`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error(
    "stage_limit_exceeded/input-bytes-exceeded: encoded size {actual} exceeds the configured reader bound of {bound} bytes; raise `replay.input_bytes`"
)]
pub struct BoundExceeded {
    /// The configured `replay.input_bytes` bound.
    pub bound: u64,
    /// The encoding's actual size in bytes.
    pub actual: u128,
}

impl BoundExceeded {
    /// The setting that raises this bound.
    pub const SETTING: Setting = Setting::ReplayInputBytes;

    /// Check `encoded_bytes` (an already-measured encoded length; #231
    /// builds no byte-level wire serializer of its own, so every caller
    /// tracks this length structurally rather than producing real encoded
    /// bytes) against `limits`, returning [`BoundExceeded`] if it is over
    /// the bound.
    pub fn check(encoded_bytes: usize, limits: ReplayLimits) -> Result<(), Self> {
        let actual = u128::try_from(encoded_bytes).unwrap_or(u128::MAX);
        if actual > u128::from(limits.input_bytes) {
            Err(Self {
                bound: limits.input_bytes,
                actual,
            })
        } else {
            Ok(())
        }
    }

    /// This refusal as the stage limit outcome FR-255 defines.
    pub const fn limit(&self) -> LimitExceeded {
        LimitExceeded::new(Self::SETTING, self.bound, self.actual)
    }
}

impl qsl_foundation::diagnostic::CatalogCoded for BoundExceeded {
    fn catalog_code(&self) -> qsl_foundation::diagnostic::CatalogCode {
        self.limit().catalog_code()
    }

    /// The `stage_limit_exceeded` row's payload (FR-096, FR-255): the kind,
    /// the reader's bound, the encoding's actual size and the setting.
    fn catalog_fields(&self) -> Option<std::collections::BTreeMap<&'static str, String>> {
        self.limit().catalog_fields()
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    const DEFAULT: usize = DEFAULT_REPLAY_INPUT_BYTES as usize;

    #[test]
    fn admits_a_length_at_or_under_the_bound() {
        assert!(BoundExceeded::check(DEFAULT, ReplayLimits::default()).is_ok());
    }

    #[test]
    fn refuses_a_length_over_the_bound() {
        let err = BoundExceeded::check(DEFAULT + 1, ReplayLimits::default()).unwrap_err();
        assert_eq!(err.actual, DEFAULT as u128 + 1);
    }

    /// `BoundExceeded` reports `stage_limit_exceeded/input-bytes-exceeded`,
    /// carrying the caller's bound, the actual encoded size and the setting
    /// `replay.input_bytes`.
    #[trace("TC-428", "FR-096-AC-7")]
    #[trace("TC-737", "FR-263-AC-3")]
    #[test]
    fn reports_stage_limit_exceeded_input_bytes_naming_the_setting() {
        use qsl_foundation::diagnostic::{CatalogCode, CatalogCoded};

        let err =
            BoundExceeded::check(1001, ReplayLimits::default().with_input_bytes(1000)).unwrap_err();
        assert_eq!(
            err.catalog_code(),
            CatalogCode::new("stage_limit_exceeded", "input-bytes-exceeded")
        );
        assert_eq!((err.bound, err.actual), (1000, 1001));
        let fields = err.catalog_fields().expect("a key-table row");
        assert_eq!(
            fields.into_iter().collect::<Vec<_>>(),
            [
                ("actual", "1001".to_owned()),
                ("bound", "1000".to_owned()),
                ("kind", "input-bytes-exceeded".to_owned()),
                ("setting", "replay.input_bytes".to_owned()),
            ]
        );
        assert_eq!(err.limit().setting(), Setting::ReplayInputBytes);
    }

    /// The caller's limit decides: an encoding one byte over a raised bound
    /// refuses, and it is admitted once the bound is raised by one.
    #[trace("TC-737", "FR-263-AC-3")]
    #[test]
    fn a_caller_raised_bound_admits_what_the_default_refuses() {
        let over = DEFAULT + 1;
        assert!(BoundExceeded::check(over, ReplayLimits::default()).is_err());
        let raised = ReplayLimits::default().with_input_bytes(over as u64);
        assert!(BoundExceeded::check(over, raised).is_ok());
        assert!(BoundExceeded::check(over + 1, raised).is_err());
    }
}

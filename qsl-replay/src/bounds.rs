// SPDX-License-Identifier: AGPL-3.0-or-later
//! The configured reader bound every #231 envelope's decoder enforces
//! before admitting an oversized encoding (FR-069-AC-4, FR-070-AC-7,
//! FR-071-AC-7, FR-072-AC-5).
//!
//! Every reader in this module checks its input's encoded byte length
//! against [`MAX_ENCODED_BYTES`] before parsing any other member, and
//! refuses with [`BoundExceeded`] rather than returning a truncated or
//! partially-populated value. This is deliberately one shared constant, not
//! one per envelope: the four envelopes share one "configured reader
//! bound" per their acceptance criteria, and a single constant is the only
//! way a future change to it cannot silently diverge between them.

/// The maximum encoded size, in bytes, a #231 reader admits. 1 MiB: well
/// above any legitimate proof result, witness, replay request or replay
/// result, and small enough that an adversarial or malformed oversized
/// input is cheap to reject before any further parsing.
pub const MAX_ENCODED_BYTES: usize = 1 << 20;

/// A #231 reader refused an encoding exceeding [`MAX_ENCODED_BYTES`]:
/// `stage_limit_exceeded/input-bytes-exceeded` (QSL-236, catalog revision
/// `1-draft.7`), through [`qsl_foundation::diagnostic::CatalogCoded`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error(
    "stage_limit_exceeded/input-bytes-exceeded: encoded size {actual} exceeds the configured reader bound of {} bytes",
    MAX_ENCODED_BYTES
)]
pub struct BoundExceeded {
    /// The encoding's actual size in bytes.
    pub actual: usize,
}

impl qsl_foundation::diagnostic::CatalogCoded for BoundExceeded {
    fn catalog_code(&self) -> qsl_foundation::diagnostic::CatalogCode {
        qsl_foundation::diagnostic::CatalogCode::new("stage_limit_exceeded", "input-bytes-exceeded")
    }

    /// The `stage_limit_exceeded` row's payload (FR-096): the kind, the
    /// reader's bound and the encoding's actual size.
    fn catalog_fields(&self) -> Option<std::collections::BTreeMap<&'static str, String>> {
        Some(std::collections::BTreeMap::from([
            ("kind", "input-bytes-exceeded".to_owned()),
            ("bound", MAX_ENCODED_BYTES.to_string()),
            ("actual", self.actual.to_string()),
        ]))
    }
}

impl BoundExceeded {
    /// Check `encoded_bytes` (an already-measured encoded length; #231
    /// builds no byte-level wire serializer of its own, so every caller
    /// tracks this length structurally rather than producing real encoded
    /// bytes) against [`MAX_ENCODED_BYTES`], returning [`BoundExceeded`] if
    /// it is over the bound.
    pub fn check(encoded_bytes: usize) -> Result<(), Self> {
        if encoded_bytes > MAX_ENCODED_BYTES {
            Err(Self {
                actual: encoded_bytes,
            })
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admits_a_length_at_or_under_the_bound() {
        assert!(BoundExceeded::check(MAX_ENCODED_BYTES).is_ok());
    }

    #[test]
    fn refuses_a_length_over_the_bound() {
        let err = BoundExceeded::check(MAX_ENCODED_BYTES + 1).unwrap_err();
        assert_eq!(err.actual, MAX_ENCODED_BYTES + 1);
    }

    /// QSL-236: `BoundExceeded` reports `stage_limit_exceeded/
    /// input-bytes-exceeded`, carrying the bound (`MAX_ENCODED_BYTES`) and
    /// the actual encoded size.
    #[test]
    fn reports_stage_limit_exceeded_input_bytes() {
        use qsl_foundation::diagnostic::{CatalogCode, CatalogCoded};

        let err = BoundExceeded::check(MAX_ENCODED_BYTES + 5).unwrap_err();
        assert_eq!(
            err.catalog_code(),
            CatalogCode::new("stage_limit_exceeded", "input-bytes-exceeded")
        );
        assert_eq!(err.actual, MAX_ENCODED_BYTES + 5);
        let fields = err.catalog_fields().expect("a key-table row");
        assert_eq!(
            fields.into_iter().collect::<Vec<_>>(),
            [
                ("actual", (MAX_ENCODED_BYTES + 5).to_string()),
                ("bound", MAX_ENCODED_BYTES.to_string()),
                ("kind", "input-bytes-exceeded".to_owned()),
            ]
        );
        assert_eq!(
            err.to_string(),
            format!(
                "stage_limit_exceeded/input-bytes-exceeded: encoded size {} exceeds the configured reader bound of {MAX_ENCODED_BYTES} bytes",
                MAX_ENCODED_BYTES + 5
            )
        );
    }
}

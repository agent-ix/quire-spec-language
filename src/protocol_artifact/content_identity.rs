// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-051/FR-052 document content identities (QSL-220).
//!
//! A checked-handoff or native-temporal document's content identity is the
//! lowercase hex SHA-256 that `quire-canonical` computes over the RFC 8785
//! encoding of the document's identity preimage, under the document's
//! contract label as digest domain: the label's byte length as a big-endian
//! `u64`, the label, then the canonical text. ADR-013 §2 (ADR-013:113) names
//! `quire-canonical` the one RFC 8785 implementation, so the identity does
//! not depend on the order a Rust struct declares its fields in.

use serde::Serialize;

/// Why a preimage has no content identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Refusal {
    /// The canonical text would exceed the caller's output-byte limit.
    OutputBytes,
    /// The preimage nests deeper than the caller's JSON depth limit.
    JsonDepth,
    /// The preimage has no RFC 8785 encoding, e.g. an integer with no exact
    /// IEEE 754 double.
    NotEncodable,
}

/// The content identity of `preimage` under the `contract` digest domain,
/// encoded within `max_bytes` canonical bytes and `max_depth` nesting levels.
pub(crate) fn of(
    contract: &str,
    preimage: &impl Serialize,
    max_bytes: usize,
    max_depth: usize,
) -> Result<String, Refusal> {
    let max_bytes = u64::try_from(max_bytes).unwrap_or(u64::MAX);
    let max_depth = u32::try_from(max_depth)
        .unwrap_or(u32::MAX)
        .min(quire_canonical::Limits::MAX_DEPTH);
    let limits =
        quire_canonical::Limits::new(max_bytes, max_depth).map_err(|_| Refusal::JsonDepth)?;
    quire_canonical::sha256_with_domain(contract.as_bytes(), preimage, limits)
        .map(|digest| digest.to_string())
        .map_err(|error| match error {
            quire_canonical::Error::Limit(limit) => match limit.kind {
                quire_canonical::LimitKind::NestingDepth => Refusal::JsonDepth,
                quire_canonical::LimitKind::CanonicalBytes
                | quire_canonical::LimitKind::ObjectBytes => Refusal::OutputBytes,
            },
            _ => Refusal::NotEncodable,
        })
}

#[cfg(test)]
mod tests {
    use super::{of, Refusal};
    use serde::Serialize;

    const CONTRACT: &str = "quire.checked-predicate/v1";

    /// Hand-written RFC 8785 text of both preimages below: members sorted by
    /// name, no insignificant whitespace.
    const CANONICAL: &str = r#"{"alpha":1,"nested":{"x":true,"y":"é"},"zulu":"z"}"#;

    /// SHA-256 over `u64be(26) || "quire.checked-predicate/v1" || CANONICAL`,
    /// computed outside this crate (`printf ... | sha256sum`).
    const GOLDEN: &str = "78b0330fa4553eec14d02a4b5c3e6d1ac2681969b4310cb7031a965f0a3955b3";

    #[derive(Serialize)]
    struct Nested {
        y: &'static str,
        x: bool,
    }

    #[derive(Serialize)]
    struct Declared {
        zulu: &'static str,
        nested: Nested,
        alpha: u8,
    }

    #[derive(Serialize)]
    struct NestedSorted {
        x: bool,
        y: &'static str,
    }

    #[derive(Serialize)]
    struct Sorted {
        alpha: u8,
        nested: NestedSorted,
        zulu: &'static str,
    }

    fn declared() -> Declared {
        Declared {
            zulu: "z",
            nested: Nested { y: "é", x: true },
            alpha: 1,
        }
    }

    #[test]
    fn identity_matches_the_golden_vector() {
        let canonical =
            quire_canonical::to_vec(&declared(), quire_canonical::Limits::new(1024, 8).unwrap())
                .unwrap();
        assert_eq!(std::str::from_utf8(&canonical).unwrap(), CANONICAL);
        assert_eq!(of(CONTRACT, &declared(), 1024, 8).unwrap(), GOLDEN);
    }

    #[test]
    fn identity_is_independent_of_struct_field_order() {
        let sorted = Sorted {
            alpha: 1,
            nested: NestedSorted { x: true, y: "é" },
            zulu: "z",
        };
        assert_eq!(
            of(CONTRACT, &declared(), 1024, 8).unwrap(),
            of(CONTRACT, &sorted, 1024, 8).unwrap()
        );
    }

    #[test]
    fn identity_is_separated_by_contract_domain() {
        assert_ne!(
            of(CONTRACT, &declared(), 1024, 8).unwrap(),
            of("quire.checked-temporal-subject/v1", &declared(), 1024, 8).unwrap()
        );
    }

    #[test]
    fn identity_refuses_past_its_limits_and_inexact_integers() {
        assert_eq!(
            of(CONTRACT, &declared(), CANONICAL.len() - 1, 8),
            Err(Refusal::OutputBytes)
        );
        assert!(of(CONTRACT, &declared(), CANONICAL.len(), 8).is_ok());
        assert_eq!(of(CONTRACT, &declared(), 1024, 1), Err(Refusal::JsonDepth));
        assert_eq!(
            of(CONTRACT, &((1_u64 << 53) + 1), 1024, 8),
            Err(Refusal::NotEncodable)
        );
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-049 StaticAuthority digests shared by state evaluation fixtures.
//!
//! Each digest is computed, never stored: the member's FR-049 source text is
//! parsed, encoded with the independent RFC 8785 canonicalizer and hashed
//! with SHA-256. `static_authority_jcs_vectors_and_refusal_order` compares
//! them with the table FR-049 publishes, read from the spec file at run time.

use quire_spec_language::state::CanonicalDigest;
use sha2::{Digest, Sha256};

pub fn canonical(value: char) -> CanonicalDigest {
    let source = match value {
        '1' => r#"{ "version": 1, "name": "orders" }"#,
        '2' => r#"{"objects": ["Order", "Line"], "id": "orders-model"}"#,
        '3' => r#"{"limits": {"depth": 8}, "profile": "strict"}"#,
        '4' => r#"{"enabled": true, "mode": "batch"}"#,
        _ => panic!("unknown StaticAuthority vector"),
    };
    let parsed: serde_json::Value = serde_json::from_str(source).expect("source parses");
    let preimage = serde_json_canonicalizer::to_vec(&parsed).expect("independent JCS encoding");
    let hex: String = Sha256::digest(&preimage)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    CanonicalDigest(format!("sha256-jcs:{hex}"))
}

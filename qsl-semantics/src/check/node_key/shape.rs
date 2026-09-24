// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092 "The group order", steps 1-2: a recursion-group member's full and
//! anonymous shapes and its targets.
//!
//! The one place node keying rewrites a preimage rather than builds it: the
//! member's RFC 8785 bytes are read back into a `serde_json::Value`, each
//! `group_reference` placeholder's ordinal is removed (in the order the
//! bytes carry them), and for the anonymous shape `owner` is removed and
//! `declaration` cleared. Each shape is encoded again by `quire-canonical`
//! (ADR-013 §2, ADR-013:113). This module hashes nothing: the caller hashes
//! the shape text, so no file both reads JSON and hashes (arch-lint
//! `canonical-encoder`).

use serde::Serialize;
use serde_json::Value;

use super::{canonical_bytes, NodeKeyRefusal};

/// One member's shapes, as RFC 8785 text, and its targets.
pub(super) struct MemberShapes {
    /// The full shape.
    pub(super) full: String,
    /// The anonymous shape: `declaration` `null`, no `owner`.
    pub(super) anonymous: String,
    /// Each placeholder's target position, in RFC 8785 order.
    pub(super) targets: Vec<usize>,
}

/// The shapes of the member whose preimage's RFC 8785 bytes are
/// `canonical`, its placeholders' ordinals naming positions below `count`.
pub(super) fn member_shapes(
    canonical: &[u8],
    count: usize,
) -> Result<MemberShapes, NodeKeyRefusal> {
    let mut shape = read_back(canonical)?;
    let mut targets = Vec::new();
    shape_targets(&mut shape, &mut targets, count)?;
    let full = canonical_text(&shape)?;
    if let Value::Object(map) = &mut shape {
        map.insert("declaration".to_owned(), Value::Null);
        map.remove("owner");
    }
    let anonymous = canonical_text(&shape)?;
    Ok(MemberShapes {
        full,
        anonymous,
        targets,
    })
}

/// Remove each placeholder's ordinal from `value`, in RFC 8785 order (a
/// `serde_json` map iterates its keys sorted, which is RFC 8785's order for
/// these fixed ASCII names), appending it to `targets`. Every ordinal is a
/// member's position, below `count`.
fn shape_targets(
    value: &mut Value,
    targets: &mut Vec<usize>,
    count: usize,
) -> Result<(), NodeKeyRefusal> {
    match value {
        Value::Object(map) => {
            if map.get("term").and_then(Value::as_str) == Some("group_reference") {
                let target = map
                    .remove("ordinal")
                    .as_ref()
                    .and_then(Value::as_u64)
                    .and_then(|ordinal| usize::try_from(ordinal).ok())
                    .filter(|target| *target < count)
                    .ok_or(NodeKeyRefusal::InvalidGroup)?;
                targets.push(target);
                return Ok(());
            }
            for member in map.values_mut() {
                shape_targets(member, targets, count)?;
            }
        }
        Value::Array(items) => {
            for item in items {
                shape_targets(item, targets, count)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
    Ok(())
}

/// `canonical` read back into a `serde_json::Value`. `serde_json`'s default
/// 128-level parse limit is lifted: a body at `MAX_CHECKING_DEPTH` terms
/// nests about twice as many JSON levels. The depth is still bounded, by
/// `IDENTITY_LIMITS`' depth, which the encoder enforced when it wrote these bytes.
fn read_back(canonical: &[u8]) -> Result<Value, NodeKeyRefusal> {
    let mut reader = serde_json::Deserializer::from_slice(canonical);
    reader.disable_recursion_limit();
    serde::Deserialize::deserialize(&mut reader).map_err(|error: serde_json::Error| {
        NodeKeyRefusal::Encode {
            reason: error.to_string(),
        }
    })
}

/// `value`'s RFC 8785 text: [`canonical_bytes`], which are UTF-8.
fn canonical_text(value: &impl Serialize) -> Result<String, NodeKeyRefusal> {
    String::from_utf8(canonical_bytes(value)?).map_err(|error| NodeKeyRefusal::Encode {
        reason: error.to_string(),
    })
}

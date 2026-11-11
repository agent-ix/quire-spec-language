// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded FR-196 unit descriptors, with no recursive serde JSON tree.

use super::*;
use quire_canonical::{Node, NodeRef};
use quire_exact::{NodeKey, UnitId};
use std::cell::RefCell;

/// Decode the unit descriptor's tagged term and integer-text forms.
#[ix_trace_rs::string_edge]
pub(super) fn read(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Option<UnitId>, ProbabilityDocumentError> {
    poll(cancel)?;
    let admission = RefCell::new(writer::Admission::new(limits.storage_bytes, cancel));
    let result = quire_canonical::read_with_reservation(
        bytes,
        limits.bytes,
        &mut writer::Reserve(&admission),
    );
    if let Some(error) = admission.borrow_mut().take_stop() {
        return Err(error);
    }
    let parsed = result.map_err(|error| match error {
        quire_canonical::ReadError::Limit(limit) => {
            exceeded(ProbabilityDocumentLimit::Bytes, limit.bound, limit.required)
        }
        _ => ProbabilityDocumentError::Malformed,
    })?;
    let root = parsed.root();
    if matches!(root.node(), Node::Null) {
        return Ok(None);
    }
    let Node::Array(mut parts) = root.node() else {
        return Err(ProbabilityDocumentError::Malformed);
    };
    if parts.len() != 2 {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let tag = parts.next().ok_or(ProbabilityDocumentError::Malformed)?;
    let form = parts.next().ok_or(ProbabilityDocumentError::Malformed)?;
    match tag.node() {
        Node::String("declared") => Ok(Some(UnitId::declared(node(form, limits, &admission)?))),
        Node::String("compound") => {
            let Node::Array(items) = form.node() else {
                return Err(ProbabilityDocumentError::Malformed);
            };
            let mut terms = Vec::new();
            for term in items {
                poll(cancel)?;
                let Node::Object(members) = term.node() else {
                    return Err(ProbabilityDocumentError::Malformed);
                };
                if members.len() != 2 {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                let unit = node(
                    term.get("unit")
                        .ok_or(ProbabilityDocumentError::Malformed)?,
                    limits,
                    &admission,
                )?;
                let exponent = term
                    .get("exponent")
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                let Node::String(text) = exponent.node() else {
                    return Err(ProbabilityDocumentError::Malformed);
                };
                let digits = text.strip_prefix('-').unwrap_or(text);
                if digits.is_empty()
                    || !digits.bytes().all(|byte| byte.is_ascii_digit())
                    || digits.len() > 1 && digits.starts_with('0')
                    || text == "-0"
                {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                let minimum_bits = (digits.len() as u64)
                    .saturating_sub(1)
                    .saturating_mul(3)
                    .saturating_add(1);
                if minimum_bits > limits.scalar_bits && digits.len() > 1 {
                    return Err(ProbabilityDocumentError::IntegerText {
                        bit_bound: limits.scalar_bits,
                        decimal_digits: digits.len() as u64,
                    });
                }
                reserve(&admission, digits.len().saturating_mul(4).div_ceil(8))?;
                let exponent: Integer = text
                    .parse()
                    .map_err(|_| ProbabilityDocumentError::Malformed)?;
                if exponent.magnitude_bits() > limits.scalar_bits {
                    return Err(exceeded(
                        ProbabilityDocumentLimit::RationalBits,
                        limits.scalar_bits,
                        exponent.magnitude_bits(),
                    ));
                }
                if exponent == Integer::from(0_i64)
                    || terms.last().is_some_and(|(previous, _)| previous >= &unit)
                {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                reserve(&admission, std::mem::size_of::<(NodeKey, Integer)>())?;
                terms
                    .try_reserve_exact(1)
                    .map_err(|_| ProbabilityDocumentError::Malformed)?;
                terms.push((unit, exponent));
            }
            Ok(Some(UnitId::compound(terms)))
        }
        _ => Err(ProbabilityDocumentError::Malformed),
    }
}

fn reserve(
    admission: &RefCell<writer::Admission<'_>>,
    bytes: usize,
) -> Result<(), ProbabilityDocumentError> {
    let result = admission.borrow_mut().reserve(bytes);
    if result.is_err() {
        return Err(admission
            .borrow_mut()
            .take_stop()
            .unwrap_or(ProbabilityDocumentError::Malformed));
    }
    Ok(())
}

fn node(
    value: NodeRef<'_>,
    limits: ProbabilityDocumentLimits,
    admission: &RefCell<writer::Admission<'_>>,
) -> Result<NodeKey, ProbabilityDocumentError> {
    let bytes = writer::encode_reserved(&value, limits.bytes, admission)?;
    reserve(admission, bytes.len())?;
    let text = std::str::from_utf8(&bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    qsl_foundation::node_wire::node_key_from_text(text)
        .map_err(|_| ProbabilityDocumentError::Malformed)
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-357, FR-358: the obligation identity of a parity claim, recomputed
//! from ADR-013 O-09's one parity preimage through `quire-canonical`, the one
//! RFC 8785 encoder (ADR-013 section 2). The preimage is a typed view, so this
//! module names no JSON crate.
//!
//! An operator-parity claim and a composite equality claim share the
//! preimage: the application node and its occurrence key, the obligation kind
//! and one argument per operand position, each an operand identity and a
//! domain (a range for a scalar, harness bounds for a composite).

use qsl_foundation::bound::{DomainKey, FiniteBound, ProofBound};
use qsl_foundation::digest::WireNodeId;
use quire_canonical::{Encode, Error, Limits, Sink, Writer};
use quire_exact::{IntegerInterval, Origin};

use crate::identity::ObligationIdentity;
use crate::scalar::OperandIdentity;

/// Why a preimage could not be encoded. The claim refuses; no identity is
/// ever produced from an error.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityEncodeError {
    /// A JSON integer beyond the encoder's 2^53 limit (an occurrence
    /// ordinal, say). An exact integer past it travels as a decimal string,
    /// and the claim's integers that can exceed it do.
    #[error("the integer {magnitude} is beyond the encoder's 2^53 limit")]
    IntegerBeyondEncoder {
        /// The refused integer.
        magnitude: i128,
    },
    /// Two harness bounds of one operand name the same domain key.
    #[error("two harness bounds name the domain key {key}")]
    DuplicateKey {
        /// The repeated key, in its display form.
        key: String,
    },
    /// A value-parity argument's `position` is not its index in the declared
    /// parameter list (FR-484): the encoder never sorts the arguments, so the
    /// list and the positions must already agree.
    #[error("the value-parity argument at index {index} claims position {position}")]
    PositionOrder {
        /// The argument's index in the list.
        index: usize,
        /// The position the argument claims.
        position: usize,
    },
    /// The encoder refused for another reason (its error type is
    /// non-exhaustive); the reason is its message.
    #[error("the encoder refused the preimage: {0}")]
    Refused(String),
}

impl From<Error> for IdentityEncodeError {
    fn from(error: Error) -> Self {
        match error {
            Error::IntegerMagnitudeAboveMaximum(magnitude) => {
                Self::IntegerBeyondEncoder { magnitude }
            }
            // `quire_canonical::Error` is `non_exhaustive`.
            other => Self::Refused(other.to_string()),
        }
    }
}

/// A composite operand's harness bounds: one entry per domain key, ascending
/// by the bytes of the key's RFC 8785 encoding (ADR-013 O-09), which is not
/// `DomainKey`'s derived order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundEntries(Vec<ProofBound>);

impl BoundEntries {
    /// The bounds in the preimage's order, refusing a repeated key.
    pub fn new(bounds: Vec<ProofBound>) -> Result<Self, IdentityEncodeError> {
        let mut keyed = bounds
            .into_iter()
            .map(|bound| Ok((encoded_key(bound.domain())?, bound)))
            .collect::<Result<Vec<_>, IdentityEncodeError>>()?;
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        if let Some(pair) = keyed.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            return Err(IdentityEncodeError::DuplicateKey {
                key: pair[0].1.domain().to_string(),
            });
        }
        Ok(Self(keyed.into_iter().map(|(_, bound)| bound).collect()))
    }

    /// The bounds, in the preimage's order.
    pub fn entries(&self) -> &[ProofBound] {
        &self.0
    }
}

/// What an operand ranges over (ADR-013 O-09's `domain`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Domain {
    /// A scalar operand's inclusive integer range; an inline literal's is
    /// the singleton `(value, value)`.
    Range(IntegerInterval),
    /// A composite operand's harness bounds.
    Bounds(BoundEntries),
}

/// One argument of the preimage: an operand's identity and domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParityArgument {
    /// How the operand is named.
    pub identity: OperandIdentity,
    /// What the operand ranges over.
    pub domain: Domain,
}

/// ADR-013 O-09's parity preimage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParityPreimage {
    /// The application node.
    pub node: WireNodeId,
    /// The application node's occurrence key.
    pub occurrence: Origin,
    /// The CG `ObligationKind` wire string.
    pub obligation_kind: String,
    /// One argument per operand position, in operand order.
    pub arguments: Vec<ParityArgument>,
}

/// The identity ADR-013 O-09 gives a parity claim: the digest of the RFC
/// 8785 encoding of `preimage`. An encoder refusal is an error, never an
/// identity.
pub fn parity_obligation(
    preimage: &ParityPreimage,
) -> Result<ObligationIdentity, IdentityEncodeError> {
    let digest = quire_canonical::sha256(preimage, Limits::new(u64::MAX))?;
    Ok(ObligationIdentity::from_digest(*digest.as_bytes()))
}

/// FR-484: one declared parameter of a function-value parity claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValueParityArgument {
    /// The parameter's `value`/`parameter` node id, the id
    /// `FunctionSite.parameters` pairs with the parameter.
    pub parameter: WireNodeId,
    /// The parameter's zero-based position in the declared parameter list.
    pub position: usize,
    /// The node id of the parameter node's `semantic_type`, its declared
    /// type.
    pub semantic_type: WireNodeId,
}

/// FR-484's preimage of a function-value parity claim. The occurrence key
/// (the function's one `declaration` occurrence) and the `subject_kind` are
/// fixed by the form and are not fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueParityPreimage {
    /// The function's checked `function` node id.
    pub node: WireNodeId,
    /// One entry per declared parameter, in declared parameter order.
    pub arguments: Vec<ValueParityArgument>,
}

/// FR-484: the identity of a function-value parity claim, the SHA-256 of
/// `preimage`'s canonical bytes through the one encoder (ADR-013 section 2).
/// The arguments are not sorted; an argument whose `position` is not its
/// index refuses [`IdentityEncodeError::PositionOrder`].
pub fn value_parity_obligation(
    preimage: &ValueParityPreimage,
) -> Result<ObligationIdentity, IdentityEncodeError> {
    if let Some((index, argument)) = preimage
        .arguments
        .iter()
        .enumerate()
        .find(|(index, argument)| argument.position != *index)
    {
        return Err(IdentityEncodeError::PositionOrder {
            index,
            position: argument.position,
        });
    }
    let digest = quire_canonical::sha256(preimage, Limits::new(u64::MAX))?;
    Ok(ObligationIdentity::from_digest(*digest.as_bytes()))
}

impl Encode for ValueParityArgument {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        let position = i128::try_from(self.position).map_err(|_| Error::Internal {
            invariant: "a parameter position fits i128",
        })?;
        writer.begin_object()?;
        writer.name("parameter")?;
        writer.string(&self.parameter.to_string())?;
        writer.name("position")?;
        writer.integer(position)?;
        writer.name("type")?;
        writer.string(&self.semantic_type.to_string())?;
        writer.end_object()
    }
}

impl Encode for ValueParityPreimage {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("arguments")?;
        writer.begin_array()?;
        for argument in &self.arguments {
            argument.encode_into(writer)?;
        }
        writer.end_array()?;
        writer.name("node")?;
        writer.string(&self.node.to_string())?;
        writer.name("occurrence_key")?;
        writer.begin_object()?;
        writer.name("ordinal")?;
        writer.integer(0)?;
        writer.name("role")?;
        writer.string("declaration")?;
        writer.end_object()?;
        writer.name("subject_kind")?;
        writer.string("function-value-parity")?;
        writer.end_object()
    }
}

/// The digest of one argument entry's canonical text alone.
#[cfg(test)]
pub(crate) fn argument_digest(
    argument: &ParityArgument,
    node: WireNodeId,
    occurrence: &Origin,
    position: usize,
) -> Result<[u8; 32], IdentityEncodeError> {
    let digest = quire_canonical::sha256(
        &ArgumentEncode {
            argument,
            node,
            occurrence,
            position,
        },
        Limits::new(u64::MAX),
    )?;
    Ok(*digest.as_bytes())
}

fn int<S: Sink + ?Sized>(writer: &mut Writer<'_, S>, value: impl Into<i128>) -> Result<(), Error> {
    writer.integer(value.into())
}

fn occurrence_key<S: Sink + ?Sized>(writer: &mut Writer<'_, S>, key: &Origin) -> Result<(), Error> {
    writer.begin_object()?;
    writer.name("ordinal")?;
    int(writer, key.ordinal())?;
    writer.name("role")?;
    writer.string(key.role().as_str())?;
    writer.end_object()
}

fn interval<S: Sink + ?Sized>(
    writer: &mut Writer<'_, S>,
    tag: &str,
    range: &IntegerInterval,
) -> Result<(), Error> {
    writer.begin_object()?;
    writer.name("lower")?;
    writer.string(&range.lower().to_string())?;
    writer.name("tag")?;
    writer.string(tag)?;
    writer.name("upper")?;
    writer.string(&range.upper().to_string())?;
    writer.end_object()
}

struct KeyEncode<'a>(&'a DomainKey);

impl Encode for KeyEncode<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        match self.0 {
            DomainKey::Node { node, path } => {
                writer.name("node_id")?;
                writer.string(&node.to_string())?;
                writer.name("path")?;
                writer.begin_array()?;
                for index in path {
                    int(writer, *index)?;
                }
                writer.end_array()?;
                writer.name("tag")?;
                writer.string("node")?;
            }
            DomainKey::Population {
                member_type,
                ordinal,
            } => {
                writer.name("member_type")?;
                writer.string(&member_type.to_string())?;
                writer.name("ordinal")?;
                int(writer, *ordinal)?;
                writer.name("tag")?;
                writer.string("population")?;
            }
        }
        writer.end_object()
    }
}

fn encoded_key(key: &DomainKey) -> Result<Vec<u8>, IdentityEncodeError> {
    Ok(quire_canonical::to_vec(
        &KeyEncode(key),
        Limits::new(u64::MAX),
    )?)
}

fn bound<S: Sink + ?Sized>(writer: &mut Writer<'_, S>, bound: &FiniteBound) -> Result<(), Error> {
    match bound {
        FiniteBound::Cardinality { maximum } => {
            writer.begin_object()?;
            writer.name("maximum")?;
            writer.string(&maximum.to_string())?;
            writer.name("tag")?;
            writer.string("cardinality")?;
            writer.end_object()
        }
        FiniteBound::IntegerRange(range) => interval(writer, "integer_range", range),
        FiniteBound::Depth { maximum } => {
            writer.begin_object()?;
            writer.name("maximum")?;
            writer.string(&maximum.to_string())?;
            writer.name("tag")?;
            writer.string("depth")?;
            writer.end_object()
        }
        FiniteBound::Variants { members } => {
            writer.begin_object()?;
            writer.name("tag")?;
            writer.string("variants")?;
            writer.name("variants")?;
            writer.begin_array()?;
            for member in members {
                writer.string(member)?;
            }
            writer.end_array()?;
            writer.end_object()
        }
    }
}

fn domain<S: Sink + ?Sized>(writer: &mut Writer<'_, S>, domain: &Domain) -> Result<(), Error> {
    match domain {
        Domain::Range(range) => interval(writer, "range", range),
        Domain::Bounds(bounds) => {
            writer.begin_object()?;
            writer.name("entries")?;
            writer.begin_array()?;
            for entry in bounds.entries() {
                writer.begin_object()?;
                writer.name("bound")?;
                bound(writer, entry.bound())?;
                writer.name("key")?;
                KeyEncode(entry.domain()).encode_into(writer)?;
                writer.end_object()?;
            }
            writer.end_array()?;
            writer.name("tag")?;
            writer.string("bounds")?;
            writer.end_object()
        }
    }
}

struct ArgumentEncode<'a> {
    argument: &'a ParityArgument,
    node: WireNodeId,
    occurrence: &'a Origin,
    position: usize,
}

impl Encode for ArgumentEncode<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        let position = i128::try_from(self.position).map_err(|_| Error::Internal {
            invariant: "an operand position fits i128",
        })?;
        writer.begin_object()?;
        writer.name("domain")?;
        domain(writer, &self.argument.domain)?;
        writer.name("operand")?;
        writer.begin_object()?;
        match self.argument.identity {
            OperandIdentity::GraphChild(node) => {
                writer.name("node_id")?;
                writer.string(&node.to_string())?;
                writer.name("tag")?;
                writer.string("graph_child")?;
            }
            OperandIdentity::InlineLiteral => {
                writer.name("node_id")?;
                writer.string(&self.node.to_string())?;
                writer.name("occurrence_key")?;
                occurrence_key(writer, self.occurrence)?;
                writer.name("position")?;
                writer.integer(position)?;
                writer.name("tag")?;
                writer.string("inline_literal")?;
            }
        }
        writer.end_object()?;
        writer.name("position")?;
        writer.integer(position)?;
        writer.end_object()
    }
}

impl Encode for ParityPreimage {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("arguments")?;
        writer.begin_array()?;
        for (position, argument) in self.arguments.iter().enumerate() {
            ArgumentEncode {
                argument,
                node: self.node,
                occurrence: &self.occurrence,
                position,
            }
            .encode_into(writer)?;
        }
        writer.end_array()?;
        writer.name("node")?;
        writer.string(&self.node.to_string())?;
        writer.name("obligation_kind")?;
        writer.string(&self.obligation_kind)?;
        writer.name("occurrence_key")?;
        occurrence_key(writer, &self.occurrence)?;
        writer.end_object()
    }
}

/// FR-290: the obligation identity of a process-provider item. A process
/// item has no CG obligation (ADR-029 PV-4 ruling 3), so QSL computes it:
/// the SHA-256 of the RFC 8785 encoding of the item's proof bounds in O-09's
/// parity `bounds` form, the occurrence's node and key, the obligation kind
/// `value-validity` and the subject kind `process-provider`.
pub(crate) fn process_provider_obligation(
    node: WireNodeId,
    occurrence: &Origin,
    bounds: &BoundEntries,
) -> Result<ObligationIdentity, IdentityEncodeError> {
    let digest = quire_canonical::sha256(
        &ProcessProviderPreimage {
            node,
            occurrence,
            bounds,
        },
        Limits::new(u64::MAX),
    )?;
    Ok(ObligationIdentity::from_digest(*digest.as_bytes()))
}

struct ProcessProviderPreimage<'a> {
    node: WireNodeId,
    occurrence: &'a Origin,
    bounds: &'a BoundEntries,
}

impl Encode for ProcessProviderPreimage<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("declared_domains")?;
        domain(writer, &Domain::Bounds(self.bounds.clone()))?;
        writer.name("node")?;
        writer.string(&self.node.to_string())?;
        writer.name("obligation_kind")?;
        writer.string("value-validity")?;
        writer.name("occurrence_key")?;
        occurrence_key(writer, self.occurrence)?;
        writer.name("subject_kind")?;
        writer.string("process-provider")?;
        writer.end_object()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_foundation::bound::DomainKind;

    fn node() -> WireNodeId {
        WireNodeId::from_digest([0x11; 32])
    }

    fn key(path: Vec<u32>) -> DomainKey {
        DomainKey::Node { node: node(), path }
    }

    /// FR-357-AC-19: harness bounds order by the encoded key's bytes, not by
    /// `DomainKey`'s derived order, and a repeated key refuses.
    #[trace("TC-904", "FR-357-AC-19")]
    #[test]
    fn bound_entries_order_by_the_encoded_key_and_refuse_a_repeat() {
        let bound = |path: Vec<u32>| {
            ProofBound::new(
                key(path),
                Some(DomainKind::Collection),
                FiniteBound::cardinality(1),
            )
            .unwrap()
        };
        let entries =
            BoundEntries::new(vec![bound(vec![2]), bound(vec![10]), bound(vec![0])]).unwrap();
        let order: Vec<_> = entries
            .entries()
            .iter()
            .map(|entry| {
                entry
                    .domain()
                    .to_string()
                    .rsplit('/')
                    .next()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(order, ["0", "10", "2"]);
        assert!(matches!(
            BoundEntries::new(vec![bound(vec![1]), bound(vec![1])]),
            Err(IdentityEncodeError::DuplicateKey { .. })
        ));
    }
}

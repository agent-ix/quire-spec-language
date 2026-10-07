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
            .map(|bound| Ok((encoded_key(&bound.domain)?, bound)))
            .collect::<Result<Vec<_>, IdentityEncodeError>>()?;
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        if let Some(pair) = keyed.windows(2).find(|pair| pair[0].0 == pair[1].0) {
            return Err(IdentityEncodeError::DuplicateKey {
                key: pair[0].1.domain.to_string(),
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
                bound(writer, &entry.bound)?;
                writer.name("key")?;
                KeyEncode(&entry.domain).encode_into(writer)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_foundation::bound::DomainKind;
    use qsl_foundation::ByteDigest;
    use quire_exact::{Integer, Role};

    fn node() -> WireNodeId {
        WireNodeId::from_digest([0x11; 32])
    }

    fn key(path: Vec<u32>) -> DomainKey {
        DomainKey::Node { node: node(), path }
    }

    fn sequence_argument() -> ParityArgument {
        ParityArgument {
            identity: OperandIdentity::GraphChild(node()),
            domain: Domain::Bounds(
                BoundEntries::new(vec![
                    ProofBound {
                        domain: key(vec![0]),
                        kind: Some(DomainKind::Collection),
                        bound: FiniteBound::cardinality(3),
                    },
                    ProofBound {
                        domain: key(vec![0, 0]),
                        kind: Some(DomainKind::Integer),
                        bound: FiniteBound::integer_range(
                            Integer::from(0_i64),
                            Integer::from(9_i64),
                        )
                        .unwrap(),
                    },
                ])
                .unwrap(),
            ),
        }
    }

    /// FR-357-AC-19: the worked example of ADR-013 O-09: a record parameter
    /// with one `Sequence` field, whose argument entry is the canonical text
    /// the ADR shows, digested independently of the encoder.
    #[trace("TC-904", "FR-357-AC-19")]
    #[test]
    fn the_adr_worked_example_has_the_documented_digest() {
        let id = "1".repeat(64);
        let text = format!(
            "{{\"domain\":{{\"entries\":[{{\"bound\":{{\"lower\":\"0\",\"tag\":\"integer_range\",\"upper\":\"9\"}},\"key\":{{\"node_id\":\"{id}\",\"path\":[0,0],\"tag\":\"node\"}}}},{{\"bound\":{{\"maximum\":\"3\",\"tag\":\"cardinality\"}},\"key\":{{\"node_id\":\"{id}\",\"path\":[0],\"tag\":\"node\"}}}}],\"tag\":\"bounds\"}},\"operand\":{{\"node_id\":\"{id}\",\"tag\":\"graph_child\"}},\"position\":0}}"
        );
        let digest = argument_digest(
            &sequence_argument(),
            node(),
            &Origin::new(Role::new("expression"), 0),
            0,
        )
        .unwrap();
        assert_eq!(digest, ByteDigest::of(text.as_bytes()).as_bytes());
        assert_eq!(
            ByteDigest::of(text.as_bytes())
                .as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            "4c36290f8830ff25cbb3915fa955b093f802fb801de2074415a6c4884fc9659f"
        );
    }

    /// FR-357-AC-19: the whole preimage of the worked example, through the
    /// one function both arms call, is the digest of its canonical text.
    #[trace("TC-904", "FR-357-AC-19")]
    #[test]
    fn the_worked_example_preimage_digests_through_the_one_function() {
        let id = "1".repeat(64);
        let entry = format!(
            "{{\"domain\":{{\"entries\":[{{\"bound\":{{\"lower\":\"0\",\"tag\":\"integer_range\",\"upper\":\"9\"}},\"key\":{{\"node_id\":\"{id}\",\"path\":[0,0],\"tag\":\"node\"}}}},{{\"bound\":{{\"maximum\":\"3\",\"tag\":\"cardinality\"}},\"key\":{{\"node_id\":\"{id}\",\"path\":[0],\"tag\":\"node\"}}}}],\"tag\":\"bounds\"}},\"operand\":{{\"node_id\":\"{id}\",\"tag\":\"graph_child\"}},\"position\":0}}"
        );
        let app = "aa".repeat(32);
        let text = format!(
            "{{\"arguments\":[{entry}],\"node\":\"{app}\",\"obligation_kind\":\"bounded_shadow\",\"occurrence_key\":{{\"ordinal\":0,\"role\":\"expression\"}}}}"
        );
        let preimage = ParityPreimage {
            node: WireNodeId::from_digest([0xaa; 32]),
            occurrence: Origin::new(Role::new("expression"), 0),
            obligation_kind: "bounded_shadow".to_owned(),
            arguments: vec![sequence_argument()],
        };
        assert_eq!(
            parity_obligation(&preimage).unwrap(),
            ObligationIdentity::from_digest(ByteDigest::of(text.as_bytes()).as_bytes())
        );
    }

    /// FR-357-AC-19 (IR-631): an operator-parity preimage over one graph-child
    /// operand with the range `[-2, 3]`. The canonical text is written by
    /// hand from ADR-013 O-09's rules (members sorted, bounds as decimal
    /// strings, node ids as 64 lowercase hex digits, counters as numbers); the
    /// encoder's bytes and the identity digest must equal it.
    #[trace("TC-904", "FR-357-AC-19")]
    #[test]
    fn an_operator_parity_range_preimage_has_its_hand_written_text_and_digest() {
        let text = concat!(
            r#"{"arguments":[{"domain":{"lower":"-2","tag":"range","upper":"3"},"#,
            r#""operand":{"node_id":"2222222222222222222222222222222222222222222222222222222222222222","tag":"graph_child"},"#,
            r#""position":0}],"#,
            r#""node":"1111111111111111111111111111111111111111111111111111111111111111","#,
            r#""obligation_kind":"operator_parity","#,
            r#""occurrence_key":{"ordinal":0,"role":"expression"}}"#
        );
        let preimage = ParityPreimage {
            node: node(),
            occurrence: Origin::new(Role::new("expression"), 0),
            obligation_kind: "operator_parity".to_owned(),
            arguments: vec![ParityArgument {
                identity: OperandIdentity::GraphChild(WireNodeId::from_digest([0x22; 32])),
                domain: Domain::Range(
                    IntegerInterval::new(Integer::from(-2_i64), Integer::from(3_i64)).unwrap(),
                ),
            }],
        };
        let bytes = quire_canonical::to_vec(&preimage, Limits::new(u64::MAX)).unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), text);
        let digest = ByteDigest::of(text.as_bytes());
        assert_eq!(
            digest
                .as_bytes()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            "ce7c6db3f790558d63e613427d14cfc5916802329b1e3ce032daa5878dc8ee2a"
        );
        assert_eq!(
            parity_obligation(&preimage).unwrap(),
            ObligationIdentity::from_digest(digest.as_bytes())
        );
    }

    /// FR-357-AC-19: harness bounds order by the encoded key's bytes, not by
    /// `DomainKey`'s derived order, and a repeated key refuses.
    #[trace("TC-904", "FR-357-AC-19")]
    #[test]
    fn bound_entries_order_by_the_encoded_key_and_refuse_a_repeat() {
        let bound = |path: Vec<u32>| ProofBound {
            domain: key(path),
            kind: Some(DomainKind::Collection),
            bound: FiniteBound::cardinality(1),
        };
        let entries =
            BoundEntries::new(vec![bound(vec![2]), bound(vec![10]), bound(vec![0])]).unwrap();
        let order: Vec<_> = entries
            .entries()
            .iter()
            .map(|entry| {
                entry
                    .domain
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

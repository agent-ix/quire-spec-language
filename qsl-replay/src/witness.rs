// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-070 (ADR-013 O-25): the typed counterexample/witness envelope.
//!
//! Two nested types, matching ADR-013 O-25's own split:
//!
//! - [`Witness`] is the backend witness: exactly one stored field, the
//!   admitted transcript, with every other witness fact derived from it on
//!   every call. [`Witness::decode`] reads its typed [`WitnessValue`]s
//!   against the caller's [`WitnessBinding`]s. CG's backend adapter parses
//!   a backend run into the transcript [`Witness::parse`] admits.
//! - [`WitnessEnvelope`] is the common ADR-013 O-25 carrier: every member
//!   FR-070's Behavior section lists, plus the [`FamilyPayload`] extension
//!   point a family (starting with #186's state `forall`) attaches its own
//!   typed payload through.

use std::fmt;

#[cfg(test)]
use quire_exact::Origin;
use quire_exact::ScalarLimits;

use crate::bounds::{BoundExceeded, ReplayLimits};
use crate::identity::{
    Backend, DeclaredDomain, ObligationIdentity, ProfileSelection, QualifiedName, RawSourceRef,
    SourceDigestWire, TracePosition,
};
use qsl_foundation::bound::{DomainKey, FiniteBound};
use qsl_foundation::digest::{
    ByteDigest, DigestDomain, DigestRecord, InvalidDigestRecord, WireNodeId,
};
use qsl_foundation::source::provenance::OccurrenceKey;

// ---------------------------------------------------------------------------
// `Witness`: the one-stored-field transcript carrier (ADR-013 O-25, QC-13).
// ---------------------------------------------------------------------------

/// [`Witness::parse`]'s admission failure.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum MalformedTranscript {
    /// The transcript names no assertion block at all.
    #[error("transcript names no assertion block")]
    NoAssertionBlock,
    /// The transcript names more than one assertion block.
    #[error("transcript names more than one assertion block")]
    MultipleAssertionBlocks,
    /// The transcript's one block is a cover-playback (or other non-assertion)
    /// block, never admitted.
    #[error("transcript names a cover-playback block, not an assertion block")]
    NotAnAssertionBlock,
    /// The transcript carries bytes outside its single assertion block.
    #[error("transcript carries bytes outside its single assertion block")]
    Untrimmed,
}

/// The admitted backend witness (ADR-013 O-25). Exactly one field: the
/// selected, trimmed assertion-block transcript. `harness_symbol()`,
/// `check()`, `check_text()`, `concrete_values()` and `decode` each
/// recompute their answer from `transcript` on every call; none is cached
/// alongside it, so no code path can leave a derived fact disagreeing with
/// the stored transcript (FR-070-AC-1).
///
/// quire:canonical
#[derive(Clone, Eq, PartialEq)]
pub struct Witness {
    transcript: String,
}

impl Witness {
    /// Admit `transcript`: it must contain exactly one assertion block, and
    /// the whole string must be exactly that block (no leading or trailing
    /// bytes). A cover-playback block, zero blocks, more than one block, or
    /// extra surrounding bytes each refuse; none produces a
    /// partially-built `Witness`.
    #[qsl_attrs::string_edge]
    pub fn parse(transcript: impl Into<String>) -> Result<Self, MalformedTranscript> {
        let transcript = transcript.into();
        let blocks = find_blocks(&transcript);
        let block = match blocks.as_slice() {
            [] => return Err(MalformedTranscript::NoAssertionBlock),
            [only] => *only,
            _ => return Err(MalformedTranscript::MultipleAssertionBlocks),
        };
        if block != transcript {
            return Err(MalformedTranscript::Untrimmed);
        }
        let fields = parse_block(block).ok_or(MalformedTranscript::NoAssertionBlock)?;
        if fields.kind != "assertion" {
            return Err(MalformedTranscript::NotAnAssertionBlock);
        }
        Ok(Self { transcript })
    }

    /// The full, unredacted transcript. A typed accessor (FR-073): always
    /// returns the complete content, unaffected by `Debug`/`Display`'s
    /// redaction.
    pub fn transcript(&self) -> &str {
        &self.transcript
    }

    fn fields(&self) -> BlockFields<'_> {
        parse_block(&self.transcript).expect("an admitted transcript always re-parses")
    }

    /// The harness symbol, recomputed from the stored transcript.
    pub fn harness_symbol(&self) -> &str {
        self.fields().harness
    }

    /// The check kind (always `"assertion"` for an admitted `Witness`),
    /// recomputed from the stored transcript.
    pub fn check(&self) -> &str {
        self.fields().kind
    }

    /// The check's text, recomputed from the stored transcript.
    pub fn check_text(&self) -> &str {
        self.fields().check_text
    }

    /// The transcript's raw `(name, value text)` entries, split but not yet
    /// parsed -- an entry whose text does not parse still appears here,
    /// unlike [`Self::concrete_values`], so [`Self::decode`] can tell "no
    /// entry names this parameter" apart from "an entry names it but its
    /// value is malformed". An entry that is not a `name=value` pair is its
    /// own `Err`, so decode can refuse it rather than drop it.
    fn raw_bindings(&self) -> Vec<Result<(&str, &str), &str>> {
        self.fields()
            .values
            .split(';')
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.split_once('=').ok_or(entry))
            .collect()
    }

    /// The transcript's concrete `(name, value)` bindings, recomputed from
    /// the stored transcript on every call, read without a schema: an entry
    /// that is not a `name=value` pair, or whose value does not parse as an
    /// integer, is skipped here, where
    /// [`Self::decode`] reads each entry as its binding's declared type and
    /// refuses a malformed one.
    pub fn concrete_values(&self) -> Vec<(String, i128)> {
        self.raw_bindings()
            .into_iter()
            .filter_map(|entry| {
                let (name, value) = entry.ok()?;
                Some((name.to_owned(), value.parse().ok()?))
            })
            .collect()
    }

    /// The transcript's values for `bindings`, one typed [`WitnessValue`]
    /// per binding, in `bindings` order (ADR-013 O-25: harness argument
    /// order equals `arguments` order). Each transcript entry is joined to
    /// the binding whose parameter node id it names, never by position,
    /// and its text is read as the binding's declared
    /// [`WitnessValueType`]. Refuses with [`DecodeRefusal::Missing`] when a
    /// binding's parameter has no entry, [`DecodeRefusal::Duplicate`] when
    /// it has more than one, [`DecodeRefusal::MalformedEntry`] when an entry
    /// is not a `name=value` pair, [`DecodeRefusal::Unbound`] when an entry
    /// names no binding's parameter, and [`DecodeRefusal::Malformed`] when an
    /// entry's text is not a value of the binding's type.
    pub fn decode(&self, bindings: &[WitnessBinding]) -> Result<Vec<WitnessValue>, DecodeRefusal> {
        let entries = self
            .raw_bindings()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|entry| DecodeRefusal::MalformedEntry(entry.to_owned()))?;
        let names: Vec<String> = bindings
            .iter()
            .map(|binding| binding.parameter.to_string())
            .collect();
        if let Some((name, _)) = entries
            .iter()
            .find(|(name, _)| !names.iter().any(|bound| bound == name))
        {
            return Err(DecodeRefusal::Unbound((*name).to_owned()));
        }
        bindings
            .iter()
            .zip(&names)
            .map(|(binding, name)| {
                let mut matching = entries.iter().filter(|(entry, _)| entry == name);
                let (_, text) = matching
                    .next()
                    .ok_or(DecodeRefusal::Missing(binding.parameter))?;
                if matching.next().is_some() {
                    return Err(DecodeRefusal::Duplicate(binding.parameter));
                }
                binding
                    .value_type
                    .read(text)
                    .ok_or_else(|| DecodeRefusal::Malformed {
                        parameter: binding.parameter,
                        value_type: binding.value_type,
                        text: (*text).to_owned(),
                    })
            })
            .collect()
    }
}

/// The declared primitive type of one harness argument (ADR-013 O-25).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WitnessValueType {
    /// A Boolean, written `0` (`false`) or `1` (`true`) in a transcript, as
    /// a backend's one-byte encoding carries it.
    Boolean,
    /// A signed 128-bit integer (FR-091's i128 ceiling), written in decimal
    /// in a transcript.
    I128,
}

impl WitnessValueType {
    /// `text` as a value of this type, or `None` when it is not one.
    #[qsl_attrs::string_edge]
    fn read(self, text: &str) -> Option<WitnessValue> {
        match self {
            Self::Boolean => match text {
                "0" => Some(WitnessValue::Boolean(false)),
                "1" => Some(WitnessValue::Boolean(true)),
                _ => None,
            },
            Self::I128 => text.parse().ok().map(WitnessValue::Integer),
        }
    }
}

/// One declared harness argument [`Witness::decode`] reads: the parameter
/// node id the transcript entry names, and the argument's declared type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WitnessBinding {
    /// The parameter this argument binds.
    pub parameter: WireNodeId,
    /// The argument's declared primitive type.
    pub value_type: WitnessValueType,
}

/// One concrete argument value, typed by the binding or assignment that
/// carries it. An integer widens into the kernel's unbounded integer
/// without loss (ADR-013 C-11).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WitnessValue {
    /// A Boolean value.
    Boolean(bool),
    /// A signed 128-bit integer value.
    Integer(i128),
}

/// [`Witness::decode`]'s structured refusal. Each join failure is its own
/// variant, so a malformed value is never reported as an absent one.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DecodeRefusal {
    /// No transcript entry names this parameter.
    #[error("no witness entry for parameter {0}")]
    Missing(WireNodeId),
    /// More than one transcript entry names this parameter.
    #[error("more than one witness entry for parameter {0}")]
    Duplicate(WireNodeId),
    /// A transcript entry names no binding's parameter.
    #[error("witness entry {0:?} names no bound parameter")]
    Unbound(String),
    /// A transcript entry is not a `name=value` pair.
    #[error("witness entry {0:?} is not a name=value pair")]
    MalformedEntry(String),
    /// A transcript entry's text is not a value of its binding's type.
    #[error("witness entry for parameter {parameter} is not a {value_type:?} value: {text:?}")]
    Malformed {
        /// The parameter the entry names.
        parameter: WireNodeId,
        /// The binding's declared type.
        value_type: WitnessValueType,
        /// The entry's text.
        text: String,
    },
}

/// FR-073: `Debug` never reproduces the full transcript -- only a bounded
/// descriptor (its byte length and a content digest).
impl fmt::Debug for Witness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Witness")
            .field("transcript_len", &self.transcript.len())
            .field(
                "transcript_digest",
                &DigestRecord::mint(
                    DigestDomain::VerificationJcs,
                    ByteDigest::of(self.transcript.as_bytes()).as_bytes(),
                ),
            )
            .finish()
    }
}

/// FR-073: `Display` never reproduces the full transcript either.
impl fmt::Display for Witness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "witness({} bytes)", self.transcript.len())
    }
}

struct BlockFields<'a> {
    kind: &'a str,
    harness: &'a str,
    check_text: &'a str,
    values: &'a str,
}

/// This module's transcript grammar: a block is
/// `<<<kind|harness|check_text|values>>>`, `values` a `;`-separated list
/// of `name=value` entries, `name` a parameter node id. It carries exactly
/// the structure ADR-013 O-25 admission and decode read: a cover/assertion
/// kind distinction, a single trimmed block, and per-parameter entries.
fn find_blocks(text: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut offset = 0;
    while let Some(start) = text[offset..].find("<<<") {
        let start = offset + start;
        if let Some(end_rel) = text[start..].find(">>>") {
            let end = start + end_rel + 3;
            blocks.push(&text[start..end]);
            offset = end;
        } else {
            break;
        }
    }
    blocks
}

#[qsl_attrs::string_edge]
fn parse_block(block: &str) -> Option<BlockFields<'_>> {
    let inner = block.strip_prefix("<<<")?.strip_suffix(">>>")?;
    let mut parts = inner.splitn(4, '|');
    let kind = parts.next()?;
    let harness = parts.next()?;
    let check_text = parts.next()?;
    let values = parts.next().unwrap_or("");
    Some(BlockFields {
        kind,
        harness,
        check_text,
        values,
    })
}

// ---------------------------------------------------------------------------
// `ReplaySource`, `WitnessEnvelope<P>`: the common O-25 carrier.
// ---------------------------------------------------------------------------

/// A counterexample's canonical assignment when it did not come from a
/// backend transcript (the `ReplaySource::Input` arm): a parameter's wire
/// node id and its typed concrete value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalAssignment {
    /// The parameter this assignment binds.
    pub parameter: WireNodeId,
    /// The parameter's concrete value.
    pub value: WitnessValue,
}

/// ADR-013 O-25: the packet's `source`, an enum with two variants that
/// never both hold data at once. `Witness` derives replay input from its
/// transcript; `Input` is replayed from its own stored canonical
/// assignments (a corpus counterexample), and settles
/// `reproduced-without-witness`, never backend evidence (FR-072).
#[derive(Clone, Eq, PartialEq)]
pub enum ReplaySource {
    /// Replay derives its input from a backend transcript.
    Witness(Witness),
    /// Replay derives its input from stored canonical assignments (a corpus
    /// counterexample); settles `reproduced-without-witness`, never backend
    /// evidence.
    Input(Vec<CanonicalAssignment>),
}

impl ReplaySource {
    /// The bytes this source adds to an encoded record: a witness's
    /// transcript, or for each canonical assignment a 32-byte node id plus a
    /// value of at most 8 bytes (QC-1's digest-addressed shape), a fixed
    /// per-entry bound with no `Debug`-rendered text involved.
    pub(crate) fn measured_bytes(&self) -> usize {
        match self {
            Self::Witness(witness) => witness.transcript().len(),
            Self::Input(assignments) => assignments.len() * (32 + 16),
        }
    }
}

/// FR-073-AC-2/AC-3: neither arm's rendering reproduces its full content.
/// The `Witness` arm delegates to [`Witness`]'s own redacted `Debug`; the
/// `Input` arm renders only its entry count and a content digest, never any
/// [`CanonicalAssignment::value`] (the concrete-argument payload FR-073's
/// Behavior names).
impl fmt::Debug for ReplaySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Witness(witness) => f.debug_tuple("Witness").field(witness).finish(),
            Self::Input(assignments) => {
                let mut buf = Vec::with_capacity(assignments.len() * 40);
                for assignment in assignments {
                    buf.extend_from_slice(assignment.parameter.as_bytes());
                    match assignment.value {
                        WitnessValue::Boolean(value) => {
                            buf.extend_from_slice(&[0, u8::from(value)])
                        }
                        WitnessValue::Integer(value) => {
                            buf.push(1);
                            buf.extend_from_slice(&value.to_le_bytes());
                        }
                    }
                }
                f.debug_struct("Input")
                    .field("entries", &assignments.len())
                    .field("digest", &ByteDigest::of(&buf))
                    .finish()
            }
        }
    }
}

/// FR-070's extension point (FR-070-AC-5): a family attaches its own typed
/// payload by implementing this trait, never by writing into a
/// `String`-keyed map. The common envelope carries `P: FamilyPayload` as a
/// generic parameter and never inspects it.
pub trait FamilyPayload: Clone + fmt::Debug + Eq {
    /// The payload's own measured encoded size, counted against the
    /// envelope's reader bound (FR-070-AC-7): by default its inline size; a
    /// payload holding variable-length content measures that content too.
    fn measured_bytes(&self) -> usize {
        std::mem::size_of_val(self)
    }
}

/// The payload for a family with nothing of its own to attach (e.g. a
/// function-application exemplar, TC-192).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NoPayload;
impl FamilyPayload for NoPayload {}

// FR-116: the `ProtocolClause` family's own payload, a frame counterexample.
mod frame;
pub use frame::{ClaimedChange, FrameCounterexample, FrameOperation};

// FR-122: the state-clause family's own payload, a state-clause
// counterexample.
mod state_clause;
pub use state_clause::StateClauseCounterexample;

// FR-265: the one derivation of a state clause's settlement basis and
// separating witness, which the clause run and the replay share.
mod derivation;
pub(crate) use derivation::derive_separating_witness;
#[cfg(test)]
pub(crate) use derivation::Derived;

/// FR-070/ADR-013 O-25: the typed counterexample/witness envelope, generic
/// over its family-owned payload `P` (the extension point FR-070-AC-5
/// requires). Every field below is one of ADR-013 O-25's own listed
/// members; there is no `String`-keyed or otherwise untyped extra field.
///
/// `derive(Debug)` is safe here for FR-073: the bulk-content fields this
/// type can carry are the transcript (through `source`'s
/// `ReplaySource::Witness(Witness)` arm) and the canonical assignment
/// values (through `source`'s `ReplaySource::Input(Vec<CanonicalAssignment>)`
/// arm), and [`ReplaySource`] implements its own redacted `Debug` covering
/// both arms -- the derive here delegates to that impl for the nested field
/// rather than dumping raw content itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitnessEnvelope<P: FamilyPayload> {
    obligation_identity: ObligationIdentity,
    occurrence_key: OccurrenceKey,
    clause_node: WireNodeId,
    selected_function: QualifiedName,
    package_id: DigestRecord,
    source_digests: Vec<RawSourceRef>,
    profile_selections: Vec<ProfileSelection>,
    run_limits: ScalarLimits,
    declared_domains: Vec<DeclaredDomain>,
    backend: Backend,
    trace_position: Option<TracePosition>,
    source: ReplaySource,
    family_payload: P,
}

impl<P: FamilyPayload> WitnessEnvelope<P> {
    /// The CG-computed obligation identity this counterexample belongs to.
    pub fn obligation_identity(&self) -> ObligationIdentity {
        self.obligation_identity
    }
    /// The occurrence key this counterexample was produced at.
    pub fn occurrence_key(&self) -> &OccurrenceKey {
        &self.occurrence_key
    }
    /// The clause node this counterexample witnesses against.
    pub fn clause_node(&self) -> WireNodeId {
        self.clause_node
    }
    /// The selected function, always a typed [`QualifiedName`].
    pub fn selected_function(&self) -> &QualifiedName {
        &self.selected_function
    }
    /// The package this counterexample was produced from.
    pub fn package_id(&self) -> DigestRecord {
        self.package_id
    }
    /// The package's declared source references.
    pub fn source_digests(&self) -> &[RawSourceRef] {
        &self.source_digests
    }
    /// The semantic profile selections in effect for the proving run.
    pub fn profile_selections(&self) -> &[ProfileSelection] {
        &self.profile_selections
    }
    /// The proving run's `quire.value.accounting/v1` limits (ADR-014 B-2).
    /// They are resource limits, not proof bounds: the finite domains the
    /// run was requested over are [`Self::declared_domains`].
    pub fn run_limits(&self) -> ScalarLimits {
        self.run_limits
    }
    /// The parameter domains declared for the proving run.
    pub fn declared_domains(&self) -> &[DeclaredDomain] {
        &self.declared_domains
    }
    /// The backend that produced this counterexample.
    pub fn backend(&self) -> &Backend {
        &self.backend
    }
    /// The trace position this counterexample was produced at, if the
    /// backend reported one.
    pub fn trace_position(&self) -> Option<&TracePosition> {
        self.trace_position.as_ref()
    }
    /// The witness or input replay source.
    pub fn source(&self) -> &ReplaySource {
        &self.source
    }
    /// The family-owned typed extension payload (FR-070-AC-5).
    pub fn family_payload(&self) -> &P {
        &self.family_payload
    }
}

#[cfg(test)]
mod witness_tests {
    use super::*;
    use ix_trace_rs::trace;

    fn assertion(harness: &str, check: &str, values: &str) -> String {
        format!("<<<assertion|{harness}|{check}|{values}>>>")
    }

    fn integer(parameter: WireNodeId) -> WitnessBinding {
        WitnessBinding {
            parameter,
            value_type: WitnessValueType::I128,
        }
    }

    fn boolean(parameter: WireNodeId) -> WitnessBinding {
        WitnessBinding {
            parameter,
            value_type: WitnessValueType::Boolean,
        }
    }

    /// N5: `check()` and `check_text()` are named in FR-070's Behavior
    /// (`harness_symbol()`, `check()`/`check_text()`, `concrete_values()`
    /// and `decode()` "each recompute their answer from the stored
    /// transcript"), but neither had a test.
    #[test]
    fn check_and_check_text_recompute_from_the_stored_transcript() {
        let witness = Witness::parse(assertion("harness_a", "x == 1", "x=1")).unwrap();
        assert_eq!(witness.check(), "assertion");
        assert_eq!(witness.check_text(), "x == 1");

        let other = Witness::parse(assertion("harness_b", "y != 0", "y=2")).unwrap();
        assert_eq!(other.check_text(), "y != 0");
        assert_ne!(witness.check_text(), other.check_text());
    }

    /// FR-070-AC-1 (TC-180): the type has exactly one field (the
    /// transcript), so an exhaustive no-`..` destructure naming only it
    /// compiles; this is the falsifier a construction-time cache (e.g. a
    /// `OnceCell` alongside `transcript`) would fail, since it would add a
    /// second field this pattern would then have to name.
    #[trace("TC-180", "FR-070-AC-1")]
    #[test]
    fn tc_180_exactly_one_field_and_derived_facts_track_the_stored_transcript() {
        let x = WireNodeId::from_digest([1; 32]);
        let y = WireNodeId::from_digest([2; 32]);
        let bindings = [integer(x), integer(y)];
        let text = assertion("harness_a", "x == 1", &format!("{x}=1;{y}=2"));
        let witness = Witness::parse(text.clone()).unwrap();
        // Step 1: exhaustive destructure naming only `transcript`.
        let Witness { transcript } = witness;
        assert_eq!(transcript, text);

        // Step 2: repeated calls agree with each other.
        let witness = Witness::parse(text.clone()).unwrap();
        assert_eq!(witness.harness_symbol(), witness.harness_symbol());
        assert_eq!(witness.concrete_values(), witness.concrete_values());
        assert_eq!(
            witness.decode(&bindings),
            Ok(vec![WitnessValue::Integer(1), WitnessValue::Integer(2)])
        );
        assert_eq!(witness.decode(&bindings), witness.decode(&bindings));

        // Step 3: a second envelope from identical bytes via a distinct
        // path (simulated here as a second independent `parse` call, this
        // module's stand-in for deserialization) matches the first.
        let second = Witness::parse(text).unwrap();
        assert_eq!(witness.concrete_values(), second.concrete_values());
        assert_eq!(witness.harness_symbol(), second.harness_symbol());

        // Step 4: a differing transcript produces a differing derived value.
        let third_text = assertion("harness_a", "x == 1", &format!("{x}=99;{y}=2"));
        let third = Witness::parse(third_text).unwrap();
        assert_ne!(witness.concrete_values(), third.concrete_values());
        assert_eq!(
            third.decode(&bindings),
            Ok(vec![WitnessValue::Integer(99), WitnessValue::Integer(2)])
        );
    }

    /// FR-070-AC-2 (TC-181, transcript half): a cover-playback transcript,
    /// an untrimmed transcript, and a transcript with zero or two assertion
    /// blocks each refuse at construction, with no readable partial value.
    #[trace("TC-181", "FR-070-AC-2")]
    #[test]
    fn tc_181_refuses_malformed_transcripts() {
        assert_eq!(
            Witness::parse("<<<cover|h|c|>>>"),
            Err(MalformedTranscript::NotAnAssertionBlock)
        );
        assert_eq!(
            Witness::parse(format!("leading {}", assertion("h", "c", ""))),
            Err(MalformedTranscript::Untrimmed)
        );
        assert_eq!(
            Witness::parse(format!("{} trailing", assertion("h", "c", ""))),
            Err(MalformedTranscript::Untrimmed)
        );
        assert_eq!(
            Witness::parse("no blocks here"),
            Err(MalformedTranscript::NoAssertionBlock)
        );
        let two_blocks = format!("{}{}", assertion("a", "c", ""), assertion("b", "c", ""));
        assert_eq!(
            Witness::parse(two_blocks),
            Err(MalformedTranscript::MultipleAssertionBlocks)
        );
    }

    /// FR-098-AC-2: `decode` reads one typed value per binding, in binding
    /// order whatever order the transcript lists its entries in, joined by
    /// parameter node id: `0`/`1` as a Boolean binding's `false`/`true`, and
    /// decimal text as an integer binding's value, `i128::MIN` and
    /// `i128::MAX` included.
    #[trace("TC-444", "FR-098-AC-2")]
    #[test]
    fn tc_444_decode_reads_typed_values_by_parameter_node_id() {
        let flag = WireNodeId::from_digest([1; 32]);
        let low = WireNodeId::from_digest([2; 32]);
        let high = WireNodeId::from_digest([3; 32]);
        let values = format!("{high}={};{flag}=1;{low}={}", i128::MAX, i128::MIN);
        let witness = Witness::parse(assertion("h", "c", &values)).unwrap();
        assert_eq!(
            witness.decode(&[boolean(flag), integer(low), integer(high)]),
            Ok(vec![
                WitnessValue::Boolean(true),
                WitnessValue::Integer(i128::MIN),
                WitnessValue::Integer(i128::MAX),
            ])
        );
        let off = Witness::parse(assertion("h", "c", &format!("{flag}=0"))).unwrap();
        assert_eq!(
            off.decode(&[boolean(flag)]),
            Ok(vec![WitnessValue::Boolean(false)])
        );
    }

    /// FR-098-AC-4: each join failure refuses with its own typed variant --
    /// a parameter with no entry, a parameter with two, an entry naming no
    /// bound parameter, an entry that is not a `name=value` pair, and an
    /// entry whose text is not a value of its
    /// binding's type (`2` for a Boolean, non-decimal text and a value past
    /// `i128::MAX` for an integer).
    #[trace("TC-444", "FR-098-AC-4")]
    #[test]
    fn tc_444_decode_refuses_each_join_failure_with_its_own_variant() {
        let x = WireNodeId::from_digest([1; 32]);
        let b = WireNodeId::from_digest([2; 32]);
        let decode = |values: String, bindings: &[WitnessBinding]| {
            Witness::parse(assertion("h", "c", &values))
                .unwrap()
                .decode(bindings)
        };

        assert_eq!(
            decode(format!("{x}=1"), &[integer(x), boolean(b)]),
            Err(DecodeRefusal::Missing(b))
        );
        assert_eq!(
            decode(format!("{x}=1;{x}=2"), &[integer(x)]),
            Err(DecodeRefusal::Duplicate(x))
        );
        assert_eq!(
            decode(format!("{x}=1;{b}=0"), &[integer(x)]),
            Err(DecodeRefusal::Unbound(b.to_string()))
        );
        assert_eq!(
            decode(format!("{x}=1;junk"), &[integer(x)]),
            Err(DecodeRefusal::MalformedEntry("junk".to_owned()))
        );
        assert_eq!(
            decode(format!("{b}=2"), &[boolean(b)]),
            Err(DecodeRefusal::Malformed {
                parameter: b,
                value_type: WitnessValueType::Boolean,
                text: "2".to_owned(),
            })
        );
        let past_max = format!("{}0", i128::MAX);
        for text in ["not-a-number", past_max.as_str()] {
            assert_eq!(
                decode(format!("{x}={text}"), &[integer(x)]),
                Err(DecodeRefusal::Malformed {
                    parameter: x,
                    value_type: WitnessValueType::I128,
                    text: text.to_owned(),
                })
            );
        }
    }

    /// FR-073-AC-1 (TC-209): neither `Debug` nor `Display` of a constructed
    /// `Witness` reproduces the full transcript text; a distinctive marker
    /// embedded in a long transcript never appears in either rendering, and
    /// the typed accessor still returns it in full.
    #[trace("TC-209", "FR-073-AC-1")]
    #[test]
    fn tc_209_debug_and_display_never_reproduce_the_full_transcript() {
        let marker = "MARKER-6f1e2a3b-distinctive";
        let padding = "x".repeat(4096);
        let values = format!("{marker}=1;padding_{padding}=2");
        let text = assertion("harness_a", "check", &values);
        let witness = Witness::parse(text.clone()).unwrap();

        let debug = format!("{witness:?}");
        let display = witness.to_string();
        assert!(!debug.contains(marker));
        assert!(!display.contains(marker));
        assert!(debug.len() < text.len());
        assert!(display.len() < text.len());

        // The typed accessor is unaffected by rendering redaction.
        assert!(witness
            .concrete_values()
            .iter()
            .any(|(name, _)| name == marker));
        assert_eq!(witness.transcript(), text);
    }
}

// ---------------------------------------------------------------------------
// Reconstruction (FR-070-AC-3, FR-070-AC-4, FR-070-AC-6, FR-070-AC-7).
// ---------------------------------------------------------------------------

/// The wire shape of one [`WitnessEnvelope`]'s O-25 members: every member
/// is `Option`, so omitting one (rather than supplying an empty/default
/// value) is exactly how a caller models "this member is missing"
/// (FR-070-AC-4/TC-183). `trace_position` is `Option<Option<TracePosition>>`
/// because ADR-013 O-25 distinguishes "the packet did not say" (outer
/// `None`, refuses) from "this family has none" (`Some(None)`, admitted).
#[derive(Clone, Debug, Default)]
pub struct WitnessPacket<P: FamilyPayload> {
    /// The CG-computed obligation identity, as raw digest bytes.
    pub obligation_identity: Option<[u8; 32]>,
    /// The occurrence key this counterexample was produced at.
    pub occurrence_key: Option<OccurrenceKey>,
    /// The clause node this counterexample witnesses against.
    pub clause_node: Option<WireNodeId>,
    /// The selected function.
    pub selected_function: Option<QualifiedName>,
    /// `(digest domain, digest hex)` naming the package.
    pub package_id: Option<(Option<String>, String)>,
    /// `(authority, identity, revision namespace, revision, digest domain,
    /// digest hex)` per declared source reference.
    pub source_digests: Option<Vec<SourceDigestWire>>,
    /// The semantic profile selections in effect for the proving run.
    pub profile_selections: Option<Vec<ProfileSelection>>,
    /// The proving run's `quire.value.accounting/v1` limits (ADR-014 B-2).
    pub run_limits: Option<ScalarLimits>,
    /// The parameter domains declared for the proving run.
    pub declared_domains: Option<Vec<DeclaredDomain>>,
    /// The provider identity of the backend that produced this
    /// counterexample (ADR-013 O-19: the identity string alone).
    pub backend: Option<String>,
    /// The family's trace position: outer `None` means the packet omitted
    /// the member (refuses); `Some(None)` means the family has none
    /// (admitted).
    pub trace_position: Option<Option<TracePosition>>,
    /// The witness or input replay source.
    pub source: Option<ReplaySource>,
    /// The family-owned typed extension payload.
    pub family_payload: Option<P>,
}

/// The reader's own measurement of `packet`'s encoded size (FR-070-AC-7):
/// the sum of every variable-length member's own byte length -- never a
/// caller-declared number a packet could understate to launder an oversized
/// transcript or byte-provision content past the bound (B3). `Option`s the
/// packet omitted contribute zero, exactly like an absent wire member would.
fn measured_encoded_bytes<P: FamilyPayload>(packet: &WitnessPacket<P>) -> usize {
    let mut total = 0usize;
    total += packet
        .selected_function
        .as_ref()
        .map_or(0, |name| name.to_string().len());
    total += packet.package_id.as_ref().map_or(0, |(_, hex)| hex.len());
    total += packet.source_digests.as_ref().map_or(0, |digests| {
        digests.iter().map(RawSourceRef::wire_len).sum()
    });
    total += packet.profile_selections.as_ref().map_or(0, |selections| {
        selections
            .iter()
            .map(|s| s.profile().len() + s.value().len())
            .sum()
    });
    total += packet.declared_domains.as_ref().map_or(0, |domains| {
        domains
            .iter()
            .map(|declared| {
                finite_bound_bytes(declared.bound())
                    + match declared.domain() {
                        DomainKey::Node { path, .. } => std::mem::size_of_val(path.as_slice()),
                        DomainKey::Population { ordinal, .. } => std::mem::size_of_val(ordinal),
                    }
            })
            .sum()
    });
    total += packet.backend.as_ref().map_or(0, String::len);
    total += packet
        .trace_position
        .as_ref()
        .and_then(|position| position.as_ref())
        .map_or(0, |position| position.as_str().len());
    total += packet
        .source
        .as_ref()
        .map_or(0, ReplaySource::measured_bytes);
    total += packet
        .family_payload
        .as_ref()
        .map_or(0, FamilyPayload::measured_bytes);
    total
}

/// A declared domain's measured size: 32 bytes for its parameter node id,
/// then 8 per fixed-width maximum, or the decimal digits of both ends of an
/// integer range, whose ends are unbounded integers.
fn finite_bound_bytes(bound: &FiniteBound) -> usize {
    32 + match bound {
        FiniteBound::Cardinality { .. } | FiniteBound::Depth { .. } => 8,
        FiniteBound::IntegerRange(range) => {
            range.lower().to_string().len() + range.upper().to_string().len()
        }
    }
}

/// [`WitnessEnvelope::reconstruct`]'s structured refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WitnessRefusal {
    /// A required O-25 member is absent from the packet.
    #[error("missing_declaration: O-25 member {0:?} is absent")]
    MissingMember(&'static str),
    /// The `backend` member is empty (`invalid_identifier`, ADR-013 O-19).
    #[error("invalid_identifier: the backend identity is empty")]
    EmptyBackendIdentity,
    /// A digest names a domain outside the closed FR-201 set, or supplies no
    /// domain at all.
    #[error("stale_dependency/digest-domain-mismatch: {0:?}: {1}")]
    DigestDomainMismatch(&'static str, InvalidDigestRecord),
    /// A digest names a domain FR-201 admits, but its own hex encoding is
    /// malformed (wrong length, or not lowercase hex) -- not a domain
    /// problem, so it is never spelled `digest-domain-mismatch`.
    #[error("invalid_digest/malformed-encoding: {0:?}: {1}")]
    MalformedDigest(&'static str, InvalidDigestRecord),
    /// The encoded packet exceeds the configured reader bound.
    #[error(transparent)]
    BoundExceeded(#[from] BoundExceeded),
}

/// Route `err` to [`WitnessRefusal::DigestDomainMismatch`] or
/// [`WitnessRefusal::MalformedDigest`] by its real cause, so a wrong hex
/// length is never reported as a domain problem.
fn classify_digest_error(member: &'static str, err: InvalidDigestRecord) -> WitnessRefusal {
    if err.is_domain_mismatch() {
        WitnessRefusal::DigestDomainMismatch(member, err)
    } else {
        WitnessRefusal::MalformedDigest(member, err)
    }
}

impl<P: FamilyPayload> WitnessEnvelope<P> {
    /// Reconstruct a positive envelope from `packet`. Refuses (with no
    /// partially-built envelope returned) if the encoded size is over the
    /// configured bound, if any O-25 member is absent, or if a digest names a domain outside the closed FR-201 set.
    #[qsl_attrs::string_edge]
    pub fn reconstruct(
        packet: WitnessPacket<P>,
        limits: ReplayLimits,
    ) -> Result<Self, WitnessRefusal> {
        BoundExceeded::check(measured_encoded_bytes(&packet), limits)?;

        let obligation_identity = packet
            .obligation_identity
            .map(ObligationIdentity::from_digest)
            .ok_or(WitnessRefusal::MissingMember("obligation_identity"))?;
        let occurrence_key = packet
            .occurrence_key
            .ok_or(WitnessRefusal::MissingMember("occurrence_key"))?;
        let clause_node = packet
            .clause_node
            .ok_or(WitnessRefusal::MissingMember("clause_node"))?;
        let selected_function = packet
            .selected_function
            .ok_or(WitnessRefusal::MissingMember("selected_function"))?;
        let (package_domain, package_hex) = packet
            .package_id
            .ok_or(WitnessRefusal::MissingMember("package_id"))?;
        let package_id = DigestRecord::from_wire(package_domain.as_deref(), &package_hex)
            .map_err(|e| classify_digest_error("package_id", e))?;
        let source_digests = packet
            .source_digests
            .ok_or(WitnessRefusal::MissingMember("source_digests"))?
            .into_iter()
            .map(|wire| {
                RawSourceRef::from_wire(wire)
                    .map_err(|e| classify_digest_error("source_digests", e))
            })
            .collect::<Result<Vec<_>, WitnessRefusal>>()?;
        let profile_selections = packet
            .profile_selections
            .ok_or(WitnessRefusal::MissingMember("profile_selections"))?;
        let run_limits = packet
            .run_limits
            .ok_or(WitnessRefusal::MissingMember("run_limits"))?;
        let declared_domains = packet
            .declared_domains
            .ok_or(WitnessRefusal::MissingMember("declared_domains"))?;
        let backend_identity = packet
            .backend
            .ok_or(WitnessRefusal::MissingMember("backend"))?;
        if backend_identity.is_empty() {
            return Err(WitnessRefusal::EmptyBackendIdentity);
        }
        let backend = Backend::new(backend_identity);
        let trace_position = packet
            .trace_position
            .ok_or(WitnessRefusal::MissingMember("trace_position"))?;
        let source = packet
            .source
            .ok_or(WitnessRefusal::MissingMember("source"))?;
        let family_payload = packet
            .family_payload
            .ok_or(WitnessRefusal::MissingMember("family_payload"))?;

        Ok(Self {
            obligation_identity,
            occurrence_key,
            clause_node,
            selected_function,
            package_id,
            source_digests,
            profile_selections,
            run_limits,
            declared_domains,
            backend,
            trace_position,
            source,
            family_payload,
        })
    }

    /// This envelope's members, in wire-packet shape, for round-tripping
    /// through [`Self::reconstruct`] (FR-070-AC-3's construct -> serialize
    /// -> read round trip; #231 uses this in-process shape rather than a
    /// byte-level wire format, since no FR-322/`counterexamples` wire
    /// writer exists yet on `origin/main`).
    pub fn to_packet(&self) -> WitnessPacket<P> {
        WitnessPacket {
            obligation_identity: Some(*self.obligation_identity.as_bytes()),
            occurrence_key: Some(self.occurrence_key.clone()),
            clause_node: Some(self.clause_node),
            selected_function: Some(self.selected_function.clone()),
            package_id: Some((
                Some(self.package_id.domain().as_str().to_owned()),
                self.package_id.hex(),
            )),
            source_digests: Some(
                self.source_digests
                    .iter()
                    .map(RawSourceRef::to_wire)
                    .collect(),
            ),
            profile_selections: Some(self.profile_selections.clone()),
            run_limits: Some(self.run_limits),
            declared_domains: Some(self.declared_domains.clone()),
            backend: Some(self.backend.identity().to_owned()),
            trace_position: Some(self.trace_position.clone()),
            source: Some(self.source.clone()),
            family_payload: Some(self.family_payload.clone()),
        }
    }
}

/// Build an [`Origin`] from a role string and ordinal, this module's own
/// small convenience over `quire_exact::Origin::new`. `#[cfg(test)]`: used
/// only to build occurrence keys in this module's and its siblings' test
/// fixtures, never part of the CG-facing public API this module re-exports
/// (ADR-011 FB-05) -- genuinely test-only, not product surface, so it lives
/// under the same cfg as its callers rather than behind `#[allow(dead_code)]`.
#[cfg(test)]
pub(crate) fn origin(role: &str, ordinal: u64) -> Origin {
    Origin::new(role.into(), ordinal)
}

#[cfg(test)]
mod envelope_tests {
    use super::*;
    use crate::bounds::DEFAULT_INPUT_BYTES;
    use ix_trace_rs::trace;
    use qsl_foundation::bound::ProofBound;
    use quire_exact::Identifier;

    fn digest(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn scalar_limits(seed: u64) -> ScalarLimits {
        ScalarLimits {
            integer_bits: seed,
            decimal_digits: seed,
            scale_expansion: seed,
            text_input_bytes: seed,
            text_scalars: seed,
            normalized_scalars: seed,
            unit_edges: seed,
            value_occurrences: seed,
            work_units: seed,
            result_units: seed,
        }
    }

    fn full_packet(occurrence_ordinal: u64) -> WitnessPacket<NoPayload> {
        WitnessPacket {
            obligation_identity: Some(digest(1)),
            occurrence_key: Some(OccurrenceKey::new(
                WireNodeId::from_digest(digest(2)),
                origin("reference", occurrence_ordinal),
            )),
            clause_node: Some(WireNodeId::from_digest(digest(3))),
            selected_function: Some(
                QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap(),
            ),
            package_id: Some((
                Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::PackageSemanticV2, digest(4)).hex(),
            )),
            source_digests: Some(vec![(
                "registry".to_owned(),
                "pkg-a".to_owned(),
                "git".to_owned(),
                "rev-1".to_owned(),
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::SourceBytesV1, digest(5)).hex(),
            )]),
            profile_selections: Some(vec![ProfileSelection::new(
                "quire.profile.v1".to_owned(),
                "finite-state".to_owned(),
            )]),
            run_limits: Some(scalar_limits(64)),
            declared_domains: Some(vec![DeclaredDomain::new(ProofBound {
                domain: DomainKey::Node {
                    node: WireNodeId::from_digest(digest(6)),
                    path: Vec::new(),
                },
                bound: FiniteBound::integer_range(
                    quire_exact::Integer::from(0_i64),
                    quire_exact::Integer::from(u64::from(u32::MAX)),
                )
                .expect("a non-empty range"),
            })]),
            backend: Some("kani-backend-1".to_owned()),
            trace_position: Some(Some(TracePosition::new("frame-0".to_owned()))),
            source: Some(ReplaySource::Witness(
                Witness::parse("<<<assertion|h|c|x=1>>>").unwrap(),
            )),
            family_payload: Some(NoPayload),
        }
    }

    /// FR-070-AC-3 (TC-182): a positive envelope's construct -> serialize
    /// (`to_packet`) -> read (`reconstruct`) round trip preserves the
    /// transcript and every O-25 member exactly, and two occurrences of the
    /// same node id stay distinguishable by role/ordinal after the round
    /// trip.
    #[trace("TC-182", "FR-070-AC-3")]
    #[test]
    fn tc_182_round_trip_preserves_every_o25_member_and_the_transcript() {
        let envelope_a =
            WitnessEnvelope::reconstruct(full_packet(0), crate::ReplayLimits::default()).unwrap();
        let envelope_b =
            WitnessEnvelope::reconstruct(full_packet(1), crate::ReplayLimits::default()).unwrap();

        let round_tripped_a =
            WitnessEnvelope::reconstruct(envelope_a.to_packet(), crate::ReplayLimits::default())
                .unwrap();
        assert_eq!(envelope_a, round_tripped_a);
        let round_tripped_b =
            WitnessEnvelope::reconstruct(envelope_b.to_packet(), crate::ReplayLimits::default())
                .unwrap();
        assert_eq!(envelope_b, round_tripped_b);

        // Same node id, distinguishable by role/ordinal, after round trip.
        assert_eq!(
            round_tripped_a.occurrence_key().node(),
            round_tripped_b.occurrence_key().node()
        );
        assert_ne!(
            round_tripped_a.occurrence_key().origin().ordinal(),
            round_tripped_b.occurrence_key().origin().ordinal()
        );
        assert_ne!(
            round_tripped_a.occurrence_key(),
            round_tripped_b.occurrence_key()
        );

        // `Input`-arm envelope round trip too.
        let mut input_packet = full_packet(0);
        input_packet.source = Some(ReplaySource::Input(vec![CanonicalAssignment {
            parameter: WireNodeId::from_digest(digest(9)),
            value: WitnessValue::Integer(42),
        }]));
        let input_envelope =
            WitnessEnvelope::reconstruct(input_packet, crate::ReplayLimits::default()).unwrap();
        let input_round_tripped = WitnessEnvelope::reconstruct(
            input_envelope.to_packet(),
            crate::ReplayLimits::default(),
        )
        .unwrap();
        assert_eq!(input_envelope, input_round_tripped);
    }

    /// FR-070-AC-3 (TC-182; ADR-014 §1, §11): the proving run's accounting
    /// limits travel as `run_limits` (B-2) and its per-parameter finite
    /// domains as typed `FiniteBound`s (B-4); each round-trips as itself,
    /// and a packet without `run_limits` refuses by that member's name.
    #[trace("TC-182", "FR-070-AC-3")]
    #[test]
    fn tc_182_run_limits_and_declared_domains_round_trip_as_their_own_kinds() {
        let mut packet = full_packet(0);
        // Two bounds on one parameter, at the parameter's own position and
        // at its element: distinct by their domain keys.
        let parameter = WireNodeId::from_digest(digest(6));
        let whole = DomainKey::Node {
            node: parameter,
            path: Vec::new(),
        };
        let element = DomainKey::Node {
            node: parameter,
            path: vec![0],
        };
        packet.declared_domains = Some(vec![
            DeclaredDomain::new(ProofBound {
                domain: whole.clone(),
                bound: FiniteBound::cardinality(8),
            }),
            DeclaredDomain::new(ProofBound {
                domain: element.clone(),
                bound: FiniteBound::depth(3).unwrap(),
            }),
        ]);
        let envelope =
            WitnessEnvelope::reconstruct(packet, crate::ReplayLimits::default()).unwrap();
        assert_eq!(envelope.run_limits(), scalar_limits(64));
        assert_eq!(
            envelope
                .declared_domains()
                .iter()
                .map(|declared| (declared.domain(), declared.bound()))
                .collect::<Vec<_>>(),
            [
                (&whole, &FiniteBound::cardinality(8)),
                (&element, &FiniteBound::depth(3).unwrap()),
            ]
        );
        assert!(envelope.declared_domains().iter().all(|declared| matches!(
            declared.domain(),
            DomainKey::Node { node, .. } if *node == parameter
        )));
        let round_tripped =
            WitnessEnvelope::reconstruct(envelope.to_packet(), crate::ReplayLimits::default())
                .unwrap();
        assert_eq!(round_tripped, envelope);

        let mut missing = full_packet(0);
        missing.run_limits = None;
        assert!(matches!(
            WitnessEnvelope::reconstruct(missing, crate::ReplayLimits::default()),
            Err(WitnessRefusal::MissingMember("run_limits"))
        ));
    }

    /// FR-070-AC-4 (TC-183): reconstruction refuses when any one required
    /// O-25 member is absent, with no defaulted value substituted.
    #[trace("TC-183", "FR-070-AC-4")]
    #[test]
    fn tc_183_refuses_reconstruction_when_any_o25_member_is_missing() {
        let mut missing_backend = full_packet(0);
        missing_backend.backend = None;
        assert!(matches!(
            WitnessEnvelope::reconstruct(missing_backend, crate::ReplayLimits::default()),
            Err(WitnessRefusal::MissingMember("backend"))
        ));

        let mut missing_trace_position = full_packet(0);
        missing_trace_position.trace_position = None;
        assert!(matches!(
            WitnessEnvelope::reconstruct(missing_trace_position, crate::ReplayLimits::default()),
            Err(WitnessRefusal::MissingMember("trace_position"))
        ));

        let mut missing_source_digest = full_packet(0);
        missing_source_digest.source_digests = None;
        assert!(matches!(
            WitnessEnvelope::reconstruct(missing_source_digest, crate::ReplayLimits::default()),
            Err(WitnessRefusal::MissingMember("source_digests"))
        ));

        let mut missing_obligation = full_packet(0);
        missing_obligation.obligation_identity = None;
        assert!(matches!(
            WitnessEnvelope::reconstruct(missing_obligation, crate::ReplayLimits::default()),
            Err(WitnessRefusal::MissingMember("obligation_identity"))
        ));
    }

    /// FR-070-AC-5 (TC-184): the extension point is a generic parameter
    /// bounded by [`FamilyPayload`], never a `String`-keyed map. A new
    /// family-owned payload type (standing in for #186's state `forall`)
    /// attaches with no edit to this module.
    #[trace("TC-184", "FR-070-AC-5")]
    #[test]
    fn tc_184_family_payload_is_a_typed_extension_point() {
        #[derive(Clone, Debug, Eq, PartialEq)]
        struct StateForallPayload {
            bound_index: u32,
        }
        impl FamilyPayload for StateForallPayload {}

        let mut packet = full_packet(0).into_generic();
        packet.family_payload = Some(StateForallPayload { bound_index: 3 });
        let envelope =
            WitnessEnvelope::reconstruct(packet, crate::ReplayLimits::default()).unwrap();
        assert_eq!(envelope.family_payload().bound_index, 3);
        let round_tripped =
            WitnessEnvelope::reconstruct(envelope.to_packet(), crate::ReplayLimits::default())
                .unwrap();
        assert_eq!(envelope, round_tripped);

        // The extension point is a generic type parameter, not a
        // `String`-keyed map: there is no `envelope.get_extra("bound_index")`
        // or `envelope.payload["bound_index"]` API on `WitnessEnvelope` at
        // all -- that call site does not exist to attempt compiling.
    }

    /// FR-070-AC-6 (TC-181, digest-domain half): a `RawSourceRef` digest
    /// whose domain is outside the closed FR-201 set refuses at
    /// reconstruction.
    #[trace("TC-181", "FR-070-AC-6")]
    #[test]
    fn tc_181_refuses_an_out_of_domain_digest() {
        let mut packet = full_packet(0);
        packet.source_digests = Some(vec![(
            "registry".to_owned(),
            "pkg-a".to_owned(),
            "git".to_owned(),
            "rev-1".to_owned(),
            Some("quire.not-a-real-domain/v1".to_owned()),
            "ab".repeat(32),
        )]);
        assert!(matches!(
            WitnessEnvelope::reconstruct(packet, crate::ReplayLimits::default()),
            Err(WitnessRefusal::DigestDomainMismatch("source_digests", _))
        ));
    }

    /// N5: a `RawSourceRef` digest with a valid FR-201 domain but malformed
    /// hex (wrong length) refuses distinctly from an out-of-domain digest --
    /// never spelled `digest-domain-mismatch`, since the domain itself named
    /// no problem here.
    #[test]
    fn refuses_a_malformed_digest_encoding_distinctly_from_a_domain_mismatch() {
        let mut packet = full_packet(0);
        packet.source_digests = Some(vec![(
            "registry".to_owned(),
            "pkg-a".to_owned(),
            "git".to_owned(),
            "rev-1".to_owned(),
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            "ab".repeat(31), // 62 hex chars, not 64
        )]);
        assert!(matches!(
            WitnessEnvelope::reconstruct(packet, crate::ReplayLimits::default()),
            Err(WitnessRefusal::MalformedDigest("source_digests", _))
        ));
    }

    /// ADR-013 O-19: an empty `backend` member refuses reconstruction.
    #[test]
    fn an_empty_backend_member_refuses() {
        let mut packet = full_packet(0);
        packet.backend = Some(String::new());
        assert_eq!(
            WitnessEnvelope::reconstruct(packet, crate::ReplayLimits::default()),
            Err(WitnessRefusal::EmptyBackendIdentity)
        );
    }

    /// FR-070-AC-7 (TC-181, bound half): an oversized encoding refuses, and
    /// no envelope is returned.
    #[trace("TC-181", "FR-070-AC-7")]
    #[test]
    fn tc_181_refuses_an_oversized_encoding() {
        // B3: the bound check measures the packet's own content -- there is
        // no `encoded_bytes` field a caller could understate -- so an
        // oversized packet has to actually carry oversized content.
        let mut packet = full_packet(0);
        if let Some(identity) = packet.backend.as_mut() {
            *identity = "x".repeat(DEFAULT_INPUT_BYTES + 1);
        } else {
            panic!("full_packet carries a backend");
        }
        assert!(matches!(
            WitnessEnvelope::reconstruct(packet, crate::ReplayLimits::default()),
            Err(WitnessRefusal::BoundExceeded(_))
        ));
    }

    /// TC-181 (FR-070-AC-7): a declared domain's key path counts toward the
    /// measured size, so a path long enough to pass the reader bound on its
    /// own refuses, where the same domain with a one-entry path admits.
    #[trace("TC-181", "FR-070-AC-7")]
    #[test]
    fn tc_181_a_long_declared_domain_path_counts_toward_the_bound() {
        let domain = |path: Vec<u32>| {
            DeclaredDomain::new(ProofBound {
                domain: DomainKey::Node {
                    node: WireNodeId::from_digest(digest(6)),
                    path,
                },
                bound: FiniteBound::cardinality(8),
            })
        };
        let mut short = full_packet(0);
        short.declared_domains = Some(vec![domain(vec![0])]);
        assert!(WitnessEnvelope::reconstruct(short, crate::ReplayLimits::default()).is_ok());

        let entries = DEFAULT_INPUT_BYTES / std::mem::size_of::<u32>() + 1;
        let mut long = full_packet(0);
        long.declared_domains = Some(vec![domain(vec![0; entries])]);
        assert!(matches!(
            WitnessEnvelope::reconstruct(long, crate::ReplayLimits::default()),
            Err(WitnessRefusal::BoundExceeded(_))
        ));
    }

    impl WitnessPacket<NoPayload> {
        /// Test helper: re-tag a `NoPayload` packet's `family_payload` slot
        /// as `None` under a different `P`, so TC-184 can supply its own
        /// family payload type without constructing a whole new packet by
        /// hand.
        fn into_generic<P: FamilyPayload>(self) -> WitnessPacket<P> {
            WitnessPacket {
                obligation_identity: self.obligation_identity,
                occurrence_key: self.occurrence_key,
                clause_node: self.clause_node,
                selected_function: self.selected_function,
                package_id: self.package_id,
                source_digests: self.source_digests,
                profile_selections: self.profile_selections,
                run_limits: self.run_limits,
                declared_domains: self.declared_domains,
                backend: self.backend,
                trace_position: self.trace_position,
                source: self.source,
                family_payload: None,
            }
        }
    }
}

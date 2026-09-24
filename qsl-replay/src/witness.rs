// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-070 (ADR-013 O-25): the typed counterexample/witness envelope.
//!
//! Two nested types, matching ADR-013 O-25's own split:
//!
//! - [`Witness`] mirrors IR's own `Witness{transcript}` (`src/kani/
//!   witness.rs`, IR PR #139): exactly one stored field, the admitted
//!   transcript, with every other witness fact derived from it on every
//!   call. QSL has no Rust dependency edge onto IR's real `Witness` type
//!   (the `quire-contract-ir` Cargo dependency is a compatibility bridge to
//!   `quire-contract-model`, a distinct crate with no Kani/witness code at
//!   all), so this is QSL's own copy of the same admission rule, applied to
//!   the transcript text this envelope reads off the FR-331 wire, not a
//!   port of IR's implementation.
//! - [`WitnessEnvelope`] is the common ADR-013 O-25 carrier: every member
//!   FR-070's Behavior section lists, plus the [`FamilyPayload`] extension
//!   point a family (starting with #186's state `forall`) attaches its own
//!   typed payload through.

use std::fmt;

#[cfg(test)]
use quire_exact::Origin;
use quire_exact::ScalarLimits;

use crate::bounds::BoundExceeded;
use crate::identity::{
    Backend, DeclaredDomain, ObligationIdentity, ProfileSelection, QualifiedName, RawSourceRef,
    SourceDigestWire, TracePosition,
};
use qsl_foundation::digest::{
    ByteDigest, DigestDomain, DigestRecord, InvalidDigestRecord, ManifestDigest, WireNodeId,
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

    /// The transcript's raw `(name, value text)` bindings, split but not yet
    /// parsed -- an entry whose text after `=` does not parse as an integer
    /// still appears here, unlike [`Self::concrete_values`], so
    /// [`Self::decode`] can tell "no binding named this parameter" apart
    /// from "a binding named it but its value was malformed" (the diagnosis
    /// [`Self::concrete_values`]'s silent `Option`-drop used to erase).
    fn raw_bindings(&self) -> Vec<(&str, &str)> {
        self.fields()
            .values
            .split(';')
            .filter(|entry| !entry.is_empty())
            .filter_map(|entry| entry.split_once('='))
            .collect()
    }

    /// The transcript's concrete `(name, value)` bindings, recomputed from
    /// the stored transcript on every call. A binding whose text does not
    /// parse as an integer is skipped here; [`Self::decode`] reports that
    /// case with a typed [`DecodeRefusal::Malformed`] rather than silently
    /// treating the parameter as unbound.
    pub fn concrete_values(&self) -> Vec<(String, i64)> {
        self.raw_bindings()
            .into_iter()
            .filter_map(|(name, value)| Some((name.to_owned(), value.parse().ok()?)))
            .collect()
    }

    /// [`Self::concrete_values`] projected onto `order`'s parameter names,
    /// the harness argument order (ADR-013 O-25: "harness argument order
    /// equals `arguments` order"). Refuses with [`DecodeRefusal::Missing`]
    /// if a name in `order` has no binding in the transcript at all, or
    /// [`DecodeRefusal::Malformed`] if it has a binding whose text does not
    /// parse as an integer -- the two are distinguished by reading
    /// `raw_bindings` directly rather than through
    /// [`Self::concrete_values`]'s already-filtered, already-parsed list.
    pub fn decode(&self, order: &[String]) -> Result<Vec<i64>, DecodeRefusal> {
        let bindings = self.raw_bindings();
        order
            .iter()
            .map(|name| {
                match bindings
                    .iter()
                    .find(|(bound_name, _)| *bound_name == name.as_str())
                {
                    Some((_, text)) => text
                        .parse()
                        .map_err(|_| DecodeRefusal::Malformed(name.clone(), (*text).to_owned())),
                    None => Err(DecodeRefusal::Missing(name.clone())),
                }
            })
            .collect()
    }
}

/// [`Witness::decode`]'s structured refusal: a named parameter with no
/// binding at all in the transcript, distinguished from one whose bound text
/// failed to parse as an integer -- conflating the two would report a
/// malformed value as an absent parameter.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DecodeRefusal {
    /// No witness binding names this parameter at all.
    #[error("no witness binding for parameter {0:?}")]
    Missing(String),
    /// A witness binding names this parameter, but its bound text does not
    /// parse as an integer.
    #[error("witness binding for parameter {0:?} does not parse as an integer: {1:?}")]
    Malformed(String, String),
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

/// This module's own transcript grammar (not IR's real Kani-playback
/// syntax, which QSL cannot reach -- see the module doc comment): a block
/// is `<<<kind|harness|check_text|values>>>`, `values` a `;`-separated list
/// of `name=integer` bindings. Exactly the structural shape ADR-013 O-25
/// admission needs to be tested against (a cover/assertion kind
/// distinction, and single-block/trimmed-content checks), with no claim to
/// any wider fidelity.
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
/// node id and its concrete integer value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalAssignment {
    /// The parameter this assignment binds.
    pub parameter: WireNodeId,
    /// The parameter's concrete integer value.
    pub value: i64,
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
                    buf.extend_from_slice(&assignment.value.to_le_bytes());
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
pub trait FamilyPayload: Clone + fmt::Debug + Eq {}

/// The payload for a family with nothing of its own to attach (e.g. a
/// function-application exemplar, TC-192).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NoPayload;
impl FamilyPayload for NoPayload {}

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
    package_contract_version: String,
    source_digests: Vec<RawSourceRef>,
    profile_selections: Vec<ProfileSelection>,
    proof_bounds: ScalarLimits,
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
    /// The package's declared contract version.
    pub fn package_contract_version(&self) -> &str {
        &self.package_contract_version
    }
    /// The package's declared source references.
    pub fn source_digests(&self) -> &[RawSourceRef] {
        &self.source_digests
    }
    /// The semantic profile selections in effect for the proving run.
    pub fn profile_selections(&self) -> &[ProfileSelection] {
        &self.profile_selections
    }
    /// The proving run's scalar accounting bounds.
    pub fn proof_bounds(&self) -> ScalarLimits {
        self.proof_bounds
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
        let text = assertion("harness_a", "x == 1", "x=1;y=2");
        let witness = Witness::parse(text.clone()).unwrap();
        // Step 1: exhaustive destructure naming only `transcript`.
        let Witness { transcript } = witness;
        assert_eq!(transcript, text);

        // Step 2: repeated calls agree with each other.
        let witness = Witness::parse(text.clone()).unwrap();
        assert_eq!(witness.harness_symbol(), witness.harness_symbol());
        assert_eq!(witness.concrete_values(), witness.concrete_values());
        assert_eq!(
            witness.decode(&["x".to_owned()]),
            witness.decode(&["x".to_owned()])
        );

        // Step 3: a second envelope from identical bytes via a distinct
        // path (simulated here as a second independent `parse` call, this
        // module's stand-in for deserialization) matches the first.
        let second = Witness::parse(text).unwrap();
        assert_eq!(witness.concrete_values(), second.concrete_values());
        assert_eq!(witness.harness_symbol(), second.harness_symbol());

        // Step 4: a differing transcript produces a differing derived value.
        let third_text = assertion("harness_a", "x == 1", "x=99;y=2");
        let third = Witness::parse(third_text).unwrap();
        assert_ne!(witness.concrete_values(), third.concrete_values());
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

    /// N5: `decode` distinguishes a parameter with no binding at all from
    /// one whose bound text fails to parse as an integer -- the fix for
    /// `concrete_values`'s old silent `.ok()?` drop, which used to make
    /// `decode` misreport a malformed value as a missing parameter.
    #[test]
    fn decode_distinguishes_malformed_value_from_missing_binding() {
        let witness = Witness::parse(assertion("h", "c", "x=not-a-number;y=2")).unwrap();

        assert_eq!(
            witness.decode(&["x".to_owned()]),
            Err(DecodeRefusal::Malformed(
                "x".to_owned(),
                "not-a-number".to_owned()
            ))
        );
        assert_eq!(
            witness.decode(&["z".to_owned()]),
            Err(DecodeRefusal::Missing("z".to_owned()))
        );
        assert_eq!(witness.decode(&["y".to_owned()]), Ok(vec![2]));

        // `concrete_values()` still silently skips the malformed entry (its
        // own documented behavior); only `decode` must not conflate the two
        // causes.
        assert!(witness
            .concrete_values()
            .iter()
            .all(|(name, _)| name != "x"));
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
    /// The package's declared contract version.
    pub package_contract_version: Option<String>,
    /// `(authority, identity, revision, digest domain, digest hex)` per
    /// declared source reference.
    pub source_digests: Option<Vec<SourceDigestWire>>,
    /// The semantic profile selections in effect for the proving run.
    pub profile_selections: Option<Vec<ProfileSelection>>,
    /// The proving run's scalar accounting bounds.
    pub proof_bounds: Option<ScalarLimits>,
    /// The parameter domains declared for the proving run.
    pub declared_domains: Option<Vec<DeclaredDomain>>,
    /// `(identity, manifest digest domain, manifest digest hex)` for the
    /// backend that produced this counterexample.
    pub backend: Option<(String, Option<String>, String)>,
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
    total += packet
        .package_contract_version
        .as_deref()
        .map_or(0, str::len);
    total += packet.source_digests.as_ref().map_or(0, |digests| {
        digests
            .iter()
            .map(|(authority, identity, revision, _, hex)| {
                authority.len() + identity.len() + revision.len() + hex.len()
            })
            .sum()
    });
    total += packet.profile_selections.as_ref().map_or(0, |selections| {
        selections
            .iter()
            .map(|s| s.profile().len() + s.value().len())
            .sum()
    });
    total += packet
        .declared_domains
        .as_ref()
        .map_or(0, |domains| domains.iter().map(|d| d.domain().len()).sum());
    total += packet
        .backend
        .as_ref()
        .map_or(0, |(identity, _, hex)| identity.len() + hex.len());
    total += packet
        .trace_position
        .as_ref()
        .and_then(|position| position.as_ref())
        .map_or(0, |position| position.as_str().len());
    total += match &packet.source {
        Some(ReplaySource::Witness(witness)) => witness.transcript().len(),
        // Each canonical assignment is a 32-byte node id plus an 8-byte
        // integer value on the wire (QC-1's digest-addressed shape) -- a
        // fixed per-entry size, no `Debug`-rendered text involved.
        Some(ReplaySource::Input(assignments)) => assignments.len() * (32 + 8),
        None => 0,
    };
    total += packet
        .family_payload
        .as_ref()
        .map_or(0, std::mem::size_of_val);
    total
}

/// [`WitnessEnvelope::reconstruct`]'s structured refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WitnessRefusal {
    /// A required O-25 member is absent from the packet.
    #[error("missing_declaration: O-25 member {0:?} is absent")]
    MissingMember(&'static str),
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
    /// configured bound, if any O-25 member is absent, or if a digest
    /// names a domain outside the closed FR-201 set.
    pub fn reconstruct(packet: WitnessPacket<P>) -> Result<Self, WitnessRefusal> {
        BoundExceeded::check(measured_encoded_bytes(&packet))?;

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
        let package_contract_version = packet
            .package_contract_version
            .ok_or(WitnessRefusal::MissingMember("package_contract_version"))?;
        let source_digests = packet
            .source_digests
            .ok_or(WitnessRefusal::MissingMember("source_digests"))?
            .into_iter()
            .map(|(authority, identity, revision, domain, hex)| {
                let digest = DigestRecord::from_wire(domain.as_deref(), &hex)
                    .map_err(|e| classify_digest_error("source_digests", e))?;
                Ok(RawSourceRef::new(authority, identity, revision, digest))
            })
            .collect::<Result<Vec<_>, WitnessRefusal>>()?;
        let profile_selections = packet
            .profile_selections
            .ok_or(WitnessRefusal::MissingMember("profile_selections"))?;
        let proof_bounds = packet
            .proof_bounds
            .ok_or(WitnessRefusal::MissingMember("proof_bounds"))?;
        let declared_domains = packet
            .declared_domains
            .ok_or(WitnessRefusal::MissingMember("declared_domains"))?;
        let (backend_identity, backend_domain, backend_hex) = packet
            .backend
            .ok_or(WitnessRefusal::MissingMember("backend"))?;
        // ADR-013 C-27 (QSL-227): the backend digest is not just any FR-201
        // domain -- it must be `quire.tool-manifest.jcs/v1`, checked before
        // the hex bytes are read. `ManifestDigest::from_wire` cannot
        // construct anything else.
        let backend_digest = ManifestDigest::from_wire(backend_domain.as_deref(), &backend_hex)
            .map_err(|e| classify_digest_error("backend", e))?;
        let backend = Backend::new(backend_identity, backend_digest);
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
            package_contract_version,
            source_digests,
            profile_selections,
            proof_bounds,
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
            package_contract_version: Some(self.package_contract_version.clone()),
            source_digests: Some(
                self.source_digests
                    .iter()
                    .map(|r| {
                        (
                            r.authority().to_owned(),
                            r.identity().to_owned(),
                            r.revision().to_owned(),
                            Some(r.digest().domain().as_str().to_owned()),
                            r.digest().hex(),
                        )
                    })
                    .collect(),
            ),
            profile_selections: Some(self.profile_selections.clone()),
            proof_bounds: Some(self.proof_bounds),
            declared_domains: Some(self.declared_domains.clone()),
            backend: Some((
                self.backend.identity().to_owned(),
                Some(
                    self.backend
                        .manifest_digest()
                        .record()
                        .domain()
                        .as_str()
                        .to_owned(),
                ),
                self.backend.manifest_digest().record().hex(),
            )),
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
    use crate::bounds::MAX_ENCODED_BYTES;
    use ix_trace_rs::trace;
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
            package_contract_version: Some("quire.checked-package/v2".to_owned()),
            source_digests: Some(vec![(
                "registry".to_owned(),
                "pkg-a".to_owned(),
                "rev-1".to_owned(),
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::SourceBytesV1, digest(5)).hex(),
            )]),
            profile_selections: Some(vec![ProfileSelection::new(
                "quire.profile.v1".to_owned(),
                "finite-state".to_owned(),
            )]),
            proof_bounds: Some(scalar_limits(64)),
            declared_domains: Some(vec![DeclaredDomain::new(
                WireNodeId::from_digest(digest(6)),
                "u32".to_owned(),
            )]),
            backend: Some((
                "kani-backend-1".to_owned(),
                Some(DigestDomain::ToolManifestJcsV1.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::ToolManifestJcsV1, digest(7)).hex(),
            )),
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
        let envelope_a = WitnessEnvelope::reconstruct(full_packet(0)).unwrap();
        let envelope_b = WitnessEnvelope::reconstruct(full_packet(1)).unwrap();

        let round_tripped_a = WitnessEnvelope::reconstruct(envelope_a.to_packet()).unwrap();
        assert_eq!(envelope_a, round_tripped_a);
        let round_tripped_b = WitnessEnvelope::reconstruct(envelope_b.to_packet()).unwrap();
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
            value: 42,
        }]));
        let input_envelope = WitnessEnvelope::reconstruct(input_packet).unwrap();
        let input_round_tripped = WitnessEnvelope::reconstruct(input_envelope.to_packet()).unwrap();
        assert_eq!(input_envelope, input_round_tripped);
    }

    /// FR-070-AC-4 (TC-183): reconstruction refuses when any one required
    /// O-25 member is absent, with no defaulted value substituted.
    #[trace("TC-183", "FR-070-AC-4")]
    #[test]
    fn tc_183_refuses_reconstruction_when_any_o25_member_is_missing() {
        let mut missing_backend = full_packet(0);
        missing_backend.backend = None;
        assert!(matches!(
            WitnessEnvelope::reconstruct(missing_backend),
            Err(WitnessRefusal::MissingMember("backend"))
        ));

        let mut missing_trace_position = full_packet(0);
        missing_trace_position.trace_position = None;
        assert!(matches!(
            WitnessEnvelope::reconstruct(missing_trace_position),
            Err(WitnessRefusal::MissingMember("trace_position"))
        ));

        let mut missing_source_digest = full_packet(0);
        missing_source_digest.source_digests = None;
        assert!(matches!(
            WitnessEnvelope::reconstruct(missing_source_digest),
            Err(WitnessRefusal::MissingMember("source_digests"))
        ));

        let mut missing_obligation = full_packet(0);
        missing_obligation.obligation_identity = None;
        assert!(matches!(
            WitnessEnvelope::reconstruct(missing_obligation),
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
        let envelope = WitnessEnvelope::reconstruct(packet).unwrap();
        assert_eq!(envelope.family_payload().bound_index, 3);
        let round_tripped = WitnessEnvelope::reconstruct(envelope.to_packet()).unwrap();
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
            "rev-1".to_owned(),
            Some("quire.not-a-real-domain/v1".to_owned()),
            "ab".repeat(32),
        )]);
        assert!(matches!(
            WitnessEnvelope::reconstruct(packet),
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
            "rev-1".to_owned(),
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            "ab".repeat(31), // 62 hex chars, not 64
        )]);
        assert!(matches!(
            WitnessEnvelope::reconstruct(packet),
            Err(WitnessRefusal::MalformedDigest("source_digests", _))
        ));
    }

    /// QSL-227 positive control: a real witness whose `backend` digest is in
    /// the required `quire.tool-manifest.jcs/v1` domain (as `full_packet`
    /// already builds it) reconstructs, and the resulting envelope's backend
    /// carries that domain.
    #[test]
    fn backend_digest_in_the_required_domain_reconstructs() {
        let envelope = WitnessEnvelope::reconstruct(full_packet(0)).unwrap();
        assert_eq!(
            envelope.backend().manifest_digest().record().domain(),
            DigestDomain::ToolManifestJcsV1
        );
    }

    /// QSL-227 (ADR-013 C-27): a `backend` digest in a recognized FR-201
    /// domain other than `quire.tool-manifest.jcs/v1` refuses with the same
    /// typed cause the reader already uses for a source-digest domain
    /// mismatch (`tc_181_refuses_an_out_of_domain_digest`), and the domain is
    /// checked before the digest bytes: pairing the wrong domain with
    /// malformed hex still reports the domain mismatch, not a hex-encoding
    /// problem.
    #[test]
    fn backend_digest_in_any_other_fr201_domain_refuses() {
        let mut packet = full_packet(0);
        packet.backend = Some((
            "kani-backend-1".to_owned(),
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            DigestRecord::mint(DigestDomain::SourceBytesV1, digest(7)).hex(),
        ));
        assert!(matches!(
            WitnessEnvelope::reconstruct(packet),
            Err(WitnessRefusal::DigestDomainMismatch("backend", _))
        ));

        let mut packet_with_bad_hex = full_packet(0);
        packet_with_bad_hex.backend = Some((
            "kani-backend-1".to_owned(),
            Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
            "not-hex".to_owned(),
        ));
        assert!(matches!(
            WitnessEnvelope::reconstruct(packet_with_bad_hex),
            Err(WitnessRefusal::DigestDomainMismatch("backend", _))
        ));
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
        packet.package_contract_version = Some("x".repeat(MAX_ENCODED_BYTES + 1));
        assert!(matches!(
            WitnessEnvelope::reconstruct(packet),
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
                package_contract_version: self.package_contract_version,
                source_digests: self.source_digests,
                profile_selections: self.profile_selections,
                proof_bounds: self.proof_bounds,
                declared_domains: self.declared_domains,
                backend: self.backend,
                trace_position: self.trace_position,
                source: self.source,
                family_payload: None,
            }
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-154: lift a spec-bundle to Semantic IR 2.0.0 bytes, admit them as a
//! domain package selection, and read the admitted document's IR nodes into
//! [`super::domain_package::DomainPackageRecord`]s.
//!
//! Three parts. [`lift_document`] wraps
//! `agent_ix_extraction_frontend::lift::lift`, the sole entry point that
//! turns a spec bundle plus its module roots into IR 2.0.0 document bytes
//! (FCD FR-091..097). [`admit`] runs FR-154's own four-check admission
//! table (`model-complete.md:58`) over a selection and a byte map, and
//! returns the package parsed once as a [`PackageDocument`], the only place
//! intake parses package bytes. [`read_records`] is the per-node reader:
//! it dispatches every `types[]`/`populations[]` entry purely by the
//! `meaning` its `kind` resolves to in the document's own `constructs[]`
//! table (FR-208), never by `kind.name`/`kind.module` directly.
//!
//! **This reader binds only what QSpec admits and refuses everything else.**
//! It never invents a value and never drops data. Two shapes FCD emits
//! today are still FCD gaps: bare-string core kinds (`"scalar"`/`"alias"`/...)
//! declared directly in `types[]`, and FCD's own identity form
//! (`ix://<pkg>/type/<id>`, `ix://<pkg>/field/<Owner>-<name>`) rather than
//! QSpec's (`ix://<pkg>/<id>`, `<owner>/<name>`). This reader refuses both
//! rather than working around them. A type's inline `relationships[]`
//! (model-complete.md's Relationships row: each end names an object type or
//! a process, carries a role and a multiplicity, and `direction` is the full
//! source-to-target/target-to-source/bidirectional/undirected vocabulary)
//! was a third gap -- FCD's old `verb`/`category`/`composite` shape carried
//! neither a role nor that direction vocabulary -- fixed upstream by FCD
//! #199/#200's `sourceEnd`/`targetEnd`/`role`/`direction` shape, so this
//! reader now reads it rather than refusing it. A non-empty operation
//! `frame` was a fourth gap, fixed by this reader's own FR-103
//! `resolve_pending_frames`/`resolve_frame_array`, which classifies each
//! `modifies`/`creates`/`deletes` entry into an [`OperationEffect`] rather
//! than refusing the frame outright.
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{BTreeMap, HashMap};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use agent_ix_extraction_frontend::lift::{lift, LiftOutcome, LiftRequest};
use agent_ix_extraction_frontend::{Diagnostic, Refusal};
use serde_json::Value;

use crate::model::domain_package::{
    AllocationRecord, ComponentRecord, DomainPackageRecord, DomainPackageRef, EndpointRecord,
    Extent, FieldMemberRecord, Multiplicity, NativeValueType, ObjectTypeRecord, OperationEffect,
    OperationMemberRecord, OperationParameterRecord, OperationResult, PopulationRecord,
    PortDirection, RecordValueTypeRecord, RelationshipDirection, RelationshipEnd,
    RelationshipRecord, ScalarTypeRecord, ValueTypeRef,
};
use crate::model::key::{hex, raw_bytes_digest, DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
use crate::model::refusal::{Inexact, IntakeLimit};
use qsl_foundation::diagnostic::{Code, JsonPointer};
use qsl_foundation::source::{LocatedSpan, Position};
use qsl_foundation::IntakeLimits;
use quire_canonical::Node;
use quire_exact::Presence;

mod unit;
pub use unit::{
    admit_unit, admit_unit_with_cancel, package_input, SelectedModel, UnitIntakeCause,
    UnitIntakeRefusal,
};

/// A Quire meaning id (FR-208): the sole legitimate way to determine what a
/// construct or type definition IS. Kind names and modules are never
/// matched directly; every reader dispatches on this table's values.
pub mod meaning {
    /// A domain object type: a `TypeDefinition` declaring an entity with
    /// fields, operations, relationships and an identity, read into an
    /// [`super::super::domain_package::ObjectTypeRecord`].
    pub const OBJECT_TYPE: &str = "quire.meaning.model.object-type/v1";
    /// A domain value type: a `TypeDefinition` naming a scalar bound to one
    /// of QSL's own native value types.
    pub const VALUE_TYPE: &str = "quire.meaning.model.value-type/v1";
    /// A record value type: a `TypeDefinition` whose fields carry no
    /// identity of their own (model-complete.md's grouped "Record value
    /// type, event type, ..." row) -- a known FR-208 meaning with no QSL
    /// record shape yet, refused as `unsupported_construct`.
    pub const RECORD_VALUE_TYPE: &str = "quire.meaning.model.record-value-type/v1";
    /// A variant type: a `TypeDefinition` declaring one `enum` with one
    /// `variant` case per member.
    pub const VARIANT_TYPE: &str = "quire.meaning.model.variant-type/v1";
    /// A domain event type: a `TypeDefinition` declaring the shape of one
    /// event -- a known FR-208 meaning with no QSL record shape yet,
    /// refused as `unsupported_construct`.
    pub const EVENT_TYPE: &str = "quire.meaning.model.event-type/v1";
    /// A state machine: a `TypeDefinition` declaring a type's states and
    /// transitions -- a known FR-208 meaning with no QSL record shape yet,
    /// refused as `unsupported_construct`.
    pub const STATE_MACHINE: &str = "quire.meaning.model.state-machine/v1";
    /// A process: a `TypeDefinition` declaring a behavioural, non-object
    /// participant -- a known FR-208 meaning with no QSL record shape yet,
    /// refused as `unsupported_construct`.
    pub const PROCESS: &str = "quire.meaning.model.process/v1";
    /// A persistence interface: a `TypeDefinition` declaring a storage
    /// contract for a domain type -- a known FR-208 meaning with no QSL
    /// record shape yet, refused as `unsupported_construct`.
    pub const PERSISTENCE_INTERFACE: &str = "quire.meaning.model.persistence-interface/v1";
    /// A namespace: a `TypeDefinition` declaring a naming scope rather than
    /// an instantiable type -- a known FR-208 meaning with no QSL record
    /// shape yet, refused as `unsupported_construct`.
    pub const NAMESPACE: &str = "quire.meaning.model.namespace/v1";
    /// A population declaration: a `populations[]` node naming the member
    /// types and the object-closure extent a domain package instantiates,
    /// read into a [`super::super::domain_package::PopulationRecord`].
    pub const POPULATION: &str = "quire.meaning.model.population/v1";
    /// FR-152's Part kind: a structural component instance owned by a
    /// composite type, read into a [`super::super::domain_package::ComponentRecord`].
    pub const SYSTEMS_PART: &str = "quire.meaning.systems.part/v1";
    /// FR-152's Port kind: a component's typed, directional interaction
    /// point, read into an [`super::super::domain_package::EndpointRecord`].
    pub const SYSTEMS_PORT: &str = "quire.meaning.systems.port/v1";
    /// FR-152's Interface kind: an object type carrying a `featureOrder`,
    /// read into an [`super::super::domain_package::ObjectTypeRecord`] with its
    /// interface features populated.
    pub const SYSTEMS_INTERFACE: &str = "quire.meaning.systems.interface/v1";
    /// FR-152's Connection kind: a wiring between two ports, read into a
    /// [`super::super::domain_package::RelationshipRecord`].
    pub const SYSTEMS_CONNECTION: &str = "quire.meaning.systems.connection/v1";
    /// FR-152's Allocation kind: an assignment of a Part, Port or operation
    /// to a Part, read into an [`super::super::domain_package::AllocationRecord`].
    pub const SYSTEMS_ALLOCATION: &str = "quire.meaning.systems.allocation/v1";

    /// Every meaning id FR-208 declares, as this reader knows it. A type's
    /// resolved meaning outside this closed list is not a real FR-208
    /// meaning at all (`invalid_model_binding`/`malformed-declaration`); one
    /// inside it but not covered by `read_type_node`'s dispatch is
    /// instead a known-but-unsupported declaration form
    /// (`unsupported_construct`/`declaration-form`) -- two different
    /// refusal causes for two different defects.
    pub const ALL: &[&str] = &[
        OBJECT_TYPE,
        VALUE_TYPE,
        RECORD_VALUE_TYPE,
        VARIANT_TYPE,
        EVENT_TYPE,
        STATE_MACHINE,
        PROCESS,
        PERSISTENCE_INTERFACE,
        NAMESPACE,
        POPULATION,
        SYSTEMS_PART,
        SYSTEMS_PORT,
        SYSTEMS_INTERFACE,
        SYSTEMS_CONNECTION,
        SYSTEMS_ALLOCATION,
    ];
}

/// A `typeRef` naming one of QSL's own native value types rather than a
/// node of any package: `ix://quire/native/<Name>`, declaring no node
/// (shared-grammar.md's `type-ref` production, ~:285-292). `<Name>` is
/// [`super::domain_package::NativeValueType`]'s own closed set -- the
/// unparameterized value-type keywords the grammar reserves (`Boolean`,
/// `Integer`, `Rational`, `Decimal`, `Float32`, `Float64`, `Text`). A
/// parameterized or generic form (`Int[..]`, `Decimal[..]`, `Reference<T>`,
/// a collection type) is never a bare native reference: FR-151's bounded
/// scalars (`model.Count`/`model.Small`) are package-declared
/// [`super::domain_package::ScalarTypeRecord`]s, and a relationship's own
/// end is read elsewhere, not through a field `typeRef`.
pub mod native {
    /// The prefix a native value type reference carries.
    pub const PREFIX: &str = "ix://quire/native/";

    /// The bare domain-package identity [`PREFIX`] is built from (ADR-010
    /// OBS-006, ADR-013 O-03): the reserved pseudo-package no domain package
    /// selection may name. [`super::admit`] refuses a selection naming it
    /// outright, before FR-154's own four-check admission table runs,
    /// because a package that *did* declare this identity would mint node
    /// identities of the exact `ix://quire/native/<Name>` shape
    /// `read_value_type_ref` always reads as a native reference
    /// first -- so any of that package's own declarations could never be
    /// reached through [`super::super::domain_package::ValueTypeRef::Package`],
    /// and it would silently share the native key space rather than merely
    /// refusing to resolve.
    ///
    /// Kept as its own constant rather than re-deriving it from [`PREFIX`]
    /// by string surgery at each use site, and pinned against `PREFIX` by
    /// this module's own test so the two cannot drift.
    pub const RESERVED_IDENTITY: &str = "quire/native";

    #[cfg(test)]
    mod tests {
        use super::*;

        /// (#213 S-2) [`PREFIX`] and [`RESERVED_IDENTITY`] name the same
        /// pseudo-package; this pins them together so an edit to one alone
        /// cannot leave the other stale.
        #[test]
        fn reserved_identity_matches_native_prefix() {
            assert_eq!(PREFIX, format!("ix://{RESERVED_IDENTITY}/"));
        }
    }
}

/// FCD `agent_ix_extraction_frontend::lift` did not produce a document.
///
/// Wraps FCD's own outcome types rather than stringifying them (matching
/// `crate::command::extraction::ExtractionError`'s `Context(Vec<SemanticFailure>)`):
/// the caller's diagnostics stay FCD-owned and typed.
#[derive(Debug, thiserror::Error)]
pub enum LiftFailure {
    /// A scratch directory for `lift`'s required output path could not be
    /// created.
    #[error("could not create a scratch directory for lift output: {0}")]
    Scratch(#[source] std::io::Error),
    /// FCD refused the bundle outright (parse, manifest or module failure).
    #[error("bundle refused: {0}")]
    Refused(Refusal),
    /// FCD produced at least one blocking diagnostic (schema or rules
    /// failure); no document was written.
    #[error("{} blocking diagnostic(s)", .0.len())]
    Blocked(Vec<Diagnostic>),
}

/// Lift one spec bundle to Semantic IR 2.0.0 document bytes.
///
/// `lift` (FCD FR-099) always writes its document to a required output
/// path; the diagnostics sidecar is skipped (`None`).
/// The scratch directory is removed when this function returns.
pub fn lift_document(bundle_root: &Path, module_roots: &[PathBuf]) -> Result<Vec<u8>, LiftFailure> {
    let scratch = tempfile::tempdir().map_err(LiftFailure::Scratch)?;
    let request = LiftRequest {
        bundle_root: bundle_root.to_path_buf(),
        module_roots: module_roots.to_vec(),
        out: scratch.path().join("document.json"),
        diagnostics: None,
    };
    match lift(&request) {
        LiftOutcome::Refused(refusal) => Err(LiftFailure::Refused(refusal)),
        LiftOutcome::Blocked { diagnostics } => Err(LiftFailure::Blocked(diagnostics)),
        LiftOutcome::Written { document, .. } => Ok(document),
    }
}

/// One domain-package document, parsed exactly once.
///
/// [`PackageDocument::parse`] is the only place intake turns package bytes
/// into a tree. It reads the bytes once, through `quire-canonical`'s shared
/// reader (ADR-030 D-4.6), and derives from that one tree both the
/// `agent-ix-semantic-ir` `Json` that `validate_with_semantic_ir` must be
/// handed (its `decide` accepts only that crate's own `Json`) and this
/// crate's `serde_json::Value` view. [`admit`]'s JCS digest encodes the
/// reader's tree itself, and [`read_records`]'s per-node reader reads the
/// derived view, so the three can never disagree about what the document is.
///
/// The reader refuses malformed input with its byte offset: bytes that are
/// not UTF-8, a byte order mark, a lone UTF-16 surrogate escape (RFC 8785,
/// via RFC 7493, admits none), a repeated member name, and every other
/// departure from the JSON grammar.
/// Each refuses as `invalid_model_binding`/`malformed-declaration` at the
/// document root `$`, carrying that offset (FR-260 B4). That refusal is this
/// parse's own: FR-154 admission ([`admit`]) and [`package_input`] digest
/// bytes the reader refuses raw (FR-056). A number with no finite double
/// (`1e400`) is not among them: it refuses `noncanonical_wire` below.
///
/// A number with no exact RFC 8785 spelling (`inexactness`) refuses
/// `noncanonical_wire`, carrying the RFC 6901 `document_pointer` of the
/// first such number in document order, before any digest is taken. The
/// exception is a number with no finite double: the reader refuses it when it
/// reaches it, before any tree exists, so it is named ahead of an earlier
/// inexact number, and ahead of a repeated member name the reader would
/// detect only when that object closes. When the bytes carry several reader
/// faults, the first one `quire_canonical::read` returns decides.
/// The cause is `inexact-integer` for a whole value beyond ±2^53 and
/// `inexact-number` for a value that is not its nearest double's shortest round-trip text. RFC 8785
/// writes that text in its place, so the digest of such a document would be
/// the digest of a different value, shared with every document differing
/// from it only in that number. [`admit`] returns this refusal where it
/// returns a limit refusal, before check 3 (FR-056).
///
/// Every number that remains is exactly the value of its double's RFC 8785
/// text: the derived view keeps an integer as `serde_json` reads it, exact
/// within ±2^53, and takes a non-whole number's correctly rounded double
/// from `quire_canonical`, the double the digest spells (`serde_json`'s own
/// float parse can land one unit in the last place away from it).
///
/// The caller's `IntakeLimits::input_bytes` refuses as
/// [`ModelRefusalCause::IntakeLimitExceeded`] naming the limit, never as a
/// malformed document. Nesting depth is no limit (ADR-030): the reader, the
/// `serde_json` view and their drops all run without recursion. A read
/// or digest that cannot reserve memory refuses as
/// [`ModelRefusalCause::AllocationFailed`] (FR-259 B6).
///
/// The `sha256-jcs` digest is taken here, once, by `quire-canonical` (ADR-013
/// §2, ADR-013:113: the one RFC 8785 implementation) under
/// `digest_limits`, and [`admit`]'s check 3 compares it.
pub struct PackageDocument {
    /// `{"ir": <document>}`: the bundle `agent_ix_semantic_ir::decide` reads
    /// (`input-bundle.schema.json` requires an `ir` member).
    bundle: agent_ix_semantic_ir::json::Json,
    /// The same document as a `serde_json::Value`.
    tree: Value,
    /// SHA-256 over the document's RFC 8785 bytes: the package's
    /// `sha256-jcs` digest.
    jcs_digest: [u8; 32],
}

/// The digest only: `Value` and `Json` format by recursion, once per level,
/// so a derived `Debug` would overflow the stack on a deep document.
impl std::fmt::Debug for PackageDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackageDocument")
            .field("jcs_digest", &self.jcs_digest)
            .finish_non_exhaustive()
    }
}

impl Drop for PackageDocument {
    /// Drops the `serde_json` tree without recursing: `Value`'s own drop uses
    /// one native frame per level.
    fn drop(&mut self) {
        quire_canonical::drop_value(std::mem::take(&mut self.tree));
    }
}

impl PackageDocument {
    /// Parses package document bytes once.
    ///
    /// Input over the caller's `input_bytes` limit refuses
    /// `resource_exhausted`/`intake-limit-exceeded` naming the limit and its
    /// bound. A read or digest that cannot reserve memory refuses
    /// `resource_exhausted`/`allocation-failed` carrying the bytes requested.
    /// Bytes the shared reader refuses as malformed refuse
    /// `invalid_model_binding`/`malformed-declaration` against the document
    /// root `$`, carrying the reader's byte offset. A document holding a
    /// number with no exact RFC 8785 spelling refuses `noncanonical_wire`
    /// (`inexact-integer` or `inexact-number`) at that number's pointer,
    /// before the digest is taken.
    pub fn parse(bytes: &[u8], limits: IntakeLimits) -> Result<Self, ModelRefusal> {
        let max_input_bytes = limits.input_bytes;
        // `usize` is at most 64 bits on every target Rust supports: lossless.
        let length = bytes.len() as u64;
        if length > max_input_bytes {
            return Err(input_bytes_exceeded(max_input_bytes, length));
        }
        let document = quire_canonical::read(bytes, max_input_bytes)
            .map_err(|error| read_refusal(error, max_input_bytes, length))?;
        if let Some((document_pointer, inexact, lexeme)) = first_inexact_number(document.root()) {
            return Err(noncanonical_number(document_pointer, inexact, lexeme));
        }
        let jcs_digest = *quire_canonical::sha256(&document, digest_limits(max_input_bytes))
            .map_err(|error| digest_refusal(error, max_input_bytes, length))?
            .as_bytes();
        let tree = value_of(document.root()).map_err(view_refusal)?;
        let ir = match json_of(document.root()) {
            Ok(json) => json,
            Err(fault) => {
                quire_canonical::drop_value(tree);
                return Err(view_refusal(match fault {
                    BuildFault::Scalar(never) => match never {},
                    BuildFault::Empty => BuildFault::Empty,
                }));
            }
        };
        Ok(Self {
            bundle: agent_ix_semantic_ir::json::Json::Object(vec![("ir".to_owned(), ir)]),
            tree,
            jcs_digest,
        })
    }

    /// The parsed document.
    pub fn tree(&self) -> &Value {
        &self.tree
    }

    /// The package's `sha256-jcs` digest: SHA-256 over the document's
    /// RFC 8785 bytes. The replay byte provision verifies a domain package
    /// entry against its declared digest with this (ADR-013 QC-1).
    pub fn jcs_digest(&self) -> [u8; 32] {
        self.jcs_digest
    }
}

/// How many times longer a document's RFC 8785 text can be than the document
/// itself. Only number spelling grows: `1e20` (4 bytes) canonicalizes to
/// `100000000000000000000` (21 bytes), 5.25 times as long, and no shorter
/// lexeme grows more (from `1e21` on the text is exponent form again).
/// Whitespace and escapes only shrink. Rounded up to 6.
const CANONICAL_GROWTH: u64 = 6;

/// The limits intake's `sha256-jcs` digest encodes under: canonical text up
/// to [`CANONICAL_GROWTH`] times the document's byte limit, a ceiling no
/// admitted document reaches.
fn digest_limits(input_bytes: u64) -> quire_canonical::Limits {
    quire_canonical::Limits::new(input_bytes.saturating_mul(CANONICAL_GROWTH))
}

/// FR-260 B5: a package document of `actual` bytes is over
/// `intake.input_bytes` at `bound`.
fn input_bytes_exceeded(bound: u64, actual: u64) -> ModelRefusal {
    limit_exceeded(IntakeLimit::InputBytes, bound, actual)
}

/// ADR-011 Limits: a package document reached `limit`, whose bound is
/// `bound` and whose measure reached `actual`.
fn limit_exceeded(limit: IntakeLimit, bound: u64, actual: u64) -> ModelRefusal {
    ModelRefusal {
        code: Code::ResourceExhausted,
        cause: ModelRefusalCause::IntakeLimitExceeded {
            limit,
            bound,
            actual,
        },
        detail: format!(
            "package document exceeds intake's {} limit of {bound}",
            limit.as_str()
        ),
    }
}

/// [`PackageDocument::parse`]'s refusal for a refusal of the shared reader.
fn read_refusal(error: quire_canonical::ReadError, bound: u64, actual: u64) -> ModelRefusal {
    match error {
        quire_canonical::ReadError::Malformed { offset, kind } => malformed_declaration(
            "$".to_owned(),
            None,
            None,
            format!("package document is malformed JSON at byte {offset}: {kind}"),
        ),
        quire_canonical::ReadError::Limit(_) => input_bytes_exceeded(bound, actual),
        quire_canonical::ReadError::Allocation { requested } => allocation_failed(requested),
        quire_canonical::ReadError::NumberOutOfRange {
            pointer, lexeme, ..
        } => match out_of_range_number(&pointer, &lexeme) {
            Some((document_pointer, inexact)) => {
                noncanonical_number(document_pointer, inexact, &lexeme)
            }
            None => malformed_declaration(
                "$".to_owned(),
                None,
                None,
                format!("package document number {lexeme} has no finite double"),
            ),
        },
        other => malformed_declaration(
            "$".to_owned(),
            None,
            None,
            format!("package document could not be read: {other}"),
        ),
    }
}

/// The pointer and cause of a number the shared reader refused for having no
/// finite double (FR-056): a whole value beyond ±2^53, exponent forms
/// included, is [`Inexact::Integer`]; any other is [`Inexact::Number`]. Decided
/// on the lexeme's digits alone, never through a double. `None` only when the
/// reader's `pointer` is not RFC 6901, which it never produces.
pub(super) fn out_of_range_number(pointer: &str, lexeme: &str) -> Option<(JsonPointer, Inexact)> {
    let pointer = pointer.parse().ok()?;
    let inexact = if Decimal::of(lexeme).is_whole_beyond_2_53() {
        Inexact::Integer
    } else {
        Inexact::Number
    };
    Some((pointer, inexact))
}

/// [`PackageDocument::parse`]'s refusal for an error of the `sha256-jcs`
/// digest's encoding.
fn digest_refusal(error: quire_canonical::Error, bound: u64, actual: u64) -> ModelRefusal {
    match error {
        quire_canonical::Error::Limit(_) => input_bytes_exceeded(bound, actual),
        quire_canonical::Error::Allocation { requested } => allocation_failed(requested),
        // A read tree always has an RFC 8785 encoding: every number is a
        // finite double and every member name a string.
        other => malformed_declaration(
            "$".to_owned(),
            None,
            None,
            format!("package document could not be RFC 8785-encoded: {other}"),
        ),
    }
}

/// FR-056: the package document's number `lexeme`, at `document_pointer`,
/// has no exact RFC 8785 spelling, for the reason `inexact` names.
fn noncanonical_number(
    document_pointer: JsonPointer,
    inexact: Inexact,
    lexeme: &str,
) -> ModelRefusal {
    let pointer = document_pointer.as_str();
    let detail = match inexact {
        Inexact::Integer => format!(
            "package document number {lexeme} at {pointer:?} denotes a whole value beyond \
             ±2^53, which RFC 8785 cannot encode exactly"
        ),
        Inexact::Number => format!(
            "package document number {lexeme} at {pointer:?} is not the value of its nearest \
             double's shortest round-trip text, which RFC 8785 encodes in its place"
        ),
    };
    ModelRefusal {
        code: Code::NoncanonicalWire,
        detail,
        cause: ModelRefusalCause::NoncanonicalNumber {
            inexact,
            document_pointer,
        },
    }
}

/// 2^53 in decimal: the largest magnitude up to which every whole number
/// has an exact IEEE 754 double, and so an exact RFC 8785 spelling.
const TWO_TO_THE_53: &[u8] = b"9007199254740992";

/// The exact value of a decimal number's text: `digits × 10^shift`, where
/// `digits` are its significant digits, with no leading and no trailing
/// zero. Read from an RFC 8259 number lexeme, or from the RFC 8785 spelling
/// of a double, in time linear in the text and with nothing
/// allocated. The exponent accumulates saturating at `u64::MAX`; a text is
/// far shorter than that, so a saturated exponent decides every comparison
/// as the exact one would.
struct Decimal<'t> {
    negative: bool,
    integer: &'t str,
    fraction: &'t str,
    /// Zeros before the first significant digit.
    leading: usize,
    /// The number of significant digits; zero for the value zero.
    significant: usize,
    shift: i128,
}

impl<'t> Decimal<'t> {
    #[qsl_attrs::string_edge]
    fn of(text: &'t str) -> Self {
        let (negative, unsigned) = match text.strip_prefix('-') {
            Some(unsigned) => (true, unsigned),
            None => (false, text),
        };
        let (mantissa, exponent) = unsigned.split_once(['e', 'E']).unwrap_or((unsigned, ""));
        let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        let digits = || integer.bytes().chain(fraction.bytes());
        let total = integer.len() + fraction.len();
        let leading = digits().take_while(|digit| *digit == b'0').count();
        let trailing = if leading == total {
            0
        } else {
            fraction
                .bytes()
                .rev()
                .chain(integer.bytes().rev())
                .take_while(|digit| *digit == b'0')
                .count()
        };
        let (exponent_negative, exponent) = match exponent.strip_prefix('-') {
            Some(magnitude) => (true, magnitude),
            None => (false, exponent.strip_prefix('+').unwrap_or(exponent)),
        };
        let magnitude = i128::from(exponent.bytes().fold(0u64, |value, digit| {
            value
                .saturating_mul(10)
                .saturating_add(u64::from(digit.saturating_sub(b'0')))
        }));
        let exponent = if exponent_negative {
            -magnitude
        } else {
            magnitude
        };
        Self {
            negative,
            integer,
            fraction,
            leading,
            significant: total - leading - trailing,
            shift: exponent - count(fraction.len()) + count(trailing),
        }
    }

    /// The significant digits, as ASCII bytes.
    fn digits(&self) -> impl Iterator<Item = u8> + '_ {
        self.integer
            .bytes()
            .chain(self.fraction.bytes())
            .skip(self.leading)
            .take(self.significant)
    }

    /// Whether the value is whole and its magnitude exceeds 2^53. Because
    /// the digits end in a non-zero digit, the value is whole exactly when
    /// `shift >= 0`, and then it has `significant + shift` digits.
    fn is_whole_beyond_2_53(&self) -> bool {
        if self.significant == 0 || self.shift < 0 {
            return false;
        }
        match (count(self.significant) + self.shift).cmp(&count(TWO_TO_THE_53.len())) {
            std::cmp::Ordering::Less => false,
            std::cmp::Ordering::Greater => true,
            // Equal lengths: the digits, then `shift` zeros, compared with
            // 2^53's digit by digit. `shift` is at most 16 here.
            std::cmp::Ordering::Equal => {
                let zeros = usize::try_from(self.shift).unwrap_or(usize::MAX);
                self.digits()
                    .chain(std::iter::repeat_n(b'0', zeros))
                    .cmp(TWO_TO_THE_53.iter().copied())
                    .is_gt()
            }
        }
    }

    /// Whether `self` and `other` denote the same exact value. Zero has no
    /// sign: `-0` and `0` are equal.
    fn same_value(&self, other: &Decimal<'_>) -> bool {
        if self.significant == 0 || other.significant == 0 {
            return self.significant == other.significant;
        }
        self.negative == other.negative
            && self.significant == other.significant
            && self.shift == other.shift
            && self.digits().eq(other.digits())
    }
}

/// `length`, a byte count of one text, as a wide signed integer.
fn count(length: usize) -> i128 {
    // A slice length is at most `isize::MAX`: lossless.
    i128::try_from(length).unwrap_or(i128::MAX)
}

/// Why `number` has no exact RFC 8785 spelling, or `None` when it has one
/// (FR-056). A whole value beyond ±2^53 is [`Inexact::Integer`], decided on
/// the text alone. Otherwise the number is exact when its text's value
/// equals the value of the shortest round-trip text of its nearest double,
/// the text RFC 8785 writes: `0.1`, `1.0`, `-0` and `5e-324` are exact,
/// `0.1000000000000000000001`, `9007199254740993.5` and the underflowing
/// `1e-400` are [`Inexact::Number`]. Both comparisons are exact decimal
/// reasoning over digits; no floating-point arithmetic decides either.
pub(super) fn inexactness(number: quire_canonical::Number<'_>) -> Option<Inexact> {
    let written = Decimal::of(number.text());
    if written.is_whole_beyond_2_53() {
        return Some(Inexact::Integer);
    }
    // The text quire-canonical writes for the double is RFC 8785's: the
    // shortest round-trip text, the closest of those and then the even digit
    // on a tie.
    let mut spelled = Spelled::default();
    let Ok(()) = quire_canonical::Writer::new(&mut spelled, quire_canonical::Limits::new(64))
        .number(number.value())
    else {
        return Some(Inexact::Number);
    };
    let exact = spelled
        .text()
        .is_some_and(|text| written.same_value(&Decimal::of(text)));
    (!exact).then_some(Inexact::Number)
}

/// A [`quire_canonical::Sink`] holding one number's RFC 8785 text: at most 25
/// bytes, so it needs no allocation.
#[derive(Default)]
struct Spelled {
    bytes: [u8; 32],
    length: usize,
}

impl Spelled {
    fn text(&self) -> Option<&str> {
        std::str::from_utf8(self.bytes.get(..self.length)?).ok()
    }
}

impl quire_canonical::Sink for Spelled {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), quire_canonical::Error> {
        let end = self.length + bytes.len();
        let room =
            self.bytes
                .get_mut(self.length..end)
                .ok_or(quire_canonical::Error::Internal {
                    invariant: "a number's RFC 8785 text fits 32 bytes",
                })?;
        room.copy_from_slice(bytes);
        self.length = end;
        Ok(())
    }
}

/// The RFC 6901 pointer, the reason and the lexeme of the first number of
/// `root`, in document order, that has no exact RFC 8785 spelling
/// ([`inexactness`]); `None` when every number has one. Walked over an
/// explicit heap stack, so a document of any depth is walked on any thread
/// stack.
pub(super) fn first_inexact_number(
    root: quire_canonical::NodeRef<'_>,
) -> Option<(JsonPointer, Inexact, &str)> {
    /// How a value is reached from the container holding it.
    enum Step<'d> {
        Member(&'d str),
        Element(usize),
    }
    /// The values of one open container not yet walked.
    enum Children<'d> {
        Items(std::iter::Enumerate<quire_canonical::Items<'d>>),
        Members(quire_canonical::Members<'d>),
    }
    // Each open container, with the step that reached it (`None` for the
    // root).
    let mut open: Vec<(Option<Step<'_>>, Children<'_>)> = Vec::new();
    let mut next = Some((None, root));
    loop {
        if let Some((step, value)) = next.take() {
            match value.node() {
                Node::Number(number) => {
                    if let Some(inexact) = inexactness(number) {
                        let steps = open.iter().filter_map(|(step, _)| step.as_ref());
                        let pointer = steps.chain(step.as_ref()).fold(
                            JsonPointer::root(),
                            |pointer, step| match step {
                                Step::Member(name) => pointer.key(name),
                                Step::Element(index) => pointer.key(&index.to_string()),
                            },
                        );
                        return Some((pointer, inexact, number.text()));
                    }
                }
                Node::Array(items) => open.push((step, Children::Items(items.enumerate()))),
                Node::Object(members) => open.push((step, Children::Members(members))),
                Node::Null | Node::Bool(_) | Node::String(_) => {}
            }
        }
        let (_, children) = open.last_mut()?;
        let child = match children {
            Children::Items(items) => items
                .next()
                .map(|(index, item)| (Step::Element(index), item)),
            Children::Members(members) => members
                .next()
                .map(|(name, value)| (Step::Member(name), value)),
        };
        match child {
            Some((step, value)) => next = Some((Some(step), value)),
            None => {
                open.pop();
            }
        }
    }
}

/// FR-259 B6: reading or digesting a package document could not reserve
/// `requested` bytes of memory.
fn allocation_failed(requested: usize) -> ModelRefusal {
    ModelRefusal {
        code: Code::ResourceExhausted,
        cause: ModelRefusalCause::AllocationFailed { requested },
        detail: format!(
            "reading the package document could not reserve {requested} bytes of memory"
        ),
    }
}

/// A value that holds no other value.
enum Scalar<'d> {
    Null,
    Bool(bool),
    Number(quire_canonical::Number<'d>),
    String(&'d str),
}

/// Why [`build`] returned no tree.
#[derive(Debug, PartialEq, Eq)]
enum BuildFault<E> {
    /// A scalar's conversion failed.
    Scalar(E),
    /// The walk finished with no value for the root: a reader tree always
    /// has one, so this is a bug, reported rather than guessed past.
    Empty,
}

/// How [`Builder`] closes one node at its exit.
enum Shape {
    /// A scalar: its value was pushed at enter.
    Leaf,
    /// An array of this many children.
    Array(usize),
    /// An object with these member names, in document order.
    Object(Vec<String>),
}

/// [`build`]'s walk (ADR-030): a node's exit closes it from the values its
/// children's exits left on `results`, so a tree of any depth builds on a
/// constant native stack.
struct Builder<'d, T, E, S, A, O> {
    scalar: S,
    array: A,
    object: O,
    results: Vec<T>,
    nodes: std::marker::PhantomData<(&'d (), E)>,
}

impl<T, E, S, A, O> Builder<'_, T, E, S, A, O> {
    /// The last `count` finished values, in document order.
    fn take(&mut self, count: usize) -> Vec<T> {
        let from = self.results.len().saturating_sub(count);
        self.results.split_off(from)
    }
}

impl<'d, T, E, S, A, O> quire_walk::Walk for Builder<'d, T, E, S, A, O>
where
    S: FnMut(Scalar<'d>) -> Result<T, E>,
    A: Fn(Vec<T>) -> T,
    O: Fn(Vec<(String, T)>) -> T,
{
    type Node = quire_canonical::NodeRef<'d>;
    type Frame = Shape;
    type Stop = E;

    fn enter(
        &mut self,
        node: quire_canonical::NodeRef<'d>,
        children: &mut quire_walk::Children<'_, quire_canonical::NodeRef<'d>>,
    ) -> ControlFlow<E, Shape> {
        let leaf = match node.node() {
            Node::Array(items) => {
                let count = items.len();
                children.extend(items);
                return ControlFlow::Continue(Shape::Array(count));
            }
            Node::Object(members) => {
                let mut names = Vec::with_capacity(members.len());
                for (name, member) in members {
                    names.push(name.to_owned());
                    children.push(member);
                }
                return ControlFlow::Continue(Shape::Object(names));
            }
            Node::Null => Scalar::Null,
            Node::Bool(value) => Scalar::Bool(value),
            Node::Number(number) => Scalar::Number(number),
            Node::String(text) => Scalar::String(text),
        };
        match (self.scalar)(leaf) {
            Ok(value) => {
                self.results.push(value);
                ControlFlow::Continue(Shape::Leaf)
            }
            Err(error) => ControlFlow::Break(error),
        }
    }

    fn exit(&mut self, shape: Shape) -> ControlFlow<E> {
        let closed = match shape {
            Shape::Leaf => return ControlFlow::Continue(()),
            Shape::Array(count) => {
                let items = self.take(count);
                (self.array)(items)
            }
            Shape::Object(names) => {
                let values = self.take(names.len());
                (self.object)(names.into_iter().zip(values).collect())
            }
        };
        self.results.push(closed);
        ControlFlow::Continue(())
    }
}

/// `root` as a tree of `T`, built by a [`Builder`] walk. `scalar` makes a `T`
/// from a null, boolean, number or string; `array` and `object` close a
/// container from its finished children, in document order. The first
/// `scalar` error ends the build, after `discard` has taken every finished
/// value the walk holds.
fn build<'d, T, E>(
    root: quire_canonical::NodeRef<'d>,
    scalar: impl FnMut(Scalar<'d>) -> Result<T, E>,
    array: impl Fn(Vec<T>) -> T,
    object: impl Fn(Vec<(String, T)>) -> T,
    discard: impl Fn(T),
) -> Result<T, BuildFault<E>> {
    let mut builder = Builder {
        scalar,
        array,
        object,
        results: Vec::new(),
        nodes: std::marker::PhantomData,
    };
    match quire_walk::walk(&mut builder, root) {
        ControlFlow::Continue(()) => builder.results.pop().ok_or(BuildFault::Empty),
        ControlFlow::Break(error) => {
            builder.results.into_iter().for_each(discard);
            Err(BuildFault::Scalar(error))
        }
    }
}

/// `node` as `agent-ix-semantic-ir`'s `Json`, each number by its lexeme.
fn json_of(
    node: quire_canonical::NodeRef<'_>,
) -> Result<agent_ix_semantic_ir::json::Json, BuildFault<std::convert::Infallible>> {
    use agent_ix_semantic_ir::json::Json;
    build(
        node,
        |leaf| {
            Ok(match leaf {
                Scalar::Null => Json::Null,
                Scalar::Bool(value) => Json::Bool(value),
                Scalar::Number(number) => Json::Number(number.text().to_owned()),
                Scalar::String(text) => Json::Str(text.to_owned()),
            })
        },
        Json::Array,
        Json::Object,
        drop,
    )
}

/// `node` as a `serde_json::Value`, or the first number lexeme `serde_json`
/// cannot represent. The tree drops through [`quire_canonical::drop_value`]:
/// `Value`'s own drop recurses once per level.
fn value_of(node: quire_canonical::NodeRef<'_>) -> Result<Value, BuildFault<&str>> {
    build(
        node,
        |leaf| {
            Ok(match leaf {
                Scalar::Null => Value::Null,
                Scalar::Bool(value) => Value::Bool(value),
                Scalar::Number(number) => {
                    let lexeme = number.text();
                    let parsed: serde_json::Number = lexeme.parse().map_err(|_| lexeme)?;
                    // serde_json's float parser can land one unit in the last
                    // place off; the digest encodes the correctly rounded
                    // double, so the tree takes that same double.
                    Value::Number(if parsed.is_f64() {
                        serde_json::Number::from_f64(number.value()).ok_or(lexeme)?
                    } else {
                        parsed
                    })
                }
                Scalar::String(text) => Value::String(text.to_owned()),
            })
        },
        Value::Array,
        |members| Value::Object(members.into_iter().collect()),
        quire_canonical::drop_value,
    )
}

/// The refusal for a view [`build`] could not make.
fn view_refusal(fault: BuildFault<&str>) -> ModelRefusal {
    let detail = match fault {
        BuildFault::Scalar(lexeme) => {
            format!("package document number {lexeme} has no serde_json representation")
        }
        BuildFault::Empty => "package document read to a tree with no root".to_owned(),
    };
    malformed_declaration("$".to_owned(), None, None, detail)
}

/// FR-154 Intake's four-check admission table (`model-complete.md:58-70`).
///
/// Runs the checks in table order and stops at the first failure: digest
/// domain, then byte presence, then digest equality
/// (`check_package_digest`), then the package's own declared
/// identity/version.
///
/// The package bytes are parsed once, here, into the [`PackageDocument`]
/// this returns alongside the admitted selection; [`read_records`] reads
/// that same tree rather than parsing the bytes again.
///
/// `offered` carries `{identity, version, digest}` -- [`DomainPackageRef`]'s
/// own flat FR-321 shape (its `digest_domain` is implicitly `sha256-jcs`).
/// `digest_domain` is taken as a separate string, not assumed to already be
/// `sha256-jcs`: check 1 of the table below is exactly the question of
/// whether the caller's offered digest domain is the one Intake accepts, so
/// a parameter that already assumed the answer could not state that check.
#[qsl_attrs::string_edge]
pub fn admit(
    offered: &DomainPackageRef,
    digest_domain: &str,
    bytes_by_digest: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: IntakeLimits,
) -> Result<(DomainPackageRef, PackageDocument), ModelRefusal> {
    // ADR-010 OBS-006 / ADR-013 O-03: the reserved `quire/native`
    // pseudo-package never selects, regardless of what its bytes would
    // otherwise admit -- checked first, ahead of FR-154's own four-check
    // table, because this rejects the offered identity itself rather than
    // anything about the package's bytes.
    if offered.identity == native::RESERVED_IDENTITY {
        return Err(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::ReservedPackageIdentity {
                selection: offered.clone(),
            },
            detail: format!(
                "domain package selection names identity {:?}, the reserved native-reference \
                 pseudo-package -- a native value type resolves only as ValueTypeRef::Native \
                 and never shares a real package's key space",
                offered.identity
            ),
        });
    }
    if digest_domain != SHA256_JCS_DIGEST_DOMAIN {
        return Err(ModelRefusal {
            code: Code::StaleDependency,
            cause: ModelRefusalCause::DigestDomainMismatch {
                expected: SHA256_JCS_DIGEST_DOMAIN,
                actual: digest_domain.to_owned(),
            },
            detail: format!(
                "domain package selection {}@{} names digest domain {digest_domain:?}, not {SHA256_JCS_DIGEST_DOMAIN:?}",
                offered.identity, offered.version
            ),
        });
    }
    let package_ref = offered.clone();
    let Some(bytes) = bytes_by_digest.get(&offered.digest) else {
        return Err(ModelRefusal {
            code: Code::MissingImport,
            cause: ModelRefusalCause::MissingSelection {
                selection: package_ref.clone(),
            },
            detail: format!(
                "no package bytes supplied under digest {}",
                hex(&offered.digest)
            ),
        });
    };
    // A document over one of the parse limits, one whose read could not
    // reserve memory, and one holding a number with no exact RFC 8785
    // spelling have no JCS digest check 3 can compare: each refuses naming
    // the limit (ADR-011 Limits), the failed reservation (FR-259 B6) or the
    // number's pointer (FR-056, IR FR-038-AC-93), never as a digest
    // mismatch. Any other parse failure is not itself a refusal here: bytes
    // the shared reader refuses are digested raw by check 3 (FR-056), and
    // supply no identity/version to check 4.
    let document = match PackageDocument::parse(bytes, limits) {
        Ok(document) => Some(document),
        Err(
            refusal @ ModelRefusal {
                cause:
                    ModelRefusalCause::IntakeLimitExceeded { .. }
                    | ModelRefusalCause::AllocationFailed { .. }
                    | ModelRefusalCause::NoncanonicalNumber { .. },
                ..
            },
        ) => return Err(refusal),
        Err(_) => None,
    };
    check_package_digest(
        offered.digest,
        bytes,
        document.as_ref().map(|document| document.jcs_digest),
    )
    .map_err(|mismatch| match mismatch {
        DigestMismatch::Content { recomputed } => ModelRefusal {
            code: Code::StaleDependency,
            cause: ModelRefusalCause::ContentMismatch {
                selected: offered.digest,
                recomputed,
            },
            detail: format!(
                "domain package {}@{} recomputes to digest {}, not the selected {}",
                offered.identity,
                offered.version,
                hex(&recomputed),
                hex(&offered.digest)
            ),
        },
        DigestMismatch::RawBytes { digest } => ModelRefusal {
            code: Code::StaleDependency,
            cause: ModelRefusalCause::ByteDigestMismatch {
                expected: offered.digest,
                actual: digest,
            },
            detail: format!(
                "domain package {}@{} bytes hash to {}, not the selected {}",
                offered.identity,
                offered.version,
                hex(&digest),
                hex(&offered.digest)
            ),
        },
    })?;
    // The package's own declared identity/version, read defensively: bytes
    // that fail to parse or omit `package` simply supply no identity/version,
    // which check 4 below reports as a `wrong-model-selection` mismatch
    // rather than a separate malformed-package case FR-154's table does not
    // name.
    let package = document
        .as_ref()
        .and_then(|document| document.tree.get("package"));
    let declared = |member: &str| {
        package
            .and_then(|package| package.get(member))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let actual_identity = declared("identity");
    let actual_version = declared("version");
    match document {
        Some(document)
            if actual_identity == offered.identity && actual_version == offered.version =>
        {
            Ok((package_ref, document))
        }
        _ => Err(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::WrongModelSelection {
                selection: package_ref,
                actual_identity: actual_identity.clone(),
                actual_version: actual_version.clone(),
            },
            detail: format!(
                "domain package selection names {}@{} but the package declares {actual_identity}@{actual_version}",
                offered.identity, offered.version
            ),
        }),
    }
}

/// FR-154 Intake check 3 (`model-complete.md:69`): the canonical digest of
/// the supplied document equals the selected digest, else
/// `stale_dependency`/`content-mismatch`; bytes that did not parse compare
/// their raw digest and refuse `stale_dependency`/`byte-digest-mismatch`.
///
/// When the bytes parsed, the digest is taken over this crate's own RFC 8785
/// JCS canonicalisation of the parsed tree, not the raw bytes verbatim, so
/// two byte-different-but-JCS-equivalent encodings of the same document
/// (a whitespace variant, say) digest identically. Bytes that did not parse
/// have no JCS form; the digest is then taken over the raw bytes, which a
/// producer that actually selected this digest could never have done for
/// unparseable bytes either, so this still reports the real mismatch rather
/// than fabricating a match.
///
/// The parsed document's digest is the one [`PackageDocument::parse`] took
/// through `quire-canonical` (ADR-013 §2, ADR-013:113).
///
/// Generalized to serve a second caller, `model::observation`'s
/// own FR-106 admission: `parsed_digest` is the caller's own already-taken
/// `sha256-jcs` digest (through `quire-canonical` directly) when its bytes
/// parsed, or `None` when they did not -- this function never parses
/// `bytes` itself and never names `serde_json`, so it stays the one place
/// `raw_bytes_digest`'s non-parsing fallback is called for either caller
/// (ADR-013 §2's one-encoder rule, which exempts this
/// function): `Err`'s digest is the mismatch to report, `Ok(())` a match.
/// A caller constructs its own refusal from `Err`'s digest, since intake's
/// [`ModelRefusal`] and FR-106's `AdmissionFailure` are different types.
pub(super) fn check_package_digest(
    expected: [u8; 32],
    bytes: &[u8],
    parsed_digest: Option<[u8; 32]>,
) -> Result<(), DigestMismatch> {
    let (actual_digest, mismatch): (_, fn([u8; 32]) -> DigestMismatch) = match parsed_digest {
        Some(digest) => (digest, |recomputed| DigestMismatch::Content { recomputed }),
        None => (raw_bytes_digest(bytes), |digest| DigestMismatch::RawBytes {
            digest,
        }),
    };
    if actual_digest == expected {
        Ok(())
    } else {
        Err(mismatch(actual_digest))
    }
}

/// Why [`check_package_digest`] refused, naming the digest to report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DigestMismatch {
    /// The supplied document parsed and its canonical `sha256-jcs` digest
    /// differs from the selected one (`content-mismatch`).
    Content {
        /// The digest recomputed from the supplied document.
        recomputed: [u8; 32],
    },
    /// The supplied bytes did not parse, so their raw digest was compared
    /// and differs from the selected one (`byte-digest-mismatch`).
    RawBytes {
        /// The raw bytes' own digest.
        digest: [u8; 32],
    },
}

/// ADR-013 O-01: admits every selection in `offered`, in order, through
/// [`admit`], and additionally refuses a second selection naming a
/// domain-package `identity` already admitted earlier in this same call --
/// "a package selects at most one version of a domain-package identity"
/// (QC-5, catalogued `duplicate_selection`/`duplicate-identity`, revision
/// `1-draft.6`) -- whether or not the repeated selection's version matches
/// the one already admitted. Checked before [`admit`]'s own byte-level work
/// for the repeated item, since the defect is in the selection list itself,
/// not in that item's bytes. No production caller offers more than one
/// domain package yet; this is the single-selection rule #131 was asked to
/// wire together with [`admit`] and did not (ADR-013 O-01's own "Implementing
/// ticket" row).
pub fn admit_selections(
    offered: &[DomainPackageRef],
    digest_domain: &str,
    bytes_by_digest: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: IntakeLimits,
) -> Result<Vec<(DomainPackageRef, PackageDocument)>, ModelRefusal> {
    admit_located(
        offered,
        |selection| selection,
        digest_domain,
        bytes_by_digest,
        limits,
    )
    .map(|admitted| {
        admitted
            .into_iter()
            .map(|(_, admitted_ref, document)| (admitted_ref, document))
            .collect()
    })
    .map_err(|(_, refusal)| refusal)
}

/// One item [`admit_located`] admitted: the item, its admitted selection
/// and its package document.
pub(crate) type Admitted<'a, T> = (&'a T, DomainPackageRef, PackageDocument);

/// [`admit_selections`] over any `offered` items, each naming its
/// selection through `selection`: an admitted item comes back with its
/// package document, and a refusal names the item it concerns.
pub(crate) fn admit_located<'a, T>(
    offered: &'a [T],
    selection: impl Fn(&T) -> &DomainPackageRef,
    digest_domain: &str,
    bytes_by_digest: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: IntakeLimits,
) -> Result<Vec<Admitted<'a, T>>, (&'a T, ModelRefusal)> {
    let mut admitted = Vec::with_capacity(offered.len());
    let mut selected_versions: BTreeMap<&str, &str> = BTreeMap::new();
    for item in offered {
        let offered_ref = selection(item);
        if let Some(&already_selected_version) =
            selected_versions.get(offered_ref.identity.as_str())
        {
            return Err((
                item,
                ModelRefusal {
                    code: Code::DuplicateSelection,
                    cause: ModelRefusalCause::DuplicateSelection {
                        identity: offered_ref.identity.clone(),
                        already_selected_version: already_selected_version.to_owned(),
                        requested_version: offered_ref.version.clone(),
                    },
                    detail: format!(
                        "domain package identity {:?} is already selected at version {:?}; this \
                         call additionally selects it at version {:?}, and a package selects at \
                         most one version of a domain-package identity",
                        offered_ref.identity, already_selected_version, offered_ref.version
                    ),
                },
            ));
        }
        let (admitted_ref, document) = admit(offered_ref, digest_domain, bytes_by_digest, limits)
            .map_err(|refusal| (item, refusal))?;
        selected_versions.insert(&offered_ref.identity, &offered_ref.version);
        admitted.push((item, admitted_ref, document));
    }
    Ok(admitted)
}

// ---------------------------------------------------------------------------
// Declaration identity form (model-complete.md's Identity row; FCD #199 gap
// 2, "the identity form"): a type-definition node's identity is exactly
// `ix://<package identity>/<artifact id>` (one segment past the package); a
// member node's identity is exactly `<owner identity>/<member name>`. An
// artifact id or member name is `^[A-Za-z][A-Za-z0-9_]*$`.
// ---------------------------------------------------------------------------

fn is_artifact_segment(text: &str) -> bool {
    let mut chars = text.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The single segment past `ix://<package_identity>/`, when `identity` has
/// exactly that form.
#[qsl_attrs::string_edge]
pub(crate) fn type_identity_segment<'a>(
    package_identity: &str,
    identity: &'a str,
) -> Option<&'a str> {
    let rest = identity
        .strip_prefix("ix://")?
        .strip_prefix(package_identity)?;
    let rest = rest.strip_prefix('/')?;
    (!rest.is_empty() && !rest.contains('/') && is_artifact_segment(rest)).then_some(rest)
}

/// The member name past `<owner_identity>/`, when `identity` has exactly
/// that form.
pub(crate) fn member_identity_name<'a>(owner_identity: &str, identity: &'a str) -> Option<&'a str> {
    let rest = identity.strip_prefix(owner_identity)?.strip_prefix('/')?;
    (!rest.is_empty() && !rest.contains('/') && is_artifact_segment(rest)).then_some(rest)
}

// ---------------------------------------------------------------------------
// Per-node reading context: builds FR-154 malformed-declaration/
// declaration-form refusals carrying the node's own identity, source
// artifact and span (`model-complete.md`:74-83), read from the node's own
// `identity`/`origin` -- never a caller-supplied string that could drift
// from what the node itself declares.
// ---------------------------------------------------------------------------

/// The node's own identity, exactly as the wire supplied it, or (when the
/// wire supplies none readable) the dotted path to the node itself -- still
/// a deterministic, human-locatable label, never a fabricated identity.
fn node_identity_label(value: &Value, at: &str) -> String {
    value
        .get("identity")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_else(|| at.to_owned())
}

/// The node's `origin.source.sourceIdentity`/`startLine`/`startColumn`,
/// mapped onto [`qsl_foundation::source::LocatedSpan`]'s own `{start, end}` shape.
/// FCD's wire (matching its `extraction-frontend::diagnostics::Locus`)
/// carries a start position only -- no byte offset and no end position --
/// so both ends of the mapped span are that same point and its byte offset
/// is `0`, a placeholder QSL does not treat as meaningful. `(None, None)`
/// when the node's origin is `generated`, or is itself absent or not the
/// expected shape.
fn node_span(value: &Value) -> (Option<String>, Option<LocatedSpan>) {
    let Some(source) = value.get("origin").and_then(|origin| origin.get("source")) else {
        return (None, None);
    };
    let artifact = source
        .get("sourceIdentity")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let span = match (
        source.get("startLine").and_then(Value::as_u64),
        source.get("startColumn").and_then(Value::as_u64),
    ) {
        (Some(start_line), Some(start_column)) => {
            let position = Position {
                byte: 0,
                line: start_line as usize,
                column: start_column as usize,
            };
            Some(LocatedSpan {
                start: position,
                end: position,
            })
        }
        _ => None,
    };
    (artifact, span)
}

/// Validates a parsed [`PackageDocument`] against `agent-ix-semantic-ir`'s
/// own independent reader before [`read_records`] walks a single node. That
/// crate decides `conformance/schema/input-bundle.schema.json` by wrapping
/// the document as `{"ir": <document>}` (`input-bundle.schema.json` requires
/// an `ir` member carrying `semantic-ir.schema.json`) and reports every
/// defect as an RFC 6901 pointer into the wrapped bundle. FR-056-AC-2/FR-056
/// Admission: "a reader-refused document retains every reader diagnostic
/// and admits no declaration" -- so this refuses `malformed-declaration`
/// once per `Severity::Error` diagnostic the reader reports, not only the
/// first, each carrying that diagnostic's own owning declaration as the
/// node (`Located::owner`, the same identity [`agent_ix_semantic_ir::diag::owner_for`]
/// computes) and, when its `locus` names one, the artifact id and span
/// ([`located_span`]), so a shape this reader's own hand-rolled checks miss
/// (an `abstract` that is not a boolean, say) still refuses -- fully, not
/// just its first defect -- rather than being read as a default or having
/// every diagnostic but the first silently dropped (PR #200 review R2-2).
///
/// `agent-ix-semantic-ir` at the pinned rev (`Cargo.toml`, FCD PR #200)
/// resolves `ix://quire/native/<Name>` at the validator layer
/// (`crates/semantic-ir/src/rules.rs`), against FCD's own closed
/// native-scalar set (`UUID`, `Boolean`, `Integer`, `Decimal`, `String`,
/// `Timestamp`, `Duration`, `Bytes`, `JsonObject`), so
/// `validate_with_semantic_ir` no longer refuses a document merely for
/// naming a native type reference (a rev predating FCD #199/#200 read every
/// such `typeRef` as `agent-ix.semantic-ir.UNRESOLVED_TYPE_REF`, an
/// `Error`-severity diagnostic this validator refused like any other; that
/// is no longer observable at this pin). That does not widen what *this*
/// reader accepts: [`read_value_type_ref`] independently refuses any
/// `<Name>` outside [`NativeValueType`]'s own closed set (QSpec's
/// `type-ref`, R5's ruling -- narrower than FCD's) as
/// `malformed-declaration`, a later, separate refusal from this one --
/// which is exactly why `Pump.id`/`Sys.id`/`Tank.id` still refuse in
/// `tests/model_intake.rs`'s golden-shape test even though the schema
/// itself now admits them.
fn validate_with_semantic_ir(document: &PackageDocument) -> Result<(), Vec<ModelRefusal>> {
    let verdict = agent_ix_semantic_ir::decide(&document.bundle);
    let refusals: Vec<ModelRefusal> = verdict
        .diagnostics
        .iter()
        .filter(|located| located.severity == agent_ix_semantic_ir::diag::Severity::Error)
        // FR-103: a `modifies`/`creates`/`deletes` frame entry's
        // resolution is QSL's own, strictly finer classification
        // (`resolve_pending_frames`/`resolve_frame_array`), not this
        // validator's own coarser `UNRESOLVED_FRAME_PATH` rule (which
        // cannot distinguish "names no declaration at all" from "names a
        // declaration of the wrong kind", the distinction FR-103-AC-2
        // itself draws). Surfacing it here as a `malformed-declaration`
        // refusal would preempt `resolve_pending_frames` for every frame
        // entry it also flags, so it is filtered out and left entirely to
        // this reader's own per-node classification.
        .filter(|located| located.code != agent_ix_semantic_ir::constructs::UNRESOLVED_FRAME_PATH)
        .map(|located| {
            let node = if located.owner.is_empty() {
                "$".to_owned()
            } else {
                located.owner.clone()
            };
            let (artifact, span) = located_span(located);
            malformed_declaration(
                node,
                artifact,
                span,
                format!(
                    "agent-ix-semantic-ir refused this document at {} ({}): {}",
                    located.pointer, located.code, located.message
                ),
            )
        })
        .collect();
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals)
    }
}

/// The artifact id and span one `agent-ix-semantic-ir` diagnostic's own
/// `locus` carries. `Located::locus` is already the `origin.source` object
/// itself (see `agent_ix_semantic_ir::diag::locus_for`), not wrapped in an
/// `origin` member the way [`node_span`] must unwrap from a node of
/// [`PackageDocument::tree`] -- the two functions read the same
/// `sourceIdentity`/`startLine`/`startColumn` shape from the two views a
/// [`PackageDocument`] keeps of its one parse, one FCD's own `Json`, one this
/// crate's `serde_json::Value` derived from it.
fn located_span(
    located: &agent_ix_semantic_ir::diag::Located,
) -> (Option<String>, Option<LocatedSpan>) {
    let Some(source) = &located.locus else {
        return (None, None);
    };
    let artifact = source
        .get("sourceIdentity")
        .and_then(agent_ix_semantic_ir::json::Json::as_str)
        .map(str::to_owned);
    let span = match (
        source
            .get("startLine")
            .and_then(agent_ix_semantic_ir::json::Json::as_i64),
        source
            .get("startColumn")
            .and_then(agent_ix_semantic_ir::json::Json::as_i64),
    ) {
        (Some(start_line), Some(start_column)) if start_line >= 0 && start_column >= 0 => {
            let position = Position {
                byte: 0,
                line: start_line as usize,
                column: start_column as usize,
            };
            Some(LocatedSpan {
                start: position,
                end: position,
            })
        }
        _ => None,
    };
    (artifact, span)
}

/// FR-154's `invalid_model_binding`/`malformed-declaration` refusal: the one
/// constructor every intake malformed-declaration refusal goes through.
fn malformed_declaration(
    node: String,
    artifact: Option<String>,
    span: Option<LocatedSpan>,
    detail: String,
) -> ModelRefusal {
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: ModelRefusalCause::IntakeMalformedDeclaration {
            node,
            artifact,
            span,
        },
        detail,
    }
}

fn malformed_at(value: &Value, at: &str, reason: impl std::fmt::Display) -> ModelRefusal {
    let (artifact, span) = node_span(value);
    malformed_declaration(
        node_identity_label(value, at),
        artifact,
        span,
        format!("{at}: {reason}"),
    )
}

fn unsupported_at(value: &Value, at: &str, what: impl Into<String>) -> ModelRefusal {
    let node = node_identity_label(value, at);
    let what = what.into();
    ModelRefusal {
        code: Code::UnsupportedConstruct,
        cause: ModelRefusalCause::UnsupportedDeclarationForm {
            node,
            what: what.clone(),
        },
        detail: format!("{at}: construct meaning/capability {what:?} has no reader yet"),
    }
}

/// FR-103: `entry` (a `modifies`/`creates`/`deletes` frame entry) names no
/// declaration of the operation's own domain package. `node`/`artifact`/
/// `span` are the operation's own (`PendingFrame`'s own fields, read once
/// at intake).
fn missing_frame_declaration(
    node: String,
    artifact: Option<String>,
    span: Option<LocatedSpan>,
    entry: &str,
    at: &str,
) -> ModelRefusal {
    ModelRefusal {
        code: Code::MissingDeclaration,
        cause: ModelRefusalCause::FrameEntryMissing {
            node,
            artifact,
            span,
            entry: entry.to_owned(),
        },
        detail: format!("{at}: {entry:?} names no declaration of this domain package"),
    }
}

/// FR-103: `entry` (a `modifies`/`creates`/`deletes` frame entry) names a
/// relationship or a process -- QSpec's Frames row admits both, but no
/// FR-106 observation carries a relationship link or a process instance, so
/// a grant over either could never be enforced. See
/// [`missing_frame_declaration`] for `node`/`artifact`/`span`.
fn unsupported_frame_feature(
    node: String,
    artifact: Option<String>,
    span: Option<LocatedSpan>,
    entry: &str,
    at: &str,
) -> ModelRefusal {
    ModelRefusal {
        code: Code::UnknownRequiredFeature,
        cause: ModelRefusalCause::FrameEntryUnsupported {
            node,
            artifact,
            span,
            entry: entry.to_owned(),
        },
        detail: format!("{at}: {entry:?} names a relationship or a process, never grantable"),
    }
}

/// FR-103: `entry` (a `modifies`/`creates`/`deletes` frame entry) names a
/// declaration of this domain package, but of the wrong meaning for that
/// member (an object type for `modifies`, a field for `creates`/`deletes`,
/// and so on). See [`missing_frame_declaration`] for `node`/`artifact`/
/// `span`.
fn malformed_frame_entry(
    node: String,
    artifact: Option<String>,
    span: Option<LocatedSpan>,
    entry: &str,
    at: &str,
) -> ModelRefusal {
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: ModelRefusalCause::FrameEntryMalformed {
            node,
            artifact,
            span,
            entry: entry.to_owned(),
        },
        detail: format!("{at}: {entry:?} names a declaration of a different meaning"),
    }
}

/// A native value type whose required parameters
/// `agent-ix-semantic-ir`'s own constraint keyword vocabulary
/// (`min`/`max`/`exclusiveMin`/`exclusiveMax`/`minLength`/`maxLength`/
/// `pattern`/`enumValues`/`nonEmpty`/`unique`/`format`) cannot express, no
/// matter what a producer's own field-level constraints supply -- names
/// which parameters are missing.
fn unsupported_native_parameters(
    value: &Value,
    at: &str,
    native: NativeValueType,
    missing: &[&str],
) -> ModelRefusal {
    let node = node_identity_label(value, at);
    let what = format!("typeRef:ix://quire/native/{}", native.as_str());
    ModelRefusal {
        code: Code::UnsupportedConstruct,
        cause: ModelRefusalCause::UnsupportedDeclarationForm {
            node,
            what: what.clone(),
        },
        detail: format!(
            "{at}: typeRef: native value type {:?} requires parameter(s) {} that agent-ix-semantic-ir's constraint vocabulary cannot express",
            native.as_str(),
            missing.join(", ")
        ),
    }
}

/// One IR node under reading: its own JSON value and dotted document path.
/// Every read helper reports a malformed-declaration/declaration-form
/// refusal against `value`'s own `identity`/`origin`, so a nested check
/// (a field's `typeRef`, a multiplicity's `lower`) still names the owning
/// declaration, not an anonymous sub-object with neither.
struct NodeCtx<'a> {
    value: &'a Value,
    at: String,
}

impl<'a> NodeCtx<'a> {
    fn new(value: &'a Value, at: impl Into<String>) -> Self {
        Self {
            value,
            at: at.into(),
        }
    }

    fn malformed(&self, reason: impl std::fmt::Display) -> ModelRefusal {
        malformed_at(self.value, &self.at, reason)
    }

    fn str_field(&self, field: &'static str) -> Result<&'a str, ModelRefusal> {
        self.value
            .get(field)
            .and_then(Value::as_str)
            .ok_or_else(|| self.malformed(format!("{field}: missing or not a string")))
    }

    fn opt_str_field(&self, field: &str) -> Option<&'a str> {
        self.value.get(field).and_then(Value::as_str)
    }

    /// `value[field]` as an array, or `&[]` when the member is absent
    /// entirely (every array member the readers below touch is optional at
    /// the wire level, e.g. a business type's `fields`/`operations`, a
    /// field's `subsets`).
    fn array_field(&self, field: &'static str) -> Result<&'a [Value], ModelRefusal> {
        match self.value.get(field) {
            None => Ok(&[]),
            Some(array) => array
                .as_array()
                .map(Vec::as_slice)
                .ok_or_else(|| self.malformed(format!("{field}: not an array"))),
        }
    }

    /// `value[field]` as a list of identity strings. `agent-ix-semantic-ir`'s
    /// own `identity_list` validates every present item as an
    /// `ix://<owner>/<name>`-shaped string before [`validate_with_semantic_ir`]
    /// lets a document reach this reader; a non-string item still refuses
    /// here, so a dependency pin that stops guaranteeing it degrades to a
    /// refusal, not a panic.
    fn identity_keys(
        &self,
        package: &str,
        field: &'static str,
    ) -> Result<Vec<DeclarationKey>, ModelRefusal> {
        self.array_field(field)?
            .iter()
            .enumerate()
            .map(|(position, item)| {
                item.as_str()
                    .map(|node| declaration_key(package, node))
                    .ok_or_else(|| self.malformed(format!("{field}[{position}]: not a string")))
            })
            .collect()
    }

    /// The `{module, name}` pair of this node's `kind` object, the key a
    /// `constructs[]` entry is indexed by (FR-208).
    fn kind_key(&self) -> Result<(&'a str, &'a str), ModelRefusal> {
        let kind = self
            .value
            .get("kind")
            .ok_or_else(|| self.malformed("kind: missing"))?;
        let module = kind
            .get("module")
            .and_then(Value::as_str)
            .ok_or_else(|| self.malformed("kind.module: missing or not a string"))?;
        let name = kind
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| self.malformed("kind.name: missing or not a string"))?;
        Ok((module, name))
    }

    /// FR-154's own multiplicity shape (`model-complete.md`:167-169):
    /// `{lower, upper, ordered, unique}`. `lower` and `upper` were already
    /// read before this PR; `ordered` and `unique` are real wire members
    /// too (FCD's own `MULTIPLICITY_MEMBERS`), and QSpec admits no default
    /// for either -- a missing one refuses, rather than silently supplying
    /// FR-113's own field/end default the way this reader once did.
    fn multiplicity(&self, field: &'static str) -> Result<Multiplicity, ModelRefusal> {
        let object = self
            .value
            .get(field)
            .ok_or_else(|| self.malformed(format!("{field}: missing multiplicity")))?;
        let lower = object.get("lower").and_then(Value::as_u64).ok_or_else(|| {
            self.malformed(format!(
                "{field}.lower: missing or not a non-negative integer"
            ))
        })?;
        let upper = match object.get("upper") {
            None | Some(Value::Null) => None,
            Some(value) => Some(value.as_u64().ok_or_else(|| {
                self.malformed(format!("{field}.upper: not a non-negative integer"))
            })?),
        };
        let ordered = object
            .get("ordered")
            .and_then(Value::as_bool)
            .ok_or_else(|| self.malformed(format!("{field}.ordered: missing or not a boolean")))?;
        let unique = object
            .get("unique")
            .and_then(Value::as_bool)
            .ok_or_else(|| self.malformed(format!("{field}.unique: missing or not a boolean")))?;
        Ok(Multiplicity {
            lower,
            upper,
            ordered,
            unique,
        })
    }
}

fn declaration_key(package: &str, node: &str) -> DeclarationKey {
    DeclarationKey {
        package: package.to_owned(),
        node: node.to_owned(),
    }
}

/// Validates `identity`'s form against the Identity row and returns the
/// node's own [`DeclarationKey`] built from `at`'s own identity string --
/// never a substituted or normalized one.
fn read_type_identity(
    package: &str,
    ctx: &NodeCtx<'_>,
) -> Result<(&'static str, DeclarationKey), ModelRefusal> {
    let identity = ctx.str_field("identity")?;
    if type_identity_segment(package, identity).is_none() {
        return Err(ctx.malformed(format!(
            "identity: {identity:?} is not ix://{package}/<artifact id>"
        )));
    }
    Ok(("", declaration_key(package, identity)))
}

fn read_operation_parameter(
    package: &str,
    owner_identity: &str,
    param: &Value,
    at: &str,
) -> Result<OperationParameterRecord, ModelRefusal> {
    let ctx = NodeCtx::new(param, at);
    let node = ctx.str_field("identity")?;
    if member_identity_name(owner_identity, node).is_none() {
        return Err(ctx.malformed(format!("identity: {node:?} is not {owner_identity}/<name>")));
    }
    let type_ref = ctx.str_field("typeRef")?;
    let value_type = read_value_type_ref(package, &ctx, type_ref)?;
    let multiplicity = ctx.multiplicity("multiplicity")?;
    Ok(OperationParameterRecord {
        key: declaration_key(package, node),
        value_type,
        multiplicity,
    })
}

/// One operation's frame, read as raw identity strings only (FR-103): I1
/// resolves `modifies`/`creates`/`deletes` against the whole package's
/// declarations only after every node is read (`resolve_pending_frames`),
/// since an entry may name a sibling type's field or a type declared later
/// in the document (TC-458-AC-5: admission is order-independent).
struct PendingFrame {
    /// This operation's own declaration key, to write the resolved
    /// [`OperationEffect`] back onto its [`OperationMemberRecord`].
    operation: DeclarationKey,
    /// The operation's own identity label, artifact and span, for a
    /// refusal naming the operation (the same triple
    /// [`ModelRefusalCause::IntakeMalformedDeclaration`] carries).
    node: String,
    artifact: Option<String>,
    span: Option<LocatedSpan>,
    /// Each member's entries, in document order, with the JSON path to
    /// report against.
    modifies: Vec<(String, String)>,
    creates: Vec<(String, String)>,
    deletes: Vec<(String, String)>,
}

/// `frame[member]`'s entries as `(identity, at-path)` pairs, in document
/// order. `agent-ix-semantic-ir`'s own reader already validates this shape
/// (FR-103's own dependency note, `crates/semantic-ir/src/schema.rs`), but a
/// non-string entry still refuses here rather than panicking, the same
/// defensive posture every other reader in this module takes.
fn frame_entries(
    frame: &Value,
    frame_at: &str,
    member: &'static str,
) -> Result<Vec<(String, String)>, ModelRefusal> {
    let member_at = format!("{frame_at}.{member}");
    let member_ctx = NodeCtx::new(frame, member_at.clone());
    member_ctx
        .array_field(member)?
        .iter()
        .enumerate()
        .map(|(position, entry)| {
            let entry_at = format!("{member_at}[{position}]");
            entry
                .as_str()
                .map(|identity| (identity.to_owned(), entry_at.clone()))
                .ok_or_else(|| NodeCtx::new(entry, entry_at).malformed("not a string"))
        })
        .collect()
}

fn read_operation_member(
    package: &str,
    owner_identity: &str,
    operation: &Value,
    at: &str,
    pending: &mut Vec<PendingFrame>,
) -> Result<OperationMemberRecord, ModelRefusal> {
    let ctx = NodeCtx::new(operation, at);
    let node = ctx.str_field("identity")?;
    if member_identity_name(owner_identity, node).is_none() {
        return Err(ctx.malformed(format!("identity: {node:?} is not {owner_identity}/<name>")));
    }
    let params_at = format!("{at}.params");
    let parameters = ctx
        .array_field("params")?
        .iter()
        .enumerate()
        .map(|(position, param)| {
            read_operation_parameter(package, node, param, &format!("{params_at}[{position}]"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let result = match operation.get("returns") {
        None => None,
        Some(returns) => {
            let returns_ctx = NodeCtx::new(operation, at);
            let type_ref = returns
                .get("typeRef")
                .and_then(Value::as_str)
                .ok_or_else(|| returns_ctx.malformed("returns.typeRef: missing or not a string"))?;
            let value_type = read_value_type_ref(package, &returns_ctx, type_ref)?;
            let multiplicity = returns_ctx.multiplicity_of(returns, "returns.multiplicity")?;
            Some(OperationResult {
                value_type,
                multiplicity,
            })
        }
    };
    // `pre` is shape-checked as an array here; its clauses are FR-146
    // expressions, which `crate::model` does not parse (see
    // `domain_package::PostconditionClause`), so nothing is kept from it.
    ctx.array_field("pre")?;
    // FR-103: `frame` (model-complete.md's Frames row). An absent frame, or
    // one whose three members are all absent or empty, reads as the empty
    // effect; a frame that declares anything is queued for
    // `resolve_pending_frames` rather than resolved (or refused) here.
    if let Some(frame) = operation.get("frame") {
        let frame_at = format!("{at}.frame");
        let modifies = frame_entries(frame, &frame_at, "modifies")?;
        let creates = frame_entries(frame, &frame_at, "creates")?;
        let deletes = frame_entries(frame, &frame_at, "deletes")?;
        if !(modifies.is_empty() && creates.is_empty() && deletes.is_empty()) {
            let (artifact, span) = node_span(operation);
            pending.push(PendingFrame {
                operation: declaration_key(package, node),
                node: node_identity_label(operation, at),
                artifact,
                span,
                modifies,
                creates,
                deletes,
            });
        }
    }
    Ok(OperationMemberRecord {
        key: declaration_key(package, node),
        owner: declaration_key(package, owner_identity),
        parameters,
        result,
        // Resolved from `pending` by `resolve_pending_frames`, once every
        // node of the document is read; the empty effect until then.
        effect: OperationEffect::default(),
        // FR-146's expression parser is out of scope for `crate::model`
        // (`domain_package::PostconditionClause`'s own module docs); this
        // reader states none rather than inventing one.
        own_postcondition_clauses: Vec::new(),
        has_body: false,
        redefines: None,
    })
}

impl<'a> NodeCtx<'a> {
    /// [`Self::multiplicity`], reading from an arbitrary sub-value (e.g. an
    /// operation's `returns` object) rather than `self.value` directly,
    /// while still reporting against `self`'s own node identity/origin.
    fn multiplicity_of(&self, container: &Value, at: &str) -> Result<Multiplicity, ModelRefusal> {
        let object = container
            .get("multiplicity")
            .ok_or_else(|| self.malformed(format!("{at}: missing multiplicity")))?;
        let lower = object.get("lower").and_then(Value::as_u64).ok_or_else(|| {
            self.malformed(format!("{at}.lower: missing or not a non-negative integer"))
        })?;
        let upper = match object.get("upper") {
            None | Some(Value::Null) => None,
            Some(value) => Some(value.as_u64().ok_or_else(|| {
                self.malformed(format!("{at}.upper: not a non-negative integer"))
            })?),
        };
        let ordered = object
            .get("ordered")
            .and_then(Value::as_bool)
            .ok_or_else(|| self.malformed(format!("{at}.ordered: missing or not a boolean")))?;
        let unique = object
            .get("unique")
            .and_then(Value::as_bool)
            .ok_or_else(|| self.malformed(format!("{at}.unique: missing or not a boolean")))?;
        Ok(Multiplicity {
            lower,
            upper,
            ordered,
            unique,
        })
    }
}

/// Resolves a `typeRef` (field, parameter or `returns` alike -- FR-154's
/// Fields row, `model-complete.md`:157: "`typeRef` names a native value
/// type, or a type of the package"): the sole resolver every `typeRef` site
/// goes through. `ix://quire/native/<Name>` names one of
/// [`NativeValueType`]'s closed set and resolves to [`ValueTypeRef::Native`],
/// declaring no node of `package` -- refusing `malformed-declaration` for a
/// `<Name>` outside that set, and `unsupported_construct`/`declaration-form`
/// for a native type whose required parameters
/// [`unsupported_native_parameters`] names as unexpressable
/// (shared-grammar.md's `Rational`/`Decimal`/`Text` productions). Any other
/// `typeRef` must itself be `ix://<package>/<artifact id>`, a node of
/// `package` (the same Identity-row form `read_type_identity` already
/// enforces for a declaration's own `identity`), and resolves to
/// [`ValueTypeRef::Package`].
#[qsl_attrs::string_edge]
fn read_value_type_ref(
    package: &str,
    ctx: &NodeCtx<'_>,
    type_ref: &str,
) -> Result<ValueTypeRef, ModelRefusal> {
    if let Some(name) = type_ref.strip_prefix(native::PREFIX) {
        let native = name.parse::<NativeValueType>().map_err(|()| {
            ctx.malformed(format!(
                "typeRef: {type_ref:?} names no native value type QSL declares"
            ))
        })?;
        let missing = native.unsupported_parameters();
        if !missing.is_empty() {
            return Err(unsupported_native_parameters(
                ctx.value, &ctx.at, native, missing,
            ));
        }
        return Ok(ValueTypeRef::Native(native));
    }
    if type_identity_segment(package, type_ref).is_none() {
        return Err(ctx.malformed(format!(
            "typeRef: {type_ref:?} is not ix://quire/native/<Name> or ix://{package}/<artifact id>"
        )));
    }
    Ok(ValueTypeRef::Package(declaration_key(package, type_ref)))
}

#[qsl_attrs::string_edge]
fn read_field_member(
    package: &str,
    owner_identity: &str,
    field: &Value,
    at: &str,
) -> Result<FieldMemberRecord, ModelRefusal> {
    let ctx = NodeCtx::new(field, at);
    let node = ctx.str_field("identity")?;
    if member_identity_name(owner_identity, node).is_none() {
        return Err(ctx.malformed(format!("identity: {node:?} is not {owner_identity}/<name>")));
    }
    let type_ref = ctx.str_field("typeRef")?;
    let value_type = read_value_type_ref(package, &ctx, type_ref)?;
    let multiplicity = ctx.multiplicity("multiplicity")?;
    // QSpec's own Presence row (`model-complete.md`:158): `presence` is
    // exactly `required` or `optional`, independent of `multiplicity`'s
    // lower bound; any other value is `malformed-declaration` per the
    // Presence row, refusing rather than aborting, so a future FCD pin
    // widening this enum degrades to a refusal, not a crash.
    let presence = match ctx.str_field("presence")? {
        "required" => Presence::Required,
        "optional" => Presence::Optional,
        other => {
            return Err(ctx.malformed(format!(
                "presence: {other:?} is not required/optional, the only presences this reader recognizes"
            )))
        }
    };
    let subsets = ctx.identity_keys(package, "subsets")?;
    let redefines = ctx
        .opt_str_field("redefines")
        .map(|target| declaration_key(package, target));
    Ok(FieldMemberRecord {
        key: declaration_key(package, node),
        owner: declaration_key(package, owner_identity),
        value_type,
        multiplicity,
        presence,
        subsets,
        redefines,
    })
}

/// Reads an `OBJECT_TYPE`/`SYSTEMS_INTERFACE` type into an
/// [`ObjectTypeRecord`] plus one [`FieldMemberRecord`]/[`OperationMemberRecord`]
/// per declared field/operation and one [`RelationshipRecord`] per declared
/// inline relationship, appending all of them to `records` in the type's
/// own field-then-operation-then-relationship order.
fn read_object_type(
    package: &str,
    type_value: &Value,
    node: &str,
    at: &str,
    is_interface: bool,
    records: &mut Vec<DomainPackageRecord>,
    pending: &mut Vec<PendingFrame>,
) -> Result<(), ModelRefusal> {
    let ctx = NodeCtx::new(type_value, at);
    let supertypes = ctx.identity_keys(package, "supertypes")?;
    // `agent-ix-semantic-ir`'s own `expect_bool` guarantees `abstract`, when
    // present, is a boolean, before this reader sees the document; a
    // non-boolean still refuses here rather than panicking. Absent reads as
    // `false` (QSpec's own default for the property).
    let abstract_type = match type_value.get("abstract") {
        None => false,
        Some(value) => value
            .as_bool()
            .ok_or_else(|| ctx.malformed("abstract: not a boolean"))?,
    };
    let interface_features = if is_interface {
        Some(ctx.identity_keys(package, "featureOrder")?)
    } else {
        None
    };
    records.push(DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: declaration_key(package, node),
        interface_features,
        abstract_type,
        supertypes,
    }));
    let fields_at = format!("{at}.fields");
    for (position, field) in ctx.array_field("fields")?.iter().enumerate() {
        records.push(DomainPackageRecord::FieldMember(read_field_member(
            package,
            node,
            field,
            &format!("{fields_at}[{position}]"),
        )?));
    }
    let operations_at = format!("{at}.operations");
    for (position, operation) in ctx.array_field("operations")?.iter().enumerate() {
        records.push(DomainPackageRecord::OperationMember(read_operation_member(
            package,
            node,
            operation,
            &format!("{operations_at}[{position}]"),
            pending,
        )?));
    }
    // A type's own inline `relationships[]` is QSpec's Relationship
    // declaration kind (model-complete.md's Relationships row), read by
    // `read_relationship` -- FCD #199 gap 3 is fixed upstream, so this is
    // read rather than refused.
    let relationships_at = format!("{at}.relationships");
    for (position, relationship) in ctx.array_field("relationships")?.iter().enumerate() {
        records.push(DomainPackageRecord::Relationship(read_relationship(
            package,
            node,
            relationship,
            &format!("{relationships_at}[{position}]"),
        )?));
    }
    Ok(())
}

/// Reads a `RECORD_VALUE_TYPE` type (FR-208-AC-4) into a
/// [`RecordValueTypeRecord`] plus one [`FieldMemberRecord`] per declared
/// field, in declaration order.
///
/// A record value type has one or more fields and no identity field: an
/// empty `fields` or a non-empty `identityFields` refuses
/// `invalid_model_binding`/`malformed-declaration`. An operation refuses
/// `unsupported_construct`/`declaration-form` naming the operation. A
/// `supertypes` entry naming a type of another meaning refuses
/// `malformed-declaration` at that entry (FR-208-AC-12), one naming no type
/// of the document `dangling_reference`/`unknown-general`, and one naming a
/// record value type `declaration-form`: record-value-type generalization is
/// not read yet. An inline relationship refuses `malformed-declaration`,
/// since a relationship end must name an object type.
///
/// `type_meanings` maps each type identity of the document to the meaning
/// its `kind` resolves to.
fn read_record_value_type(
    package: &str,
    type_value: &Value,
    node: &str,
    at: &str,
    type_meanings: &HashMap<&str, &str>,
    records: &mut Vec<DomainPackageRecord>,
) -> Result<(), ModelRefusal> {
    let ctx = NodeCtx::new(type_value, at);
    if !ctx.array_field("identityFields")?.is_empty() {
        return Err(ctx.malformed("identityFields: a record value type has no identity field"));
    }
    if let Some(operation) = ctx.array_field("operations")?.first() {
        let operation_at = format!("{at}.operations[0]");
        return Err(unsupported_at(
            operation,
            &operation_at,
            format!("{}:operations", meaning::RECORD_VALUE_TYPE),
        ));
    }
    let supertypes = ctx.identity_keys(package, "supertypes")?;
    for (position, general) in supertypes.iter().enumerate() {
        match type_meanings.get(general.node.as_str()) {
            Some(&meaning::RECORD_VALUE_TYPE) => {}
            Some(other) => {
                return Err(NodeCtx::new(type_value, format!("{at}.supertypes[{position}]"))
                    .malformed(format!(
                        "{} resolves to {other:?}; a record value type generalizes only a record value type",
                        general.node
                    )))
            }
            None => {
                return Err(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::UnknownGeneral {
                        supertype: declaration_key(package, node),
                        general: general.clone(),
                    },
                    detail: format!(
                        "{at}.supertypes[{position}]: {} names no type of this domain package",
                        general.node
                    ),
                })
            }
        }
    }
    // Every entry names a record value type: a generalization this reader
    // does not read yet.
    if !supertypes.is_empty() {
        return Err(unsupported_at(
            type_value,
            at,
            format!("{}:supertypes", meaning::RECORD_VALUE_TYPE),
        ));
    }
    if !ctx.array_field("relationships")?.is_empty() {
        return Err(
            NodeCtx::new(type_value, format!("{at}.relationships[0]")).malformed(
                "a record value type owns no relationship; a relationship end names an object type",
            ),
        );
    }
    let fields = ctx.array_field("fields")?;
    if fields.is_empty() {
        return Err(ctx.malformed("fields: a record value type has one or more fields"));
    }
    records.push(DomainPackageRecord::RecordValueType(
        RecordValueTypeRecord {
            key: declaration_key(package, node),
        },
    ));
    let fields_at = format!("{at}.fields");
    for (position, field) in fields.iter().enumerate() {
        records.push(DomainPackageRecord::FieldMember(read_field_member(
            package,
            node,
            field,
            &format!("{fields_at}[{position}]"),
        )?));
    }
    Ok(())
}

/// Reads the plain-scalar shape of a `VALUE_TYPE` type (FR-056's
/// `value-type/v1` scalar reader) into a [`ScalarTypeRecord`]:
/// [`ScalarTypeRecord`]'s own narrow bound-integer slice, not a general
/// scalar type system. The pinned `agent-ix-semantic-ir` also binds
/// `value-type/v1` to a record-shaped form (a non-empty `fields`); that
/// shape names no scalar at all and is never routed here -- see
/// [`read_type_node`]'s own dispatch, which sends it to the generic
/// known-but-unsupported bucket instead.
///
/// The wire's own `scalar` member names the bound native scalar
/// (`agent-ix-semantic-ir`'s closed `SCALARS` set); only `"integer"`
/// resolves here, since [`ScalarTypeRecord`] bounds only an integer domain.
/// Its `constraints[]` carry the domain as `min`/`max` entries
/// (`agent-ix-semantic-ir`'s own closed constraint-keyword vocabulary), each
/// naming a numeric `operands.value` -- read structurally from those typed
/// fields, never by parsing a rendered `Int[lo,hi]` string.
///
/// **Unsupported vs. malformed.** A shape this reader's own narrow slice
/// cannot hold -- a scalar keyword other than `"integer"`, a constraint
/// keyword this integer slice has no case for, or a half-bounded domain
/// (only `min` or only `max`; both are itself schema-valid FCD wire, e.g. a
/// natural-number domain) -- refuses `unsupported_construct`: a real,
/// schema-valid form this reader does not implement yet, not a defect in
/// the document. A shape that is wrong regardless of what this reader
/// implements -- an `operations`/`relationships`/`supertypes` member on a
/// scalar, a constraint keyword given twice, a non-numeric bound, no bound
/// at all, or `lower` greater than `upper` -- refuses
/// `invalid_model_binding`/`malformed-declaration`. Members this reader has
/// no use for at all (`variants`, `clauses`, `abstract`, ...) are read by no
/// reader and left alone, same as for every other construct kind's own
/// irrelevant members -- only the members a value type could plausibly
/// carry structure through (`operations`/`relationships`/`supertypes`) are
/// checked so they are never silently dropped.
#[qsl_attrs::string_edge]
fn read_value_type(
    package: &str,
    type_value: &Value,
    node: &str,
    at: &str,
) -> Result<ScalarTypeRecord, ModelRefusal> {
    let ctx = NodeCtx::new(type_value, at);
    if let Some(operation) = ctx.array_field("operations")?.first() {
        let operation_at = format!("{at}.operations[0]");
        return Err(unsupported_at(
            operation,
            &operation_at,
            format!("{}:operations", meaning::VALUE_TYPE),
        ));
    }
    if let Some(relationship) = ctx.array_field("relationships")?.first() {
        return Err(NodeCtx::new(relationship, format!("{at}.relationships[0]"))
            .malformed("a value type owns no relationship"));
    }
    if let Some(supertype) = ctx.array_field("supertypes")?.first() {
        return Err(NodeCtx::new(supertype, format!("{at}.supertypes[0]"))
            .malformed("a value type declares no supertype"));
    }
    let scalar = ctx.str_field("scalar")?;
    if scalar != "integer" {
        return Err(unsupported_at(
            type_value,
            at,
            format!("{}:scalar={scalar}", meaning::VALUE_TYPE),
        ));
    }
    let constraints = ctx.array_field("constraints")?;
    let mut lower: Option<i128> = None;
    let mut upper: Option<i128> = None;
    for (position, constraint) in constraints.iter().enumerate() {
        let constraint_at = format!("{at}.constraints[{position}]");
        let constraint_ctx = NodeCtx::new(constraint, constraint_at.clone());
        let keyword = constraint_ctx.str_field("keyword")?;
        let slot = match keyword {
            "min" => &mut lower,
            "max" => &mut upper,
            other => {
                return Err(unsupported_at(
                    constraint,
                    &constraint_at,
                    format!("{}:constraints:{other}", meaning::VALUE_TYPE),
                ))
            }
        };
        if slot.is_some() {
            return Err(constraint_ctx.malformed(format!("keyword: {keyword:?} is declared twice")));
        }
        let operand = constraint
            .get("operands")
            .and_then(|operands| operands.get("value"))
            .ok_or_else(|| constraint_ctx.malformed("operands.value: missing or not an integer"))?;
        *slot = Some(
            bound_operand(operand, keyword).map_err(|reason| constraint_ctx.malformed(reason))?,
        );
    }
    let (lower, upper) = match (lower, upper) {
        (Some(lower), Some(upper)) => (lower, upper),
        (None, None) => {
            return Err(ctx.malformed(
                "constraints: an integer value type requires both a min and a max bound",
            ))
        }
        (lower, _upper) => {
            // Exactly one of `min`/`max` is present: a half-bounded integer
            // domain, schema-valid `agent-ix-semantic-ir` wire this reader's
            // own [`ScalarTypeRecord`] has no shape for (it bounds a closed
            // `[lower, upper]` interval only) -- a real form, not a defect.
            return Err(unsupported_at(
                type_value,
                at,
                format!(
                    "{}:constraints:half-bounded({})",
                    meaning::VALUE_TYPE,
                    if lower.is_some() { "min" } else { "max" }
                ),
            ));
        }
    };
    if lower > upper {
        return Err(ctx.malformed(format!(
            "constraints: lower bound {lower} is greater than upper bound {upper}"
        )));
    }
    Ok(ScalarTypeRecord {
        key: declaration_key(package, node),
        lower,
        upper,
    })
}

/// A scalar bound operand: a JSON integer (the read already refuses one
/// beyond +/-2^53 as inexact) or a canonical decimal string, either exact in
/// `i128::MIN..=i128::MAX` (FR-056). `Err` is the refusal's reason, naming
/// `keyword` and the written value.
fn bound_operand(operand: &Value, keyword: &str) -> Result<i128, String> {
    if let Some(number) = operand.as_i64() {
        return Ok(i128::from(number));
    }
    let Some(text) = operand.as_str() else {
        return Err("operands.value: missing or not an integer".to_owned());
    };
    // Canonical: no sign but `-`, no leading zero, no `-0`; so it prints
    // back as it was written.
    match text.parse::<i128>() {
        Ok(value) if value.to_string() == text => Ok(value),
        Ok(_) | Err(_) => Err(format!(
            "operands.value: {text:?} for keyword {keyword:?} is not a canonical decimal \
             integer in i128::MIN..=i128::MAX"
        )),
    }
}

fn read_component(
    package: &str,
    ctx: &NodeCtx<'_>,
    node: &str,
) -> Result<ComponentRecord, ModelRefusal> {
    // FR-103/SR-722 FND-008: FCD makes `operations` optional on every
    // construct kind (its own `Member::Operations` default), but this
    // reader (and `read_endpoint`/`read_connection`/`read_allocation`
    // alike) has no `OperationMemberRecord` shape for a systems part, so a
    // non-empty `operations` here would otherwise be silently dropped
    // (never queued, never resolved, never refused) once
    // `validate_with_semantic_ir` stops surfacing FCD's own
    // `UNRESOLVED_FRAME_PATH` diagnostic for an unresolved frame on it.
    // Refuse it explicitly instead, the same way [`read_record_value_type`]
    // already refuses an operation on a record value type.
    if let Some(operation) = ctx.array_field("operations")?.first() {
        let operation_at = format!("{}.operations[0]", ctx.at);
        return Err(unsupported_at(
            operation,
            &operation_at,
            format!("{}:operations", meaning::SYSTEMS_PART),
        ));
    }
    let owner = ctx.str_field("owner")?;
    let value_type = ctx.str_field("declaredType")?;
    let multiplicity = ctx.multiplicity("multiplicity")?;
    Ok(ComponentRecord {
        key: declaration_key(package, node),
        owning_type: declaration_key(package, owner),
        value_type: declaration_key(package, value_type),
        multiplicity,
        // FR-152's Part kind is exactly this meaning (`meaning::SYSTEMS_PART`);
        // there is no separate wire capability to read.
        has_part_signature: true,
    })
}

fn read_endpoint(
    package: &str,
    ctx: &NodeCtx<'_>,
    node: &str,
) -> Result<EndpointRecord, ModelRefusal> {
    // See [`read_component`]'s own comment (SR-722 FND-008).
    if let Some(operation) = ctx.array_field("operations")?.first() {
        let operation_at = format!("{}.operations[0]", ctx.at);
        return Err(unsupported_at(
            operation,
            &operation_at,
            format!("{}:operations", meaning::SYSTEMS_PORT),
        ));
    }
    let owner = ctx.str_field("owner")?;
    let value_type = ctx.str_field("interfaceType")?;
    let multiplicity = ctx.multiplicity("multiplicity")?;
    // `agent-ix-semantic-ir`'s own `expect_enum` guarantees `direction`,
    // when present, is one of `in`/`out`/`inout` before this reader sees
    // the document; presence itself is not FCD-guaranteed, so absent stays
    // `None` here. An unrecognized value still refuses rather than aborting
    // the process, so a future FCD pin widening this enum degrades to a
    // refusal, not a crash.
    let direction = match ctx.opt_str_field("direction") {
        Some("in") => Some(PortDirection::In),
        Some("out") => Some(PortDirection::Out),
        Some("inout") => Some(PortDirection::InOut),
        Some(other) => {
            return Err(ctx.malformed(format!(
                "direction: {other:?} is not a direction this reader recognizes"
            )))
        }
        None => None,
    };
    Ok(EndpointRecord {
        key: declaration_key(package, node),
        owning_component: declaration_key(package, owner),
        value_type: declaration_key(package, value_type),
        direction,
        multiplicity,
    })
}

#[qsl_attrs::string_edge]
fn read_connection(
    package: &str,
    ctx: &NodeCtx<'_>,
    node: &str,
) -> Result<RelationshipRecord, ModelRefusal> {
    // See [`read_component`]'s own comment (SR-722 FND-008).
    if let Some(operation) = ctx.array_field("operations")?.first() {
        let operation_at = format!("{}.operations[0]", ctx.at);
        return Err(unsupported_at(
            operation,
            &operation_at,
            format!("{}:operations", meaning::SYSTEMS_CONNECTION),
        ));
    }
    let source_end = ctx
        .value
        .get("sourceEnd")
        .ok_or_else(|| ctx.malformed("sourceEnd: missing"))?;
    let target_end = ctx
        .value
        .get("targetEnd")
        .ok_or_else(|| ctx.malformed("targetEnd: missing"))?;
    let source_type = source_end
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| ctx.malformed("sourceEnd.type: missing or not a string"))?;
    let target_type = target_end
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| ctx.malformed("targetEnd.type: missing or not a string"))?;
    let source_multiplicity = ctx.multiplicity_of(source_end, "sourceEnd.multiplicity")?;
    let target_multiplicity = ctx.multiplicity_of(target_end, "targetEnd.multiplicity")?;
    // `agent-ix-semantic-ir`'s own `expect_enum` guarantees `flowDirection`
    // is one of `source-to-target`/`target-to-source`/`bidirectional` when
    // present, and `str_field` above already refused its absence. An
    // unrecognized value still refuses rather than aborting the process, so
    // a future FCD pin widening this enum degrades to a refusal, not a
    // crash.
    let direction = match ctx.str_field("flowDirection")? {
        "source-to-target" => RelationshipDirection::SourceToTarget,
        "target-to-source" => RelationshipDirection::TargetToSource,
        "bidirectional" => RelationshipDirection::Bidirectional,
        other => {
            return Err(ctx.malformed(format!(
                "flowDirection: {other:?} is not a direction this reader recognizes"
            )))
        }
    };
    Ok(RelationshipRecord {
        key: declaration_key(package, node),
        source: RelationshipEnd {
            type_identity: declaration_key(package, source_type),
            // `ConnectionEnd` (FCD's wire shape for a Connection node's
            // ends) carries no `role` member at all -- not merely an absent
            // optional one -- so this end never has one to preserve.
            role: None,
            multiplicity: source_multiplicity,
        },
        target: RelationshipEnd {
            type_identity: declaration_key(package, target_type),
            role: None,
            multiplicity: target_multiplicity,
        },
        direction,
    })
}

/// Reads a type's own inline `relationships[]` entry (model-complete.md's
/// Relationships row: each end names an object type or a process, carries
/// a role and a multiplicity with `lower <= upper`, and `direction` is the
/// full `source-to-target`/`target-to-source`/`bidirectional`/`undirected`
/// vocabulary) into a [`RelationshipRecord`].
///
/// Distinct from [`read_connection`]: a Connection node's ends carry no
/// `role` at all, and its direction lives under `flowDirection`, restricted
/// to `source-to-target`/`target-to-source`/`bidirectional` -- never
/// `undirected`. This shape's own member is `direction`, not
/// `flowDirection`; the two are never conflated.
#[qsl_attrs::string_edge]
fn read_relationship(
    package: &str,
    owner_identity: &str,
    relationship: &Value,
    at: &str,
) -> Result<RelationshipRecord, ModelRefusal> {
    let ctx = NodeCtx::new(relationship, at.to_owned());
    let node = ctx.str_field("identity")?;
    // FR-056 Declarations: "a member's identity is its owner's identity,
    // `/` and the member name" -- exactly [`member_identity_name`]'s rule,
    // the same one [`read_field_member`]/[`read_operation_member`] apply to
    // their own owned members. A relationship member is no exception: FCD's
    // own `ix://<pkg>/relationship/<Owner>-<verb>-<Other>` identity form
    // (FCD #199 gap 2's sibling for relationships) does not parse as
    // `owner_identity/<name>`, so it refuses here rather than being taken
    // verbatim as this record's key (FR-056-AC-5: "a relationship member
    // with no declared name ... refuses `malformed-declaration`" -- this
    // reader derives a member's declared name from its identity's own
    // suffix, the same mechanism field/operation members use, so an
    // identity with no such suffix IS "no declared name").
    if member_identity_name(owner_identity, node).is_none() {
        return Err(ctx.malformed(format!("identity: {node:?} is not {owner_identity}/<name>")));
    }
    // FR-056-AC-5's other half: "a relationship member with no ... source
    // span refuses `malformed-declaration`".
    let (_, span) = node_span(relationship);
    if span.is_none() {
        return Err(ctx.malformed("origin.source: missing; a relationship member with no source span refuses (FR-056-AC-5)"));
    }
    let source = read_relationship_end(package, &ctx, "sourceEnd")?;
    let target = read_relationship_end(package, &ctx, "targetEnd")?;
    // `agent-ix-semantic-ir`'s own `expect_enum` guarantees `direction` is
    // one of the four `RelationshipDirection` variants when present, and
    // `str_field` above already refused its absence; an unrecognized value
    // still refuses rather than aborting the process, so a future FCD pin
    // widening this enum degrades to a refusal, not a crash.
    let direction = match ctx.str_field("direction")? {
        "source-to-target" => RelationshipDirection::SourceToTarget,
        "target-to-source" => RelationshipDirection::TargetToSource,
        "bidirectional" => RelationshipDirection::Bidirectional,
        "undirected" => RelationshipDirection::Undirected,
        other => {
            return Err(ctx.malformed(format!(
                "direction: {other:?} is not a direction this reader recognizes"
            )))
        }
    };
    Ok(RelationshipRecord {
        key: declaration_key(package, node),
        source,
        target,
        direction,
    })
}

/// One `sourceEnd`/`targetEnd` of a [`read_relationship`] node:
/// `{role, multiplicity, type}`. `role` is required on `sourceEnd` and
/// optional on `targetEnd` (`agent-ix-semantic-ir`'s own
/// `relationshipSourceEnd`/`relationshipTargetEnd` schemas), so it is read
/// as present-or-absent here rather than assumed on both ends.
fn read_relationship_end(
    package: &str,
    ctx: &NodeCtx<'_>,
    field: &'static str,
) -> Result<RelationshipEnd, ModelRefusal> {
    let end = ctx
        .value
        .get(field)
        .ok_or_else(|| ctx.malformed(format!("{field}: missing")))?;
    let type_ref = end
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| ctx.malformed(format!("{field}.type: missing or not a string")))?;
    let role = end.get("role").and_then(Value::as_str).map(str::to_owned);
    let multiplicity = ctx.multiplicity_of(end, &format!("{field}.multiplicity"))?;
    Ok(RelationshipEnd {
        type_identity: declaration_key(package, type_ref),
        role,
        multiplicity,
    })
}

fn read_allocation(
    package: &str,
    ctx: &NodeCtx<'_>,
    node: &str,
) -> Result<AllocationRecord, ModelRefusal> {
    // See [`read_component`]'s own comment (SR-722 FND-008).
    if let Some(operation) = ctx.array_field("operations")?.first() {
        let operation_at = format!("{}.operations[0]", ctx.at);
        return Err(unsupported_at(
            operation,
            &operation_at,
            format!("{}:operations", meaning::SYSTEMS_ALLOCATION),
        ));
    }
    // model-complete.md:335: an Allocation names a source element and a
    // target element, and nothing else -- no multiplicity, no direction.
    let source = ctx.str_field("sourceElement")?;
    let target = ctx.str_field("targetElement")?;
    Ok(AllocationRecord {
        key: declaration_key(package, node),
        source_element: declaration_key(package, source),
        target_element: declaration_key(package, target),
    })
}

#[qsl_attrs::string_edge]
fn read_population(
    package: &str,
    ctx: &NodeCtx<'_>,
    node: &str,
    meanings: &HashMap<(String, String), String>,
) -> Result<PopulationRecord, ModelRefusal> {
    // `agent-ix-semantic-ir`'s own `POPULATION_MEMBERS` requires `kind`
    // present with a shape `construct_kind` validates; `kind_key` still
    // refuses a shape that guarantee missed. The meaning resolution is QSL's
    // own: `kind` must resolve to POPULATION, not merely to some constructs
    // entry.
    let (module, name) = ctx.kind_key()?;
    match meanings.get(&(module.to_owned(), name.to_owned())) {
        Some(resolved) if resolved == meaning::POPULATION => {}
        Some(other) => {
            return Err(ctx.malformed(format!(
                "kind: resolves to {other:?}, not {:?}",
                meaning::POPULATION
            )))
        }
        None => return Err(ctx.malformed("kind: names no constructs[] entry")),
    }
    let member_types = ctx.identity_keys(package, "members")?;
    // A native value-type identity (`ix://quire/native/<Name>`) is a
    // well-formed `ix://<owner>/<name>` string to `agent-ix-semantic-ir`
    // (it has no concept of "native" at all), but QSpec's own population
    // Members row admits only object types or processes -- QSL's own,
    // stricter check.
    if let Some(native_member) = member_types
        .iter()
        .find(|member| member.node.starts_with(native::PREFIX))
    {
        return Err(ctx.malformed(format!(
            "members: {:?} names a native value type, not an object type or process",
            native_member.node
        )));
    }
    // `POPULATION_MEMBERS` requires `extent` present, and `expect_enum`
    // guarantees it is `closed` or `open` when present. An unrecognized
    // value still refuses rather than aborting the process, so a future FCD
    // pin widening this enum degrades to a refusal, not a crash.
    let extent = match ctx.str_field("extent")? {
        "closed" => Extent::Closed,
        "open" => Extent::Open,
        other => {
            return Err(ctx.malformed(format!(
                "extent: {other:?} is not closed/open, the only extents this reader recognizes"
            )))
        }
    };
    Ok(PopulationRecord {
        key: declaration_key(package, node),
        member_types,
        extent,
    })
}

/// The `{module, name} -> meaning` lookup a document's own `constructs[]`
/// table states (FR-208): the sole legitimate way a type's `kind` resolves
/// to what it IS.
fn meaning_index(document: &Value) -> Result<HashMap<(String, String), String>, ModelRefusal> {
    // `agent-ix-semantic-ir`'s own top-level `IR_MEMBERS` requires
    // `constructs` present, `CONSTRUCT_ENTRY_MEMBERS` requires each entry's
    // `kind`/`construct` present, `construct_kind` validates `kind.module`/
    // `kind.name` are strings, and `DECLARATION_REQUIRED` guarantees
    // `construct.meaning` is a string -- all before this reader sees the
    // document. Each is still checked here, so a dependency pin that stops
    // guaranteeing one refuses the document rather than panicking.
    let constructs = document
        .get("constructs")
        .and_then(Value::as_array)
        .ok_or_else(|| malformed_at(document, "$", "constructs: missing or not an array"))?;
    let mut index = HashMap::with_capacity(constructs.len());
    for (position, entry) in constructs.iter().enumerate() {
        let ctx = NodeCtx::new(entry, format!("$.constructs[{position}]"));
        let (module, name) = ctx.kind_key()?;
        let meaning = entry
            .get("construct")
            .and_then(|construct| construct.get("meaning"))
            .and_then(Value::as_str)
            .ok_or_else(|| ctx.malformed("construct.meaning: missing or not a string"))?;
        index.insert((module.to_owned(), name.to_owned()), meaning.to_owned());
    }
    Ok(index)
}

/// Reads one `types[]` entry, dispatching purely by the `meaning` its `kind`
/// resolves to in the document's own `constructs[]` table (FR-208) -- never
/// by `kind.name`/`kind.module` directly. Checks report in FR-154's own
/// order: the node's own identity form first (the object-id check), then
/// its `kind`/meaning, then the meaning-specific shape.
#[qsl_attrs::string_edge]
fn read_type_node(
    package: &str,
    meanings: &HashMap<(String, String), String>,
    type_meanings: &HashMap<&str, &str>,
    type_value: &Value,
    at: &str,
    pending: &mut Vec<PendingFrame>,
) -> Result<Vec<DomainPackageRecord>, ModelRefusal> {
    let ctx = NodeCtx::new(type_value, at);
    let (_, key) = read_type_identity(package, &ctx)?;
    let node = key.node.as_str();
    if !type_value.get("kind").is_some_and(Value::is_object) {
        // A bare-string kind (`"scalar"`, `"alias"`, `"record"`, ...) is a
        // core kind FCD's own schema admits, but QSpec's declaration-kinds
        // table has no row for a package declaring one of these directly in
        // `types[]` -- every declared type resolves via a `constructs[]`
        // entry to a real FR-208 meaning instead. A field's own `typeRef`
        // never names a node like this one anyway: [`read_value_type_ref`]
        // resolves a native value type through the dedicated
        // `ix://quire/native/<Name>` form, not through a bare-string-kind
        // `types[]` declaration, so this node being refused as a top-level
        // declaration never stops a field resolving its own type reference.
        return Err(ctx.malformed("kind: a bare-string core kind is not a declared FR-208 meaning"));
    }
    let (module, name) = ctx.kind_key()?;
    let Some(construct_meaning) = meanings.get(&(module.to_owned(), name.to_owned())) else {
        return Err(ctx.malformed("kind: names no constructs[] entry"));
    };
    // `agent-ix-semantic-ir` binds `VALUE_TYPE` to two wire shapes: a plain
    // scalar (`scalar` + `constraints`, no `fields`) and a record-shaped one
    // (a non-empty `fields`, e.g. its own `value_object`/`event` forms).
    // `read_value_type` claims only the former; a record-shaped node falls
    // through to the generic known-but-unsupported bucket below, the same
    // refusal it got before that reader existed, rather than being routed
    // into the scalar reader's own malformed/missing-declaration checks.
    if construct_meaning.as_str() == meaning::VALUE_TYPE && !ctx.array_field("fields")?.is_empty() {
        return Err(unsupported_at(type_value, at, meaning::VALUE_TYPE));
    }
    let mut records = Vec::new();
    match construct_meaning.as_str() {
        meaning::OBJECT_TYPE => {
            read_object_type(package, type_value, node, at, false, &mut records, pending)?
        }
        meaning::SYSTEMS_INTERFACE => {
            read_object_type(package, type_value, node, at, true, &mut records, pending)?
        }
        meaning::RECORD_VALUE_TYPE => {
            read_record_value_type(package, type_value, node, at, type_meanings, &mut records)?
        }
        meaning::VALUE_TYPE => {
            records.push(DomainPackageRecord::ScalarType(read_value_type(
                package, type_value, node, at,
            )?));
        }
        meaning::SYSTEMS_PART => {
            records.push(DomainPackageRecord::Component(read_component(
                package, &ctx, node,
            )?));
        }
        meaning::SYSTEMS_PORT => {
            records.push(DomainPackageRecord::Endpoint(read_endpoint(
                package, &ctx, node,
            )?));
        }
        meaning::SYSTEMS_CONNECTION => {
            records.push(DomainPackageRecord::Relationship(read_connection(
                package, &ctx, node,
            )?));
        }
        meaning::SYSTEMS_ALLOCATION => {
            records.push(DomainPackageRecord::Allocation(read_allocation(
                package, &ctx, node,
            )?));
        }
        other if meaning::ALL.contains(&other) => {
            // A real FR-208 meaning (e.g. EVENT_TYPE, STATE_MACHINE) with no
            // QSL record shape yet, or a real meaning whose shape this
            // reader claims only part of (VALUE_TYPE's record-shaped form,
            // caught above before this match) -- refused as a
            // known-but-unsupported declaration form, never silently folded
            // into `ObjectTypeRecord`.
            return Err(unsupported_at(type_value, at, other));
        }
        other => {
            return Err(ctx.malformed(format!("kind: resolves to {other:?}, outside FR-208")));
        }
    }
    Ok(records)
}

/// What a domain package record names, for FR-103's frame-entry
/// classification. No `Process` case exists: `read_type_node` already
/// refuses a real FR-208 meaning with no reader (`unsupported_at`), so a
/// process declaration (QSpec `model-complete.md`'s Frames row admits
/// `creates`/`deletes` naming one) never reaches an admitted document's
/// records at all -- the whole document refuses earlier, at the process
/// type's own node. FR-103's own "or a process" case is therefore
/// unreachable through this reader today, not silently mis-classified.
#[derive(Clone, Copy, Eq, PartialEq)]
enum FrameEntryKind {
    Field,
    ObjectType,
    Relationship,
    /// Every other record kind: a value type, a population, a component, an
    /// endpoint, an allocation or an operation.
    Other,
}

fn frame_entry_kind(record: &DomainPackageRecord) -> FrameEntryKind {
    match record {
        DomainPackageRecord::FieldMember(_) => FrameEntryKind::Field,
        DomainPackageRecord::ObjectType(_) => FrameEntryKind::ObjectType,
        DomainPackageRecord::Relationship(_) => FrameEntryKind::Relationship,
        DomainPackageRecord::RecordValueType(_)
        | DomainPackageRecord::ScalarType(_)
        | DomainPackageRecord::OperationMember(_)
        | DomainPackageRecord::Component(_)
        | DomainPackageRecord::Endpoint(_)
        | DomainPackageRecord::Allocation(_)
        | DomainPackageRecord::Population(_) => FrameEntryKind::Other,
    }
}

/// One `modifies`/`creates`/`deletes` array's entries, resolved against
/// `classification` in document order, checking the entries of `modifies`,
/// then `creates`, then `deletes` (FR-103's own document order), and
/// reporting the first that fails within this array. `expected` is the one
/// [`FrameEntryKind`] this array admits (`Field` for `modifies`,
/// `ObjectType` for `creates`/`deletes`); `unsupported`, when given, is the
/// other kind the Frames row admits but this reader cannot grant
/// (`Relationship` for `modifies`). `creates`/`deletes` pass `None`: the
/// Frames row also admits a process there, but [`FrameEntryKind`] has no
/// case for it (see its own doc), so every other kind there is
/// `malformed-declaration`, never `unsupported-feature`, until a process
/// can actually be classified.
fn resolve_frame_array(
    package: &str,
    classification: &HashMap<&str, FrameEntryKind>,
    frame: &PendingFrame,
    entries: &[(String, String)],
    expected: FrameEntryKind,
    unsupported: Option<FrameEntryKind>,
) -> Result<Vec<DeclarationKey>, ModelRefusal> {
    let mut keys = Vec::with_capacity(entries.len());
    for (entry, at) in entries {
        match classification.get(entry.as_str()) {
            None => {
                return Err(missing_frame_declaration(
                    frame.node.clone(),
                    frame.artifact.clone(),
                    frame.span,
                    entry,
                    at,
                ))
            }
            Some(kind) if *kind == expected => keys.push(declaration_key(package, entry)),
            Some(kind) if Some(*kind) == unsupported => {
                return Err(unsupported_frame_feature(
                    frame.node.clone(),
                    frame.artifact.clone(),
                    frame.span,
                    entry,
                    at,
                ))
            }
            Some(_) => {
                return Err(malformed_frame_entry(
                    frame.node.clone(),
                    frame.artifact.clone(),
                    frame.span,
                    entry,
                    at,
                ))
            }
        }
    }
    Ok(keys)
}

/// One [`PendingFrame`]'s resolved [`OperationEffect`]: `modifies`, then
/// `creates`, then `deletes`, each checked in document order, reporting the
/// first that fails (FR-103's own Behavior; TC-458-AC-2 step 5).
fn resolve_frame(
    package: &str,
    classification: &HashMap<&str, FrameEntryKind>,
    frame: &PendingFrame,
) -> Result<OperationEffect, ModelRefusal> {
    let modifies = resolve_frame_array(
        package,
        classification,
        frame,
        &frame.modifies,
        FrameEntryKind::Field,
        Some(FrameEntryKind::Relationship),
    )?;
    let creates = resolve_frame_array(
        package,
        classification,
        frame,
        &frame.creates,
        FrameEntryKind::ObjectType,
        None,
    )?;
    let deletes = resolve_frame_array(
        package,
        classification,
        frame,
        &frame.deletes,
        FrameEntryKind::ObjectType,
        None,
    )?;
    Ok(OperationEffect {
        modifies,
        creates,
        deletes,
    })
}

/// FR-103: resolves every [`PendingFrame`] `read_operation_member` queued,
/// against a package-wide classification of every record `read_nodes`
/// collected (order-independent: TC-458-AC-5), and writes each resolved
/// [`OperationEffect`] onto its own [`OperationMemberRecord`] in `records`.
/// Every resolution failure is collected into `refusals`, exactly as every
/// other node's own refusal is (FR-154, `model-complete.md`:74-83).
fn resolve_pending_frames(
    package: &str,
    pending: &[PendingFrame],
    records: &mut [DomainPackageRecord],
    refusals: &mut Vec<ModelRefusal>,
) {
    let classification: HashMap<&str, FrameEntryKind> = records
        .iter()
        .map(|record| (record.key().node.as_str(), frame_entry_kind(record)))
        .collect();
    let mut resolved: Vec<(DeclarationKey, OperationEffect)> = Vec::with_capacity(pending.len());
    for frame in pending {
        match resolve_frame(package, &classification, frame) {
            Ok(effect) => resolved.push((frame.operation.clone(), effect)),
            Err(refusal) => refusals.push(refusal),
        }
    }
    if !refusals.is_empty() {
        return;
    }
    for (operation, effect) in resolved {
        let member = records.iter_mut().find(|record| record.key() == &operation);
        // `operation` is always a key `read_operation_member` minted for the
        // very `OperationMemberRecord` its own caller (`read_object_type`)
        // pushed into `records` moments before queuing this `PendingFrame`
        // (see `PendingFrame`'s own doc comment) -- a miss here is a broken
        // invariant of this reader, never a property of the package, so it
        // is loud in every debug build (every test in this crate runs
        // unoptimized) rather than a silent no-op that would admit the
        // operation with the empty effect in release.
        debug_assert!(
            matches!(member, Some(DomainPackageRecord::OperationMember(_))),
            "resolve_pending_frames: {operation:?} names no OperationMemberRecord of records"
        );
        if let Some(DomainPackageRecord::OperationMember(member)) = member {
            member.effect = effect;
        }
    }
}

/// One document node under the combined `types[]`/`populations[]` ascending
/// sort ("ascending by declaration key", `model-complete.md`:74). Carries
/// its own array position (rather than re-deriving it by searching `types`/
/// `populations` back for a pointer match) so the read loop below reports
/// against the same `$.types[N]`/`$.populations[N]` path this node was
/// built from.
enum DocumentNode<'a> {
    Type(&'a Value, usize, String),
    Population(&'a Value, usize, String),
}

/// Reads an admitted Semantic IR 2.0.0 document's `types[]` and
/// `populations[]` into [`DomainPackageRecord`]s under `package_identity`.
/// Reads every node -- ascending by its own declaration key -- and collects
/// every refusal rather than stopping at the first (FR-154,
/// `model-complete.md`:74-83): `Ok` only when every node read cleanly,
/// `Err` with every node's refusal, in that same ascending order, otherwise.
pub fn read_records(
    package_identity: &str,
    document: &PackageDocument,
) -> Result<Vec<DomainPackageRecord>, Vec<ModelRefusal>> {
    validate_with_semantic_ir(document)?;
    read_nodes(package_identity, &document.tree)
}

/// [`read_records`]'s per-node reader over a document
/// [`validate_with_semantic_ir`] has already accepted. Every shape the
/// validator guarantees is still checked, so a dependency pin that stops
/// guaranteeing one degrades to a refusal rather than a panic.
fn read_nodes(
    package_identity: &str,
    document: &Value,
) -> Result<Vec<DomainPackageRecord>, Vec<ModelRefusal>> {
    let meanings = meaning_index(document).map_err(|refusal| vec![refusal])?;
    // `agent-ix-semantic-ir`'s own `IR_MEMBERS` requires `types` present and
    // `expect_array` guarantees it is an array; `populations` is optional
    // but array-shaped when present.
    let root = NodeCtx::new(document, "$");
    let types = document
        .get("types")
        .and_then(Value::as_array)
        .ok_or_else(|| vec![root.malformed("types: missing or not an array")])?;
    let populations = root
        .array_field("populations")
        .map_err(|refusal| vec![refusal])?;

    // Each type identity -> the meaning its `kind` resolves to, for the
    // cross-node meaning checks (a record value type's supertypes).
    let type_meanings: HashMap<&str, &str> = types
        .iter()
        .filter_map(|type_value| {
            let identity = type_value.get("identity")?.as_str()?;
            let kind = type_value.get("kind")?;
            let key = (
                kind.get("module")?.as_str()?.to_owned(),
                kind.get("name")?.as_str()?.to_owned(),
            );
            Some((identity, meanings.get(&key)?.as_str()))
        })
        .collect();
    let mut nodes: Vec<DocumentNode<'_>> = Vec::with_capacity(types.len() + populations.len());
    for (position, type_value) in types.iter().enumerate() {
        let at = format!("$.types[{position}]");
        let label = node_identity_label(type_value, &at);
        nodes.push(DocumentNode::Type(type_value, position, label));
    }
    for (position, population) in populations.iter().enumerate() {
        let at = format!("$.populations[{position}]");
        let label = node_identity_label(population, &at);
        nodes.push(DocumentNode::Population(population, position, label));
    }
    nodes.sort_by(|a, b| {
        let label = |node: &DocumentNode<'_>| match node {
            DocumentNode::Type(_, _, label) | DocumentNode::Population(_, _, label) => {
                label.clone()
            }
        };
        label(a).cmp(&label(b))
    });

    let mut records = Vec::new();
    let mut refusals = Vec::new();
    let mut pending: Vec<PendingFrame> = Vec::new();
    for node in nodes {
        match node {
            DocumentNode::Type(type_value, position, _) => {
                let at = format!("$.types[{position}]");
                match read_type_node(
                    package_identity,
                    &meanings,
                    &type_meanings,
                    type_value,
                    &at,
                    &mut pending,
                ) {
                    Ok(mut new_records) => records.append(&mut new_records),
                    Err(refusal) => refusals.push(refusal),
                }
            }
            DocumentNode::Population(population, position, _) => {
                let at = format!("$.populations[{position}]");
                let ctx = NodeCtx::new(population, at.clone());
                let result = read_type_identity(package_identity, &ctx).and_then(|(_, key)| {
                    read_population(package_identity, &ctx, &key.node, &meanings)
                });
                match result {
                    Ok(record) => records.push(DomainPackageRecord::Population(record)),
                    Err(refusal) => refusals.push(refusal),
                }
            }
        }
    }
    // FR-103: resolve every queued operation frame against the whole
    // package's own records, only once every node above has been read --
    // an entry may name a sibling type's field or a type declared later in
    // the document (TC-458-AC-5: admission is deterministic regardless of
    // document order). Skipped entirely when an earlier node already
    // refused: a broken package resolves no frame.
    if refusals.is_empty() {
        resolve_pending_frames(package_identity, &pending, &mut records, &mut refusals);
    }
    if refusals.is_empty() {
        Ok(records)
    } else {
        Err(refusals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use sha2::{Digest, Sha256};

    fn digest_of(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }

    /// `value`'s RFC 8785 bytes as the `sha256-jcs` digest reads them: the
    /// one encoder (ADR-013:113) over the shared reader's tree of `value`'s
    /// JSON text, each number the double its text denotes.
    fn canonical(value: &Value) -> Vec<u8> {
        let text = serde_json::to_vec(value).expect("the test value serializes");
        let document = quire_canonical::read(&text, u64::MAX).expect("the test value reads");
        quire_canonical::to_vec(
            &document,
            digest_limits(IntakeLimits::default().input_bytes),
        )
        .expect("the test value has an RFC 8785 encoding")
    }

    /// Test document bytes through intake's one parse.
    fn parse_document(bytes: &[u8]) -> PackageDocument {
        PackageDocument::parse(bytes, qsl_foundation::IntakeLimits::default())
            .expect("the test document parses as JSON")
    }

    fn package_bytes(identity: &str, version: &str) -> Vec<u8> {
        serde_json::json!({
            "contractVersion": "2.0.0",
            "package": {"identity": identity, "version": version},
        })
        .to_string()
        .into_bytes()
    }

    fn selection(
        identity: &str,
        version: &str,
        digest_domain: &str,
        digest: [u8; 32],
    ) -> (DomainPackageRef, String) {
        (
            DomainPackageRef {
                identity: identity.to_owned(),
                version: version.to_owned(),
                digest,
            },
            digest_domain.to_owned(),
        )
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn admits_matching_selection() {
        let bytes = package_bytes("acme/orders", "1");
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes.clone());
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let (package_ref, document) = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap();
        assert_eq!(package_ref.identity, "acme/orders");
        assert_eq!(package_ref.version, "1");
        assert_eq!(package_ref.digest, digest);
        assert_eq!(
            document.tree(),
            &serde_json::from_slice::<Value>(&bytes).unwrap()
        );
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn admits_a_whitespace_variant_under_the_same_jcs_digest() {
        // The digest is taken over JCS bytes, not the raw bytes verbatim,
        // so a byte-different-but-JCS-equivalent re-encoding of the same
        // document (extra whitespace here) still admits under the digest
        // recorded for the compact encoding.
        let compact = package_bytes("acme/orders", "1");
        let digest = digest_of(&canonical(&serde_json::from_slice(&compact).unwrap()));
        let padded = b"  \n\t"
            .iter()
            .chain(compact.iter())
            .copied()
            .collect::<Vec<u8>>();
        assert_ne!(padded, compact, "the two encodings are byte-different");
        let mut map = BTreeMap::new();
        map.insert(digest, padded.clone());
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let (package_ref, document) = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap();
        assert_eq!(package_ref.digest, digest);
        assert_eq!(
            document.tree(),
            &serde_json::from_slice::<Value>(&compact).unwrap()
        );
    }

    // These five `admit` refusal tests assert the whole `ModelRefusal` --
    // `code`, every typed field of the matched `ModelRefusalCause` variant,
    // and `detail` -- not just `.code`/`.cause.as_str()`, so a regression
    // that keeps the right tag but drops or corrupts a typed field (the
    // wrong `expected` digest domain, the caller's own selection echoed
    // back wrong) still fails the test.

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_foreign_digest_domain() {
        let map = BTreeMap::new();
        let (offered, digest_domain) = selection("acme/orders", "1", "sha1", [0; 32]);
        let refusal = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::StaleDependency,
                cause: ModelRefusalCause::DigestDomainMismatch {
                    expected: SHA256_JCS_DIGEST_DOMAIN,
                    actual: "sha1".to_owned(),
                },
                detail: "domain package selection acme/orders@1 names digest \
                          domain \"sha1\", not \"sha256-jcs\""
                    .to_owned(),
            }
        );
    }

    /// (#260 review round 3) `"sha1"` above is not an FR-201 label at all,
    /// so it cannot tell this check apart from one that refuses only
    /// *unrecognized* domains while silently admitting a *valid but wrong*
    /// one -- a real FR-201 label (`ir-canonical`, O-18's own vocabulary)
    /// that is not `sha256-jcs` must refuse here too (FR-154 Intake check
    /// 1 is a total match against `sha256-jcs`, not "is this any known
    /// domain").
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_valid_fr201_domain_that_is_not_sha256_jcs() {
        let map = BTreeMap::new();
        let (offered, digest_domain) = selection("acme/orders", "1", "ir-canonical", [0; 32]);
        let refusal = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::StaleDependency,
                cause: ModelRefusalCause::DigestDomainMismatch {
                    expected: SHA256_JCS_DIGEST_DOMAIN,
                    actual: "ir-canonical".to_owned(),
                },
                detail: "domain package selection acme/orders@1 names digest \
                          domain \"ir-canonical\", not \"sha256-jcs\""
                    .to_owned(),
            }
        );
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_missing_bytes() {
        let map = BTreeMap::new();
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, [0; 32]);
        let refusal = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::MissingImport,
                cause: ModelRefusalCause::MissingSelection {
                    selection: DomainPackageRef {
                        identity: "acme/orders".to_owned(),
                        version: "1".to_owned(),
                        digest: [0; 32],
                    },
                },
                detail: format!("no package bytes supplied under digest {}", hex(&[0; 32])),
            }
        );
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn a_parsed_document_under_another_digest_refuses_content_mismatch() {
        let bytes = package_bytes("acme/orders", "1");
        let wrong_digest = digest_of(b"not the package");
        let actual_digest = digest_of(&canonical(&serde_json::from_slice(&bytes).unwrap()));
        let mut map = BTreeMap::new();
        map.insert(wrong_digest, bytes);
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, wrong_digest);
        let refusal = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::StaleDependency,
                cause: ModelRefusalCause::ContentMismatch {
                    selected: wrong_digest,
                    recomputed: actual_digest,
                },
                detail: format!(
                    "domain package acme/orders@1 recomputes to digest {}, not the selected {}",
                    hex(&actual_digest),
                    hex(&wrong_digest)
                ),
            }
        );
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_wrong_package_identity() {
        let bytes = package_bytes("acme/other", "1");
        let digest = digest_of(&canonical(&serde_json::from_slice(&bytes).unwrap()));
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let refusal = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::WrongModelSelection {
                    selection: DomainPackageRef {
                        identity: "acme/orders".to_owned(),
                        version: "1".to_owned(),
                        digest,
                    },
                    actual_identity: "acme/other".to_owned(),
                    actual_version: "1".to_owned(),
                },
                detail: "domain package selection names acme/orders@1 but \
                          the package declares acme/other@1"
                    .to_owned(),
            }
        );
    }

    /// `admit`'s version-only mismatch -- the selected bytes' own package
    /// identity matches the selection but the version does not -- is a
    /// distinct branch of the same `WrongModelSelection` check from
    /// `refuses_wrong_package_identity`'s identity mismatch, so it gets its
    /// own whole-outcome test.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_version_only_mismatch() {
        let bytes = package_bytes("acme/orders", "2");
        let digest = digest_of(&canonical(&serde_json::from_slice(&bytes).unwrap()));
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let refusal = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::WrongModelSelection {
                    selection: DomainPackageRef {
                        identity: "acme/orders".to_owned(),
                        version: "1".to_owned(),
                        digest,
                    },
                    actual_identity: "acme/orders".to_owned(),
                    actual_version: "2".to_owned(),
                },
                detail: "domain package selection names acme/orders@1 but \
                          the package declares acme/orders@2"
                    .to_owned(),
            }
        );
    }

    /// `admit`'s non-JSON-bytes case is a distinct branch from
    /// `refuses_wrong_package_identity`'s JSON-but-wrong-identity case --
    /// bytes that are not JSON at all never reach a `package.identity`
    /// comparison, so `parsed` stays `None` and both `actual_identity`/
    /// `actual_version` default to empty strings.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_non_json_bytes_as_wrong_selection() {
        let bytes = b"not json".to_vec();
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let refusal = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::WrongModelSelection {
                    selection: DomainPackageRef {
                        identity: "acme/orders".to_owned(),
                        version: "1".to_owned(),
                        digest,
                    },
                    actual_identity: String::new(),
                    actual_version: String::new(),
                },
                detail: "domain package selection names acme/orders@1 but \
                          the package declares @"
                    .to_owned(),
            }
        );
    }

    // M5 (review of PR #200): `read_records` now runs `validate_with_semantic_ir`
    // before it walks a single node, so a document meant to exercise this
    // reader's own per-node checks below (a bare-string kind, FCD's own
    // identity form) must itself be one `agent-ix-semantic-ir`'s schema layer
    // accepts -- these helpers fill every member `semantic-ir.schema.json`
    // requires at the document, type-definition and constructs-entry levels
    // with schema-valid stand-ins, so the refusal each test asserts on is
    // this reader's own, not the validator's completeness check firing first.

    const PLACEHOLDER_DIGEST: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    fn wire_envelope(package_identity: &str, constructs: Value, types: Value) -> Value {
        serde_json::json!({
            "contractVersion": "2.0.0",
            "source": {
                "identity": format!("ix://{package_identity}/spec"),
                "version": "1.0.0",
                "dialect": "spec-bundle",
                "digest": PLACEHOLDER_DIGEST,
            },
            "package": {
                "identity": package_identity,
                "version": "1.0.0",
                "manifestDigest": PLACEHOLDER_DIGEST,
                "mappingVersions": [],
                "profileVersions": [],
                "lockDigest": PLACEHOLDER_DIGEST,
            },
            "occurrences": [],
            "extensions": [],
            "constructs": constructs,
            "types": types,
        })
    }

    fn wire_construct(module: &str, name: &str, meaning: &str, members: Value) -> Value {
        serde_json::json!({
            "kind": {"module": module, "name": name},
            "moduleVersion": "1.0.0",
            "manifestDigest": PLACEHOLDER_DIGEST,
            "construct": {
                "identity": "none",
                "shape": "record",
                "members": members,
                "meaning": meaning,
            },
        })
    }

    /// Every member `typeDefinition` requires beyond `identity`/`kind`,
    /// filled with schema-valid stand-ins; `extra`'s own members are then
    /// merged on top.
    fn wire_type(identity: &str, kind: Value, extra: Value) -> Value {
        let mut node = serde_json::json!({
            "identity": identity,
            "displayName": identity,
            "kind": kind,
            "roles": [],
            "origin": {
                "generated": {
                    "generatorIdentity": identity,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [identity],
                }
            },
            "constraints": [],
            "extensions": [],
            "unknownPolicy": "reject",
        });
        if let (Some(node), Some(extra)) = (node.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                node.insert(key.clone(), value.clone());
            }
        }
        node
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_document_that_is_not_json() {
        let refusal = PackageDocument::parse(b"not json", qsl_foundation::IntakeLimits::default())
            .unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "$".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "package document is malformed JSON at byte 0: unexpected character"
                    .to_owned(),
            }
        );
    }

    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn refuses_a_bare_string_kind_as_malformed_declaration() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([]),
            serde_json::json!([wire_type(
                "ix://acme/orders/Count",
                serde_json::json!("scalar"),
                serde_json::json!({"scalar": "integer"}),
            )]),
        )
        .to_string();
        let refusals =
            read_records("acme/orders", &parse_document(document.as_bytes())).unwrap_err();
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "ix://acme/orders/Count".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "$.types[0]: kind: a bare-string core kind is not a declared \
                          FR-208 meaning"
                    .to_owned(),
            }]
        );
    }

    /// FR-056-AC-4's second half (PR #200 review R2-3): "changing only
    /// `title` or `displayName` leaves every key, export, ordering and
    /// binding unchanged, and two artifacts with equal titles stay distinct
    /// declarations." FCD's own wire carries `displayName` (not a separate
    /// `title`); this reader's `identity`-only key derivation never reads
    /// it, so both halves hold by construction -- this test is the actual
    /// Test verification the row claims, not just the argument for it.
    #[trace("TC-145", "FR-056-AC-4")]
    #[test]
    fn changing_only_displayname_leaves_records_unchanged_and_equal_titles_stay_distinct() {
        let one_type = |identity: &str, display_name: &str| {
            let mut node = wire_type(
                identity,
                serde_json::json!({"module": "acme/orders", "name": "order"}),
                serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
            );
            node["displayName"] = serde_json::json!(display_name);
            node
        };
        let document_of = |types: Value| {
            wire_envelope(
                "acme/orders",
                serde_json::json!([wire_construct(
                    "acme/orders",
                    "order",
                    meaning::OBJECT_TYPE,
                    serde_json::json!({}),
                )]),
                types,
            )
            .to_string()
            .into_bytes()
        };

        // Half one: changing only displayName changes nothing else.
        let titled_a = document_of(serde_json::json!([one_type(
            "ix://acme/orders/Order",
            "Order A"
        )]));
        let titled_b = document_of(serde_json::json!([one_type(
            "ix://acme/orders/Order",
            "A Totally Different Title"
        )]));
        let records_a = read_records("acme/orders", &parse_document(&titled_a))
            .expect("displayName alone never refuses");
        let records_b = read_records("acme/orders", &parse_document(&titled_b))
            .expect("displayName alone never refuses");
        assert_eq!(
            records_a, records_b,
            "changing only displayName must not change the resulting records"
        );

        // Half two: two artifacts sharing one title stay distinct declarations.
        let shared_title = document_of(serde_json::json!([
            one_type("ix://acme/orders/Order", "Shared Title"),
            one_type("ix://acme/orders/Invoice", "Shared Title"),
        ]));
        let records = read_records("acme/orders", &parse_document(&shared_title))
            .expect("two artifacts with equal titles are still two distinct declarations");
        assert_eq!(records.len(), 2, "both equal-titled artifacts are admitted");
        let keys: std::collections::BTreeSet<_> =
            records.iter().map(DomainPackageRecord::key).collect();
        assert_eq!(
            keys.len(),
            2,
            "equal titles must not collapse two distinct artifacts onto one key"
        );
    }

    #[trace("TC-145", "FR-056-AC-4")]
    #[test]
    fn refuses_fcd_own_identity_form_as_malformed_declaration() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "order",
                meaning::OBJECT_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([wire_type(
                "ix://acme/orders/type/Order",
                serde_json::json!({"module": "acme/orders", "name": "order"}),
                serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
            )]),
        )
        .to_string();
        let refusals =
            read_records("acme/orders", &parse_document(document.as_bytes())).unwrap_err();
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "ix://acme/orders/type/Order".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "$.types[0]: identity: \"ix://acme/orders/type/Order\" is not \
                          ix://acme/orders/<artifact id>"
                    .to_owned(),
            }]
        );
    }

    /// M5: an `abstract` present but not a boolean is exactly the shape
    /// `expect_bool` in `agent-ix-semantic-ir`'s own schema layer refuses
    /// (`crates/semantic-ir/src/schema.rs`'s `type_definition`), so this
    /// document never reaches `read_type_node`'s own hand-rolled `abstract`
    /// check at all -- `validate_with_semantic_ir` refuses it first, naming
    /// the validator's own diagnostic (its owning declaration, code and
    /// message).
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_non_boolean_abstract_via_the_semantic_ir_validator() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "order",
                meaning::OBJECT_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([wire_type(
                "ix://acme/orders/Order",
                serde_json::json!({"module": "acme/orders", "name": "order"}),
                serde_json::json!({
                    "supertypes": [],
                    "fields": [],
                    "operations": [],
                    "abstract": "yes",
                }),
            )]),
        )
        .to_string();
        let refusals =
            read_records("acme/orders", &parse_document(document.as_bytes())).unwrap_err();
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    // `Located::owner` (PR #200 review R2-2): the nearest
                    // declaration that owns `/ir/types/0/abstract`, not the
                    // raw pointer itself -- FR-056-AC-2 retains "every
                    // reader diagnostic" with "its IR node", and an actual
                    // declaration identity is a real IR node in a way a JSON
                    // pointer into the wrapped bundle is not.
                    node: "ix://acme/orders/Order".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "agent-ix-semantic-ir refused this document at \
                          /ir/types/0/abstract (agent-ix.semantic-ir.SCHEMA_VIOLATION): \
                          abstract is a boolean"
                    .to_owned(),
            }],
            "the refusal names the validator's own diagnostic (owning declaration, code, \
             message), not this reader's own hand-rolled check"
        );
    }

    /// R2-2 (PR #200 review round 2): `validate_with_semantic_ir` used to
    /// build one refusal from the *first* `Severity::Error` diagnostic and
    /// silently drop every later one -- FR-056-AC-2 requires retaining
    /// "every reader diagnostic". Two schema-invalid types, each with its
    /// own non-boolean `abstract`, must both be named.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_every_semantic_ir_diagnostic_not_only_the_first() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "order",
                meaning::OBJECT_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([
                wire_type(
                    "ix://acme/orders/Order",
                    serde_json::json!({"module": "acme/orders", "name": "order"}),
                    serde_json::json!({
                        "supertypes": [],
                        "fields": [],
                        "operations": [],
                        "abstract": "yes",
                    }),
                ),
                wire_type(
                    "ix://acme/orders/Invoice",
                    serde_json::json!({"module": "acme/orders", "name": "order"}),
                    serde_json::json!({
                        "supertypes": [],
                        "fields": [],
                        "operations": [],
                        "abstract": "also yes",
                    }),
                ),
            ]),
        )
        .to_string();
        let refusals =
            read_records("acme/orders", &parse_document(document.as_bytes())).unwrap_err();
        assert_eq!(
            refusals.len(),
            2,
            "both invalid types refuse, not only the first"
        );
        let nodes: std::collections::BTreeSet<&str> = refusals
            .iter()
            .map(|refusal| match &refusal.cause {
                ModelRefusalCause::IntakeMalformedDeclaration { node, .. } => node.as_str(),
                other => panic!("expected IntakeMalformedDeclaration, got {other:?}"),
            })
            .collect();
        assert_eq!(
            nodes,
            std::collections::BTreeSet::from([
                "ix://acme/orders/Order",
                "ix://acme/orders/Invoice"
            ]),
            "each diagnostic names its own owning declaration, not just the first one's"
        );
    }

    /// Every member `semantic-ir.schema.json`'s `field` requires beyond
    /// `identity`/`typeRef`, filled with schema-valid stand-ins.
    fn wire_field(identity: &str, name: &str, type_ref: &str) -> Value {
        serde_json::json!({
            "identity": identity,
            "name": name,
            "typeRef": type_ref,
            "presence": "optional",
            "nullable": false,
            "defaultKind": "none",
            // `crate::model::intake::NodeCtx::multiplicity` requires
            // `ordered`/`unique` present on every field multiplicity it
            // reads, but `agent-ix-semantic-ir`'s own `field_rules` refuses
            // either key present at all on a single-valued one (`upper <=
            // 1`, `FLAGS_ON_NON_COLLECTION`) -- an unbounded multiplicity
            // satisfies both.
            "multiplicity": {"lower": 0, "ordered": false, "unique": true},
            "origin": {
                "generated": {
                    "generatorIdentity": identity,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [identity],
                }
            },
        })
    }

    /// A one-type, one-field document, wrapping `field`'s `typeRef` in
    /// `field`.
    fn document_with_one_field(field: Value) -> Vec<u8> {
        wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "order",
                meaning::OBJECT_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([
                wire_type(
                    "ix://acme/orders/Widget",
                    serde_json::json!({"module": "acme/orders", "name": "order"}),
                    serde_json::json!({
                        "supertypes": [],
                        "fields": [field],
                        "operations": [],
                    }),
                ),
                // A real node of the package, for a `typeRef` naming a
                // package type -- `agent-ix-semantic-ir`'s own
                // `UNRESOLVED_TYPE_REF` rule requires the reference to
                // resolve to a declared identity, same as QSL's own
                // `read_value_type_ref`.
                wire_type(
                    "ix://acme/orders/OtherType",
                    serde_json::json!({"module": "acme/orders", "name": "order"}),
                    serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
                ),
            ]),
        )
        .to_string()
        .into_bytes()
    }

    /// A field whose multiplicity `upper` is 2^60 + 1, a whole number no
    /// double equals, is refused at the parse `noncanonical_wire` at that
    /// bound's pointer. Admission returns that refusal before check 3
    /// (FR-056, IR FR-038-AC-93), whatever digest selects the bytes: under
    /// the RFC 8785 digest of the document with the bound read as its
    /// double, 2^60, which ECMAScript spells `1152921504606847000`
    /// (`JSON.stringify(JSON.parse("1152921504606846977"))` in Node prints
    /// exactly that) and under which the document used to admit, and under
    /// the bytes' raw digest, never `byte-digest-mismatch` or
    /// `wrong-model-selection`.
    #[trace("TC-145", "FR-056-AC-13")]
    #[test]
    fn a_u64_bound_above_2_53_refuses_at_its_pointer_before_check_3() {
        const EXACT: u64 = (1 << 60) + 1;
        let mut field = wire_field(
            "ix://acme/orders/Widget/count",
            "count",
            "ix://quire/native/Integer",
        );
        field["multiplicity"] =
            serde_json::json!({"lower": 0, "upper": EXACT, "ordered": false, "unique": true});
        let bytes = document_with_one_field(field);
        let expected = noncanonical_number(
            "/types/0/fields/0/multiplicity/upper".parse().unwrap(),
            Inexact::Integer,
            &EXACT.to_string(),
        );
        assert_eq!(
            PackageDocument::parse(&bytes, qsl_foundation::IntakeLimits::default()).unwrap_err(),
            expected
        );

        // The RFC 8785 text the document used to admit under, the bound
        // spelled as ECMAScript spells 2^60.
        let mut rounded: Value = serde_json::from_slice(&bytes).unwrap();
        rounded["types"][0]["fields"][0]["multiplicity"]["upper"] =
            Value::Number(serde_json::Number::from_f64(1_152_921_504_606_846_976.0).unwrap());
        let text = String::from_utf8(canonical(&rounded)).unwrap();
        assert!(text.contains(r#""upper":1152921504606847000}"#), "{text}");
        let rounded_digest = digest_of(text.as_bytes());
        let raw = digest_of(&bytes);
        let map = BTreeMap::from([(rounded_digest, bytes.clone()), (raw, bytes)]);
        for digest in [rounded_digest, raw] {
            let (offered, digest_domain) =
                selection("acme/orders", "1.0.0", SHA256_JCS_DIGEST_DOMAIN, digest);
            assert_eq!(
                admit(
                    &offered,
                    &digest_domain,
                    &map,
                    qsl_foundation::IntakeLimits::default()
                )
                .unwrap_err(),
                expected
            );
        }
    }

    // `agent-ix-semantic-ir` at the pinned rev resolves
    // `ix://quire/native/<Name>` at the validator layer (see
    // `validate_with_semantic_ir`'s own doc comment for FCD's upstream fix
    // and why it still does not widen what this reader accepts), and
    // end-to-end native-typeRef resolution is exercised via
    // `tests/model_intake.rs`'s
    // `reading_fcd_199s_golden_shape_admits_the_schema_and_refuses_4_of_its_12_types`.
    // These tests instead drive [`read_value_type_ref`] directly, the same
    // resolver every field/parameter/`returns` typeRef site calls, so each
    // one's own refusal/success shape is pinned independently of any one
    // fixture document.

    /// PR #200 review R2-4/R3-3: untagged, not re-traced. This drives
    /// [`read_value_type_ref`] directly -- a native `typeRef`'s own
    /// resolution rule, no FR-056 AC's subject (AC-1 covers per-IR-node
    /// declaration production: meaning, exports, artifact id, span). FR-152
    /// (`quire-specification`) does carry acceptance criteria for the
    /// systems binder's own reference resolution (FR-152-AC-1..8), but no
    /// `FR-151-AC` or `FR-152-AC` row exists anywhere in this repo's own
    /// matrices (`spec/model-linking/tests.md` and siblings) -- tagging a
    /// test with one would point at a row that does not exist locally.
    /// FR-152-AC-4 in particular ("port direction, port interface type and
    /// multiplicity are all enforced before a connection is exposed",
    /// verified by TC-197) is about the binder's own `check_connection`
    /// (`crate::model::systems`), not about intake's wire-level `typeRef`
    /// reading this test exercises. Untagged rather than mistagged, the
    /// same honest-untag choice as `xtask/src/cargo_pin.rs`'s tests (F6).
    #[test]
    fn resolves_a_known_native_type_ref() {
        let node = serde_json::json!({"identity": "ix://acme/orders/Widget/flag"});
        let ctx = NodeCtx::new(&node, "$.types[0].fields[0]");
        let value_type = read_value_type_ref("acme/orders", &ctx, "ix://quire/native/Boolean")
            .expect("a known, unparameterized native typeRef resolves");
        assert_eq!(value_type, ValueTypeRef::Native(NativeValueType::Boolean));
    }

    /// PR #200 review R2-4/R3-3: untagged, not re-traced -- see
    /// `resolves_a_known_native_type_ref`'s own doc comment (no local
    /// FR-151-AC/FR-152-AC row exists to tag against, and FR-152-AC-4 is
    /// the binder's `check_connection`, not intake's typeRef reading).
    /// Also a direct `read_value_type_ref` test, not AC-3's kind/meaning
    /// subject.
    #[test]
    fn refuses_an_unknown_native_type_ref_as_malformed_declaration() {
        let node = serde_json::json!({"identity": "ix://acme/orders/Widget/flag"});
        let ctx = NodeCtx::new(&node, "$.types[0].fields[0]");
        let refusal = read_value_type_ref("acme/orders", &ctx, "ix://quire/native/Frobnicate")
            .expect_err("a native-prefixed typeRef naming no native value type refuses");
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "ix://acme/orders/Widget/flag".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "$.types[0].fields[0]: typeRef: \"ix://quire/native/Frobnicate\" \
                          names no native value type QSL declares"
                    .to_owned(),
            }
        );
    }

    /// R5 (the PR owner's ruling, authorized by the repo owner): QSpec's
    /// `type-ref` production (`shared-grammar.md:285-296`) is
    /// `{Boolean, Integer, Rational, Decimal, Float32, Float64, Text}`, and
    /// that is the closed set [`NativeValueType`] declares -- FCD's own
    /// native-scalar set is wider (adds `UUID`, `String`, `Timestamp`,
    /// `Duration`, `Bytes`, `JsonObject`), and QSL does not widen its own
    /// vocabulary, add a mapping, or otherwise translate a name outside its
    /// grammar. `ix://quire/native/UUID` -- the exact typeRef FCD #199's new
    /// golden gives `Pump.id`/`Sys.id`/`Tank.id` -- is this ruling's
    /// concrete case: it refuses today, and it keeps refusing after the FCD
    /// pin bumps (FCD's own validator resolving the prefix, F7's fix to
    /// `validate_with_semantic_ir`'s doc comment, does not change what this
    /// reader itself accepts). PLAT-836 (filed separately, lands after FCD
    /// #200) narrows FCD's own emission so `UUID` stops being a native
    /// reference there at all -- a field like `Pump.id` becomes a reference
    /// to a model-declared type instead, at which point this exact refusal
    /// stops occurring, but as a consequence of what the *document* emits,
    /// not because this test or this reader changed.
    ///
    /// PR #200 review R2-4/R3-3: untagged, not re-traced -- see
    /// `resolves_a_known_native_type_ref`'s own doc comment (no local
    /// FR-151-AC/FR-152-AC row exists to tag against, and FR-152-AC-4 is
    /// the binder's `check_connection`, not intake's typeRef reading).
    /// Also a direct `read_value_type_ref` test, not AC-3's kind/meaning
    /// subject.
    #[test]
    fn refuses_uuid_as_malformed_declaration_r5_holds_until_plat_836() {
        let node = serde_json::json!({"identity": "ix://agent-ix/architecture/Pump/id"});
        let ctx = NodeCtx::new(&node, "$.types[0].fields[0]");
        let refusal = read_value_type_ref("agent-ix/architecture", &ctx, "ix://quire/native/UUID")
            .expect_err(
                "R5: FCD's native-scalar set is wider than QSpec's type-ref; QSL does not widen \
                 to match it, so ix://quire/native/UUID refuses",
            );
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "ix://agent-ix/architecture/Pump/id".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "$.types[0].fields[0]: typeRef: \"ix://quire/native/UUID\" \
                          names no native value type QSL declares"
                    .to_owned(),
            }
        );
    }

    /// H3: `Rational`/`Decimal`/`Text` are parameterized in shared-grammar.md,
    /// and `agent-ix-semantic-ir`'s own constraint keyword vocabulary has no
    /// `dmin`/`dmax`/`profile` keyword at all -- so these refuse
    /// unconditionally, naming which parameters are unexpressable, never
    /// resolving to a made-up declaration.
    /// PR #200 review R2-4/R3-3: untagged, not re-traced -- see
    /// `resolves_a_known_native_type_ref`'s own doc comment (no local
    /// FR-151-AC/FR-152-AC row exists to tag against, and FR-152-AC-4 is
    /// the binder's `check_connection`, not intake's typeRef reading).
    /// Also a direct `read_value_type_ref` test, not AC-3's kind/meaning
    /// subject.
    #[test]
    fn refuses_a_parameterized_native_type_as_unsupported() {
        let node = serde_json::json!({"identity": "ix://acme/orders/Widget/amount"});
        let ctx = NodeCtx::new(&node, "$.types[0].fields[0]");
        let refusal = read_value_type_ref("acme/orders", &ctx, "ix://quire/native/Decimal")
            .expect_err(
                "Decimal requires dmin/dmax, which the constraint vocabulary cannot express",
            );
        assert_eq!(refusal.code, Code::UnsupportedConstruct);
        assert_eq!(refusal.cause.as_str(), "declaration-form");
        assert!(
            refusal.detail.contains("dmin") && refusal.detail.contains("dmax"),
            "the detail names the missing parameters: {:?}",
            refusal.detail
        );
    }

    // `read_records`'s own two hand-rolled `unsupported_at` refusals -- a
    // non-empty operation `frame` and a non-empty type-level
    // `relationships[]` -- and its known-but-unsupported-FR-208-meaning
    // refusal are branches that survive `validate_with_semantic_ir`'s gate:
    // FCD's own schema and `constructs.rs` rules permit all three shapes
    // generically (`frame`, `relationships` and an FR-208 `meaning` string
    // are all optional, free-form members from FCD's point of view), so a
    // document reaching them is schema-valid and these are genuinely this
    // reader's own narrower-than-FCD refusals, not the validator's.

    /// A two-type document -- `Widget` (the node under test) and
    /// `OtherType` (a real node of the package for a `target`/`creates`
    /// reference to resolve against, since `agent-ix-semantic-ir`'s own
    /// `constructs.rs` rules -- `frames`'s `UNRESOLVED_FRAME_PATH`, a
    /// relationship's own reference checks -- require these to name a real
    /// declared node) -- with `extra` merged onto `Widget`'s own members,
    /// on top of the `fields`/`operations`/`supertypes` every `OBJECT_TYPE`
    /// needs present for `read_object_type` to run at all.
    fn document_with_object_type_extra(extra: Value) -> Vec<u8> {
        let mut widget = serde_json::json!({
            "supertypes": [],
            "fields": [],
            "operations": [],
        });
        if let (Some(widget_obj), Some(extra_obj)) = (widget.as_object_mut(), extra.as_object()) {
            for (key, value) in extra_obj {
                widget_obj.insert(key.clone(), value.clone());
            }
        }
        wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "order",
                meaning::OBJECT_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([
                wire_type(
                    "ix://acme/orders/Widget",
                    serde_json::json!({"module": "acme/orders", "name": "order"}),
                    widget,
                ),
                wire_type(
                    "ix://acme/orders/OtherType",
                    serde_json::json!({"module": "acme/orders", "name": "order"}),
                    serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
                ),
            ]),
        )
        .to_string()
        .into_bytes()
    }

    /// FR-103: an operation `frame` that actually declares
    /// something -- here a `creates` naming a real object type of the
    /// document -- resolves to an `OperationEffect` naming that type's own
    /// key, instead of refusing as unsupported (superseding this test's own
    /// prior FR-056-AC-3 assertion, from before FR-103 amended it).
    #[trace("TC-146", "FR-103-AC-1")]
    #[test]
    fn a_non_empty_operation_frame_resolves_to_its_effect() {
        let document = document_with_object_type_extra(serde_json::json!({
            "operations": [{
                "identity": "ix://acme/orders/Widget/discard",
                "name": "discard",
                "params": [],
                "pre": [],
                "post": [],
                "origin": {
                    "generated": {
                        "generatorIdentity": "ix://acme/orders/Widget/discard",
                        "generatorVersion": "1.0.0",
                        "inputIdentities": ["ix://acme/orders/Widget/discard"],
                    }
                },
                "frame": {
                    "modifies": [],
                    "creates": ["ix://acme/orders/OtherType"],
                    "deletes": [],
                },
            }],
        }));
        let records = read_records("acme/orders", &parse_document(&document))
            .expect("a creates entry naming a real object type is admitted");
        let operation = records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::OperationMember(operation) => Some(operation),
                _ => None,
            })
            .expect("the operation is read");
        assert_eq!(
            operation.effect,
            OperationEffect {
                modifies: Vec::new(),
                creates: vec![DeclarationKey {
                    package: "acme/orders".to_owned(),
                    node: "ix://acme/orders/OtherType".to_owned(),
                }],
                deletes: Vec::new(),
            }
        );
    }

    /// A type's own non-empty inline `relationships[]`, in the real
    /// `sourceEnd`/`targetEnd` shape FCD #199/#200 gap 3 fixes (`role` is
    /// required on `sourceEnd`, optional on `targetEnd` --
    /// `agent-ix-semantic-ir`'s own `relationshipSourceEnd`/
    /// `relationshipTargetEnd` schemas -- so both are exercised here), is
    /// read into a [`RelationshipRecord`], not refused.
    ///
    /// This calls [`read_object_type`] directly rather than through
    /// [`read_records`]: at this crate's now-pinned `agent-ix-semantic-ir`
    /// rev the schema for this shape admits `sourceEnd`/`targetEnd`, so the
    /// full pipeline would work too (confirmed by
    /// `reading_fcd_199s_golden_shape_admits_the_schema_and_refuses_4_of_its_12_types`
    /// in `tests/model_intake.rs`, whose `Flow2` type reads clean this same
    /// way); this unit test still drives `read_object_type` directly so its
    /// own hand-rolled `sourceEnd`/`targetEnd` shape is pinned independently
    /// of any one fixture document. Same reason
    /// [`resolves_a_known_native_type_ref`] drives `read_value_type_ref`
    /// directly instead of through the full pipeline.
    #[trace("TC-145", "FR-056-AC-5")]
    #[test]
    fn reads_an_inline_relationship_with_the_real_source_end_and_target_end_shape() {
        let type_value = serde_json::json!({
            "identity": "ix://acme/orders/Flow2",
            "supertypes": ["ix://acme/orders/Flow"],
            "fields": [],
            "operations": [],
            "featureOrder": [],
            "relationships": [{
                "identity": "ix://acme/orders/Flow2/specializes",
                "category": "structural",
                "composite": false,
                "direction": "source-to-target",
                "origin": {"source": {"sourceIdentity": "ix://acme/orders/spec", "startLine": 1, "startColumn": 1}},
                "sourceEnd": {
                    "role": "specializes",
                    "type": "ix://acme/orders/Flow2",
                    "multiplicity": {"lower": 0, "ordered": false, "unique": false},
                },
                "targetEnd": {
                    "type": "ix://acme/orders/Flow",
                    "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
                },
            }],
        });
        let mut records = Vec::new();
        read_object_type(
            "acme/orders",
            &type_value,
            "ix://acme/orders/Flow2",
            "$.types[0]",
            true,
            &mut records,
            &mut Vec::new(),
        )
        .expect("the real sourceEnd/targetEnd relationship shape reads, it is not refused");

        let relationship = records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::Relationship(relationship) => Some(relationship),
                _ => None,
            })
            .expect("read_object_type appends one Relationship record per declared relationship");

        assert_eq!(
            relationship.key,
            declaration_key("acme/orders", "ix://acme/orders/Flow2/specializes")
        );
        assert_eq!(
            relationship.direction,
            RelationshipDirection::SourceToTarget
        );
        assert_eq!(relationship.source.role.as_deref(), Some("specializes"));
        assert_eq!(
            relationship.source.type_identity,
            declaration_key("acme/orders", "ix://acme/orders/Flow2")
        );
        assert_eq!(relationship.source.multiplicity.upper, None);
        // `targetEnd.role` is absent here: `relationshipTargetEnd`'s own
        // schema does not require it, and this reader preserves that as
        // `None`, not a fabricated default.
        assert_eq!(relationship.target.role, None);
        assert_eq!(
            relationship.target.type_identity,
            declaration_key("acme/orders", "ix://acme/orders/Flow")
        );
        assert_eq!(relationship.target.multiplicity.upper, Some(1));
    }

    /// `direction` on this shape is source-to-target/target-to-source/
    /// bidirectional/undirected -- the full vocabulary
    /// [`RelationshipDirection`] already carries for [`read_connection`]'s
    /// `flowDirection` (which never admits `undirected`) -- so `undirected`
    /// is exercised here specifically, on this shape's own `direction`
    /// member, never `flowDirection`.
    #[trace("TC-145", "FR-056-AC-5")]
    #[test]
    fn reads_an_undirected_inline_relationship() {
        let type_value = serde_json::json!({
            "identity": "ix://acme/orders/Widget",
            "supertypes": [],
            "fields": [],
            "operations": [],
            "relationships": [{
                "identity": "ix://acme/orders/Widget/peer_link",
                "category": "structural",
                "composite": false,
                "direction": "undirected",
                "origin": {"source": {"sourceIdentity": "ix://acme/orders/spec", "startLine": 1, "startColumn": 1}},
                "sourceEnd": {
                    "role": "peer",
                    "type": "ix://acme/orders/Widget",
                    "multiplicity": {"lower": 0, "ordered": false, "unique": false},
                },
                "targetEnd": {
                    "role": "peer",
                    "type": "ix://acme/orders/Widget",
                    "multiplicity": {"lower": 0, "ordered": false, "unique": false},
                },
            }],
        });
        let mut records = Vec::new();
        read_object_type(
            "acme/orders",
            &type_value,
            "ix://acme/orders/Widget",
            "$.types[0]",
            false,
            &mut records,
            &mut Vec::new(),
        )
        .expect("undirected is a real member of RelationshipDirection, not refused");

        let relationship = records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::Relationship(relationship) => Some(relationship),
                _ => None,
            })
            .expect("read_object_type appends one Relationship record per declared relationship");
        assert_eq!(relationship.direction, RelationshipDirection::Undirected);
    }

    /// H1 (PR #200 review): FCD's own identity form for a relationship
    /// member -- `ix://<pkg>/relationship/<Owner>-<verb>-<Other>`, an extra
    /// `relationship` segment and `-` separators, never QSpec's
    /// `<owner>/<name>` -- must refuse like any other member, exactly the
    /// same rule [`read_field_member`]/[`read_operation_member`] apply
    /// through [`member_identity_name`]. Before this test, `read_relationship`
    /// applied no identity check at all and took this form verbatim as the
    /// record's key; this is the regression test for that gap.
    #[trace("TC-145", "FR-056-AC-5")]
    #[test]
    fn refuses_a_relationship_member_whose_identity_is_not_owner_slash_name() {
        let type_value = serde_json::json!({
            "identity": "ix://acme/orders/Flow2",
            "supertypes": [],
            "fields": [],
            "operations": [],
            "relationships": [{
                "identity": "ix://acme/orders/relationship/Flow2-specializes-Flow",
                "category": "structural",
                "composite": false,
                "direction": "source-to-target",
                "origin": {"source": {"sourceIdentity": "ix://acme/orders/spec", "startLine": 1, "startColumn": 1}},
                "sourceEnd": {
                    "role": "specializes",
                    "type": "ix://acme/orders/Flow2",
                    "multiplicity": {"lower": 0, "ordered": false, "unique": false},
                },
                "targetEnd": {
                    "type": "ix://acme/orders/Flow",
                    "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
                },
            }],
        });
        let mut records = Vec::new();
        let refusal = read_object_type(
            "acme/orders",
            &type_value,
            "ix://acme/orders/Flow2",
            "$.types[0]",
            false,
            &mut records,
            &mut Vec::new(),
        )
        .expect_err(
            "FCD's own relationship identity form does not parse as owner/<name> and refuses",
        );
        match &refusal.cause {
            ModelRefusalCause::IntakeMalformedDeclaration { node, .. } => {
                assert_eq!(node, "ix://acme/orders/relationship/Flow2-specializes-Flow");
            }
            other => panic!("expected IntakeMalformedDeclaration, got {other:?}"),
        }
        assert!(
            refusal
                .detail
                .contains("is not ix://acme/orders/Flow2/<name>"),
            "{}",
            refusal.detail
        );
    }

    /// H1 (PR #200 review), FR-056-AC-5's other half: "a relationship
    /// member with no ... source span refuses `malformed-declaration`".
    #[trace("TC-145", "FR-056-AC-5")]
    #[test]
    fn refuses_a_relationship_member_with_no_source_span() {
        let type_value = serde_json::json!({
            "identity": "ix://acme/orders/Flow2",
            "supertypes": [],
            "fields": [],
            "operations": [],
            "relationships": [{
                "identity": "ix://acme/orders/Flow2/specializes",
                "category": "structural",
                "composite": false,
                "direction": "source-to-target",
                "sourceEnd": {
                    "role": "specializes",
                    "type": "ix://acme/orders/Flow2",
                    "multiplicity": {"lower": 0, "ordered": false, "unique": false},
                },
                "targetEnd": {
                    "type": "ix://acme/orders/Flow",
                    "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
                },
            }],
        });
        let mut records = Vec::new();
        let refusal = read_object_type(
            "acme/orders",
            &type_value,
            "ix://acme/orders/Flow2",
            "$.types[0]",
            false,
            &mut records,
            &mut Vec::new(),
        )
        .expect_err("a relationship member with no origin.source refuses per FR-056-AC-5");
        assert!(
            refusal.detail.contains("origin.source: missing"),
            "{}",
            refusal.detail
        );
    }

    /// R2-5 (PR #200 review round 2): `read_relationship`'s own `direction`
    /// match is the fourth panic-to-refusal conversion H3 made (the review's
    /// brief named three -- `read_endpoint`'s `direction`, `read_connection`'s
    /// `flowDirection`, `read_population`'s `extent` -- but this fourth one
    /// changed too, and had no regression test of its own).
    #[test]
    fn refuses_an_unrecognized_relationship_direction_instead_of_panicking() {
        let type_value = serde_json::json!({
            "identity": "ix://acme/orders/Flow2",
            "supertypes": [],
            "fields": [],
            "operations": [],
            "relationships": [{
                "identity": "ix://acme/orders/Flow2/specializes",
                "category": "structural",
                "composite": false,
                "direction": "sideways",
                "origin": {"source": {"sourceIdentity": "ix://acme/orders/spec", "startLine": 1, "startColumn": 1}},
                "sourceEnd": {
                    "role": "specializes",
                    "type": "ix://acme/orders/Flow2",
                    "multiplicity": {"lower": 0, "ordered": false, "unique": false},
                },
                "targetEnd": {
                    "type": "ix://acme/orders/Flow",
                    "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
                },
            }],
        });
        let mut records = Vec::new();
        let refusal = read_object_type(
            "acme/orders",
            &type_value,
            "ix://acme/orders/Flow2",
            "$.types[0]",
            false,
            &mut records,
            &mut Vec::new(),
        )
        .expect_err("an unrecognized relationship direction refuses rather than panicking");
        assert!(
            refusal.detail.contains("sideways")
                && refusal
                    .detail
                    .contains("is not a direction this reader recognizes"),
            "{}",
            refusal.detail
        );
    }

    /// A real FR-208 meaning (`EVENT_TYPE`) FCD's own schema accepts
    /// generically -- any string in `kind`'s resolved `meaning` is
    /// schema-valid -- but [`read_type_node`]'s dispatch has no reader for
    /// yet, refuses as a known-but-unsupported declaration form rather than
    /// silently folding into `ObjectTypeRecord`.
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn refuses_a_known_but_unsupported_fr208_meaning_as_unsupported() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "happened",
                meaning::EVENT_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([wire_type(
                "ix://acme/orders/OrderPlaced",
                serde_json::json!({"module": "acme/orders", "name": "happened"}),
                serde_json::json!({}),
            )]),
        )
        .to_string()
        .into_bytes();
        let refusals = read_records("acme/orders", &parse_document(&document)).expect_err(
            "a real FR-208 meaning with no QSL record shape yet refuses, not folds into \
             ObjectTypeRecord",
        );
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::UnsupportedConstruct,
                cause: ModelRefusalCause::UnsupportedDeclarationForm {
                    node: "ix://acme/orders/OrderPlaced".to_owned(),
                    what: meaning::EVENT_TYPE.to_owned(),
                },
                detail: format!(
                    "$.types[0]: construct meaning/capability {:?} has no reader yet",
                    meaning::EVENT_TYPE
                ),
            }]
        );
    }

    fn money_field(name: &str, type_ref: &str) -> Value {
        serde_json::json!({
            "identity": format!("ix://acme/orders/Money/{name}"),
            "name": name,
            "typeRef": type_ref,
            "presence": "required",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
        })
    }

    fn money(extra: Value) -> Value {
        let mut node = serde_json::json!({
            "identity": "ix://acme/orders/Money",
            "fields": [
                money_field("amount_minor", "ix://quire/native/Integer"),
                money_field("paid", "ix://quire/native/Boolean"),
            ],
        });
        if let (Some(node), Some(extra)) = (node.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                node.insert(key.clone(), value.clone());
            }
        }
        node
    }

    fn read_money(extra: Value) -> Result<Vec<DomainPackageRecord>, ModelRefusal> {
        let mut records = Vec::new();
        read_record_value_type(
            "acme/orders",
            &money(extra),
            "ix://acme/orders/Money",
            "$.types[0]",
            &HashMap::from([
                ("ix://acme/orders/Money", meaning::RECORD_VALUE_TYPE),
                ("ix://acme/orders/Base", meaning::RECORD_VALUE_TYPE),
                ("ix://acme/orders/Order", meaning::OBJECT_TYPE),
                ("ix://acme/orders/Placed", meaning::EVENT_TYPE),
            ]),
            &mut records,
        )
        .map(|()| records)
    }

    /// A record value type (FR-208-AC-4) reads as its own record kind, never
    /// an `ObjectTypeRecord`, followed by one field member per field, each
    /// owned by the record value type.
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn reads_a_record_value_type_and_its_fields() {
        let key = |node: &str| declaration_key("acme/orders", node);
        let records = read_money(serde_json::json!({})).expect("Money reads");
        assert_eq!(records.len(), 3);
        assert_eq!(
            records[0],
            DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
                key: key("ix://acme/orders/Money"),
            })
        );
        for (record, (name, native)) in records[1..].iter().zip([
            ("amount_minor", NativeValueType::Integer),
            ("paid", NativeValueType::Boolean),
        ]) {
            let DomainPackageRecord::FieldMember(field) = record else {
                panic!("a field member follows its record value type")
            };
            assert_eq!(field.key, key(&format!("ix://acme/orders/Money/{name}")));
            assert_eq!(field.owner, key("ix://acme/orders/Money"));
            assert_eq!(field.value_type, ValueTypeRef::Native(native));
        }
    }

    /// A record value type with an identity field, an operation, a
    /// supertype, an inline relationship or no field is not valid for its
    /// meaning: each refuses with FR-208-AC-4/AC-12's cause, naming the node
    /// (the operation for an operation), and reads nothing. A supertype of
    /// another meaning (an object type, an event type) refuses
    /// `malformed-declaration` at that entry; a record value type supertype
    /// is a generalization this reader does not read yet
    /// (`declaration-form`); one naming no type is a dangling reference.
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn refuses_a_record_value_type_its_meaning_does_not_admit() {
        let operation = serde_json::json!({"identity": "ix://acme/orders/Money/add"});
        for (extra, code, node, detail) in [
            (
                serde_json::json!({"identityFields": ["ix://acme/orders/Money/paid"]}),
                Code::InvalidModelBinding,
                "ix://acme/orders/Money",
                "$.types[0]: identityFields",
            ),
            (
                serde_json::json!({"operations": [operation]}),
                Code::UnsupportedConstruct,
                "ix://acme/orders/Money/add",
                "$.types[0].operations[0]:",
            ),
            (
                serde_json::json!({"supertypes": ["ix://acme/orders/Base"]}),
                Code::UnsupportedConstruct,
                "ix://acme/orders/Money",
                "$.types[0]:",
            ),
            (
                serde_json::json!({"supertypes": ["ix://acme/orders/Order"]}),
                Code::InvalidModelBinding,
                "ix://acme/orders/Money",
                "$.types[0].supertypes[0]:",
            ),
            (
                serde_json::json!({"supertypes": ["ix://acme/orders/Base", "ix://acme/orders/Placed"]}),
                Code::InvalidModelBinding,
                "ix://acme/orders/Money",
                "$.types[0].supertypes[1]:",
            ),
            (
                serde_json::json!({"supertypes": ["ix://acme/orders/Placed"]}),
                Code::InvalidModelBinding,
                "ix://acme/orders/Money",
                "$.types[0].supertypes[0]:",
            ),
            (
                serde_json::json!({"supertypes": ["ix://acme/orders/Ghost"]}),
                Code::DanglingReference,
                "ix://acme/orders/Money",
                "$.types[0].supertypes[0]:",
            ),
            (
                serde_json::json!({"relationships": [{"identity": "ix://acme/orders/Money/owner"}]}),
                Code::InvalidModelBinding,
                "ix://acme/orders/Money",
                "$.types[0].relationships[0]:",
            ),
            (
                serde_json::json!({"fields": []}),
                Code::InvalidModelBinding,
                "ix://acme/orders/Money",
                "$.types[0]: fields",
            ),
        ] {
            let refusal = read_money(extra.clone()).expect_err("Money refuses");
            assert_eq!(refusal.code, code, "{extra}");
            let named = match &refusal.cause {
                ModelRefusalCause::IntakeMalformedDeclaration { node, .. }
                | ModelRefusalCause::UnsupportedDeclarationForm { node, .. } => node,
                ModelRefusalCause::UnknownGeneral { supertype, .. } => &supertype.node,
                other => panic!("{extra}: {other:?}"),
            };
            assert_eq!(named, node, "{extra}");
            assert!(
                refusal.detail.starts_with(detail),
                "{extra}: {}",
                refusal.detail
            );
        }
    }

    /// The wire shape TC-458's fixture declares for `VersionNumber`: an
    /// integer scalar bound `0..=1000`.
    fn version_number(extra: Value) -> Value {
        let mut node = serde_json::json!({
            "identity": "ix://acme/orders/VersionNumber",
            "scalar": "integer",
            "constraints": [
                {"keyword": "min", "operands": {"value": 0}},
                {"keyword": "max", "operands": {"value": 1000}},
            ],
        });
        if let (Some(node), Some(extra)) = (node.as_object_mut(), extra.as_object()) {
            for (key, value) in extra {
                node.insert(key.clone(), value.clone());
            }
        }
        node
    }

    fn read_version_number(extra: Value) -> Result<ScalarTypeRecord, ModelRefusal> {
        read_value_type(
            "acme/orders",
            &version_number(extra),
            "ix://acme/orders/VersionNumber",
            "$.types[0]",
        )
    }

    /// FR-056's `value-type/v1` scalar reader admits a bound
    /// integer value type, reading its `min`/`max` constraints structurally
    /// into a [`ScalarTypeRecord`] -- never by parsing a rendered
    /// `Int[lo,hi]` string.
    #[trace("TC-458", "FR-103-AC-1")]
    #[test]
    fn reads_a_bound_integer_value_type() {
        let record = read_version_number(serde_json::json!({})).expect("VersionNumber reads");
        assert_eq!(
            record,
            ScalarTypeRecord {
                key: declaration_key("acme/orders", "ix://acme/orders/VersionNumber"),
                lower: 0,
                upper: 1000,
            }
        );
    }

    /// FR-056-AC-16 (TC-911 step 1): a bound beyond +/-2^53 arrives as a
    /// canonical decimal string and reads exactly, up to the i128 limits; a
    /// JSON integer bound still reads.
    #[trace("TC-911", "FR-056-AC-16")]
    #[test]
    fn reads_decimal_string_bounds_up_to_i128() {
        for (upper, expected) in [
            (
                serde_json::json!("18446744073709551615"),
                i128::from(u64::MAX),
            ),
            (
                serde_json::json!("9223372036854775808"),
                9_223_372_036_854_775_808,
            ),
            (
                serde_json::json!("170141183460469231731687303715884105727"),
                i128::MAX,
            ),
            (serde_json::json!(1000), 1000),
        ] {
            let record = read_version_number(serde_json::json!({"constraints": [
                {"keyword": "min", "operands": {"value": 0}},
                {"keyword": "max", "operands": {"value": upper}},
            ]}))
            .unwrap_or_else(|refusal| panic!("{upper}: {refusal:?}"));
            assert_eq!(record.upper, expected, "{upper}");
        }
        let record = read_version_number(serde_json::json!({"constraints": [
            {"keyword": "min", "operands": {"value": "-170141183460469231731687303715884105728"}},
            {"keyword": "max", "operands": {"value": 0}},
        ]}))
        .expect("i128::MIN reads");
        assert_eq!(record.lower, i128::MIN);
    }

    /// Every malformed or unsupported bound-scalar wire shape this reader
    /// refuses -- structurally, from the constraint's own typed `keyword`/
    /// `operands.value` fields, never from a rendered `Int[lo,hi]` string.
    /// Every row asserts the reader's own full detail text, not just its
    /// location prefix, so a reader that ignores the actual scalar keyword,
    /// the actual failing constraint keyword, or one side of a half-bounded
    /// domain could not pass by accident.
    #[trace("TC-458", "FR-103-AC-1")]
    #[test]
    fn refuses_a_malformed_or_unsupported_bound_scalar() {
        for (extra, code, detail) in [
            (
                serde_json::json!({"scalar": "string"}),
                Code::UnsupportedConstruct,
                "$.types[0]: construct meaning/capability \
                 \"quire.meaning.model.value-type/v1:scalar=string\" has no reader yet",
            ),
            (
                serde_json::json!({"constraints": []}),
                Code::InvalidModelBinding,
                "$.types[0]: constraints: an integer value type requires both a min and a max \
                 bound",
            ),
            (
                serde_json::json!({"constraints": [{"keyword": "min", "operands": {"value": 0}}]}),
                Code::UnsupportedConstruct,
                "$.types[0]: construct meaning/capability \
                 \"quire.meaning.model.value-type/v1:constraints:half-bounded(min)\" has no \
                 reader yet",
            ),
            (
                serde_json::json!({"constraints": [{"keyword": "max", "operands": {"value": 1000}}]}),
                Code::UnsupportedConstruct,
                "$.types[0]: construct meaning/capability \
                 \"quire.meaning.model.value-type/v1:constraints:half-bounded(max)\" has no \
                 reader yet",
            ),
            (
                serde_json::json!({"constraints": [
                    {"keyword": "min", "operands": {"value": 1000}},
                    {"keyword": "max", "operands": {"value": 0}},
                ]}),
                Code::InvalidModelBinding,
                "$.types[0]: constraints: lower bound 1000 is greater than upper bound 0",
            ),
            (
                serde_json::json!({"constraints": [
                    {"keyword": "min", "operands": {"value": 0}},
                    {"keyword": "min", "operands": {"value": 1}},
                    {"keyword": "max", "operands": {"value": 1000}},
                ]}),
                Code::InvalidModelBinding,
                "$.types[0].constraints[1]: keyword: \"min\" is declared twice",
            ),
            (
                serde_json::json!({"constraints": [
                    {"keyword": "min", "operands": {"value": 0}},
                    {"keyword": "exclusiveMax", "operands": {"value": 1000}},
                ]}),
                Code::UnsupportedConstruct,
                "$.types[0].constraints[1]: construct meaning/capability \
                 \"quire.meaning.model.value-type/v1:constraints:exclusiveMax\" has no reader yet",
            ),
            (
                serde_json::json!({"constraints": [
                    {"keyword": "min", "operands": {"value": "zero"}},
                    {"keyword": "max", "operands": {"value": 1000}},
                ]}),
                Code::InvalidModelBinding,
                "$.types[0].constraints[0]: operands.value: \"zero\" for keyword \"min\" is not a \
                 canonical decimal integer in i128::MIN..=i128::MAX",
            ),
            (
                serde_json::json!({"constraints": [
                    {"keyword": "min", "operands": {"value": true}},
                    {"keyword": "max", "operands": {"value": 1000}},
                ]}),
                Code::InvalidModelBinding,
                "$.types[0].constraints[0]: operands.value: missing or not an integer",
            ),
            (
                serde_json::json!({"constraints": [
                    {"keyword": "min", "operands": {"value": 0}},
                    {"keyword": "max", "operands": {"value": "170141183460469231731687303715884105728"}},
                ]}),
                Code::InvalidModelBinding,
                "$.types[0].constraints[1]: operands.value: \
                 \"170141183460469231731687303715884105728\" for keyword \"max\" is not a \
                 canonical decimal integer in i128::MIN..=i128::MAX",
            ),
            (
                serde_json::json!({"constraints": [
                    {"keyword": "min", "operands": {"value": 0}},
                    {"keyword": "max", "operands": {"value": "0018"}},
                ]}),
                Code::InvalidModelBinding,
                "$.types[0].constraints[1]: operands.value: \"0018\" for keyword \"max\" is not \
                 a canonical decimal integer in i128::MIN..=i128::MAX",
            ),
            (
                serde_json::json!({"operations": [{"identity": "ix://acme/orders/VersionNumber/inc"}]}),
                Code::UnsupportedConstruct,
                "$.types[0].operations[0]: construct meaning/capability \
                 \"quire.meaning.model.value-type/v1:operations\" has no reader yet",
            ),
            (
                serde_json::json!({"relationships": [{"identity": "ix://acme/orders/VersionNumber/rel"}]}),
                Code::InvalidModelBinding,
                "$.types[0].relationships[0]: a value type owns no relationship",
            ),
            (
                serde_json::json!({"supertypes": ["ix://acme/orders/Other"]}),
                Code::InvalidModelBinding,
                "$.types[0].supertypes[0]: a value type declares no supertype",
            ),
        ] {
            let refusal = read_version_number(extra.clone()).expect_err("VersionNumber refuses");
            assert_eq!(refusal.code, code, "{extra}");
            assert_eq!(refusal.detail, detail, "{extra}");
        }
    }

    /// A record-shaped `VALUE_TYPE` node (a non-empty `fields`, FCD's own
    /// `value_object`/`event` forms at the pinned rev) is not this reader's
    /// scalar shape at all: [`read_type_node`]'s own dispatch (not
    /// [`read_value_type`], which never sees this node) falls it through to
    /// the generic known-but-unsupported bucket -- the same refusal it got
    /// before [`read_value_type`] existed, never the scalar reader's own
    /// malformed/missing-declaration checks.
    #[trace("TC-458", "FR-103-AC-1")]
    #[test]
    fn refuses_a_record_shaped_value_type_as_unsupported_not_malformed() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "value_type",
                meaning::VALUE_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([wire_type(
                "ix://acme/orders/Money",
                serde_json::json!({"module": "acme/orders", "name": "value_type"}),
                serde_json::json!({
                    "scalar": "integer",
                    "fields": [wire_field(
                        "ix://acme/orders/Money/amount_minor",
                        "amount_minor",
                        "ix://quire/native/Integer",
                    )],
                }),
            )]),
        )
        .to_string()
        .into_bytes();
        let refusals = read_records("acme/orders", &parse_document(&document))
            .expect_err("a record-shaped value type refuses, never admits as a scalar");
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::UnsupportedConstruct,
                cause: ModelRefusalCause::UnsupportedDeclarationForm {
                    node: "ix://acme/orders/Money".to_owned(),
                    what: meaning::VALUE_TYPE.to_owned(),
                },
                detail: format!(
                    "$.types[0]: construct meaning/capability {:?} has no reader yet",
                    meaning::VALUE_TYPE
                ),
            }]
        );
    }

    /// [`read_type_node`]'s own dispatch arm for `VALUE_TYPE` -- not just
    /// [`read_value_type`] called directly -- resolves a `types[]` entry
    /// into a [`DomainPackageRecord::ScalarType`] through the real
    /// `constructs[]`-meaning lookup [`read_records`] exercises end to end.
    /// Deleting the dispatch arm (while leaving [`read_value_type`] itself
    /// untouched) would leave every other test in this module green; this
    /// one would not.
    #[trace("TC-458", "FR-103-AC-1")]
    #[test]
    fn reads_a_bound_integer_value_type_through_the_real_dispatch() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "value_type",
                meaning::VALUE_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([wire_type(
                "ix://acme/orders/VersionNumber",
                serde_json::json!({"module": "acme/orders", "name": "value_type"}),
                serde_json::json!({
                    "scalar": "integer",
                    // `agent-ix-semantic-ir`'s own `CONSTRAINT_MEMBERS`
                    // requires every constraint's `identity`/`appliesTo`/
                    // `diagnosticCode`/`origin` present, since `read_records`
                    // (unlike `read_value_type` called directly) runs the
                    // full `validate_with_semantic_ir` schema/rules check
                    // first. `appliesTo` names the native scalar the value
                    // type binds (`ix://quire/native/Integer`), not
                    // `VersionNumber`'s own identity -- see
                    // `model_operations.rs`'s own matching fixture for why.
                    "constraints": [
                        {
                            "identity": "ix://acme/orders/VersionNumber/constraints/min",
                            "keyword": "min",
                            "operands": {"value": 0},
                            "appliesTo": "ix://quire/native/Integer",
                            "diagnosticCode": "bound.min",
                            "origin": {"generated": {
                                "generatorIdentity": "ix://acme/orders/VersionNumber",
                                "generatorVersion": "1.0.0",
                                "inputIdentities": ["ix://acme/orders/VersionNumber"],
                            }},
                        },
                        {
                            "identity": "ix://acme/orders/VersionNumber/constraints/max",
                            "keyword": "max",
                            "operands": {"value": 1000},
                            "appliesTo": "ix://quire/native/Integer",
                            "diagnosticCode": "bound.max",
                            "origin": {"generated": {
                                "generatorIdentity": "ix://acme/orders/VersionNumber",
                                "generatorVersion": "1.0.0",
                                "inputIdentities": ["ix://acme/orders/VersionNumber"],
                            }},
                        },
                    ],
                }),
            )]),
        )
        .to_string()
        .into_bytes();
        let records = read_records("acme/orders", &parse_document(&document))
            .expect("VersionNumber admits through the real dispatch");
        assert_eq!(
            records,
            vec![DomainPackageRecord::ScalarType(ScalarTypeRecord {
                key: declaration_key("acme/orders", "ix://acme/orders/VersionNumber"),
                lower: 0,
                upper: 1000,
            })]
        );
    }

    /// The package document of FR-056-AC-16's `Wide` scalar type: `min` the
    /// JSON integer 0 and `max` the raw JSON text `max_json`. Also returns
    /// the RFC 6901 pointer of the `max` operand.
    fn wide_document(max_json: &str) -> (Vec<u8>, String) {
        const SENTINEL: &str = "@@max@@";
        let constraint = |keyword: &str, value: Value| {
            serde_json::json!({
                "identity": format!("ix://acme/orders/Wide/constraints/{keyword}"),
                "keyword": keyword,
                "operands": {"value": value},
                "appliesTo": "ix://quire/native/Integer",
                "diagnosticCode": format!("bound.{keyword}"),
                "origin": {"generated": {
                    "generatorIdentity": "ix://acme/orders/Wide",
                    "generatorVersion": "1.0.0",
                    "inputIdentities": ["ix://acme/orders/Wide"],
                }},
            })
        };
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "value_type",
                meaning::VALUE_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([wire_type(
                "ix://acme/orders/Wide",
                serde_json::json!({"module": "acme/orders", "name": "value_type"}),
                serde_json::json!({
                    "scalar": "integer",
                    "constraints": [
                        constraint("min", serde_json::json!(0)),
                        constraint("max", Value::String(SENTINEL.to_owned())),
                    ],
                }),
            )]),
        );
        let pointer = "/types/0/constraints/1/operands/value".to_owned();
        assert_eq!(
            document.pointer(&pointer),
            Some(&Value::String(SENTINEL.to_owned())),
            "the max operand sits where the pointer says"
        );
        let text = document
            .to_string()
            .replace(&format!("\"{SENTINEL}\""), max_json);
        (text.into_bytes(), pointer)
    }

    /// FR-056-AC-16 (TC-911 step 1 and 2), through the package's one parse:
    /// `Wide` with `max` the string `"18446744073709551615"` admits and reads
    /// as `[0, 18446744073709551615]`; `max` the JSON number
    /// `18446744073709551615` refuses `noncanonical_wire`/`inexact-integer`
    /// at that number's pointer.
    #[trace("TC-911", "FR-056-AC-16")]
    #[test]
    fn a_wide_bound_is_a_decimal_string_and_a_wide_json_number_is_inexact() {
        let (bytes, _) = wide_document("\"18446744073709551615\"");
        let records = read_records("acme/orders", &parse_document(&bytes))
            .expect("Wide with a string bound admits");
        assert_eq!(
            records,
            vec![DomainPackageRecord::ScalarType(ScalarTypeRecord {
                key: declaration_key("acme/orders", "ix://acme/orders/Wide"),
                lower: 0,
                upper: i128::from(u64::MAX),
            })]
        );

        let (bytes, pointer) = wide_document("18446744073709551615");
        let refusal =
            PackageDocument::parse(&bytes, qsl_foundation::IntakeLimits::default()).unwrap_err();
        assert_eq!(
            refusal,
            noncanonical_number(
                pointer.parse().unwrap(),
                Inexact::Integer,
                "18446744073709551615"
            )
        );
        assert_eq!(refusal.code, Code::NoncanonicalWire);
    }

    /// A construct's `meaning` is schema-free text to
    /// `agent-ix-semantic-ir` (`vocabulary.rs`'s `DECLARATION_REQUIRED`
    /// only requires it present and non-empty) -- FR-208's closed
    /// [`meaning::ALL`] list is entirely this reader's own vocabulary, so a
    /// meaning string outside it is schema-valid and reaches
    /// [`read_type_node`]'s final `other` arm as `malformed-declaration`,
    /// not `unsupported-construct`: an unrecognized meaning is not a known
    /// FR-208 shape this reader merely lacks a case for.
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn refuses_a_meaning_outside_fr208_as_malformed_declaration() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "gadget",
                "acme.custom.meaning/v1",
                serde_json::json!({}),
            )]),
            serde_json::json!([wire_type(
                "ix://acme/orders/Gadget",
                serde_json::json!({"module": "acme/orders", "name": "gadget"}),
                serde_json::json!({}),
            )]),
        )
        .to_string()
        .into_bytes();
        let refusals = read_records("acme/orders", &parse_document(&document))
            .expect_err("a meaning outside FR-208 is not a declared vocabulary at all");
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "ix://acme/orders/Gadget".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "$.types[0]: kind: resolves to \"acme.custom.meaning/v1\", outside \
                          FR-208"
                    .to_owned(),
            }]
        );
    }

    /// FR-056-AC-3's last clause (PR #200 review R2-4): "renaming a kind
    /// while keeping its meaning id changes no meaning or export." This
    /// reader dispatches purely on the construct's resolved `meaning`
    /// (FR-056-CON-3), never on `kind.name`/`kind.module`, so two documents
    /// whose only difference is the construct/type `kind` name (with the
    /// same `meaning::OBJECT_TYPE`) must read to the exact same record.
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn renaming_a_kind_while_keeping_its_meaning_id_changes_no_record() {
        let document_named = |kind_name: &str| {
            wire_envelope(
                "acme/orders",
                serde_json::json!([wire_construct(
                    "acme/orders",
                    kind_name,
                    meaning::OBJECT_TYPE,
                    serde_json::json!({}),
                )]),
                serde_json::json!([wire_type(
                    "ix://acme/orders/Order",
                    serde_json::json!({"module": "acme/orders", "name": kind_name}),
                    serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
                )]),
            )
            .to_string()
            .into_bytes()
        };
        let records_a = read_records("acme/orders", &parse_document(&document_named("order")))
            .expect("a real OBJECT_TYPE meaning always reads");
        let records_b = read_records(
            "acme/orders",
            &parse_document(&document_named("purchase_order")),
        )
        .expect("a real OBJECT_TYPE meaning always reads");
        assert_eq!(
            records_a, records_b,
            "renaming the kind while keeping the same meaning id must not change the record"
        );
    }

    /// `read_population`'s own "kind: resolves to {other}, not
    /// POPULATION" branch, the one `read_population` refusal that survives
    /// `validate_with_semantic_ir`'s gate -- `agent-ix-semantic-ir`'s own `population_schema`
    /// (`schema.rs`) checks a population's `kind` resolves to *some*
    /// constructs entry, never that its resolved meaning is specifically
    /// `POPULATION` -- unlike this same document's `extent`
    /// (`POPULATION_EXTENTS` is exactly `["closed", "open"]`, so a bad
    /// `extent` is always FCD's own refusal first) or a dangling `kind`
    /// (FCD's own "names no constructs entry" check, identical wording,
    /// fires first).
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn refuses_a_population_kind_resolving_to_a_non_population_meaning() {
        let mut document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "order",
                meaning::OBJECT_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([wire_type(
                "ix://acme/orders/Widget",
                serde_json::json!({"module": "acme/orders", "name": "order"}),
                serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
            )]),
        );
        document["populations"] = serde_json::json!([{
            "identity": "ix://acme/orders/Fleet",
            "displayName": "Fleet",
            "kind": {"module": "acme/orders", "name": "order"},
            "members": [],
            "extent": "closed",
            "origin": {
                "generated": {
                    "generatorIdentity": "ix://acme/orders/Fleet",
                    "generatorVersion": "1.0.0",
                    "inputIdentities": ["ix://acme/orders/Fleet"],
                }
            },
        }]);
        let document = document.to_string().into_bytes();
        let refusals = read_records("acme/orders", &parse_document(&document)).expect_err(
            "a population's kind names a real constructs entry, but that entry's own \
             meaning is object-type, not population",
        );
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "ix://acme/orders/Fleet".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: format!(
                    "$.populations[0]: kind: resolves to {:?}, not {:?}",
                    meaning::OBJECT_TYPE,
                    meaning::POPULATION
                ),
            }]
        );
    }

    /// `read_type_identity`'s own package-scoping check --
    /// `agent-ix-semantic-ir`'s `is_semantic_identity` (`diag.rs`) accepts
    /// any lowercase owner segment, never checking it against the
    /// document's own `package.identity`, so a type identity naming a
    /// different package's owner is schema-valid to FCD and reaches this
    /// reader's own check, distinct from `refuses_fcd_own_identity_form_as_malformed_declaration`'s
    /// same-package-wrong-segments case.
    #[trace("TC-145", "FR-056-AC-4")]
    #[test]
    fn refuses_a_type_identity_naming_a_different_package_as_malformed_declaration() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "order",
                meaning::OBJECT_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([wire_type(
                "ix://acme/other/Thing",
                serde_json::json!({"module": "acme/orders", "name": "order"}),
                serde_json::json!({}),
            )]),
        )
        .to_string()
        .into_bytes();
        let refusals = read_records("acme/orders", &parse_document(&document)).expect_err(
            "a type identity naming a different package's owner is not a node of acme/orders",
        );
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "ix://acme/other/Thing".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "$.types[0]: identity: \"ix://acme/other/Thing\" is not \
                          ix://acme/orders/<artifact id>"
                    .to_owned(),
            }]
        );
    }

    /// `read_field_member`'s own member-identity-form check
    /// (`member_identity_name`, model-complete.md's Identity row): a field
    /// member's identity is exactly `<owner identity>/<member name>` --
    /// `agent-ix-semantic-ir`'s own `is_semantic_identity` (`diag.rs`)
    /// accepts any lowercase-owner, near-arbitrary-name identity string, so
    /// a field identity that swaps the required `/` for a `-` reads as
    /// schema-valid to FCD and reaches this reader's own check.
    #[trace("TC-145", "FR-056-AC-4")]
    #[test]
    fn refuses_a_member_identity_not_shaped_owner_slash_name() {
        let document = document_with_one_field(wire_field(
            "ix://acme/orders/Widget-other",
            "other",
            "ix://acme/orders/OtherType",
        ));
        let refusals = read_records("acme/orders", &parse_document(&document)).expect_err(
            "a field identity that is not exactly <owner identity>/<member name> is malformed",
        );
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "ix://acme/orders/Widget-other".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "$.types[0].fields[0]: identity: \"ix://acme/orders/Widget-other\" \
                          is not ix://acme/orders/Widget/<name>"
                    .to_owned(),
            }]
        );
    }

    /// `read_value_type_ref`'s own package-scoping check: a `typeRef`
    /// naming a different package's node (not `ix://quire/native/<Name>`,
    /// not `ix://<this field's own package>/<artifact id>`) is schema-valid
    /// to `agent-ix-semantic-ir` (it only requires the reference to resolve
    /// to *some* declared identity, `UNRESOLVED_TYPE_REF`), so this reaches
    /// this reader's own check, distinct from
    /// `refuses_a_type_identity_naming_a_different_package_as_malformed_declaration`'s
    /// same check over a type-definition node's own `identity` rather than
    /// a `typeRef`.
    #[trace("TC-145", "FR-056-AC-4")]
    #[test]
    fn refuses_a_foreign_package_type_ref_as_malformed_declaration() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([wire_construct(
                "acme/orders",
                "order",
                meaning::OBJECT_TYPE,
                serde_json::json!({}),
            )]),
            serde_json::json!([
                wire_type(
                    "ix://acme/orders/Widget",
                    serde_json::json!({"module": "acme/orders", "name": "order"}),
                    serde_json::json!({
                        "supertypes": [],
                        "fields": [wire_field(
                            "ix://acme/orders/Widget/other",
                            "other",
                            "ix://acme/other/Thing",
                        )],
                        "operations": [],
                    }),
                ),
                // Declared, so `agent-ix-semantic-ir`'s own resolution
                // succeeds -- this reaches this reader's own package-scoping
                // check on the `typeRef`, not FCD's `UNRESOLVED_TYPE_REF`.
                wire_type(
                    "ix://acme/other/Thing",
                    serde_json::json!({"module": "acme/orders", "name": "order"}),
                    serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
                ),
            ]),
        )
        .to_string()
        .into_bytes();
        let refusals = read_records("acme/orders", &parse_document(&document)).expect_err(
            "a typeRef naming a different package's node is not this field's own package",
        );
        assert_eq!(
            refusals,
            vec![
                ModelRefusal {
                    code: Code::InvalidModelBinding,
                    cause: ModelRefusalCause::IntakeMalformedDeclaration {
                        node: "ix://acme/orders/Widget/other".to_owned(),
                        artifact: None,
                        span: None,
                    },
                    detail: "$.types[0].fields[0]: typeRef: \"ix://acme/other/Thing\" is not \
                              ix://quire/native/<Name> or ix://acme/orders/<artifact id>"
                        .to_owned(),
                },
                ModelRefusal {
                    code: Code::InvalidModelBinding,
                    cause: ModelRefusalCause::IntakeMalformedDeclaration {
                        node: "ix://acme/other/Thing".to_owned(),
                        artifact: None,
                        span: None,
                    },
                    detail: "$.types[1]: identity: \"ix://acme/other/Thing\" is not \
                              ix://acme/orders/<artifact id>"
                        .to_owned(),
                },
            ]
        );
    }

    /// `read_connection`'s own `sourceEnd`/`targetEnd` multiplicity
    /// requirement is stricter than `agent-ix-semantic-ir`'s own
    /// `connection_end` schema (`schema.rs`), which requires only `type`
    /// and treats `multiplicity` as optional -- so a connection end with no
    /// multiplicity at all is schema-valid to FCD and reaches this reader's
    /// own check.
    ///
    /// PR #200 review R2-4/R3-3: untagged, not re-traced -- this is
    /// `read_connection`'s own connection-end shape check, not AC-3's
    /// kind/meaning subject. `quire-specification`'s FR-152 does carry
    /// acceptance criteria for the systems binder's own multiplicity/
    /// direction/interface-type enforcement (FR-152-AC-4), but no
    /// `FR-152-AC` row exists in this repo's own matrices to tag against
    /// (FR-056-AC-8 cites FR-152 only end to end, not this connection-end
    /// wire-shape check), and FR-152-AC-4 covers the binder's own
    /// `check_connection` (`crate::model::systems`, verified by TC-197 in
    /// `quire-specification`), not this reader's wire-shape check. Same
    /// honest-untag choice as `resolves_a_known_native_type_ref`'s doc
    /// comment describes (F6).
    #[test]
    fn refuses_a_connection_end_with_no_multiplicity() {
        let document = wire_envelope(
            "acme/orders",
            serde_json::json!([
                wire_construct(
                    "acme/orders",
                    "order",
                    meaning::OBJECT_TYPE,
                    serde_json::json!({}),
                ),
                wire_construct(
                    "acme/orders",
                    "connection",
                    meaning::SYSTEMS_CONNECTION,
                    serde_json::json!({
                        "fields": "forbidden",
                        "flowDirection": "required",
                        "operations": "forbidden",
                        "sourceEnd": "required",
                        "targetEnd": "required",
                    }),
                ),
            ]),
            serde_json::json!([
                wire_type(
                    "ix://acme/orders/PumpOut",
                    serde_json::json!({"module": "acme/orders", "name": "order"}),
                    serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
                ),
                wire_type(
                    "ix://acme/orders/TankIn",
                    serde_json::json!({"module": "acme/orders", "name": "order"}),
                    serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
                ),
                wire_type(
                    "ix://acme/orders/Pipe",
                    serde_json::json!({"module": "acme/orders", "name": "connection"}),
                    serde_json::json!({
                        "sourceEnd": {"type": "ix://acme/orders/PumpOut"},
                        "targetEnd": {
                            "type": "ix://acme/orders/TankIn",
                            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
                        },
                        "flowDirection": "source-to-target",
                    }),
                ),
            ]),
        )
        .to_string()
        .into_bytes();
        let refusals = read_records("acme/orders", &parse_document(&document)).expect_err(
            "a connection end with no multiplicity at all is stricter than \
             agent-ix-semantic-ir's own optional check",
        );
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "ix://acme/orders/Pipe".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "$.types[2]: sourceEnd.multiplicity: missing multiplicity".to_owned(),
            }]
        );
    }

    /// H3 (PR #200 review): `agent-ix-semantic-ir`'s own `expect_enum`
    /// restricts a real port's `direction` to `in`/`out`/`inout` before this
    /// reader ever sees a document, so this exact branch is unreachable
    /// through `read_records` today -- exercised here by calling
    /// `read_endpoint` directly, bypassing that schema gate, the same way
    /// `refuses_uuid_as_malformed_declaration_r5_holds_until_plat_836` calls
    /// `read_value_type_ref` directly. Before this test, an unrecognized
    /// value here panicked (`agent-ix-semantic-ir guarantees direction is
    /// in/out/inout when present, got {other:?}`) rather than refusing, so
    /// a future FCD pin widening this enum would abort the process instead
    /// of degrading to a refusal.
    #[test]
    fn refuses_an_unrecognized_port_direction_instead_of_panicking() {
        let node = serde_json::json!({
            "owner": "ix://acme/orders/SysPump",
            "interfaceType": "ix://acme/orders/Flow",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
            "direction": "sideways",
        });
        let ctx = NodeCtx::new(&node, "$.types[0]");
        let refusal = read_endpoint("acme/orders", &ctx, "ix://acme/orders/PumpOut")
            .expect_err("an unrecognized direction refuses rather than panicking");
        assert!(
            refusal.detail.contains("sideways")
                && refusal
                    .detail
                    .contains("is not a direction this reader recognizes"),
            "{}",
            refusal.detail
        );
    }

    /// H3 (PR #200 review): same reasoning as
    /// [`refuses_an_unrecognized_port_direction_instead_of_panicking`], for
    /// [`read_connection`]'s `flowDirection`. Before this test, an
    /// unrecognized value here panicked rather than refusing.
    #[test]
    fn refuses_an_unrecognized_connection_flow_direction_instead_of_panicking() {
        let node = serde_json::json!({
            "sourceEnd": {
                "type": "ix://acme/orders/PumpOut",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            },
            "targetEnd": {
                "type": "ix://acme/orders/TankIn",
                "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
            },
            "flowDirection": "sideways",
        });
        let ctx = NodeCtx::new(&node, "$.types[0]");
        let refusal = read_connection("acme/orders", &ctx, "ix://acme/orders/Pipe")
            .expect_err("an unrecognized flowDirection refuses rather than panicking");
        assert!(
            refusal.detail.contains("sideways")
                && refusal
                    .detail
                    .contains("is not a direction this reader recognizes"),
            "{}",
            refusal.detail
        );
    }

    /// H3 (PR #200 review): same reasoning, for [`read_population`]'s
    /// `extent`. Before this test, an unrecognized value here panicked
    /// rather than refusing.
    #[test]
    fn refuses_an_unrecognized_population_extent_instead_of_panicking() {
        let node = serde_json::json!({
            "kind": {"module": "acme/orders", "name": "population"},
            "members": [],
            "extent": "sideways",
        });
        let ctx = NodeCtx::new(&node, "$.populations[0]");
        let mut meanings = HashMap::new();
        meanings.insert(
            ("acme/orders".to_owned(), "population".to_owned()),
            meaning::POPULATION.to_owned(),
        );
        let refusal = read_population("acme/orders", &ctx, "ix://acme/orders/AllPumps", &meanings)
            .expect_err("an unrecognized extent refuses rather than panicking");
        assert!(
            refusal.detail.contains("sideways")
                && refusal
                    .detail
                    .contains("closed/open, the only extents this reader recognizes"),
            "{}",
            refusal.detail
        );
    }

    /// A `field`'s own `presence` beyond `identity`/`typeRef`/`multiplicity`,
    /// filled with schema-valid stand-ins, so [`read_field_member`] reaches
    /// its own `presence` check.
    fn field_with_presence(presence: Option<Value>) -> Value {
        let mut field = serde_json::json!({
            "identity": "ix://acme/orders/Widget/code",
            "name": "code",
            "typeRef": "ix://quire/native/Integer",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": false},
        });
        if let (Some(object), Some(presence)) = (field.as_object_mut(), presence) {
            object.insert("presence".to_owned(), presence);
        }
        field
    }

    /// Every refusal here goes through [`NodeCtx::malformed`], so all three
    /// carry `invalid_model_binding`/`IntakeMalformedDeclaration`
    /// (FR-056-AC-10) -- the same shape as every other malformed field
    /// value, distinguished by `detail`.
    #[trace("TC-443", "FR-056-AC-10")]
    #[test]
    fn refuses_an_unrecognized_presence_value() {
        let field = field_with_presence(Some(serde_json::json!("sometimes")));
        let refusal = read_field_member(
            "acme/orders",
            "ix://acme/orders/Widget",
            &field,
            "$.types[0].fields[0]",
        )
        .expect_err("an unrecognized presence value refuses rather than admitting it");
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        assert!(matches!(
            refusal.cause,
            ModelRefusalCause::IntakeMalformedDeclaration { .. }
        ));
        assert!(
            refusal.detail.contains("sometimes")
                && refusal.detail.contains(
                    "is not required/optional, the only presences this reader recognizes"
                ),
            "{}",
            refusal.detail
        );
    }

    #[trace("TC-443", "FR-056-AC-10")]
    #[test]
    fn refuses_a_missing_presence_value() {
        let field = field_with_presence(None);
        let refusal = read_field_member(
            "acme/orders",
            "ix://acme/orders/Widget",
            &field,
            "$.types[0].fields[0]",
        )
        .expect_err("a missing presence refuses rather than admitting it");
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        assert!(matches!(
            refusal.cause,
            ModelRefusalCause::IntakeMalformedDeclaration { .. }
        ));
        assert!(
            refusal.detail.contains("presence: missing or not a string"),
            "{}",
            refusal.detail
        );
    }

    #[trace("TC-443", "FR-056-AC-10")]
    #[test]
    fn refuses_a_non_string_presence_value() {
        let field = field_with_presence(Some(serde_json::json!(true)));
        let refusal = read_field_member(
            "acme/orders",
            "ix://acme/orders/Widget",
            &field,
            "$.types[0].fields[0]",
        )
        .expect_err("a non-string presence refuses rather than admitting it");
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        assert!(matches!(
            refusal.cause,
            ModelRefusalCause::IntakeMalformedDeclaration { .. }
        ));
        assert!(
            refusal.detail.contains("presence: missing or not a string"),
            "{}",
            refusal.detail
        );
    }

    /// PR #200 review R2-4/R3-3: untagged, not re-traced -- see
    /// `resolves_a_known_native_type_ref`'s own doc comment (no local
    /// FR-151-AC/FR-152-AC row exists to tag against). This one drives
    /// `read_records` end to end rather than `read_value_type_ref` directly,
    /// but its subject is still typeRef resolution, not AC-1's per-node
    /// declaration production.
    #[test]
    fn resolves_a_package_type_ref_to_a_node_of_the_package() {
        let document = document_with_one_field(wire_field(
            "ix://acme/orders/Widget/other",
            "other",
            "ix://acme/orders/OtherType",
        ));
        let records = read_records("acme/orders", &parse_document(&document))
            .expect("a typeRef naming a node of the field's own package reads");
        let field = records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::FieldMember(field) => Some(field),
                _ => None,
            })
            .expect("the document declares exactly one field member");
        assert_eq!(
            field.value_type,
            ValueTypeRef::Package(DeclarationKey {
                package: "acme/orders".to_owned(),
                node: "ix://acme/orders/OtherType".to_owned(),
            }),
            "a package-scoped typeRef resolves under the field's own package, \
             exactly as its own identity string names"
        );
    }

    // Every shape the per-node reader once `expect`ed from the
    // pinned `agent-ix-semantic-ir` validator now refuses instead. Each test
    // below breaks one of those shapes in an otherwise clean document and
    // checks two layers. Through `read_records`, the validator or the reader
    // refuses with the stable `invalid_model_binding`/`malformed-declaration`
    // code and nothing panics. Through `read_nodes` alone, which is what a
    // pin bump that drops the validator's guarantee would leave, the reader's
    // own refusal names the node and the broken member.

    /// One object type (`Widget`) and one population (`Fleet`) of it, every
    /// member the validator requires present, reading clean.
    fn reader_base_document() -> Value {
        let mut document = wire_envelope(
            "acme/orders",
            serde_json::json!([
                wire_construct(
                    "acme/orders",
                    "order",
                    meaning::OBJECT_TYPE,
                    serde_json::json!({}),
                ),
                wire_construct(
                    "acme/orders",
                    "fleet",
                    meaning::POPULATION,
                    serde_json::json!({}),
                ),
            ]),
            serde_json::json!([wire_type(
                "ix://acme/orders/Widget",
                serde_json::json!({"module": "acme/orders", "name": "order"}),
                serde_json::json!({"supertypes": [], "fields": [], "operations": []}),
            )]),
        );
        document["populations"] = serde_json::json!([{
            "identity": "ix://acme/orders/Fleet",
            "displayName": "Fleet",
            "kind": {"module": "acme/orders", "name": "fleet"},
            "members": ["ix://acme/orders/Widget"],
            "extent": "closed",
            "origin": {
                "generated": {
                    "generatorIdentity": "ix://acme/orders/Fleet",
                    "generatorVersion": "1.0.0",
                    "inputIdentities": ["ix://acme/orders/Fleet"],
                }
            },
        }]);
        document
    }

    /// Breaks `reader_base_document` with `mutate`, then checks both layers
    /// (see the comment above `reader_base_document`).
    fn assert_refuses_without_panicking(mutate: impl FnOnce(&mut Value), expected: ModelRefusal) {
        let mut document = reader_base_document();
        mutate(&mut document);
        let package = parse_document(document.to_string().as_bytes());
        let refusals = read_records("acme/orders", &package)
            .expect_err("the broken shape refuses through read_records");
        assert!(!refusals.is_empty());
        for refusal in &refusals {
            assert_eq!(refusal.code, Code::InvalidModelBinding, "{refusal:?}");
            assert!(
                matches!(
                    refusal.cause,
                    ModelRefusalCause::IntakeMalformedDeclaration { .. }
                ),
                "{refusal:?}"
            );
        }
        assert_eq!(
            read_nodes("acme/orders", package.tree()),
            Err(vec![expected])
        );
    }

    /// The expected reader refusal for a generated-origin node: no artifact,
    /// no span.
    fn malformed(node: &str, detail: &str) -> ModelRefusal {
        malformed_declaration(node.to_owned(), None, None, detail.to_owned())
    }

    #[trace("TC-145", "FR-056-AC-1")]
    #[test]
    fn reader_base_document_reads_clean() {
        let package = parse_document(reader_base_document().to_string().as_bytes());
        let records = read_records("acme/orders", &package)
            .expect("the base document the refusal tests break reads clean");
        assert_eq!(records.len(), 2, "one object type and one population");
    }

    /// Was `identity_keys`'s `expect("... identity_list items are strings")`.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_non_string_identity_list_item() {
        assert_refuses_without_panicking(
            |document| document["types"][0]["supertypes"] = serde_json::json!([7]),
            malformed(
                "ix://acme/orders/Widget",
                "$.types[0]: supertypes[0]: not a string",
            ),
        );
    }

    /// Was `read_object_type`'s `expect("... abstract is a boolean when present")`.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_non_boolean_abstract_in_the_reader() {
        assert_refuses_without_panicking(
            |document| document["types"][0]["abstract"] = serde_json::json!("yes"),
            malformed(
                "ix://acme/orders/Widget",
                "$.types[0]: abstract: not a boolean",
            ),
        );
    }

    /// Was `read_population`'s `expect("... kind is present on a population")`.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_population_with_no_kind() {
        assert_refuses_without_panicking(
            |document| {
                document["populations"][0]
                    .as_object_mut()
                    .expect("the base population is an object")
                    .remove("kind");
            },
            malformed("ix://acme/orders/Fleet", "$.populations[0]: kind: missing"),
        );
    }

    /// Was `read_population`'s `expect("... kind.module is a string")`.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_population_kind_module_that_is_not_a_string() {
        assert_refuses_without_panicking(
            |document| document["populations"][0]["kind"]["module"] = serde_json::json!(7),
            malformed(
                "ix://acme/orders/Fleet",
                "$.populations[0]: kind.module: missing or not a string",
            ),
        );
    }

    /// Was `read_population`'s `expect("... kind.name is a string")`.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_population_kind_name_that_is_not_a_string() {
        assert_refuses_without_panicking(
            |document| document["populations"][0]["kind"]["name"] = serde_json::json!(7),
            malformed(
                "ix://acme/orders/Fleet",
                "$.populations[0]: kind.name: missing or not a string",
            ),
        );
    }

    /// Was `meaning_index`'s `expect("... constructs is present and an array")`.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_document_with_no_constructs_array() {
        assert_refuses_without_panicking(
            |document| document["constructs"] = serde_json::json!({}),
            malformed("$", "$: constructs: missing or not an array"),
        );
    }

    /// Was `meaning_index`'s `expect("... kind is present on a constructs entry")`.
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn refuses_a_constructs_entry_with_no_kind() {
        assert_refuses_without_panicking(
            |document| {
                document["constructs"][0]
                    .as_object_mut()
                    .expect("the base constructs entry is an object")
                    .remove("kind");
            },
            malformed("$.constructs[0]", "$.constructs[0]: kind: missing"),
        );
    }

    /// Was `meaning_index`'s `expect("... kind.module is a string")`.
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn refuses_a_constructs_kind_module_that_is_not_a_string() {
        assert_refuses_without_panicking(
            |document| document["constructs"][0]["kind"]["module"] = serde_json::json!(7),
            malformed(
                "$.constructs[0]",
                "$.constructs[0]: kind.module: missing or not a string",
            ),
        );
    }

    /// Was `meaning_index`'s `expect("... kind.name is a string")`.
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn refuses_a_constructs_kind_name_that_is_not_a_string() {
        assert_refuses_without_panicking(
            |document| document["constructs"][0]["kind"]["name"] = serde_json::json!(7),
            malformed(
                "$.constructs[0]",
                "$.constructs[0]: kind.name: missing or not a string",
            ),
        );
    }

    /// Was `meaning_index`'s `expect("... construct.meaning is a string")`.
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn refuses_a_construct_with_no_meaning_string() {
        assert_refuses_without_panicking(
            |document| document["constructs"][0]["construct"]["meaning"] = serde_json::json!(7),
            malformed(
                "$.constructs[0]",
                "$.constructs[0]: construct.meaning: missing or not a string",
            ),
        );
    }

    /// Was `read_records`'s `expect("... types is present and an array")`.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_document_with_no_types_array() {
        assert_refuses_without_panicking(
            |document| {
                document
                    .as_object_mut()
                    .expect("the base document is an object")
                    .remove("types");
            },
            malformed("$", "$: types: missing or not an array"),
        );
    }

    /// Was `read_records`'s `expect("... populations is an array when present")`.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_populations_member_that_is_not_an_array() {
        assert_refuses_without_panicking(
            |document| document["populations"] = serde_json::json!({}),
            malformed("$", "$: populations: not an array"),
        );
    }

    /// Was `read_records`'s re-parse `expect("validate_with_semantic_ir
    /// already confirmed these bytes parse as JSON ...")`. The two parsers
    /// disagreed on a number serde_json cannot represent: the validator's
    /// reader accepted `1e400`, and the re-parse panicked on it. Intake now
    /// reads once, through the shared reader, which refuses the number with
    /// its pointer and lexeme; intake refuses it `noncanonical_wire`, not as
    /// a malformed document.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_number_serde_json_cannot_represent_at_the_one_parse() {
        let text = reader_base_document().to_string().replacen(
            "\"extent\":\"closed\"",
            "\"extent\":\"closed\",\"weight\":1e400",
            1,
        );
        assert!(
            text.contains("1e400"),
            "the out-of-range number is in the document"
        );
        assert!(
            agent_ix_semantic_ir::json::parse(&text).is_ok(),
            "the validator's own reader accepts it, which is what made the re-parse panic"
        );
        let refusal =
            PackageDocument::parse(text.as_bytes(), qsl_foundation::IntakeLimits::default())
                .unwrap_err();
        assert_eq!(refusal.code, Code::NoncanonicalWire);
        assert!(
            matches!(
                &refusal.cause,
                ModelRefusalCause::NoncanonicalNumber {
                    inexact: Inexact::Integer,
                    document_pointer,
                } if document_pointer.as_str().ends_with("/weight")
            ),
            "{refusal:?}"
        );
    }

    /// A number with no finite double refuses at the one parse
    /// `noncanonical_wire`, decided from its lexeme: a whole value beyond
    /// ±2^53, exponent and decimal forms included, is `inexact-integer`;
    /// the underflow `1e-400` is `inexact-number`. Each carries its pointer
    /// at the top level and nested, and none refuses as malformed.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_number_with_no_finite_double_by_its_lexeme() {
        for number in ["1e400", "-1e400", "1.5E+400", "0.1e401", "123e1000000000"] {
            assert_noncanonical(
                &document_with_count(number),
                Inexact::Integer,
                "/package/count",
                number,
            );
            assert_noncanonical(number, Inexact::Integer, "", number);
        }
        for number in ["1e-400", "-1e-400"] {
            assert_noncanonical(
                &document_with_count(number),
                Inexact::Number,
                "/package/count",
                number,
            );
            assert_noncanonical(number, Inexact::Number, "", number);
        }
        assert_noncanonical(
            r#"{"a/b":[0,{"c~d":-1e400}],"z":1e-400}"#,
            Inexact::Integer,
            "/a~1b/1/c~0d",
            "-1e400",
        );
        // Overflow with a fraction is not whole; with a negative exponent that
        // brings it back to a whole 1e399 it is.
        let wide = format!("1{}.5", "0".repeat(400));
        assert_noncanonical(
            &document_with_count(&wide),
            Inexact::Number,
            "/package/count",
            &wide,
        );
        let wide_whole = format!("1{}e-1", "0".repeat(400));
        assert_noncanonical(
            &document_with_count(&wide_whole),
            Inexact::Integer,
            "/package/count",
            &wide_whole,
        );
        // The first reader fault decides: the number is reached before the
        // repeated name is detected at object close, and before the
        // truncation; a repeated name followed by an underflow is the
        // reader's malformed refusal.
        assert_noncanonical(
            r#"{"a":1,"a":2,"n":1e400}"#,
            Inexact::Integer,
            "/n",
            "1e400",
        );
        assert_noncanonical("[1e400", Inexact::Integer, "/0", "1e400");
        let repeated = PackageDocument::parse(
            br#"{"a":1,"a":2,"n":1e-400}"#,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert!(
            matches!(
                repeated.cause,
                ModelRefusalCause::IntakeMalformedDeclaration { .. }
            ),
            "{repeated:?}"
        );
        // The reader refuses the out-of-range number before any tree exists
        // to walk, so it is named ahead of an earlier inexact number.
        assert_noncanonical(r#"{"x":[1e-400,1e400]}"#, Inexact::Integer, "/x/1", "1e400");
    }

    /// FR-154 admission refuses a number with no finite double
    /// `noncanonical_wire` at its pointer under a digest that matches
    /// nothing, never `stale_dependency`.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn admission_refuses_a_number_with_no_finite_double_under_an_unrelated_digest() {
        let digest = [7_u8; 32];
        let offered = DomainPackageRef {
            identity: "acme/orders".to_owned(),
            version: "1".to_owned(),
            digest,
        };
        for (text, inexact, pointer) in [
            (
                document_with_count("1e400"),
                Inexact::Integer,
                "/package/count",
            ),
            (
                document_with_count("-1e-400"),
                Inexact::Number,
                "/package/count",
            ),
        ] {
            let bytes = BTreeMap::from([(digest, text.clone().into_bytes())]);
            let refusal = admit(
                &offered,
                SHA256_JCS_DIGEST_DOMAIN,
                &bytes,
                qsl_foundation::IntakeLimits::default(),
            )
            .unwrap_err();
            assert_eq!(refusal.code, Code::NoncanonicalWire, "{text}");
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::NoncanonicalNumber {
                    inexact,
                    document_pointer: pointer.parse().unwrap(),
                },
                "{text}"
            );
        }
    }

    /// The JCS digest check still runs over the one parse.
    /// A document nested past serde_json's default recursion limit (128)
    /// admits under its JCS digest, which the old `serde_json::from_slice`
    /// in `admit` could not parse and so digested raw.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn admits_a_deeply_nested_document_under_its_jcs_digest() {
        let mut nested = serde_json::json!(0);
        for _ in 0..150 {
            nested = serde_json::json!([nested]);
        }
        let document = serde_json::json!({
            "package": {"identity": "acme/orders", "version": "1"},
            "payload": nested,
        });
        let padded = format!("  {document}  ").into_bytes();
        assert!(serde_json::from_slice::<Value>(&padded).is_err());
        let digest = digest_of(&canonical(&document));
        let mut map = BTreeMap::new();
        map.insert(digest, padded);
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let (package_ref, admitted) = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .expect("a deeply nested document admits under its JCS digest");
        assert_eq!(package_ref.digest, digest);
        assert_eq!(admitted.tree(), &document);
    }

    // PR #379 review F1: a lone UTF-16 surrogate escape refuses at the one
    // parse. The reader would replace it with U+FFFD, so without this check
    // `"\ud800"`, `"\udc00"` and `"�"` would read as one tree and admit
    // under one JCS digest.

    /// `text` refuses at the one parse as a lone surrogate at the offset of
    /// `escape`'s first occurrence.
    fn assert_lone_surrogate_refused(text: &str, escape: &str) {
        let offset = text.find(escape).expect("the escape is in the text");
        assert_eq!(
            PackageDocument::parse(text.as_bytes(), qsl_foundation::IntakeLimits::default())
                .unwrap_err(),
            malformed_declaration(
                "$".to_owned(),
                None,
                None,
                format!(
                    "package document is malformed JSON at byte {offset}: lone surrogate escape"
                ),
            ),
            "{text}"
        );
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_lone_high_surrogate_escape() {
        assert_lone_surrogate_refused(r#"{"s":"\ud800"}"#, r"\ud800");
        assert_lone_surrogate_refused(r#"{"s":"a\uD800b"}"#, r"\uD800");
        // Followed by an escape that is not a low surrogate.
        assert_lone_surrogate_refused(r#"{"s":"\ud800A"}"#, r"\ud800");
        // Followed by a second high surrogate.
        assert_lone_surrogate_refused(r#"{"s":"\ud800𐀀"}"#, r"\ud800");
        // In a member name.
        assert_lone_surrogate_refused(r#"{"\udbff":1}"#, r"\udbff");
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_lone_low_surrogate_escape() {
        assert_lone_surrogate_refused(r#"{"s":"\udc00"}"#, r"\udc00");
        assert_lone_surrogate_refused(r#"["ok", "\uDFFF"]"#, r"\uDFFF");
    }

    /// TC-730 step 3: on a thread with a 512 KiB stack, a package document
    /// holding `"\udc00"` in a string is refused
    /// `invalid_model_binding`/`malformed-declaration` at `$`, carrying the
    /// escape's byte offset from the shared reader.
    #[trace("TC-730", "FR-260-AC-2")]
    #[test]
    fn tc_730_a_lone_low_surrogate_refuses_at_its_byte_offset() {
        let text = r#"{"package":{"identity":"acme/orders","version":"1"},"s":"a\udc00"}"#;
        let offset = text.find(r"\udc00").expect("the escape is in the text");
        let refusal = std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(move || {
                PackageDocument::parse(text.as_bytes(), qsl_foundation::IntakeLimits::default())
            })
            .expect("the test thread spawns")
            .join()
            .expect("the parse does not panic")
            .unwrap_err();
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        assert_eq!(
            refusal,
            malformed_declaration(
                "$".to_owned(),
                None,
                None,
                format!(
                    "package document is malformed JSON at byte {offset}: lone surrogate escape"
                ),
            )
        );
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_a_reversed_surrogate_pair() {
        assert_lone_surrogate_refused(r#"{"s":"\ude00\ud83d"}"#, r"\ude00");
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn admits_a_valid_surrogate_pair_and_an_escaped_backslash_before_u() {
        for text in [
            r#"{"s":"😀"}"#,
            r#"{"s":"😀 and 𐀀"}"#,
            // `\\` is one escaped backslash; the `ud800` after it is text.
            r#"{"s":"\\ud800"}"#,
            r#"{"s":"\"😀\""}"#,
        ] {
            let document =
                PackageDocument::parse(text.as_bytes(), qsl_foundation::IntakeLimits::default())
                    .unwrap_or_else(|refusal| panic!("{text} refused: {refusal:?}"));
            assert_eq!(
                document.tree(),
                &serde_json::from_str::<Value>(text).unwrap(),
                "{text}"
            );
        }
    }

    /// The reviewer's scenario: lone-surrogate bytes offered under the JCS
    /// digest of the U+FFFD document no longer admit.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn a_lone_surrogate_no_longer_admits_under_the_replacement_character_digest() {
        let replacement = serde_json::json!({
            "package": {"identity": "acme/orders", "version": "1"},
            "s": "\u{FFFD}",
        });
        let digest = digest_of(&canonical(&replacement));
        let lone = br#"{"package":{"identity":"acme/orders","version":"1"},"s":"\ud800"}"#.to_vec();
        let mut map = BTreeMap::new();
        map.insert(digest, lone.clone());
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let refusal = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert_eq!(refusal.code, Code::StaleDependency);
        assert_eq!(
            refusal.cause,
            ModelRefusalCause::ByteDigestMismatch {
                expected: digest,
                actual: digest_of(&lone),
            },
            "unparseable bytes digest raw, so they cannot match a JCS digest"
        );
    }

    // PR #379 review F2: a document over a parse limit that arrives with its
    // correct JCS digest refuses naming the limit, not as a digest mismatch.

    /// `depth` arrays nested around `inner`.
    fn nested_arrays(depth: usize, inner: &str) -> String {
        format!("{}{inner}{}", "[".repeat(depth), "]".repeat(depth))
    }

    /// FR-260-AC-5: intake has no depth limit. A document nested
    /// 100,000 deep reads, digests and drops on a
    /// 512 KiB thread; it is judged on its content, never refused as a depth.
    #[trace("TC-730", "FR-260-AC-5")]
    #[test]
    fn a_package_document_nested_100000_deep_is_read_not_refused_as_a_depth() {
        on_a_small_stack(|| {
            let depth = 100_000;
            let text = format!(
                r#"{{"package":{{"identity":"acme/orders","version":"1"}},"payload":{}0{}}}"#,
                "[".repeat(depth),
                "]".repeat(depth)
            );
            let document =
                PackageDocument::parse(text.as_bytes(), qsl_foundation::IntakeLimits::default())
                    .expect("a deep document reads");
            drop(document);
        });
    }

    /// FR-260-AC-4 (FR-056-AC-2): a document one byte over `intake.input_bytes`
    /// at bound `B` refuses naming the limit, `B`, `B + 1` and the setting;
    /// raised to `B + 1` through the builder and through the settings
    /// operation, the same document is judged on its content.
    #[trace("TC-145", "FR-056-AC-2")]
    #[trace("TC-732", "FR-260-AC-4")]
    #[test]
    fn refuses_an_oversize_document_with_its_correct_digest_as_a_size_limit() {
        use qsl_foundation::SettingLimits;
        let head = r#"{"package":{"identity":"acme/orders","version":"1"},"pad":""#;
        let tail = r#""}"#;
        let bound = 4096_u64;
        let pad = bound as usize + 1 - head.len() - tail.len();
        let bytes = format!("{head}{}{tail}", "x".repeat(pad)).into_bytes();
        assert_eq!(bytes.len() as u64, bound + 1);
        let digest = digest_of(&canonical(&serde_json::from_slice(&bytes).unwrap()));
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let at_bound = IntakeLimits::default().with_input_bytes(bound);
        let refusal = admit(&offered, &digest_domain, &map, at_bound).unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::ResourceExhausted,
                cause: ModelRefusalCause::IntakeLimitExceeded {
                    limit: IntakeLimit::InputBytes,
                    bound,
                    actual: bound + 1,
                },
                detail: format!("package document exceeds intake's input_bytes limit of {bound}"),
            }
        );
        let outcome = refusal.cause.limit_exceeded().expect("a limit outcome");
        assert_eq!(outcome.setting(), qsl_foundation::Setting::IntakeInputBytes);
        assert_eq!(outcome.configured_bound(), bound);
        assert_eq!(outcome.actual(), u128::from(bound) + 1);
        // Raised through the builder: judged on its content, not the size.
        let raised = at_bound.with_input_bytes(bound + 1);
        assert!(admit(&offered, &digest_domain, &map, raised).is_ok());
        // Raised through the settings operation.
        let mut operated = at_bound;
        let operand = format!("intake.input_bytes={}", bound + 1);
        for (setting, value) in qsl_foundation::setting::parse_operands([operand.as_str()]).unwrap()
        {
            assert!(operated.set_bound(setting, value));
        }
        assert_eq!(operated, raised);
        assert!(admit(&offered, &digest_domain, &map, operated).is_ok());
    }

    /// PR #379 review F3: unparseable bytes supply no document, so even an
    /// empty offered identity and version, whose raw digest matches, refuse
    /// `wrong-model-selection`. Before `admit` required a parsed document,
    /// the two empty strings compared equal and the bytes admitted.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_unparseable_bytes_under_an_empty_identity_and_version() {
        let bytes = b"not json".to_vec();
        let digest = digest_of(&bytes);
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let (offered, digest_domain) = selection("", "", SHA256_JCS_DIGEST_DOMAIN, digest);
        let refusal = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::WrongModelSelection {
                    selection: offered.clone(),
                    actual_identity: String::new(),
                    actual_version: String::new(),
                },
                detail: "domain package selection names @ but the package declares @".to_owned(),
            }
        );
    }

    /// PR #379 review F4: the tree the one parse derives, and the digest it
    /// takes, equal what `serde_json::from_slice` reads from the same bytes
    /// and the RFC 8785 digest of that value, for numbers at every edge
    /// where serde_json's read is exact and every string escape (a non-whole
    /// number's double is the digest's:
    /// `a_non_whole_number_in_the_tree_is_the_digests_double`). A number with no exact RFC 8785 spelling is
    /// refused instead (`refuses_a_whole_number_beyond_2_53_at_its_pointer`,
    /// `refuses_a_number_that_is_not_its_doubles_shortest_text`).
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn the_one_parse_reads_what_serde_json_reads() {
        let cases = [
            // Numbers.
            "0",
            "-0",
            "-0.0",
            "1.0",
            "0.1",
            "1e2",
            "1E+2",
            "9007199254740992",
            "-9007199254740992",
            "9.007199254740992e15",
            "5e-324",
            "2.5",
            r#"[0, -0, 1.5, -2, 3e-7, -1E-2]"#,
            // Out of range for both: refused, never a different value.
            "1e400",
            "-1e400",
            // Every string escape, and surrogate pairs.
            r#""\" \\ \/ \b \f \n \r \t""#,
            r#""\u0000 \u001f \u007f é € ￿ �""#,
            r#""😀 𝄞 􏿿 𐀀""#,
            r#""é 😀 raw""#,
            r#"{"k\"ey":"v\\al","\/":"\n"}"#,
        ];
        for text in cases {
            let expected = serde_json::from_slice::<Value>(text.as_bytes());
            match (
                PackageDocument::parse(text.as_bytes(), qsl_foundation::IntakeLimits::default()),
                expected,
            ) {
                (Ok(document), Ok(expected)) => {
                    assert_eq!(document.tree(), &expected, "{text}");
                    assert_eq!(
                        document.jcs_digest(),
                        digest_of(&canonical(&expected)),
                        "{text}"
                    );
                }
                (Err(_), Err(_)) => assert!(text.ends_with("1e400"), "{text}: both refused"),
                (parsed, expected) => {
                    panic!("{text}: the one parse gave {parsed:?}, serde_json gave {expected:?}")
                }
            }
        }
    }

    /// A non-whole number in the tree is the correctly rounded double the
    /// digest encodes, even where serde_json's own float parse is one unit
    /// in the last place off.
    #[trace("TC-145", "FR-056-AC-15")]
    #[test]
    fn a_non_whole_number_in_the_tree_is_the_digests_double() {
        for text in ["1.5e-300", "0.1", "2.5", "-3e-7"] {
            let document =
                PackageDocument::parse(text.as_bytes(), qsl_foundation::IntakeLimits::default())
                    .unwrap();
            let expected: f64 = text.parse().unwrap();
            assert_eq!(document.tree().as_f64(), Some(expected), "{text}");
            assert_eq!(
                document.jcs_digest(),
                digest_of(&canonical(document.tree())),
                "{text}"
            );
        }
    }

    /// A package document holding `number` at `/package/count`, beside the
    /// package's own identity and version.
    fn document_with_count(number: &str) -> String {
        format!(r#"{{"package":{{"identity":"acme/orders","version":"1","count":{number}}}}}"#)
    }

    /// `text` refuses at the one parse `noncanonical_wire` with cause
    /// `inexact` and `document_pointer` `pointer`, naming `number`.
    fn assert_noncanonical(text: &str, inexact: Inexact, pointer: &str, number: &str) {
        let refusal =
            PackageDocument::parse(text.as_bytes(), qsl_foundation::IntakeLimits::default())
                .unwrap_err();
        assert_eq!(
            refusal,
            noncanonical_number(pointer.parse().unwrap(), inexact, number),
            "{text}"
        );
        assert_eq!(refusal.code, Code::NoncanonicalWire, "{text}");
        assert_eq!(
            refusal.cause.catalog_code(),
            qsl_foundation::diagnostic::CatalogCode::new("noncanonical_wire", inexact.as_str()),
            "{text}"
        );
        assert_eq!(
            refusal.cause,
            ModelRefusalCause::NoncanonicalNumber {
                inexact,
                document_pointer: pointer.parse().unwrap(),
            },
            "{text}"
        );
    }

    /// A number denoting a whole value beyond ±2^53 refuses at the one parse
    /// `noncanonical_wire`/`inexact-integer` carrying its RFC 6901 pointer,
    /// however it is spelled: past `u64`, as an exponent, as a decimal, or
    /// just past 2^53. A member name holding `/` or `~` is escaped in the
    /// pointer, an element is named by its index, a bare number is the
    /// whole document, and of two such numbers the first in document order
    /// is named.
    #[trace("TC-145", "FR-056-AC-13")]
    #[test]
    fn refuses_a_whole_number_beyond_2_53_at_its_pointer() {
        for number in [
            "18446744073709551616",
            "-18446744073709551616",
            "1e20",
            "9.007199254740993e15",
            "9007199254740993",
            "-9007199254740993",
            "9007199254740993.0",
            "0.9007199254740993E+16",
            "18446744073709551615",
            "-9223372036854775809",
            "123456789012345678901234567890",
            "1.7976931348623157e308",
        ] {
            assert_noncanonical(
                &document_with_count(number),
                Inexact::Integer,
                "/package/count",
                number,
            );
        }
        for (text, number, pointer) in [
            (
                r#"{"a/b":[{"c~d":1e20}],"package":{}}"#,
                "1e20",
                "/a~1b/0/c~0d",
            ),
            (
                r#"{"b":9007199254740993,"a":[18446744073709551616]}"#,
                "9007199254740993",
                "/b",
            ),
            (
                r#"[0,9007199254740992,[1,-9007199254740993]]"#,
                "-9007199254740993",
                "/2/1",
            ),
            (
                "123456789012345678901234567890",
                "123456789012345678901234567890",
                "",
            ),
        ] {
            assert_noncanonical(text, Inexact::Integer, pointer, number);
        }
    }

    /// A number whose exact value is not the exact value of its nearest
    /// double's shortest round-trip text refuses at the one parse
    /// `noncanonical_wire`/`inexact-number` carrying its pointer: a
    /// non-whole value beyond 2^53, a decimal with more digits than its
    /// double, and a literal that underflows to zero. A number that fits
    /// both causes refuses `inexact-integer`: `9007199254740993` is also not
    /// its double's shortest text (`9007199254740992`), and `1e20` is
    /// exactly a double yet whole beyond 2^53. The first inexact number in
    /// document order is named, whichever cause it has.
    #[trace("TC-145", "FR-056-AC-14")]
    #[test]
    fn refuses_a_number_that_is_not_its_doubles_shortest_text() {
        let long = "0.123456789012345678901234567890123456789012345678901234567890123456789012345";
        for number in [
            "9007199254740993.5",
            "0.1000000000000000000001",
            "1e-400",
            "-1e-400",
            "4.9e-324",
            "9007199254740992.5",
            long,
        ] {
            assert_noncanonical(
                &document_with_count(number),
                Inexact::Number,
                "/package/count",
                number,
            );
        }
        for number in ["9007199254740993", "1e20"] {
            assert_noncanonical(
                &document_with_count(number),
                Inexact::Integer,
                "/package/count",
                number,
            );
        }
        // 2^53 is the double nearest to 9007199254740993.
        assert!(!Decimal::of("9007199254740993").same_value(&Decimal::of("9007199254740992")));
        assert_noncanonical(
            r#"{"a":[0.5,1e-400],"b":9007199254740993}"#,
            Inexact::Number,
            "/a/1",
            "1e-400",
        );
    }

    /// A double with two equally close shortest round-trip texts is spelled
    /// by RFC 8785 with the even last digit, as quire-canonical writes it
    /// (`1125899906842624.25` is `1125899906842624.2`, not `.3`). Only that
    /// spelling is exact: the other refuses `inexact-number`, and the exact
    /// one admits with a tree holding its double and the digest of its text.
    #[trace("TC-145", "FR-056-AC-14")]
    #[test]
    fn a_tie_between_two_shortest_texts_admits_only_the_even_digit() {
        for number in [
            "1500000000000000.2",
            "1125899906842624.2",
            "-1125899906842624.2",
            "2.9802322387695312e-8",
        ] {
            let document = PackageDocument::parse(
                document_with_count(number).as_bytes(),
                qsl_foundation::IntakeLimits::default(),
            )
            .unwrap_or_else(|refusal| panic!("{number}: {refusal:?}"));
            let expected: f64 = number.parse().unwrap();
            assert_eq!(
                document.tree()["package"]["count"].as_f64(),
                Some(expected),
                "{number}"
            );
            assert_eq!(
                document.jcs_digest(),
                digest_of(&canonical(document.tree())),
                "{number}"
            );
        }
        for number in [
            "1500000000000000.3",
            "1125899906842624.3",
            "2.9802322387695313e-8",
        ] {
            assert_noncanonical(
                &document_with_count(number),
                Inexact::Number,
                "/package/count",
                number,
            );
        }
    }

    /// ±2^53, every whole number within them, and every number that is
    /// exactly its double's shortest round-trip text admit, however they
    /// are spelled, with the digest of that text: `0.1` and `1.0`, `-0`
    /// (zero has no sign), and the least subnormal `5e-324`. A whole number
    /// within ±2^53 is refused under neither cause.
    #[trace("TC-145", "FR-056-AC-13", "FR-056-AC-14")]
    #[test]
    fn admits_2_53_its_negation_and_every_exact_number() {
        for (number, exact) in [
            ("9007199254740992", Some(9_007_199_254_740_992_i64)),
            ("-9007199254740992", Some(-9_007_199_254_740_992)),
            ("9007199254740991", Some(9_007_199_254_740_991)),
            ("-4503599627370497", Some(-4_503_599_627_370_497)),
            ("1e15", None),
            ("9.007199254740992e15", None),
            ("90071992547409920e-1", None),
            ("9007199254740992.000", None),
            ("0.1", None),
            ("1.0", None),
            ("-0", None),
            ("-0.0", None),
            ("5e-324", None),
            ("1E+2", None),
            ("-1E-2", None),
            ("0.10", None),
        ] {
            let text = document_with_count(number);
            let document =
                PackageDocument::parse(text.as_bytes(), qsl_foundation::IntakeLimits::default())
                    .unwrap_or_else(|refusal| panic!("{number}: {refusal:?}"));
            let count = &document.tree()["package"]["count"];
            if let Some(exact) = exact {
                assert_eq!(count.as_i64(), Some(exact), "{number}");
            }
            let expected: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(count, &expected["package"]["count"], "{number}");
            assert_eq!(
                document.jcs_digest(),
                digest_of(&canonical(&expected)),
                "{number}"
            );
        }
    }

    /// Two documents that differ only in a number rounding to one double
    /// used to share one `sha256-jcs` digest, the digest of that double's
    /// text. Now a document whose number is not that text refuses at the
    /// parse, and admission returns that refusal under the digest they used
    /// to share: `18446744073709551615` and `18446744073709551616` both
    /// refuse, and of `0.1` and `0.1000000000000000000001` only `0.1`
    /// admits. No two documents share a digest.
    #[trace("TC-145", "FR-056-AC-13", "FR-056-AC-14")]
    #[test]
    fn documents_differing_only_in_an_inexact_number_share_no_digest() {
        let shared_integer = digest_of(
            br#"{"package":{"count":18446744073709552000,"identity":"acme/orders","version":"1"}}"#,
        );
        let shared_tenth =
            digest_of(br#"{"package":{"count":0.1,"identity":"acme/orders","version":"1"}}"#);
        for (number, shared, inexact) in [
            ("18446744073709551615", shared_integer, Inexact::Integer),
            ("18446744073709551616", shared_integer, Inexact::Integer),
            ("0.1000000000000000000001", shared_tenth, Inexact::Number),
        ] {
            let bytes = document_with_count(number).into_bytes();
            let map = BTreeMap::from([(shared, bytes)]);
            let (offered, digest_domain) =
                selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, shared);
            assert_eq!(
                admit(
                    &offered,
                    &digest_domain,
                    &map,
                    qsl_foundation::IntakeLimits::default()
                )
                .unwrap_err(),
                noncanonical_number("/package/count".parse().unwrap(), inexact, number),
                "{number}"
            );
        }
        let tenth = document_with_count("0.1").into_bytes();
        let map = BTreeMap::from([(shared_tenth, tenth)]);
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, shared_tenth);
        assert!(admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default()
        )
        .is_ok());
    }

    /// The decision reads the number's text in time linear in its length
    /// and allocates nothing, so a nine-digit exponent decides at once, and
    /// a fraction or exponent of any length decides by exact decimal
    /// reasoning rather than a rounded double. Two texts denote the same
    /// value exactly when their digits and scale agree; zero has no sign.
    #[trace("TC-145", "FR-056-AC-13", "FR-056-AC-14")]
    #[test]
    fn decides_on_the_text_without_rounding() {
        let long_fraction = format!("9007199254740992.{}1", "0".repeat(400));
        let long_exponent = format!("9007199254740993e{}", "0".repeat(400));
        let long_negative_exponent = format!("9007199254740993e-{}1", "0".repeat(400));
        let zeros_then_digits = format!("0.{}9007199254740993e416", "0".repeat(400));
        for (lexeme, beyond) in [
            ("1e999999999", true),
            ("-1E+999999999", true),
            ("1e-999999999", false),
            ("1e99999999999999999999999999999", true),
            ("1e-99999999999999999999999999999", false),
            ("0.0e99999999999999999999999999999", false),
            ("9007199254740992e0", false),
            ("9007199254740993e0", true),
            ("900719925474099.3e1", true),
            ("900719925474099.2e1", false),
            ("10000000000000000", true),
            ("9999999999999999", true),
            ("1000000000000000", false),
            (long_fraction.as_str(), false),
            (long_exponent.as_str(), true),
            (long_negative_exponent.as_str(), false),
            (zeros_then_digits.as_str(), true),
        ] {
            assert_eq!(
                Decimal::of(lexeme).is_whole_beyond_2_53(),
                beyond,
                "{lexeme}"
            );
        }
        let tenth_zeros = format!("0.1{}", "0".repeat(400));
        for (left, right, same) in [
            ("0.1", "1e-1", true),
            ("0.10", "1e-1", true),
            (tenth_zeros.as_str(), "1e-1", true),
            ("100", "1e2", true),
            ("-0", "0e0", true),
            ("-0.0e-999", "0e0", true),
            ("1e-400", "0e0", false),
            ("-1.5", "1.5e0", false),
            ("0.1000000000000000000001", "1e-1", false),
            ("9007199254740993.5", "9.007199254740994e15", false),
            ("1e-99999999999999999999999999999", "0e0", false),
        ] {
            assert_eq!(
                Decimal::of(left).same_value(&Decimal::of(right)),
                same,
                "{left} = {right}"
            );
        }
    }

    /// A repeated member name, which `serde_json` resolves last-wins, is
    /// refused by the one parse as malformed at the repeated name, and a
    /// leading byte order mark at byte 0: RFC 8785 requires I-JSON, so
    /// neither document has a JCS digest. Admission digests such bytes raw,
    /// so a repeated-name document offered under the JCS digest of its
    /// last-wins value refuses `byte-digest-mismatch`.
    #[trace("TC-145", "FR-056-AC-12")]
    #[test]
    fn a_repeated_member_name_or_byte_order_mark_does_not_parse() {
        for (text, repeated) in [
            (r#"{"a":1,"a":2}"#, r#""a":2"#),
            (r#"{"a":{"x":1},"a":{"y":2}}"#, r#""a":{"y""#),
            (r#"{"o":{"a":1,"a":[1]}}"#, r#""a":[1]"#),
            (r#"{"😀":1,"😀":2}"#, r#""😀":2"#),
        ] {
            assert!(
                serde_json::from_slice::<Value>(text.as_bytes()).is_ok(),
                "{text}"
            );
            let offset = text
                .find(repeated)
                .expect("the repeated name is in the text");
            assert_eq!(
                PackageDocument::parse(text.as_bytes(), qsl_foundation::IntakeLimits::default())
                    .unwrap_err(),
                malformed(
                    "$",
                    &format!(
                        "package document is malformed JSON at byte {offset}: \
                         duplicate member name"
                    )
                ),
                "{text}"
            );
        }

        let bom = b"\xEF\xBB\xBF{}";
        assert_eq!(
            PackageDocument::parse(bom, qsl_foundation::IntakeLimits::default()).unwrap_err(),
            malformed(
                "$",
                "package document is malformed JSON at byte 0: unexpected character"
            )
        );

        let repeated = br#"{"package":{"identity":"acme/orders","version":"1"},"s":1,"s":2}"#;
        let last_wins: Value = serde_json::from_slice(repeated).unwrap();
        let digest = digest_of(&canonical(&last_wins));
        let mut map = BTreeMap::new();
        map.insert(digest, repeated.to_vec());
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let refusal = admit(
            &offered,
            &digest_domain,
            &map,
            qsl_foundation::IntakeLimits::default(),
        )
        .unwrap_err();
        assert_eq!(refusal.code, Code::StaleDependency);
        assert_eq!(
            refusal.cause,
            ModelRefusalCause::ByteDigestMismatch {
                expected: digest,
                actual: digest_of(repeated),
            },
            "a repeated-name document digests raw"
        );
    }

    /// The reader's and the encoder's allocation failures refuse
    /// `resource_exhausted`/`allocation-failed` carrying the bytes requested:
    /// not a limit, and not a malformed document.
    #[trace("TC-729", "FR-259-AC-5")]
    #[test]
    fn an_allocation_failure_refuses_allocation_failed() {
        let expected = allocation_failed(4096);
        assert_eq!(expected.code, Code::ResourceExhausted);
        assert_eq!(
            expected.cause,
            ModelRefusalCause::AllocationFailed { requested: 4096 }
        );
        assert_eq!(expected.cause.as_str(), "allocation-failed");
        assert_eq!(
            read_refusal(
                quire_canonical::ReadError::Allocation { requested: 4096 },
                1,
                2
            ),
            expected
        );
        assert_eq!(
            digest_refusal(quire_canonical::Error::Allocation { requested: 4096 }, 1, 2),
            expected
        );
    }

    /// `text` as a reader document, on a 512 KiB thread's behalf: the reader
    /// itself is iterative.
    fn read_text(text: &str) -> quire_canonical::Document {
        quire_canonical::read(text.as_bytes(), u64::MAX).expect("the document reads")
    }

    /// Runs `body` on a 512 KiB thread and fails the test if it overflows.
    fn on_a_small_stack(body: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(body)
            .expect("the test thread spawns")
            .join()
            .expect("the body does not overflow the stack");
    }

    /// TC-730 step 4: a reader tree nested 100,000 deep, far past
    /// `serde_json`'s 128 limit, builds both views on a 512 KiB thread, and
    /// the `serde_json` view drops without recursion.
    #[trace("TC-730", "FR-260-AC-5")]
    #[test]
    fn tc_730_a_tree_nested_100000_deep_builds_both_views_on_a_small_stack() {
        let depth = 100_000;
        let objects = format!("{}null{}", r#"{"k":"#.repeat(depth), "}".repeat(depth));
        for text in [nested_arrays(depth, "0"), objects] {
            on_a_small_stack(move || {
                let document = read_text(&text);
                let tree = value_of(document.root()).expect("the deep tree builds");
                quire_canonical::drop_value(tree);
                drop(json_of(document.root()).expect("the deep tree builds"));
            });
        }
    }

    /// TC-730 step 4: a `PackageDocument` holding a `serde_json` view
    /// nested 100,000 deep drops on a 512 KiB thread.
    #[trace("TC-730", "FR-260-AC-5")]
    #[test]
    fn tc_730_a_package_document_nested_100000_deep_drops_on_a_small_stack() {
        on_a_small_stack(|| {
            let document = read_text(&nested_arrays(100_000, "0"));
            let package = PackageDocument {
                bundle: agent_ix_semantic_ir::json::Json::Null,
                tree: value_of(document.root()).expect("the deep tree builds"),
                jcs_digest: [0; 32],
            };
            drop(package);
        });
    }

    /// TC-730 step 4: a leaf conversion that fails after a 100,000-deep
    /// sibling has finished returns the failure, and the finished sibling
    /// drops without recursion.
    #[trace("TC-730", "FR-260-AC-5")]
    #[test]
    fn tc_730_a_failing_leaf_after_a_deep_sibling_returns_the_failure() {
        on_a_small_stack(|| {
            let text = format!("[{},\"stop\"]", nested_arrays(100_000, "0"));
            let document = read_text(&text);
            let failed = build(
                document.root(),
                |leaf| match leaf {
                    Scalar::String("stop") => Err("stop"),
                    Scalar::Null | Scalar::Bool(_) | Scalar::Number(_) | Scalar::String(_) => {
                        Ok(Value::Null)
                    }
                },
                Value::Array,
                |members| Value::Object(members.into_iter().collect()),
                quire_canonical::drop_value,
            );
            assert_eq!(failed, Err(BuildFault::Scalar("stop")));
        });
    }

    /// TC-730 step 4: a shallow document builds the same trees the
    /// recursive builders built: the `serde_json` tree `serde_json` reads
    /// from the text, and a `Json` with members in document order and each
    /// number by its lexeme.
    #[trace("TC-730", "FR-260-AC-5")]
    #[test]
    fn tc_730_a_shallow_document_builds_the_same_trees() {
        use agent_ix_semantic_ir::json::Json;
        let text = r#"{"a":[1,2.50,"x\\n",null,true,[],{}],"b":{"c":[[false]]},"z":0}"#;
        let document = read_text(text);
        assert_eq!(
            value_of(document.root()).expect("builds"),
            serde_json::from_str::<Value>(text).expect("serde_json reads it")
        );
        let number = |lexeme: &str| Json::Number(lexeme.to_owned());
        assert_eq!(
            json_of(document.root()),
            Ok(Json::Object(vec![
                (
                    "a".to_owned(),
                    Json::Array(vec![
                        number("1"),
                        number("2.50"),
                        Json::Str("x\\n".to_owned()),
                        Json::Null,
                        Json::Bool(true),
                        Json::Array(vec![]),
                        Json::Object(vec![]),
                    ])
                ),
                (
                    "b".to_owned(),
                    Json::Object(vec![(
                        "c".to_owned(),
                        Json::Array(vec![Json::Array(vec![Json::Bool(false)])])
                    )])
                ),
                ("z".to_owned(), number("0")),
            ]))
        );
    }
}

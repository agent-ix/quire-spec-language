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
//! It never invents a value and never drops data. Three shapes FCD emits
//! today are still FCD gaps: bare-string core kinds (`"scalar"`/`"alias"`/...)
//! declared directly in `types[]`, FCD's own identity form
//! (`ix://<pkg>/type/<id>`, `ix://<pkg>/field/<Owner>-<name>`) rather than
//! QSpec's (`ix://<pkg>/<id>`, `<owner>/<name>`), and a non-empty operation
//! `frame`. This reader refuses all three rather than working around them.
//! A type's inline `relationships[]` (model-complete.md's Relationships
//! row: each end names an object type or a process, carries a role and a
//! multiplicity, and `direction` is the full source-to-target/
//! target-to-source/bidirectional/undirected vocabulary) was a fourth gap
//! -- FCD's old `verb`/`category`/`composite` shape carried neither a role
//! nor that direction vocabulary -- fixed upstream by FCD #199/#200's
//! `sourceEnd`/`targetEnd`/`role`/`direction` shape, so this reader now
//! reads it rather than refusing it.
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use agent_ix_extraction_frontend::lift::{lift, LiftOutcome, LiftRequest};
use agent_ix_extraction_frontend::{Diagnostic, Refusal};
use serde_json::Value;

use crate::model::domain_package::{
    AllocationRecord, ComponentRecord, DomainPackageRecord, DomainPackageRef, EndpointRecord,
    Extent, FieldMemberRecord, Multiplicity, NativeValueType, ObjectTypeRecord, OperationEffect,
    OperationMemberRecord, OperationParameterRecord, OperationResult, PopulationRecord,
    PortDirection, RecordValueTypeRecord, RelationshipDirection, RelationshipEnd,
    RelationshipRecord, ValueTypeRef,
};
use crate::model::key::{hex, raw_bytes_digest, DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
use crate::model::refusal::IntakeLimit;
use crate::value::semantic_node::IDENTITY_LIMITS as LIMITS;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::{LocatedSpan, Position};

mod unit;
pub use unit::{admit_unit, package_input, SelectedModel, UnitIntakeCause, UnitIntakeRefusal};

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
/// path; the diagnostics and provenance sidecars are skipped (`None`).
/// The scratch directory is removed when this function returns.
pub fn lift_document(bundle_root: &Path, module_roots: &[PathBuf]) -> Result<Vec<u8>, LiftFailure> {
    let scratch = tempfile::tempdir().map_err(LiftFailure::Scratch)?;
    let request = LiftRequest {
        bundle_root: bundle_root.to_path_buf(),
        module_roots: module_roots.to_vec(),
        out: scratch.path().join("document.json"),
        diagnostics: None,
        provenance: None,
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
/// into a tree. It reads the bytes with `agent-ix-semantic-ir`'s own JSON
/// reader, the one `validate_with_semantic_ir` must be handed (its
/// `decide` accepts only that crate's own `Json`), and derives this crate's
/// `serde_json::Value` view from that same tree rather than from the bytes
/// again. [`admit`]'s JCS digest and [`read_records`]'s per-node reader both
/// read the derived view, so the three can never disagree about what the
/// document is.
///
/// The reader's two limits, `json::MAX_INPUT_BYTES` and `json::MAX_DEPTH`,
/// refuse as [`ModelRefusalCause::IntakeLimitExceeded`] naming the limit,
/// never as a malformed document. The derived view keeps the reader's
/// last-wins resolution of a repeated member name, and each number is
/// converted from the lexeme the document carried with `serde_json`'s own
/// number parser, so it equals what `serde_json::from_slice` would have read,
/// every integer exact. A number with no finite value (`1e400`) refuses. A lone UTF-16
/// surrogate escape refuses too: RFC 8785 (via RFC 7493) admits none, and the
/// reader would otherwise replace it with U+FFFD, so two different documents
/// would share one tree and one JCS digest.
///
/// The `sha256-jcs` digest is taken here, once, by `quire-canonical` (ADR-013
/// §2, ADR-013:113: the one RFC 8785 implementation) under
/// `INTAKE_DIGEST_LIMITS`, reading an integer outside ±2^53 as the double RFC
/// 8785 reads it as (`Rfc8785Numbers`), and [`admit`]'s check 3 compares it.
#[derive(Debug, Clone)]
pub struct PackageDocument {
    /// `{"ir": <document>}`: the bundle `agent_ix_semantic_ir::decide` reads
    /// (`input-bundle.schema.json` requires an `ir` member).
    bundle: agent_ix_semantic_ir::json::Json,
    /// The same document as a `serde_json::Value`, derived from `bundle`'s
    /// own `ir` member.
    tree: Value,
    /// SHA-256 over `tree`'s RFC 8785 bytes: the package's `sha256-jcs`
    /// digest.
    jcs_digest: [u8; 32],
}

impl PackageDocument {
    /// Parses package document bytes once.
    ///
    /// Input over `json::MAX_INPUT_BYTES`, or a value enclosed by
    /// `json::MAX_DEPTH` or more arrays and objects, refuses
    /// `resource_exhausted`/`intake-limit-exceeded` naming the limit and its
    /// bound. Bytes that are not UTF-8, do not parse as JSON, carry a lone
    /// surrogate escape or carry a number with no finite value refuse
    /// `invalid_model_binding`/`malformed-declaration` against the document
    /// root `$`.
    pub fn parse(bytes: &[u8]) -> Result<Self, ModelRefusal> {
        use agent_ix_semantic_ir::json::{MAX_DEPTH, MAX_INPUT_BYTES};
        if bytes.len() > MAX_INPUT_BYTES {
            return Err(limit_exceeded(IntakeLimit::InputBytes, MAX_INPUT_BYTES));
        }
        let scan = ByteScan::of(bytes);
        if scan.too_deep {
            return Err(limit_exceeded(IntakeLimit::NestingDepth, MAX_DEPTH));
        }
        let root_malformed =
            |detail: String| malformed_declaration("$".to_owned(), None, None, detail);
        let text = std::str::from_utf8(bytes).map_err(|err| {
            root_malformed(format!("package document bytes are not UTF-8: {err}"))
        })?;
        let ir = agent_ix_semantic_ir::json::parse(text).map_err(|err| {
            root_malformed(format!(
                "package document bytes do not parse as JSON for agent-ix-semantic-ir: {err}"
            ))
        })?;
        if let Some(offset) = scan.lone_surrogate {
            return Err(root_malformed(format!(
                "package document carries a lone UTF-16 surrogate escape at byte {offset}"
            )));
        }
        let tree = value_of(&ir).map_err(|lexeme| {
            root_malformed(format!(
                "package document number {lexeme} has no serde_json representation"
            ))
        })?;
        // Every tree `value_of` builds has an RFC 8785 encoding within
        // `INTAKE_DIGEST_LIMITS`; the one refusal left is a failed heap
        // reservation, reported against the root rather than aborting.
        let jcs_digest = *quire_canonical::sha256(&Rfc8785Numbers(&tree), INTAKE_DIGEST_LIMITS)
            .map_err(|err| {
                root_malformed(format!(
                    "package document could not be RFC 8785-encoded: {err}"
                ))
            })?
            .as_bytes();
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
}

/// How many times longer a document's RFC 8785 text can be than the document
/// itself. Only number spelling grows: `1e20` (4 bytes) canonicalizes to
/// `100000000000000000000` (21 bytes), 5.25 times as long, and no shorter
/// lexeme grows more (from `1e21` on the text is exponent form again).
/// Whitespace, escapes and repeated member names only shrink. Rounded up to 6.
const CANONICAL_GROWTH: u64 = 6;

/// The limits intake's `sha256-jcs` digest encodes under: canonical text up
/// to [`CANONICAL_GROWTH`] times the reader's `MAX_INPUT_BYTES` (384 MiB), a
/// ceiling no admitted document reaches, and [`LIMITS`]'s depth, above the
/// reader's own `MAX_DEPTH` (200).
const INTAKE_DIGEST_LIMITS: quire_canonical::Limits = match quire_canonical::Limits::new(
    // `usize` is at most 64 bits on every target Rust supports: lossless.
    agent_ix_semantic_ir::json::MAX_INPUT_BYTES as u64 * CANONICAL_GROWTH,
    LIMITS.max_depth(),
) {
    Ok(limits) => limits,
    // `LIMITS.max_depth()` is within `Limits::MAX_DEPTH` by construction;
    // evaluated at compile time.
    Err(_) => panic!("LIMITS' depth is within Limits::MAX_DEPTH"),
};

/// ADR-011 Limits: a package document reached `limit`, whose bound is
/// `bound`.
fn limit_exceeded(limit: IntakeLimit, bound: usize) -> ModelRefusal {
    ModelRefusal {
        code: Code::ResourceExhausted,
        cause: ModelRefusalCause::IntakeLimitExceeded { limit, bound },
        detail: format!(
            "package document exceeds intake's {} limit of {bound}",
            limit.as_str()
        ),
    }
}

/// What one pass over package bytes finds that the reader's own result does
/// not say: whether the reader will refuse at its depth limit (its
/// `JsonError` carries no kind), and whether a string carries a lone
/// surrogate escape (the reader replaces one silently).
///
/// The depth rule mirrors `agent_ix_semantic_ir::json`'s reader: it refuses
/// when it starts a value already enclosed by `MAX_DEPTH` arrays and
/// objects. A member name counts as that value's start, since a member
/// always carries one. The scan runs over any bytes, well-formed or not;
/// `lone_surrogate` is only meaningful once the reader has accepted them.
struct ByteScan {
    /// The reader will refuse these bytes at its nesting limit.
    too_deep: bool,
    /// The byte offset of the first lone surrogate escape (`\uD800`-
    /// `\uDFFF` not in a high-then-low pair).
    lone_surrogate: Option<usize>,
}

impl ByteScan {
    fn of(bytes: &[u8]) -> Self {
        use agent_ix_semantic_ir::json::MAX_DEPTH;
        let mut scan = Self {
            too_deep: false,
            lone_surrogate: None,
        };
        let mut depth = 0usize;
        let mut in_string = false;
        let mut at = 0usize;
        while let Some(&byte) = bytes.get(at) {
            if in_string {
                match byte {
                    b'"' => in_string = false,
                    b'\\' if bytes.get(at + 1) == Some(&b'u') => {
                        let unit = utf16_escape(bytes, at);
                        let paired_low = utf16_escape(bytes, at + 6).is_some_and(is_low_surrogate);
                        match unit {
                            Some(unit) if is_high_surrogate(unit) && paired_low => at += 6,
                            Some(unit) if is_high_surrogate(unit) || is_low_surrogate(unit) => {
                                scan.lone_surrogate.get_or_insert(at);
                            }
                            _ => {}
                        }
                        at += 6;
                        continue;
                    }
                    // Any other escape is two bytes; skipping the second
                    // keeps an escaped quote from closing the string.
                    b'\\' => at += 1,
                    _ => {}
                }
            } else {
                match byte {
                    b']' | b'}' => depth = depth.saturating_sub(1),
                    b' ' | b'\t' | b'\n' | b'\r' | b',' | b':' => {}
                    _ if depth >= MAX_DEPTH => {
                        scan.too_deep = true;
                        return scan;
                    }
                    b'[' | b'{' => depth += 1,
                    b'"' => in_string = true,
                    // A scalar's bytes; none opens or closes anything.
                    _ => {}
                }
            }
            at += 1;
        }
        scan
    }
}

/// The UTF-16 code unit of a `\uXXXX` escape starting at `at`, when four
/// hex digits follow it.
fn utf16_escape(bytes: &[u8], at: usize) -> Option<u16> {
    let escape = bytes.get(at..at + 6)?;
    let (prefix, digits) = escape.split_at(2);
    if prefix != b"\\u" || !digits.iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    let digits = std::str::from_utf8(digits).ok()?;
    u16::from_str_radix(digits, 16).ok()
}

fn is_high_surrogate(unit: u16) -> bool {
    (0xD800..0xDC00).contains(&unit)
}

fn is_low_surrogate(unit: u16) -> bool {
    (0xDC00..0xE000).contains(&unit)
}

/// `json` as a `serde_json::Value`, or the first number lexeme `serde_json`
/// cannot represent. Recursion is bounded by the reader's own `MAX_DEPTH`,
/// which already held when `json` was parsed.
fn value_of(json: &agent_ix_semantic_ir::json::Json) -> Result<Value, &str> {
    use agent_ix_semantic_ir::json::Json;
    Ok(match json {
        Json::Null => Value::Null,
        Json::Bool(value) => Value::Bool(*value),
        Json::Number(lexeme) => Value::Number(lexeme.parse().map_err(|_| lexeme.as_str())?),
        Json::Str(text) => Value::String(text.clone()),
        Json::Array(items) => Value::Array(items.iter().map(value_of).collect::<Result<_, _>>()?),
        Json::Object(members) => {
            // In document order, so a repeated name resolves last-wins, the
            // same way `Json::get` and `serde_json::from_slice` resolve it.
            let mut object = serde_json::Map::with_capacity(members.len());
            for (name, value) in members {
                object.insert(name.clone(), value_of(value)?);
            }
            Value::Object(object)
        }
    })
}

/// The magnitude up to which every integer is an IEEE 754 double exactly.
const EXACT_DOUBLE_INTEGER: u64 = 1 << 53;

/// `tree` as the `sha256-jcs` digest reads it (QSL-194): every value as is,
/// except that an integer outside ±2^53 serializes as the IEEE 754 double
/// nearest it. RFC 8785 §3.2.2.3 serializes the double a number parses to, so
/// every spelling of one double -- `18446744073709551615`,
/// `18446744073709551616` -- canonicalizes alike (`18446744073709552000`).
///
/// Only the digest reads through this view. The tree [`read_records`] reads
/// keeps each integer exact, so a `u64` field above 2^53 reads as written.
/// This is a number adapter, not an encoder: `quire-canonical` still orders,
/// escapes and spells everything.
struct Rfc8785Numbers<'a>(&'a Value);

impl serde::Serialize for Rfc8785Numbers<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap, SerializeSeq};
        match self.0 {
            Value::Number(number) => {
                // `as` rounds an integer to the nearest double, ties to even:
                // the double RFC 8785 reads the integer's decimal text as.
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "rounding to the nearest double is what RFC 8785 does"
                )]
                match (number.as_u64(), number.as_i64()) {
                    (Some(value), _) if value > EXACT_DOUBLE_INTEGER => {
                        serializer.serialize_f64(value as f64)
                    }
                    (None, Some(value)) if value.unsigned_abs() > EXACT_DOUBLE_INTEGER => {
                        serializer.serialize_f64(value as f64)
                    }
                    _ => number.serialize(serializer),
                }
            }
            Value::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(&Rfc8785Numbers(item))?;
                }
                seq.end()
            }
            Value::Object(members) => {
                let mut map = serializer.serialize_map(Some(members.len()))?;
                for (name, member) in members {
                    map.serialize_entry(name, &Rfc8785Numbers(member))?;
                }
                map.end()
            }
            Value::Null | Value::Bool(_) | Value::String(_) => self.0.serialize(serializer),
        }
    }
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
pub fn admit(
    offered: &DomainPackageRef,
    digest_domain: &str,
    bytes_by_digest: &BTreeMap<[u8; 32], Vec<u8>>,
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
    // A document over one of the parse limits has no parsed form to take a
    // JCS digest of, so check 3 cannot tell whether it matches: that refuses
    // naming the limit (ADR-011 Limits), not as a digest mismatch. Any other
    // parse failure is not itself a refusal here: bytes that do not parse
    // are digested raw by check 3, and supply no identity/version to check 4.
    let document = match PackageDocument::parse(bytes) {
        Ok(document) => Some(document),
        Err(
            refusal @ ModelRefusal {
                cause: ModelRefusalCause::IntakeLimitExceeded { .. },
                ..
            },
        ) => return Err(refusal),
        Err(_) => None,
    };
    check_package_digest(offered, bytes, document.as_ref())?;
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

/// FR-154 Intake check 3 (`model-complete.md:69`): SHA-256 over the
/// package's JCS bytes equals the selected digest, else `stale_dependency`.
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
fn check_package_digest(
    offered: &DomainPackageRef,
    bytes: &[u8],
    document: Option<&PackageDocument>,
) -> Result<(), ModelRefusal> {
    let actual_digest: [u8; 32] = match document {
        Some(document) => document.jcs_digest,
        None => raw_bytes_digest(bytes),
    };
    if actual_digest == offered.digest {
        return Ok(());
    }
    Err(ModelRefusal {
        code: Code::StaleDependency,
        cause: ModelRefusalCause::ByteDigestMismatch {
            expected: offered.digest,
            actual: actual_digest,
        },
        detail: format!(
            "domain package {}@{} bytes hash to {}, not the selected {}",
            offered.identity,
            offered.version,
            hex(&actual_digest),
            hex(&offered.digest)
        ),
    })
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
) -> Result<Vec<(DomainPackageRef, PackageDocument)>, ModelRefusal> {
    admit_located(offered, digest_domain, bytes_by_digest).map_err(|(_, refusal)| refusal)
}

/// [`admit_selections`], with the index into `offered` of the selection a
/// refusal concerns.
pub(crate) fn admit_located(
    offered: &[DomainPackageRef],
    digest_domain: &str,
    bytes_by_digest: &BTreeMap<[u8; 32], Vec<u8>>,
) -> Result<Vec<(DomainPackageRef, PackageDocument)>, (usize, ModelRefusal)> {
    let mut admitted = Vec::with_capacity(offered.len());
    let mut selected_versions: BTreeMap<&str, &str> = BTreeMap::new();
    for (index, selection) in offered.iter().enumerate() {
        if let Some(&already_selected_version) = selected_versions.get(selection.identity.as_str())
        {
            return Err((index, ModelRefusal {
                code: Code::DuplicateSelection,
                cause: ModelRefusalCause::DuplicateSelection {
                    identity: selection.identity.clone(),
                    already_selected_version: already_selected_version.to_owned(),
                    requested_version: selection.version.clone(),
                },
                detail: format!(
                    "domain package identity {:?} is already selected at version {:?}; this call \
                     additionally selects it at version {:?}, and a package selects at most one \
                     version of a domain-package identity",
                    selection.identity, already_selected_version, selection.version
                ),
            }));
        }
        let (admitted_ref, document) = admit(selection, digest_domain, bytes_by_digest)
            .map_err(|refusal| (index, refusal))?;
        selected_versions.insert(&selection.identity, &selection.version);
        admitted.push((admitted_ref, document));
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

fn read_operation_member(
    package: &str,
    owner_identity: &str,
    operation: &Value,
    at: &str,
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
    // `frame` is a real QSpec capability
    // (model-complete.md's Frames row) this reader does not yet turn into
    // an `OperationEffect`. An absent frame, or one whose `modifies`,
    // `creates` and `deletes` are all absent or empty, carries no effect
    // data at all and reads as `OperationEffect::default()` honestly; a
    // frame that actually declares anything refuses rather than being
    // silently dropped.
    let effect = match operation.get("frame") {
        None => OperationEffect::default(),
        Some(frame) => {
            let non_empty = ["modifies", "creates", "deletes"].iter().any(|member| {
                frame
                    .get(member)
                    .and_then(Value::as_array)
                    .is_some_and(|entries| !entries.is_empty())
            });
            if non_empty {
                return Err(unsupported_at(
                    operation,
                    &format!("{at}.frame"),
                    "operation.frame",
                ));
            }
            OperationEffect::default()
        }
    };
    Ok(OperationMemberRecord {
        key: declaration_key(package, node),
        owner: declaration_key(package, owner_identity),
        parameters,
        result,
        effect,
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
    let subsets = ctx.identity_keys(package, "subsets")?;
    let redefines = ctx
        .opt_str_field("redefines")
        .map(|target| declaration_key(package, target));
    Ok(FieldMemberRecord {
        key: declaration_key(package, node),
        owner: declaration_key(package, owner_identity),
        value_type,
        multiplicity,
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

fn read_component(
    package: &str,
    ctx: &NodeCtx<'_>,
    node: &str,
) -> Result<ComponentRecord, ModelRefusal> {
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

fn read_connection(
    package: &str,
    ctx: &NodeCtx<'_>,
    node: &str,
) -> Result<RelationshipRecord, ModelRefusal> {
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
fn read_type_node(
    package: &str,
    meanings: &HashMap<(String, String), String>,
    type_meanings: &HashMap<&str, &str>,
    type_value: &Value,
    at: &str,
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
    let mut records = Vec::new();
    match construct_meaning.as_str() {
        meaning::OBJECT_TYPE => {
            read_object_type(package, type_value, node, at, false, &mut records)?
        }
        meaning::SYSTEMS_INTERFACE => {
            read_object_type(package, type_value, node, at, true, &mut records)?
        }
        meaning::RECORD_VALUE_TYPE => {
            read_record_value_type(package, type_value, node, at, type_meanings, &mut records)?
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
            // A real FR-208 meaning (e.g. RECORD_VALUE_TYPE) with no
            // QSL record shape yet -- refused as a known-but-unsupported
            // declaration form, never silently folded into `ObjectTypeRecord`.
            return Err(unsupported_at(type_value, at, other));
        }
        other => {
            return Err(ctx.malformed(format!("kind: resolves to {other:?}, outside FR-208")));
        }
    }
    Ok(records)
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
    for node in nodes {
        match node {
            DocumentNode::Type(type_value, position, _) => {
                let at = format!("$.types[{position}]");
                match read_type_node(package_identity, &meanings, &type_meanings, type_value, &at) {
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
    /// one encoder (ADR-013:113) over the `Rfc8785Numbers` view.
    fn canonical(value: &Value) -> Vec<u8> {
        quire_canonical::to_vec(&Rfc8785Numbers(value), LIMITS)
            .expect("the test value has an RFC 8785 encoding")
    }

    /// Test document bytes through intake's one parse.
    fn parse_document(bytes: &[u8]) -> PackageDocument {
        PackageDocument::parse(bytes).expect("the test document parses as JSON")
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
        let (package_ref, document) = admit(&offered, &digest_domain, &map).unwrap();
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
        let (package_ref, document) = admit(&offered, &digest_domain, &map).unwrap();
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
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
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
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
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
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
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
    fn refuses_byte_digest_mismatch() {
        let bytes = package_bytes("acme/orders", "1");
        let wrong_digest = digest_of(b"not the package");
        let actual_digest = digest_of(&canonical(&serde_json::from_slice(&bytes).unwrap()));
        let mut map = BTreeMap::new();
        map.insert(wrong_digest, bytes);
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, wrong_digest);
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::StaleDependency,
                cause: ModelRefusalCause::ByteDigestMismatch {
                    expected: wrong_digest,
                    actual: actual_digest,
                },
                detail: format!(
                    "domain package acme/orders@1 bytes hash to {}, not the selected {}",
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
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
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
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
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
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
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
        let refusal = PackageDocument::parse(b"not json").unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::IntakeMalformedDeclaration {
                    node: "$".to_owned(),
                    artifact: None,
                    span: None,
                },
                detail: "package document bytes do not parse as JSON for \
                          agent-ix-semantic-ir: an unrecognised literal at byte 0"
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

    /// QSL-194 review: RFC 8785's rounding applies to the hashed bytes
    /// only. A field whose multiplicity `upper` is 2^60 + 1 -- no double
    /// equals it -- is admitted, intake's parsed tree holds the bound
    /// exactly, and the
    /// package digest is the RFC 8785 digest of the document with that
    /// bound as the double RFC 8785 reads, 2^60, which ECMAScript spells
    /// `1152921504606847000` (`JSON.stringify(JSON.parse("1152921504606846977"))`
    /// in Node prints exactly that).
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn a_u64_bound_above_2_53_reads_exactly_and_digests_as_rfc_8785_does() {
        const EXACT: u64 = (1 << 60) + 1;
        let mut field = wire_field(
            "ix://acme/orders/Widget/count",
            "count",
            "ix://quire/native/Integer",
        );
        field["multiplicity"] =
            serde_json::json!({"lower": 0, "upper": EXACT, "ordered": false, "unique": true});
        let bytes = document_with_one_field(field);

        // The RFC 8785 text, the bound spelled as ECMAScript spells 2^60.
        let mut rounded: Value = serde_json::from_slice(&bytes).unwrap();
        rounded["types"][0]["fields"][0]["multiplicity"]["upper"] =
            Value::Number(serde_json::Number::from_f64(1_152_921_504_606_846_976.0).unwrap());
        let text = String::from_utf8(canonical(&rounded)).unwrap();
        assert!(text.contains(r#""upper":1152921504606847000}"#), "{text}");
        assert!(!text.contains(&EXACT.to_string()), "{text}");
        let digest = digest_of(text.as_bytes());

        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let (offered, digest_domain) =
            selection("acme/orders", "1.0.0", SHA256_JCS_DIGEST_DOMAIN, digest);
        let (_, document) = admit(&offered, &digest_domain, &map).expect("the document admits");
        assert_eq!(
            document.tree()["types"][0]["fields"][0]["multiplicity"]["upper"].as_u64(),
            Some(EXACT),
            "the parsed tree keeps the exact integer"
        );
        // The per-node reader then sees the exact integer too. Its FCD
        // validator admits a bound below 2^53 only, on `main` as here, so
        // the refusal names the bound -- a schema rule, not a rounded read.
        let refusals = read_records("acme/orders", &document).unwrap_err();
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert!(
            refusals[0]
                .detail
                .contains("/fields/0/multiplicity/upper (agent-ix.semantic-ir.SCHEMA_VIOLATION)"),
            "{refusals:?}"
        );
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

    /// An operation `frame` that actually declares something -- here a
    /// `creates` naming a real type of the document --
    /// refuses as a known-but-unsupported declaration form. It is never
    /// silently read as `OperationEffect::default()`, unlike an absent
    /// frame or one whose `modifies`/`creates`/`deletes` are all empty.
    #[trace("TC-146", "FR-056-AC-3")]
    #[test]
    fn refuses_a_non_empty_operation_frame_as_unsupported() {
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
        let refusals = read_records("acme/orders", &parse_document(&document))
            .expect_err("a frame that declares a creates entry is not silently dropped");
        assert_eq!(
            refusals,
            vec![ModelRefusal {
                code: Code::UnsupportedConstruct,
                cause: ModelRefusalCause::UnsupportedDeclarationForm {
                    node: "ix://acme/orders/Widget/discard".to_owned(),
                    what: "operation.frame".to_owned(),
                },
                detail: "$.types[0].operations[0].frame: construct meaning/capability \
                          \"operation.frame\" has no reader yet"
                    .to_owned(),
            }]
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

    // QSL-201: every shape the per-node reader once `expect`ed from the
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
    /// parses once, and that parse refuses it.
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
        assert_eq!(
            PackageDocument::parse(text.as_bytes()).unwrap_err(),
            malformed(
                "$",
                "package document number 1e400 has no serde_json representation"
            )
        );
    }

    /// AC-3 of QSL-201: the JCS digest check still runs over the one parse.
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
        let (package_ref, admitted) = admit(&offered, &digest_domain, &map)
            .expect("a document within the reader's 200-deep bound admits under its JCS digest");
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
            PackageDocument::parse(text.as_bytes()).unwrap_err(),
            malformed_declaration(
                "$".to_owned(),
                None,
                None,
                format!("package document carries a lone UTF-16 surrogate escape at byte {offset}"),
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
            let document = PackageDocument::parse(text.as_bytes())
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
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
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

    /// The scan's depth rule agrees with the reader's own at the boundary.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn nesting_limit_matches_the_readers_own_boundary() {
        let max = agent_ix_semantic_ir::json::MAX_DEPTH;
        let cases = [
            (nested_arrays(max - 1, "0"), true),
            (nested_arrays(max, ""), true),
            (nested_arrays(max, "0"), false),
            (nested_arrays(max, "[]"), false),
            (nested_arrays(max, "\"s\""), false),
            (nested_arrays(max - 1, "{}"), true),
            (nested_arrays(max - 1, r#"{"a":0}"#), false),
            (nested_arrays(max + 50, "0"), false),
        ];
        for (text, within) in cases {
            let label = &text[text.len() / 2 - 4..text.len() / 2 + 4];
            assert_eq!(
                agent_ix_semantic_ir::json::parse(&text).is_ok(),
                within,
                "the reader, around {label}"
            );
            assert_eq!(
                !ByteScan::of(text.as_bytes()).too_deep,
                within,
                "the scan, around {label}"
            );
            match PackageDocument::parse(text.as_bytes()) {
                Ok(_) => assert!(within, "{label}"),
                Err(refusal) => {
                    assert!(!within, "{label}: {refusal:?}");
                    assert_eq!(refusal, limit_exceeded(IntakeLimit::NestingDepth, max));
                }
            }
        }
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_an_overdeep_document_with_its_correct_digest_as_a_depth_limit() {
        let max = agent_ix_semantic_ir::json::MAX_DEPTH;
        let mut nested = serde_json::json!(0);
        for _ in 0..max {
            nested = serde_json::json!([nested]);
        }
        let document = serde_json::json!({
            "package": {"identity": "acme/orders", "version": "1"},
            "payload": nested,
        });
        let bytes = document.to_string().into_bytes();
        let digest = digest_of(&canonical(&document));
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::ResourceExhausted,
                cause: ModelRefusalCause::IntakeLimitExceeded {
                    limit: IntakeLimit::NestingDepth,
                    bound: max,
                },
                detail: format!("package document exceeds intake's nesting_depth limit of {max}"),
            }
        );
    }

    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn refuses_an_oversize_document_with_its_correct_digest_as_a_size_limit() {
        let max = agent_ix_semantic_ir::json::MAX_INPUT_BYTES;
        let head = r#"{"package":{"identity":"acme/orders","version":"1"},"pad":""#;
        let tail = r#""}"#;
        let pad = max + 1 - head.len() - tail.len();
        let bytes = format!("{head}{}{tail}", "x".repeat(pad)).into_bytes();
        assert_eq!(bytes.len(), max + 1);
        let digest = digest_of(&canonical(&serde_json::from_slice(&bytes).unwrap()));
        let mut map = BTreeMap::new();
        map.insert(digest, bytes);
        let (offered, digest_domain) =
            selection("acme/orders", "1", SHA256_JCS_DIGEST_DOMAIN, digest);
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
        assert_eq!(
            refusal,
            ModelRefusal {
                code: Code::ResourceExhausted,
                cause: ModelRefusalCause::IntakeLimitExceeded {
                    limit: IntakeLimit::InputBytes,
                    bound: max,
                },
                detail: format!("package document exceeds intake's input_bytes limit of {max}"),
            }
        );
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
        let refusal = admit(&offered, &digest_domain, &map).unwrap_err();
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

    /// PR #379 review F4: the tree and JCS bytes the one parse derives equal
    /// what `serde_json::from_slice` reads from the same bytes, for numbers
    /// at every edge, repeated member names and every string escape.
    #[trace("TC-145", "FR-056-AC-2")]
    #[test]
    fn the_one_parse_reads_what_serde_json_reads() {
        let cases = [
            // Numbers.
            "0",
            "-0",
            "-0.0",
            "1.0",
            "1e2",
            "1E+2",
            "1e-400",
            "-1e-400",
            "18446744073709551615",
            "18446744073709551616",
            "-9223372036854775808",
            "-9223372036854775809",
            "123456789012345678901234567890",
            "5e-324",
            "4.9e-324",
            "1.7976931348623157e308",
            "0.123456789012345678901234567890123456789012345678901234567890123456789012345",
            r#"[0, -0, 1.5, -2, 3e-7, -1E-2]"#,
            // Out of range for both: refused, never a different value.
            "1e400",
            "-1e400",
            // Repeated member names: the last wins.
            r#"{"a":1,"a":2}"#,
            r#"{"a":{"x":1},"a":{"y":2}}"#,
            r#"{"o":{"a":1,"a":[1]},"o":{"a":2,"b":3,"a":{"c":4}}}"#,
            r#"{"a":1,"a":2}"#,
            r#"{"a":1,"a":2}"#,
            r#"{"😀":1,"😀":2}"#,
            // Every string escape, and surrogate pairs.
            r#""\" \\ \/ \b \f \n \r \t""#,
            r#""\u0000 \u001f \u007f é € ￿ �""#,
            r#""😀 𝄞 􏿿 𐀀""#,
            r#""é 😀 raw""#,
            r#"{"k\"ey":"v\\al","\/":"\n"}"#,
        ];
        for text in cases {
            let expected = serde_json::from_slice::<Value>(text.as_bytes());
            match (PackageDocument::parse(text.as_bytes()), expected) {
                (Ok(document), Ok(expected)) => {
                    assert_eq!(document.tree(), &expected, "{text}");
                    assert_eq!(canonical(document.tree()), canonical(&expected), "{text}");
                }
                (Err(_), Err(_)) => assert!(text.ends_with("1e400"), "{text}: both refused"),
                (parsed, expected) => {
                    panic!("{text}: the one parse gave {parsed:?}, serde_json gave {expected:?}")
                }
            }
        }
    }
}

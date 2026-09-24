// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-150 producer keys and QSL-derived `quire.model.*` identity domains.
//!
//! An original declaration identity is the producer key the correspondence
//! supplies: `{package, node, digest_domain: "sha256-jcs"}`, the domain
//! package identity and the IR node identity (`model-complete.md`, "Identity
//! domains"). Every QSL-derived identity is SHA-256 over the RFC 8785 JCS
//! bytes of a schema-valid preimage. Each preimage is a typed, borrowed
//! `Serialize` view below (`*Wire`), encoded by `quire-canonical`, the one
//! RFC 8785 implementation (ADR-013 §2, ADR-013:113); the encoder orders
//! members itself, so a view's field order carries no meaning.
use std::sync::Arc;

use qsl_foundation::ByteDigest;
use serde::Serialize;

use crate::model::refusal::ModelRefusalCause;
use crate::value::semantic_node::IDENTITY_LIMITS as LIMITS;

/// The kernel's canonical `EffectiveId` (ADR-013 O-05, QC-15): 32 bytes in
/// domain `quire.model.effective-declaration/v1`, minted only through
/// [`quire_exact::EffectiveId::from_digest`]. Re-exported at this
/// path so every existing `crate::model::key::EffectiveId` import keeps
/// working unchanged; this crate no longer defines a second, model-owned
/// copy of the type (ADR-013 §9 Consequences names this fold explicitly).
/// [`EffectiveIdExt`] below adds this module's own formatting
/// conveniences, which cannot be inherent methods on a foreign-crate type
/// (Rust's orphan rule) and so live as a local extension trait instead --
/// one place for all of them, rather than ad hoc hex-building at each call
/// site.
pub use quire_exact::EffectiveId;

/// The fixed digest domain of a declaration key and a domain package
/// selection (`model-complete.md`, "Identity domains").
pub const SHA256_JCS_DIGEST_DOMAIN: &str = "sha256-jcs";

/// Digest domain of one effective declaration (`quire.model.effective-declaration/v1`).
pub const EFFECTIVE_DECLARATION_DOMAIN: &str = "quire.model.effective-declaration/v1";

/// Digest domain of one effective view (`quire.model.effective-view/v1`).
pub const EFFECTIVE_VIEW_DOMAIN: &str = "quire.model.effective-view/v1";

/// Digest domain of one object universe (`quire.model.object-universe/v1`).
pub const OBJECT_UNIVERSE_DOMAIN: &str = "quire.model.object-universe/v1";

/// The `quire.model.complete.rules/v1` manifest identity this crate reads.
pub const RULES_IDENTITY: &str = "quire.model.complete.rules/v1";

/// The pinned rules manifest revision (`complete-model-lock.json`).
pub const RULES_REVISION: &str = "1-draft.1";

/// Every derivation fact cites one of these four rules
/// (`model-effective-declaration.schema.json`, `$defs.RuleRef`); `decode` and
/// `canonicalize` never appear as a fact's rule.
pub const RULE_QUALIFY: RuleRef = RuleRef {
    identity: "quire.model.normalize.qualify/v1",
    revision: "1-draft.1",
};
/// See [`RULE_QUALIFY`].
pub const RULE_INHERIT: RuleRef = RuleRef {
    identity: "quire.model.normalize.inherit/v1",
    revision: "1-draft.1",
};
/// See [`RULE_QUALIFY`].
pub const RULE_SUBSET: RuleRef = RuleRef {
    identity: "quire.model.normalize.subset/v1",
    revision: "1-draft.1",
};
/// See [`RULE_QUALIFY`].
pub const RULE_REDEFINE: RuleRef = RuleRef {
    identity: "quire.model.normalize.redefine/v1",
    revision: "1-draft.1",
};

/// One original declaration key: `{package, node, digest_domain: "sha256-jcs"}`
/// (`model-complete.md`, "Identity domains"; `model-effective-declaration.schema.json`,
/// `$defs.DeclarationKey`). A declaration key carries no digest: two versions
/// of one domain package with equal nodes yield equal declaration keys.
///
/// Ordering matches `model-complete.md`: `package`, then `node`, each as
/// UTF-8 bytes, a proper prefix first. Field declaration order below gives
/// exactly that comparison under `derive(Ord)`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeclarationKey {
    /// The domain package identity (FR-154).
    pub package: String,
    /// The IR node identity.
    pub node: String,
}

impl DeclarationKey {
    /// A `test/orders` key over `node`, following TC-195's fixture
    /// convention: every TC-195 fixture is a domain package with identity
    /// `test/orders`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(node: impl Into<String>) -> Self {
        Self {
            package: "test/orders".to_owned(),
            node: node.into(),
        }
    }

    /// This key's RFC 8785 length.
    fn canonical_len(&self) -> u64 {
        canonical_len(&self.wire())
    }

    /// This key's `$defs.DeclarationKey` preimage view.
    pub(super) fn wire(&self) -> DeclarationKeyWire<'_> {
        DeclarationKeyWire {
            package: &self.package,
            node: &self.node,
            digest_domain: SHA256_JCS_DIGEST_DOMAIN,
        }
    }
}

/// A [`DeclarationKey`]'s preimage form: `{package, node, digest_domain}`.
#[derive(Serialize)]
pub(super) struct DeclarationKeyWire<'a> {
    package: &'a str,
    node: &'a str,
    digest_domain: &'static str,
}

/// A rule reference: `{identity, revision}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuleRef {
    /// The normative rule identity, e.g. `quire.model.normalize.qualify/v1`.
    pub identity: &'static str,
    /// The rule revision; every rule in this rung is `1-draft.1`.
    pub revision: &'static str,
}

/// A [`RuleRef`]'s preimage form: `{identity, revision}`.
#[derive(Serialize)]
pub(super) struct RuleRefWire {
    pub(super) identity: &'static str,
    pub(super) revision: &'static str,
}

impl RuleRef {
    fn wire(&self) -> RuleRefWire {
        RuleRefWire {
            identity: self.identity,
            revision: self.revision,
        }
    }
}

/// A run of producer keys that several facts share -- an ancestor path --
/// with the length of its keys' RFC 8785 array elements counted once
/// (QSL-216). Cloning shares the keys, so every fact derived along one path
/// holds that path once, not a copy each.
#[derive(Clone, Debug, Default)]
pub(crate) struct KeyPath {
    keys: Arc<[DeclarationKey]>,
    /// The encoded length of `keys` as array elements: each key's RFC 8785
    /// length, plus one separating comma between each two.
    encoded_len: u64,
}

impl KeyPath {
    /// This path extended by `key`, counting only `key`'s own encoding.
    pub(crate) fn extended(&self, key: &DeclarationKey) -> Self {
        let mut keys = Vec::with_capacity(self.keys.len() + 1);
        keys.extend_from_slice(&self.keys);
        keys.push(key.clone());
        Self {
            keys: keys.into(),
            encoded_len: joined_len(self.encoded_len, !self.keys.is_empty(), key.canonical_len()),
        }
    }

    /// The keys, in order.
    pub(crate) fn keys(&self) -> &[DeclarationKey] {
        &self.keys
    }

    /// The number of keys.
    pub(crate) fn len(&self) -> usize {
        self.keys.len()
    }
}

/// The encoded length of `extra` bytes of array elements appended after
/// `prefix` bytes of elements, with the comma between them when the prefix
/// is non-empty.
fn joined_len(prefix: u64, prefix_nonempty: bool, extra: u64) -> u64 {
    prefix
        .saturating_add(u64::from(prefix_nonempty))
        .saturating_add(extra)
}

/// A fact's ordered producer keys: a shared ancestor path, then the keys this
/// fact adds after it (QSL-216). Compares, and reads, as the one flat list
/// of keys it spells.
#[derive(Clone)]
pub struct FactInputs {
    path: KeyPath,
    tail: Vec<DeclarationKey>,
}

impl FactInputs {
    /// `path`, then `tail`.
    pub(crate) fn new(path: KeyPath, tail: Vec<DeclarationKey>) -> Self {
        Self { path, tail }
    }

    /// Every input key, in order.
    pub fn iter(&self) -> impl Iterator<Item = &DeclarationKey> {
        self.path.keys().iter().chain(&self.tail)
    }

    /// The number of input keys.
    pub fn len(&self) -> usize {
        self.path.len() + self.tail.len()
    }

    /// Whether these inputs begin with the very allocation `other`'s path
    /// holds: shared, not copied (test-only; QSL-216).
    #[cfg(test)]
    pub(crate) fn shares_path_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.path.keys, &other.path.keys)
    }

    /// Whether the fact consumed no key.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The encoded length of the keys as array elements, reading the shared
    /// path's length rather than re-encoding it.
    fn encoded_len(&self) -> u64 {
        self.tail
            .iter()
            .fold(
                (self.path.encoded_len, !self.path.keys.is_empty()),
                |(len, nonempty), key| (joined_len(len, nonempty, key.canonical_len()), true),
            )
            .0
    }
}

impl From<Vec<DeclarationKey>> for FactInputs {
    fn from(keys: Vec<DeclarationKey>) -> Self {
        let path = keys
            .iter()
            .fold(KeyPath::default(), |path, key| path.extended(key));
        Self::new(path, Vec::new())
    }
}

impl std::fmt::Debug for FactInputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl PartialEq for FactInputs {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl Eq for FactInputs {}

impl PartialEq<Vec<DeclarationKey>> for FactInputs {
    fn eq(&self, other: &Vec<DeclarationKey>) -> bool {
        self.iter().eq(other.iter())
    }
}

impl PartialEq<FactInputs> for Vec<DeclarationKey> {
    fn eq(&self, other: &FactInputs) -> bool {
        other == self
    }
}

/// One derivation fact: `{ordinal, rule, inputs}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fact {
    /// Zero-based decimal-string position in the declaration's `derivation` array.
    pub ordinal: usize,
    /// The rule that derived this fact.
    pub rule: RuleRef,
    /// Ordered producer keys the fact consumed.
    pub inputs: FactInputs,
}

/// A [`Fact`]'s preimage form: `{ordinal, rule, inputs}`, the ordinal a
/// decimal string.
#[derive(Serialize)]
struct FactWire<'a> {
    ordinal: String,
    rule: RuleRefWire,
    inputs: Vec<DeclarationKeyWire<'a>>,
}

impl Fact {
    fn wire(&self) -> FactWire<'_> {
        FactWire {
            ordinal: self.ordinal.to_string(),
            rule: self.rule.wire(),
            inputs: self.inputs.iter().map(DeclarationKey::wire).collect(),
        }
    }

    /// This fact's RFC 8785 length: the fact with no inputs, plus its
    /// inputs' element length. `"inputs":[]` grows by exactly that length.
    fn canonical_len(&self) -> u64 {
        let bare = FactWire {
            ordinal: self.ordinal.to_string(),
            rule: self.rule.wire(),
            inputs: Vec::new(),
        };
        canonical_len(&bare).saturating_add(self.inputs.encoded_len())
    }
}

/// [`EffectiveId`]'s formatting conveniences owned by this module (ADR-013
/// O-05's model side), not by the kernel: the kernel type has one public
/// constructor (`from_digest`) and no preimage or JSON knowledge at all
/// (ADR-013 T-6: "the kernel imports none of them").
pub trait EffectiveIdExt {
    /// The first eight hex digits, as TC-195's vectors abbreviate identities.
    fn short_hex(&self) -> String;
}

impl EffectiveIdExt for EffectiveId {
    fn short_hex(&self) -> String {
        hex(&self.as_bytes()[..4])
    }
}

/// An [`EffectiveId`]'s `{domain, digest}` preimage form
/// (`model-effective-declaration.schema.json`). A local view rather than a
/// `Serialize` impl, which the orphan rule forbids on the kernel type.
#[derive(Serialize)]
pub(super) struct EffectiveIdWire {
    domain: &'static str,
    digest: String,
}

impl From<&EffectiveId> for EffectiveIdWire {
    fn from(id: &EffectiveId) -> Self {
        Self {
            domain: EFFECTIVE_DECLARATION_DOMAIN,
            digest: id.to_string(),
        }
    }
}

/// One effective declaration's preimage: `{version, owner_effective_type,
/// original, derivation}`. `owner_effective_type` is `None` for an effective
/// type, `Some` for an effective member of that owning effective type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveDeclarationPreimage {
    /// `None` for an effective type; the owning effective type's identity for
    /// an effective member.
    pub owner_effective_type: Option<EffectiveId>,
    /// The original producer declaration this effective declaration derives from.
    pub original: DeclarationKey,
    /// The ordered derivation facts.
    pub derivation: Vec<Fact>,
}

/// An [`EffectiveDeclarationPreimage`]'s preimage form: `{version,
/// owner_effective_type, original, derivation}`, `owner_effective_type`
/// `null` for an effective type.
#[derive(Serialize)]
pub(super) struct EffectiveDeclarationWire<'a> {
    version: &'static str,
    owner_effective_type: Option<EffectiveIdWire>,
    original: DeclarationKeyWire<'a>,
    derivation: Vec<FactWire<'a>>,
}

impl EffectiveDeclarationPreimage {
    /// This preimage's schema-valid form (`pub(super)` so
    /// `crate::model::normalize` embeds it in the effective view).
    pub(super) fn wire(&self) -> EffectiveDeclarationWire<'_> {
        EffectiveDeclarationWire {
            version: EFFECTIVE_DECLARATION_DOMAIN,
            owner_effective_type: self
                .owner_effective_type
                .as_ref()
                .map(EffectiveIdWire::from),
            original: self.original.wire(),
            derivation: self.derivation.iter().map(Fact::wire).collect(),
        }
    }

    /// This preimage's `quire.model.effective-declaration/v1` identity,
    /// encoded by `quire-canonical` (ADR-013 §2, ADR-013:113: one RFC 8785
    /// implementation).
    pub fn identity(&self) -> EffectiveId {
        self.identity_and_canonical_len().0
    }

    /// The length of this preimage's RFC 8785 bytes (for `normalize.hash`
    /// accounting), counted by the encoder.
    pub fn canonical_len(&self) -> u64 {
        canonical_len(&self.wire())
    }

    /// This preimage's RFC 8785 length, counted from its parts without
    /// encoding its facts' shared paths (QSL-216): the preimage with no
    /// derivation, plus each fact's length and the commas between them.
    /// Equal to [`Self::canonical_len`], so `normalize.hash` is charged
    /// before the preimage is encoded and hashed.
    pub(crate) fn canonical_len_from_parts(&self) -> u64 {
        let bare = EffectiveDeclarationWire {
            version: EFFECTIVE_DECLARATION_DOMAIN,
            owner_effective_type: self
                .owner_effective_type
                .as_ref()
                .map(EffectiveIdWire::from),
            original: self.original.wire(),
            derivation: Vec::new(),
        };
        self.derivation
            .iter()
            .enumerate()
            .fold(canonical_len(&bare), |len, (position, fact)| {
                joined_len(len, position > 0, fact.canonical_len())
            })
    }

    /// This preimage's identity and its RFC 8785 byte length, from one
    /// encoding: the encoder hashes as it encodes and returns the count.
    pub(super) fn identity_and_canonical_len(&self) -> (EffectiveId, u64) {
        let (digest, len) = sha256_and_len(&self.wire());
        (EffectiveId::from_digest(digest), len)
    }

    /// Whether `derivation` is well-formed: every fact's `ordinal` matches
    /// its array position (`unsorted-derivation`), and no two facts retain
    /// the same input path twice (`duplicate-path`) — TC-195 N10's
    /// `invalid_mutations` named these "refused by the semantic check", i.e.
    /// this engine's own job over an already-constructed preimage (for
    /// example one read back from a checked-package `model_correspondence`
    /// node), not a wire decode (PR #140 F5). Returns `(cause, detail)` on
    /// the first defect found, in derivation order.
    #[allow(
        clippy::result_large_err,
        reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
    )]
    pub fn validate_derivation(&self) -> Result<(), (ModelRefusalCause, String)> {
        for (position, fact) in self.derivation.iter().enumerate() {
            if fact.ordinal != position {
                return Err((
                    ModelRefusalCause::UnsortedDerivation {
                        original: self.original.clone(),
                        position,
                        ordinal: fact.ordinal,
                    },
                    format!(
                        "{} derivation fact at position {position} has ordinal {}, not {position}",
                        self.original.node, fact.ordinal
                    ),
                ));
            }
        }
        for earlier in 0..self.derivation.len() {
            for later in (earlier + 1)..self.derivation.len() {
                if self.derivation[earlier].inputs == self.derivation[later].inputs {
                    return Err((
                        ModelRefusalCause::DuplicatePath {
                            original: self.original.clone(),
                            earlier,
                            later,
                        },
                        format!(
                            "{} derivation retains the same input path at positions {earlier} and {later}",
                            self.original.node
                        ),
                    ));
                }
            }
        }
        Ok(())
    }
}

/// A lowercase hex encoding of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The SHA-256 digest of `preimage`'s RFC 8785 bytes and their length, from
/// one encoding by `quire-canonical` (ADR-013 §2, ADR-013:113: the one RFC
/// 8785 implementation). Every preimage here is a top-level object, which the
/// encoder buffers whole before it reaches any sink, so collecting the bytes
/// and hashing them after costs no extra copy of note.
///
/// Every preimage this module and `normalize`/`population` hand here is a
/// typed view of strings, `null`s, arrays and objects with fixed ASCII member
/// names: it has an RFC 8785 encoding, and [`LIMITS`] sets no byte ceiling
/// and a depth far above any view's fixed nesting. The one refusal left is a
/// failed heap reservation for an object's buffered members, which the
/// `serde_json` encoder this replaced aborted the process on; this panics
/// with the encoder's reason instead.
pub(super) fn sha256_and_len(preimage: &impl Serialize) -> ([u8; 32], u64) {
    let bytes = quire_canonical::to_vec(preimage, LIMITS)
        .unwrap_or_else(|error| panic!("a typed identity preimage encodes: {error}"));
    let len = quire_exact::length_amount(bytes.len());
    (ByteDigest::of(&bytes).as_bytes(), len)
}

/// The length of `preimage`'s RFC 8785 bytes, counted by `quire-canonical`
/// while it encodes into a discarding sink. No `serde_json::Value` is built
/// and nothing is kept once the call returns; while it runs, the encoder
/// still buffers each open object's members to sort them (a top-level
/// object in full), as every encoding does. Refuses only as
/// [`sha256_and_len`] does.
pub(super) fn canonical_len(preimage: &impl Serialize) -> u64 {
    quire_canonical::encode(
        &mut quire_canonical::WriteSink(std::io::sink()),
        preimage,
        LIMITS,
    )
    .unwrap_or_else(|error| panic!("a typed identity preimage encodes: {error}"))
}

/// The SHA-256 of `bytes` exactly as given, with no canonicalization: intake's
/// check-3 digest of package bytes that did not parse, which have no RFC 8785
/// form. Kept here, beside the canonical digests, so intake -- which reads
/// JSON -- names no hasher itself (arch-lint `canonical-encoder`).
pub(super) fn raw_bytes_digest(bytes: &[u8]) -> [u8; 32] {
    ByteDigest::of(bytes).as_bytes()
}

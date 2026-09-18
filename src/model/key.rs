// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-150 producer keys and QSL-derived `quire.model.*` identity domains.
//!
//! An original declaration identity is the producer key the correspondence
//! supplies (`model-complete.md`, "Identity domains"). Every QSL-derived
//! identity is SHA-256 over the RFC 8785 JCS bytes of a schema-valid preimage,
//! built here as a `serde_json::Value`: `serde_json::Map` is a `BTreeMap`
//! (this crate selects no `preserve_order` feature), so `serde_json::to_vec`
//! already emits ascending-key, whitespace-free bytes for every ASCII member
//! name this schema defines, which is RFC 8785 JCS for these preimages.

use std::fmt;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::model::refusal::ModelRefusalCause;
use crate::value::length_amount;

/// Digest domain of every producer digest selection (`filament-canonical-json-1`).
pub const PRODUCER_DIGEST_DOMAIN: &str = "filament-canonical-json-1";

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

/// A namespaced producer revision label (FCD FR-113).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Revision {
    /// The revision namespace, e.g. `filament-core-data/producer-object-revision-1`.
    pub namespace: String,
    /// The revision value within that namespace.
    pub value: String,
}

impl Revision {
    /// A revision in the standard producer-object-revision namespace.
    pub fn producer_object(value: impl Into<String>) -> Self {
        Self {
            namespace: "filament-core-data/producer-object-revision-1".to_owned(),
            value: value.into(),
        }
    }

    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.insert(
            "namespace".to_owned(),
            Value::String(self.namespace.clone()),
        );
        object.insert("value".to_owned(), Value::String(self.value.clone()));
        Value::Object(object)
    }
}

/// A producer digest selection: `{domain, sha256}`.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProducerDigest {
    /// The digest domain; every fixture in this rung is `filament-canonical-json-1`.
    pub domain: String,
    /// The lowercase 64-hex SHA-256 digest.
    pub sha256: [u8; 32],
}

impl ProducerDigest {
    /// A `filament-canonical-json-1` digest over `identity`'s exact UTF-8 bytes,
    /// following TC-195's fixture convention.
    ///
    /// Test-only (PR #140 F13): this derives a digest from a display
    /// identity, which is exactly the name-derived-identity defect this
    /// engine exists to exclude. Gated behind `test-support` so a production
    /// caller cannot reach it; `cargo test --all-features` enables it for
    /// `tests/model_normalization.rs`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn of_identity(identity: &str) -> Self {
        Self {
            domain: PRODUCER_DIGEST_DOMAIN.to_owned(),
            sha256: Sha256::digest(identity.as_bytes()).into(),
        }
    }

    fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.insert("domain".to_owned(), Value::String(self.domain.clone()));
        object.insert("sha256".to_owned(), Value::String(hex(&self.sha256)));
        Value::Object(object)
    }
}

impl fmt::Debug for ProducerDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProducerDigest")
            .field("domain", &self.domain)
            .field("sha256", &hex(&self.sha256))
            .finish()
    }
}

/// One producer's original-declaration key: `{authority, identity, revision, digest}`.
///
/// Ordering matches `model-complete.md`: `authority`, `identity`,
/// `revision.namespace`, `revision.value`, `digest.domain`, `digest.sha256`,
/// each as UTF-8 bytes, a proper prefix first. Field declaration order below
/// gives exactly that comparison under `derive(Ord)`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProducerKey {
    /// The correspondence `ProducerObject.authority` (FCD FR-116).
    pub authority: String,
    /// The original declaration's producer identity string.
    pub identity: String,
    /// The namespaced producer revision.
    pub revision: Revision,
    /// The `filament-canonical-json-1` digest selection.
    pub digest: ProducerDigest,
}

impl ProducerKey {
    /// A `filament-core-data` key over `identity`, following TC-195/196/197/198's
    /// fixture convention: revision value `"1"`, digest over `identity`'s bytes.
    ///
    /// Test-only (PR #140 F13): see [`ProducerDigest::of_identity`].
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(identity: impl Into<String>) -> Self {
        let identity = identity.into();
        Self {
            authority: "filament-core-data".to_owned(),
            digest: ProducerDigest::of_identity(&identity),
            revision: Revision::producer_object("1"),
            identity,
        }
    }

    pub(super) fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.insert(
            "authority".to_owned(),
            Value::String(self.authority.clone()),
        );
        object.insert("identity".to_owned(), Value::String(self.identity.clone()));
        object.insert("revision".to_owned(), self.revision.to_json());
        object.insert("digest".to_owned(), self.digest.to_json());
        Value::Object(object)
    }
}

/// A rule reference: `{identity, revision}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuleRef {
    /// The normative rule identity, e.g. `quire.model.normalize.qualify/v1`.
    pub identity: &'static str,
    /// The rule revision; every rule in this rung is `1-draft.1`.
    pub revision: &'static str,
}

impl RuleRef {
    pub(super) fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.insert(
            "identity".to_owned(),
            Value::String(self.identity.to_owned()),
        );
        object.insert(
            "revision".to_owned(),
            Value::String(self.revision.to_owned()),
        );
        Value::Object(object)
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
    pub inputs: Vec<ProducerKey>,
}

impl Fact {
    pub(super) fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.insert(
            "ordinal".to_owned(),
            Value::String(self.ordinal.to_string()),
        );
        object.insert("rule".to_owned(), self.rule.to_json());
        object.insert(
            "inputs".to_owned(),
            Value::Array(self.inputs.iter().map(ProducerKey::to_json).collect()),
        );
        Value::Object(object)
    }
}

/// A `quire.model.effective-declaration/v1` identity: an effective type or an
/// effective member of an effective type.
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EffectiveId([u8; 32]);

impl EffectiveId {
    /// An effective declaration identity from its raw 32-byte digest, with no
    /// re-hash. FR-143's reference identity triple carries a
    /// `quire.model.effective-declaration/v1` digest as a reference's
    /// most-specific type, so the bridge between a model reference and a
    /// `crate::value` `Reference<T>` (`crate::value::node::NodeKey`, the same
    /// 32 raw bytes under a different domain tag) is a direct byte transfer.
    pub(crate) fn from_digest_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The raw 32-byte digest.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The first eight hex digits, as TC-195's vectors abbreviate identities.
    pub fn short_hex(&self) -> String {
        hex(&self.0[..4])
    }

    /// The full 64-hex digest.
    pub fn hex(&self) -> String {
        hex(&self.0)
    }

    pub(super) fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.insert(
            "domain".to_owned(),
            Value::String(EFFECTIVE_DECLARATION_DOMAIN.to_owned()),
        );
        object.insert("digest".to_owned(), Value::String(self.hex()));
        Value::Object(object)
    }
}

impl fmt::Debug for EffectiveId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EffectiveId({})", self.hex())
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
    pub original: ProducerKey,
    /// The ordered derivation facts.
    pub derivation: Vec<Fact>,
}

impl EffectiveDeclarationPreimage {
    /// This preimage's schema-valid JSON form (PR #140 F10: `pub(super)` so
    /// `crate::model::normalize` can reuse it directly instead of
    /// round-tripping through JCS bytes and back).
    pub(super) fn to_json(&self) -> Value {
        let mut object = Map::new();
        object.insert(
            "version".to_owned(),
            Value::String(EFFECTIVE_DECLARATION_DOMAIN.to_owned()),
        );
        object.insert(
            "owner_effective_type".to_owned(),
            match &self.owner_effective_type {
                Some(id) => id.to_json(),
                None => Value::Null,
            },
        );
        object.insert("original".to_owned(), self.original.to_json());
        object.insert(
            "derivation".to_owned(),
            Value::Array(self.derivation.iter().map(Fact::to_json).collect()),
        );
        Value::Object(object)
    }

    /// This preimage's `quire.model.effective-declaration/v1` identity.
    pub fn identity(&self) -> EffectiveId {
        digest_of(&self.to_json())
    }

    /// The exact JCS bytes of this preimage (for `normalize.hash` accounting).
    pub fn jcs_bytes(&self) -> Vec<u8> {
        jcs_bytes(&self.to_json())
    }

    /// This preimage's identity and its JCS byte length, from one `to_json`
    /// build rather than two (PR #140 F10: `identity()` and `jcs_bytes()`
    /// each independently rebuilt the same JSON value).
    pub(super) fn identity_and_jcs_len(&self) -> (EffectiveId, u64) {
        let bytes = jcs_bytes(&self.to_json());
        (digest_of_bytes(&bytes), length_amount(bytes.len()))
    }

    /// Whether `derivation` is well-formed: every fact's `ordinal` matches
    /// its array position (`unsorted-derivation`), and no two facts retain
    /// the same input path twice (`duplicate-path`) — TC-195 N10's
    /// `invalid_mutations` named these "refused by the semantic check", i.e.
    /// this engine's own job over an already-constructed preimage (for
    /// example one read back from a checked-package `model_correspondence`
    /// node), not a wire decode (PR #140 F5). Returns `(cause, detail)` on
    /// the first defect found, in derivation order.
    pub fn validate_derivation(&self) -> Result<(), (ModelRefusalCause, String)> {
        for (position, fact) in self.derivation.iter().enumerate() {
            if fact.ordinal != position {
                return Err((
                    ModelRefusalCause::UnsortedDerivation,
                    format!(
                        "{} derivation fact at position {position} has ordinal {}, not {position}",
                        self.original.identity, fact.ordinal
                    ),
                ));
            }
        }
        for earlier in 0..self.derivation.len() {
            for later in (earlier + 1)..self.derivation.len() {
                if self.derivation[earlier].inputs == self.derivation[later].inputs {
                    return Err((
                        ModelRefusalCause::DuplicatePath,
                        format!(
                            "{} derivation retains the same input path at positions {earlier} and {later}",
                            self.original.identity
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

/// The exact RFC 8785 JCS bytes of `value` under this crate's canonical
/// `serde_json::Value` serialization.
pub(super) fn jcs_bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).expect("a constructed preimage value always serializes")
}

/// The SHA-256 digest of `value`'s JCS bytes, as an [`EffectiveId`].
pub(super) fn digest_of(value: &Value) -> EffectiveId {
    digest_of_bytes(&jcs_bytes(value))
}

/// The SHA-256 digest of already-serialized JCS `bytes`, as an [`EffectiveId`].
fn digest_of_bytes(bytes: &[u8]) -> EffectiveId {
    EffectiveId(Sha256::digest(bytes).into())
}

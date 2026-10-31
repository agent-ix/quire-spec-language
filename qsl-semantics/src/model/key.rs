// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-150 producer keys and structural model identities.
//!
//! An original declaration identity is the producer key the correspondence
//! supplies: `{package, node}`, the domain package identity and the IR node
//! identity (`model-complete.md`, "Identity domains"). An effective identity
//! is the structural [`EffectiveId`] chain of those original keys; a
//! declaration's node identity is the IR `declaration_id` recipe
//! ([`model_node_key`]). Nothing here hashes or parses the opaque node
//! string.
//!
//! `DeclarationKey` is a plain value of `quire-checked` (FR-400, ADR-011 X-13)
//! and is not re-exported here. Its old path no longer resolves:
//!
//! ```compile_fail,E0603
//! use qsl_semantics::model::key::DeclarationKey;
//! ```
//!
//! Its new owner does:
//!
//! ```no_run
//! use quire_checked::key::DeclarationKey;
//! let _ = DeclarationKey { package: String::new(), node: String::new() };
//! ```
use quire_canonical::{FixedShape, Limits};
use quire_checked::key::DeclarationKey;
use quire_exact::{InvalidNodeKey, NodeKey, OriginalDeclarationKey};
use serde::Serialize;

use crate::model::normalize::ModelRefusal;
use crate::model::refusal::ModelRefusalCause;
use qsl_foundation::diagnostic::Code;

/// The model stage's identity text encoding limits: no byte bound of their
/// own; a declaration key is a field of a domain package that intake's
/// `intake.input_bytes` already bounded.
const LIMITS: Limits = Limits::new(u64::MAX);

/// The kernel's structural `EffectiveId` (QSpec FR-150): the owner chain of
/// original declaration keys. Re-exported at this path so every
/// `crate::model::key::EffectiveId` import keeps working unchanged.
pub use quire_exact::EffectiveId;

/// The structural node identity of the model declaration `key` names under
/// `form_tag`/`form`: the IR `declaration_id` recipe
/// `{package: identity, path: [tag, form, node], ordinal: 0}`. The opaque
/// node string is one segment, never split or parsed.
///
/// # Errors
///
/// [`InvalidNodeKey`] when the package is empty or storage cannot be reserved.
pub fn model_node_key(
    package: &str,
    form_tag: &str,
    form: &str,
    node: &str,
) -> Result<NodeKey, InvalidNodeKey> {
    crate::check::declared_node_key(
        package.to_owned(),
        vec![form_tag.to_owned(), form.to_owned(), node.to_owned()],
        0,
    )
}

/// The structural node identity of the object type `key` declares.
///
/// # Errors
///
/// As [`model_node_key`].
pub fn object_type_node_key(key: &DeclarationKey) -> Result<NodeKey, InvalidNodeKey> {
    model_node_key(&key.package, "model", "object_type", &key.node)
}

/// The structural node identity of the population `key` declares.
///
/// # Errors
///
/// As [`model_node_key`].
pub fn population_node_key(key: &DeclarationKey) -> Result<NodeKey, InvalidNodeKey> {
    model_node_key(&key.package, "relation", "population", &key.node)
}

/// The structural effective identity of `original` under `owner`.
#[must_use]
pub fn effective_id_of(owner: Option<EffectiveId>, original: &DeclarationKey) -> EffectiveId {
    EffectiveId::new(
        owner,
        OriginalDeclarationKey::new(original.package.clone(), original.node.clone()),
    )
}

/// The `quire.model.complete.rules/v1` manifest identity this crate reads.
pub const RULES_IDENTITY: &str = "quire.model.complete.rules/v1";

/// Every derivation fact cites one of these four rules
/// (`model-effective-declaration.schema.json`, `$defs.RuleRef`); `decode` and
/// `canonicalize` never appear as a fact's rule.
pub const RULE_QUALIFY: RuleRef = RuleRef {
    identity: "quire.model.normalize.qualify/v1",
};
/// See [`RULE_QUALIFY`].
pub const RULE_INHERIT: RuleRef = RuleRef {
    identity: "quire.model.normalize.inherit/v2",
};
/// See [`RULE_QUALIFY`].
pub const RULE_SUBSET: RuleRef = RuleRef {
    identity: "quire.model.normalize.subset/v2",
};
/// See [`RULE_QUALIFY`].
pub const RULE_REDEFINE: RuleRef = RuleRef {
    identity: "quire.model.normalize.redefine/v2",
};

/// A [`DeclarationKey`]'s `$defs.DeclarationKey` preimage view. The key is a
/// plain value in `quire-checked`; its identity encoding stays with the model
/// stage that mints identities (FR-400).
pub(super) fn declaration_key_wire(key: &DeclarationKey) -> DeclarationKeyWire<'_> {
    DeclarationKeyWire {
        package: &key.package,
        node: &key.node,
    }
}

/// The RFC 8785 text of `key`'s preimage (FR-120 "Identities"), the identity
/// text a model transition carries for the declaration `key` names.
pub fn declaration_identity_text(key: &DeclarationKey) -> String {
    let bytes = quire_canonical::to_vec(&declaration_key_wire(key), LIMITS)
        .unwrap_or_else(|error| panic!("a declaration key encodes: {error}"));
    String::from_utf8(bytes).unwrap_or_else(|error| panic!("RFC 8785 text is UTF-8: {error}"))
}

/// A [`DeclarationKey`]'s preimage form: `{package, node}`.
#[derive(Serialize, FixedShape)]
pub(super) struct DeclarationKeyWire<'a> {
    package: &'a str,
    node: &'a str,
}

/// A rule reference: `{identity}`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuleRef {
    /// The normative rule identity, e.g. `quire.model.normalize.qualify/v1`.
    pub identity: &'static str,
}

/// One derivation fact: `{ordinal, rule, inputs, parent}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fact {
    /// Zero-based decimal-string position in the declaration's `derivation` array.
    pub ordinal: usize,
    /// The rule that derived this fact.
    pub rule: RuleRef,
    /// Ordered producer keys the fact consumed.
    pub inputs: Vec<DeclarationKey>,
    /// The effective declaration an `inherit` fact links to; `None` for
    /// every other rule.
    pub parent: Option<EffectiveId>,
}

/// One effective declaration's structural facts: the owner, the original
/// producer key and the derivation. `owner_effective_type` is `None` for an
/// effective type, `Some` for an effective member of that owning effective type.
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

impl EffectiveDeclarationPreimage {
    /// This declaration's structural effective identity: its original key
    /// under its owner.
    pub fn identity(&self) -> EffectiveId {
        effective_id_of(self.owner_effective_type.clone(), &self.original)
    }

    /// Whether `derivation` is well-formed: every fact's `ordinal` matches
    /// its array position (`unsorted-derivation`), and no two facts retain
    /// the same rule and input path twice (`duplicate-path`) — TC-195 N10's
    /// `invalid_mutations` named these "refused by the semantic check", i.e.
    /// this engine's own job over an already-constructed preimage (for
    /// example one read back from a checked-package `model_correspondence`
    /// node), not a wire decode (PR #140 F5). Returns `(cause, detail)` on
    /// the first defect found, in derivation order.
    #[allow(
        clippy::result_large_err,
        reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline for typed model-admission failures"
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
                if self.derivation[earlier].rule == self.derivation[later].rule
                    && self.derivation[earlier].inputs == self.derivation[later].inputs
                {
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

/// The refusal for a node identity that cannot be carried structurally: a
/// failed reservation is resource exhaustion, any other shape is a malformed
/// declaration.
#[must_use]
pub fn node_key_refusal(error: &InvalidNodeKey) -> ModelRefusal {
    match error {
        InvalidNodeKey::Allocation { requested } => ModelRefusal {
            code: Code::ResourceExhausted,
            cause: ModelRefusalCause::AllocationFailed {
                requested: *requested,
            },
            detail: format!("a structural node identity could not be reserved: {error}"),
        },
        other => ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::MalformedDeclaration,
            detail: format!("a declaration has no structural node identity: {other}"),
        },
    }
}

/// The refusal for an object universe that has no structural identity.
#[must_use]
pub fn universe_identity_refusal(
    error: &crate::model::normalize::UniverseIdentityError,
) -> ModelRefusal {
    match error {
        crate::model::normalize::UniverseIdentityError::Key(key) => node_key_refusal(key),
        crate::model::normalize::UniverseIdentityError::NoRootType => ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::MalformedDeclaration,
            detail: "the object universe declares no root object type".to_owned(),
        },
    }
}

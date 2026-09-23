// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-013 O-08 to O-11, O-14 and C-26 (FR-088, QSL-158 S-3b): frame and
//! clause identity, clause kind, qualified names and checked type
//! descriptors. FR-088's own scope note applies throughout: this module
//! builds the identity and resolution shapes S-3b owns; it does not build
//! `KaniObligationIdentity` (CG conformance work, FR-088-CON-1), FR-340's
//! frame semantics (#210, FR-088-CON-2), the occurrence-key-keyed source map
//! (S-4, FR-088-CON-3) or the `replay` facade's E9 lookup (FR-088-CON-4).
//!
//! # Naming: `CheckedClauseKind`, not `ClauseKind`
//!
//! `qsl_forms::ClauseKind` already names a different, pre-existing
//! concept this same `check` module imports: the value-expression dispatch
//! context (`Invariant`/`Precondition`/`Postcondition`/`Body`, FR-151). This
//! module's [`CheckedClauseKind`] (ADR-013 O-10) is not a variant or a
//! replacement of that type -- it is the canonical native-runtime clause
//! kind (claim/temporal/protocol/state-transition) ADR-013 §3 O-10 names.
//! `syntax::ClauseKind` (native-v1, three variants: `Invariant`,
//! `Precondition`, `Postcondition`) is the third, lane-private clause-kind
//! enum ADR-013 §6 names; it is untouched and gains no new variant or
//! consumer here (FR-088-CON-5).
//!
//! # Confinement (R-06)
//!
//! This module (`identity`) is a private submodule of `check` (declared
//! `mod identity;`, not `pub mod`), so no path outside `crate::check` can
//! name anything in it directly. [`check::mod`](super) re-exports the
//! public shapes below at `crate::check`'s own surface. `CheckedGraph`
//! itself (`super::CheckedGraph`) exposes only a node-id-keyed accessor for
//! the model correspondence ([`CheckedGraph::resolve_declaration`](super::CheckedGraph::resolve_declaration)), never a
//! name-keyed one, matching R-06's "no name -> identity lookup exists after
//! the check stage" (FR-088-AC-5).
//!
//! # PR #300 review: wiring, and round 2's HIGH-1 correction
//!
//! [`to_kernel_value_type`] is called from real `check` production code
//! (`super::PackageDeclarations::check`), not only from this module's own
//! unit tests (round 1 finding 1): every admitted composite and enum
//! declaration in a checked package's `TypeEnvironment` becomes a real
//! [`CheckedTypeNode`], and `CheckedGraph` exposes them by node id
//! (`CheckedGraph::checked_type_node`).
//!
//! **Round 1 minted a second, parallel type identity; round 2 (HIGH-1)
//! deletes it.** Round 1's `mint_type_declaration_identity` minted a fresh
//! node id from a declaration's name and shape -- an id nothing else in the
//! checker ever read, since `type_named` (`check.rs`), field types
//! (`family.rs`) and `EnumValue`'s own member keys all still resolved
//! through `composite.key()`/`EnumDeclaration::key()`, the declaration's
//! pre-existing identity. FR-088-AC-7 requires every reference to resolve to
//! "the one node id the single declaration was minted with" -- a second,
//! unused id cannot satisfy that. The fix is not to build a third scheme:
//! `check` was never the minter of a composite's or enum's own declaration
//! identity in the first place (`CompositeDeclaration`'s key is
//! producer-assigned, `value/declaration.rs`; `EnumDeclaration`'s key is
//! verified against its own content-addressed preimage at `admit`,
//! `value/enumeration.rs`, whose `owner` field already carries the
//! declaring package/source scope AC-7 requires). `check` now simply carries
//! that existing key, unchanged byte for byte, into the `CheckedTypeNode`'s
//! own node id: `CheckedTypeNode::Composite { node: composite.key() }`,
//! `Sum { node: binding.declaration.key(), .. }`. Both declarations hold the
//! kernel `quire_exact::NodeKey` this module uses, so carrying the key
//! across the module boundary re-hashes or
//! re-derives nothing and needs no conversion function -- exactly the
//! identity `type_named` and field types already read (once carried into
//! this space) -- closing HIGH-1 and, as a side effect, HIGH-2 (a qualified
//! declared name like `"P::R"` no longer needs validating at all here, since
//! no name-keyed preimage is minted from it). [`mint_variant_id`] stays:
//! AC-10's `VariantId` is still computed from the declaring sum and its
//! member, now from the sum's own existing (converted) key rather than a
//! minted one.
//!
//! The model correspondence ([`ModelCorrespondence`]) is populated from
//! `PackageDeclarations`' own `model_correspondence` field
//! (`check/check.rs`), matching this file's own pre-existing
//! `dispatch_operations`/`dispatch_tables` precedent of "built by the caller
//! (the `model` bridge); the checker only records/resolves against it" --
//! see that field's doc for why no such domain-declaration bridge exists in
//! production yet, and `mod.rs`'s test module for tests that go through
//! `CheckedPackage::graph().resolve_declaration`/`checked_type_node`, not a
//! hand-built `ModelCorrespondence`.

use std::collections::{BTreeMap, BTreeSet};

use quire_exact::{
    DecimalType, EnumShape, Identifier, IeeeWidth, IntegerInterval, NodeKey, RationalDomain,
    TextType, UnitId, ValueType,
};

use crate::model::key::DeclarationKey;
use crate::value::enumeration::mint_variant_id;

// ---------------------------------------------------------------------
// O-10: clause kind
// ---------------------------------------------------------------------

/// ADR-013 O-10: one closed checked clause-kind enum, defined once in the
/// layer-3 `check` core (FR-088-AC-1). See this module's own doc for why it
/// is not named `ClauseKind`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CheckedClauseKind {
    /// A `claim` node: a native-runtime boolean claim.
    Claim,
    /// A `temporal` node: a native-runtime temporal formula.
    Temporal,
    /// A `protocol` node: a native-runtime protocol control.
    Protocol,
    /// A `state` node with `semantic_form: "frame"` (ADR-013 O-08): the
    /// frame identity this module also builds.
    StateTransition,
}

impl CheckedClauseKind {
    /// `kind`'s own successor in `all()`'s enumeration order, or the first
    /// variant when `kind` is `None`, or `None` after the last variant.
    /// Exhaustive over `Self` with no `_` arm (PR #300 review finding 8/
    /// round 2 L6): adding a fifth variant to the enum without adding its
    /// own arm here is a missing-match-arm compile error, not merely a
    /// match that stays green while a *separate* array literal silently
    /// forgets it -- this chain is the one place a variant's position in
    /// `all()` comes from.
    fn next(kind: Option<Self>) -> Option<Self> {
        match kind {
            None => Some(Self::Claim),
            Some(Self::Claim) => Some(Self::Temporal),
            Some(Self::Temporal) => Some(Self::Protocol),
            Some(Self::Protocol) => Some(Self::StateTransition),
            Some(Self::StateTransition) => None,
        }
    }

    /// Every variant, for a totality/injectivity scan over the whole
    /// vocabulary, walked from `Self::next`'s own exhaustive chain (private,
    /// so not linked here). The
    /// trailing `debug_assert` closes the one gap `next`'s exhaustiveness
    /// alone cannot: a variant added to the chain but not to this walk
    /// would leave `next(Some(d))` still `Some(_)`, which fails loudly
    /// (in every debug-mode test that calls `all()`, starting with TC-250's
    /// own totality test below) rather than silently returning a
    /// four-element array one short of the real vocabulary.
    pub fn all() -> [Self; 4] {
        let a = Self::next(None).expect("Self::next(None) is always Some(Self::Claim)");
        let b = Self::next(Some(a)).expect("Claim is always followed by Temporal");
        let c = Self::next(Some(b)).expect("Temporal is always followed by Protocol");
        let d = Self::next(Some(c)).expect("Protocol is always followed by StateTransition");
        debug_assert!(
            Self::next(Some(d)).is_none(),
            "CheckedClauseKind::next has a variant after StateTransition that \
             CheckedClauseKind::all does not walk to -- extend this walk to match"
        );
        [a, b, c, d]
    }

    /// This variant's exactly-one v2 clause-operation identity (ADR-013
    /// O-10's own wire vocabulary: `quire.op.claim.clause`,
    /// `quire.op.temporal.clause`, `quire.op.protocol.control`,
    /// `quire.op.state.transition`). Total and injective over the four
    /// variants, with no `_` arm (FR-088-AC-4).
    pub fn wire_operation_identity(self) -> &'static str {
        match self {
            Self::Claim => "quire.op.claim.clause",
            Self::Temporal => "quire.op.temporal.clause",
            Self::Protocol => "quire.op.protocol.control",
            Self::StateTransition => "quire.op.state.transition",
        }
    }

    /// The variant whose [`Self::wire_operation_identity`] is exactly
    /// `wire`, or `None` when `wire` names none of them: backward totality
    /// is over exactly this enum's own closed wire vocabulary, not an open
    /// string set (FR-088-AC-4).
    pub fn from_wire_operation_identity(wire: &str) -> Option<Self> {
        match wire {
            "quire.op.claim.clause" => Some(Self::Claim),
            "quire.op.temporal.clause" => Some(Self::Temporal),
            "quire.op.protocol.control" => Some(Self::Protocol),
            "quire.op.state.transition" => Some(Self::StateTransition),
            _ => None,
        }
    }

    /// This variant's own v2 syntax `node_tag`, with a `semantic_form` where
    /// the node kind is overloaded (ADR-013 O-08/O-10): `claim`, `temporal`
    /// and `protocol` are each their own dedicated node kind with no
    /// `semantic_form` disambiguation needed (this type's own per-variant
    /// doc comments above already name them so); [`Self::StateTransition`]
    /// is the `state` node kind's `semantic_form: "frame"` case specifically
    /// (its own doc). FR-088-AC-4's "node_tag/semantic_form pair" half of
    /// the wire vocabulary, alongside [`Self::wire_operation_identity`]'s
    /// "clause operation identity" half -- total and injective over the
    /// four variants, with no `_` arm.
    pub fn node_tag_and_semantic_form(self) -> (&'static str, Option<&'static str>) {
        match self {
            Self::Claim => ("claim", None),
            Self::Temporal => ("temporal", None),
            Self::Protocol => ("protocol", None),
            Self::StateTransition => ("state", Some("frame")),
        }
    }

    /// The variant whose [`Self::node_tag_and_semantic_form`] is exactly
    /// `(node_tag, semantic_form)`, or `None` when the pair names none of
    /// them: backward totality over this closed vocabulary, not an open
    /// pair of strings (FR-088-AC-4).
    pub fn from_node_tag_and_semantic_form(
        node_tag: &str,
        semantic_form: Option<&str>,
    ) -> Option<Self> {
        match (node_tag, semantic_form) {
            ("claim", None) => Some(Self::Claim),
            ("temporal", None) => Some(Self::Temporal),
            ("protocol", None) => Some(Self::Protocol),
            ("state", Some("frame")) => Some(Self::StateTransition),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------
// O-04/O-08: model correspondence and frame identity
// ---------------------------------------------------------------------

/// ADR-013 O-04: the checked-node-id -> domain-declaration correspondence
/// the S3 checker records as it processes a package. [`FrameSubjects::resolve`]
/// (FR-088-AC-2) reads it and nothing else: no consumer re-derives a
/// `NodeKey`'s `DeclarationKey` by searching source or a collection whose
/// order no declaration defines (R-05).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelCorrespondence {
    entries: BTreeMap<NodeKey, DeclarationKey>,
}

impl ModelCorrespondence {
    /// Record one checked node's own domain declaration. The S3 checker is
    /// the only writer (ADR-013 O-04). PR #300 review finding 4:
    /// `pub(super)`, not `pub` -- only `super::PackageDeclarations::check`
    /// (this module's parent, `check`) ever records a correspondence entry;
    /// no other module has a legitimate reason to construct one directly,
    /// so this is not part of `crate::check`'s public re-export surface
    /// (`check/mod.rs`'s `pub use identity::{...}` list carries the type,
    /// never this method).
    pub(super) fn record(&mut self, node: NodeKey, declaration: DeclarationKey) {
        self.entries.insert(node, declaration);
    }

    /// `node`'s `DeclarationKey`, read only from this recorded
    /// correspondence -- absent if `node` was never recorded, and never
    /// re-derived by another means (R-05).
    pub fn resolve(&self, node: NodeKey) -> Option<&DeclarationKey> {
        self.entries.get(&node)
    }
}

/// ADR-013 O-08: one frame's FR-340 subject sets, each a set of checked
/// `relation`/`model` node keys. This module builds only the identity and
/// the resolution step below (FR-088-CON-2); FR-340's own frame semantics
/// are #210's.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FrameSubjects {
    /// Field members and relations this frame's operation writes.
    pub modifies: BTreeSet<NodeKey>,
    /// Objects this frame's operation creates.
    pub creates: BTreeSet<NodeKey>,
    /// Objects this frame's operation deletes.
    pub deletes: BTreeSet<NodeKey>,
}

/// [`FrameSubjects`], resolved to `DeclarationKey`s through a
/// [`ModelCorrespondence`]. A subject key absent from the correspondence
/// resolves to `None` in place, rather than failing the whole resolution or
/// falling back to a search (FR-088-AC-2).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResolvedFrameSubjects {
    /// [`FrameSubjects::modifies`], each resolved.
    pub modifies: Vec<(NodeKey, Option<DeclarationKey>)>,
    /// [`FrameSubjects::creates`], each resolved.
    pub creates: Vec<(NodeKey, Option<DeclarationKey>)>,
    /// [`FrameSubjects::deletes`], each resolved.
    pub deletes: Vec<(NodeKey, Option<DeclarationKey>)>,
}

impl FrameSubjects {
    /// Resolve every subject node key in all three sets to its
    /// `DeclarationKey`, reading only `correspondence` (ADR-013 O-04).
    pub fn resolve(&self, correspondence: &ModelCorrespondence) -> ResolvedFrameSubjects {
        let resolve_set = |set: &BTreeSet<NodeKey>| {
            set.iter()
                .map(|node| (*node, correspondence.resolve(*node).cloned()))
                .collect()
        };
        ResolvedFrameSubjects {
            modifies: resolve_set(&self.modifies),
            creates: resolve_set(&self.creates),
            deletes: resolve_set(&self.deletes),
        }
    }
}

/// ADR-013 O-08: a frame's identity is the checked node id of the `state`
/// node with `semantic_form: "frame"`, carrying its own FR-340 subject
/// sets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    node: NodeKey,
    subjects: FrameSubjects,
}

impl Frame {
    /// Name a frame by its `state` node's own checked node id and its
    /// declared subject sets.
    pub fn new(node: NodeKey, subjects: FrameSubjects) -> Self {
        Self { node, subjects }
    }

    /// This frame's identity: the checked node id of its `state` node.
    pub fn node(&self) -> NodeKey {
        self.node
    }

    /// This frame's declared `modifies`/`creates`/`deletes` sets.
    pub fn subjects(&self) -> &FrameSubjects {
        &self.subjects
    }
}

// ---------------------------------------------------------------------
// O-11: qualified names
// ---------------------------------------------------------------------
//
// ADR-013 O-11 already has one canonical implementation in this crate:
// `quire_spec_language::value::expression::family::QualifiedName` (layer 5, ADR-011),
// which predates FR-088 (it is the layer-6 `replay`/`CheckedPackage::call`
// function-selection key, FR-062/FR-065). This module does not define a
// second `QualifiedName` type: FR-068-AC-3 forbids `check` (layer 3) from
// importing `value::expression` (layer 5) at all, so a `check`-owned type
// could never be the same type as that one, and a same-named but distinct
// type here would only invite the two to be confused. `Identifier`
// (`quire_exact::Identifier`, imported above) is this module's own
// name-*segment* use -- a single component, never a declared name sequence
// claiming the O-11 name itself; PR #300 review round 2 (HIGH-1) removed the
// one caller that used to build a `&[Identifier]` name sequence from it
// (`mint_type_declaration_identity`), so today `Identifier` names only a
// `SumVariant`'s own single declared case. `quire_exact` is the kernel K
// layer itself, so this is not a `value` import at all, and FR-068-AC-6's
// tier bound on `check`'s `value` imports does not apply to it.
//
// PR #300 review finding 2: the checker's own name -> node id resolution
// function already exists and is exercised in production --
// `CheckedGraph::function`/`function_identity`/`callable` (`check/mod.rs`),
// reached from outside `check` only through `CheckedPackage::call`'s
// (`qsl_package::CheckedPackage`, via `CheckedPackageEvaluation`)
// `QualifiedName` lookup (the
// one O-11/R-06 lookup this crate builds outside `replay`'s own E9
// exception). TC-251 and TC-257 (`tests/it/name_resolution_confinement.rs`,
// `tests/it/clause_kind_canonical.rs`) are the whole-crate scans that verify
// that confinement and this module's own single-canonical-enum claim.

// ---------------------------------------------------------------------
// O-14/C-26: checked type descriptors and the kernel `ValueType` conversion
// ---------------------------------------------------------------------

/// A `scalar_type` checked node's own declared shape (ADR-013 O-14).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScalarShape {
    /// `Boolean`.
    Boolean,
    /// The unbounded mathematical `Integer`.
    Integer,
    /// A `Rational[..]` domain.
    Rational(RationalDomain),
    /// A `Decimal[..]` domain.
    Decimal(DecimalType),
    /// `Float32`/`Float64`.
    Float(IeeeWidth),
    /// A quantity in exactly this unit.
    Quantity(UnitId),
    /// A `Text[..]` domain.
    Text(TextType),
}

/// One declared member of a sum-type checked node: its own name, which
/// [`crate::value::enumeration::mint_variant_id`] combines with the
/// declaring sum's node id to compute the member's `VariantId` (ADR-013 O-14
/// "Sum types", QC-15).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SumVariant {
    name: Identifier,
}

impl SumVariant {
    /// Declare a sum variant by its own name.
    pub(super) fn new(name: Identifier) -> Self {
        Self { name }
    }

    /// This variant's own declared name.
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
}

/// [`CheckedTypeNode::Sum`]'s own declared variants (PR #300 review finding
/// 6), plus whether the declaration selects `ordered enum` semantics
/// (ADR-013 O-14/OQ-D). Private fields: the only way to build one is
/// [`Self::new`], which refuses two variants sharing one declared name
/// rather than silently admitting both (a duplicate would make two different
/// `VariantId`s answer to the same case name, or -- worse -- collapse under
/// some future name-keyed lookup; O-06 member identity is declared,
/// `(declaring node id, name)`, so a name collision within one declaring sum
/// is exactly the case that identity rule cannot resolve), and, when
/// `ordered` is `false`, refuses a declared order that is not already sorted
/// by name -- the same invariant `value::enumeration::EnumDeclaration::admit`
/// enforces over the same FR-141 canonical-member-list rule (declaration
/// order for an ordered enum, case-identifier byte order otherwise).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SumVariants {
    ordered: bool,
    variants: Vec<SumVariant>,
}

/// [`SumVariants::new`]'s refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum InvalidSumVariants {
    /// Two variants declared the same name.
    #[error("duplicate sum variant name: {0}")]
    Duplicate(String),
    /// An unordered sum's declared variants are not sorted by name (ADR-013
    /// O-14/OQ-D: the FR-141 canonical member list of an unordered
    /// declaration is case-identifier byte order).
    #[error("unordered sum variants are not sorted by name")]
    Unsorted,
}

impl SumVariants {
    /// Admit `variants` as one sum's declared members, refusing a duplicate
    /// declared name and, for `ordered: false`, a declared order that is not
    /// already sorted by name (ADR-013 O-14/OQ-D).
    pub fn new(variants: Vec<SumVariant>, ordered: bool) -> Result<Self, InvalidSumVariants> {
        let mut seen = BTreeSet::new();
        for variant in &variants {
            if !seen.insert(variant.name()) {
                return Err(InvalidSumVariants::Duplicate(variant.name().to_owned()));
            }
        }
        if !ordered && !variants.is_sorted_by(|a, b| a.name() <= b.name()) {
            return Err(InvalidSumVariants::Unsorted);
        }
        Ok(Self { ordered, variants })
    }

    /// The declared variants, in FR-141 canonical order (declaration order
    /// when [`Self::is_ordered`], case-identifier byte order otherwise --
    /// [`Self::new`] already refused an unordered declaration whose variants
    /// arrived in any other order).
    pub fn iter(&self) -> impl Iterator<Item = &SumVariant> {
        self.variants.iter()
    }

    /// Whether this sum selects `ordered enum` semantics.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// The declared variant count.
    pub fn len(&self) -> usize {
        self.variants.len()
    }

    /// Whether this sum declares no variant (never a real sum in practice,
    /// but not this type's own concern to refuse).
    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }
}

/// ADR-013 O-14: a package type declaration, identified by its checked node
/// id -- one checked graph node per `scalar_type`, `composite_type`,
/// `bounded_domain` or sum form. Record, tuple and union identity is that
/// node id alone (FR-143-AC-6): [`Self::Composite`] carries no further
/// shape, since the kernel `ValueType::Composite(NodeKey)` needs none -- the
/// node's own id carries the declaration's own pre-existing key
/// (`composite.key()`, `value/declaration.rs`) unchanged into this module's
/// checked-node-id space -- the declaration holds the kernel `NodeKey` this
/// module uses, so no conversion is needed -- the
/// same identity `type_named` and field types already read, reused rather
/// than minted afresh (PR #300 review round 2, HIGH-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckedTypeNode {
    /// A `scalar_type` node.
    Scalar {
        /// This declaration's own checked node id.
        node: NodeKey,
        /// The declared scalar shape.
        shape: ScalarShape,
    },
    /// A `composite_type` node (a record or a tuple).
    Composite {
        /// This declaration's own checked node id.
        node: NodeKey,
    },
    /// A `bounded_domain` node: a declared bounded integer domain.
    BoundedDomain {
        /// This declaration's own checked node id.
        node: NodeKey,
        /// The declared `Int[lo, hi]` bound.
        bound: IntegerInterval,
    },
    /// The sum-type checked node form (ADR-013 O-14 "Sum types"): its own
    /// node id, plus its declared variants as O-06 members.
    Sum {
        /// This declaration's own checked node id.
        node: NodeKey,
        /// The declared variants, in source order (their `VariantId`s are
        /// position-independent regardless, ADR-013 O-14/QC-15).
        variants: SumVariants,
    },
}

impl CheckedTypeNode {
    /// This declaration's own checked node id, whichever form it is.
    pub fn node(&self) -> NodeKey {
        match self {
            Self::Scalar { node, .. }
            | Self::Composite { node }
            | Self::BoundedDomain { node, .. }
            | Self::Sum { node, .. } => *node,
        }
    }
}

/// ADR-013 C-26: the total conversion, checked type node -> kernel
/// `ValueType`, over every checked type-node form with no `_` arm
/// (FR-088-AC-9): a checked type-node form added without a corresponding
/// arm here fails to compile -- enforced at compile time, not only by
/// convention, by PR #300 review finding 11's `#[deny]` below. For the sum
/// form, the source node's own id is never re-minted (it simply is not read
/// into the kernel shape at all, so there is nothing here that could
/// re-mint it), and each variant's `VariantId` is computed by
/// [`crate::value::enumeration::mint_variant_id`] from the declaring sum and
/// that variant's own name (OQ-F ruling), never from its position in
/// `variants` (FR-088-AC-10). The kernel `EnumShape` additionally carries
/// each variant's canonical rank (ADR-013 O-14/OQ-D): `variants` is already
/// in FR-141 canonical order ([`SumVariants::new`] refused any other order
/// for an unordered sum), so the rank is exactly each variant's index here.
// PR #300 review finding 11: a checked-in `deny`, not only ADR-011 §5's
// documented convention -- adding a `CheckedTypeNode` form without a
// matching arm below now fails `cargo clippy` (and `-D warnings` promotes
// that to a build failure), not only a human reviewer's attention.
#[deny(clippy::wildcard_enum_match_arm)]
pub fn to_kernel_value_type(type_node: &CheckedTypeNode) -> ValueType {
    match type_node {
        CheckedTypeNode::Scalar { shape, .. } => match shape {
            ScalarShape::Boolean => ValueType::Boolean,
            ScalarShape::Integer => ValueType::Integer,
            ScalarShape::Rational(domain) => ValueType::Rational(domain.clone()),
            ScalarShape::Decimal(decimal) => ValueType::Decimal(decimal.clone()),
            ScalarShape::Float(width) => ValueType::Float(*width),
            ScalarShape::Quantity(unit) => ValueType::Quantity(*unit),
            ScalarShape::Text(text) => ValueType::Text(*text),
        },
        CheckedTypeNode::Composite { node } => ValueType::Composite(*node),
        CheckedTypeNode::BoundedDomain { bound, .. } => ValueType::Int(bound.clone()),
        CheckedTypeNode::Sum { node, variants } => ValueType::Enum(EnumShape::new(
            variants.is_ordered(),
            variants
                .iter()
                .map(|variant| mint_variant_id(*node, variant.name())),
        )),
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{Integer, Origin, Value};

    use super::*;
    use crate::check::family::OccurrenceMap;

    fn node_key(fill: u8) -> NodeKey {
        let mut bytes = [0_u8; 32];
        bytes[31] = fill;
        NodeKey::from_digest(bytes)
    }

    fn identifier(name: &str) -> Identifier {
        Identifier::new(name).unwrap()
    }

    // -- O-10: clause kind ------------------------------------------------

    /// TC-250 steps 2a-5: the forward mapping matches a fixed table
    /// (written independently of [`CheckedClauseKind::wire_operation_identity`]
    /// itself) and is total/injective in both directions.
    #[trace("TC-250", "FR-088-AC-4")]
    #[test]
    fn wire_mapping_matches_fixed_table_and_is_total_and_injective() {
        let expected = [
            (CheckedClauseKind::Claim, "quire.op.claim.clause"),
            (CheckedClauseKind::Temporal, "quire.op.temporal.clause"),
            (CheckedClauseKind::Protocol, "quire.op.protocol.control"),
            (
                CheckedClauseKind::StateTransition,
                "quire.op.state.transition",
            ),
        ];
        for (kind, wire) in expected {
            assert_eq!(kind.wire_operation_identity(), wire, "{kind:?}");
            assert_eq!(
                CheckedClauseKind::from_wire_operation_identity(wire),
                Some(kind)
            );
        }
        let forward: Vec<&str> = CheckedClauseKind::all()
            .iter()
            .map(|kind| kind.wire_operation_identity())
            .collect();
        let mut sorted = forward.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            forward.len(),
            "forward collision: {forward:?}"
        );
        assert_eq!(
            CheckedClauseKind::from_wire_operation_identity("not-a-real-wire-string"),
            None
        );
    }

    /// TC-250 (FR-088-AC-4): the `node_tag`/`semantic_form` half of the wire
    /// vocabulary is likewise total and injective, and matches the fixed
    /// table this type's own per-variant doc comments state (`claim`/
    /// `temporal`/`protocol` each their own node kind, `StateTransition` the
    /// `state`/`semantic_form: "frame"` case). PR #300 review round 2, L7:
    /// this closes the gap the prior version of this test suite left --
    /// only the clause-operation-identity half of AC-4's wire vocabulary had
    /// a totality/injectivity test.
    #[trace("TC-250", "FR-088-AC-4")]
    #[test]
    fn node_tag_and_semantic_form_mapping_matches_a_fixed_table_and_is_total_and_injective() {
        let expected = [
            (CheckedClauseKind::Claim, ("claim", None)),
            (CheckedClauseKind::Temporal, ("temporal", None)),
            (CheckedClauseKind::Protocol, ("protocol", None)),
            (CheckedClauseKind::StateTransition, ("state", Some("frame"))),
        ];
        for (kind, (node_tag, semantic_form)) in expected {
            assert_eq!(kind.node_tag_and_semantic_form(), (node_tag, semantic_form));
            assert_eq!(
                CheckedClauseKind::from_node_tag_and_semantic_form(node_tag, semantic_form),
                Some(kind)
            );
        }
        let forward: Vec<(&str, Option<&str>)> = CheckedClauseKind::all()
            .iter()
            .map(|kind| kind.node_tag_and_semantic_form())
            .collect();
        let mut sorted = forward.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            forward.len(),
            "forward collision: {forward:?}"
        );
        assert_eq!(
            CheckedClauseKind::from_node_tag_and_semantic_form("state", None),
            None,
            "a bare `state` node_tag with no semantic_form names no variant"
        );
        assert_eq!(
            CheckedClauseKind::from_node_tag_and_semantic_form("not-a-real-node-tag", None),
            None
        );
    }

    // -- O-08: frame identity ----------------------------------------------

    /// TC-248: a frame's identity is its `state` node's own checked node
    /// id; its subject sets resolve to `DeclarationKey`s only by reading
    /// the recorded model correspondence, and a subject removed from that
    /// correspondence resolves to `None` rather than being re-derived by
    /// any other means. See `super::super::tests` (`check/mod.rs`) for the
    /// companion test that builds this same correspondence through a real
    /// `PackageDeclarations::check` run rather than the hand-built one
    /// below (PR #300 review finding 1) -- this unit test is retained for
    /// the frame-subject *resolution mechanics* themselves, which have no
    /// real source syntax to check from yet (FR-340 frame semantics are
    /// #210's, FR-088-CON-2).
    #[trace("TC-248", "FR-088-AC-2")]
    #[test]
    fn frame_subjects_resolve_only_through_the_recorded_correspondence() {
        let modifies_node = node_key(1);
        let creates_node = node_key(2);
        let modifies_decl = DeclarationKey {
            package: "test/orders".to_owned(),
            node: "Order.status".to_owned(),
        };
        let creates_decl = DeclarationKey {
            package: "test/orders".to_owned(),
            node: "Order".to_owned(),
        };

        let mut correspondence = ModelCorrespondence::default();
        correspondence.record(modifies_node, modifies_decl.clone());
        correspondence.record(creates_node, creates_decl.clone());

        let frame_node = node_key(3);
        let mut subjects = FrameSubjects::default();
        subjects.modifies.insert(modifies_node);
        subjects.creates.insert(creates_node);
        let frame = Frame::new(frame_node, subjects);

        // Step 2: the frame's own identity is the state node's checked node id.
        assert_eq!(frame.node(), frame_node);

        // Step 3: every subject resolves to its DeclarationKey.
        let resolved = frame.subjects().resolve(&correspondence);
        assert_eq!(
            resolved.modifies,
            vec![(modifies_node, Some(modifies_decl.clone()))]
        );
        assert_eq!(
            resolved.creates,
            vec![(creates_node, Some(creates_decl.clone()))]
        );
        assert!(resolved.deletes.is_empty());

        // Step 4 (adverse): a correspondence rebuilt without the
        // `modifies` entry (simulating a stale/incomplete correspondence)
        // resolves that subject to `None`; `resolve` has no other source to
        // fall back to.
        let mut stale = ModelCorrespondence::default();
        stale.record(creates_node, creates_decl.clone());
        let resolved_stale = frame.subjects().resolve(&stale);
        assert_eq!(resolved_stale.modifies, vec![(modifies_node, None)]);

        // Step 5: a freshly rebuilt correspondence with every entry
        // reproduces exactly step 3's resolution.
        let mut rebuilt = ModelCorrespondence::default();
        rebuilt.record(modifies_node, modifies_decl);
        rebuilt.record(creates_node, creates_decl);
        assert_eq!(frame.subjects().resolve(&rebuilt), resolved);
    }

    // -- O-09 (clause half): clause identity and the occurrence key -------

    /// TC-249: two occurrences of a structurally identical clause (the same
    /// checked node id, since node ids are content-addressed) get distinct
    /// occurrence keys; a lookup keyed on (node id, occurrence key)
    /// distinguishes them where node id alone cannot, and neither
    /// occurrence's identity depends on the order the pair is iterated in.
    /// See `check::mod::tests::call_occurrences_of_the_same_callee_share_an_identity_and_disambiguate_by_occurrence_key`
    /// (PR #300 review finding 7) for the real-checker adverse case this
    /// unit test's own mechanics feed into: this crate has no `claim`/
    /// `temporal`/`protocol` clause syntax yet (FR-088-CON-1 scopes this
    /// requirement to the identity and occurrence-key mechanism only), so
    /// the closest *real*, checker-produced case of "one identity, two
    /// occurrences" is a repeated call to the same function -- both use the
    /// same real `OccurrenceMap` this test exercises directly.
    #[trace("TC-249", "FR-088-AC-3")]
    #[test]
    fn clause_occurrence_keys_disambiguate_structurally_identical_clauses() {
        let clause_node = node_key(9);
        let mut occurrences: OccurrenceMap<()> = OccurrenceMap::default();
        let first = occurrences.record(clause_node, "clause", ());
        let second = occurrences.record(clause_node, "clause", ());

        // Step 3: equal node ids (both occurrences of the one clause),
        // distinct occurrence keys.
        assert_eq!(first.role(), second.role());
        assert_ne!(first.ordinal(), second.ordinal());

        // Step 4: (node id, occurrence key) disambiguates what node id
        // alone cannot.
        let mut by_pair: BTreeMap<(NodeKey, Origin), &str> = BTreeMap::new();
        by_pair.insert((clause_node, first.clone()), "occurrence A");
        by_pair.insert((clause_node, second.clone()), "occurrence B");
        assert_eq!(
            by_pair.len(),
            2,
            "node id alone would have collapsed these to one entry"
        );

        // Step 5 (adverse, R-05): inserting the same two pairs in the
        // opposite order produces the identical map -- (node id, occurrence
        // key) is what determines identity/membership, not the order the
        // pairs happened to be built or read in.
        let mut inserted_in_reverse = BTreeMap::new();
        inserted_in_reverse.insert((clause_node, second), "occurrence B");
        inserted_in_reverse.insert((clause_node, first), "occurrence A");
        assert_eq!(by_pair, inserted_in_reverse);
    }

    // -- O-14/C-26: type descriptors and the kernel conversion -------------
    //
    // PR #300 review round 2 (HIGH-1): the former TC-258/TC-259 tests here
    // exercised `mint_type_declaration_identity`, a second, parallel type
    // identity nothing else in the checker read -- deleted along with the
    // minter itself (see this module's own doc). AC-7's package-scoping
    // claim and AC-6's "not an identity in its own right" claim are now
    // properties of `composite.key()`/`EnumDeclaration::key()`, the
    // pre-existing identities `check` reuses unchanged (see
    // `crate::check::mod::tests` for the real-`check()` coverage of AC-7),
    // not of anything this module mints; TC-258's own coverage lives at
    // `value::expression::family`'s `QualifiedName` (that type's own
    // `#[trace("TC-258", ...)]` test), the O-11 type this ADR-013 note
    // itself names as canonical.

    /// PR #300 review finding 5: a golden digest vector for
    /// `mint_variant_id`, mirroring `check::family::
    /// mint_declaration_identity_matches_a_checked_in_digest`'s own
    /// convention -- this catches a reordered field or a renamed tag an
    /// equality-only test (comparing two identities minted in the same
    /// process) cannot, since both sides would move together and stay
    /// green. Regenerate the constant only when the preimage grammar change
    /// is the one actually intended, and say so in the commit (this
    /// repository's own digest-freshness rule, `CLAUDE.md`), never to make a
    /// red test green.
    ///
    /// **This digest moved under OQ-F (ADR-013 §8, ruled 2026-09-22).**
    /// `mint_variant_id` no longer computes the private, QSpec-unowned
    /// `"sum-variant-member"` preimage this test used to pin
    /// (`sha256("sum-variant-member" || sum-node-bytes || "Active")`); it
    /// now computes QSpec FR-141's own enum-member node key,
    /// `quire.checked-semantic-node/v1` over `{version: "quire.enum-
    /// member-node/v1", declaration_node_id: {domain: "quire.checked-
    /// semantic-node/v1", digest: sum}, case: "Active"}` -- the same
    /// preimage `value::enumeration::EnumMemberPreimage::digest` recomputes
    /// at admission, verified against QSpec's own
    /// `proposals/checked-package-v2/node-identity-vectors.json` fixture.
    /// Every occurrence of the old digest was this one test (grep found no
    /// other golden or vector pinned to it); no other test or fixture moved.
    ///
    /// PR #300 review round 2, L9: retagged `TC-252`/`FR-088-AC-10` (this
    /// function's own owning pair per `spec/tests.md`) -- the prior
    /// `TC-259`/`FR-088-AC-10` pairing named a combination absent from the
    /// Test Matrix (TC-259 verifies AC-7; AC-10 is TC-252).
    ///
    /// Also TC-409 step 2 (FR-088-AC-11): the pinned digest is the same
    /// independently computed FR-141 member node key TC-409's own procedure
    /// asks for.
    #[trace("TC-252", "FR-088-AC-10")]
    #[trace("TC-409", "FR-088-AC-11")]
    #[test]
    fn mint_variant_id_matches_a_checked_in_digest() {
        let sum = node_key(7);
        let variant_id = mint_variant_id(sum, "Active");
        assert_eq!(
            variant_id.to_string(),
            "e72a0028859b92eaefee0e78eb414a49a58ede1b610cc4233dd2724b3dfc5bdc",
            "the preimage byte grammar changed -- see this test's own doc \
             before regenerating this constant"
        );
    }

    /// TC-252 (FR-088-AC-9): every non-sum checked type-node form converts
    /// to the matching kernel `ValueType`, with no panic.
    #[trace("TC-252", "FR-088-AC-9")]
    #[test]
    fn c26_converts_every_non_sum_form() {
        let scalar = CheckedTypeNode::Scalar {
            node: node_key(1),
            shape: ScalarShape::Boolean,
        };
        assert_eq!(to_kernel_value_type(&scalar), ValueType::Boolean);

        let composite = CheckedTypeNode::Composite { node: node_key(2) };
        assert_eq!(
            to_kernel_value_type(&composite),
            ValueType::Composite(node_key(2))
        );

        let bound = IntegerInterval::new(Integer::zero(), Integer::from(10_i64)).unwrap();
        let bounded = CheckedTypeNode::BoundedDomain {
            node: node_key(3),
            bound: bound.clone(),
        };
        assert_eq!(to_kernel_value_type(&bounded), ValueType::Int(bound));
    }

    /// TC-252 (FR-088-AC-10): the sum form's own node id is unaffected by
    /// conversion, each variant's `VariantId` is computed from the declaring
    /// sum and that variant's name -- not its position -- and the converted
    /// kernel shape carries no `NodeKey`. A sum value carries its
    /// `(VariantId, rank)` pair (ADR-013 O-14/OQ-D), never a bare index.
    ///
    /// Also TC-409 steps 2 and 5 (FR-088-AC-11): each variant's `VariantId`
    /// is the FR-141 member node key, its rank is its canonical-list
    /// position, and admission refuses a well-formed `VariantId` paired
    /// with the wrong rank.
    #[trace("TC-252", "FR-088-AC-10")]
    #[trace("TC-409", "FR-088-AC-11")]
    #[test]
    fn c26_sum_preserves_node_id_and_mints_name_derived_variant_ids() {
        let sum_node = node_key(4);
        let active = SumVariant::new(identifier("Active"));
        let closed = SumVariant::new(identifier("Closed"));
        let sum = CheckedTypeNode::Sum {
            node: sum_node,
            variants: SumVariants::new(vec![active.clone(), closed.clone()], false).unwrap(),
        };

        let converted = to_kernel_value_type(&sum);

        // The source node's own id is unchanged by conversion.
        assert_eq!(sum.node(), sum_node);

        let ValueType::Enum(shape) = &converted else {
            panic!("sum form must convert to ValueType::Enum: {converted:?}");
        };
        let active_id = mint_variant_id(sum_node, active.name());
        let closed_id = mint_variant_id(sum_node, closed.name());
        assert!(shape.contains(active_id));
        assert!(shape.contains(closed_id));

        // A sum value carries its (VariantId, rank) pair, at the rank the
        // (already case-sorted, unordered) shape assigns it -- never a bare
        // index of its own choosing.
        assert_eq!(shape.rank(active_id), Some(0));
        assert_eq!(shape.rank(closed_id), Some(1));
        assert!(converted.admits(&Value::Enum(quire_exact::EnumMember::new(active_id, 0))));
        assert!(!converted.admits(&Value::Enum(quire_exact::EnumMember::new(active_id, 1))));
    }

    /// ADR-013 O-14/OQ-D (mutation proof): identity (`VariantId`) never
    /// depends on declared order, but *rank* does, for an ordered sum --
    /// exactly the property `quire_exact::compare_keys`'s FR-144
    /// canonical-key ordering now relies on. An implementation that dropped
    /// `SumVariants`' `ordered` flag and always case-sorted (the bug this
    /// test catches) would give both declared orders the same rank for
    /// "Active", making this assertion fail.
    ///
    /// Also TC-409 step 3 (FR-088-AC-11): an ordered enum's ranks follow
    /// declaration order.
    #[trace("TC-252", "FR-088-AC-10")]
    #[trace("TC-409", "FR-088-AC-11")]
    #[test]
    fn c26_ordered_sum_rank_follows_declared_order_not_identity() {
        let sum_node = node_key(4);
        let active = SumVariant::new(identifier("Active"));
        let closed = SumVariant::new(identifier("Closed"));
        let declared = CheckedTypeNode::Sum {
            node: sum_node,
            variants: SumVariants::new(vec![active.clone(), closed.clone()], true).unwrap(),
        };
        let reversed = CheckedTypeNode::Sum {
            node: sum_node,
            variants: SumVariants::new(vec![closed, active], true).unwrap(),
        };

        let ValueType::Enum(declared_shape) = to_kernel_value_type(&declared) else {
            panic!("sum form must convert to ValueType::Enum");
        };
        let ValueType::Enum(reversed_shape) = to_kernel_value_type(&reversed) else {
            panic!("sum form must convert to ValueType::Enum");
        };

        let active_id = mint_variant_id(sum_node, "Active");
        let closed_id = mint_variant_id(sum_node, "Closed");

        // Identity never depends on declared order.
        assert!(declared_shape.contains(active_id) && reversed_shape.contains(active_id));
        assert!(declared_shape.contains(closed_id) && reversed_shape.contains(closed_id));

        // Rank does, for an ordered sum.
        assert_eq!(declared_shape.rank(active_id), Some(0));
        assert_eq!(declared_shape.rank(closed_id), Some(1));
        assert_eq!(reversed_shape.rank(closed_id), Some(0));
        assert_eq!(reversed_shape.rank(active_id), Some(1));
        assert_ne!(declared_shape, reversed_shape);
    }

    /// ADR-013 O-14/OQ-D: an unordered sum's declared variants must already
    /// be sorted by name -- the same invariant
    /// `value::enumeration::EnumDeclaration::admit` enforces -- so a caller
    /// cannot silently mis-rank an unordered enum by declaring it out of
    /// order.
    ///
    /// Also TC-409 step 4 (FR-088-AC-11): an unordered enum's canonical
    /// order is case-identifier byte order, enforced here at construction.
    #[trace("TC-252", "FR-088-AC-10")]
    #[trace("TC-409", "FR-088-AC-11")]
    #[test]
    fn sum_variants_refuses_an_unsorted_unordered_declaration() {
        let closed = SumVariant::new(identifier("Closed"));
        let active = SumVariant::new(identifier("Active"));
        let refusal = SumVariants::new(vec![closed, active], false).unwrap_err();
        assert_eq!(refusal, InvalidSumVariants::Unsorted);
    }

    /// TC-409 step 6 (FR-088-AC-11): two declarations that differ only in
    /// identity (here, two distinct declaration `NodeKey`s standing in for
    /// a rename -- a declaration's `NodeKey` is a digest over `{owner,
    /// qualified_declaration, ordered, members}`, so any of those changing
    /// produces exactly this) mint different `VariantId`s for the same case
    /// name, at the same rank, under the same declared order. This tests
    /// what FR-141's member preimage (`{version, declaration_node_id,
    /// case}`) unambiguously guarantees -- unlike TC-409's own procedure
    /// text, which frames step 6 as a package *version* bump; SR-511
    /// FND-007 found that framing unsupported by QSpec (a version bump only
    /// changes the key through `owner`, not directly), so this test uses a
    /// declaration-identity change instead, which is what FR-141 actually
    /// keys on.
    #[trace("TC-409", "FR-088-AC-11")]
    #[test]
    fn c26_declaration_identity_change_mints_new_variant_ids_at_the_same_ranks() {
        let original_node = node_key(4);
        let renamed_node = node_key(5);
        assert_ne!(original_node, renamed_node);

        let variants = || {
            SumVariants::new(
                vec![
                    SumVariant::new(identifier("Active")),
                    SumVariant::new(identifier("Closed")),
                ],
                true,
            )
            .unwrap()
        };
        let original = CheckedTypeNode::Sum {
            node: original_node,
            variants: variants(),
        };
        let renamed = CheckedTypeNode::Sum {
            node: renamed_node,
            variants: variants(),
        };
        let ValueType::Enum(original_shape) = to_kernel_value_type(&original) else {
            panic!("sum form must convert to ValueType::Enum");
        };
        let ValueType::Enum(renamed_shape) = to_kernel_value_type(&renamed) else {
            panic!("sum form must convert to ValueType::Enum");
        };

        let original_active = mint_variant_id(original_node, "Active");
        let renamed_active = mint_variant_id(renamed_node, "Active");
        let original_closed = mint_variant_id(original_node, "Closed");
        let renamed_closed = mint_variant_id(renamed_node, "Closed");

        // Every VariantId changes with the declaration's own identity.
        assert_ne!(original_active, renamed_active);
        assert_ne!(original_closed, renamed_closed);

        // Ranks and canonical order do not: both shapes still rank "Active"
        // 0 and "Closed" 1, the same declared order in both.
        assert_eq!(original_shape.rank(original_active), Some(0));
        assert_eq!(original_shape.rank(original_closed), Some(1));
        assert_eq!(renamed_shape.rank(renamed_active), Some(0));
        assert_eq!(renamed_shape.rank(renamed_closed), Some(1));
    }

    /// PR #300 review finding 6: two variants sharing one declared name
    /// refuse at construction rather than silently collapsing.
    #[trace("TC-252", "FR-088-AC-10")]
    #[test]
    fn sum_variants_refuses_a_duplicate_declared_name() {
        let first = SumVariant::new(identifier("Active"));
        let duplicate = SumVariant::new(identifier("Active"));
        let refusal = SumVariants::new(vec![first, duplicate], false).unwrap_err();
        assert_eq!(refusal, InvalidSumVariants::Duplicate("Active".to_owned()));
    }
}

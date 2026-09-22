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
//! `crate::forms::ClauseKind` already names a different, pre-existing
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
//! the model correspondence ([`CheckedGraph::resolve_declaration`]), never a
//! name-keyed one, matching R-06's "no name -> identity lookup exists after
//! the check stage" (FR-088-AC-5).
//!
//! # PR #300 review: wiring (findings 1-11)
//!
//! This module's minters and [`to_kernel_value_type`] are now called from
//! real `check` production code (`super::PackageDeclarations::check`), not
//! only from this module's own unit tests (finding 1): every admitted
//! composite and enum declaration in a checked package's `TypeEnvironment`
//! becomes a real [`CheckedTypeNode`], minted through
//! [`mint_type_declaration_identity`] from its own declared shape (finding
//! 3), and `CheckedGraph` exposes them by node id
//! (`CheckedGraph::checked_type_node`). The model correspondence
//! ([`ModelCorrespondence`]) is populated from `PackageDeclarations`' own
//! new `model_correspondence` field (`check/check.rs`), matching this
//! file's own pre-existing `dispatch_operations`/`dispatch_tables`
//! precedent of "built by the caller (the `model` bridge); the checker only
//! records/resolves against it" -- see that field's doc for why no such
//! domain-declaration bridge exists in production yet, and
//! `mod.rs`'s test module for a test that goes through
//! `CheckedPackage::graph().resolve_declaration`, not a hand-built
//! `ModelCorrespondence` (finding 1's own "not hand-built correspondences").
//! The remaining findings (4-11) are visibility, encoding and totality
//! fixes on these same types, noted at each site below.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use quire_exact::{
    DecimalType, EnumShape, IeeeWidth, IntegerInterval, NodeKey, RationalDomain, TextType, UnitId,
    ValueType, VariantId,
};

use super::family::{encode_value_type, Preimage};
use crate::model::key::DeclarationKey;
use crate::value::composite::{CompositeShape, Presence};
use crate::value::Identifier;

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

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
    /// Every variant, for a totality/injectivity scan over the whole
    /// vocabulary. PR #300 review finding 8: derived through an exhaustive
    /// match over `Self` itself (no `_` arm), not only a hand-maintained
    /// array literal -- adding a fifth variant fails this match to compile
    /// until the array below is extended to match, so `all()` cannot
    /// silently under-cover TC-250's totality scan the way a plain literal
    /// with no compiler backstop could.
    pub fn all() -> [Self; 4] {
        #[allow(
            clippy::match_same_arms,
            reason = "the exhaustiveness check is the point, not the arm bodies"
        )]
        match Self::Claim {
            Self::Claim | Self::Temporal | Self::Protocol | Self::StateTransition => {}
        }
        [
            Self::Claim,
            Self::Temporal,
            Self::Protocol,
            Self::StateTransition,
        ]
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
// `crate::value::expression::family::QualifiedName` (layer 5, ADR-011),
// which predates FR-088 (it is the layer-6 `replay`/`CheckedPackage::call`
// function-selection key, FR-062/FR-065). This module does not define a
// second `QualifiedName` type: FR-068-AC-3 forbids `check` (layer 3) from
// importing `value::expression` (layer 5) at all, so a `check`-owned type
// could never be the same type as that one, and a same-named but distinct
// type here would only invite the two to be confused. `check` code that
// needs a declared name sequence (`mint_type_declaration_identity` below)
// takes a plain `&[Identifier]` instead -- the sequence itself, not a
// wrapper claiming the O-11 name.
//
// PR #300 review finding 2: the checker's own name -> node id resolution
// function already exists and is exercised in production --
// `CheckedGraph::function`/`function_identity`/`callable` (`check/mod.rs`),
// reached from outside `check` only through
// `value::expression::CheckedPackage::call`'s `QualifiedName` lookup (the
// one O-11/R-06 lookup this crate builds outside `replay`'s own E9
// exception). TC-251 and TC-257 (`tests/name_resolution_confinement.rs`,
// `tests/clause_kind_canonical.rs`) are the whole-crate scans that verify
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
/// [`mint_variant_id`] combines with the declaring sum's node id to compute
/// the member's `VariantId` (ADR-013 O-14 "Sum types", QC-15).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SumVariant {
    name: Identifier,
}

impl SumVariant {
    /// Declare a sum variant by its own name.
    pub fn new(name: Identifier) -> Self {
        Self { name }
    }

    /// This variant's own declared name.
    pub fn name(&self) -> &Identifier {
        &self.name
    }
}

/// [`CheckedTypeNode::Sum`]'s own declared variants (PR #300 review finding
/// 6). Private field: the only way to build one is [`Self::new`], which
/// refuses two variants sharing one declared name rather than silently
/// admitting both (a duplicate would make two different `VariantId`s answer
/// to the same case name, or -- worse -- collapse under some future
/// name-keyed lookup; O-06 member identity is declared, `(declaring node
/// id, name)`, so a name collision within one declaring sum is exactly the
/// case that identity rule cannot resolve).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SumVariants(Vec<SumVariant>);

/// [`SumVariants::new`]'s refusal: two variants declared the same name.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("duplicate sum variant name: {0}")]
pub struct DuplicateSumVariant(String);

impl SumVariants {
    /// Admit `variants` as one sum's declared members, in declaration
    /// order, refusing a duplicate declared name.
    pub fn new(variants: Vec<SumVariant>) -> Result<Self, DuplicateSumVariant> {
        let mut seen = BTreeSet::new();
        for variant in &variants {
            if !seen.insert(variant.name().as_str()) {
                return Err(DuplicateSumVariant(variant.name().as_str().to_owned()));
            }
        }
        Ok(Self(variants))
    }

    /// The declared variants, in declaration order.
    pub fn iter(&self) -> impl Iterator<Item = &SumVariant> {
        self.0.iter()
    }

    /// The declared variant count.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether this sum declares no variant (never a real sum in practice,
    /// but not this type's own concern to refuse).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// ADR-013 O-14: a package type declaration, identified by its checked node
/// id -- one checked graph node per `scalar_type`, `composite_type`,
/// `bounded_domain` or sum form. Record, tuple and union identity is that
/// node id alone (FR-143-AC-6): [`Self::Composite`] carries no further
/// shape, since the kernel `ValueType::Composite(NodeKey)` needs none (the
/// node's own id, minted from the full declared shape by
/// [`mint_type_declaration_identity`], already carries that content -- see
/// [`DeclaredShape::Composite`]).
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
/// [`mint_variant_id`] from the declaring sum and that variant's own
/// member, never from its position in `variants` (FR-088-AC-10).
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
            variants
                .iter()
                .map(|variant| mint_variant_id(*node, variant.name())),
        )),
    }
}

/// [`mint_type_declaration_identity`]'s own declared-shape parameter (PR
/// #300 review findings 3 and 9): closed to exactly the four checked
/// type-node forms O-14 names, replacing a bare `shape_tag: &str`. A caller
/// could pass an inconsistent tag string against a bare `&str` (for example
/// `"composite_type"` for a value that is really a bounded domain); this
/// enum forecloses that by construction, and every arm both fixes its own
/// tag internally and writes its own real declared content into the
/// preimage, so two declarations that differ only in shape -- not in name
/// or package -- now mint different node ids too (closing the O-04 "equal
/// ids mean structurally identical nodes" gap a name-only preimage left
/// open).
pub(super) enum DeclaredShape<'a> {
    /// A `scalar_type` node's own declared domain. Not yet constructed by
    /// `check()` itself: no production caller admits a standalone
    /// `scalar_type`/`bounded_domain` declaration today (complete-V1's own
    /// `TypeEnvironment` carries only composite and enum declarations;
    /// `type Name = T;` aliases "create no declaration identity" per
    /// `check.rs`'s own doc) -- that intake is `model`'s bridge work, not
    /// yet built, and out of FR-088/S-3b's own scope. The variant stays
    /// real, not deleted: O-14/FR-088-AC-9 name `scalar_type` as one of the
    /// four checked type-node forms C-26 must convert, and this crate's own
    /// unit tests construct it (`#[cfg_attr(not(test), allow(dead_code))]`
    /// below says exactly that, rather than a bare, unexplained `allow`).
    #[cfg_attr(not(test), allow(dead_code))]
    Scalar(&'a ScalarShape),
    /// A `composite_type` node's own declared fields or tuple positions
    /// (native-v1's own `value::composite::CompositeShape`), encoded
    /// through [`encode_value_type`] exactly as a function declaration's
    /// own parameter and result types already are (`family.rs`'s
    /// `mint_declaration_identity`) -- the same encoding, not a second one.
    Composite(&'a CompositeShape),
    /// A `bounded_domain` node's own declared `Int[lo, hi]` bound. See
    /// [`Self::Scalar`]'s own doc: not yet constructed by `check()` either,
    /// for the same reason.
    #[cfg_attr(not(test), allow(dead_code))]
    BoundedDomain(&'a IntegerInterval),
    /// The sum form's own declared variants, in declaration order. Unlike a
    /// member's own `VariantId` (position-independent, ADR-013 O-14/QC-15),
    /// this declaration-level node id is allowed to depend on declaration
    /// order: reordering a sum's declared variants is a real textual edit
    /// to the declaration, not a value the kernel treats as interchangeable
    /// the way it treats a resolved member.
    Sum(&'a SumVariants),
}

fn encode_scalar_shape(out: &mut Preimage, shape: &ScalarShape) {
    match shape {
        ScalarShape::Boolean => out.write_str("boolean"),
        ScalarShape::Integer => out.write_str("integer"),
        ScalarShape::Rational(domain) => {
            out.write_str("rational");
            out.write_str(&domain.numerator().lower().to_string());
            out.write_str(&domain.numerator().upper().to_string());
            out.write_str(&domain.denominator().lower().to_string());
            out.write_str(&domain.denominator().upper().to_string());
        }
        ScalarShape::Decimal(decimal) => {
            out.write_str("decimal");
            out.write_str(&decimal.lower().to_string());
            out.write_str(&decimal.upper().to_string());
            out.write_str(&decimal.min_scale().to_string());
            out.write_str(&decimal.max_scale().to_string());
        }
        ScalarShape::Float(width) => {
            out.write_str("float");
            out.write_str(match width {
                IeeeWidth::Binary32 => "binary32",
                IeeeWidth::Binary64 => "binary64",
            });
        }
        ScalarShape::Quantity(unit) => {
            out.write_str("quantity");
            out.write_bytes(unit.as_bytes());
        }
        ScalarShape::Text(text) => {
            out.write_str("text");
            out.write_str(&text.min().to_string());
            out.write_str(&text.max().to_string());
        }
    }
}

fn encode_composite_shape(out: &mut Preimage, shape: &CompositeShape) {
    match shape {
        CompositeShape::Record(fields) => {
            out.write_str("record");
            for field in fields {
                out.write_str(field.name());
                encode_value_type(out, field.value_type());
                out.write_str(match field.presence() {
                    Presence::Required => "required",
                    Presence::Optional => "optional",
                });
            }
        }
        CompositeShape::Tuple(elements) => {
            out.write_str("tuple");
            for element in elements {
                encode_value_type(out, element);
            }
        }
    }
}

/// ADR-013 O-04/QC-18's package-scoped preimage, applied to a package type
/// declaration (O-14): the declaring package's `name@version`, this
/// declaration's own qualified name segments (a declared preimage
/// component, O-11 -- never an identity by itself, FR-088-AC-6) and its
/// declared shape (PR #300 review finding 3). Two declarations differing in
/// package, name, or declared shape mint different node ids; the same
/// three facts always mint the same node id (deterministic,
/// content-addressed, FR-088-AC-7/TC-259).
pub(super) fn mint_type_declaration_identity(
    package_identity: &str,
    name: &[Identifier],
    shape: DeclaredShape<'_>,
) -> NodeKey {
    // `u64::MAX`: this minter has no `StageLimits` budget to report
    // against, unlike `ValueFunctionFamily::check` (see `Preimage::new`'s
    // own doc).
    let mut preimage = Preimage::new(u64::MAX);
    preimage.write_str("checked-type-declaration");
    preimage.write_str(package_identity);
    for segment in name {
        preimage.write_str(segment.as_str());
    }
    match shape {
        DeclaredShape::Scalar(scalar) => {
            preimage.write_str("scalar_type");
            encode_scalar_shape(&mut preimage, scalar);
        }
        DeclaredShape::Composite(shape) => {
            preimage.write_str("composite_type");
            encode_composite_shape(&mut preimage, shape);
        }
        DeclaredShape::BoundedDomain(bound) => {
            preimage.write_str("bounded_domain");
            preimage.write_str(&bound.lower().to_string());
            preimage.write_str(&bound.upper().to_string());
        }
        DeclaredShape::Sum(variants) => {
            preimage.write_str("sum");
            for variant in variants.iter() {
                preimage.write_str(variant.name().as_str());
            }
        }
    }
    NodeKey::from_digest(sha256(&preimage.finish()))
}

/// ADR-013 O-14/QC-15: a sum variant's `VariantId`, computed from the
/// declaring sum's own node id and the variant's own name -- never from its
/// position in a declared list (FR-088-AC-10).
pub(super) fn mint_variant_id(sum: NodeKey, member_name: &Identifier) -> VariantId {
    let mut preimage = Preimage::new(u64::MAX);
    preimage.write_str("sum-variant-member");
    preimage.write_bytes(sum.as_bytes());
    preimage.write_str(member_name.as_str());
    VariantId::from_digest(sha256(&preimage.finish()))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{Integer, Origin, Value};

    use super::*;
    use crate::check::family::OccurrenceMap;
    use crate::value::composite::FieldDeclaration;
    // This module's own `ValueType` (from `use super::*`) is the kernel
    // `quire_exact::ValueType` C-26 converts *into*; `CompositeShape`'s
    // field/element types are the native, pre-existing
    // `value::composite::ValueType` C-26 converts *from* nothing of the
    // sort (composite fields are never converted by C-26 at all -- see
    // `CheckedTypeNode::Composite`'s own doc) -- these two same-named,
    // unrelated types collide in one `use super::*` scope, so this alias
    // disambiguates the native one wherever a `CompositeShape` is built.
    use crate::value::composite::ValueType as NativeValueType;

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

    /// TC-250 step 6: named mutants of the forward mapping, checked against
    /// this test's own verification (the fixed-table equality plus
    /// totality/injectivity above), confirm each is caught. Mutant (a)
    /// ("delete one match arm") is not expressible as a runtime value at
    /// all: `CheckedClauseKind::wire_operation_identity`'s match has no `_`
    /// arm, so deleting an arm is a compile error, the strongest possible
    /// result. Mutants (b) and (c) are simulated directly below.
    #[trace("TC-250", "FR-088-AC-4")]
    #[test]
    fn named_mutants_of_the_forward_mapping_are_caught() {
        fn verify(forward: impl Fn(CheckedClauseKind) -> &'static str) -> Result<(), String> {
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
                if forward(kind) != wire {
                    return Err(format!("{kind:?} mismatched the fixed table"));
                }
            }
            let outputs: Vec<&str> = CheckedClauseKind::all()
                .iter()
                .map(|kind| forward(*kind))
                .collect();
            let mut sorted = outputs.clone();
            sorted.sort_unstable();
            sorted.dedup();
            if sorted.len() != outputs.len() {
                return Err(format!("forward collision: {outputs:?}"));
            }
            Ok(())
        }

        assert!(verify(CheckedClauseKind::wire_operation_identity).is_ok());

        // (b) swap two variants' target strings: still total and injective
        // (a permutation of the same codomain), caught only by the
        // fixed-table check.
        let swapped = |kind: CheckedClauseKind| match kind {
            CheckedClauseKind::Claim => "quire.op.temporal.clause",
            CheckedClauseKind::Temporal => "quire.op.claim.clause",
            CheckedClauseKind::Protocol => "quire.op.protocol.control",
            CheckedClauseKind::StateTransition => "quire.op.state.transition",
        };
        assert!(verify(swapped).is_err(), "a swap mutant must be caught");

        // (c) collide one variant onto another's wire string: caught by
        // injectivity.
        let collided = |kind: CheckedClauseKind| match kind {
            CheckedClauseKind::Claim => "quire.op.claim.clause",
            CheckedClauseKind::Temporal => "quire.op.claim.clause",
            CheckedClauseKind::Protocol => "quire.op.protocol.control",
            CheckedClauseKind::StateTransition => "quire.op.state.transition",
        };
        assert!(
            verify(collided).is_err(),
            "a forward collision mutant must be caught"
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

        // Step 5: no collection-iteration order is load-bearing (R-05):
        // the pair set is the same set regardless of the order it is built
        // or read in.
        let mut forward: Vec<_> = by_pair.keys().cloned().collect();
        let mut reversed = forward.clone();
        reversed.reverse();
        forward.sort();
        reversed.sort();
        assert_eq!(forward, reversed);
    }

    // -- O-11: qualified names ----------------------------------------------

    /// TC-258 step 4 (adverse): two declarations with an equal qualified
    /// name but minted under different packages are distinguished by node
    /// id, never by the qualified name they happen to share.
    #[trace("TC-258", "FR-088-AC-6")]
    #[test]
    fn equal_qualified_names_do_not_make_two_declarations_the_same_identity() {
        let name = [identifier("Order"), identifier("status")];
        let shape = CompositeShape::Tuple(vec![NativeValueType::Boolean]);
        let first = mint_type_declaration_identity(
            "test/orders@1.0.0",
            &name,
            DeclaredShape::Composite(&shape),
        );
        let second = mint_type_declaration_identity(
            "test/billing@1.0.0",
            &name,
            DeclaredShape::Composite(&shape),
        );
        assert_ne!(
            first, second,
            "equal qualified names must not collapse distinct declarations"
        );
    }

    // -- O-14/C-26: type descriptors and the kernel conversion -------------

    /// TC-259 steps 3-5: a package type's identity is scoped to its
    /// declaring package (two packages declaring a structurally identical
    /// type mint distinct node ids), and re-minting the same declared facts
    /// (standing in for a second reference site, or a fresh recompile)
    /// reproduces exactly the same node id.
    #[trace("TC-259", "FR-088-AC-7")]
    #[test]
    fn package_type_identity_is_scoped_to_its_declaring_package() {
        let name = [identifier("Order")];
        let shape = CompositeShape::Tuple(vec![NativeValueType::Boolean]);
        let a = mint_type_declaration_identity(
            "test/orders@1.0.0",
            &name,
            DeclaredShape::Composite(&shape),
        );
        let b = mint_type_declaration_identity(
            "test/billing@1.0.0",
            &name,
            DeclaredShape::Composite(&shape),
        );
        assert_ne!(a, b);

        let a_again = mint_type_declaration_identity(
            "test/orders@1.0.0",
            &name,
            DeclaredShape::Composite(&shape),
        );
        assert_eq!(a, a_again);
    }

    /// PR #300 review finding 3: two declarations that share every fact
    /// except their declared shape must not collapse onto one node id --
    /// the gap a bare `shape_tag: &str` (able to lie about what shape it
    /// named) left open. This is the adverse case that closes O-04's
    /// "equal ids mean structurally identical nodes" for this preimage.
    #[trace("TC-259", "FR-088-AC-7")]
    #[test]
    fn differently_shaped_declarations_under_the_same_name_mint_different_ids() {
        let name = [identifier("Order")];
        let tuple_of_one = CompositeShape::Tuple(vec![NativeValueType::Boolean]);
        let tuple_of_two =
            CompositeShape::Tuple(vec![NativeValueType::Boolean, NativeValueType::Integer]);
        let record = CompositeShape::Record(vec![FieldDeclaration::new(
            "flag",
            NativeValueType::Boolean,
            Presence::Required,
        )]);

        let a = mint_type_declaration_identity(
            "test/orders@1.0.0",
            &name,
            DeclaredShape::Composite(&tuple_of_one),
        );
        let b = mint_type_declaration_identity(
            "test/orders@1.0.0",
            &name,
            DeclaredShape::Composite(&tuple_of_two),
        );
        let c = mint_type_declaration_identity(
            "test/orders@1.0.0",
            &name,
            DeclaredShape::Composite(&record),
        );
        assert_ne!(a, b, "a differing tuple arity must mint a different id");
        assert_ne!(a, c, "a tuple and a record must mint different ids");
        assert_ne!(b, c);

        // A `bounded_domain` and a `scalar_type` with unrelated content,
        // under the very same name, likewise never collide with either of
        // the above or each other.
        let bound = IntegerInterval::new(Integer::zero(), Integer::from(10_i64)).unwrap();
        let bounded =
            mint_type_declaration_identity("test/orders@1.0.0", &name, DeclaredShape::BoundedDomain(&bound));
        let scalar = mint_type_declaration_identity(
            "test/orders@1.0.0",
            &name,
            DeclaredShape::Scalar(&ScalarShape::Integer),
        );
        for other in [a, b, c] {
            assert_ne!(bounded, other);
            assert_ne!(scalar, other);
        }
        assert_ne!(bounded, scalar);
    }

    /// PR #300 review finding 5: a golden digest vector for
    /// `mint_type_declaration_identity`, the same convention
    /// `check::family::mint_declaration_identity_matches_a_checked_in_digest`
    /// establishes -- this catches a reordered `write_str` or a renamed tag
    /// in either this function or `encode_scalar_shape`/
    /// `encode_composite_shape`, which an equality-only test (comparing two
    /// identities minted in the same process) cannot: both sides would move
    /// together and every such test would stay green. Regenerate the
    /// constant only when the preimage grammar change is the one actually
    /// intended, and say so in the commit (this repository's own
    /// digest-freshness rule, `CLAUDE.md`), never to make a red test green.
    #[trace("TC-259", "FR-088-AC-7")]
    #[test]
    fn mint_type_declaration_identity_matches_a_checked_in_digest() {
        let name = [identifier("Order"), identifier("Status")];
        let shape = CompositeShape::Record(vec![
            FieldDeclaration::new("id", NativeValueType::Integer, Presence::Required),
            FieldDeclaration::new(
                "label",
                NativeValueType::Text(
                    crate::value::text::TextType::new(
                        1,
                        40,
                        crate::value::text::TextProfile::UnicodeScalars,
                    )
                    .unwrap(),
                ),
                Presence::Optional,
            ),
        ]);
        let identity = mint_type_declaration_identity(
            "test/orders@1.0.0",
            &name,
            DeclaredShape::Composite(&shape),
        );
        assert_eq!(
            identity.to_string(),
            "8782013256cdbacff29780ca85ab7a41cf4540bf7601141f1d82567bf6ab1630",
            "the preimage byte grammar changed -- see this test's own doc \
             before regenerating this constant"
        );
    }

    /// PR #300 review finding 5: a golden digest vector for
    /// `mint_variant_id`, mirroring the same rationale.
    #[trace("TC-259", "FR-088-AC-10")]
    #[test]
    fn mint_variant_id_matches_a_checked_in_digest() {
        let sum = node_key(7);
        let variant_id = mint_variant_id(sum, &identifier("Active"));
        assert_eq!(
            variant_id.to_string(),
            "25f808ee82e3446d65360b5e87ef36350a58cc6085f3bf3d2200cab7c0998030",
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
    /// conversion, each variant's `VariantId` is computed from the
    /// declaring sum and that variant's member -- not its position -- and
    /// the converted kernel shape carries no `NodeKey`. A sum value carries
    /// its `VariantId`, never a bare index.
    #[trace("TC-252", "FR-088-AC-10")]
    #[test]
    fn c26_sum_preserves_node_id_and_mints_position_independent_variant_ids() {
        let sum_node = node_key(4);
        let active = SumVariant::new(identifier("Active"));
        let closed = SumVariant::new(identifier("Closed"));
        let sum = CheckedTypeNode::Sum {
            node: sum_node,
            variants: SumVariants::new(vec![active.clone(), closed.clone()]).unwrap(),
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

        // Reordering the declared variants changes no variant's VariantId
        // (a digest over sum + member, never an index).
        let reordered = CheckedTypeNode::Sum {
            node: sum_node,
            variants: SumVariants::new(vec![closed, active]).unwrap(),
        };
        assert_eq!(to_kernel_value_type(&reordered), converted);

        // A sum value carries its VariantId, never a bare index.
        assert!(converted.admits(&Value::Enum(active_id)));
    }

    /// PR #300 review finding 6: two variants sharing one declared name
    /// refuse at construction rather than silently collapsing.
    #[trace("TC-252", "FR-088-AC-10")]
    #[test]
    fn sum_variants_refuses_a_duplicate_declared_name() {
        let first = SumVariant::new(identifier("Active"));
        let duplicate = SumVariant::new(identifier("Active"));
        let refusal = SumVariants::new(vec![first, duplicate]).unwrap_err();
        assert_eq!(refusal, DuplicateSumVariant("Active".to_owned()));
    }
}

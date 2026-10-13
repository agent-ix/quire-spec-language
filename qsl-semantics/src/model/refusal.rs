// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`ModelRefusalCause`]: the closed FR-150/151/152/153/272 cause vocabulary.
//!
//! Lives in its own module, lower-level than [`crate::model::normalize`],
//! because [`crate::model::key`] needs the type too and `key` is itself a
//! dependency of `normalize` — defining the enum in `normalize` made `key`'s
//! import of it a module cycle (#141 finding 4). `normalize` re-exports this
//! type at its own path (`crate::model::normalize::ModelRefusalCause`), so
//! nothing outside this crate's module tree observes a rename. This module
//! imports [`crate::model::key::DeclarationKey`]/[`crate::model::key::EffectiveId`]
//! back from `key`: a mutual `use` between sibling modules is not a cycle
//! Rust's compiler rejects (#163 re-review ruling 3) — only a real cyclic
//! type/const definition would be.

use std::collections::BTreeSet;

use crate::model::domain_package::{DomainPackageRef, Multiplicity, ValueTypeRef};
use crate::model::key::{DeclarationKey, EffectiveId};
use crate::model::origin::JsonValue;
use qsl_foundation::diagnostic::{CatalogCode, JsonPointer, ALLOCATION_FAILED};
use qsl_foundation::source::LocatedSpan;
use quire_exact::UniverseId;

/// The offered model selection at a `foreign-model-selection` refusal's
/// two sites (#163 review finding: each offered value stays distinguishable
/// in the typed cause, not just in `detail`'s text).
///
/// The population-document site (`crate::model::population::admit_binding`'s
/// `modelIdentity` check) has only the document's declared `modelIdentity`
/// string — a `PopulationDocument` carries no full `DomainPackageRef` of its
/// own — so it carries that string instead, never a substituted or
/// partially-populated `DomainPackageRef`. The population-declaration site
/// (the same function's population-key resolution, FR-153's "Its
/// declaration key must belong to the binding's ModelSelection") has only
/// the caller-supplied population `DeclarationKey`, so it carries that.
/// There is no effective-view site: an effective view carries its own
/// domain package, so admission has no second package to compare it with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OfferedSelection {
    /// The population document's declared `modelIdentity`.
    Document(String),
    /// The caller-supplied population declaration key.
    Population(DeclarationKey),
}

/// Where an operation signature names a type: the member path of the
/// `typeRef`, and the operation's source artifact and position (both `None`
/// when the node has no source origin), as
/// [`ModelRefusalCause::IntakeMalformedDeclaration`] carries them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureSite {
    /// `Operation.params[i].typeRef` or `Operation.returns.typeRef`.
    pub member: MemberPath,
    /// The operation's source artifact.
    pub artifact: Option<String>,
    /// The operation's source position.
    pub span: Option<LocatedSpan>,
}

/// The declaration kind a [`MemberPath`] is rooted at, as QSpec's
/// `model-complete.md` names members (`TypeDefinition.supertypes[0]`,
/// `Operation.pre`, `constructs[].meaning`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemberRoot {
    /// The package document itself (`$`).
    Package,
    /// A `TypeDefinition` declaration.
    TypeDefinition,
    /// A `PopulationDefinition` declaration.
    PopulationDefinition,
    /// An `Operation` member.
    Operation,
    /// A `Relationship` member.
    Relationship,
    /// The package's `constructs[]` array.
    Constructs,
    /// One case of a variant type.
    Variant,
    /// A field member.
    Field,
}

/// One step below a [`MemberPath`]'s root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemberSegment {
    /// A named member (`supertypes`, `occurrenceField`).
    Name(String),
    /// A zero-based array position (`[0]`).
    Index(usize),
}

/// The member of a declaration a refusal is about, as the spec writes it:
/// `TypeDefinition.transitions[0].emits[0]`. FR-208 and the TC-235 rows name
/// this path in a malformed, missing or unsupported refusal, so the cause
/// carries it as a value rather than only in `detail`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberPath {
    root: MemberRoot,
    segments: Vec<MemberSegment>,
}

impl MemberPath {
    /// The root itself, with no member below it.
    #[must_use]
    pub const fn new(root: MemberRoot) -> Self {
        Self {
            root,
            segments: Vec::new(),
        }
    }

    /// A path rooted at `TypeDefinition`.
    #[must_use]
    pub const fn type_definition() -> Self {
        Self::new(MemberRoot::TypeDefinition)
    }

    /// This path extended by the named member `name`.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.segments.push(MemberSegment::Name(name.into()));
        self
    }

    /// This path extended by array position `index`.
    #[must_use]
    pub fn index(mut self, index: usize) -> Self {
        self.segments.push(MemberSegment::Index(index));
        self
    }

    /// The path's root.
    #[must_use]
    pub const fn root(&self) -> MemberRoot {
        self.root
    }

    /// The path's segments below the root.
    #[must_use]
    pub fn segments(&self) -> &[MemberSegment] {
        &self.segments
    }

    /// The TC-235 spelling of an operation's contract: a type's
    /// `operations[m].pre[k]` or `.post[k]` member is `Operation.pre[k]` /
    /// `Operation.post[k]` (M08 run 6 spells `Operation.pre`). Any other path
    /// is returned unchanged.
    #[must_use]
    #[qsl_attrs::string_edge]
    pub fn operation_contract(self) -> Self {
        match self.segments.as_slice() {
            [MemberSegment::Name(operations), MemberSegment::Index(_), MemberSegment::Name(contract), ..]
                if self.root == MemberRoot::TypeDefinition
                    && operations == "operations"
                    && (contract == "pre" || contract == "post") =>
            {
                Self {
                    root: MemberRoot::Operation,
                    segments: self.segments[2..].to_vec(),
                }
            }
            _ => self,
        }
    }

    /// This path extended by the dotted relative path `relative`
    /// (`kind.module`, `steps[1].emits`). Text that is not such a path
    /// becomes one named segment, so the result always names something.
    #[must_use]
    pub fn child(mut self, relative: &str) -> Self {
        match segments(&format!(".{relative}")) {
            Some(more) => self.segments.extend(more),
            None => self.segments.push(MemberSegment::Name(relative.to_owned())),
        }
        self
    }

    /// The member path of the node the reader locates at `location`, the
    /// reader's own dotted document location (`$.types[3].fields[0]`): the
    /// `types[n]`, `populations[n]` or `constructs[n]` prefix becomes the
    /// root, and the rest the segments.
    #[must_use]
    #[qsl_attrs::string_edge]
    pub fn from_document_location(location: &str) -> Self {
        let rest = location.strip_prefix('$').unwrap_or(location);
        let Some(mut tokens) = segments(rest) else {
            // The reader builds every location itself; an unparseable one is
            // kept whole rather than dropped.
            return Self::new(MemberRoot::Package).name(location);
        };
        let root = match tokens.as_slice() {
            [MemberSegment::Name(name), MemberSegment::Index(_), ..] => match name.as_str() {
                "types" => Some(MemberRoot::TypeDefinition),
                "populations" => Some(MemberRoot::PopulationDefinition),
                "constructs" => Some(MemberRoot::Constructs),
                _ => None,
            },
            _ => None,
        };
        let located = match root {
            Some(root) => Self {
                root,
                segments: tokens.split_off(2),
            },
            None => Self {
                root: MemberRoot::Package,
                segments: tokens,
            },
        };
        located.operation_contract()
    }
}

/// Splits `.name` and `[n]` steps exactly as [`MemberPath`]'s `Display`
/// prints them: a name is non-empty and holds none of `.`, `[`, `]`, and an
/// index is decimal digits. Anything else is `None`.
fn segments(mut rest: &str) -> Option<Vec<MemberSegment>> {
    let mut tokens = Vec::new();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('[') {
            let (digits, tail) = after.split_once(']')?;
            if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            tokens.push(MemberSegment::Index(digits.parse().ok()?));
            rest = tail;
        } else {
            let body = rest.strip_prefix('.')?;
            let end = body.find(['.', '[', ']']).unwrap_or(body.len());
            if end == 0 {
                return None;
            }
            tokens.push(MemberSegment::Name(body[..end].to_owned()));
            rest = &body[end..];
        }
    }
    Some(tokens)
}

/// Text that is not a member path as [`MemberPath`]'s `Display` prints one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseMemberPathError;

impl std::fmt::Display for ParseMemberPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not a member path: it starts with $, TypeDefinition, PopulationDefinition, Operation, Relationship or constructs[], then .name and [n] steps")
    }
}

impl std::error::Error for ParseMemberPathError {}

/// Parses the spec's spelling, the inverse of [`MemberPath`]'s `Display`:
/// `TypeDefinition.transitions[0].emits[0]`, `constructs[].meaning`.
impl std::str::FromStr for MemberPath {
    type Err = ParseMemberPathError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        for (spelling, root) in [
            ("$", MemberRoot::Package),
            ("TypeDefinition", MemberRoot::TypeDefinition),
            ("PopulationDefinition", MemberRoot::PopulationDefinition),
            ("Operation", MemberRoot::Operation),
            ("Relationship", MemberRoot::Relationship),
            ("Variant", MemberRoot::Variant),
            ("Field", MemberRoot::Field),
            ("constructs[]", MemberRoot::Constructs),
        ] {
            if let Some(rest) = text.strip_prefix(spelling) {
                if rest.is_empty() || rest.starts_with(['.', '[']) {
                    return Ok(Self {
                        root,
                        segments: segments(rest).ok_or(ParseMemberPathError)?,
                    });
                }
            }
        }
        Err(ParseMemberPathError)
    }
}

impl std::fmt::Display for MemberPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self.root {
            MemberRoot::Package => "$",
            MemberRoot::TypeDefinition => "TypeDefinition",
            MemberRoot::PopulationDefinition => "PopulationDefinition",
            MemberRoot::Operation => "Operation",
            MemberRoot::Relationship => "Relationship",
            MemberRoot::Variant => "Variant",
            MemberRoot::Field => "Field",
            MemberRoot::Constructs => "constructs[]",
        })?;
        for segment in &self.segments {
            match segment {
                MemberSegment::Name(name) => write!(f, ".{name}")?,
                MemberSegment::Index(index) => write!(f, "[{index}]")?,
            }
        }
        Ok(())
    }
}

/// A [`MemberPath`] read on a named type: the spec's
/// `ExpressLifecycle.states[1]` is `owner` `ExpressLifecycle` and member
/// `TypeDefinition.states[1]`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedMemberPath {
    /// The type whose member this is.
    pub owner: DeclarationKey,
    /// The member, rooted at that type's declaration.
    pub member: MemberPath,
}

/// The closed FR-150/151/152/153/272 cause of a
/// [`crate::model::normalize::ModelRefusal`] (#141 F9a): each variant is one
/// condition a caller must distinguish, checked by the compiler rather than
/// compared by string.
///
/// Shared by every `crate::model` rung that refuses through `ModelRefusal`
/// (`normalize`, `population`, `dispatch`, `conformance`, `systems`) and by
/// [`crate::model::key::EffectiveDeclarationPreimage::validate_derivation`],
/// which refuses a preimage's own well-formedness through the same closed
/// vocabulary rather than a second one-off string pair.
///
/// Variants carry a typed field for every identity/path/name their `detail`
/// text names, wherever every construction site for that variant supplies
/// the same shape of data (#141 F9a, review finding 3), and the *real* type
/// of the underlying value — [`DeclarationKey`] or [`EffectiveId`], not a
/// pre-formatted `String` — wherever the call site holds one (#163 re-review
/// ruling 3). A variant with no natural data, or whose construction sites
/// disagree in shape (a plain count instead of an identity, or a different
/// number of identities), stays a unit variant; `detail`'s free text is
/// still the full account there. [`ModelRefusalCause::as_str`] returns the
/// same tag either way, and no `detail` string changed: this is a
/// structural change only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelRefusalCause {
    /// A dispatch family walk followed more `redefines` edges than the
    /// caller-configured `family_steps` ceiling
    /// ([`crate::model::accounting::ModelNormalizationLimits::family_steps`]).
    FamilySteps {
        /// The dispatch original the family was built from.
        original: DeclarationKey,
        /// The configured `family_steps` bound the walk reached.
        limit: u64,
        /// The edge count the refused charge would have reached.
        reached: u64,
        /// The setting that raises the bound (`model.family_steps`).
        setting: qsl_foundation::Setting,
    },
    /// An operation family closes over an unresolved or unproved method set.
    UnclosedMethodSet,
    /// A dispatch original names an operation absent from the domain package.
    UnknownOriginal {
        /// The absent original.
        original: DeclarationKey,
    },
    /// A dispatch candidate names an operation absent from the domain package.
    UnknownCandidate {
        /// The absent candidate.
        candidate: DeclarationKey,
    },
    /// No candidate in a dispatch family is applicable to the call.
    NoApplicable,
    /// More than one candidate is applicable and none dominates the others.
    MultipleUndominated,
    /// View declarations are not sorted ascending by effective identity.
    UnsortedView {
        /// The effective identity at the out-of-order position.
        at: EffectiveId,
    },
    /// A type generalizes back to itself through its own ancestor path.
    SpecializationCycle {
        /// The ancestor that closes the cycle.
        ancestor: DeclarationKey,
        /// The object type whose own `supertypes[]` entry the cycle is
        /// discovered via.
        via: DeclarationKey,
    },
    /// A field or operation member names an owner that is not a declared
    /// object type.
    UnknownOwner {
        /// The member naming the owner.
        member: DeclarationKey,
        /// The absent owner.
        owner: DeclarationKey,
    },
    /// An object type's own `supertypes[]` property names a general that is
    /// not a declared object type.
    UnknownGeneral {
        /// The owning object type.
        supertype: DeclarationKey,
        /// The absent general.
        general: DeclarationKey,
    },
    /// An operation parameter or result names a value type that is not
    /// declared.
    UnknownValueType {
        /// The operation naming the value type.
        operation: DeclarationKey,
        /// The parameter naming it, or `None` when a result names it
        /// instead — the two construction sites' only difference in shape.
        parameter: Option<DeclarationKey>,
        /// The absent value type.
        value_type: ValueTypeRef,
        /// Where the operation names it (boxed to keep the refusal small).
        site: Box<SignatureSite>,
    },
    /// An operation effect writes a field that is not a declared member.
    UnknownFieldWrite {
        /// The operation whose effect writes the field.
        operation: DeclarationKey,
        /// The absent field.
        field: DeclarationKey,
    },
    /// An operation effect names a type that is not a declared object type.
    UnknownEffectType {
        /// The operation whose effect names the type.
        operation: DeclarationKey,
        /// The absent type.
        type_name: DeclarationKey,
    },
    /// A field or operation member's own `redefines` or `subsets` property
    /// names a member absent from its owner.
    UnknownMember {
        /// The member declaring the `redefines`/`subsets` property.
        record: DeclarationKey,
        /// The absent member.
        member: DeclarationKey,
    },
    /// A type has two or more undominated redefinitions of one member.
    DerivationConflict {
        /// The type with the conflicting redefinitions.
        type_: DeclarationKey,
        /// The contended redefinition target.
        member: DeclarationKey,
        /// The undominated redefining members, in domain package order.
        redefiners: Vec<DeclarationKey>,
        /// The member paths the TC-235 rows name for the conflict, the
        /// redefining side first (`ExpressLifecycle.states[1]`, then
        /// `OrderLifecycle.states[1]`). Empty where no row names one.
        paths: Vec<OwnedMemberPath>,
    },
    /// A required item does not supply the producer capability its
    /// interface revision requires.
    UnsuppliedProducerRecord,
    /// A conformance walk expanded more ancestor types than the
    /// caller-configured `ancestor_steps` ceiling
    /// ([`crate::model::accounting::ModelNormalizationLimits::ancestor_steps`]).
    AncestorSteps {
        /// The conformance check's starting specific.
        from: DeclarationKey,
        /// The configured `ancestor_steps` bound the walk reached.
        limit: u64,
        /// The edge count the refused charge would have reached.
        reached: u64,
        /// The setting that raises the bound: `model.ancestor_steps` for a
        /// normalization or conformance walk, `admission.ancestor_steps`
        /// for a population admission walk.
        setting: qsl_foundation::Setting,
    },
    /// A redefining or subsetting result/effect does not conform to the
    /// redefined/subsetted one under FR-151 variance.
    VarianceResult,
    /// A redefining or subsetting multiplicity does not conform to the
    /// redefined/subsetted one.
    MultiplicityNarrowing {
        /// The redefining/subsetting/parameter/result/connection-end
        /// multiplicity.
        from: Multiplicity,
        /// The redefined/subsetted/port multiplicity it does not conform
        /// to.
        to: Multiplicity,
    },
    /// A subsetting field's value type does not conform to the subsetted
    /// field's value type.
    SubsettingType {
        /// The subsetting field's value type.
        subsetting: ValueTypeRef,
        /// The subsetted field's value type.
        subsetted: ValueTypeRef,
    },
    /// Two compared items disagree in kind, arity or declared type where
    /// FR-151/FR-152 require agreement.
    TypeMismatch,
    /// A redefining operation parameter does not conform to the redefined
    /// one under contravariant parameter checking.
    VarianceParameter {
        /// The parameter's position.
        index: usize,
        /// The redefining (declared) parameter's value type.
        declared: ValueTypeRef,
        /// The redefined parameter's value type.
        redefined: ValueTypeRef,
    },
    /// A redefining operation's effect writes a field the redefined
    /// operation's effect does not cover.
    EffectEscape {
        /// The uncovered write.
        field: DeclarationKey,
    },
    /// A redefinition narrows a fact FR-146 has no proof form to establish,
    /// or narrows past an established fact with no supporting proof.
    UnprovedRefinement,
    /// A redefinition whose target is not a member inherited by its owning
    /// type, or two or more members of one type that redefine one target
    /// with no single valid candidate remaining after dominance (#204 round
    /// 1: typed like [`Self::DerivationConflict`], the sibling cause for
    /// the other R07 ambiguity shape).
    RedefinitionTarget {
        /// The contending redefining members sharing one owner, with no
        /// single owner dominating a resolvable choice among them, or every
        /// redefining member naming an unreachable target. Empty where no
        /// redefining member's own claim is even known (a construction site
        /// with no candidate edge to name).
        redefiners: Vec<DeclarationKey>,
        /// The contended or unreachable redefinition target.
        target: DeclarationKey,
    },
    /// A population document or population key names a model selection
    /// other than the admitting domain package's.
    ForeignModelSelection {
        /// The offered model selection: the document's `modelIdentity`, or
        /// the population key that names no declaration of the package.
        actual: OfferedSelection,
        /// The admitting domain package's model selection, in full.
        expected: DomainPackageRef,
    },
    /// A population's declared extent is not `closed`.
    IncompleteScope {
        /// The model selection named by the not-`closed` population.
        selection: String,
    },
    /// A closed-extent population's generalization graph is not itself
    /// closed.
    UnclosedSubtypes {
        /// The model selection.
        selection: String,
        /// A member's type from the population document's first member, or
        /// `None` when the document declares no members.
        type_name: Option<DeclarationKey>,
    },
    /// A population member's type is not covered by the population's own
    /// declared `member_types` (FR-153:57/:72: covered means the member
    /// type itself, or a type conforming to a declared member type), or is
    /// absent from the effective view outright.
    ForeignType {
        /// The member.
        member: String,
        /// The uncovered or absent type.
        type_name: DeclarationKey,
    },
    /// D05 (`model-complete.md:156`): a population member's most-specific
    /// type is abstract, which has no direct instances.
    AbstractInstance {
        /// The member.
        member: String,
        /// The member's abstract most-specific type.
        abstract_type: DeclarationKey,
    },
    /// A population declaration's `member_types` names a type that is not a
    /// declared object type (model-complete.md's "Populations" row).
    UnknownPopulationMemberType {
        /// The population declaration naming the member type.
        population: DeclarationKey,
        /// The absent type.
        type_name: DeclarationKey,
    },
    /// One object is declared with two conflicting types.
    ConflictingIdentity {
        /// The object.
        object: String,
        /// Its already-recorded type.
        existing_type: EffectiveId,
        /// The newly declared, conflicting type.
        declared_type: EffectiveId,
    },
    /// An operator is applied to a target it cannot act on: (FR-151,
    /// `quire.model.dispatch.single/v1`) a dispatch target with a declared
    /// result and effect set that disqualify it as a query.
    OperatorIneligible,
    /// A selected population count exceeds its declared maximum.
    AboveMaximum {
        /// The selected count.
        selected: usize,
        /// The declared maximum it exceeds.
        maximum: usize,
    },
    /// A reference key names a universe other than the binding's.
    ForeignUniverse {
        /// The reference key's raw universe bytes, exactly as supplied. Not
        /// always a well-formed 32-byte identity: a
        /// [`crate::model::population::LookupKey`] whose `universe` is some
        /// other length is also refused under this cause
        /// (`crate::model::population::lookup`'s own raw-byte universe
        /// comparison, never a separate malformed case), reporting the bytes
        /// the caller actually supplied rather than a substituted or
        /// truncated identity.
        actual: Vec<u8>,
        /// The binding's universe (ADR-013 §8 OQ-C ruling: a `UniverseId`,
        /// not an `EffectiveId`).
        expected: UniverseId,
    },
    /// A reference key is not a member of the bound population.
    AbsentKey {
        /// The population binding the query ran against, rendered for the
        /// catalog's `binding` payload item (its `PopulationId`'s display).
        binding: String,
        /// The absent key's raw object bytes, exactly as supplied. A
        /// [`crate::model::population::LookupKey`]'s plain `object` bytes,
        /// not a [`DeclarationKey`] -- not always valid UTF-8 itself
        /// (`crate::model::population::lookup`'s own raw-byte membership
        /// check, never a separate malformed case), reporting the bytes the
        /// caller actually supplied rather than a substituted or
        /// lossily-decoded string.
        key: Vec<u8>,
    },
    /// A domain package record does not export the required [`crate::model::key`]
    /// kind for its role.
    WrongExport,
    /// Two records in the same domain package share one [`DeclarationKey`]
    /// (FR-154: "Two nodes share one identity"). Under #131's flat
    /// `DeclarationKey` (`package`/`node` only, no `revision`/`digest`), a
    /// domain package that declares the same node identity twice is no
    /// longer distinguishable by revision and must refuse rather than
    /// silently let the later record replace or shadow the earlier one in
    /// every by-key index this module builds.
    ConflictingBinding {
        /// What repeats. A package-level duplicate carries the key more
        /// than one record declares; a duplicate state or transition
        /// carries the repeated member's own key (TC-235 M06); a duplicate
        /// process step carries the process (TC-235 M07 run 3).
        key: DeclarationKey,
        /// The member the duplicate sits at (`TypeDefinition.states[3]`),
        /// or `None` for a declaration key declared twice at package level.
        member: Option<MemberPath>,
    },
    /// A systems relationship names an endpoint absent from the domain package.
    UnknownRelationship {
        /// The absent relationship.
        relationship: DeclarationKey,
    },
    /// A connection's source end names a port that is not a declared
    /// endpoint.
    UnknownSourcePort {
        /// The absent source port.
        port: DeclarationKey,
    },
    /// A connection's target end names a port that is not a declared
    /// endpoint.
    UnknownTargetPort {
        /// The absent target port.
        port: DeclarationKey,
    },
    /// A connection's direction is not compatible with its source/target
    /// ports.
    PortDirection {
        /// The source port.
        source: DeclarationKey,
        /// The target port.
        target: DeclarationKey,
    },
    /// `check_field_redefinition`'s `redefining_key` argument names a field
    /// absent from the domain package.
    UnknownRedefining {
        /// The absent redefining member.
        member: DeclarationKey,
    },
    /// `check_field_redefinition`'s `redefined_key` argument names a field
    /// absent from the domain package.
    UnknownRedefined {
        /// The absent redefined member.
        member: DeclarationKey,
    },
    /// `check_subsetting`'s `subsetting_key` argument names a field absent
    /// from the domain package.
    UnknownSubsetting {
        /// The absent subsetting member.
        member: DeclarationKey,
    },
    /// `check_subsetting`'s `subsetted_key` argument names a field absent
    /// from the domain package.
    UnknownSubsetted {
        /// The absent subsetted member.
        member: DeclarationKey,
    },
    /// An endpoint's owning component names a key absent from the domain package.
    UnknownComponent {
        /// The endpoint naming the owning component.
        item: DeclarationKey,
        /// The absent component.
        missing: DeclarationKey,
    },
    /// A relationship end names a type identity absent from the domain package.
    UnknownEndpoint {
        /// Which end: `"source"` or `"target"`.
        end: &'static str,
        /// The relationship naming the endpoint.
        relationship: DeclarationKey,
        /// The absent endpoint (#163 re-review ruling 3: structured data —
        /// `end`/`relationship`/`missing` — not the prose
        /// `"{end} end of {relationship}"` `item` string the original PR
        /// carried).
        missing: DeclarationKey,
    },
    /// An [`crate::model::key::EffectiveDeclarationPreimage`]'s derivation
    /// fact has an `ordinal` that does not match its array position.
    UnsortedDerivation {
        /// The preimage's own original declaration.
        original: DeclarationKey,
        /// The out-of-order fact's array position.
        position: usize,
        /// The fact's actual (wrong) ordinal.
        ordinal: usize,
    },
    /// An [`crate::model::key::EffectiveDeclarationPreimage`]'s derivation
    /// retains the same input path at two positions.
    DuplicatePath {
        /// The preimage's own original declaration.
        original: DeclarationKey,
        /// The earlier position retaining the path.
        earlier: usize,
        /// The later position retaining the same path.
        later: usize,
    },
    /// A scalar type's declared domain has a lower bound greater than its
    /// upper bound. FR-272's `invalid_model_binding` cause list is closed;
    /// there is no dedicated scalar-domain variant, so this is the
    /// catalogued `malformed-declaration` (#157). Stays a unit variant: its
    /// one construction site's data is the domain's own numeric
    /// lower/upper bounds, not an identity/path/name.
    MalformedDeclaration,
    /// FR-154's own intake malformed-declaration cause
    /// (`model-complete.md`:74-83): a read IR node is not a well-formed
    /// declaration under QSpec's own shape -- carrying the node's own
    /// identity, its source artifact and its span, exactly as FR-154
    /// requires a malformed-declaration refusal to carry. Kept distinct
    /// from the unit [`Self::MalformedDeclaration`] above (#157's scalar-
    /// domain-bounds case, whose one construction site has no identity of
    /// its own): every [`crate::model::intake`] construction site shares
    /// this one shape, so it carries it, sharing the same tag string via
    /// [`Self::as_str`] rather than a second free-text-only cause.
    IntakeMalformedDeclaration {
        /// The node's own identity string, exactly as the wire supplied it
        /// -- not a [`DeclarationKey`], since the identity itself may be
        /// the very thing that is malformed.
        node: String,
        /// The node's source artifact (FCD's `origin.source.sourceIdentity`),
        /// or `None` when the node's origin is `generated` rather than
        /// `source`, or is itself absent or malformed.
        artifact: Option<String>,
        /// The node's source position, mapped from FCD's `origin.source`
        /// (`startLine`/`startColumn`, one-based) into
        /// [`qsl_foundation::source::LocatedSpan`]'s own `{start, end}` shape. FCD's
        /// wire carries a start position only, no byte offset and no end
        /// position, so both ends of the mapped span are that same point
        /// and its byte offset is `0` -- a placeholder QSL does not treat
        /// as meaningful. `None` under the same conditions as `artifact`.
        span: Option<LocatedSpan>,
        /// The member the node is malformed at, as FR-208 and the TC-235 rows
        /// name it (`TypeDefinition.occurrenceField`).
        member: MemberPath,
    },
    /// An original model reference names no declaration in the selected
    /// package. The path and FCD origin belong to the referring node.
    IntakeMissingDeclaration {
        /// The referring node's original identity.
        node: String,
        /// The absent declaration's original identity.
        reference: String,
        /// The exact FCD origin, including generated provenance or all source coordinates.
        origin: JsonValue,
        /// The reader's original document pointer to the reference.
        path: JsonPointer,
    },
    /// A type's resolved construct meaning, or a member capability, is real
    /// under FR-208 but [`crate::model::intake`] has no reader for it yet
    /// (model-complete.md's declaration-kinds table) -- distinct from
    /// [`Self::IntakeMalformedDeclaration`]'s "not a real QSpec meaning at
    /// all": this one names a legitimate declaration kind this reader does
    /// not yet turn into a [`crate::model::domain_package::DomainPackageRecord`].
    UnsupportedDeclarationForm {
        /// The node's own identity.
        node: String,
        /// The unresolved but real capability: a construct meaning id
        /// (e.g. `quire.meaning.model.record-value-type/v1`) or a short
        /// label naming the unread member (e.g. `"operation.frame"`).
        what: String,
        /// The member the unsupported capability sits at
        /// (`TypeDefinition.supertypes`).
        member: MemberPath,
    },
    /// FR-103 (QSpec `model-complete.md`'s Frames row): an operation's
    /// `modifies`, `creates` or `deletes` entry names no declaration of
    /// this domain package. Carries the operation's own node/artifact/span
    /// triple, the same shape [`Self::IntakeMalformedDeclaration`] does,
    /// plus the entry itself.
    FrameEntryMissing {
        /// The operation's own identity string.
        node: String,
        /// The operation's source artifact, as
        /// [`Self::IntakeMalformedDeclaration`]'s own field.
        artifact: Option<String>,
        /// The operation's source position, as
        /// [`Self::IntakeMalformedDeclaration`]'s own field.
        span: Option<LocatedSpan>,
        /// The frame entry identity string that resolved to nothing.
        entry: String,
    },
    /// FR-103: an operation's `modifies` entry names a relationship, or its
    /// `creates`/`deletes` entry names a process. QSpec's Frames row admits
    /// both, but no FR-106 observation carries a relationship link or a
    /// process instance, so a grant over either could never be enforced.
    FrameEntryUnsupported {
        /// The operation's own identity string.
        node: String,
        /// The operation's source artifact, as
        /// [`Self::IntakeMalformedDeclaration`]'s own field.
        artifact: Option<String>,
        /// The operation's source position, as
        /// [`Self::IntakeMalformedDeclaration`]'s own field.
        span: Option<LocatedSpan>,
        /// The frame entry identity string that names the relationship or
        /// process.
        entry: String,
    },
    /// FR-103: an operation's `modifies`/`creates`/`deletes` entry names a
    /// declaration of this domain package, but of the wrong meaning for
    /// that member (e.g. `modifies` naming an object type, `creates`
    /// naming a field). Shares [`Self::IntakeMalformedDeclaration`]'s tag
    /// (`malformed-declaration`) but, like [`Self::FrameEntryMissing`] and
    /// [`Self::FrameEntryUnsupported`], carries the entry itself in a
    /// structured field rather than only in free-text `detail`.
    FrameEntryMalformed {
        /// The operation's own identity string.
        node: String,
        /// The operation's source artifact, as
        /// [`Self::IntakeMalformedDeclaration`]'s own field.
        artifact: Option<String>,
        /// The operation's source position, as
        /// [`Self::IntakeMalformedDeclaration`]'s own field.
        span: Option<LocatedSpan>,
        /// The frame entry identity string that names a declaration of the
        /// wrong meaning for its member.
        entry: String,
    },
    /// FR-154 Intake check 1 (`model-complete.md:67`): the selection's
    /// digest domain is not the one domain Intake accepts.
    DigestDomainMismatch {
        /// The one digest domain Intake accepts (`sha256-jcs`).
        expected: &'static str,
        /// The selection's actual digest domain.
        actual: String,
    },
    /// FR-154 Intake check 2 (`model-complete.md:68`): the package input
    /// supplies no bytes under the selection's digest.
    MissingSelection {
        /// The caller's selection, already admitted past check 1.
        selection: DomainPackageRef,
    },
    /// FR-154 Intake check 3 (`model-complete.md:69`): the canonical
    /// `sha256-jcs` digest of the supplied domain-package document differs
    /// from the selected digest.
    ContentMismatch {
        /// The selected digest.
        selected: [u8; 32],
        /// The digest recomputed from the supplied document.
        recomputed: [u8; 32],
    },
    /// FR-154 Intake check 3 (`model-complete.md:69`) for bytes the shared
    /// reader refuses: such bytes have no canonical form, so the raw bytes'
    /// own digest is compared with the selected digest and differs.
    ByteDigestMismatch {
        /// The selected digest.
        expected: [u8; 32],
        /// The digest of the supplied raw bytes.
        actual: [u8; 32],
    },
    /// ADR-011 Limits: a domain package document reached one of intake's
    /// parse limits, so it has no parsed form to digest or read. Names the
    /// limit and its bound. The limit is a caller-set setting carried by
    /// the intake limits type (FR-255 Behavior 1, FR-260 item 5), so it refuses
    /// `resource_exhausted`.
    IntakeLimitExceeded {
        /// The limit the document reached.
        limit: IntakeLimit,
        /// That limit's bound.
        bound: u64,
        /// What the document measured: its length in bytes for the byte
        /// limit.
        actual: u64,
    },
    /// Canonical encoding or digesting could not reserve memory. Not a
    /// limit: it names no bound and no setting.
    AllocationFailed {
        /// The size in bytes of the reservation that failed.
        requested: usize,
    },
    /// FR-056: a domain package document carries a number with no exact
    /// RFC 8785 spelling. RFC 8785 encodes every number as the shortest
    /// round-trip text of its nearest double, so the document's
    /// `sha256-jcs` digest would be the digest of a different value;
    /// intake's read refuses it before the digest is taken.
    NoncanonicalNumber {
        /// Why the number has no exact spelling; its cause tag.
        inexact: Inexact,
        /// The RFC 6901 pointer of the first such number, in document
        /// order.
        document_pointer: JsonPointer,
    },
    /// FR-154 Intake check 4 (`model-complete.md:70`): the package's own
    /// identity and version disagree with the selection.
    WrongModelSelection {
        /// The caller's selection.
        selection: DomainPackageRef,
        /// The package's own declared identity.
        actual_identity: String,
        /// The package's own declared version.
        actual_version: String,
    },
    /// ADR-010 OBS-006 / ADR-013 O-03: a selection names the reserved
    /// `quire/native` pseudo-package identity
    /// (`crate::model::intake::native::RESERVED_IDENTITY`), which shares no
    /// key space with a real domain package -- a native value type
    /// resolves only as `ValueTypeRef::Native`. FR-272's
    /// `invalid_model_binding` cause list has no dedicated variant for
    /// this, so this shares the catalogued `malformed-declaration` tag with
    /// [`Self::MalformedDeclaration`] (#157's precedent for a condition the
    /// closed catalog does not name separately).
    ReservedPackageIdentity {
        /// The offered selection naming the reserved identity.
        selection: DomainPackageRef,
    },
    /// ADR-013 O-01/QC-5, catalogued `duplicate_selection`/`duplicate-identity`
    /// (revision `1-draft.6`): a package selects at most one version of a
    /// domain-package identity; a second selection of the same identity
    /// refuses at intake, whether or not the requested version matches the
    /// one already selected.
    DuplicateSelection {
        /// The repeated domain-package identity.
        identity: String,
        /// The version already selected for `identity`.
        already_selected_version: String,
        /// The version this second selection requested.
        requested_version: String,
    },
    /// A population member declares the same field twice in its
    /// `field_values`. FR-272's `invalid_runtime_input` cause list is
    /// closed; there is no dedicated duplicate-field variant, so this is
    /// the catalogued `duplicate-member` (#157).
    DuplicateMember {
        /// The object declaring the field twice.
        object: String,
        /// The duplicated field.
        field: DeclarationKey,
    },
    /// A population member's subsetting-feature value is not among its
    /// subsetted feature's values (#157).
    SubsettingViolation {
        /// The object whose subsetting feature is violated.
        object: String,
        /// The subsetting field.
        subsetting: DeclarationKey,
        /// The subsetted field.
        subsetted: DeclarationKey,
    },
    /// FR-046/FR-151: an invocation creates an object outside its
    /// operation's declared `creates` frame. One of four distinct
    /// `crate::model::population::decide_frame` conditions sharing the
    /// catalogued `Code::FrameViolation`/`unauthorized-change` cause tag
    /// (`native-diagnostics.md`); kept as its own variant, like every other
    /// cause in this enum, so a caller distinguishes it by match arm rather
    /// than by re-parsing `detail`.
    FrameCreateOutsideGrant {
        /// The created object.
        object: String,
        /// Its most-specific type.
        type_name: DeclarationKey,
    },
    /// FR-046/FR-151: an invocation changes an object's most-specific type
    /// between pre and post, which no `creates`/`deletes`/`modifies`
    /// grant may authorize. `Code::FrameViolation`/`unauthorized-change`.
    FrameTypeChanged {
        /// The object whose type changed.
        object: String,
        /// Its pre most-specific type.
        pre_type: DeclarationKey,
        /// Its post most-specific type.
        post_type: DeclarationKey,
    },
    /// FR-046/FR-151: an invocation deletes an object outside its
    /// operation's declared `deletes` frame. `Code::FrameViolation`/
    /// `unauthorized-change`.
    FrameDeleteOutsideGrant {
        /// The deleted object.
        object: String,
        /// Its pre most-specific type.
        type_name: DeclarationKey,
    },
    /// FR-046/FR-151: an invocation changes a surviving object's field
    /// outside its operation's declared `modifies` frame.
    /// `Code::FrameViolation`/`unauthorized-change`.
    FrameFieldWriteOutsideGrant {
        /// The object whose field changed.
        object: String,
        /// The changed field.
        field: DeclarationKey,
    },
    /// FR-046: an invocation's caller-supplied delta declares the same
    /// identity more than once in one of its own `created`/`deleted` lists.
    /// One of three distinct `crate::model::population::check_declared_delta`
    /// conditions sharing the catalogued `Code::PopulationDeltaMismatch`/
    /// `delta-disagreement` cause tag.
    DuplicateDeclaredIdentity {
        /// The identity declared twice in the same list.
        identity: String,
    },
    /// FR-046: an invocation's caller-supplied delta declares the same
    /// identity as both created and deleted. `Code::PopulationDeltaMismatch`/
    /// `delta-disagreement`.
    DeclaredCreateDeleteOverlap {
        /// The identity declared as both created and deleted.
        identity: String,
    },
    /// FR-046: an invocation's caller-supplied delta disagrees with the
    /// complete created/deleted sets `decide_frame` computed from the pre
    /// and post populations. `Code::PopulationDeltaMismatch`/
    /// `delta-disagreement`.
    DeclaredDeltaMismatch {
        /// The delta's own declared created identities.
        declared_created: BTreeSet<String>,
        /// The delta's own declared deleted identities.
        declared_deleted: BTreeSet<String>,
        /// The complete created identities computed from pre/post.
        computed_created: BTreeSet<String>,
        /// The complete deleted identities computed from pre/post.
        computed_deleted: BTreeSet<String>,
    },
}

/// One parse limit a domain package document is read under
/// ([`ModelRefusalCause::IntakeLimitExceeded`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntakeLimit {
    /// The document's length in bytes.
    InputBytes,
}

impl IntakeLimit {
    /// The setting that raises this limit (FR-255).
    pub const fn setting(self) -> qsl_foundation::Setting {
        match self {
            Self::InputBytes => qsl_foundation::Setting::IntakeInputBytes,
        }
    }

    /// The limit's name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InputBytes => "input_bytes",
        }
    }
}

/// Why a document's number has no exact RFC 8785 spelling
/// ([`ModelRefusalCause::NoncanonicalNumber`], FR-056).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Inexact {
    /// The number denotes a whole value whose magnitude exceeds 2^53.
    Integer,
    /// The number's exact value is not the exact value of the shortest
    /// round-trip text of its nearest double, such as
    /// `0.1000000000000000000001` or `1e-400`.
    Number,
}

impl Inexact {
    /// The cause tag: `inexact-integer` or `inexact-number`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Integer => "inexact-integer",
            Self::Number => "inexact-number",
        }
    }
}

impl ModelRefusalCause {
    /// An ancestor-steps refusal: the walk from `from` reached `limit`
    /// edges, and the next edge, the one past the bound, would have made
    /// `limit + 1`. `setting` names the bound.
    pub fn ancestor_steps(
        from: DeclarationKey,
        limit: u64,
        setting: qsl_foundation::Setting,
    ) -> Self {
        Self::AncestorSteps {
            from,
            limit,
            reached: limit.saturating_add(1),
            setting,
        }
    }

    /// A family-steps refusal: the dispatch family of `original` reached
    /// `limit` edges, and the next one would have made `limit + 1`.
    pub fn family_steps(original: DeclarationKey, limit: u64) -> Self {
        Self::FamilySteps {
            original,
            limit,
            reached: limit.saturating_add(1),
            setting: qsl_foundation::Setting::ModelFamilySteps,
        }
    }

    /// This refusal as the caller-budget denial FR-255 Behavior 1 describes,
    /// when it is a reached walk ceiling or intake's byte limit. A walk
    /// ceiling is a denied meter charge of one edge past the bound
    /// (QSpec FR-150-AC-11: `incomplete(ancestor_steps, limit, consumed,
    /// 1, model.ancestor-edge)`), so it refuses
    /// `resource_exhausted`/`insufficient-next-charge`; intake's byte limit
    /// is a size ceiling and keeps `intake-limit-exceeded`.
    pub fn denial(&self) -> Option<qsl_foundation::diagnostic::BudgetDenial> {
        use crate::model::accounting::MODEL_NORMALIZATION_OWNER;
        use crate::model::population::ADMISSION_OWNER;
        use qsl_foundation::diagnostic::{BudgetDenial, DeniedCharge};
        use qsl_foundation::Setting;
        // A walk ceiling is a denied one-edge charge past the limit.
        let walk = |setting, limit, owner, limit_kind, charge_point| {
            BudgetDenial::charge(
                setting,
                limit,
                DeniedCharge {
                    owner,
                    limit_kind,
                    charge_point,
                    consumed: limit,
                    next_charge: 1,
                },
            )
        };
        match self {
            Self::AncestorSteps { limit, setting, .. } => {
                Some(if *setting == Setting::AdmissionAncestorSteps {
                    walk(
                        *setting,
                        *limit,
                        ADMISSION_OWNER,
                        "ancestor_steps",
                        "admission.ancestor-edge",
                    )
                } else {
                    walk(
                        *setting,
                        *limit,
                        MODEL_NORMALIZATION_OWNER,
                        "ancestor_steps",
                        "model.ancestor-edge",
                    )
                })
            }
            Self::FamilySteps { limit, setting, .. } => Some(walk(
                *setting,
                *limit,
                MODEL_NORMALIZATION_OWNER,
                "family_steps",
                "model.family-edge",
            )),
            Self::IntakeLimitExceeded {
                limit,
                bound,
                actual,
            } => Some(BudgetDenial::size(
                limit.setting(),
                *bound,
                u128::from(*actual),
            )),
            _ => None,
        }
    }

    /// FR-096: the catalog payload of this cause, for a cause with a row in
    /// FR-096's key table (the walk ceilings and intake's byte limit take
    /// FR-255 Behavior 1's caller-budget record, [`Self::denial`]), else `None`: a cause with no row has no fields to
    /// give, and an empty map would pass for a record with none required.
    /// Read from the variant, never from `detail`. Matched exhaustively, so
    /// a new cause forces a decision here.
    #[deny(clippy::wildcard_enum_match_arm)]
    pub fn catalog_fields(&self) -> Option<std::collections::BTreeMap<&'static str, String>> {
        match self {
            Self::AbsentKey { binding, key } => Some(std::collections::BTreeMap::from([
                ("binding", binding.clone()),
                ("key", render_identity(key)),
            ])),
            Self::ForeignUniverse { actual, expected } => Some(std::collections::BTreeMap::from([
                ("required", crate::model::key::hex(expected.as_bytes())),
                ("supplied", crate::model::key::hex(actual)),
            ])),
            // A caller-budget ceiling (ADR-014 section 1): FR-255 Behavior 1's
            // record, built once by `BudgetDenial`.
            Self::AncestorSteps { .. }
            | Self::FamilySteps { .. }
            | Self::IntakeLimitExceeded { .. } => {
                self.denial().map(|denial| denial.catalog_fields())
            }
            Self::UnclosedMethodSet { .. }
            | Self::UnknownOriginal { .. }
            | Self::UnknownCandidate { .. }
            | Self::NoApplicable { .. }
            | Self::MultipleUndominated { .. }
            | Self::UnsortedView { .. }
            | Self::SpecializationCycle { .. }
            | Self::UnknownOwner { .. }
            | Self::UnknownGeneral { .. }
            | Self::UnknownValueType { .. }
            | Self::UnknownFieldWrite { .. }
            | Self::UnknownEffectType { .. }
            | Self::UnknownMember { .. }
            | Self::DerivationConflict { .. }
            | Self::UnsuppliedProducerRecord { .. }
            | Self::VarianceResult { .. }
            | Self::MultiplicityNarrowing { .. }
            | Self::SubsettingType { .. }
            | Self::TypeMismatch { .. }
            | Self::VarianceParameter { .. }
            | Self::EffectEscape { .. }
            | Self::UnprovedRefinement { .. }
            | Self::RedefinitionTarget { .. }
            | Self::ForeignModelSelection { .. }
            | Self::IncompleteScope { .. }
            | Self::UnclosedSubtypes { .. }
            | Self::ForeignType { .. }
            | Self::AbstractInstance { .. }
            | Self::UnknownPopulationMemberType { .. }
            | Self::ConflictingIdentity { .. }
            | Self::OperatorIneligible { .. }
            | Self::AboveMaximum { .. }
            | Self::WrongExport { .. }
            | Self::ConflictingBinding { .. }
            | Self::UnknownRelationship { .. }
            | Self::UnknownSourcePort { .. }
            | Self::UnknownTargetPort { .. }
            | Self::PortDirection { .. }
            | Self::UnknownRedefining { .. }
            | Self::UnknownRedefined { .. }
            | Self::UnknownSubsetting { .. }
            | Self::UnknownSubsetted { .. }
            | Self::UnknownComponent { .. }
            | Self::UnknownEndpoint { .. }
            | Self::UnsortedDerivation { .. }
            | Self::DuplicatePath { .. }
            | Self::MalformedDeclaration { .. }
            | Self::IntakeMalformedDeclaration { .. }
            | Self::IntakeMissingDeclaration { .. }
            | Self::UnsupportedDeclarationForm { .. }
            | Self::FrameEntryMissing { .. }
            | Self::FrameEntryUnsupported { .. }
            | Self::FrameEntryMalformed { .. }
            | Self::DigestDomainMismatch { .. }
            | Self::MissingSelection { .. }
            | Self::ContentMismatch { .. }
            | Self::ByteDigestMismatch { .. }
            | Self::AllocationFailed { .. }
            | Self::NoncanonicalNumber { .. }
            | Self::WrongModelSelection { .. }
            | Self::ReservedPackageIdentity { .. }
            | Self::DuplicateSelection { .. }
            | Self::DuplicateMember { .. }
            | Self::SubsettingViolation { .. }
            | Self::FrameCreateOutsideGrant { .. }
            | Self::FrameTypeChanged { .. }
            | Self::FrameDeleteOutsideGrant { .. }
            | Self::FrameFieldWriteOutsideGrant { .. }
            | Self::DuplicateDeclaredIdentity { .. }
            | Self::DeclaredCreateDeleteOverlap { .. }
            | Self::DeclaredDeltaMismatch { .. } => None,
        }
    }

    /// The cause tag (the exact spelling every FR-150/151/152/153/272 test
    /// tracing and every prior wire-visible string used before #141).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FamilySteps { .. } => "family-steps",
            Self::UnclosedMethodSet => "unclosed-method-set",
            Self::UnknownOriginal { .. } => "unknown-original",
            Self::UnknownCandidate { .. } => "unknown-candidate",
            Self::NoApplicable => "no-applicable",
            Self::MultipleUndominated => "multiple-undominated",
            Self::UnsortedView { .. } => "unsorted-view",
            Self::SpecializationCycle { .. } => "specialization-cycle",
            Self::UnknownOwner { .. } => "unknown-owner",
            Self::UnknownGeneral { .. } => "unknown-general",
            Self::UnknownValueType { .. } => "unknown-value-type",
            Self::UnknownFieldWrite { .. } => "unknown-field-write",
            Self::UnknownEffectType { .. } => "unknown-effect-type",
            Self::UnknownMember { .. } => "unknown-member",
            Self::DerivationConflict { .. } => "derivation-conflict",
            Self::UnsuppliedProducerRecord => "unsupplied-producer-record",
            Self::AncestorSteps { .. } => "ancestor-steps",
            Self::VarianceResult => "variance-result",
            Self::MultiplicityNarrowing { .. } => "multiplicity-narrowing",
            Self::SubsettingType { .. } => "subsetting-type",
            Self::TypeMismatch => "type-mismatch",
            Self::VarianceParameter { .. } => "variance-parameter",
            Self::EffectEscape { .. } => "effect-escape",
            Self::UnprovedRefinement => "unproved-refinement",
            Self::RedefinitionTarget { .. } => "redefinition-target",
            Self::ForeignModelSelection { .. } => "foreign-model-selection",
            Self::IncompleteScope { .. } => "incomplete-scope",
            Self::UnclosedSubtypes { .. } => "unclosed-subtypes",
            Self::ForeignType { .. } => "foreign-type",
            Self::AbstractInstance { .. } => "abstract-instance",
            Self::UnknownPopulationMemberType { .. } => "missing-name",
            Self::ConflictingIdentity { .. } => "conflicting-identity",
            Self::OperatorIneligible => "operator-ineligible",
            Self::AboveMaximum { .. } => "above-maximum",
            Self::ForeignUniverse { .. } => "foreign-universe",
            Self::AbsentKey { .. } => "absent-key",
            Self::WrongExport => "wrong-export",
            Self::ConflictingBinding { .. } => "conflicting-binding",
            Self::UnknownRelationship { .. } => "unknown-relationship",
            Self::UnknownSourcePort { .. } => "unknown-source-port",
            Self::UnknownTargetPort { .. } => "unknown-target-port",
            Self::PortDirection { .. } => "port-direction",
            Self::UnknownRedefining { .. } => "unknown-redefining",
            Self::UnknownRedefined { .. } => "unknown-redefined",
            Self::UnknownSubsetting { .. } => "unknown-subsetting",
            Self::UnknownSubsetted { .. } => "unknown-subsetted",
            Self::UnknownComponent { .. } => "unknown-component",
            Self::UnknownEndpoint { .. } => "unknown-endpoint",
            Self::UnsortedDerivation { .. } => "unsorted-derivation",
            Self::DuplicatePath { .. } => "duplicate-path",
            Self::MalformedDeclaration
            | Self::IntakeMalformedDeclaration { .. }
            | Self::FrameEntryMalformed { .. }
            | Self::ReservedPackageIdentity { .. } => "malformed-declaration",
            Self::DuplicateSelection { .. } => "duplicate-identity",
            Self::UnsupportedDeclarationForm { .. } => "declaration-form",
            Self::FrameEntryMissing { .. } | Self::IntakeMissingDeclaration { .. } => {
                "missing-name"
            }
            Self::FrameEntryUnsupported { .. } => "unsupported-feature",
            Self::DigestDomainMismatch { .. } => "digest-domain-mismatch",
            Self::MissingSelection { .. } => "missing-selection",
            Self::ContentMismatch { .. } => "content-mismatch",
            Self::ByteDigestMismatch { .. } => "byte-digest-mismatch",
            Self::WrongModelSelection { .. } => "wrong-model-selection",
            Self::IntakeLimitExceeded { .. } => "intake-limit-exceeded",
            Self::AllocationFailed { .. } => ALLOCATION_FAILED.cause(),
            Self::NoncanonicalNumber { inexact, .. } => inexact.as_str(),
            Self::DuplicateMember { .. } => "duplicate-member",
            Self::SubsettingViolation { .. } => "subsetting-violation",
            Self::FrameCreateOutsideGrant { .. }
            | Self::FrameTypeChanged { .. }
            | Self::FrameDeleteOutsideGrant { .. }
            | Self::FrameFieldWriteOutsideGrant { .. } => "unauthorized-change",
            Self::DuplicateDeclaredIdentity { .. }
            | Self::DeclaredCreateDeleteOverlap { .. }
            | Self::DeclaredDeltaMismatch { .. } => "delta-disagreement",
        }
    }
}

impl ModelRefusalCause {
    /// ADR-013 O-17: this cause's `quire.native.diagnostics/v1` catalog
    /// code, with [`Self::as_str`] as its cause tag. One exhaustive match,
    /// one arm per catalog code and no `_` arm, read from the cause alone.
    /// `UnsortedDerivation` and `DuplicatePath` share `UnsortedView`'s code:
    /// all three are TC-195 N10 well-formedness refusals of an effective
    /// declaration or view.
    pub fn catalog_code(&self) -> CatalogCode {
        let code = match self {
            Self::UnknownOriginal { .. }
            | Self::UnknownCandidate { .. }
            | Self::UnknownOwner { .. }
            | Self::UnknownGeneral { .. }
            | Self::UnknownValueType { .. }
            | Self::UnknownFieldWrite { .. }
            | Self::UnknownEffectType { .. }
            | Self::UnknownMember { .. }
            | Self::UnknownRelationship { .. }
            | Self::UnknownSourcePort { .. }
            | Self::UnknownTargetPort { .. }
            | Self::UnknownRedefining { .. }
            | Self::UnknownRedefined { .. }
            | Self::UnknownSubsetting { .. }
            | Self::UnknownSubsetted { .. }
            | Self::UnknownComponent { .. }
            | Self::UnknownEndpoint { .. } => "dangling_reference",
            Self::UnsortedView { .. }
            | Self::UnsortedDerivation { .. }
            | Self::DuplicatePath { .. }
            | Self::SpecializationCycle { .. }
            | Self::DerivationConflict { .. }
            | Self::UnsuppliedProducerRecord
            | Self::RedefinitionTarget { .. }
            | Self::WrongExport
            | Self::ConflictingBinding { .. }
            | Self::PortDirection { .. }
            | Self::MalformedDeclaration
            | Self::IntakeMalformedDeclaration { .. }
            | Self::FrameEntryMalformed { .. }
            | Self::ReservedPackageIdentity { .. }
            | Self::WrongModelSelection { .. } => "invalid_model_binding",
            Self::FamilySteps { .. }
            | Self::AncestorSteps { .. }
            | Self::IntakeLimitExceeded { .. } => "resource_exhausted",
            Self::AllocationFailed { .. } => ALLOCATION_FAILED.code(),
            Self::NoncanonicalNumber { .. } => "noncanonical_wire",
            Self::UnclosedMethodSet
            | Self::IncompleteScope { .. }
            | Self::UnclosedSubtypes { .. } => "incomplete_population",
            Self::NoApplicable | Self::MultipleUndominated => "ambiguous_dispatch",
            Self::VarianceResult
            | Self::MultiplicityNarrowing { .. }
            | Self::SubsettingType { .. }
            | Self::TypeMismatch
            | Self::VarianceParameter { .. }
            | Self::EffectEscape { .. }
            | Self::OperatorIneligible => "ill_typed",
            Self::UnprovedRefinement => "undefined_expression",
            Self::ForeignModelSelection { .. }
            | Self::ForeignType { .. }
            | Self::ForeignUniverse { .. } => "foreign_reference",
            Self::AbstractInstance { .. }
            | Self::ConflictingIdentity { .. }
            | Self::AbsentKey { .. }
            | Self::DuplicateMember { .. }
            | Self::SubsettingViolation { .. } => "invalid_runtime_input",
            Self::UnknownPopulationMemberType { .. }
            | Self::FrameEntryMissing { .. }
            | Self::IntakeMissingDeclaration { .. } => "missing_declaration",
            Self::AboveMaximum { .. } => "cardinality_out_of_bound",
            Self::UnsupportedDeclarationForm { .. } => "unsupported_construct",
            Self::FrameEntryUnsupported { .. } => "unknown_required_feature",
            Self::DigestDomainMismatch { .. }
            | Self::ContentMismatch { .. }
            | Self::ByteDigestMismatch { .. } => "stale_dependency",
            Self::MissingSelection { .. } => "missing_import",
            Self::DuplicateSelection { .. } => "duplicate_selection",
            Self::FrameCreateOutsideGrant { .. }
            | Self::FrameTypeChanged { .. }
            | Self::FrameDeleteOutsideGrant { .. }
            | Self::FrameFieldWriteOutsideGrant { .. } => "frame_violation",
            Self::DuplicateDeclaredIdentity { .. }
            | Self::DeclaredCreateDeleteOverlap { .. }
            | Self::DeclaredDeltaMismatch { .. } => "population_delta_mismatch",
        };
        match self {
            // A walk ceiling is a denied meter charge (QSpec FR-150-AC-11,
            // FR-272): the catalog has no `ancestor-steps` or `family-steps`
            // cause, only `insufficient-next-charge`.
            Self::AncestorSteps { .. } | Self::FamilySteps { .. } => {
                CatalogCode::new(code, "insufficient-next-charge")
            }
            _ => CatalogCode::new(code, self.as_str()),
        }
    }
}

impl std::fmt::Display for ModelRefusalCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(any(test, feature = "test-support"))]
pub mod fixtures {
    //! One sample of every [`ModelRefusalCause`] variant, shared by this
    //! module's tests and the layer-5 evaluator's catalog tests
    //! (`value::expression::causes`), which reach it across the
    //! `qsl-semantics` crate boundary through `test-support`.
    use super::{ModelRefusalCause, OfferedSelection};
    use crate::model::domain_package::{DomainPackageRef, Multiplicity, ValueTypeRef};
    use crate::model::key::DeclarationKey;
    use qsl_foundation::source::{LocatedSpan, Position};
    use quire_exact::UniverseId;

    fn key(identity: &str) -> DeclarationKey {
        DeclarationKey::fixture(identity)
    }

    fn effective_id() -> crate::model::key::EffectiveId {
        // The SHA-256 of the RFC 8785 text `null`: any fixed digest serves.
        crate::model::key::EffectiveId::from_digest(
            qsl_foundation::ByteDigest::of(b"null").as_bytes(),
        )
    }

    fn universe_id() -> UniverseId {
        UniverseId::from_digest(*effective_id().as_bytes())
    }

    fn multiplicity() -> Multiplicity {
        Multiplicity {
            lower: 0,
            upper: None,
            ordered: false,
            unique: true,
        }
    }

    /// One sample of every [`ModelRefusalCause`] variant. The macro builds
    /// an exhaustive match over the listed variant names with no `_` arm,
    /// so a variant left out of the list fails to compile (E0004), and it
    /// asserts each sample is the variant it is listed under.
    pub fn exhaustive_samples() -> Vec<ModelRefusalCause> {
        macro_rules! samples {
            ($($variant:ident => $sample:expr),* $(,)?) => {{
                fn covered(cause: &ModelRefusalCause) {
                    match cause {
                        $(ModelRefusalCause::$variant { .. } => {})*
                    }
                }
                vec![$({
                    let sample = $sample;
                    covered(&sample);
                    assert!(
                        matches!(sample, ModelRefusalCause::$variant { .. }),
                        "sample {sample:?} is listed under {}",
                        stringify!($variant)
                    );
                    sample
                }),*]
            }};
        }
        samples![
        FamilySteps => ModelRefusalCause::family_steps(key("p"), 1),
        UnclosedMethodSet => ModelRefusalCause::UnclosedMethodSet,
        UnknownOriginal => ModelRefusalCause::UnknownOriginal { original: key("p") },
        UnknownCandidate => ModelRefusalCause::UnknownCandidate {
            candidate: key("p"),
        },
        NoApplicable => ModelRefusalCause::NoApplicable,
        MultipleUndominated => ModelRefusalCause::MultipleUndominated,
        UnsortedView => ModelRefusalCause::UnsortedView { at: effective_id() },
        SpecializationCycle => ModelRefusalCause::SpecializationCycle {
            ancestor: key("p"),
            via: key("p"),
        },
        UnknownOwner => ModelRefusalCause::UnknownOwner {
            member: key("p"),
            owner: key("p"),
        },
        UnknownGeneral => ModelRefusalCause::UnknownGeneral {
            supertype: key("p"),
            general: key("p"),
        },
        UnknownValueType => ModelRefusalCause::UnknownValueType {
            operation: key("p"),
            parameter: Some(key("p")),
            value_type: ValueTypeRef::Package(key("p")),
            site: Box::new(crate::model::refusal::SignatureSite {
                member: crate::model::refusal::MemberPath::new(
                    crate::model::refusal::MemberRoot::Operation,
                ),
                artifact: None,
                span: None,
            }),
        },
        UnknownFieldWrite => ModelRefusalCause::UnknownFieldWrite {
            operation: key("p"),
            field: key("p"),
        },
        UnknownEffectType => ModelRefusalCause::UnknownEffectType {
            operation: key("p"),
            type_name: key("p"),
        },
        UnknownMember => ModelRefusalCause::UnknownMember {
            record: key("p"),
            member: key("p"),
        },
        DerivationConflict => ModelRefusalCause::DerivationConflict {
            type_: key("p"),
            member: key("p"),
            redefiners: vec![key("p")],
            paths: Vec::new(),
        },
        UnsuppliedProducerRecord => ModelRefusalCause::UnsuppliedProducerRecord,
        AncestorSteps => ModelRefusalCause::ancestor_steps(
            key("p"),
            1,
            qsl_foundation::Setting::ModelAncestorSteps,
        ),
        VarianceResult => ModelRefusalCause::VarianceResult,
        MultiplicityNarrowing => ModelRefusalCause::MultiplicityNarrowing {
            from: multiplicity(),
            to: multiplicity(),
        },
        SubsettingType => ModelRefusalCause::SubsettingType {
            subsetting: ValueTypeRef::Package(key("p")),
            subsetted: ValueTypeRef::Package(key("p")),
        },
        TypeMismatch => ModelRefusalCause::TypeMismatch,
        VarianceParameter => ModelRefusalCause::VarianceParameter {
            index: 0,
            declared: ValueTypeRef::Package(key("p")),
            redefined: ValueTypeRef::Package(key("p")),
        },
        EffectEscape => ModelRefusalCause::EffectEscape { field: key("p") },
        UnprovedRefinement => ModelRefusalCause::UnprovedRefinement,
        RedefinitionTarget => ModelRefusalCause::RedefinitionTarget {
            redefiners: vec![key("p")],
            target: key("p"),
        },
        ForeignModelSelection => ModelRefusalCause::ForeignModelSelection {
            actual: OfferedSelection::Document(String::new()),
            expected: crate::model::domain_package::DomainPackageRef::fixture("p"),
        },
        IncompleteScope => ModelRefusalCause::IncompleteScope {
            selection: String::new(),
        },
        UnclosedSubtypes => ModelRefusalCause::UnclosedSubtypes {
            selection: String::new(),
            type_name: Some(key("p")),
        },
        ForeignType => ModelRefusalCause::ForeignType {
            member: String::new(),
            type_name: key("p"),
        },
        AbstractInstance => ModelRefusalCause::AbstractInstance {
            member: String::new(),
            abstract_type: key("p"),
        },
        UnknownPopulationMemberType => ModelRefusalCause::UnknownPopulationMemberType {
            population: key("p"),
            type_name: key("p"),
        },
        ConflictingIdentity => ModelRefusalCause::ConflictingIdentity {
            object: String::new(),
            existing_type: effective_id(),
            declared_type: effective_id(),
        },
        OperatorIneligible => ModelRefusalCause::OperatorIneligible,
        AboveMaximum => ModelRefusalCause::AboveMaximum {
            selected: 0,
            maximum: 0,
        },
        ForeignUniverse => ModelRefusalCause::ForeignUniverse {
            actual: effective_id().as_bytes().to_vec(),
            expected: universe_id(),
        },
        AbsentKey => ModelRefusalCause::AbsentKey {
            binding: String::new(),
            key: Vec::new()
        },
        WrongExport => ModelRefusalCause::WrongExport,
        ConflictingBinding => ModelRefusalCause::ConflictingBinding {
            key: key("p"),
            member: None,
        },
        UnknownRelationship => ModelRefusalCause::UnknownRelationship {
            relationship: key("p"),
        },
        UnknownSourcePort => ModelRefusalCause::UnknownSourcePort { port: key("p") },
        UnknownTargetPort => ModelRefusalCause::UnknownTargetPort { port: key("p") },
        PortDirection => ModelRefusalCause::PortDirection {
            source: key("p"),
            target: key("p"),
        },
        UnknownRedefining => ModelRefusalCause::UnknownRedefining { member: key("p") },
        UnknownRedefined => ModelRefusalCause::UnknownRedefined { member: key("p") },
        UnknownSubsetting => ModelRefusalCause::UnknownSubsetting { member: key("p") },
        UnknownSubsetted => ModelRefusalCause::UnknownSubsetted { member: key("p") },
        UnknownComponent => ModelRefusalCause::UnknownComponent {
            item: key("p"),
            missing: key("p"),
        },
        UnknownEndpoint => ModelRefusalCause::UnknownEndpoint {
            end: "source",
            relationship: key("p"),
            missing: key("p"),
        },
        UnsortedDerivation => ModelRefusalCause::UnsortedDerivation {
            original: key("p"),
            position: 0,
            ordinal: 0,
        },
        DuplicatePath => ModelRefusalCause::DuplicatePath {
            original: key("p"),
            earlier: 0,
            later: 0,
        },
        MalformedDeclaration => ModelRefusalCause::MalformedDeclaration,
        IntakeMalformedDeclaration => ModelRefusalCause::IntakeMalformedDeclaration {
            node: String::new(),
            artifact: Some(String::new()),
            span: Some(LocatedSpan {
                start: Position {
                    byte: 0,
                    line: 1,
                    column: 1,
                },
                end: Position {
                    byte: 0,
                    line: 1,
                    column: 1,
                },
            }),
            member: super::MemberPath::type_definition(),
        },
        UnsupportedDeclarationForm => ModelRefusalCause::UnsupportedDeclarationForm {
            node: String::new(),
            what: String::new(),
            member: super::MemberPath::type_definition(),
        },
        IntakeMissingDeclaration => ModelRefusalCause::IntakeMissingDeclaration {
            node: String::new(), reference: String::new(), origin: super::JsonValue::default(), path: qsl_foundation::diagnostic::JsonPointer::root(),
        },
        FrameEntryMissing => ModelRefusalCause::FrameEntryMissing {
            node: String::new(),
            artifact: Some(String::new()),
            span: Some(LocatedSpan {
                start: Position {
                    byte: 0,
                    line: 1,
                    column: 1,
                },
                end: Position {
                    byte: 0,
                    line: 1,
                    column: 1,
                },
            }),
            entry: String::new(),
        },
        FrameEntryUnsupported => ModelRefusalCause::FrameEntryUnsupported {
            node: String::new(),
            artifact: Some(String::new()),
            span: Some(LocatedSpan {
                start: Position {
                    byte: 0,
                    line: 1,
                    column: 1,
                },
                end: Position {
                    byte: 0,
                    line: 1,
                    column: 1,
                },
            }),
            entry: String::new(),
        },
        FrameEntryMalformed => ModelRefusalCause::FrameEntryMalformed {
            node: String::new(),
            artifact: Some(String::new()),
            span: Some(LocatedSpan {
                start: Position {
                    byte: 0,
                    line: 1,
                    column: 1,
                },
                end: Position {
                    byte: 0,
                    line: 1,
                    column: 1,
                },
            }),
            entry: String::new(),
        },
        DigestDomainMismatch => ModelRefusalCause::DigestDomainMismatch {
            expected: crate::model::key::SHA256_JCS_DIGEST_DOMAIN,
            actual: String::new(),
        },
        MissingSelection => ModelRefusalCause::MissingSelection {
            selection: DomainPackageRef::fixture("p"),
        },
        ContentMismatch => ModelRefusalCause::ContentMismatch {
            selected: [0; 32],
            recomputed: [0; 32],
        },
        ByteDigestMismatch => ModelRefusalCause::ByteDigestMismatch {
            expected: [0; 32],
            actual: [0; 32],
        },
        WrongModelSelection => ModelRefusalCause::WrongModelSelection {
            selection: DomainPackageRef::fixture("p"),
            actual_identity: String::new(),
            actual_version: String::new(),
        },
        IntakeLimitExceeded => ModelRefusalCause::IntakeLimitExceeded {
            limit: super::IntakeLimit::InputBytes,
            bound: 0,
            actual: 1,
        },
        AllocationFailed => ModelRefusalCause::AllocationFailed { requested: 0 },
        NoncanonicalNumber => ModelRefusalCause::NoncanonicalNumber {
            inexact: super::Inexact::Number,
            document_pointer: qsl_foundation::diagnostic::JsonPointer::root(),
        },
        ReservedPackageIdentity => ModelRefusalCause::ReservedPackageIdentity {
            selection: DomainPackageRef::fixture("p"),
        },
        DuplicateSelection => ModelRefusalCause::DuplicateSelection {
            identity: String::new(),
            already_selected_version: String::new(),
            requested_version: String::new(),
        },
        DuplicateMember => ModelRefusalCause::DuplicateMember {
            object: String::new(),
            field: key("p"),
        },
        SubsettingViolation => ModelRefusalCause::SubsettingViolation {
            object: String::new(),
            subsetting: key("p"),
            subsetted: key("p"),
        },
        FrameCreateOutsideGrant => ModelRefusalCause::FrameCreateOutsideGrant {
            object: String::new(),
            type_name: key("p"),
        },
        FrameTypeChanged => ModelRefusalCause::FrameTypeChanged {
            object: String::new(),
            pre_type: key("p"),
            post_type: key("p"),
        },
        FrameDeleteOutsideGrant => ModelRefusalCause::FrameDeleteOutsideGrant {
            object: String::new(),
            type_name: key("p"),
        },
        FrameFieldWriteOutsideGrant => ModelRefusalCause::FrameFieldWriteOutsideGrant {
            object: String::new(),
            field: key("p"),
        },
        DuplicateDeclaredIdentity => ModelRefusalCause::DuplicateDeclaredIdentity {
            identity: String::new(),
        },
        DeclaredCreateDeleteOverlap => ModelRefusalCause::DeclaredCreateDeleteOverlap {
            identity: String::new(),
        },
        DeclaredDeltaMismatch => ModelRefusalCause::DeclaredDeltaMismatch {
            declared_created: std::collections::BTreeSet::new(),
            declared_deleted: std::collections::BTreeSet::new(),
            computed_created: std::collections::BTreeSet::new(),
            computed_deleted: std::collections::BTreeSet::new(),
        },
        ]
    }
}

/// The payload rendering of a member identity. A population member's
/// identity is a JSON string, so a well-formed identity is valid UTF-8 and
/// renders as itself. Bytes that are not UTF-8 render as
/// `identity bytes 0x<lowercase hex> (not UTF-8)`: two distinct malformed
/// identities never render alike, and a malformed identity never renders as
/// a lossy decoding that could equal a real member's.
pub fn render_identity(identity: &[u8]) -> String {
    match std::str::from_utf8(identity) {
        Ok(identity) => identity.to_owned(),
        Err(_) => format!(
            "identity bytes 0x{} (not UTF-8)",
            crate::model::key::hex(identity)
        ),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::fixtures::exhaustive_samples;
    use super::ModelRefusalCause;
    use ix_trace_rs::trace;
    use qsl_foundation::diagnostic::CatalogCode;

    /// The spec's spelling parses to the typed path and prints back, and the
    /// reader's document location maps onto the same path.
    #[test]
    fn member_path_spelling_round_trips_and_maps_from_document_locations() {
        for spelling in [
            "TypeDefinition.transitions[0].emits[0]",
            "TypeDefinition.supertypes",
            "constructs[].meaning",
            "Operation.pre",
            "Relationship.target",
            "$.populations",
        ] {
            let parsed: super::MemberPath = spelling.parse().expect("well-formed");
            assert_eq!(parsed.to_string(), spelling);
        }
        for malformed in [
            "Types.x",
            "TypeDefinition.a[x]",
            "TypeDefinition.a[",
            "TypeDefinition.a[]",
            "TypeDefinition.",
            "TypeDefinition..a",
            "TypeDefinition.a]",
            "TypeDefinitionX",
            "TypeDefinition[0].a.",
        ] {
            assert!(
                malformed.parse::<super::MemberPath>().is_err(),
                "{malformed}"
            );
        }
        assert_eq!(
            super::MemberPath::from_document_location("$.types[3].fields[0]"),
            "TypeDefinition.fields[0]".parse().unwrap()
        );
        assert_eq!(
            super::MemberPath::from_document_location("$.types[3].operations[1].pre[0]"),
            "Operation.pre[0]".parse().unwrap()
        );
        assert_eq!(
            super::MemberPath::from_document_location("$.constructs[2]"),
            "constructs[]".parse().unwrap()
        );
    }

    /// #141 review finding 1, #163 re-review fix 4: the wire spellings had
    /// no test naming every variant's exact tag. This match has no
    /// wildcard arm, so a new variant added to [`ModelRefusalCause`]
    /// without a row here fails to compile (E0004), rather than silently
    /// passing an incomplete test.
    ///
    /// The expected strings are the literal `&'static str` cause values
    /// every construction site assigned directly on `origin/main` before
    /// #141's refactor introduced `as_str` at all (#163 re-review fix 5:
    /// not "`as_str`'s match arms on `origin/main`" — there was no
    /// `as_str` before this PR).
    fn expected_tag(cause: &ModelRefusalCause) -> &'static str {
        match cause {
            ModelRefusalCause::FamilySteps { .. } => "family-steps",
            ModelRefusalCause::UnclosedMethodSet => "unclosed-method-set",
            ModelRefusalCause::UnknownOriginal { .. } => "unknown-original",
            ModelRefusalCause::UnknownCandidate { .. } => "unknown-candidate",
            ModelRefusalCause::NoApplicable => "no-applicable",
            ModelRefusalCause::MultipleUndominated => "multiple-undominated",
            ModelRefusalCause::UnsortedView { .. } => "unsorted-view",
            ModelRefusalCause::SpecializationCycle { .. } => "specialization-cycle",
            ModelRefusalCause::UnknownOwner { .. } => "unknown-owner",
            ModelRefusalCause::UnknownGeneral { .. } => "unknown-general",
            ModelRefusalCause::UnknownValueType { .. } => "unknown-value-type",
            ModelRefusalCause::UnknownFieldWrite { .. } => "unknown-field-write",
            ModelRefusalCause::UnknownEffectType { .. } => "unknown-effect-type",
            ModelRefusalCause::UnknownMember { .. } => "unknown-member",
            ModelRefusalCause::DerivationConflict { .. } => "derivation-conflict",
            ModelRefusalCause::UnsuppliedProducerRecord => "unsupplied-producer-record",
            ModelRefusalCause::AncestorSteps { .. } => "ancestor-steps",
            ModelRefusalCause::VarianceResult => "variance-result",
            ModelRefusalCause::MultiplicityNarrowing { .. } => "multiplicity-narrowing",
            ModelRefusalCause::SubsettingType { .. } => "subsetting-type",
            ModelRefusalCause::TypeMismatch => "type-mismatch",
            ModelRefusalCause::VarianceParameter { .. } => "variance-parameter",
            ModelRefusalCause::EffectEscape { .. } => "effect-escape",
            ModelRefusalCause::UnprovedRefinement => "unproved-refinement",
            ModelRefusalCause::RedefinitionTarget { .. } => "redefinition-target",
            ModelRefusalCause::ForeignModelSelection { .. } => "foreign-model-selection",
            ModelRefusalCause::IncompleteScope { .. } => "incomplete-scope",
            ModelRefusalCause::UnclosedSubtypes { .. } => "unclosed-subtypes",
            ModelRefusalCause::ForeignType { .. } => "foreign-type",
            ModelRefusalCause::AbstractInstance { .. } => "abstract-instance",
            ModelRefusalCause::UnknownPopulationMemberType { .. } => "missing-name",
            ModelRefusalCause::ConflictingIdentity { .. } => "conflicting-identity",
            ModelRefusalCause::OperatorIneligible => "operator-ineligible",
            ModelRefusalCause::AboveMaximum { .. } => "above-maximum",
            ModelRefusalCause::ForeignUniverse { .. } => "foreign-universe",
            ModelRefusalCause::AbsentKey { .. } => "absent-key",
            ModelRefusalCause::WrongExport => "wrong-export",
            ModelRefusalCause::ConflictingBinding { .. } => "conflicting-binding",
            ModelRefusalCause::UnknownRelationship { .. } => "unknown-relationship",
            ModelRefusalCause::UnknownSourcePort { .. } => "unknown-source-port",
            ModelRefusalCause::UnknownTargetPort { .. } => "unknown-target-port",
            ModelRefusalCause::PortDirection { .. } => "port-direction",
            ModelRefusalCause::UnknownRedefining { .. } => "unknown-redefining",
            ModelRefusalCause::UnknownRedefined { .. } => "unknown-redefined",
            ModelRefusalCause::UnknownSubsetting { .. } => "unknown-subsetting",
            ModelRefusalCause::UnknownSubsetted { .. } => "unknown-subsetted",
            ModelRefusalCause::UnknownComponent { .. } => "unknown-component",
            ModelRefusalCause::UnknownEndpoint { .. } => "unknown-endpoint",
            ModelRefusalCause::UnsortedDerivation { .. } => "unsorted-derivation",
            ModelRefusalCause::DuplicatePath { .. } => "duplicate-path",
            ModelRefusalCause::MalformedDeclaration
            | ModelRefusalCause::IntakeMalformedDeclaration { .. }
            | ModelRefusalCause::FrameEntryMalformed { .. }
            | ModelRefusalCause::ReservedPackageIdentity { .. } => "malformed-declaration",
            ModelRefusalCause::DuplicateSelection { .. } => "duplicate-identity",
            ModelRefusalCause::UnsupportedDeclarationForm { .. } => "declaration-form",
            ModelRefusalCause::FrameEntryMissing { .. }
            | ModelRefusalCause::IntakeMissingDeclaration { .. } => "missing-name",
            ModelRefusalCause::FrameEntryUnsupported { .. } => "unsupported-feature",
            ModelRefusalCause::DigestDomainMismatch { .. } => "digest-domain-mismatch",
            ModelRefusalCause::MissingSelection { .. } => "missing-selection",
            ModelRefusalCause::ContentMismatch { .. } => "content-mismatch",
            ModelRefusalCause::ByteDigestMismatch { .. } => "byte-digest-mismatch",
            ModelRefusalCause::WrongModelSelection { .. } => "wrong-model-selection",
            ModelRefusalCause::IntakeLimitExceeded { .. } => "intake-limit-exceeded",
            ModelRefusalCause::AllocationFailed { .. } => "allocation-failed",
            ModelRefusalCause::NoncanonicalNumber {
                inexact: super::Inexact::Integer,
                ..
            } => "inexact-integer",
            ModelRefusalCause::NoncanonicalNumber {
                inexact: super::Inexact::Number,
                ..
            } => "inexact-number",
            ModelRefusalCause::DuplicateMember { .. } => "duplicate-member",
            ModelRefusalCause::SubsettingViolation { .. } => "subsetting-violation",
            ModelRefusalCause::FrameCreateOutsideGrant { .. }
            | ModelRefusalCause::FrameTypeChanged { .. }
            | ModelRefusalCause::FrameDeleteOutsideGrant { .. }
            | ModelRefusalCause::FrameFieldWriteOutsideGrant { .. } => "unauthorized-change",
            ModelRefusalCause::DuplicateDeclaredIdentity { .. }
            | ModelRefusalCause::DeclaredCreateDeleteOverlap { .. }
            | ModelRefusalCause::DeclaredDeltaMismatch { .. } => "delta-disagreement",
        }
    }

    /// FR-090-AC-5 (TC-386): each cause's expected catalog code, one
    /// literal arm per variant with no `_` arm, written independently of
    /// `ModelRefusalCause::catalog_code`'s grouped arms. Each code is the
    /// native code every construction site pairs with that cause;
    /// `UnsortedDerivation` and `DuplicatePath` take `UnsortedView`'s.
    fn expected_code(cause: &ModelRefusalCause) -> &'static str {
        match cause {
            ModelRefusalCause::UnknownOriginal { .. } => "dangling_reference",
            ModelRefusalCause::UnknownCandidate { .. } => "dangling_reference",
            ModelRefusalCause::UnknownOwner { .. } => "dangling_reference",
            ModelRefusalCause::UnknownGeneral { .. } => "dangling_reference",
            ModelRefusalCause::UnknownValueType { .. } => "dangling_reference",
            ModelRefusalCause::UnknownFieldWrite { .. } => "dangling_reference",
            ModelRefusalCause::UnknownEffectType { .. } => "dangling_reference",
            ModelRefusalCause::UnknownMember { .. } => "dangling_reference",
            ModelRefusalCause::UnknownRelationship { .. } => "dangling_reference",
            ModelRefusalCause::UnknownSourcePort { .. } => "dangling_reference",
            ModelRefusalCause::UnknownTargetPort { .. } => "dangling_reference",
            ModelRefusalCause::UnknownRedefining { .. } => "dangling_reference",
            ModelRefusalCause::UnknownRedefined { .. } => "dangling_reference",
            ModelRefusalCause::UnknownSubsetting { .. } => "dangling_reference",
            ModelRefusalCause::UnknownSubsetted { .. } => "dangling_reference",
            ModelRefusalCause::UnknownComponent { .. } => "dangling_reference",
            ModelRefusalCause::UnknownEndpoint { .. } => "dangling_reference",
            ModelRefusalCause::UnsortedView { .. } => "invalid_model_binding",
            ModelRefusalCause::UnsortedDerivation { .. } => "invalid_model_binding",
            ModelRefusalCause::DuplicatePath { .. } => "invalid_model_binding",
            ModelRefusalCause::SpecializationCycle { .. } => "invalid_model_binding",
            ModelRefusalCause::DerivationConflict { .. } => "invalid_model_binding",
            ModelRefusalCause::UnsuppliedProducerRecord => "invalid_model_binding",
            ModelRefusalCause::RedefinitionTarget { .. } => "invalid_model_binding",
            ModelRefusalCause::WrongExport => "invalid_model_binding",
            ModelRefusalCause::ConflictingBinding { .. } => "invalid_model_binding",
            ModelRefusalCause::PortDirection { .. } => "invalid_model_binding",
            ModelRefusalCause::MalformedDeclaration => "invalid_model_binding",
            ModelRefusalCause::IntakeMalformedDeclaration { .. } => "invalid_model_binding",
            ModelRefusalCause::FrameEntryMalformed { .. } => "invalid_model_binding",
            ModelRefusalCause::ReservedPackageIdentity { .. } => "invalid_model_binding",
            ModelRefusalCause::WrongModelSelection { .. } => "invalid_model_binding",
            ModelRefusalCause::IntakeLimitExceeded { .. } => "resource_exhausted",
            ModelRefusalCause::AllocationFailed { .. } => "resource_exhausted",
            ModelRefusalCause::NoncanonicalNumber { .. } => "noncanonical_wire",
            ModelRefusalCause::FamilySteps { .. } => "resource_exhausted",
            ModelRefusalCause::AncestorSteps { .. } => "resource_exhausted",
            ModelRefusalCause::UnclosedMethodSet => "incomplete_population",
            ModelRefusalCause::IncompleteScope { .. } => "incomplete_population",
            ModelRefusalCause::UnclosedSubtypes { .. } => "incomplete_population",
            ModelRefusalCause::NoApplicable => "ambiguous_dispatch",
            ModelRefusalCause::MultipleUndominated => "ambiguous_dispatch",
            ModelRefusalCause::VarianceResult => "ill_typed",
            ModelRefusalCause::MultiplicityNarrowing { .. } => "ill_typed",
            ModelRefusalCause::SubsettingType { .. } => "ill_typed",
            ModelRefusalCause::TypeMismatch => "ill_typed",
            ModelRefusalCause::VarianceParameter { .. } => "ill_typed",
            ModelRefusalCause::EffectEscape { .. } => "ill_typed",
            ModelRefusalCause::OperatorIneligible => "ill_typed",
            ModelRefusalCause::UnprovedRefinement => "undefined_expression",
            ModelRefusalCause::ForeignModelSelection { .. } => "foreign_reference",
            ModelRefusalCause::ForeignType { .. } => "foreign_reference",
            ModelRefusalCause::ForeignUniverse { .. } => "foreign_reference",
            ModelRefusalCause::AbstractInstance { .. } => "invalid_runtime_input",
            ModelRefusalCause::ConflictingIdentity { .. } => "invalid_runtime_input",
            ModelRefusalCause::AbsentKey { .. } => "invalid_runtime_input",
            ModelRefusalCause::DuplicateMember { .. } => "invalid_runtime_input",
            ModelRefusalCause::SubsettingViolation { .. } => "invalid_runtime_input",
            ModelRefusalCause::UnknownPopulationMemberType { .. } => "missing_declaration",
            ModelRefusalCause::FrameEntryMissing { .. }
            | ModelRefusalCause::IntakeMissingDeclaration { .. } => "missing_declaration",
            ModelRefusalCause::AboveMaximum { .. } => "cardinality_out_of_bound",
            ModelRefusalCause::UnsupportedDeclarationForm { .. } => "unsupported_construct",
            ModelRefusalCause::FrameEntryUnsupported { .. } => "unknown_required_feature",
            ModelRefusalCause::DigestDomainMismatch { .. } => "stale_dependency",
            ModelRefusalCause::ContentMismatch { .. } => "stale_dependency",
            ModelRefusalCause::ByteDigestMismatch { .. } => "stale_dependency",
            ModelRefusalCause::MissingSelection { .. } => "missing_import",
            ModelRefusalCause::DuplicateSelection { .. } => "duplicate_selection",
            ModelRefusalCause::FrameCreateOutsideGrant { .. } => "frame_violation",
            ModelRefusalCause::FrameTypeChanged { .. } => "frame_violation",
            ModelRefusalCause::FrameDeleteOutsideGrant { .. } => "frame_violation",
            ModelRefusalCause::FrameFieldWriteOutsideGrant { .. } => "frame_violation",
            ModelRefusalCause::DuplicateDeclaredIdentity { .. } => "population_delta_mismatch",
            ModelRefusalCause::DeclaredCreateDeleteOverlap { .. } => "population_delta_mismatch",
            ModelRefusalCause::DeclaredDeltaMismatch { .. } => "population_delta_mismatch",
        }
    }

    #[trace("FR-090-AC-5", "TC-386")]
    #[test]
    fn catalog_code_gives_every_variant_its_code_and_tag() {
        for cause in exhaustive_samples() {
            // QSpec FR-150-AC-11 and FR-272: a walk ceiling is a denied meter
            // charge, so the catalog cause is `insufficient-next-charge`.
            let tag = match cause {
                ModelRefusalCause::AncestorSteps { .. } | ModelRefusalCause::FamilySteps { .. } => {
                    "insufficient-next-charge"
                }
                _ => cause.as_str(),
            };
            assert_eq!(
                cause.catalog_code(),
                CatalogCode::new(expected_code(&cause), tag),
                "{cause:?}"
            );
        }
    }

    /// FR-255 Behavior 1 (QSL-745 (b)) and QSpec FR-150-AC-11: a reached walk
    /// ceiling is a denied one-edge charge past the effective limit; intake's
    /// byte limit is a size ceiling carrying the stage-limit record. The walks
    /// carry `setting`, `limit`, `actual` and the retained charge.
    #[trace("FR-255-AC-1")]
    #[test]
    fn a_reached_model_ceiling_carries_its_caller_budget_record() {
        use super::{DeclarationKey, IntakeLimit};
        let key = DeclarationKey::fixture("model.A");
        let fields = |pairs: &[(&'static str, &str)]| {
            Some(
                pairs
                    .iter()
                    .map(|(name, value)| (*name, (*value).to_owned()))
                    .collect::<std::collections::BTreeMap<_, _>>(),
            )
        };
        let ancestor = ModelRefusalCause::ancestor_steps(
            key.clone(),
            7,
            qsl_foundation::Setting::ModelAncestorSteps,
        );
        assert_eq!(
            ancestor.catalog_code(),
            CatalogCode::new("resource_exhausted", "insufficient-next-charge")
        );
        assert_eq!(
            ancestor.catalog_fields(),
            fields(&[
                ("setting", "model.ancestor_steps"),
                ("limit", "7"),
                ("actual", "8"),
                ("stage", "model-normalization"),
                ("accounting_contract", "ModelNormalizationLimitsV1"),
                ("limit_kind", "ancestor_steps"),
                ("consumed", "7"),
                ("next_charge", "1"),
                ("charge_point", "model.ancestor-edge"),
            ])
        );
        let admission = ModelRefusalCause::ancestor_steps(
            key.clone(),
            7,
            qsl_foundation::Setting::AdmissionAncestorSteps,
        );
        assert_eq!(
            admission.catalog_fields(),
            fields(&[
                ("setting", "admission.ancestor_steps"),
                ("limit", "7"),
                ("actual", "8"),
                ("stage", "population-admission"),
                ("accounting_contract", "PopulationAdmissionLimitsV1"),
                ("limit_kind", "ancestor_steps"),
                ("consumed", "7"),
                ("next_charge", "1"),
                ("charge_point", "admission.ancestor-edge"),
            ])
        );
        assert_eq!(
            ModelRefusalCause::family_steps(key, 9).catalog_fields(),
            fields(&[
                ("setting", "model.family_steps"),
                ("limit", "9"),
                ("actual", "10"),
                ("stage", "model-normalization"),
                ("accounting_contract", "ModelNormalizationLimitsV1"),
                ("limit_kind", "family_steps"),
                ("consumed", "9"),
                ("next_charge", "1"),
                ("charge_point", "model.family-edge"),
            ])
        );
        assert_eq!(
            ModelRefusalCause::IntakeLimitExceeded {
                limit: IntakeLimit::InputBytes,
                bound: 4096,
                actual: 4097,
            }
            .catalog_fields(),
            fields(&[
                ("kind", "input-bytes-exceeded"),
                ("bound", "4096"),
                ("actual", "4097"),
                ("setting", "intake.input_bytes"),
            ])
        );
    }

    #[test]
    fn as_str_covers_every_variant_with_its_original_tag() {
        let cases = exhaustive_samples();
        for cause in &cases {
            let expected = expected_tag(cause);
            assert_eq!(cause.as_str(), expected);
            assert_eq!(cause.to_string(), expected);
        }
    }
}

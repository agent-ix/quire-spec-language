// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`ModelRefusalCause`]: the closed FR-150/151/152/153 cause vocabulary.
//!
//! Lives in its own module, lower-level than [`crate::model::normalize`],
//! because [`crate::model::key`] needs the type too and `key` is itself a
//! dependency of `normalize` — defining the enum in `normalize` made `key`'s
//! import of it a module cycle (#141 finding 4). `normalize` re-exports this
//! type at its own path (`crate::model::normalize::ModelRefusalCause`), so
//! nothing outside this crate's module tree observes a rename.

/// The closed FR-150/151/152/153 cause of a [`crate::model::normalize::ModelRefusal`]
/// (#141 F9a): each variant is one condition a caller must distinguish,
/// checked by the compiler rather than compared by string.
///
/// Shared by every `crate::model` rung that refuses through `ModelRefusal`
/// (`normalize`, `population`, `dispatch`, `conformance`, `systems`) and by
/// [`crate::model::key::EffectiveDeclarationPreimage::validate_derivation`],
/// which refuses a preimage's own well-formedness through the same closed
/// vocabulary rather than a second one-off string pair.
///
/// Variants carry a typed field for every identity/path/name their `detail`
/// text names, wherever every construction site for that variant supplies
/// the same shape of data (#141 F9a, review finding 3). A variant with no
/// natural data, or whose construction sites disagree in shape (a plain
/// count instead of an identity, or a different number of identities), stays
/// a unit variant; `detail`'s free text is still the full account there.
/// [`ModelRefusalCause::as_str`] returns the same tag either way, and no
/// `detail` string changed: this is a structural change only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelRefusalCause {
    /// A dispatch family's redefinition chain exceeded `MAX_DISPATCH_DEPTH`.
    DispatchFamilyDepth {
        /// The dispatch original the family was built from.
        original: String,
    },
    /// An operation family closes over an unresolved or unproved method set.
    UnclosedMethodSet,
    /// A dispatch original names an operation absent from the bundle.
    UnknownOriginal {
        /// The absent original.
        original: String,
    },
    /// A dispatch candidate names an operation absent from the bundle.
    UnknownCandidate {
        /// The absent candidate.
        candidate: String,
    },
    /// No candidate in a dispatch family is applicable to the call.
    NoApplicable,
    /// More than one candidate is applicable and none dominates the others.
    MultipleUndominated,
    /// View declarations are not sorted ascending by effective identity.
    UnsortedView {
        /// The effective identity at the out-of-order position.
        at: String,
    },
    /// An ancestor path exceeded [`crate::model::normalize::MAX_GENERALIZATION_DEPTH`]
    /// generalization records.
    GeneralizationDepthExceeded {
        /// The ancestor path's root.
        root: String,
    },
    /// A type generalizes back to itself through its own ancestor path.
    SpecializationCycle {
        /// The ancestor that closes the cycle.
        ancestor: String,
        /// The generalization record the cycle is discovered via.
        via: String,
    },
    /// A field, operation, redefinition or subsetting record names an owner
    /// that is not a declared object type.
    UnknownOwner {
        /// The record naming the owner.
        member: String,
        /// The absent owner.
        owner: String,
    },
    /// A generalization record names a specific that is not a declared
    /// object type.
    UnknownSpecific {
        /// The generalization record.
        generalization: String,
        /// The absent specific.
        specific: String,
    },
    /// A generalization record names a general that is not a declared
    /// object type.
    UnknownGeneral {
        /// The generalization record.
        generalization: String,
        /// The absent general.
        general: String,
    },
    /// An operation parameter or result names a value type that is not
    /// declared.
    UnknownValueType,
    /// An operation effect writes a field that is not a declared member.
    UnknownFieldWrite {
        /// The operation whose effect writes the field.
        operation: String,
        /// The absent field.
        field: String,
    },
    /// An operation effect names a type that is not a declared object type.
    UnknownEffectType {
        /// The operation whose effect names the type.
        operation: String,
        /// The absent type.
        type_name: String,
    },
    /// A redefinition or subsetting record names a member absent from its
    /// owner.
    UnknownMember {
        /// The redefinition or subsetting record.
        record: String,
        /// The absent member.
        member: String,
    },
    /// A type has two or more undominated redefinitions of one member.
    DerivationConflict,
    /// A redefinition edge's target or redefining member is not an
    /// effective member of its owner.
    RedefinitionUnreachable {
        /// The unreachable member.
        member: String,
        /// The owner it is not an effective member of.
        owner: String,
    },
    /// A bundle record's producer interface is neither normalized nor a
    /// recognized refusal (outside FR-150's supported wire range).
    UnsupportedWire {
        /// The unsupported producer interface version.
        version: String,
    },
    /// A bundle record has no producer revision to select.
    WrongModelSelection {
        /// The record with no producer revision.
        key: String,
    },
    /// A producer key's digest domain does not match
    /// [`crate::model::key::PRODUCER_DIGEST_DOMAIN`].
    DigestDomainMismatch {
        /// The producer key.
        key: String,
        /// Its actual (wrong) digest domain.
        domain: String,
    },
    /// A required item does not supply the producer capability its
    /// interface revision requires.
    UnsuppliedProducerRecord,
    /// A conformance check's generalization walk exceeded
    /// `MAX_CONFORMANCE_DEPTH`.
    ConformanceDepth {
        /// The conformance check's starting specific.
        from: String,
    },
    /// A redefining or subsetting result/effect does not conform to the
    /// redefined/subsetted one under FR-151 variance.
    VarianceResult,
    /// A redefining or subsetting multiplicity does not conform to the
    /// redefined/subsetted one.
    MultiplicityNarrowing,
    /// A subsetting field's value type does not conform to the subsetted
    /// field's value type.
    SubsettingType {
        /// The subsetting field's value type.
        subsetting: String,
        /// The subsetted field's value type.
        subsetted: String,
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
        declared: String,
        /// The redefined parameter's value type.
        redefined: String,
    },
    /// A redefining operation's effect writes a field the redefined
    /// operation's effect does not cover.
    EffectEscape {
        /// The uncovered write.
        field: String,
    },
    /// A redefinition narrows a fact FR-146 has no proof form to establish,
    /// or narrows past an established fact with no supporting proof.
    UnprovedRefinement,
    /// More than one redefinition target candidate remains after dominance.
    RedefinitionTarget,
    /// A population document or effective view names a model selection
    /// other than the admitting bundle's.
    ForeignModelSelection {
        /// The offered model selection.
        actual: String,
        /// The admitting bundle's model selection.
        expected: String,
    },
    /// A population document does not declare `closedWorld: true`.
    IncompleteScope {
        /// The model selection named without `closedWorld: true`.
        selection: String,
    },
    /// A closed-world model selection's generalization graph is not itself
    /// closed.
    UnclosedSubtypes {
        /// The model selection.
        selection: String,
        /// A member's type identity from the population document (or a
        /// fixed placeholder when the document declares no members).
        type_name: String,
    },
    /// A population member names a type absent from the effective view.
    ForeignType {
        /// The member.
        member: String,
        /// The absent type.
        type_name: String,
    },
    /// One object is declared with two conflicting types.
    ConflictingIdentity {
        /// The object.
        object: String,
        /// Its already-recorded type.
        existing_type: String,
        /// The newly declared, conflicting type.
        declared_type: String,
    },
    /// A population binding has no declared maximum to select against.
    OperatorIneligible,
    /// A selected population count exceeds its declared maximum.
    AboveMaximum,
    /// A reference key names a universe other than the binding's.
    ForeignUniverse {
        /// The reference key's universe.
        actual: String,
        /// The binding's universe.
        expected: String,
    },
    /// A reference key is not a member of the bound population.
    AbsentKey {
        /// The absent key.
        key: String,
    },
    /// A bundle record does not export the required [`crate::model::key`]
    /// kind for its role.
    WrongExport,
    /// A systems relationship names an endpoint absent from the bundle.
    UnknownRelationship {
        /// The absent relationship.
        relationship: String,
    },
    /// A connection's source end names a port that is not a declared
    /// endpoint.
    UnknownSourcePort {
        /// The absent source port.
        port: String,
    },
    /// A connection's target end names a port that is not a declared
    /// endpoint.
    UnknownTargetPort {
        /// The absent target port.
        port: String,
    },
    /// A connection's direction is not compatible with its source/target
    /// ports.
    PortDirection {
        /// The source port.
        source: String,
        /// The target port.
        target: String,
    },
    /// A conformance redefinition record names a redefining member absent
    /// from the bundle.
    UnknownRedefining {
        /// The absent redefining member.
        member: String,
    },
    /// A conformance redefinition record names a redefined member absent
    /// from the bundle.
    UnknownRedefined {
        /// The absent redefined member.
        member: String,
    },
    /// A conformance subsetting record names a subsetting member absent
    /// from the bundle.
    UnknownSubsetting {
        /// The absent subsetting member.
        member: String,
    },
    /// A conformance subsetting record names a subsetted member absent
    /// from the bundle.
    UnknownSubsetted {
        /// The absent subsetted member.
        member: String,
    },
    /// An endpoint's owning component names a key absent from the bundle.
    UnknownComponent {
        /// The endpoint naming the owning component.
        item: String,
        /// The absent component.
        missing: String,
    },
    /// A relationship end names a type identity absent from the bundle.
    UnknownEndpoint {
        /// The relationship end.
        item: String,
        /// The absent endpoint.
        missing: String,
    },
    /// An [`crate::model::key::EffectiveDeclarationPreimage`]'s derivation
    /// fact has an `ordinal` that does not match its array position.
    UnsortedDerivation,
    /// An [`crate::model::key::EffectiveDeclarationPreimage`]'s derivation
    /// retains the same input path at two positions.
    DuplicatePath,
}

impl ModelRefusalCause {
    /// The cause tag (the exact spelling every FR-150/151/152/153 test
    /// tracing and every prior wire-visible string used before #141).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DispatchFamilyDepth { .. } => "dispatch-family-depth",
            Self::UnclosedMethodSet => "unclosed-method-set",
            Self::UnknownOriginal { .. } => "unknown-original",
            Self::UnknownCandidate { .. } => "unknown-candidate",
            Self::NoApplicable => "no-applicable",
            Self::MultipleUndominated => "multiple-undominated",
            Self::UnsortedView { .. } => "unsorted-view",
            Self::GeneralizationDepthExceeded { .. } => "generalization-depth-exceeded",
            Self::SpecializationCycle { .. } => "specialization-cycle",
            Self::UnknownOwner { .. } => "unknown-owner",
            Self::UnknownSpecific { .. } => "unknown-specific",
            Self::UnknownGeneral { .. } => "unknown-general",
            Self::UnknownValueType => "unknown-value-type",
            Self::UnknownFieldWrite { .. } => "unknown-field-write",
            Self::UnknownEffectType { .. } => "unknown-effect-type",
            Self::UnknownMember { .. } => "unknown-member",
            Self::DerivationConflict => "derivation-conflict",
            Self::RedefinitionUnreachable { .. } => "redefinition-unreachable",
            Self::UnsupportedWire { .. } => "unsupported-wire",
            Self::WrongModelSelection { .. } => "wrong-model-selection",
            Self::DigestDomainMismatch { .. } => "digest-domain-mismatch",
            Self::UnsuppliedProducerRecord => "unsupplied-producer-record",
            Self::ConformanceDepth { .. } => "conformance-depth",
            Self::VarianceResult => "variance-result",
            Self::MultiplicityNarrowing => "multiplicity-narrowing",
            Self::SubsettingType { .. } => "subsetting-type",
            Self::TypeMismatch => "type-mismatch",
            Self::VarianceParameter { .. } => "variance-parameter",
            Self::EffectEscape { .. } => "effect-escape",
            Self::UnprovedRefinement => "unproved-refinement",
            Self::RedefinitionTarget => "redefinition-target",
            Self::ForeignModelSelection { .. } => "foreign-model-selection",
            Self::IncompleteScope { .. } => "incomplete-scope",
            Self::UnclosedSubtypes { .. } => "unclosed-subtypes",
            Self::ForeignType { .. } => "foreign-type",
            Self::ConflictingIdentity { .. } => "conflicting-identity",
            Self::OperatorIneligible => "operator-ineligible",
            Self::AboveMaximum => "above-maximum",
            Self::ForeignUniverse { .. } => "foreign-universe",
            Self::AbsentKey { .. } => "absent-key",
            Self::WrongExport => "wrong-export",
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
            Self::UnsortedDerivation => "unsorted-derivation",
            Self::DuplicatePath => "duplicate-path",
        }
    }
}

impl std::fmt::Display for ModelRefusalCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::ModelRefusalCause;

    /// #141 review finding 1: the wire spellings had no test naming every
    /// variant's exact tag. These are the literal strings that shipped
    /// before #141's refactor (from `as_str`'s match arms on `origin/main`),
    /// asserted here so a future edit cannot silently change a wire-visible
    /// tag. Field values are placeholders: `as_str`/`Display` depend only on
    /// the discriminant.
    #[test]
    fn as_str_covers_every_variant_with_its_original_tag() {
        let cases: Vec<(ModelRefusalCause, &str)> = vec![
            (
                ModelRefusalCause::DispatchFamilyDepth {
                    original: String::new(),
                },
                "dispatch-family-depth",
            ),
            (ModelRefusalCause::UnclosedMethodSet, "unclosed-method-set"),
            (
                ModelRefusalCause::UnknownOriginal {
                    original: String::new(),
                },
                "unknown-original",
            ),
            (
                ModelRefusalCause::UnknownCandidate {
                    candidate: String::new(),
                },
                "unknown-candidate",
            ),
            (ModelRefusalCause::NoApplicable, "no-applicable"),
            (
                ModelRefusalCause::MultipleUndominated,
                "multiple-undominated",
            ),
            (
                ModelRefusalCause::UnsortedView { at: String::new() },
                "unsorted-view",
            ),
            (
                ModelRefusalCause::GeneralizationDepthExceeded {
                    root: String::new(),
                },
                "generalization-depth-exceeded",
            ),
            (
                ModelRefusalCause::SpecializationCycle {
                    ancestor: String::new(),
                    via: String::new(),
                },
                "specialization-cycle",
            ),
            (
                ModelRefusalCause::UnknownOwner {
                    member: String::new(),
                    owner: String::new(),
                },
                "unknown-owner",
            ),
            (
                ModelRefusalCause::UnknownSpecific {
                    generalization: String::new(),
                    specific: String::new(),
                },
                "unknown-specific",
            ),
            (
                ModelRefusalCause::UnknownGeneral {
                    generalization: String::new(),
                    general: String::new(),
                },
                "unknown-general",
            ),
            (ModelRefusalCause::UnknownValueType, "unknown-value-type"),
            (
                ModelRefusalCause::UnknownFieldWrite {
                    operation: String::new(),
                    field: String::new(),
                },
                "unknown-field-write",
            ),
            (
                ModelRefusalCause::UnknownEffectType {
                    operation: String::new(),
                    type_name: String::new(),
                },
                "unknown-effect-type",
            ),
            (
                ModelRefusalCause::UnknownMember {
                    record: String::new(),
                    member: String::new(),
                },
                "unknown-member",
            ),
            (ModelRefusalCause::DerivationConflict, "derivation-conflict"),
            (
                ModelRefusalCause::RedefinitionUnreachable {
                    member: String::new(),
                    owner: String::new(),
                },
                "redefinition-unreachable",
            ),
            (
                ModelRefusalCause::UnsupportedWire {
                    version: String::new(),
                },
                "unsupported-wire",
            ),
            (
                ModelRefusalCause::WrongModelSelection { key: String::new() },
                "wrong-model-selection",
            ),
            (
                ModelRefusalCause::DigestDomainMismatch {
                    key: String::new(),
                    domain: String::new(),
                },
                "digest-domain-mismatch",
            ),
            (
                ModelRefusalCause::UnsuppliedProducerRecord,
                "unsupplied-producer-record",
            ),
            (
                ModelRefusalCause::ConformanceDepth {
                    from: String::new(),
                },
                "conformance-depth",
            ),
            (ModelRefusalCause::VarianceResult, "variance-result"),
            (
                ModelRefusalCause::MultiplicityNarrowing,
                "multiplicity-narrowing",
            ),
            (
                ModelRefusalCause::SubsettingType {
                    subsetting: String::new(),
                    subsetted: String::new(),
                },
                "subsetting-type",
            ),
            (ModelRefusalCause::TypeMismatch, "type-mismatch"),
            (
                ModelRefusalCause::VarianceParameter {
                    index: 0,
                    declared: String::new(),
                    redefined: String::new(),
                },
                "variance-parameter",
            ),
            (
                ModelRefusalCause::EffectEscape {
                    field: String::new(),
                },
                "effect-escape",
            ),
            (ModelRefusalCause::UnprovedRefinement, "unproved-refinement"),
            (ModelRefusalCause::RedefinitionTarget, "redefinition-target"),
            (
                ModelRefusalCause::ForeignModelSelection {
                    actual: String::new(),
                    expected: String::new(),
                },
                "foreign-model-selection",
            ),
            (
                ModelRefusalCause::IncompleteScope {
                    selection: String::new(),
                },
                "incomplete-scope",
            ),
            (
                ModelRefusalCause::UnclosedSubtypes {
                    selection: String::new(),
                    type_name: String::new(),
                },
                "unclosed-subtypes",
            ),
            (
                ModelRefusalCause::ForeignType {
                    member: String::new(),
                    type_name: String::new(),
                },
                "foreign-type",
            ),
            (
                ModelRefusalCause::ConflictingIdentity {
                    object: String::new(),
                    existing_type: String::new(),
                    declared_type: String::new(),
                },
                "conflicting-identity",
            ),
            (ModelRefusalCause::OperatorIneligible, "operator-ineligible"),
            (ModelRefusalCause::AboveMaximum, "above-maximum"),
            (
                ModelRefusalCause::ForeignUniverse {
                    actual: String::new(),
                    expected: String::new(),
                },
                "foreign-universe",
            ),
            (
                ModelRefusalCause::AbsentKey { key: String::new() },
                "absent-key",
            ),
            (ModelRefusalCause::WrongExport, "wrong-export"),
            (
                ModelRefusalCause::UnknownRelationship {
                    relationship: String::new(),
                },
                "unknown-relationship",
            ),
            (
                ModelRefusalCause::UnknownSourcePort {
                    port: String::new(),
                },
                "unknown-source-port",
            ),
            (
                ModelRefusalCause::UnknownTargetPort {
                    port: String::new(),
                },
                "unknown-target-port",
            ),
            (
                ModelRefusalCause::PortDirection {
                    source: String::new(),
                    target: String::new(),
                },
                "port-direction",
            ),
            (
                ModelRefusalCause::UnknownRedefining {
                    member: String::new(),
                },
                "unknown-redefining",
            ),
            (
                ModelRefusalCause::UnknownRedefined {
                    member: String::new(),
                },
                "unknown-redefined",
            ),
            (
                ModelRefusalCause::UnknownSubsetting {
                    member: String::new(),
                },
                "unknown-subsetting",
            ),
            (
                ModelRefusalCause::UnknownSubsetted {
                    member: String::new(),
                },
                "unknown-subsetted",
            ),
            (
                ModelRefusalCause::UnknownComponent {
                    item: String::new(),
                    missing: String::new(),
                },
                "unknown-component",
            ),
            (
                ModelRefusalCause::UnknownEndpoint {
                    item: String::new(),
                    missing: String::new(),
                },
                "unknown-endpoint",
            ),
            (ModelRefusalCause::UnsortedDerivation, "unsorted-derivation"),
            (ModelRefusalCause::DuplicatePath, "duplicate-path"),
        ];
        assert_eq!(
            cases.len(),
            53,
            "expected exactly the 53 ModelRefusalCause variants"
        );
        for (cause, expected) in &cases {
            assert_eq!(cause.as_str(), *expected);
            assert_eq!(cause.to_string(), *expected);
        }
    }
}

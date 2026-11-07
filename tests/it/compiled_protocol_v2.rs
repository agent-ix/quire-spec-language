// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-138: the public compiled-protocol refusal codes are injective. The strict
//! readers' refusals themselves are asserted in `protocol_artifact_versions.rs`.

use ix_trace_rs::trace;
use quire_spec_language::number::{NumberComponent, NumberError};
use quire_spec_language::protocol_artifact::{
    self as artifact, v2, Dimension as WorkDimension, Error, Invalid, Unsupported,
};

#[trace("TC-138", "FR-050-AC-2", "FR-050-AC-3")]
#[test]
fn public_v2_refusal_codes_are_injective_across_every_declared_axis() {
    let mut refusals = Vec::new();
    refusals.extend(
        [
            v2::HeaderField::Wire,
            v2::HeaderField::Media,
            v2::HeaderField::Schema,
            v2::HeaderField::PackageType,
            v2::HeaderField::Encoding,
            v2::HeaderField::Numeric,
            v2::HeaderField::ArtifactKind,
            v2::HeaderField::ArtifactWire,
            v2::HeaderField::ArtifactVersion,
        ]
        .map(v2::Refusal::Header),
    );
    for side in [
        v2::InventorySide::Offer,
        v2::InventorySide::Expected,
        v2::InventorySide::Producer,
    ] {
        for cause in [
            v2::BindingCause::Missing,
            v2::BindingCause::Surplus,
            v2::BindingCause::Duplicate,
        ] {
            refusals.push(v2::Refusal::Binding { side, cause });
        }
    }
    refusals.extend([
        v2::Refusal::OfferOrder,
        v2::Refusal::ForeignOwner(v2::SelectionSide::Expected),
        v2::Refusal::ForeignOwner(v2::SelectionSide::Producer),
        v2::Refusal::OfferIndex(v2::BindingIndex::Declaration),
        v2::Refusal::OfferIndex(v2::BindingIndex::Definition),
    ]);
    refusals.extend(
        [
            v2::DeclarationField::Name,
            v2::DeclarationField::Requirement,
            v2::DeclarationField::Clause,
            v2::DeclarationField::Execution,
        ]
        .map(v2::Refusal::Declaration),
    );
    refusals.extend([
        v2::Refusal::Definition(v2::DefinitionField::Identity),
        v2::Refusal::Definition(v2::DefinitionField::Revision),
    ]);
    refusals.extend(
        [
            v2::ArtifactField::RefVersion,
            v2::ArtifactField::Kind,
            v2::ArtifactField::Authority,
            v2::ArtifactField::Identity,
            v2::ArtifactField::WireIdentity,
            v2::ArtifactField::WireVersion,
        ]
        .map(|field| v2::Refusal::Definition(v2::DefinitionField::Artifact(field))),
    );
    refusals.extend(
        [
            v2::ClockField::Alternative,
            v2::ClockField::SequenceAuthority,
            v2::ClockField::Epoch,
            v2::ClockField::Period,
            v2::ClockField::Unit,
            v2::ClockField::TimestampUnit,
        ]
        .map(v2::Refusal::Clock),
    );

    let codes = refusals
        .iter()
        .map(|refusal| refusal.code())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(codes.len(), refusals.len());
}

/// Tracing: TC-121, TC-138.
#[trace("TC-121", "TC-138", "FR-042-AC-7", "FR-050-AC-2")]
#[test]
fn public_error_codes_are_injective_across_every_declared_axis() {
    let mut errors = vec![Error::Allocation, Error::Json { line: 1, column: 1 }];
    errors.extend(
        [
            NumberError::NonCanonicalDecimal {
                component: NumberComponent::Decimal,
            },
            NumberError::ComponentOutOfRange {
                component: NumberComponent::Decimal,
            },
            NumberError::NonPositiveDenominator,
            NumberError::UnreducedRational,
        ]
        .map(Error::Numeric),
    );
    errors.extend(
        [
            Invalid::Selection,
            Invalid::Name,
            Invalid::StructuralInteger,
            Invalid::WrongNumericKind,
            Invalid::NumericDomain,
            Invalid::Inventory,
            Invalid::Order,
            Invalid::Duplicate,
            Invalid::Dependency,
            Invalid::Definition,
            Invalid::Model,
            Invalid::ForeignLocus,
            Invalid::Locus,
            Invalid::Reference,
            Invalid::Owner,
            Invalid::Scope,
            Invalid::Type,
            Invalid::Profile,
            Invalid::Call,
            Invalid::Binding,
            Invalid::Control,
            Invalid::Cycle,
            Invalid::Feature,
            Invalid::Canonical,
            Invalid::Encoding,
        ]
        .map(Error::Invalid),
    );
    errors.extend(
        [
            Unsupported::Wire,
            Unsupported::Feature,
            Unsupported::Definition,
            Unsupported::Profile,
            Unsupported::FamilyProof,
            Unsupported::Export,
        ]
        .map(Error::Unsupported),
    );
    errors.extend(WorkDimension::ALL.map(|dimension| {
        Error::Incomplete(artifact::Exhaustion {
            dimension,
            used: 0,
            requested: 1,
            limit: 0,
            locus: None,
        })
    }));
    errors.push(Error::V2(v2::Refusal::Header(v2::HeaderField::Wire)));

    let codes = errors
        .iter()
        .map(Error::code)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(codes.len(), errors.len());

    // The two identities the downstream mutation corpus cannot express through
    // `v2::Refusal::code` alone.
    assert_eq!(Error::Json { line: 3, column: 9 }.code(), "json");
    // A version-2 refusal keeps its own established spelling.
    assert_eq!(
        Error::V2(v2::Refusal::Header(v2::HeaderField::Wire)).code(),
        v2::Refusal::Header(v2::HeaderField::Wire).code()
    );
}

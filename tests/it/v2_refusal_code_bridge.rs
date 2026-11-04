// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-050-AC-2/3: legacy version 2 refusals delegate to the spine vocabulary.

use ix_trace_rs::trace;
use qsl_package::protocol_artifact_v2::RefusalCode as SpineCode;
use quire_spec_language::protocol_artifact::v2::{
    BindingCause, BindingIndex, HeaderField, InventorySide, Refusal, SelectionSide,
};

#[trace("TC-1308", "FR-050-AC-2", "FR-050-AC-3", "FR-513-AC-17")]
#[test]
fn every_legacy_header_and_binding_code_delegates_to_the_spine_table() {
    use BindingCause as Cause;
    use HeaderField as Header;
    use InventorySide as Side;
    use SelectionSide as Selection;

    let cases = [
        (Refusal::Header(Header::Wire), SpineCode::HeaderWire),
        (Refusal::Header(Header::Media), SpineCode::HeaderMedia),
        (Refusal::Header(Header::Schema), SpineCode::HeaderSchema),
        (
            Refusal::Header(Header::PackageType),
            SpineCode::HeaderPackageType,
        ),
        (Refusal::Header(Header::Encoding), SpineCode::HeaderEncoding),
        (Refusal::Header(Header::Numeric), SpineCode::HeaderNumeric),
        (
            Refusal::Header(Header::ArtifactKind),
            SpineCode::HeaderArtifactKind,
        ),
        (
            Refusal::Header(Header::ArtifactWire),
            SpineCode::HeaderArtifactWire,
        ),
        (
            Refusal::Header(Header::ArtifactVersion),
            SpineCode::HeaderArtifactVersion,
        ),
        (
            Refusal::Binding {
                side: Side::Offer,
                cause: Cause::Missing,
            },
            SpineCode::BindingOfferMissing,
        ),
        (
            Refusal::Binding {
                side: Side::Offer,
                cause: Cause::Surplus,
            },
            SpineCode::BindingOfferSurplus,
        ),
        (
            Refusal::Binding {
                side: Side::Expected,
                cause: Cause::Missing,
            },
            SpineCode::BindingExpectedMissing,
        ),
        (
            Refusal::Binding {
                side: Side::Expected,
                cause: Cause::Surplus,
            },
            SpineCode::BindingExpectedSurplus,
        ),
        (
            Refusal::Binding {
                side: Side::Producer,
                cause: Cause::Missing,
            },
            SpineCode::BindingProducerMissing,
        ),
        (
            Refusal::Binding {
                side: Side::Producer,
                cause: Cause::Surplus,
            },
            SpineCode::BindingProducerSurplus,
        ),
        (
            Refusal::Binding {
                side: Side::Offer,
                cause: Cause::Duplicate,
            },
            SpineCode::BindingOfferDuplicate,
        ),
        (
            Refusal::Binding {
                side: Side::Expected,
                cause: Cause::Duplicate,
            },
            SpineCode::BindingExpectedDuplicate,
        ),
        (
            Refusal::Binding {
                side: Side::Producer,
                cause: Cause::Duplicate,
            },
            SpineCode::BindingProducerDuplicate,
        ),
        (Refusal::ClockName, SpineCode::BindingOfferClockName),
        (Refusal::OfferOrder, SpineCode::BindingOfferOrder),
        (
            Refusal::ForeignOwner(Selection::Expected),
            SpineCode::BindingExpectedForeignOwner,
        ),
        (
            Refusal::ForeignOwner(Selection::Producer),
            SpineCode::BindingProducerForeignOwner,
        ),
        (
            Refusal::OfferIndex(BindingIndex::Declaration),
            SpineCode::BindingOfferDeclarationIndex,
        ),
        (
            Refusal::OfferIndex(BindingIndex::Definition),
            SpineCode::BindingOfferDefinitionIndex,
        ),
    ];

    for (legacy, spine) in cases {
        assert_eq!(legacy.code(), spine.as_str());
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-050-AC-2/3, FR-513-AC-17: the complete version 2 refusal-code table.
#![cfg(feature = "std")]

use ix_trace_rs::trace;
use qsl_package::protocol_artifact_v2::RefusalCode;

#[trace("TC-1308", "FR-050-AC-2", "FR-050-AC-3", "FR-513-AC-17")]
#[test]
fn v2_header_and_binding_codes_match_the_fixed_vocabulary() {
    let codes = [
        (RefusalCode::HeaderWire, "v2.header.wire"),
        (RefusalCode::HeaderMedia, "v2.header.media"),
        (RefusalCode::HeaderSchema, "v2.header.schema"),
        (RefusalCode::HeaderPackageType, "v2.header.package-type"),
        (RefusalCode::HeaderEncoding, "v2.header.encoding"),
        (RefusalCode::HeaderNumeric, "v2.header.numeric"),
        (RefusalCode::HeaderArtifactKind, "v2.header.artifact-kind"),
        (RefusalCode::HeaderArtifactWire, "v2.header.artifact-wire"),
        (
            RefusalCode::HeaderArtifactVersion,
            "v2.header.artifact-version",
        ),
        (RefusalCode::BindingOfferMissing, "v2.binding.offer-missing"),
        (RefusalCode::BindingOfferSurplus, "v2.binding.offer-surplus"),
        (
            RefusalCode::BindingExpectedMissing,
            "v2.binding.expected-missing",
        ),
        (
            RefusalCode::BindingExpectedSurplus,
            "v2.binding.expected-surplus",
        ),
        (
            RefusalCode::BindingProducerMissing,
            "v2.binding.producer-missing",
        ),
        (
            RefusalCode::BindingProducerSurplus,
            "v2.binding.producer-surplus",
        ),
        (
            RefusalCode::BindingOfferDuplicate,
            "v2.binding.offer-duplicate",
        ),
        (
            RefusalCode::BindingExpectedDuplicate,
            "v2.binding.expected-duplicate",
        ),
        (
            RefusalCode::BindingProducerDuplicate,
            "v2.binding.producer-duplicate",
        ),
        (
            RefusalCode::BindingOfferClockName,
            "v2.binding.offer-clock-name",
        ),
        (RefusalCode::BindingOfferOrder, "v2.binding.offer-order"),
        (
            RefusalCode::BindingExpectedForeignOwner,
            "v2.binding.expected-foreign-owner",
        ),
        (
            RefusalCode::BindingProducerForeignOwner,
            "v2.binding.producer-foreign-owner",
        ),
        (
            RefusalCode::BindingOfferDeclarationIndex,
            "v2.binding.offer-declaration-index",
        ),
        (
            RefusalCode::BindingOfferDefinitionIndex,
            "v2.binding.offer-definition-index",
        ),
    ];

    for (code, expected) in codes {
        assert_eq!(code.as_str(), expected);
    }
}

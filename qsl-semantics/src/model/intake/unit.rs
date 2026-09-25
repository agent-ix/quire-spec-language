// SPDX-License-Identifier: AGPL-3.0-or-later
//! I1 for one source unit (ADR-011 §2.1 I1, FR-056): the unit's `model`
//! declarations, each admitted under FR-154 against the package input, read
//! into domain package records and normalized into the effective view E3
//! resolves the unit's model type names against.
//!
//! A declaration selecting a domain package spells its digest
//! `sha256-jcs:<hex>`. A `sha256:<hex>` digest selects a compiled-model
//! artifact, which has no admitted form on this path (FR-056-CON-4), so it
//! refuses at its declaration.

use std::collections::BTreeMap;

use qsl_foundation::selection::{ModelDigest, ModelSelection};
use qsl_foundation::{Code, Span};

use super::{admit_located, read_records, PackageDocument};
use crate::model::accounting::{Incomplete, ModelNormalizationLimits};
use crate::model::domain_package::{DomainPackage, DomainPackageRef};
use crate::model::key::{raw_bytes_digest, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{normalize, EffectiveView, NormalizeOutcome, Refusals};

/// One `model` declaration of a unit, admitted at I1: its alias, the span
/// of its declaration and its domain package's effective view.
#[derive(Clone, Debug)]
pub struct SelectedModel {
    /// The declaration's alias, the `M` of `M::T`.
    pub alias: String,
    /// The span of the `model` declaration.
    pub span: Span,
    /// The admitted domain package's effective view.
    pub view: EffectiveView,
}

/// Why I1 admitted no domain package for one `model` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnitIntakeCause {
    /// The declaration selects a compiled-model artifact by its raw-byte
    /// `sha256:` digest, not a domain package by its `sha256-jcs:` one.
    ArtifactDigest,
    /// FR-154 admission, the record reader or normalization refused, with
    /// every refusal the refusing step reports.
    Refused(Refusals),
    /// Normalization reached a `ModelNormalizationLimitsV1` ceiling.
    Limit(Incomplete),
    /// The record reader refused with no refusal: a broken invariant of
    /// intake, never a property of the package.
    Invariant,
}

impl UnitIntakeCause {
    /// This cause's catalog code: the first model refusal's own, and
    /// `stage_limit_exceeded` for a normalization ceiling.
    pub fn code(&self) -> Code {
        match self {
            Self::ArtifactDigest => Code::InvalidModelBinding,
            Self::Refused(refusals) => refusals[0].code,
            Self::Limit(_) => Code::StageLimitExceeded,
            Self::Invariant => Code::RuntimeInvariant,
        }
    }
}

/// I1's refusal: the `model` declaration it concerns and why.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitIntakeRefusal {
    /// The declaration's alias.
    pub alias: String,
    /// The span of the `model` declaration.
    pub span: Span,
    /// The typed cause.
    pub cause: UnitIntakeCause,
}

/// The package input FR-056 names: each supplied domain package document
/// keyed by its `sha256-jcs` digest. A document that does not parse is keyed
/// by the SHA-256 of its raw bytes, the digest FR-154 check 3 takes of such
/// bytes, so a selection of it refuses there rather than as a missing
/// package. Each document is parsed here to key it and again when a
/// selection admits it.
pub fn package_input<'a>(
    documents: impl IntoIterator<Item = &'a [u8]>,
) -> BTreeMap<[u8; 32], Vec<u8>> {
    documents
        .into_iter()
        .map(|bytes| {
            let digest = PackageDocument::parse(bytes)
                .map_or_else(|_| raw_bytes_digest(bytes), |document| document.jcs_digest);
            (digest, bytes.to_vec())
        })
        .collect()
}

/// Admit each of a unit's `model` declarations, in source order, against
/// `packages` (see [`package_input`]): FR-154 admission (one version of
/// each identity), [`read_records`], then normalization under `limits`. It
/// stops at the first declaration that does not admit.
pub fn admit_unit(
    selections: &[ModelSelection],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
) -> Result<Vec<SelectedModel>, UnitIntakeRefusal> {
    let refuse = |selection: &ModelSelection, cause| UnitIntakeRefusal {
        alias: selection.alias.clone(),
        span: selection.span,
        cause,
    };
    let mut offered = Vec::with_capacity(selections.len());
    for selection in selections {
        let ModelDigest::DomainPackage(digest) = selection.model.digest() else {
            return Err(refuse(selection, UnitIntakeCause::ArtifactDigest));
        };
        offered.push((
            selection,
            DomainPackageRef {
                identity: selection.model.identity().to_owned(),
                version: selection.model.version().to_owned(),
                digest: digest.as_bytes(),
            },
        ));
    }
    let admitted = admit_located(
        &offered,
        |(_, offered_ref)| offered_ref,
        SHA256_JCS_DIGEST_DOMAIN,
        packages,
    )
    .map_err(|((selection, _), refusal)| {
        refuse(
            selection,
            UnitIntakeCause::Refused(Refusals::new(refusal, Vec::new())),
        )
    })?;
    admitted
        .into_iter()
        .map(|((selection, _), package_ref, document)| {
            let records = read_records(&package_ref.identity, &document).map_err(|refusals| {
                let cause = Refusals::try_from(refusals)
                    .map_or(UnitIntakeCause::Invariant, UnitIntakeCause::Refused);
                refuse(selection, cause)
            })?;
            let view = match normalize(&DomainPackage::new(package_ref, records), limits) {
                NormalizeOutcome::Completed(view) => view,
                NormalizeOutcome::Refused(refusals) => {
                    return Err(refuse(selection, UnitIntakeCause::Refused(refusals)))
                }
                NormalizeOutcome::Incomplete(incomplete) => {
                    return Err(refuse(selection, UnitIntakeCause::Limit(incomplete)))
                }
            };
            Ok(SelectedModel {
                alias: selection.alias.clone(),
                span: selection.span,
                view,
            })
        })
        .collect()
}

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
use qsl_foundation::{Code, IntakeLimits, Span};
use quire_exact::Cancel;

use super::{admit_located, read_domain_package, PackageDocument};
use crate::model::accounting::{Incomplete, ModelNormalizationLimits};
use crate::model::domain_package::{DomainPackageRecord, DomainPackageRef, ParsedOperationClause};
use crate::model::key::DeclarationKey;
use crate::model::key::{raw_bytes_digest, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{normalize_with_cancel, EffectiveView, NormalizeOutcome, Refusals};
use crate::model::refusal::{MemberPath, MemberRoot};
use qsl_forms::Expression;
use quire_semantic_value::location::OperationClauseKind;

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
    /// Each kept `requires`/`ensures` clause of the package's operations,
    /// parsed by the compile driver (FR-103). I1 parses none, so admission
    /// leaves this empty.
    pub operation_clauses: Vec<ParsedOperationClause>,
}

/// A kept clause the caller's parser refused: which clause, and the parser's
/// own refusal.
#[derive(Clone, Debug)]
pub struct OperationClauseFailure<E> {
    /// The operation whose clause it is.
    pub operation: DeclarationKey,
    /// The `Operation.pre[k]` or `Operation.post[k]` member the clause is at.
    pub member: MemberPath,
    /// The parser's refusal.
    pub cause: E,
}

impl SelectedModel {
    /// FR-103: this model with each kept `quire`-language `requires` and
    /// `ensures` clause of its operations parsed by `parse`, the caller's
    /// FR-146 expression production. Intake keeps the text and parses none
    /// of it, so the semantics layer takes no dependency on S1: the compile
    /// driver passes its parser here. A clause in another language is kept
    /// verbatim and not parsed. The first clause `parse` refuses stops the
    /// walk.
    pub fn with_parsed_operation_clauses<E>(
        mut self,
        mut parse: impl FnMut(&str) -> Result<Expression, E>,
    ) -> Result<Self, OperationClauseFailure<E>> {
        let mut parsed = Vec::new();
        for record in &self.view.domain_package().records {
            let DomainPackageRecord::OperationMember(operation) = record else {
                continue;
            };
            for (kind, member, clauses) in [
                (OperationClauseKind::Requires, "pre", &operation.requires),
                (OperationClauseKind::Ensures, "post", &operation.ensures),
            ] {
                for clause in clauses.iter().filter(|clause| clause.language == "quire") {
                    let expression =
                        parse(&clause.text).map_err(|cause| OperationClauseFailure {
                            operation: operation.key.clone(),
                            member: MemberPath::new(MemberRoot::Operation)
                                .name(member)
                                .index(clause.position),
                            cause,
                        })?;
                    parsed.push(ParsedOperationClause {
                        operation: operation.key.clone(),
                        kind,
                        position: clause.position,
                        expression,
                    });
                }
            }
        }
        self.operation_clauses = parsed;
        Ok(self)
    }
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

/// No byte limit: how [`package_input`] reads a document to key it.
const UNBOUNDED: IntakeLimits = IntakeLimits {
    input_bytes: u64::MAX,
};

/// The package input FR-056 names: each supplied domain package document
/// keyed by its `sha256-jcs` digest. A document that does not parse is keyed
/// by the SHA-256 of its raw bytes, the digest FR-154 check 3 takes of such
/// bytes, so a selection of it refuses at admission rather than as a
/// missing package. Each document is parsed here to key it, under no byte
/// limit, and again when a selection admits it under the caller's
/// `intake.input_bytes`, so a document over that limit refuses naming it
/// and is never mistaken for a missing package.
pub fn package_input<'a>(
    documents: impl IntoIterator<Item = &'a [u8]>,
) -> BTreeMap<[u8; 32], Vec<u8>> {
    documents
        .into_iter()
        .map(|bytes| {
            let digest = PackageDocument::parse(bytes, UNBOUNDED)
                .map_or_else(|_| raw_bytes_digest(bytes), |document| document.jcs_digest);
            (digest, bytes.to_vec())
        })
        .collect()
}

/// Admit each of a unit's `model` declarations, in source order, against
/// `packages` (see [`package_input`]): FR-154 admission (one version of
/// each identity), [`crate::model::intake::read_records`], then normalization under `limits`. It
/// stops at the first declaration that does not admit.
pub fn admit_unit(
    selections: &[ModelSelection],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
) -> Result<Vec<SelectedModel>, UnitIntakeRefusal> {
    admit_unit_with_cancel(selections, packages, limits, &Cancel::new())
}

/// [`admit_unit`] under the caller's [`Cancel`] handle, polled at every
/// normalization charge (FR-276).
pub fn admit_unit_with_cancel(
    selections: &[ModelSelection],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
    cancel: &Cancel,
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
        limits.intake,
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
            let package = read_domain_package(package_ref, &document).map_err(|refusals| {
                let cause = Refusals::try_from(refusals)
                    .map_or(UnitIntakeCause::Invariant, UnitIntakeCause::Refused);
                refuse(selection, cause)
            })?;
            let view = match normalize_with_cancel(&package, limits, cancel) {
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
                operation_clauses: Vec::new(),
            })
        })
        .collect()
}

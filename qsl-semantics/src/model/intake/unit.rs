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

use qsl_foundation::diagnostic::BudgetDenial;
use qsl_foundation::selection::{ModelDigest, ModelSelection};
use qsl_foundation::{Code, IntakeLimits, Span};
use quire_exact::Cancel;
use quire_semantic_value::catalog::CatalogCode;

use super::{admit_located, read_domain_package, PackageDocument};
use crate::model::accounting::{Incomplete, Meter, ModelNormalizationLimits};
#[cfg(any(test, feature = "test-support"))]
use crate::model::domain_package::DomainPackage;
use crate::model::domain_package::{
    DomainPackageRecord, DomainPackageRef, ParsedOperationClause, ParsedTypeInvariant,
};
use crate::model::key::DeclarationKey;
use crate::model::key::{raw_bytes_digest, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{normalize_on, EffectiveView, NormalizeOutcome, Refusals};
use crate::model::refusal::{MemberPath, MemberRoot};
use crate::model::type_invariant::ClauseRole;
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
    /// Each `Invariant` clause of the package's types, parsed by the
    /// compile driver (FR-366). I1 parses none, so admission leaves this
    /// empty.
    pub type_invariants: Vec<ParsedTypeInvariant>,
}

/// One model-admission operation: the selected models of a unit and the one
/// meter every selection was normalized on. Only admission constructs it,
/// assembly consumes it and carries the meter into checking, so the
/// conformance stage continues the work normalization counted.
#[derive(Clone, Debug)]
pub struct SelectedModels {
    entries: Vec<SelectedModel>,
    meter: Meter,
}

impl Default for SelectedModels {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            meter: Meter::new(ModelNormalizationLimits::default()),
        }
    }
}

impl std::ops::Deref for SelectedModels {
    type Target = [SelectedModel];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

/// Hand-built models whose admission is not under test carry no counted work.
#[cfg(any(test, feature = "test-support"))]
impl From<Vec<SelectedModel>> for SelectedModels {
    fn from(entries: Vec<SelectedModel>) -> Self {
        Self::fixture_entries(entries)
    }
}

impl SelectedModels {
    pub(crate) fn into_parts(self) -> (Vec<SelectedModel>, Meter) {
        (self.entries, self.meter)
    }

    /// Each selected model parsed by `parse` (see
    /// [`SelectedModel::with_parsed_operation_clauses`]); the meter is kept.
    pub fn with_parsed_operation_clauses<E>(
        self,
        mut parse: impl FnMut(&str) -> Result<Expression, E>,
    ) -> Result<Self, OperationClauseFailure<E>> {
        let Self { entries, meter } = self;
        let entries = entries
            .into_iter()
            .map(|entry| entry.with_parsed_operation_clauses(&mut parse))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { entries, meter })
    }

    /// Each selected model parsed by `parse` (see
    /// [`SelectedModel::with_parsed_type_invariants`]); the meter is kept.
    pub fn with_parsed_type_invariants<E>(
        self,
        mut parse: impl FnMut(&str) -> Result<Expression, E>,
    ) -> Result<Self, TypeInvariantFailure<E>> {
        let Self { entries, meter } = self;
        let entries = entries
            .into_iter()
            .map(|entry| entry.with_parsed_type_invariants(&mut parse))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { entries, meter })
    }

    /// The work the operation has consumed so far, for one limit kind.
    pub fn consumed(&self, kind: crate::model::accounting::LimitKind) -> u64 {
        self.meter.consumed(kind)
    }

    /// Models whose admission is not under test: no work is counted.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_entries(entries: Vec<SelectedModel>) -> Self {
        Self {
            entries,
            meter: Meter::new(ModelNormalizationLimits::UNLIMITED),
        }
    }

    /// One package admitted through the same normalization producer, on its
    /// own meter under `limits`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(
        alias: &str,
        span: Span,
        package: DomainPackage,
        limits: ModelNormalizationLimits,
    ) -> Result<Self, UnitIntakeRefusal> {
        let mut meter = Meter::new(limits);
        let view = match normalize_on(&package, &mut meter) {
            NormalizeOutcome::Completed(view) => view,
            NormalizeOutcome::Refused(refusals) => {
                return Err(UnitIntakeRefusal {
                    alias: alias.to_owned(),
                    span,
                    cause: UnitIntakeCause::Refused(refusals),
                })
            }
            NormalizeOutcome::Incomplete(incomplete) => {
                return Err(UnitIntakeRefusal {
                    alias: alias.to_owned(),
                    span,
                    cause: UnitIntakeCause::Limit(incomplete),
                })
            }
        };
        Ok(Self {
            entries: vec![SelectedModel {
                alias: alias.to_owned(),
                span,
                view,
                operation_clauses: Vec::new(),
                type_invariants: Vec::new(),
            }],
            meter,
        })
    }
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

/// A type invariant the caller's parser refused: which clause, where it is,
/// and the parser's own refusal.
#[derive(Clone, Debug)]
pub struct TypeInvariantFailure<E> {
    /// The refused clause's declaration key.
    pub clause: DeclarationKey,
    /// The `TypeDefinition.clauses[k]` member the clause is at.
    pub member: MemberPath,
    /// The parser's refusal.
    pub cause: E,
}

impl SelectedModel {
    /// FR-366: this model with the text of each type-level clause classified
    /// `Invariant` parsed by `parse`, the caller's FR-146 expression
    /// production. A clause in another language, and a guard, is not parsed.
    /// The first clause `parse` refuses stops the walk, located at
    /// `TypeDefinition.clauses[k]`, `k` the clause's position among its
    /// owner's clauses in the package.
    ///
    /// Marked `#[string_edge]`: a clause's `language` is the wire identifier
    /// of its authored language.
    #[ix_trace_rs::string_edge]
    pub fn with_parsed_type_invariants<E>(
        mut self,
        mut parse: impl FnMut(&str) -> Result<Expression, E>,
    ) -> Result<Self, TypeInvariantFailure<E>> {
        let roles = self.view.type_clause_roles();
        let mut positions: BTreeMap<&DeclarationKey, usize> = BTreeMap::new();
        let mut parsed = Vec::new();
        for record in &self.view.domain_package().records {
            let DomainPackageRecord::Clause(clause) = record else {
                continue;
            };
            let position = {
                let next = positions.entry(&clause.owner).or_default();
                let position = *next;
                *next += 1;
                position
            };
            if roles.role(&clause.key) != Some(ClauseRole::Invariant) {
                continue;
            }
            let expression = parse(&clause.text).map_err(|cause| TypeInvariantFailure {
                clause: clause.key.clone(),
                member: MemberPath::new(MemberRoot::TypeDefinition)
                    .name("clauses")
                    .index(position),
                cause,
            })?;
            parsed.push(ParsedTypeInvariant {
                clause: clause.key.clone(),
                owner: clause.owner.clone(),
                position,
                expression,
            });
        }
        self.type_invariants = parsed;
        Ok(self)
    }

    /// FR-103: this model with each kept `quire`-language `requires` and
    /// `ensures` clause of its operations parsed by `parse`, the caller's
    /// FR-146 expression production. Intake keeps the text and parses none
    /// of it, so the semantics layer takes no dependency on S1: the compile
    /// driver passes its parser here. A clause in another language is kept
    /// verbatim and not parsed. The first clause `parse` refuses stops the
    /// walk.
    ///
    /// Marked `#[string_edge]`: a clause's `language` is the wire identifier
    /// of its authored language, compared here to pick the `quire` clauses.
    #[ix_trace_rs::string_edge]
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
    /// This cause's catalog code and cause, from one source: the first model
    /// refusal's own catalog code (ADR-013 R-09), and
    /// `resource_exhausted`/`insufficient-next-charge` for a denied
    /// normalization charge, which is the caller's budget wherever it is read
    /// (ADR-014 section 1). `None` for a cause with no catalog cause.
    pub fn catalog_code(&self) -> Option<CatalogCode> {
        match self {
            Self::Refused(refusals) => Some(refusals[0].catalog_code()),
            Self::Limit(_) => Some(CatalogCode::new(
                "resource_exhausted",
                "insufficient-next-charge",
            )),
            Self::ArtifactDigest | Self::Invariant => None,
        }
    }

    /// This cause's native code, read from [`Self::catalog_code`] where the
    /// cause has one.
    pub fn code(&self) -> Code {
        match self {
            Self::ArtifactDigest => Code::InvalidModelBinding,
            Self::Invariant => Code::RuntimeInvariant,
            Self::Refused(_) | Self::Limit(_) => self
                .catalog_code()
                .and_then(|catalog| Code::from_code(catalog.code()))
                .unwrap_or(Code::RuntimeInvariant),
        }
    }

    /// The catalog cause under [`Self::code`], where the cause names one.
    pub fn cause(&self) -> Option<&'static str> {
        self.catalog_code().map(|catalog| catalog.cause())
    }

    /// The caller-budget denial of a reached ceiling (FR-255 Behavior 1):
    /// a denied normalization charge, a walk ceiling or intake's byte limit.
    /// `None` for a cause that is no ceiling.
    pub fn denial(&self) -> Option<BudgetDenial> {
        match self {
            Self::Refused(refusals) => refusals[0].cause.denial(),
            Self::Limit(incomplete) => Some(incomplete.denial()),
            Self::ArtifactDigest | Self::Invariant => None,
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
) -> Result<SelectedModels, UnitIntakeRefusal> {
    admit_unit_with_cancel(selections, packages, limits, &Cancel::new())
}

/// [`admit_unit`] under the caller's [`Cancel`] handle, polled at every
/// normalization charge (FR-276).
pub fn admit_unit_with_cancel(
    selections: &[ModelSelection],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
    cancel: &Cancel,
) -> Result<SelectedModels, UnitIntakeRefusal> {
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
    // Selections are admitted and normalized in the selection's canonical
    // order (identity bytes, then digest), never in source or alias order.
    offered.sort_by(|(_, left), (_, right)| {
        left.identity
            .as_bytes()
            .cmp(right.identity.as_bytes())
            .then_with(|| left.digest.cmp(&right.digest))
    });
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
    let mut meter = Meter::new(limits).with_cancel(cancel.clone());
    let mut entries = Vec::with_capacity(admitted.len());
    for ((selection, _), package_ref, document) in admitted {
        let package = read_domain_package(package_ref, &document).map_err(|refusals| {
            let cause = Refusals::try_from(refusals)
                .map_or(UnitIntakeCause::Invariant, UnitIntakeCause::Refused);
            refuse(selection, cause)
        })?;
        let view = match normalize_on(&package, &mut meter) {
            NormalizeOutcome::Completed(view) => view,
            NormalizeOutcome::Refused(refusals) => {
                return Err(refuse(selection, UnitIntakeCause::Refused(refusals)))
            }
            NormalizeOutcome::Incomplete(incomplete) => {
                return Err(refuse(selection, UnitIntakeCause::Limit(incomplete)))
            }
        };
        entries.push(SelectedModel {
            alias: selection.alias.clone(),
            span: selection.span,
            view,
            operation_clauses: Vec::new(),
            type_invariants: Vec::new(),
        });
    }
    Ok(SelectedModels { entries, meter })
}

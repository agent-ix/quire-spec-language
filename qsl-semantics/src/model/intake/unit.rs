// SPDX-License-Identifier: AGPL-3.0-or-later
//! I1 for one source unit (ADR-011 §2.1 I1, FR-056): the unit's `model`
//! declarations, each admitted under FR-154 against the package input, read
//! into domain package records and normalized into the effective view E3
//! resolves the unit's model type names against.
//!
//! A declaration selects a domain package by its identity alone.

use qsl_foundation::diagnostic::BudgetDenial;
use qsl_foundation::selection::ModelSelection;
use std::collections::BTreeMap;

use qsl_foundation::{Code, Span};
use quire_contract_model::{
    CheckedNodeId, CheckedNodePathSegment, SelectedExactDeclarationForm,
    SelectedExactDeclarationRow, SelectedExactFieldRow, SelectedExactIdentities,
    SelectedExactIdentity,
};
use quire_exact::{Cancel, MemberId, MemberOwner};
use quire_semantic_value::catalog::CatalogCode;

use super::{admit_located, read_domain_package, PackageInput};
use crate::model::accounting::{Incomplete, Meter, ModelNormalizationLimits};
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, ParsedOperationClause, ParsedTypeInvariant,
};
use crate::model::normalize::{normalize_on, EffectiveView, NormalizeOutcome, Refusals};
use crate::model::refusal::{MemberPath, MemberRoot};
use crate::model::type_invariant::ClauseRole;
use qsl_forms::Expression;
use quire_checked::key::DeclarationKey;
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
    exact_declarations: Vec<SelectedExactDeclarationRow>,
    exact_fields: Vec<SelectedExactFieldRow>,
}

impl Default for SelectedModels {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            meter: Meter::new(ModelNormalizationLimits::default()),
            exact_declarations: Vec::new(),
            exact_fields: Vec::new(),
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
    pub(crate) fn into_parts(
        self,
    ) -> (
        Vec<SelectedModel>,
        Meter,
        Vec<SelectedExactDeclarationRow>,
        Vec<SelectedExactFieldRow>,
    ) {
        (
            self.entries,
            self.meter,
            self.exact_declarations,
            self.exact_fields,
        )
    }

    /// The selected model stage's rows, validated against the package IR
    /// admission that retained the same selected declarations.
    pub fn exact_identities(
        &self,
        package: &quire_contract_model::CheckedPackageV2,
    ) -> Result<SelectedExactIdentities, quire_contract_model::ExactTypeProjectionError> {
        SelectedExactIdentities::from_stage_rows(
            package,
            self.exact_declarations
                .iter()
                .filter(|row| {
                    package.graph().nodes.iter().any(|node| {
                        node.node_id == row.structural_node
                            && match row.form {
                                SelectedExactDeclarationForm::RecordValueType => {
                                    node.node_tag.as_ref() == "model"
                                        && node.semantic_form.as_ref() == "record_value_type"
                                }
                                SelectedExactDeclarationForm::ObjectType => {
                                    node.node_tag.as_ref() == "model"
                                        && node.semantic_form.as_ref() == "object_type"
                                }
                                SelectedExactDeclarationForm::Union => {
                                    node.node_tag.as_ref() == "composite_type"
                                        && node.semantic_form.as_ref() == "union"
                                }
                                SelectedExactDeclarationForm::DeclaredUnit => {
                                    node.node_tag.as_ref() == "scalar_type"
                                        && node.semantic_form.as_ref() == "unit"
                                }
                            }
                    })
                })
                .cloned(),
            self.exact_fields
                .iter()
                .filter(|row| {
                    package.graph().nodes.iter().any(|node| {
                        node.node_id == row.record_node
                            && node.node_tag.as_ref() == "model"
                            && node.semantic_form.as_ref() == "record_value_type"
                    })
                })
                .cloned(),
        )
    }

    /// The declaration identity rows minted while the selected domain
    /// documents were admitted and normalized.
    pub fn exact_declaration_rows(&self) -> &[SelectedExactDeclarationRow] {
        &self.exact_declarations
    }

    /// The field identity rows minted while the selected domain documents
    /// were admitted and normalized.
    pub fn exact_field_rows(&self) -> &[SelectedExactFieldRow] {
        &self.exact_fields
    }

    /// Each selected model parsed by `parse` (see
    /// [`SelectedModel::with_parsed_operation_clauses`]); the meter is kept.
    pub fn with_parsed_operation_clauses<E>(
        self,
        mut parse: impl FnMut(&str) -> Result<Expression, E>,
    ) -> Result<Self, OperationClauseFailure<E>> {
        let Self {
            entries,
            meter,
            exact_declarations,
            exact_fields,
        } = self;
        let entries = entries
            .into_iter()
            .map(|entry| entry.with_parsed_operation_clauses(&mut parse))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            entries,
            meter,
            exact_declarations,
            exact_fields,
        })
    }

    /// Each selected model parsed by `parse` (see
    /// [`SelectedModel::with_parsed_type_invariants`]); the meter is kept.
    pub fn with_parsed_type_invariants<E>(
        self,
        mut parse: impl FnMut(&str) -> Result<Expression, E>,
    ) -> Result<Self, TypeInvariantFailure<E>> {
        let Self {
            entries,
            meter,
            exact_declarations,
            exact_fields,
        } = self;
        let entries = entries
            .into_iter()
            .map(|entry| entry.with_parsed_type_invariants(&mut parse))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            entries,
            meter,
            exact_declarations,
            exact_fields,
        })
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
            exact_declarations: Vec::new(),
            exact_fields: Vec::new(),
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
            exact_declarations: Vec::new(),
            exact_fields: Vec::new(),
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
            Self::Invariant => None,
        }
    }

    /// This cause's native code, read from [`Self::catalog_code`] where the
    /// cause has one.
    pub fn code(&self) -> Code {
        match self {
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
            Self::Invariant => None,
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

/// Produce IR's selected-identity rows from the exact admitted document and
/// normalization that `SelectedModel` retains. The row payloads are minted
/// here; consumers only validate and carry these rows with the package.
fn selected_exact_rows(
    package: &DomainPackage,
    view: &EffectiveView,
) -> Result<(Vec<SelectedExactDeclarationRow>, Vec<SelectedExactFieldRow>), ()> {
    let selected_document = package.model_selection.identity.as_str();
    let mut declarations = Vec::new();
    let mut fields = Vec::new();
    let mut record_keys = BTreeMap::new();

    for record in &package.records {
        match record {
            DomainPackageRecord::RecordValueType(record) => {
                let exact = crate::model::key::model_node_key(
                    &record.key.package,
                    "model",
                    "record_value_type",
                    &record.key.node,
                )
                .map_err(|_| ())?;
                let structural =
                    checked_model_node(&record.key.package, "record_value_type", &record.key.node);
                record_keys.insert(record.key.clone(), exact.clone());
                declarations.push(SelectedExactDeclarationRow::new(
                    structural,
                    selected_document,
                    SelectedExactDeclarationForm::RecordValueType,
                    record.key.node.as_str(),
                    SelectedExactIdentity::Record(exact),
                ));
            }
            DomainPackageRecord::ObjectType(record) => {
                let Some(effective) = view.type_identities().get(&record.key) else {
                    return Err(());
                };
                crate::model::key::object_type_node_key(&record.key).map_err(|_| ())?;
                declarations.push(SelectedExactDeclarationRow::new(
                    checked_model_node(&record.key.package, "object_type", &record.key.node),
                    selected_document,
                    SelectedExactDeclarationForm::ObjectType,
                    record.key.node.as_str(),
                    SelectedExactIdentity::Object(effective.clone()),
                ));
            }
            _ => {}
        }
    }

    for record in &package.records {
        let DomainPackageRecord::RecordValueType(record) = record else {
            continue;
        };
        let Some(record_key) = record_keys.get(&record.key) else {
            return Err(());
        };
        let Some(effective) = view.type_identities().get(&record.key) else {
            return Err(());
        };
        for entry in view.declarations().iter().filter(|entry| {
            entry.visible && entry.preimage.owner_effective_type.as_ref() == Some(effective)
        }) {
            let Some(DomainPackageRecord::FieldMember(field)) = package
                .records
                .iter()
                .find(|candidate| candidate.key() == &entry.preimage.original)
            else {
                continue;
            };
            let Some(name) = package
                .original(&field.key)
                .and_then(|original| original.member_name.as_deref())
            else {
                return Err(());
            };
            let member_id =
                MemberId::new(MemberOwner::Node(record_key.clone()), name).map_err(|_| ())?;
            fields.push(SelectedExactFieldRow::new(
                checked_model_node(&record.key.package, "record_value_type", &record.key.node),
                selected_document,
                record.key.node.as_str(),
                field.key.node.as_str(),
                member_id,
            ));
        }
    }
    Ok((declarations, fields))
}

/// Structural IR node recipe for one selected model declaration. The path
/// is exactly the `model/<form>/<original-node>` declaration key recipe used
/// by `model_node_key` above.
fn checked_model_node(package: &str, form: &str, node: &str) -> CheckedNodeId {
    CheckedNodeId {
        package: package.into(),
        path: vec![
            CheckedNodePathSegment::Name("model".into()),
            CheckedNodePathSegment::Name(form.into()),
            CheckedNodePathSegment::Name(node.into()),
        ],
        ordinal: 0,
    }
}

#[cfg(test)]
mod selected_exact_identity_tests {
    use super::*;
    use crate::model::domain_package::{
        FieldMemberRecord, Multiplicity, ObjectTypeRecord, OriginalDeclaration,
        RecordValueTypeRecord, ValueTypeRef,
    };
    use crate::model::normalize::{normalize_on, NormalizeOutcome};
    use quire_exact::Presence;
    use serde_json::json;

    fn key(node: &str) -> DeclarationKey {
        DeclarationKey {
            package: "acme/orders".into(),
            node: node.into(),
        }
    }

    #[test]
    fn intake_mints_record_field_and_reference_supply_rows_from_the_admitted_model() {
        let record = key("ix://acme/orders/Receipt");
        let parent = key("ix://acme/orders/Identified");
        let field = key("ix://acme/orders/Receipt/number");
        let inherited_field = key("ix://acme/orders/Identified/id");
        let object = key("ix://acme/orders/Order");
        let mut package = DomainPackage::new(
            crate::model::domain_package::DomainPackageRef {
                identity: "acme/orders".into(),
            },
            vec![
                DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
                    key: parent.clone(),
                    identity_fields: Vec::new(),
                    supertypes: Vec::new(),
                }),
                DomainPackageRecord::FieldMember(FieldMemberRecord {
                    key: inherited_field.clone(),
                    owner: parent.clone(),
                    value_type: ValueTypeRef::Native(
                        crate::model::domain_package::NativeValueType::Integer,
                    ),
                    multiplicity: Multiplicity {
                        lower: 1,
                        upper: Some(1),
                        ordered: false,
                        unique: true,
                    },
                    presence: Presence::Required,
                    subsets: Vec::new(),
                    redefines: None,
                    position: 0,
                }),
                DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
                    key: record.clone(),
                    identity_fields: Vec::new(),
                    supertypes: vec![parent.clone()],
                }),
                DomainPackageRecord::FieldMember(FieldMemberRecord {
                    key: field.clone(),
                    owner: record.clone(),
                    value_type: ValueTypeRef::Native(
                        crate::model::domain_package::NativeValueType::Integer,
                    ),
                    multiplicity: Multiplicity {
                        lower: 1,
                        upper: Some(1),
                        ordered: false,
                        unique: true,
                    },
                    presence: Presence::Required,
                    subsets: Vec::new(),
                    redefines: None,
                    position: 0,
                }),
                DomainPackageRecord::ObjectType(ObjectTypeRecord {
                    key: object.clone(),
                    interface_features: None,
                    abstract_type: false,
                    supertypes: Vec::new(),
                }),
            ],
        );
        package.originals.insert(
            inherited_field.clone(),
            OriginalDeclaration {
                meaning: None,
                origin: json!({}),
                member_name: Some("id".into()),
            },
        );
        package.originals.insert(
            field.clone(),
            OriginalDeclaration {
                meaning: None,
                origin: json!({}),
                member_name: Some("number".into()),
            },
        );
        let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
        let view = match normalize_on(&package, &mut meter) {
            NormalizeOutcome::Completed(view) => view,
            other => panic!("the fixture model normalizes: {other:?}"),
        };

        let (declarations, fields) = selected_exact_rows(&package, &view)
            .expect("the admitted model produces typed identity rows");
        let record_node = checked_model_node(&record.package, "record_value_type", &record.node);
        let record_key = crate::model::key::model_node_key(
            &record.package,
            "model",
            "record_value_type",
            &record.node,
        )
        .expect("the record declaration key is valid");
        assert!(declarations.iter().any(|row| {
            row.structural_node == record_node
                && row.selected_document.as_ref() == "acme/orders"
                && row.original_identity.as_ref() == record.node
                && row.form == SelectedExactDeclarationForm::RecordValueType
                && row.exact_identity == SelectedExactIdentity::Record(record_key.clone())
        }));
        assert!(declarations.iter().any(|row| {
            row.original_identity.as_ref() == object.node
                && row.form == SelectedExactDeclarationForm::ObjectType
                && row.exact_identity
                    == SelectedExactIdentity::Object(view.type_identities()[&object].clone())
        }));
        assert!(fields.iter().any(|row| {
            row.record_node == record_node
                && row.selected_document.as_ref() == "acme/orders"
                && row.record_original_identity.as_ref() == record.node
                && row.field_original_identity.as_ref() == field.node
                && row.member_id
                    == MemberId::new(MemberOwner::Node(record_key.clone()), "number")
                        .expect("a declared field name produces a member identity")
        }));
        assert!(fields.iter().any(|row| {
            row.record_node == record_node
                && row.record_original_identity.as_ref() == record.node
                && row.field_original_identity.as_ref() == inherited_field.node
                && row.member_id
                    == MemberId::new(MemberOwner::Node(record_key.clone()), "id")
                        .expect("an inherited field name produces a member identity")
        }));
    }
}

/// Admit each of a unit's `model` declarations, in source order, against
/// `packages` (a [`PackageInput`]): FR-154 admission (one version of
/// each identity), [`crate::model::intake::read_records`], then normalization under `limits`. It
/// stops at the first declaration that does not admit.
pub fn admit_unit(
    selections: &[ModelSelection],
    packages: &PackageInput,
    limits: ModelNormalizationLimits,
) -> Result<SelectedModels, UnitIntakeRefusal> {
    admit_unit_with_cancel(selections, packages, limits, &Cancel::new())
}

/// [`admit_unit`] under the caller's [`Cancel`] handle, polled at every
/// normalization charge (FR-276).
pub fn admit_unit_with_cancel(
    selections: &[ModelSelection],
    packages: &PackageInput,
    limits: ModelNormalizationLimits,
    cancel: &Cancel,
) -> Result<SelectedModels, UnitIntakeRefusal> {
    let refuse = |selection: &ModelSelection, cause| UnitIntakeRefusal {
        alias: selection.alias.clone(),
        span: selection.span,
        cause,
    };
    // Selections are admitted and normalized in canonical order (identity
    // bytes), never in source or alias order.
    let mut offered: Vec<&ModelSelection> = selections.iter().collect();
    offered.sort_by(|left, right| left.identity.as_bytes().cmp(right.identity.as_bytes()));
    let admitted = admit_located(
        &offered,
        |selection| selection.identity.as_str(),
        packages,
        limits.intake,
    )
    .map_err(|(selection, refusal)| {
        refuse(
            selection,
            UnitIntakeCause::Refused(Refusals::new(refusal, Vec::new())),
        )
    })?;
    let mut meter = Meter::new(limits).with_cancel(cancel.clone());
    let mut entries = Vec::with_capacity(admitted.len());
    let mut exact_declarations = Vec::new();
    let mut exact_fields = Vec::new();
    for (selection, package_ref, document) in admitted {
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
        let (declarations, fields) = selected_exact_rows(&package, &view)
            .map_err(|()| refuse(selection, UnitIntakeCause::Invariant))?;
        exact_declarations.extend(declarations);
        exact_fields.extend(fields);
        entries.push(SelectedModel {
            alias: selection.alias.clone(),
            span: selection.span,
            view,
            operation_clauses: Vec::new(),
            type_invariants: Vec::new(),
        });
    }
    Ok(SelectedModels {
        entries,
        meter,
        exact_declarations,
        exact_fields,
    })
}

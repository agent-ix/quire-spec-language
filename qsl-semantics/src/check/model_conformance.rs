// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conformance consumes the same selection owner and the facts of completed
//! source checking. Structural target resolution belongs to normalization.

use super::field_refinement::{checked_refinement_failures, RefinementObligation};
use super::lowering::AdmittedModel;
use super::state_clause::{StateClauseDeclaration, TypedStateClause};
use crate::model::accounting::{Incomplete, Meter};
use crate::model::conformance::{self, ConformanceCheckOutcome, ConformanceOutcome};
use crate::model::domain_package::DomainPackageRecord;
use crate::model::intake::SelectedModel;
use crate::model::key::DeclarationKey;
use crate::model::normalize::ModelRefusalCause;
use qsl_foundation::Code;

/// One failed axis of one immediate-parent member pair. A refinement
/// failure additionally names the exposed writer and its own obligation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelConformanceFailure {
    pub member: DeclarationKey,
    pub parent: DeclarationKey,
    pub writer: Option<DeclarationKey>,
    pub obligation: Option<RefinementObligation>,
    pub axis: &'static str,
    pub code: Code,
    pub cause: ModelRefusalCause,
    pub detail: String,
}

impl std::hash::Hash for ModelConformanceFailure {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        use std::hash::Hash;
        self.member.hash(state);
        self.parent.hash(state);
        self.writer.hash(state);
        self.obligation.hash(state);
        self.axis.hash(state);
        self.code.hash(state);
        self.cause.as_str().hash(state);
        self.detail.hash(state);
    }
}

pub(crate) enum StageDenial {
    Refused(Vec<ModelConformanceFailure>),
    Incomplete(Incomplete),
    MissingModel(String),
}

fn append(outcome: ConformanceCheckOutcome, member: &DeclarationKey,
    parent: &DeclarationKey, failures: &mut Vec<ModelConformanceFailure>) -> Result<(), Incomplete> {
    let axes = match outcome {
        ConformanceCheckOutcome::Incomplete(incomplete) => return Err(incomplete),
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Compatible) => return Ok(()),
        ConformanceCheckOutcome::Completed(ConformanceOutcome::Refused(axes)) => axes,
        ConformanceCheckOutcome::Refused(refusal) => vec![conformance::AxisFailure {
            axis: "target", code: refusal.code, cause: refusal.cause, detail: refusal.detail,
        }],
    };
    failures.extend(axes.into_iter().map(|axis| ModelConformanceFailure {
        member: member.clone(), parent: parent.clone(), writer: None, obligation: None,
        axis: axis.axis, code: axis.code, cause: axis.cause, detail: axis.detail,
    }));
    Ok(())
}

/// `selected` retains intake's canonical selection order and meter. Every
/// real clause has completed before entry; a denied charge discards all
/// failures collected in this unfinished conformance stage.
pub(crate) fn check(selected: &[SelectedModel], meter: &mut Meter, models: &[AdmittedModel],
    forms: &[StateClauseDeclaration], checked: &[TypedStateClause]) -> Result<(), StageDenial> {
    let mut failures = Vec::new();
    for selection in selected {
        let index = selection.view.model_index();
        let model = models.iter().find(|model| model.selection() == selection.view.model_selection())
            .ok_or_else(|| StageDenial::MissingModel(selection.view.model_selection().identity.clone()))?;
        let mut records: Vec<_> = index.package().records.iter().collect();
        records.sort_by(|a, b| a.key().cmp(b.key()));
        for record in records {
            match record {
                DomainPackageRecord::FieldMember(field) => {
                    if let Some(parent) = &field.redefines {
                        append(conformance::check_field_redefinition(index, &field.key, parent, meter),
                            &field.key, parent, &mut failures).map_err(StageDenial::Incomplete)?;
                        conformance::charge_refinement_axis(meter).map_err(StageDenial::Incomplete)?;
                        failures.extend(checked_refinement_failures(model, &field.key, parent, forms, checked)
                                .into_iter().map(|failure| ModelConformanceFailure {
                                    member: failure.member, parent: failure.parent, writer: Some(failure.writer),
                                    obligation: Some(failure.obligation), axis: "refinement", code: Code::UndefinedExpression,
                                    cause: ModelRefusalCause::UnprovedRefinement,
                                    detail: "the exposed writer does not establish its field refinement".to_owned(),
                                }));
                    }
                    for parent in &field.subsets {
                        append(conformance::check_subsetting(index, &field.key, parent, meter),
                            &field.key, parent, &mut failures).map_err(StageDenial::Incomplete)?;
                    }
                }
                DomainPackageRecord::OperationMember(operation) => {
                    if let Some(parent) = &operation.redefines {
                        append(conformance::check_operation_redefinition(index, &operation.key, parent, meter),
                            &operation.key, parent, &mut failures).map_err(StageDenial::Incomplete)?;
                    }
                }
                _ => {},
            }
        }
    }
    if failures.is_empty() { Ok(()) } else { Err(StageDenial::Refused(failures)) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::accounting::{ChargePoint, LimitKind, ModelNormalizationLimits};
    use crate::model::domain_package::{DomainPackage, DomainPackageRef, FieldMemberRecord,
        Multiplicity, NativeValueType, ObjectTypeRecord, ValueTypeRef};
    use crate::model::intake::SelectedModels;
    use ix_trace_rs::trace;
    use quire_exact::Presence;

    fn package() -> DomainPackage {
        let mut records = vec![
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: DeclarationKey::fixture("A"), interface_features: None,
                abstract_type: false, supertypes: Vec::new(),
            }),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: DeclarationKey::fixture("B"), interface_features: None,
                abstract_type: false, supertypes: vec![DeclarationKey::fixture("A")],
            }),
        ];
        for name in ["x", "y"] {
            for (owner, native, lower, upper, redefines) in [
                ("A", NativeValueType::Integer, 1, 1, None),
                ("B", NativeValueType::Boolean, 0, 2, Some(DeclarationKey::fixture(format!("A/{name}")))),
            ] {
                records.push(DomainPackageRecord::FieldMember(FieldMemberRecord {
                    key: DeclarationKey::fixture(format!("{owner}/{name}")),
                    owner: DeclarationKey::fixture(owner), value_type: ValueTypeRef::Native(native),
                    multiplicity: Multiplicity { lower, upper: Some(upper), ordered: false, unique: true },
                    presence: Presence::Required, subsets: Vec::new(), redefines,
                }));
            }
        }
        DomainPackage::new(DomainPackageRef {
            identity: "test/orders".to_owned(), version: "1".to_owned(), digest: [1; 32],
        }, records)
    }

    fn admitted(limits: ModelNormalizationLimits) -> (Vec<SelectedModel>, Meter, Vec<AdmittedModel>) {
        let group = SelectedModels::fixture("M", qsl_foundation::Span { start: 0, end: 1 }, package(), limits)
            .expect("the real structural producer admits both member pairs");
        let (selected, meter) = group.into_parts();
        let models = selected.iter().map(|selection| AdmittedModel::from_view(&selection.view)).collect();
        (selected, meter, models)
    }

    #[test]
    #[trace("TC-196")]
    fn completed_stage_retains_each_axis_and_member_pair_after_checked_clauses() {
        let (selected, mut meter, models) = admitted(ModelNormalizationLimits::UNLIMITED);
        let before = meter.consumed(LimitKind::WorkUnits);
        let Err(StageDenial::Refused(failures)) = check(&selected, &mut meter, &models, &[], &[]) else {
            panic!("both actual fields fail both static axes");
        };
        assert_eq!(failures.iter().map(|failure| (&failure.member, &failure.parent, failure.axis)).collect::<Vec<_>>(), vec![
            (&DeclarationKey::fixture("B/x"), &DeclarationKey::fixture("A/x"), "value-type"),
            (&DeclarationKey::fixture("B/x"), &DeclarationKey::fixture("A/x"), "multiplicity"),
            (&DeclarationKey::fixture("B/y"), &DeclarationKey::fixture("A/y"), "value-type"),
            (&DeclarationKey::fixture("B/y"), &DeclarationKey::fixture("A/y"), "multiplicity"),
        ]);
        assert!(failures.iter().all(|failure| failure.writer.is_none() && failure.obligation.is_none()));
        assert_eq!(meter.consumed(LimitKind::WorkUnits), before + 6);
    }

    #[test]
    #[trace("TC-196")]
    fn denied_later_pair_exposes_only_exact_incomplete_without_earlier_failures() {
        let (_, unlimited, _) = admitted(ModelNormalizationLimits::UNLIMITED);
        let normalization_work = unlimited.consumed(LimitKind::WorkUnits);
        let (selected, mut meter, models) = admitted(ModelNormalizationLimits {
            work_units: normalization_work + 3, ..ModelNormalizationLimits::UNLIMITED
        });
        let Err(StageDenial::Incomplete(incomplete)) = check(&selected, &mut meter, &models, &[], &[]) else {
            panic!("the second pair's owning type charge is unavailable");
        };
        assert_eq!(incomplete, Incomplete {
            limit_kind: LimitKind::WorkUnits, limit: normalization_work + 3,
            consumed: normalization_work + 3, next_charge: 1, charge_point: ChargePoint::ConformanceAxis,
        });
        assert_eq!(meter.consumed(LimitKind::WorkUnits), normalization_work + 3);
    }
}

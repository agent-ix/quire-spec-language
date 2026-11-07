// SPDX-License-Identifier: AGPL-3.0-or-later
//! Rejoin producer equality evidence with the admitted IR equality decision.

use quire_contract_model::{
    CheckedPackageV2, SelectedEqualityApplicationRow, SelectedEqualityFactRow,
};
use quire_semantic_value::declaration::AdmittedEqualityDecision;

use super::{Emission, EqualityFactWitness, EqualityOperandTypeRole, EqualityUnitRole};

/// The producer's equality rows of one emission rejoined to IR's row types,
/// built once and shared by every equality application read from it.
pub(crate) struct EqualityRows {
    application_rows: Vec<SelectedEqualityApplicationRow>,
    fact_rows: Vec<SelectedEqualityFactRow>,
}

impl Emission {
    /// Read the admitted equality decision for one exact emitted application.
    /// The producer rows supply only identity correspondence; IR validates
    /// them against this package's admitted decision before returning it.
    pub fn admitted_equality(
        &self,
        package: &CheckedPackageV2,
        application: &quire_contract_model::CheckedNodeId,
        occurrence: &quire_contract_model::CheckedOccurrence,
        budget: &mut quire_contract_model::NativeJoinBudget,
    ) -> Result<AdmittedEqualityDecision, quire_contract_model::AdmittedEqualityReadError> {
        self.admitted_equality_from(
            &self.equality_rows(),
            package,
            application,
            occurrence,
            budget,
        )
    }

    /// [`Self::admitted_equality`] over rows already built by
    /// [`Self::equality_rows`].
    pub(crate) fn admitted_equality_from(
        &self,
        rows: &EqualityRows,
        package: &CheckedPackageV2,
        application: &quire_contract_model::CheckedNodeId,
        occurrence: &quire_contract_model::CheckedOccurrence,
        budget: &mut quire_contract_model::NativeJoinBudget,
    ) -> Result<AdmittedEqualityDecision, quire_contract_model::AdmittedEqualityReadError> {
        package.admitted_equality(
            application,
            occurrence,
            &rows.application_rows,
            &rows.fact_rows,
            self.selected_source_unit_rows(),
            budget,
        )
    }

    /// The producer's equality witness of this emission for the package
    /// `package` was emitted as, as the handoff writes it to `equality.json`
    /// (FR-531 "The equality file").
    pub fn equality_witness(&self, package: &CheckedPackageV2) -> crate::ReadEqualityRows {
        let rows = self.equality_rows();
        crate::ReadEqualityRows::of_rows(
            package,
            rows.application_rows,
            rows.fact_rows,
            self.selected_source_unit_rows().to_vec(),
        )
    }

    /// Every producer equality row of this emission, in IR's row types.
    pub(crate) fn equality_rows(&self) -> EqualityRows {
        let application_rows = self
            .equality_producer_identity_rows()
            .iter()
            .map(|row| {
                SelectedEqualityApplicationRow::new(
                    row.checked_node().clone(),
                    row.occurrence().clone(),
                    row.source_map_entry().clone(),
                    row.application().clone(),
                    row.location().clone(),
                )
            })
            .collect::<Vec<_>>();
        let selected_nodes = self
            .wire_nodes
            .iter()
            .map(|node| &node.node_id)
            .collect::<std::collections::BTreeSet<_>>();
        let fact_rows = self
            .equality_fact_witness_rows()
            .iter()
            .filter(|row| {
                row.checked_application().package == row.structural_node().package
                    && selected_nodes.contains(row.structural_node())
            })
            .map(|row| {
                use quire_contract_model::{
                    EqualityDimensionRole as IrDimensionRole, EqualityFactWitness as IrFact,
                    EqualityOperandTypeRole as IrOperandRole, EqualityUnitRole as IrUnitRole,
                };

                let fact = match row.fact() {
                    EqualityFactWitness::OperandType { position, role } => IrFact::OperandType {
                        position: *position,
                        role: match role {
                            EqualityOperandTypeRole::Source => IrOperandRole::Source,
                            EqualityOperandTypeRole::ConversionTarget => {
                                IrOperandRole::ConversionTarget
                            }
                        },
                    },
                    EqualityFactWitness::Unit {
                        unit_id,
                        quantity_type,
                        operand_position,
                        role,
                    } => IrFact::Unit {
                        unit_id: unit_id.clone(),
                        quantity_type: quantity_type.clone(),
                        operand_position: *operand_position,
                        role: match role {
                            EqualityUnitRole::Source => IrUnitRole::Source,
                            EqualityUnitRole::ConversionTarget => IrUnitRole::ConversionTarget,
                        },
                    },
                    EqualityFactWitness::DimensionDeclaration {
                        unit_id,
                        operand_position,
                        role,
                        ordinal,
                    } => IrFact::DimensionDeclaration {
                        unit_id: unit_id.clone(),
                        operand_position: *operand_position,
                        role: match role {
                            qsl_semantics::check::EqualityDimensionRole::UnitDimension => {
                                IrDimensionRole::UnitDimension
                            }
                            qsl_semantics::check::EqualityDimensionRole::ConversionDimension => {
                                IrDimensionRole::ConversionDimension
                            }
                            qsl_semantics::check::EqualityDimensionRole::CompoundBase => {
                                IrDimensionRole::CompoundBase
                            }
                            qsl_semantics::check::EqualityDimensionRole::BaseDimension => {
                                IrDimensionRole::BaseDimension
                            }
                        },
                        ordinal: *ordinal,
                    },
                    EqualityFactWitness::DimensionTerm {
                        unit_id,
                        operand_position,
                        ordinal,
                        base,
                        base_node,
                        exponent,
                    } => IrFact::DimensionTerm {
                        unit_id: unit_id.clone(),
                        operand_position: *operand_position,
                        ordinal: *ordinal,
                        base: base.clone(),
                        base_node: base_node.clone(),
                        exponent: exponent.clone(),
                    },
                    EqualityFactWitness::ConversionEdge {
                        unit_id,
                        operand_position,
                        ordinal,
                        target,
                        target_node,
                        scale,
                        offset,
                    } => IrFact::ConversionEdge {
                        unit_id: unit_id.clone(),
                        operand_position: *operand_position,
                        ordinal: *ordinal,
                        target: target.clone(),
                        target_node: target_node.clone(),
                        scale: scale.clone(),
                        offset: offset.clone(),
                    },
                    EqualityFactWitness::TextPlanType { position, role } => IrFact::TextPlanType {
                        position: *position,
                        role: match role {
                            EqualityOperandTypeRole::Source => IrOperandRole::Source,
                            EqualityOperandTypeRole::ConversionTarget => {
                                IrOperandRole::ConversionTarget
                            }
                        },
                    },
                    EqualityFactWitness::EnumMember {
                        source_declaration,
                        source_declaration_node,
                        declaration,
                        declaration_node,
                        member,
                        member_node,
                        case,
                        ordinal,
                        ordered,
                    } => IrFact::EnumMember {
                        source_declaration: source_declaration.clone(),
                        source_declaration_node: source_declaration_node.clone(),
                        declaration: declaration.clone(),
                        declaration_node: declaration_node.clone(),
                        member: member.clone(),
                        member_node: member_node.clone(),
                        case: case.clone(),
                        ordinal: *ordinal,
                        ordered: *ordered,
                    },
                };
                SelectedEqualityFactRow {
                    application: row.application().clone(),
                    checked_application: row.checked_application().clone(),
                    occurrence: row.occurrence().clone(),
                    source_map_entry: row.source_map_entry().clone(),
                    structural_key: row.structural_key().clone(),
                    structural_node: row.structural_node().clone(),
                    owner: row.owner().cloned(),
                    value_type: row.value_type().cloned(),
                    fact,
                }
            })
            .collect::<Vec<_>>();
        EqualityRows {
            application_rows,
            fact_rows,
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Rejoin producer equality evidence with the admitted IR equality decision.

use crate::native_row as charge;
use crate::NativeRowError;
use quire_contract_model::{
    CheckedPackageV2, NativeJoinLocus, NativeJoinLocusSource, SelectedEqualityApplicationRow,
    SelectedEqualityFactRow,
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
    ) -> Result<AdmittedEqualityDecision, crate::NativeJoinFailure> {
        let rows = self.equality_rows(budget)?;
        self.admitted_equality_from(&rows, package, application, occurrence, budget)
            .map_err(|error| crate::NativeJoinFailure::Equality(Box::new(error)))
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
    ///
    /// # Errors
    ///
    /// A compact [`NativeRowError`] on an unpaid materialization charge. No
    /// partial witness is returned and the original first denial is retained.
    pub fn equality_witness(
        &self,
        package: &CheckedPackageV2,
        budget: &mut quire_contract_model::NativeJoinBudget,
    ) -> Result<crate::ReadEqualityRows, NativeRowError> {
        let locus = charge::start(budget, "equality_witness")?;
        charge::text(budget, locus, package.name())?;
        charge::items(budget, locus, self.selected_source_unit_rows().len())?;
        for (index, unit) in self.selected_source_unit_rows().iter().enumerate() {
            let locus = NativeJoinLocus {
                source: NativeJoinLocusSource::AdapterRow(index),
                form: "equality_source_unit_row",
            };
            charge::source_unit(budget, locus, unit)?;
        }
        let rows = self.equality_rows(budget)?;
        Ok(crate::ReadEqualityRows::of_rows(
            package,
            rows.application_rows,
            rows.fact_rows,
            self.selected_source_unit_rows().to_vec(),
        ))
    }

    /// Every producer equality row of this emission, in IR's row types.
    pub(crate) fn equality_rows(
        &self,
        budget: &mut quire_contract_model::NativeJoinBudget,
    ) -> Result<EqualityRows, NativeRowError> {
        let locus = charge::start(budget, "equality_rows")?;
        charge::items(budget, locus, self.equality_producer_identity_rows().len())?;
        charge::work(
            budget,
            locus,
            (self.equality_producer_identity_rows().len() as u128).saturating_mul(2),
        )?;
        for (index, row) in self.equality_producer_identity_rows().iter().enumerate() {
            let locus = NativeJoinLocus {
                source: NativeJoinLocusSource::AdapterRow(index),
                form: "equality_application_row",
            };
            charge::node(budget, locus, row.checked_node())?;
            charge::source_map(budget, locus, row.source_map_entry())?;
            charge::text(budget, locus, row.location().occurrence().role().as_str())?;
        }
        charge::work(
            budget,
            locus,
            (self.equality_fact_witness_rows().len() as u128).saturating_mul(3),
        )?;
        let fact_count = self
            .equality_fact_witness_rows()
            .iter()
            .filter(|row| row.selected_for_emission)
            .count();
        charge::items(budget, locus, fact_count)?;
        for row in self
            .equality_fact_witness_rows()
            .iter()
            .enumerate()
            .filter(|(_, row)| row.selected_for_emission)
        {
            let (index, row) = row;
            let locus = NativeJoinLocus {
                source: NativeJoinLocusSource::AdapterRow(index),
                form: "equality_fact_row",
            };
            charge::node(budget, locus, row.checked_application())?;
            charge::node(budget, locus, row.structural_node())?;
            charge::source_map(budget, locus, row.source_map_entry())?;
            if let Some(owner) = row.owner() {
                charge::owner(budget, locus, owner)?;
            }
            if let Some(value_type) = row.value_type() {
                charge::value_type(budget, locus, value_type)?;
            }
            match row.fact() {
                EqualityFactWitness::OperandType { .. }
                | EqualityFactWitness::TextPlanType { .. } => {}
                EqualityFactWitness::Unit {
                    unit_id,
                    quantity_type,
                    ..
                } => {
                    charge::unit(budget, locus, unit_id)?;
                    charge::node(budget, locus, quantity_type)?;
                }
                EqualityFactWitness::DimensionDeclaration { unit_id, .. } => {
                    charge::unit(budget, locus, unit_id)?
                }
                EqualityFactWitness::DimensionTerm {
                    unit_id,
                    base_node,
                    exponent,
                    ..
                } => {
                    charge::unit(budget, locus, unit_id)?;
                    charge::node(budget, locus, base_node)?;
                    charge::text(budget, locus, exponent)?;
                }
                EqualityFactWitness::ConversionEdge {
                    unit_id,
                    target_node,
                    scale,
                    offset,
                    ..
                } => {
                    charge::unit(budget, locus, unit_id)?;
                    charge::node(budget, locus, target_node)?;
                    for value in [
                        &scale.numerator,
                        &scale.denominator,
                        &offset.numerator,
                        &offset.denominator,
                    ] {
                        charge::text(budget, locus, value)?;
                    }
                }
                EqualityFactWitness::EnumMember {
                    source_declaration_node,
                    declaration_node,
                    member_node,
                    case,
                    ..
                } => {
                    for node in [source_declaration_node, declaration_node, member_node] {
                        charge::node(budget, locus, node)?;
                    }
                    charge::text(budget, locus, case)?;
                }
            }
        }
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
        let mut fact_rows = Vec::with_capacity(fact_count);
        for row in self
            .equality_fact_witness_rows()
            .iter()
            .filter(|row| row.selected_for_emission)
        {
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
            fact_rows.push(SelectedEqualityFactRow {
                application: row.application().clone(),
                checked_application: row.checked_application().clone(),
                occurrence: row.occurrence().clone(),
                source_map_entry: row.source_map_entry().clone(),
                structural_key: row.structural_key().clone(),
                structural_node: row.structural_node().clone(),
                owner: row.owner().cloned(),
                value_type: row.value_type().cloned(),
                fact,
            });
        }
        Ok(EqualityRows {
            application_rows,
            fact_rows,
        })
    }
}

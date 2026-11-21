// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1036: lower a checked source synthesis into one `synthesis_request` node.

use filament_exact::{NodeKey, ValueType};
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};

use super::super::node_key::{
    Binding, BindingValue, BodyTerm, GroupMember, GroupTerm, LeafTerm, MemberTerm, NodeTag,
};
use super::super::synthesis::CheckedSynthesis;
use super::Binder;
use super::{Binders, Lowering};
use crate::check::refusal::CheckRefusal;

impl<'a> Lowering<'a> {
    /// Lower one typed value or plan synthesis into the checked package graph.
    pub(crate) fn synthesis_request(
        &mut self,
        index: usize,
        checked: &'a CheckedSynthesis,
    ) -> Result<NodeKey, CheckRefusal> {
        let scope = self.scope;
        if let Some(plan) = checked.plan() {
            self.scope = plan.scope();
        }
        let result = self.synthesis_request_in_scope(index, checked);
        self.scope = scope;
        result
    }

    fn synthesis_request_in_scope(
        &mut self,
        index: usize,
        checked: &'a CheckedSynthesis,
    ) -> Result<NodeKey, CheckRefusal> {
        let form = &checked.form;
        let claim_at = synthesis_location(index, &form.name.name);
        let grammar = match &checked.grammar {
            ValueType::Reference(target) => self.object_node(target.clone(), &claim_at)?,
            other => self.type_node(other, &claim_at)?,
        };
        let domain = self.type_node(&checked.domain, &claim_at)?;
        if let Some(plan) = checked.plan() {
            let binding = &plan.shape().operation_enum;
            self.enum_nodes_at(binding, claim_at.clone())?;
            for member in &binding.members {
                let key = self.enum_member_node(&claim_at, binding.declaration.key(), member)?;
                self.record(
                    key,
                    crate::check::family::OccurrenceRole::Generated,
                    claim_at.clone(),
                );
            }
        }
        let candidate_type = checked.plan().map_or_else(
            || checked.domain.clone(),
            |plan| ValueType::Composite(plan.shape().plan.clone()),
        );
        let candidate_type = self.type_node(&candidate_type, &claim_at)?;
        let candidate = self.parameter(&form.name.name, 0, candidate_type.clone(), &claim_at)?;
        let mut binders = Binders::new(
            &checked.slot_names,
            &[Binder {
                slot: 0,
                parameter: candidate.clone().into(),
                semantic_type: candidate_type.clone().into(),
            }],
        );
        let satisfies = self.expression(checked.satisfies.root(), &mut binders)?;
        let objective = match (&form.objective, &checked.objective) {
            (None, None) => BindingValue::Group(GroupTerm::new(Vec::new())),
            (Some(objective), Some(body)) => {
                let mut binders = Binders::new(
                    &checked.slot_names,
                    &[Binder {
                        slot: 0,
                        parameter: candidate.clone().into(),
                        semantic_type: candidate_type.into(),
                    }],
                );
                let root = self.expression(body.root(), &mut binders)?;
                let direction = match objective.direction {
                    fsl_forms::SynthesisDirectionForm::Minimize => "minimize",
                    fsl_forms::SynthesisDirectionForm::Maximize => "maximize",
                };
                let text = objective.text.clone();
                BindingValue::Group(GroupTerm::new(vec![
                    GroupMember::Binding(Binding::new(
                        "direction",
                        self.text_literal(direction, &claim_at)?,
                    )),
                    GroupMember::Binding(Binding::new("root", root)),
                    GroupMember::Binding(Binding::new(
                        "text",
                        self.text_literal(&text, &claim_at)?,
                    )),
                ]))
            }
            _ => {
                return Err(super::temporal::stage_fault(
                    &claim_at,
                    "synthesis-objective-check",
                ));
            }
        };
        let declaration = super::qualified_name(&form.name.name, &claim_at)?;
        let symmetry = self.text_literal("identity", &claim_at)?;
        let key = self.insert(
            &claim_at,
            NodeTag::Claim,
            "synthesis_request",
            None,
            Some(declaration),
            BodyTerm::aggregate(vec![
                MemberTerm::bound("grammar", LeafTerm::reference(grammar)),
                MemberTerm::bound("domain", LeafTerm::reference(domain)),
                MemberTerm::bound("candidate", LeafTerm::reference(candidate)),
                MemberTerm::binding("objective", objective),
                MemberTerm::bound("satisfies", satisfies),
                MemberTerm::bound("symmetry", symmetry),
            ]),
        )?;
        self.record(
            key.clone(),
            crate::check::family::OccurrenceRole::Declaration,
            claim_at,
        );
        Ok(key)
    }
}

/// The type-declaration origin that locates the source span of this claim.
pub(crate) fn synthesis_location(index: usize, name: &str) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: synthesis_site(index, name),
    })
}

/// The site key of the block's source-bearing expression spans.
pub(crate) fn synthesis_satisfies_location(index: usize, name: &str) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: format!("{} satisfies", synthesis_site(index, name)),
    })
}

/// The site key of the objective's source-bearing expression spans.
pub(crate) fn synthesis_objective_location(index: usize, name: &str) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: format!("{} objective", synthesis_site(index, name)),
    })
}

fn synthesis_site(index: usize, name: &str) -> String {
    format!("synthesis {index} {name}")
}

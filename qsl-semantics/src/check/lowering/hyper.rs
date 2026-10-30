// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-488: the `claim`/`hyperproperty` node each checked hyper clause over
//! `behaviours` lowers to (QSpec FR-402), with the nodes its body
//! references: the atom slots, the body wrapper and its formula, and the
//! `match` conjuncts with their step-label slots.
//!
//! The node's body is the aggregate of FR-402's seven bindings. Its
//! `quantifiers` binding is a Tuple of Groups (a quantifier Group, then a
//! fairness Group per constraint), the one shape FR-322's body grammar
//! admits for that content (FR-488 "Grammar fit"). The body wrapper is a
//! `temporal`/`temporal_clause` node over the first atom slot, with fixed
//! clock, activation, captures and fairness arguments.
//!
//! A clause names no position of its own beyond its declaration, so each
//! node is located at an `Origin::TypeDeclaration` site of the clause
//! (`hyper_site`); the node's `claim` occurrence is at the clause's name.

use std::collections::BTreeMap;

use ix_trace_rs::string_edge;
use quire_exact::{Identifier, NodeKey, ValueType};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use quire_semantic_value::location::Location;

use super::temporal::{stage_fault, FormulaCondition, FormulaInput};
use super::{Binder, Binders, Lowering};
use crate::check::abstraction::{AbstractionKey, CheckedAbstractionRelation};
use crate::check::family::OccurrenceRole;
use crate::check::hyper_clause::lowering::{hyper_location, Site};
use crate::check::hyper_clause::{CheckedHyperClause, CheckedModelRelation, MatchPart};
use crate::check::node_key::{
    AggregateTerm, Binding, BindingValue, BodyTerm, GroupMember, GroupTerm, LawRole, LeafTerm,
    MemberTerm, NodeTag, Operation, OperationLaw, Operator, TupleMember, TupleTerm,
};
use crate::check::refusal::CheckRefusal;

/// One checked hyper clause over `behaviours`, as lowering reads it.
pub(crate) struct HyperClauseInput<'a> {
    /// The clause's index among the unit's hyper clauses over `behaviours`.
    pub(crate) index: usize,
    /// The checked clause.
    pub(crate) clause: &'a CheckedHyperClause,
    /// The `operation_anchor` node of each `align skip` operation, in the
    /// order written.
    pub(crate) align_skip: &'a [NodeKey],
}

/// One checked FR-180 step relation and the already keyed nodes it joins.
pub(crate) struct StepRelationInput<'a> {
    pub(crate) index: usize,
    pub(crate) relation: &'a CheckedModelRelation,
    pub(crate) anchors: &'a [super::AnchorInput<'a>],
    pub(crate) subjects: &'a [(String, NodeKey)],
    pub(crate) state_forms: &'a [crate::check::state_clause::StateClauseDeclaration],
    pub(crate) state_nodes: &'a [Option<super::LoweredClause>],
}

/// A lowered hyper clause's keys.
#[derive(Clone, Debug)]
pub(crate) struct LoweredHyperClause {
    /// The `claim`/`hyperproperty` node: the clause's identity.
    pub(crate) key: NodeKey,
}

/// A lowered FR-180 `claim`/`step_relation` node.
#[derive(Clone, Debug)]
pub(crate) struct LoweredStepRelation {
    /// The Boolean type node shared by the claim and its expression.
    pub(crate) boolean: NodeKey,
    /// The roots of FR-180's operation-contract extent, receiver then
    /// arguments for each execution, excluding any result binder.
    pub(crate) code_roots: Vec<NodeKey>,
    /// The closed claim body, pending its checked abstraction node reference.
    body: BodyTerm,
    /// The source declaration identifier.
    declaration: Identifier,
    /// The source owner included in its identity.
    owner: super::SourceOwner,
    /// The relation's claim location.
    location: Location,
    /// Whether each execution has a bound abstraction frame.
    bound_implementations: Vec<bool>,
}

impl Lowering<'_> {
    /// Lower and key a checked FR-180 step relation using its checked body,
    /// selected model roots, operation anchors, state clauses, and optional
    /// abstraction relation.
    pub(crate) fn step_relation(
        &mut self,
        input: &StepRelationInput<'_>,
    ) -> Result<LoweredStepRelation, CheckRefusal> {
        let relation = input.relation;
        if input.anchors.len() != relation.executions().len() {
            return Err(stage_fault(
                &hyper_location(input.index, Site::Relation(input.index)),
                "execution-anchor-count-matches",
            ));
        }
        let claim = hyper_location(input.index, Site::Relation(input.index));
        let boolean = self.type_node(&ValueType::Boolean, &claim)?;
        let mut subjects = Vec::with_capacity(input.subjects.len());
        for (alias, model) in input.subjects {
            subjects.push(GroupMember::Binding(Binding::new(
                alias.clone(),
                LeafTerm::reference(model.clone()),
            )));
        }

        let mut all_parameters = Vec::new();
        let mut code_roots = Vec::new();
        let mut execution_terms = Vec::with_capacity(relation.executions().len());
        let mut next_level = 0usize;
        for (((variable, operation), alias), anchor) in relation
            .executions()
            .iter()
            .zip(relation.execution_aliases())
            .zip(input.anchors)
        {
            // FR-180's relation span is the source owner for generated
            // anchor and binder nodes. It is a non-empty source region and
            // remains stable when expression-local body locations differ.
            let at = claim.clone();
            if operation.operation.as_str() != anchor.operation.name() {
                return Err(stage_fault(&at, "execution-anchor-names-operation"));
            }
            let binding = self.operation_anchor(anchor, None, &at)?;
            self.record(binding.anchor.clone(), OccurrenceRole::Anchor, at.clone());
            let mut parameters = Vec::new();
            let receiver_type = ValueType::Reference(anchor.declaring.clone());
            let mut parameter_specs = vec![("self".to_owned(), receiver_type)];
            if let Some(value_type) = anchor.operation.result() {
                parameter_specs.push(("result".to_owned(), value_type.clone()));
            }
            parameter_specs.extend(anchor.operation.parameters().iter().cloned());
            let has_result = anchor.operation.result().is_some();
            for (parameter_index, (name, value_type)) in parameter_specs.into_iter().enumerate() {
                let parameter_at = claim.clone();
                let semantic_type = self.binder_type(&value_type, None, &parameter_at)?;
                let key =
                    self.parameter(&name, next_level, semantic_type.clone(), &parameter_at)?;
                self.record(semantic_type, OccurrenceRole::Type, parameter_at.clone());
                self.record_binder(
                    super::BinderSite {
                        binder: parameter_at.clone(),
                        slot: next_level,
                    },
                    key.clone(),
                );
                parameters.push(LeafTerm::reference(key.clone()));
                all_parameters.push(key.clone());
                if parameter_index == 0 || (parameter_index > 1 || !has_result) {
                    code_roots.push(key);
                }
                next_level += 1;
            }
            let mut preconditions = Vec::new();
            let mut postconditions = Vec::new();
            for (form, node) in input.state_forms.iter().zip(input.state_nodes) {
                let Some(node) = node else { continue };
                let Some(selected) = form.operation.as_ref() else {
                    continue;
                };
                if form.selection.alias != *alias
                    || selected.declaring != anchor.declaring
                    || selected.declaration.name() != anchor.operation.name()
                {
                    continue;
                }
                match form.kind {
                    qsl_forms::StateClauseKind::Precondition => {
                        preconditions.push(node.key.clone())
                    }
                    qsl_forms::StateClauseKind::Postcondition => {
                        postconditions.push(node.key.clone())
                    }
                    qsl_forms::StateClauseKind::Invariant
                    | qsl_forms::StateClauseKind::Constraint => {}
                }
            }
            preconditions.sort();
            postconditions.sort();
            let _ = operation;
            let implementation = BindingValue::Group(GroupTerm::new(Vec::new()));
            let alias = self.text_literal(alias, &at)?;
            execution_terms.push(MemberTerm::binding(
                variable.clone(),
                BindingValue::Aggregate(AggregateTerm::new(vec![
                    MemberTerm::binding("alias", BindingValue::Leaf(alias)),
                    MemberTerm::binding(
                        "operation",
                        BindingValue::Leaf(LeafTerm::reference(binding.anchor)),
                    ),
                    MemberTerm::binding(
                        "parameters",
                        BindingValue::Group(GroupTerm::new(
                            parameters.into_iter().map(GroupMember::Leaf).collect(),
                        )),
                    ),
                    MemberTerm::binding(
                        "preconditions",
                        BindingValue::Group(GroupTerm::references(preconditions)),
                    ),
                    MemberTerm::binding(
                        "postconditions",
                        BindingValue::Group(GroupTerm::references(postconditions)),
                    ),
                    MemberTerm::binding("implementation", implementation),
                ])),
            ));
        }

        let mut invariant_terms = Vec::with_capacity(input.subjects.len());
        for (alias, _) in input.subjects {
            let mut invariants = Vec::new();
            for (form, node) in input.state_forms.iter().zip(input.state_nodes) {
                let Some(node) = node else { continue };
                if form.kind == qsl_forms::StateClauseKind::Invariant
                    && form.selection.alias == *alias
                {
                    invariants.push(node.key.clone());
                }
            }
            invariants.sort();
            invariant_terms.push(MemberTerm::binding(
                alias.clone(),
                BindingValue::Group(GroupTerm::references(invariants)),
            ));
        }

        let mut binders_scope = Vec::with_capacity(all_parameters.len());
        for (slot, parameter) in all_parameters.iter().cloned().enumerate() {
            let semantic_type = self
                .graph
                .node(parameter.clone())
                .and_then(|node| node.content.semantic_type.clone())
                .ok_or_else(|| stage_fault(&claim, "step-relation-parameter-has-type"))?;
            binders_scope.push(Binder {
                slot,
                parameter,
                semantic_type,
            });
        }
        let body_slot_names: Vec<String> = relation
            .body_parameters()
            .iter()
            .map(|(name, _)| name.clone())
            .collect();
        let mut binders = Binders::new(&body_slot_names, &binders_scope);
        let body = self.expression(relation.typed_body().root(), &mut binders)?;
        let declaration = Identifier::new(relation.name().to_owned())
            .map_err(|_| stage_fault(&claim, "step-relation-name-is-identifier"))?;
        let body = BodyTerm::aggregate(vec![
            MemberTerm::binding("subjects", BindingValue::Group(GroupTerm::new(subjects))),
            MemberTerm::binding(
                "executions",
                BindingValue::Aggregate(AggregateTerm::new(execution_terms)),
            ),
            MemberTerm::binding(
                "invariants",
                BindingValue::Aggregate(AggregateTerm::new(invariant_terms)),
            ),
            MemberTerm::bound("body", body),
        ]);
        Ok(LoweredStepRelation {
            boolean,
            code_roots,
            body,
            declaration,
            owner: self.owner.clone(),
            location: claim,
            bound_implementations: Vec::new(),
        })
    }

    /// Lower and key the hyper clause `input`, recording its one `claim`
    /// occurrence at its declared name and the body wrapper's at
    /// `hyper <c> body`.
    pub(crate) fn hyper_clause(
        &mut self,
        input: &HyperClauseInput<'_>,
    ) -> Result<LoweredHyperClause, CheckRefusal> {
        let clause = input.clause;
        let at = |site| hyper_location(input.index, site);
        let claim = at(Site::Claim);
        let boolean = self.type_node(&ValueType::Boolean, &claim)?;

        // The atom slots: one `<p>@<t>` parameter node per slot, at its own
        // index as its level.
        let mut scope: Vec<Binder> = Vec::with_capacity(clause.atom_slots().len());
        for (level, slot) in clause.atom_slots().iter().enumerate() {
            let site = at(Site::Slot(level));
            let object = clause
                .parameter_objects()
                .get(slot.parameter)
                .ok_or_else(|| stage_fault(&site, "slot-parameter-has-an-object"))?;
            let parameter = clause
                .parameters()
                .get(slot.parameter)
                .ok_or_else(|| stage_fault(&site, "slot-has-a-parameter"))?;
            let trace = clause
                .quantifiers()
                .get(slot.trace)
                .ok_or_else(|| stage_fault(&site, "slot-has-a-trace"))?;
            let name = format!("slot_s{}_s{}", esc(&parameter.name), esc(&trace.variable));
            let semantic_type =
                self.binder_type(&ValueType::Reference(object.clone()), None, &site)?;
            let node = self.parameter(&name, level, semantic_type.clone(), &site)?;
            self.record(semantic_type.clone(), OccurrenceRole::Type, site);
            scope.push(Binder {
                slot: level,
                parameter: node,
                semantic_type,
            });
        }
        let over = scope
            .first()
            .map(|binder| binder.parameter.clone())
            .ok_or_else(|| stage_fault(&claim, "clause-has-an-atom-slot"))?;

        // The body wrapper over the clause's formula.
        let conditions: BTreeMap<usize, FormulaCondition<'_>> = clause
            .atom_conditions()
            .iter()
            .map(|(index, condition)| {
                (
                    *index,
                    FormulaCondition {
                        root: condition.root(),
                        slot_names: &condition.slot_names,
                    },
                )
            })
            .collect();
        let wrapper_at = at(Site::Body);
        let formula = self.temporal_formula(
            &FormulaInput {
                formula: clause.body(),
                node_site: &|node| hyper_location(input.index, Site::Node(node)),
                conditions: &conditions,
                root_site: &wrapper_at,
            },
            boolean.clone(),
            &scope,
        )?;
        let law = OperationLaw {
            role: LawRole::TemporalProfile,
            definition: clause.profile().definition(),
        };
        let kind = self.text_literal("origin", &wrapper_at)?;
        let clock = self.text_literal("model-steps", &wrapper_at)?;
        let wrapper = self.insert(
            &wrapper_at,
            NodeTag::Temporal,
            "temporal_clause",
            Some(boolean.clone()),
            None,
            BodyTerm::application(
                Operator::Temporal,
                Operation {
                    laws: vec![law],
                    ..Operation::plain("quire.op.temporal.clause")
                },
                boolean.clone(),
                vec![
                    MemberTerm::reference(over),
                    MemberTerm::Leaf(clock),
                    MemberTerm::Group(GroupTerm::new(vec![GroupMember::Binding(Binding::new(
                        "kind", kind,
                    ))])),
                    MemberTerm::Group(GroupTerm::new(Vec::new())),
                    MemberTerm::Group(GroupTerm::new(Vec::new())),
                    MemberTerm::reference(formula),
                ],
            ),
        )?;
        self.record(wrapper.clone(), OccurrenceRole::Claim, wrapper_at);

        // The seven bindings of QSpec FR-402, in order.
        let domain = self.text_literal("behaviours", &claim)?;
        let subjects = self.subjects(clause, &claim)?;
        let mut parameters = Vec::with_capacity(clause.parameters().len());
        for (parameter, object) in clause.parameters().iter().zip(clause.parameter_objects()) {
            let node = self.object_node(object.clone(), &claim)?;
            parameters.push(GroupMember::Binding(Binding::new(
                parameter.name.clone(),
                LeafTerm::reference(node),
            )));
        }
        let quantifiers = self.quantifiers(clause, &claim)?;
        let conjuncts = self.conjuncts(input)?;
        let align_skip = GroupTerm::references(input.align_skip.iter().cloned());
        let key = self.insert(
            &claim,
            NodeTag::Claim,
            "hyperproperty",
            Some(boolean),
            None,
            BodyTerm::aggregate(vec![
                MemberTerm::bound("domain", domain),
                MemberTerm::binding("subjects", BindingValue::Group(subjects)),
                MemberTerm::binding(
                    "parameters",
                    BindingValue::Group(GroupTerm::new(parameters)),
                ),
                MemberTerm::binding("quantifiers", BindingValue::Tuple(quantifiers)),
                MemberTerm::binding("match", BindingValue::Group(conjuncts)),
                MemberTerm::binding("align_skip", BindingValue::Group(align_skip)),
                MemberTerm::bound("body", LeafTerm::reference(wrapper)),
            ]),
        )?;
        self.record(key.clone(), OccurrenceRole::Claim, claim);
        Ok(LoweredHyperClause { key })
    }

    /// `subjects`: one `binding(alias, reference)` per alias in order of
    /// first naming, to the model node of the object type of the alias's
    /// first object parameter (FR-488 Q-2).
    fn subjects(
        &mut self,
        clause: &CheckedHyperClause,
        claim: &Location,
    ) -> Result<GroupTerm, CheckRefusal> {
        let mut members = Vec::with_capacity(clause.aliases().len());
        for alias in clause.aliases() {
            let object = clause
                .parameters()
                .iter()
                .zip(clause.parameter_objects())
                .find(|(parameter, _)| &parameter.alias == alias)
                .map(|(_, object)| object.clone())
                .ok_or_else(|| stage_fault(claim, "alias-binds-a-parameter"))?;
            let node = self.object_node(object, claim)?;
            members.push(GroupMember::Binding(Binding::new(
                alias.as_str(),
                LeafTerm::reference(node),
            )));
        }
        Ok(GroupTerm::new(members))
    }

    /// `quantifiers`: a Tuple of Groups, one quantifier Group per quantifier
    /// in source order, each followed by a fairness Group per constraint of
    /// its resolved fairness set.
    fn quantifiers(
        &mut self,
        clause: &CheckedHyperClause,
        claim: &Location,
    ) -> Result<TupleTerm, CheckRefusal> {
        let mut members = Vec::new();
        for quantifier in clause.quantifiers() {
            let text = |lowering: &mut Self, value: &str| lowering.text_literal(value, claim);
            let kind = text(self, "quantifier")?;
            let variable = text(self, &quantifier.variable)?;
            let spelling = if quantifier.is_universal() {
                "forall"
            } else {
                "exists"
            };
            let spelling = text(self, spelling)?;
            let alias = text(self, quantifier.alias.as_str())?;
            members.push(TupleMember::Group(GroupTerm::new(vec![
                GroupMember::Binding(Binding::new("kind", kind)),
                GroupMember::Binding(Binding::new("variable", variable)),
                GroupMember::Binding(Binding::new("quantifier", spelling)),
                GroupMember::Binding(Binding::new("alias", alias)),
            ])));
            for constraint in &quantifier.fairness {
                let operation = constraint.operation();
                let declaration = self.object_node(operation.declaring_type(), claim)?;
                Identifier::new(operation.name().to_owned())
                    .map_err(|_| stage_fault(claim, "fairness-operation-named"))?;
                let kind = text(self, "fairness")?;
                let variable = text(self, &quantifier.variable)?;
                let fairness_kind = text(
                    self,
                    match constraint.kind() {
                        FairnessKind::Weak => "weak",
                        FairnessKind::Strong => "strong",
                    },
                )?;
                let granularity = text(
                    self,
                    match constraint.granularity() {
                        FairnessGranularity::Whole => "whole",
                        FairnessGranularity::Each => "each",
                    },
                )?;
                let name = text(self, operation.name())?;
                members.push(TupleMember::Group(GroupTerm::new(vec![
                    GroupMember::Binding(Binding::new("kind", kind)),
                    GroupMember::Binding(Binding::new("variable", variable)),
                    GroupMember::Binding(Binding::new("fairness_kind", fairness_kind)),
                    GroupMember::Binding(Binding::new("granularity", granularity)),
                    GroupMember::Binding(Binding::new(
                        "declaration",
                        LeafTerm::reference(declaration),
                    )),
                    GroupMember::Binding(Binding::new("name", name)),
                ])));
            }
        }
        Ok(TupleTerm::new(members))
    }

    /// `match`: one term per conjunct in source order, each its lowered
    /// condition over its step-label slot nodes.
    fn conjuncts(&mut self, input: &HyperClauseInput<'_>) -> Result<GroupTerm, CheckRefusal> {
        let clause = input.clause;
        let mut members = Vec::new();
        for (part, index) in clause.conjuncts() {
            let (conjunct, site, slot_site): (_, Site, &dyn Fn(usize) -> Site) = match part {
                MatchPart::Universal => (
                    clause.mu_universal().get(index),
                    Site::Universal(index),
                    &move |slot| Site::UniversalSlot(index, slot),
                ),
                MatchPart::Existential => (
                    clause.mu_existential().get(index),
                    Site::Existential(index),
                    &move |slot| Site::ExistentialSlot(index, slot),
                ),
            };
            let site = hyper_location(input.index, site);
            let conjunct = conjunct.ok_or_else(|| stage_fault(&site, "conjunct-in-range"))?;
            let lowered = conjunct.lowered();
            let reads: Vec<_> = conjunct
                .slots()
                .iter()
                .filter(|read| read.slot_name().is_some())
                .collect();
            let mut scope: Vec<Binder> = Vec::with_capacity(reads.len());
            for (level, read) in reads.into_iter().enumerate() {
                let slot_at = hyper_location(input.index, slot_site(level));
                let name =
                    read_slot_name(read).ok_or_else(|| stage_fault(&slot_at, "slot-has-a-name"))?;
                let slot_type = read
                    .slot_type()
                    .ok_or_else(|| stage_fault(&slot_at, "slot-has-a-type"))?;
                let semantic_type = self.binder_type(&slot_type, None, &slot_at)?;
                let parameter = self.parameter(&name, level, semantic_type.clone(), &slot_at)?;
                self.record(semantic_type.clone(), OccurrenceRole::Type, slot_at);
                scope.push(Binder {
                    slot: level,
                    parameter,
                    semantic_type,
                });
            }
            let mut binders = Binders::new(&lowered.slot_names, &scope);
            let term = self.expression(lowered.body.root(), &mut binders)?;
            members.push(GroupMember::Leaf(term));
        }
        Ok(GroupTerm::new(members))
    }
}

/// Finish the step-relation node after the checked abstraction relation is
/// available. The relation key is late-bound because QSL computes it from
/// the assembled graph's exact model declaration nodes.
#[string_edge]
pub(crate) fn finish_step_relation(
    mut lowered: LoweredStepRelation,
    relation: &CheckedModelRelation,
    abstraction: Option<&CheckedAbstractionRelation>,
    abstraction_node: Option<NodeKey>,
    identity: qsl_foundation::IdentityLimits,
) -> Result<(super::SemanticNode, Vec<NodeKey>), CheckRefusal> {
    let code_roots = lowered.code_roots.clone();
    lowered.bound_implementations = relation
        .executions()
        .iter()
        .map(|(_, operation)| {
            abstraction.is_some_and(|relation| {
                relation
                    .get(&AbstractionKey::Frame(operation.clone()))
                    .is_some()
            })
        })
        .collect();
    if lowered.bound_implementations.iter().any(|bound| *bound) && abstraction_node.is_none() {
        return Err(stage_fault(
            &lowered.location,
            "bound-step-relation-has-abstraction-node",
        ));
    }
    let BodyTerm::Aggregate(body) = &mut lowered.body else {
        return Err(stage_fault(
            &lowered.location,
            "step-relation-body-is-aggregate",
        ));
    };
    let executions = body
        .members
        .get_mut(1)
        .and_then(|member| match member {
            MemberTerm::Binding(binding) if binding.name == "executions" => {
                match &mut binding.value {
                    BindingValue::Aggregate(aggregate) => Some(aggregate),
                    _ => None,
                }
            }
            _ => None,
        })
        .ok_or_else(|| stage_fault(&lowered.location, "step-relation-executions-aggregate"))?;
    if executions.members.len() != lowered.bound_implementations.len() {
        return Err(stage_fault(
            &lowered.location,
            "step-relation-implementation-count-matches",
        ));
    }
    for (execution, bound) in executions
        .members
        .iter_mut()
        .zip(&lowered.bound_implementations)
    {
        if !bound {
            continue;
        }
        let MemberTerm::Binding(entry) = execution else {
            return Err(stage_fault(
                &lowered.location,
                "step-relation-execution-is-binding",
            ));
        };
        let BindingValue::Aggregate(entry) = &mut entry.value else {
            return Err(stage_fault(
                &lowered.location,
                "step-relation-execution-value-is-aggregate",
            ));
        };
        let Some(MemberTerm::Binding(implementation)) = entry.members.get_mut(5) else {
            return Err(stage_fault(
                &lowered.location,
                "step-relation-implementation-binding-exists",
            ));
        };
        if implementation.name != "implementation" {
            return Err(stage_fault(
                &lowered.location,
                "step-relation-implementation-is-last",
            ));
        }
        implementation.value = BindingValue::Leaf(LeafTerm::reference(
            abstraction_node.clone().ok_or_else(|| {
                stage_fault(
                    &lowered.location,
                    "bound-step-relation-has-abstraction-node",
                )
            })?,
        ));
    }
    let content = super::NodeContent {
        owner_model: None,
        node_tag: NodeTag::Claim,
        semantic_form: "step_relation",
        semantic_type: Some(lowered.boolean),
        declaration: Some(vec![lowered.declaration]),
        owner: Some(super::Owner::Source(lowered.owner)),
        body: lowered.body,
    };
    let keyed = super::node_key(&content.input(), identity)
        .map_err(|refusal| super::preimage_refusal(&lowered.location, refusal))?;
    Ok((
        super::SemanticNode {
            key: keyed.key,
            preimage: keyed.preimage,
            recursion: None,
            content,
            nominal: None,
        },
        code_roots,
    ))
}

/// `esc(x)` of FR-488 "Slot node names": each UTF-8 byte of `x` that is an
/// ASCII letter or digit as itself, `_` as `__`, any other byte as `_x` and
/// two uppercase hexadecimal digits.
pub(crate) fn esc(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'_' => escaped.push_str("__"),
            byte if byte.is_ascii_alphanumeric() => escaped.push(char::from(byte)),
            byte => escaped.push_str(&format!("_x{byte:02X}")),
        }
    }
    escaped
}

/// The identifier-safe name of the parameter node of a conjunct's step-label
/// read (FR-488 "Slot node names"); `None` for `stutter`, which has no slot,
/// and for an argument read whose operation is not written `T::name`.
pub(crate) fn read_slot_name(read: &crate::check::hyper_clause::StepRead) -> Option<String> {
    use crate::check::hyper_clause::StepRead;
    match read {
        StepRead::Operation { trace } => Some(format!("op_s{}", esc(trace))),
        StepRead::Receiver { trace, .. } => Some(format!("rcv_s{}", esc(trace))),
        StepRead::Argument(argument) => {
            let (object, name) = argument.written.rsplit_once("::")?;
            Some(format!(
                "arg_s{}_s{}_s{}_s{}",
                esc(&argument.trace),
                esc(object),
                esc(name),
                esc(&argument.parameter)
            ))
        }
        StepRead::Stutter => None,
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::esc;

    /// FR-488-AC-23: `esc` is the byte-wise encoding of FR-488 "Slot node
    /// names", and `slot_s<p>_s<t>` is injective over underscores, `s`, `x`,
    /// digits and non-ASCII text.
    #[trace("TC-1266", "FR-488-AC-23")]
    #[test]
    fn slot_names_are_identifier_safe_and_injective() {
        assert_eq!(esc("a_b"), "a__b");
        assert_eq!(esc("é"), "_xC3_xA9");
        assert_eq!(esc("a.b"), "a_x2Eb");
        let fields = [
            "a", "a_b", "b", "_", "s", "x", "7", "é", "a_sb", "_sa", "", "a__b",
        ];
        let mut seen = std::collections::BTreeMap::new();
        for p in fields {
            for t in fields {
                let name = format!("slot_s{}_s{}", esc(p), esc(t));
                assert!(
                    name.bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
                    "{name}"
                );
                assert_eq!(seen.insert(name.clone(), (p, t)), None, "{name}");
            }
        }
        assert_eq!(
            format!("slot_s{}_s{}", esc("a"), esc("b_c")),
            "slot_sa_sb__c"
        );
        assert_eq!(
            format!("slot_s{}_s{}", esc("a_b"), esc("c")),
            "slot_sa__b_sc"
        );
    }
}

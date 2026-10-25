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

use quire_exact::{Identifier, NodeKey, ValueType};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};
use quire_semantic_value::location::Location;

use super::temporal::{stage_fault, FormulaCondition, FormulaInput};
use super::{Binder, Binders, Lowering};
use crate::check::family::OccurrenceRole;
use crate::check::hyper_clause::lowering::{hyper_location, Site};
use crate::check::hyper_clause::{CheckedHyperClause, MatchPart};
use crate::check::node_key::{
    Binding, BindingValue, BodyTerm, GroupMember, GroupTerm, LawRole, LeafTerm, MemberTerm,
    NodeTag, Operation, OperationLaw, Operator, TupleMember, TupleTerm,
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

/// A lowered hyper clause's keys.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoweredHyperClause {
    /// The `claim`/`hyperproperty` node: the clause's identity.
    pub(crate) key: NodeKey,
}

impl Lowering<'_> {
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
            let semantic_type = self.binder_type(&ValueType::Reference(*object), None, &site)?;
            let node = self.parameter(&name, level, semantic_type, &site)?;
            self.record(semantic_type, OccurrenceRole::Type, site);
            scope.push(Binder {
                slot: level,
                parameter: node,
                semantic_type,
            });
        }
        let over = scope
            .first()
            .map(|binder| binder.parameter)
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
            boolean,
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
            Some(boolean),
            None,
            BodyTerm::application(
                Operator::Temporal,
                Operation {
                    laws: vec![law],
                    ..Operation::plain("quire.op.temporal.clause")
                },
                boolean,
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
        self.record(wrapper, OccurrenceRole::Claim, wrapper_at);

        // The seven bindings of QSpec FR-402, in order.
        let domain = self.text_literal("behaviours", &claim)?;
        let subjects = self.subjects(clause, &claim)?;
        let mut parameters = Vec::with_capacity(clause.parameters().len());
        for (parameter, object) in clause.parameters().iter().zip(clause.parameter_objects()) {
            let node = self.object_node(*object, &claim)?;
            parameters.push(GroupMember::Binding(Binding::new(
                parameter.name.clone(),
                LeafTerm::reference(node),
            )));
        }
        let quantifiers = self.quantifiers(clause, &claim)?;
        let conjuncts = self.conjuncts(input)?;
        let align_skip = GroupTerm::references(input.align_skip.iter().copied());
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
        self.record(key, OccurrenceRole::Claim, claim);
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
                .map(|(_, object)| *object)
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
                let parameter = self.parameter(&name, level, semantic_type, &slot_at)?;
                self.record(semantic_type, OccurrenceRole::Type, slot_at);
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

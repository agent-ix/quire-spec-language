// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-447 "S4 emission", QSpec FR-481: the `hybrid` nodes each checked hybrid
//! model lowers to, the term nodes under them, and the
//! `temporal`/`temporal_clause` node each hybrid claim lowers to.
//!
//! A model is one `hybrid`/`hybrid_model` node, one `hybrid`/`hybrid_mode`
//! node per mode and one `hybrid`/`hybrid_transition` node per transition,
//! each continuous variable a `value`/`parameter` node, and every invariant,
//! rate, guard, reset value and initial condition its own term node reached
//! by `reference`, so each body keeps FR-322's fixed body grammar whatever
//! the term's depth. A `hybrid` node holds no application, so it is keyed by
//! the structural preimage (FR-481 "Every `hybrid` node"); a transition has
//! neither `declaration` nor `owner`, so two transitions with equal content
//! are one node.
//!
//! A claim is the FR-370 clause of the temporal lowering with the hybrid
//! arguments FR-481 "Hybrid claims" gives it: an `each` activation with its
//! trigger and guard, captures, hybrid fairness members, past operators and
//! the interval form its profile admits.
//!
//! A construct carries no position of its own beyond its declaration, so
//! each node is located at an `Origin::TypeDeclaration` site ([`hybrid_site`]);
//! every term node of a construct is located at that construct's site.

use quire_exact::{Identifier, Integer, NodeKey, ValueType};
use quire_semantic_value::location::{Location, Origin};

use super::{fault, Lowering};
use crate::check::family::OccurrenceRole;
use crate::check::hybrid::{
    ArithmeticOp, CheckedHybridClaim, CheckedHybridModel, CompareOp, HybridActivation, HybridClock,
    HybridFairness, HybridFormula, HybridInterval, HybridTerm, LogicOp,
};
use crate::check::node_key::{
    Binding, BindingValue, BodyTerm, GroupMember, GroupTerm, LawRole, LeafTerm, LiteralValue,
    MemberTerm, NodeTag, Operation, OperationLaw, Operator, TupleMember, TupleTerm,
};
use crate::check::profile::TemporalProfile;
use crate::check::refusal::{CheckRefusal, KeyFault};
use crate::check::Signature;
use crate::value::member::{Member, TimedMember};
use qsl_foundation::bound::{TemporalInterval, TemporalUpper};

/// The stage name this module's faults report.
const STAGE: &str = "lowering.hybrid";

/// One place of a hybrid model or claim an occurrence or a refusal is
/// located at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HybridSite {
    /// The model at this index among the unit's hybrid declarations: its
    /// declared name.
    Model(usize),
    /// The mode at this index of the model at this index: the `mode` clause.
    Mode(usize, usize),
    /// The transition at this index of the mode at this index of the model
    /// at this index: the `transition` clause.
    Transition(usize, usize, usize),
    /// The claim at this index among the unit's hybrid claims: its declared
    /// name.
    Claim(usize),
    /// The `over` parameter of the claim at this index.
    Over(usize),
}

/// The `Origin::TypeDeclaration` name of `site`. Spaced, so it names no
/// declared type.
pub(crate) fn hybrid_site(site: HybridSite) -> String {
    match site {
        HybridSite::Model(model) => format!("hybrid {model}"),
        HybridSite::Mode(model, mode) => format!("hybrid {model} mode {mode}"),
        HybridSite::Transition(model, mode, transition) => {
            format!("hybrid {model} mode {mode} transition {transition}")
        }
        HybridSite::Claim(claim) => format!("hybrid claim {claim}"),
        HybridSite::Over(claim) => format!("hybrid claim {claim} over"),
    }
}

/// The root location of `site`.
pub(crate) fn hybrid_location(site: HybridSite) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: hybrid_site(site),
    })
}

/// One checked hybrid model, as lowering reads it.
pub(crate) struct HybridModelInput<'a> {
    /// The model's index among the unit's hybrid declarations.
    pub(crate) index: usize,
    /// The checked model.
    pub(crate) model: &'a CheckedHybridModel,
    /// Every function's signature, by its position in the function list.
    pub(crate) signatures: &'a [Signature],
}

/// A lowered hybrid model's keys.
#[derive(Clone, Debug)]
pub(crate) struct LoweredHybridModel {
    /// The `hybrid`/`hybrid_model` node.
    pub(crate) key: NodeKey,
    /// Each variable's `value`/`parameter` node.
    pub(crate) variables: Vec<NodeKey>,
    /// Each mode's `hybrid`/`hybrid_mode` node.
    pub(crate) modes: Vec<NodeKey>,
}

/// A lowered hybrid claim's keys.
pub(crate) struct LoweredHybridClaim {
    /// The `temporal`/`temporal_clause` node.
    pub(crate) key: NodeKey,
    /// The claim's `Boolean` result type node.
    pub(crate) boolean: NodeKey,
}

/// One checked hybrid claim, as lowering reads it.
pub(crate) struct HybridClaimInput<'a> {
    /// The claim's index among the unit's hybrid claims.
    pub(crate) index: usize,
    /// The checked claim.
    pub(crate) claim: &'a CheckedHybridClaim,
    /// The model the claim is over.
    pub(crate) model: &'a CheckedHybridModel,
    /// The model's lowered keys.
    pub(crate) lowered: &'a LoweredHybridModel,
    /// Every function's signature, by its position in the function list.
    pub(crate) signatures: &'a [Signature],
}

/// The kind of value a term node denotes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TermKind {
    Real,
    Boolean,
}

/// What the terms of one construct read: the model's variables, and for a
/// claim the activation reads and the captures.
struct TermScope<'s> {
    variables: &'s [NodeKey],
    variable_names: Vec<&'s Identifier>,
    claim: Option<&'s CheckedHybridClaim>,
    signatures: &'s [Signature],
}

impl TermScope<'_> {
    /// The level of the `i`-th activation read: `n + i` for `n` variables.
    fn activation_level(&self, index: usize) -> usize {
        self.variables.len() + index
    }

    /// The level of the `j`-th capture read: `2n + j`.
    fn capture_level(&self, index: usize) -> usize {
        2 * self.variables.len() + index
    }
}

/// One step of the work stack that lowers a term.
enum TermStep<'t> {
    Visit(&'t HybridTerm),
    Build(&'t HybridTerm),
}

impl Lowering<'_> {
    /// Lower and key the hybrid model `input`, recording the `declaration`
    /// occurrence of its model and mode nodes and an `expression` occurrence
    /// of each transition at its clause.
    pub(crate) fn hybrid_model(
        &mut self,
        input: &HybridModelInput<'_>,
    ) -> Result<LoweredHybridModel, CheckRefusal> {
        let model = input.model;
        let at = hybrid_location;
        let declared = at(HybridSite::Model(input.index));
        let rational = self.scalar("rational", &declared)?;

        let mut variables = Vec::with_capacity(model.variables.len());
        for (position, variable) in model.variables.iter().enumerate() {
            variables.push(self.parameter(
                variable.name.as_str(),
                position,
                rational,
                &declared,
            )?);
        }
        let scope = TermScope {
            variables: &variables,
            variable_names: model.variables.iter().map(|v| &v.name).collect(),
            claim: None,
            signatures: input.signatures,
        };

        let mut modes = Vec::with_capacity(model.modes.len());
        for (mode_position, mode) in model.modes.iter().enumerate() {
            let mode_at = at(HybridSite::Mode(input.index, mode_position));
            let invariant = self.hybrid_term(&mode.invariant, &scope, &mode_at)?;
            let mut rates = Vec::with_capacity(mode.rates.len());
            for rate in &mode.rates {
                rates.push(TupleMember::Leaf(self.hybrid_term(rate, &scope, &mode_at)?));
            }
            let mut transitions = Vec::with_capacity(mode.transitions.len());
            for (position, transition) in mode.transitions.iter().enumerate() {
                let transition_at =
                    at(HybridSite::Transition(input.index, mode_position, position));
                let guard = self.hybrid_term(&transition.guard, &scope, &transition_at)?;
                let mut resets = Vec::with_capacity(transition.resets.len());
                for reset in &transition.resets {
                    let value = self.hybrid_term(&reset.value, &scope, &transition_at)?;
                    let mut members = vec![
                        GroupMember::Binding(Binding::new(
                            "variable",
                            self.integer_literal(
                                Integer::from(reset.variable.get()),
                                &transition_at,
                            )?,
                        )),
                        GroupMember::Binding(Binding::new("value", value)),
                    ];
                    if reset.relation != CompareOp::Eq {
                        let relation =
                            self.text_literal(relation_spelling(reset.relation)?, &transition_at)?;
                        members.push(GroupMember::Binding(Binding::new("relation", relation)));
                    }
                    resets.push(TupleMember::Group(GroupTerm::new(members)));
                }
                let target =
                    self.integer_literal(Integer::from(transition.target.get()), &transition_at)?;
                let key = self.insert(
                    &transition_at,
                    NodeTag::Hybrid,
                    "hybrid_transition",
                    None,
                    None,
                    BodyTerm::aggregate(vec![
                        MemberTerm::bound("target", target),
                        MemberTerm::bound("guard", guard),
                        MemberTerm::binding("resets", BindingValue::Tuple(TupleTerm::new(resets))),
                    ]),
                )?;
                self.record(key, OccurrenceRole::Expression, transition_at);
                transitions.push(TupleMember::Leaf(LeafTerm::reference(key)));
            }
            let key = self.insert(
                &mode_at,
                NodeTag::Hybrid,
                "hybrid_mode",
                None,
                Some(vec![model.name.clone(), mode.name.clone()]),
                BodyTerm::aggregate(vec![
                    MemberTerm::bound("invariant", invariant),
                    MemberTerm::binding("rates", BindingValue::Tuple(TupleTerm::new(rates))),
                    MemberTerm::binding(
                        "transitions",
                        BindingValue::Tuple(TupleTerm::new(transitions)),
                    ),
                ]),
            )?;
            self.record(key, OccurrenceRole::Declaration, mode_at);
            modes.push(key);
        }

        let mut entries = Vec::with_capacity(model.variables.len());
        for (variable, node) in model.variables.iter().zip(&variables) {
            let class = self.text_literal(class_spelling(variable.class), &declared)?;
            entries.push(TupleMember::Group(GroupTerm::new(vec![
                GroupMember::Binding(Binding::new("parameter", LeafTerm::reference(*node))),
                GroupMember::Binding(Binding::new("class", class)),
            ])));
        }
        let mut members = vec![
            MemberTerm::binding("variables", BindingValue::Tuple(TupleTerm::new(entries))),
            MemberTerm::binding(
                "modes",
                BindingValue::Tuple(TupleTerm::new(
                    modes
                        .iter()
                        .map(|mode| TupleMember::Leaf(LeafTerm::reference(*mode)))
                        .collect(),
                )),
            ),
        ];
        // FR-481: the `initial` binding is omitted exactly when the checked
        // entries are the one default entry.
        let default_initial = matches!(
            model.initial.as_slice(),
            [entry] if entry.mode.get() == 0 && entry.condition == HybridTerm::Boolean(true)
        );
        if !default_initial {
            let mut initial = Vec::with_capacity(model.initial.len());
            for entry in &model.initial {
                let condition = self.hybrid_term(&entry.condition, &scope, &declared)?;
                let mode = self.integer_literal(Integer::from(entry.mode.get()), &declared)?;
                initial.push(TupleMember::Group(GroupTerm::new(vec![
                    GroupMember::Binding(Binding::new("mode", mode)),
                    GroupMember::Binding(Binding::new("condition", condition)),
                ])));
            }
            members.push(MemberTerm::binding(
                "initial",
                BindingValue::Tuple(TupleTerm::new(initial)),
            ));
        }
        let key = self.insert(
            &declared,
            NodeTag::Hybrid,
            "hybrid_model",
            None,
            Some(vec![model.name.clone()]),
            BodyTerm::aggregate(members),
        )?;
        self.record(key, OccurrenceRole::Declaration, declared);
        Ok(LoweredHybridModel {
            key,
            variables,
            modes,
        })
    }

    /// Lower and key the hybrid claim `input` as a
    /// `temporal`/`temporal_clause` node, recording its one `claim`
    /// occurrence at its declared name. Returns the clause node's key and its
    /// `Boolean` result type node.
    pub(crate) fn hybrid_claim(
        &mut self,
        input: &HybridClaimInput<'_>,
    ) -> Result<LoweredHybridClaim, CheckRefusal> {
        let claim = input.claim;
        let at = hybrid_location(HybridSite::Claim(input.index));
        let boolean = self.type_node(&ValueType::Boolean, &at)?;
        let over_at = hybrid_location(HybridSite::Over(input.index));
        let over = self.parameter(claim.over.as_str(), 0, input.lowered.key, &over_at)?;
        let scope = TermScope {
            variables: &input.lowered.variables,
            variable_names: input.model.variables.iter().map(|v| &v.name).collect(),
            claim: Some(claim),
            signatures: input.signatures,
        };

        let kind_name = match claim.activation {
            HybridActivation::Origin => "origin",
            HybridActivation::Each { .. } => "each",
        };
        let kind = self.text_literal(kind_name, &at)?;
        let mut activation = vec![GroupMember::Binding(Binding::new("kind", kind))];
        if let HybridActivation::Each { parameter, guard } = &claim.activation {
            let trigger = self.parameter(parameter.as_str(), 1, input.lowered.key, &at)?;
            activation.push(GroupMember::Binding(Binding::new(
                "trigger",
                LeafTerm::reference(trigger),
            )));
            if let Some(guard) = guard {
                let guard = self.hybrid_term(guard, &scope, &at)?;
                activation.push(GroupMember::Binding(Binding::new("guard", guard)));
            }
        }

        let mut captures = Vec::with_capacity(claim.captures.len());
        for capture in &claim.captures {
            let value = self.hybrid_term(&capture.value, &scope, &at)?;
            captures.push(GroupMember::Binding(Binding::new(
                capture.name.as_str(),
                value,
            )));
        }

        let mut fairness = Vec::with_capacity(claim.fairness.len());
        for constraint in &claim.fairness {
            fairness.push(self.hybrid_fairness(constraint, input, boolean, &at)?);
        }

        let formula = self.hybrid_formula(&claim.formula, &scope, claim, boolean, &at)?;

        let law = OperationLaw {
            role: LawRole::TemporalProfile,
            definition: claim.profile.definition(),
        };
        let clock = self.text_literal(clock_spelling(claim.clock), &at)?;
        let key = self.insert(
            &at,
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
                    MemberTerm::Group(GroupTerm::new(activation)),
                    MemberTerm::Group(GroupTerm::new(captures)),
                    MemberTerm::Group(GroupTerm::references(fairness)),
                    MemberTerm::reference(formula),
                ],
            ),
        )?;
        self.record(key, OccurrenceRole::Claim, at);
        Ok(LoweredHybridClaim { key, boolean })
    }

    /// The `temporal`/`fairness` node of `constraint` (QSpec FR-481 "Hybrid
    /// claims"). A constraint over the transitions of one mode `S` is
    /// written `declaration` the model node and `name` the mode for `fair S`,
    /// or `declaration` the mode node and `name` the target for `fair S::T`.
    /// The checked constraint retains which source form was written.
    fn hybrid_fairness(
        &mut self,
        constraint: &HybridFairness,
        input: &HybridClaimInput<'_>,
        boolean: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let Some((mode, _)) = constraint.transitions.first().copied() else {
            return Err(stage_fault(location, "fairness-names-a-transition"));
        };
        let model = input.model;
        let mode_node = |mode: crate::check::hybrid::ModeIndex| {
            input
                .lowered
                .modes
                .get(mode.get())
                .copied()
                .ok_or_else(|| stage_fault(location, "fairness-mode-lowered"))
        };
        let checked_mode = model
            .modes
            .get(mode.get())
            .ok_or_else(|| stage_fault(location, "fairness-mode-checked"))?;
        let (declaration, name) = if let Some(target) = constraint.named_target {
            let valid_target = constraint.transitions.iter().all(|(owner, position)| {
                *owner == mode
                    && model
                        .modes
                        .get(owner.get())
                        .and_then(|mode| mode.transitions.get(position.get()))
                        .is_some_and(|transition| transition.target == target)
            });
            if !valid_target {
                return Err(stage_fault(location, "fairness-target-checked"));
            }
            let target = model
                .modes
                .get(target.get())
                .ok_or_else(|| stage_fault(location, "fairness-target-checked"))?;
            (mode_node(mode)?, target.name.clone())
        } else {
            (input.lowered.key, checked_mode.name.clone())
        };
        let key = self.insert(
            location,
            NodeTag::Temporal,
            "fairness",
            Some(boolean),
            None,
            BodyTerm::application(
                Operator::TemporalFairness,
                Operation {
                    member: Some(Member::Fairness {
                        kind: constraint.kind,
                        granularity: constraint.granularity,
                        declaration,
                        name,
                    }),
                    ..Operation::plain("quire.op.temporal.fair")
                },
                boolean,
                Vec::new(),
            ),
        )?;
        self.record(key, OccurrenceRole::Expression, location.clone());
        Ok(key)
    }

    /// Key every node of `root`, each operand before its operator, and
    /// return the root's key.
    fn hybrid_formula(
        &mut self,
        root: &HybridFormula,
        scope: &TermScope<'_>,
        claim: &CheckedHybridClaim,
        boolean: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        enum Step<'f> {
            Visit(&'f HybridFormula),
            Build(&'f HybridFormula),
        }
        let mut steps = vec![Step::Visit(root)];
        let mut done: Vec<NodeKey> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                Step::Visit(formula) => match formula {
                    HybridFormula::Holds(HybridTerm::Boolean(value)) => {
                        let identity = if *value {
                            "quire.op.temporal.true"
                        } else {
                            "quire.op.temporal.false"
                        };
                        done.push(self.formula_node(
                            identity,
                            None,
                            Vec::new(),
                            boolean,
                            location,
                        )?);
                    }
                    HybridFormula::Holds(term) => {
                        let operand = self.hybrid_term(term, scope, location)?;
                        done.push(self.formula_node(
                            "quire.op.temporal.holds",
                            None,
                            vec![MemberTerm::Leaf(operand)],
                            boolean,
                            location,
                        )?);
                    }
                    HybridFormula::Not(operand)
                    | HybridFormula::Always(_, operand)
                    | HybridFormula::Eventually(_, operand)
                    | HybridFormula::Historically(_, operand)
                    | HybridFormula::Once(_, operand) => {
                        steps.push(Step::Build(formula));
                        steps.push(Step::Visit(operand));
                    }
                    HybridFormula::Logic(_, left, right)
                    | HybridFormula::Until(_, left, right)
                    | HybridFormula::Release(_, left, right)
                    | HybridFormula::Since(_, left, right)
                    | HybridFormula::Triggered(_, left, right) => {
                        steps.push(Step::Build(formula));
                        steps.push(Step::Visit(right));
                        steps.push(Step::Visit(left));
                    }
                },
                Step::Build(formula) => {
                    let (identity, interval, operands) = match formula {
                        HybridFormula::Not(_) => ("quire.op.temporal.not", None, 1),
                        HybridFormula::Logic(op, ..) => (logic_identity(*op), None, 2),
                        HybridFormula::Always(i, _) => ("quire.op.temporal.always", Some(i), 1),
                        HybridFormula::Eventually(i, _) => {
                            ("quire.op.temporal.eventually", Some(i), 1)
                        }
                        HybridFormula::Historically(i, _) => {
                            ("quire.op.temporal.historically", Some(i), 1)
                        }
                        HybridFormula::Once(i, _) => ("quire.op.temporal.once", Some(i), 1),
                        HybridFormula::Until(i, ..) => ("quire.op.temporal.until", Some(i), 2),
                        HybridFormula::Release(i, ..) => ("quire.op.temporal.release", Some(i), 2),
                        HybridFormula::Since(i, ..) => ("quire.op.temporal.since", Some(i), 2),
                        HybridFormula::Triggered(i, ..) => {
                            ("quire.op.temporal.triggered", Some(i), 2)
                        }
                        HybridFormula::Holds(_) => {
                            return Err(stage_fault(location, "holds-is-a-leaf"))
                        }
                    };
                    let mut arguments = Vec::with_capacity(operands);
                    for _ in 0..operands {
                        let operand = done
                            .pop()
                            .ok_or_else(|| stage_fault(location, "operand-keyed-first"))?;
                        arguments.push(MemberTerm::reference(operand));
                    }
                    arguments.reverse();
                    let member = match interval {
                        Some(interval) => {
                            Some(interval_member(claim.profile, interval.as_ref(), location)?)
                        }
                        None => None,
                    };
                    done.push(self.formula_node(identity, member, arguments, boolean, location)?);
                }
            }
        }
        match done.as_slice() {
            [root] => Ok(*root),
            _ => Err(stage_fault(location, "formula-root-keyed")),
        }
    }

    /// One `temporal`/`formula` node.
    fn formula_node(
        &mut self,
        identity: &'static str,
        member: Option<Member>,
        arguments: Vec<MemberTerm>,
        boolean: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let key = self.insert(
            location,
            NodeTag::Temporal,
            "formula",
            Some(boolean),
            None,
            BodyTerm::application(
                Operator::TemporalFormula,
                Operation {
                    member,
                    ..Operation::plain(identity)
                },
                boolean,
                arguments,
            ),
        )?;
        self.record(key, OccurrenceRole::Expression, location.clone());
        Ok(key)
    }

    /// Lower `root` to term nodes and return the term that names it. A
    /// variable, an activation read and a capture read are references to
    /// their `value`/`parameter` nodes; every other term is its own node.
    fn hybrid_term(
        &mut self,
        root: &HybridTerm,
        scope: &TermScope<'_>,
        location: &Location,
    ) -> Result<LeafTerm, CheckRefusal> {
        let mut steps = vec![TermStep::Visit(root)];
        let mut done: Vec<(LeafTerm, TermKind)> = Vec::new();
        while let Some(step) = steps.pop() {
            match step {
                TermStep::Visit(term) => match term {
                    HybridTerm::Variable(index) => {
                        let node = scope
                            .variables
                            .get(index.get())
                            .ok_or_else(|| stage_fault(location, "variable-lowered"))?;
                        done.push((LeafTerm::reference(*node), TermKind::Real));
                    }
                    HybridTerm::ActivationVariable(index) => {
                        let name = scope
                            .variable_names
                            .get(index.get())
                            .ok_or_else(|| stage_fault(location, "variable-named"))?;
                        let rational = self.scalar("rational", location)?;
                        let node = self.parameter(
                            name.as_str(),
                            scope.activation_level(index.get()),
                            rational,
                            location,
                        )?;
                        done.push((LeafTerm::reference(node), TermKind::Real));
                    }
                    HybridTerm::Capture(index) => {
                        let capture = scope
                            .claim
                            .and_then(|claim| claim.captures.get(index.get()))
                            .ok_or_else(|| stage_fault(location, "capture-declared"))?;
                        let ty = self.type_node(&capture.ty, location)?;
                        let node = self.parameter(
                            capture.name.as_str(),
                            scope.capture_level(index.get()),
                            ty,
                            location,
                        )?;
                        let kind = if capture.ty == ValueType::Boolean {
                            TermKind::Boolean
                        } else {
                            TermKind::Real
                        };
                        done.push((LeafTerm::reference(node), kind));
                    }
                    HybridTerm::Constant(value) => {
                        let ty = self.scalar("rational", location)?;
                        let key = self.insert(
                            location,
                            NodeTag::Value,
                            "literal",
                            Some(ty),
                            None,
                            BodyTerm::literal(ty, LiteralValue::Rational(value.clone())),
                        )?;
                        self.record(key, OccurrenceRole::Expression, location.clone());
                        done.push((LeafTerm::reference(key), TermKind::Real));
                    }
                    HybridTerm::Boolean(value) => {
                        let ty = self.type_node(&ValueType::Boolean, location)?;
                        let key = self.insert(
                            location,
                            NodeTag::Value,
                            "literal",
                            Some(ty),
                            None,
                            BodyTerm::literal(ty, LiteralValue::Boolean(*value)),
                        )?;
                        self.record(key, OccurrenceRole::Expression, location.clone());
                        done.push((LeafTerm::reference(key), TermKind::Boolean));
                    }
                    HybridTerm::Negate(operand) | HybridTerm::Not(operand) => {
                        steps.push(TermStep::Build(term));
                        steps.push(TermStep::Visit(operand));
                    }
                    HybridTerm::Arithmetic(_, left, right)
                    | HybridTerm::Compare(_, left, right)
                    | HybridTerm::Logic(_, left, right) => {
                        steps.push(TermStep::Build(term));
                        steps.push(TermStep::Visit(right));
                        steps.push(TermStep::Visit(left));
                    }
                    HybridTerm::Conditional(condition, then, otherwise) => {
                        steps.push(TermStep::Build(term));
                        steps.push(TermStep::Visit(otherwise));
                        steps.push(TermStep::Visit(then));
                        steps.push(TermStep::Visit(condition));
                    }
                    HybridTerm::Call(_, arguments) => {
                        steps.push(TermStep::Build(term));
                        steps.extend(arguments.iter().rev().map(TermStep::Visit));
                    }
                },
                TermStep::Build(term) => {
                    let (operator, identity, operands): (Operator, &str, usize) = match term {
                        HybridTerm::Negate(_) => (Operator::Unary, "quire.op.rational.negate", 1),
                        HybridTerm::Not(_) => (Operator::Unary, "quire.op.boolean.not", 1),
                        HybridTerm::Arithmetic(op, ..) => {
                            (Operator::Binary, arithmetic_identity(*op), 2)
                        }
                        HybridTerm::Compare(op, ..) => (Operator::Binary, compare_identity(*op), 2),
                        HybridTerm::Logic(op, ..) => (Operator::Binary, boolean_identity(*op), 2),
                        HybridTerm::Conditional(..) => {
                            (Operator::Conditional, "quire.op.control.if", 3)
                        }
                        HybridTerm::Call(_, arguments) => {
                            (Operator::Call, "quire.op.function.call", arguments.len())
                        }
                        HybridTerm::Variable(_)
                        | HybridTerm::ActivationVariable(_)
                        | HybridTerm::Capture(_)
                        | HybridTerm::Constant(_)
                        | HybridTerm::Boolean(_) => {
                            return Err(stage_fault(location, "leaf-is-not-built"))
                        }
                    };
                    let mut operand_terms = Vec::with_capacity(operands + 1);
                    let mut kinds = Vec::with_capacity(operands);
                    for _ in 0..operands {
                        let (term, kind) = done
                            .pop()
                            .ok_or_else(|| stage_fault(location, "operand-keyed-first"))?;
                        operand_terms.push(term);
                        kinds.push(kind);
                    }
                    operand_terms.reverse();
                    kinds.reverse();
                    let result = match term {
                        HybridTerm::Negate(_) | HybridTerm::Arithmetic(..) => TermKind::Real,
                        HybridTerm::Not(_) | HybridTerm::Compare(..) | HybridTerm::Logic(..) => {
                            TermKind::Boolean
                        }
                        HybridTerm::Conditional(..) => *kinds
                            .get(1)
                            .ok_or_else(|| stage_fault(location, "branch-kinded"))?,
                        HybridTerm::Call(function, _) => {
                            let signature = scope
                                .signatures
                                .get(function.get())
                                .ok_or_else(|| stage_fault(location, "callee-signed"))?;
                            if signature.result == ValueType::Boolean {
                                TermKind::Boolean
                            } else {
                                TermKind::Real
                            }
                        }
                        HybridTerm::Variable(_)
                        | HybridTerm::ActivationVariable(_)
                        | HybridTerm::Capture(_)
                        | HybridTerm::Constant(_)
                        | HybridTerm::Boolean(_) => {
                            return Err(stage_fault(location, "leaf-is-not-built"))
                        }
                    };
                    if let HybridTerm::Call(function, _) = term {
                        let callee = self
                            .functions
                            .get(function.get())
                            .copied()
                            .flatten()
                            .ok_or_else(|| stage_fault(location, "callee-lowered"))?;
                        operand_terms.insert(0, LeafTerm::reference(callee));
                    }
                    let result_type = match result {
                        TermKind::Boolean => self.type_node(&ValueType::Boolean, location)?,
                        TermKind::Real => self.scalar("rational", location)?,
                    };
                    let node = self.typed_application(
                        location,
                        result_type,
                        operator,
                        Operation::plain(identity),
                        operand_terms.into_iter().map(MemberTerm::Leaf).collect(),
                    )?;
                    done.push((node, result));
                }
            }
        }
        match done.as_slice() {
            [(term, _)] => Ok(term.clone()),
            _ => Err(stage_fault(location, "term-root-keyed")),
        }
    }
}

/// The `class` literal a variable's class is written as.
fn class_spelling(class: crate::check::hybrid::VariableClass) -> &'static str {
    match class {
        crate::check::hybrid::VariableClass::Clock => "clock",
        crate::check::hybrid::VariableClass::Hybrid => "hybrid",
    }
}

/// The `relation` literal of a reset that is not `=`.
fn relation_spelling(relation: CompareOp) -> Result<&'static str, CheckRefusal> {
    match relation {
        CompareOp::Ne => Ok("ne"),
        CompareOp::Lt => Ok("lt"),
        CompareOp::Le => Ok("le"),
        CompareOp::Ge => Ok("ge"),
        CompareOp::Gt => Ok("gt"),
        CompareOp::Eq => Err(stage_fault(
            &hybrid_location(HybridSite::Model(0)),
            "equality-reset-has-no-relation",
        )),
    }
}

/// The `clock` argument a hybrid claim's clock is written as.
fn clock_spelling(clock: HybridClock) -> &'static str {
    match clock {
        HybridClock::ModelTime => "model-time",
        HybridClock::ModelSteps => "model-steps",
    }
}

fn arithmetic_identity(op: ArithmeticOp) -> &'static str {
    match op {
        ArithmeticOp::Add => "quire.op.rational.add",
        ArithmeticOp::Sub => "quire.op.rational.sub",
        ArithmeticOp::Mul => "quire.op.rational.mul",
        ArithmeticOp::Div => "quire.op.rational.div",
    }
}

fn compare_identity(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Eq => "quire.op.rational.eq",
        CompareOp::Ne => "quire.op.rational.ne",
        CompareOp::Lt => "quire.op.rational.lt",
        CompareOp::Le => "quire.op.rational.le",
        CompareOp::Ge => "quire.op.rational.ge",
        CompareOp::Gt => "quire.op.rational.gt",
    }
}

fn boolean_identity(op: LogicOp) -> &'static str {
    match op {
        LogicOp::And => "quire.op.boolean.and",
        LogicOp::Or => "quire.op.boolean.or",
        LogicOp::Implies => "quire.op.boolean.implies",
    }
}

fn logic_identity(op: LogicOp) -> &'static str {
    match op {
        LogicOp::And => "quire.op.temporal.and",
        LogicOp::Or => "quire.op.temporal.or",
        LogicOp::Implies => "quire.op.temporal.implies",
    }
}

/// An operator's `temporal_interval` member in the form `profile` admits
/// (QSpec FR-370 "Placement and profile fit"): the timed form under
/// `quire.temporal.timed/v1`, the integer form under every other profile.
fn interval_member(
    profile: TemporalProfile,
    interval: Option<&HybridInterval>,
    location: &Location,
) -> Result<Member, CheckRefusal> {
    let Some(interval) = interval else {
        return Ok(Member::TemporalInterval { interval: None });
    };
    if profile == TemporalProfile::Timed {
        return Ok(Member::TimedInterval(Box::new(TimedMember {
            lower: interval.lower.clone(),
            lower_closed: !interval.lower_open,
            upper: interval.upper.clone(),
            upper_closed: interval.upper.is_some() && !interval.upper_open,
        })));
    }
    let whole = |value: &quire_exact::Rational| {
        value
            .is_integer()
            .then(|| value.numerator().to_u64())
            .flatten()
            .ok_or_else(|| stage_fault(location, "integer-profile-interval-is-whole"))
    };
    let upper = match &interval.upper {
        Some(upper) => TemporalUpper::Finite(whole(upper)?),
        None => TemporalUpper::Open,
    };
    let checked = TemporalInterval::new(whole(&interval.lower)?, upper)
        .map_err(|_| stage_fault(location, "interval-is-ordered"))?;
    Ok(Member::TemporalInterval {
        interval: Some(checked),
    })
}

/// A broken invariant of this lowering, never a refusal of the input.
fn stage_fault(location: &Location, invariant: &'static str) -> CheckRefusal {
    fault(
        location,
        KeyFault::StageFault(quire_semantic_value::catalog::InternalFault::new(
            STAGE, invariant,
        )),
    )
}

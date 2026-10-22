// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-381: the `ProtocolClause` family's S3 check of a protocol's
//! `activation`, its captures and its `compensate` templates (ADR-012 §12.2
//! Check row), and FR-218's role model rule.
//!
//! Each expression is checked through the one clause checker (FR-104) over
//! the names its stage may read and no others. A stage is one row of
//! FR-381's table ([`ObligationStage`]); the names it reads are bound as the
//! typer's parameters, so a name outside the row resolves to nothing and the
//! refusal is classified as FR-381 states ([`ReadClass`]): `forward` for a
//! capture written later in the same stage or the capture itself, `foreign`
//! for a name another template declares, `stage` for a name that exists but
//! that the stage may not read and `unbound` for a name nothing declares.
//! The protocol's `over` input is bound in every stage and refused by the
//! stages that bar model state, so that rule has precedence over the table.
//!
//! A refusal inside one template does not stop the other templates, the
//! captures or the activation from being checked, and the refusals come
//! back in source order.

mod cause;
#[cfg(test)]
mod tests;

pub use cause::{ObligationCause, ObligationOwner, ObligationStage, ReadClass};

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{
    ActivationForm, AnchorSite, BinderKind, CaptureForm, ClauseKind, CompensationCommit,
    CompensationForm, ExpressionForm, IntervalUpper, ProtocolDeclarationForm, StateClauseKind,
    TypeForm, TypeFormHead,
};
use qsl_foundation::Span;
use quire_exact::{IllTypedCause, ValueType};
use quire_semantic_value::declaration::CompositeShape;
use quire_semantic_value::location::{Location, Origin};

use super::check::{bind_parameters, Scope, StateContext, Typer};
use super::facts::Definedness;
use super::ir::{CheckedBody, NodeKind};
use super::observation::Observations;
use super::protocol_clause::{CheckedProtocol, ProtocolNodeId};
use super::refusal::{CheckCause, CheckRefusal, KeyFault};
use super::state_clause::{population_of, ClauseDeclarations, ClauseOperation, PopulationDomain};
use super::type_form::resolve_form;
use crate::family::CheckContext;
use crate::model::clause_observation::Observation;
use crate::model::operation::OperationLookup;

/// A slot no local carries: a protocol has no `self`, so a `self` written in
/// one of its expressions finds no binding.
const NO_SLOT: usize = usize::MAX;

/// Which stage list a capture belongs to (FR-381 Outputs).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureStage {
    /// A capture of the protocol.
    Protocol,
    /// A capture of a template written before `activate`.
    Registration,
    /// A capture of a template written in the activation block.
    Activation,
}

/// What a capture belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureOwner {
    /// The protocol clause.
    Protocol,
    /// The template at this index of [`CheckedObligations::compensations`].
    Compensation(usize),
}

/// One capture S3 checked (FR-381 Outputs): `CheckedCapture{name,
/// declared_type, stage, initializer, owner}`.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedCapture {
    /// The capture's name.
    pub name: String,
    /// The declared type.
    pub declared_type: ValueType,
    /// The stage the capture is evaluated in.
    pub stage: CaptureStage,
    /// The checked initializer.
    pub initializer: CheckedBody,
    /// The clause or the template that owns the capture.
    pub owner: CaptureOwner,
}

/// A binder with the type it declares.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedBinder {
    /// The binder's name.
    pub name: String,
    /// The binder's declared type.
    pub value_type: ValueType,
}

/// The protocol's activation (FR-381 Outputs).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum CheckedActivation {
    /// `activation on origin`: no binder and no guard.
    #[default]
    Origin,
    /// `activation on each (binder: record_type) when (guard)`.
    OnEach {
        /// The activation binder's name.
        binder: String,
        /// The binder's declared type.
        record_type: Box<ValueType>,
        /// The guard; `None` for the guard that is always true. A written
        /// `when ({true})` is a different clause from an absent guard.
        guard: Option<CheckedBody>,
    },
}

/// A template's closed `within [lower, upper]` deadline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedDeadline {
    /// The lower bound.
    pub lower: u64,
    /// The upper bound, at least the lower bound.
    pub upper: u64,
}

/// A template's `retry` binders and relation.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedRetry {
    /// The `earlier` binder.
    pub earlier: CheckedBinder,
    /// The `later` binder.
    pub later: CheckedBinder,
    /// The Boolean relation.
    pub relation: CheckedBody,
}

/// A template's `recover` binder and predicate.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedRecovery {
    /// The recovery binder.
    pub binder: CheckedBinder,
    /// The Boolean predicate.
    pub predicate: CheckedBody,
}

/// A template's `commit` member.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckedCommit {
    /// `commit never`: no earlier cutoff.
    Never,
    /// `commit N`: the `commit` node `N` names.
    Node(ProtocolNodeId),
}

/// One `compensate` template S3 checked (FR-381 Outputs).
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedCompensation {
    /// The template's name.
    pub name: String,
    /// The one forward `effect` node it registers for.
    pub effect: ProtocolNodeId,
    /// The index of the `by` role among the protocol's roles.
    pub role: usize,
    /// The resolved operation. The identity of its shared anchor and frame
    /// node is [`CheckedProtocol::compensation_operations`]'s entry at this
    /// template's index, minted at S4.
    pub operation: ClauseOperation,
    /// The population domain of the type declaring the operation, which
    /// the operation's frame record names (FR-104 "Requirements").
    pub(crate) frame_population: Option<PopulationDomain>,
    /// The forward binder.
    pub forward: CheckedBinder,
    /// The registration captures, as indices into
    /// [`CheckedObligations::captures`], in source order.
    pub registration_captures: Vec<usize>,
    /// The trigger binder.
    pub trigger: CheckedBinder,
    /// The Boolean trigger guard.
    pub guard: CheckedBody,
    /// The activation captures, as indices into
    /// [`CheckedObligations::captures`], in source order.
    pub activation_captures: Vec<usize>,
    /// The closed deadline.
    pub within: CheckedDeadline,
    /// The maximum number of attempts, counting the initial attempt; at
    /// least one.
    pub maximum_attempts: u64,
    /// The attempt type: a record type or an event type.
    pub attempt_type: ValueType,
    /// The retry relation.
    pub retry: CheckedRetry,
    /// The commit boundary.
    pub commit: CheckedCommit,
    /// The recovery binder and predicate.
    pub recover: CheckedRecovery,
}

/// What S3 checked of a protocol's obligations (FR-381 Outputs): the
/// activation, every capture of every stage and every template, each in
/// source order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CheckedObligations {
    /// The activation.
    pub activation: CheckedActivation,
    /// The protocol's captures, then each template's registration and
    /// activation captures, in source order.
    pub captures: Vec<CheckedCapture>,
    /// The templates, in source order.
    pub compensations: Vec<CheckedCompensation>,
}

/// What [`check`] found.
pub(crate) enum Obligations {
    /// Every obligation checked, with the package's running node total.
    Checked(Box<CheckedObligations>, u64),
    /// Refusals, in source order.
    Refused(Vec<CheckRefusal>),
    /// A declared type did not resolve. The binder-type check of
    /// `protocol_clause::content` refuses it, so there is nothing to add.
    Unresolved,
}

/// The types of the names a protocol declares, resolved once. A form that
/// does not resolve is `None`: its refusal is `content`'s.
struct Types {
    input: Option<(String, ValueType)>,
    activation: Option<(String, ValueType)>,
    captures: Vec<Option<ValueType>>,
    templates: Vec<TemplateTypes>,
}

struct TemplateTypes {
    forward: Option<ValueType>,
    trigger: Option<ValueType>,
    earlier: Option<ValueType>,
    later: Option<ValueType>,
    recovery: Option<ValueType>,
    registration: Vec<Option<ValueType>>,
    activation: Vec<Option<ValueType>>,
}

/// One template with its resolved types.
#[derive(Clone, Copy)]
struct Template<'t> {
    index: usize,
    form: &'t CompensationForm,
    types: &'t TemplateTypes,
}

/// Every name a protocol declares, by who declares it, so a refused read is
/// classified (FR-381 "Names a stage may read").
struct Universe {
    protocol: BTreeSet<String>,
    templates: Vec<BTreeSet<String>>,
    nodes: BTreeSet<String>,
}

impl Universe {
    fn of(protocol: &ProtocolDeclarationForm) -> Self {
        let mut names: BTreeSet<String> = protocol
            .binders
            .iter()
            .filter(|binder| binder.kind == BinderKind::Input)
            .map(|binder| binder.name.name.clone())
            .collect();
        if let ActivationForm::Each { parameter, .. } = &protocol.activation {
            names.insert(parameter.name.name.clone());
        }
        names.extend(
            protocol
                .captures
                .iter()
                .map(|capture| capture.parameter.name.name.clone()),
        );
        let templates: Vec<BTreeSet<String>> = protocol
            .compensations
            .iter()
            .map(|template| {
                [
                    &template.forward,
                    &template.trigger,
                    &template.retry.earlier,
                    &template.retry.later,
                    &template.recover.binder,
                ]
                .into_iter()
                .map(|binder| binder.name.name.clone())
                .chain(
                    template
                        .registration_captures
                        .iter()
                        .chain(&template.activation_captures)
                        .map(|capture| capture.parameter.name.name.clone()),
                )
                .collect::<BTreeSet<String>>()
            })
            .collect();
        let nodes = protocol
            .binders
            .iter()
            .filter(|binder| binder.kind == BinderKind::RecordBinder)
            .map(|binder| binder.name.name.clone())
            .collect();
        Self {
            protocol: names,
            templates,
            nodes,
        }
    }

    /// Why `site` may not read `name`.
    fn classify(&self, name: &str, site: &Site) -> ReadClass {
        if site.later.iter().any(|later| later == name) {
            return ReadClass::Forward;
        }
        let foreign = self
            .templates
            .iter()
            .enumerate()
            .any(|(index, names)| Some(index) != site.template && names.contains(name));
        if foreign {
            return ReadClass::Foreign;
        }
        let own = site
            .template
            .and_then(|index| self.templates.get(index))
            .is_some_and(|names| names.contains(name));
        if own || self.protocol.contains(name) || self.nodes.contains(name) {
            ReadClass::Stage
        } else {
            ReadClass::Unbound
        }
    }
}

/// One expression's reading context: its stage, its owner and the names it
/// may read, in slot order.
struct Site {
    stage: ObligationStage,
    owner: ObligationOwner,
    template: Option<usize>,
    bound: Vec<(String, ValueType)>,
    /// The slot of the `over` input among `bound`.
    input: Option<usize>,
    /// The captures of this stage written at or after the one being
    /// checked.
    later: Vec<String>,
}

impl Site {
    fn bind(&mut self, name: &str, value_type: &Option<ValueType>) {
        if let Some(value_type) = value_type {
            self.bound.push((name.to_owned(), value_type.clone()));
        }
    }

    fn bind_pair(&mut self, pair: &Option<(String, ValueType)>) {
        if let Some((name, value_type)) = pair {
            self.bound.push((name.clone(), value_type.clone()));
        }
    }

    fn bind_input(&mut self, input: &Option<(String, ValueType)>) {
        if input.is_some() {
            self.input = Some(self.bound.len());
        }
        self.bind_pair(input);
    }
}

/// What an expression is checked against.
#[derive(Clone, Copy)]
pub(super) enum Goal<'g> {
    /// A Boolean guard, relation or predicate.
    Boolean,
    /// A capture's initializer, against the type the capture declares.
    Declared {
        capture: &'g str,
        declared: &'g ValueType,
    },
    /// No expectation: the expression's own type.
    Infer,
}

/// Checks `protocol`'s roles' obligations: its activation, captures and
/// templates (FR-381). `anchors` is the protocol's resolved scoped anchors,
/// control tree and edge relation when FR-113 resolved them all; the effect
/// and commit nodes of a template and the commit's dominance come from
/// them, so without them those are not checked. `profile` is the selected
/// protocol profile's identity, which a dominance refusal names.
pub(crate) fn check<'a>(
    protocol: &ProtocolDeclarationForm,
    anchors: Option<&CheckedProtocol>,
    profile: Option<&str>,
    declarations: &ClauseDeclarations<'a>,
    cx: &mut CheckContext<'a, ClauseDeclarations<'a>>,
) -> Obligations {
    let mut checker = Checker {
        protocol,
        anchors,
        profile,
        input: declarations,
        universe: Universe::of(protocol),
        nodes: declarations.nodes_used,
        refusals: Vec::new(),
        incomplete: false,
    };
    let mut scope_cx = cx.reborrow(declarations.scope);
    let checked = checker.run(&mut scope_cx);
    checker.finish(checked)
}

fn refusal(cause: ObligationCause) -> CheckRefusal {
    CheckRefusal {
        // A protocol has no expression tree for a `Location` path to walk;
        // `region::DeclarationRegions::refusal_region` reads the cause's own
        // span, so this `Location` is never resolved.
        location: Location::root(Origin::Expression),
        cause: CheckCause::ProtocolObligation(Box::new(cause)),
    }
}

struct Checker<'c, 'a> {
    protocol: &'c ProtocolDeclarationForm,
    anchors: Option<&'c CheckedProtocol>,
    profile: Option<&'c str>,
    input: &'c ClauseDeclarations<'a>,
    universe: Universe,
    nodes: u64,
    refusals: Vec<(Span, CheckRefusal)>,
    /// Some part did not check and produced no refusal of this checker.
    incomplete: bool,
}

impl Checker<'_, '_> {
    fn scope(&self) -> &Scope {
        self.input.scope
    }

    fn resolve(&self, form: &TypeForm) -> Option<ValueType> {
        resolve_form(self.scope(), form).ok()
    }

    fn refuse(&mut self, cause: ObligationCause) {
        self.refusals.push((cause.span(), refusal(cause)));
    }

    fn finish(mut self, checked: Option<CheckedObligations>) -> Obligations {
        if !self.refusals.is_empty() {
            self.refusals.sort_by_key(|(span, _)| span.start);
            return Obligations::Refused(
                self.refusals
                    .into_iter()
                    .map(|(_, refusal)| refusal)
                    .collect(),
            );
        }
        match checked {
            Some(checked) if !self.incomplete => {
                Obligations::Checked(Box::new(checked), self.nodes)
            }
            Some(_) | None => Obligations::Unresolved,
        }
    }

    fn resolve_types(&self) -> Types {
        let protocol = self.protocol;
        let pair = |name: &str, form: &TypeForm| {
            self.resolve(form)
                .map(|value_type| (name.to_owned(), value_type))
        };
        let captures = |captures: &[CaptureForm]| -> Vec<Option<ValueType>> {
            captures
                .iter()
                .map(|capture| self.resolve(&capture.parameter.value_type))
                .collect()
        };
        Types {
            input: protocol
                .binders
                .iter()
                .find(|binder| binder.kind == BinderKind::Input)
                .and_then(|binder| pair(&binder.name.name, &binder.value_type)),
            activation: match &protocol.activation {
                ActivationForm::Origin => None,
                ActivationForm::Each { parameter, .. } => {
                    pair(&parameter.name.name, &parameter.value_type)
                }
            },
            captures: captures(&protocol.captures),
            templates: protocol
                .compensations
                .iter()
                .map(|template| TemplateTypes {
                    forward: self.resolve(&template.forward.value_type),
                    trigger: self.resolve(&template.trigger.value_type),
                    earlier: self.resolve(&template.retry.earlier.value_type),
                    later: self.resolve(&template.retry.later.value_type),
                    recovery: self.resolve(&template.recover.binder.value_type),
                    registration: captures(&template.registration_captures),
                    activation: captures(&template.activation_captures),
                })
                .collect(),
        }
    }

    fn run(&mut self, cx: &mut CheckContext<'_, Scope>) -> Option<CheckedObligations> {
        let protocol = self.protocol;
        let types = self.resolve_types();

        let activation = self.activation(&types, cx);

        let mut captures = Vec::new();
        for (position, (capture, declared)) in
            protocol.captures.iter().zip(&types.captures).enumerate()
        {
            let site = self.site(&types, ObligationStage::ProtocolCapture, None, position);
            match self.capture(
                &site,
                capture,
                declared,
                (CaptureStage::Protocol, CaptureOwner::Protocol),
                cx,
            ) {
                Some(checked) => captures.push(checked),
                None => self.incomplete = true,
            }
        }

        let mut compensations = Vec::new();
        for (index, (form, template_types)) in protocol
            .compensations
            .iter()
            .zip(&types.templates)
            .enumerate()
        {
            let template = Template {
                index,
                form,
                types: template_types,
            };
            let before = self.refusals.len();
            match self.template(&types, template, &mut captures, cx) {
                Some(checked) => compensations.push(checked),
                None if self.refusals.len() == before => self.incomplete = true,
                None => {}
            }
        }

        Some(CheckedObligations {
            activation: activation?,
            captures,
            compensations,
        })
    }

    /// The names `stage` may read in `template`, at capture `position` for a
    /// capture stage (FR-381 "Names a stage may read").
    fn site(
        &self,
        types: &Types,
        stage: ObligationStage,
        template: Option<Template<'_>>,
        position: usize,
    ) -> Site {
        let protocol = self.protocol;
        let owner = template.map_or(ObligationOwner::Protocol, |template| {
            ObligationOwner::Compensation(template.form.name.name.clone())
        });
        let mut site = Site {
            stage,
            owner,
            template: template.map(|template| template.index),
            bound: Vec::new(),
            input: None,
            later: Vec::new(),
        };
        let protocol_captures = |site: &mut Site, count: usize| {
            for (capture, declared) in protocol.captures.iter().zip(&types.captures).take(count) {
                site.bind(&capture.parameter.name.name, declared);
            }
        };
        let Some(template) = template else {
            match stage {
                ObligationStage::ProtocolActivation => {
                    site.bind_pair(&types.activation);
                    site.bind_input(&types.input);
                }
                ObligationStage::ProtocolCapture => {
                    site.bind_input(&types.input);
                    site.bind_pair(&types.activation);
                    protocol_captures(&mut site, position);
                    site.later = protocol
                        .captures
                        .iter()
                        .skip(position)
                        .map(|capture| capture.parameter.name.name.clone())
                        .collect();
                }
                ObligationStage::RegistrationCapture
                | ObligationStage::TriggerGuard
                | ObligationStage::ActivationCapture
                | ObligationStage::Retry
                | ObligationStage::Recovery => {}
            }
            return site;
        };
        let form = template.form;
        let own = template.types;
        let registration = |site: &mut Site, count: usize| {
            for (capture, declared) in form
                .registration_captures
                .iter()
                .zip(&own.registration)
                .take(count)
            {
                site.bind(&capture.parameter.name.name, declared);
            }
        };
        let activation = |site: &mut Site, count: usize| {
            for (capture, declared) in form
                .activation_captures
                .iter()
                .zip(&own.activation)
                .take(count)
            {
                site.bind(&capture.parameter.name.name, declared);
            }
        };
        match stage {
            ObligationStage::RegistrationCapture => {
                site.bind(&form.forward.name.name, &own.forward);
                site.bind_input(&types.input);
                site.bind_pair(&types.activation);
                protocol_captures(&mut site, usize::MAX);
                registration(&mut site, position);
                site.later = form
                    .registration_captures
                    .iter()
                    .skip(position)
                    .map(|capture| capture.parameter.name.name.clone())
                    .collect();
            }
            ObligationStage::TriggerGuard => {
                site.bind(&form.trigger.name.name, &own.trigger);
                protocol_captures(&mut site, usize::MAX);
                registration(&mut site, usize::MAX);
                site.bind_input(&types.input);
            }
            ObligationStage::ActivationCapture => {
                site.bind(&form.trigger.name.name, &own.trigger);
                site.bind_input(&types.input);
                protocol_captures(&mut site, usize::MAX);
                registration(&mut site, usize::MAX);
                activation(&mut site, position);
                site.later = form
                    .activation_captures
                    .iter()
                    .skip(position)
                    .map(|capture| capture.parameter.name.name.clone())
                    .collect();
            }
            ObligationStage::Retry => {
                site.bind(&form.retry.earlier.name.name, &own.earlier);
                site.bind(&form.retry.later.name.name, &own.later);
                protocol_captures(&mut site, usize::MAX);
                registration(&mut site, usize::MAX);
                activation(&mut site, usize::MAX);
                site.bind_input(&types.input);
            }
            ObligationStage::Recovery => {
                site.bind(&form.recover.binder.name.name, &own.recovery);
                protocol_captures(&mut site, usize::MAX);
                registration(&mut site, usize::MAX);
                activation(&mut site, usize::MAX);
                site.bind_input(&types.input);
            }
            ObligationStage::ProtocolActivation | ObligationStage::ProtocolCapture => {}
        }
        site
    }

    fn activation(
        &mut self,
        types: &Types,
        cx: &mut CheckContext<'_, Scope>,
    ) -> Option<CheckedActivation> {
        let protocol = self.protocol;
        let ActivationForm::Each { parameter, when } = &protocol.activation else {
            return Some(CheckedActivation::Origin);
        };
        let guard = match when {
            None => Some(None),
            Some(when) => {
                let site = self.site(types, ObligationStage::ProtocolActivation, None, 0);
                self.expression(&site, when, Goal::Boolean, cx).map(Some)
            }
        };
        match (&types.activation, guard) {
            (Some((_, record_type)), Some(guard)) => Some(CheckedActivation::OnEach {
                binder: parameter.name.name.clone(),
                record_type: Box::new(record_type.clone()),
                guard,
            }),
            _ => {
                self.incomplete = true;
                None
            }
        }
    }

    fn capture(
        &mut self,
        site: &Site,
        form: &CaptureForm,
        declared: &Option<ValueType>,
        (stage, owner): (CaptureStage, CaptureOwner),
        cx: &mut CheckContext<'_, Scope>,
    ) -> Option<CheckedCapture> {
        let declared = declared.as_ref()?;
        let goal = Goal::Declared {
            capture: &form.parameter.name.name,
            declared,
        };
        let initializer = self.expression(site, &form.value, goal, cx)?;
        Some(CheckedCapture {
            name: form.parameter.name.name.clone(),
            declared_type: declared.clone(),
            stage,
            initializer,
            owner,
        })
    }

    /// The node the anchor at `site` inside `form` resolved to.
    fn anchor(&self, form: &CompensationForm, site: AnchorSite) -> Option<ProtocolNodeId> {
        let checked = self.anchors?;
        self.protocol
            .scoped_anchors
            .iter()
            .zip(&checked.resolved_anchors)
            .find(|(anchor, _)| anchor.site == site && contains(form.span, anchor.anchor.span))
            .map(|(_, node)| *node)
    }

    /// The record type of the `effect` node `effect`: the type of its `as`
    /// binder.
    fn effect_type(&self, effect: ProtocolNodeId) -> Option<ValueType> {
        let binder = super::protocol_clause::record_binder(self.protocol, effect.0)?;
        self.resolve(&binder.value_type)
    }

    /// FR-381 "Retry, commit and recovery": a `commit` node that dominates
    /// the forward effect -- every execution reaching the effect has passed
    /// it -- refuses at the commit anchor.
    fn commit_dominance(
        &mut self,
        form: &CompensationForm,
        commit: ProtocolNodeId,
        effect: ProtocolNodeId,
    ) {
        let (Some(checked), CompensationCommit::Node(anchor)) = (self.anchors, &form.commit) else {
            return;
        };
        if !super::protocol_clause::dominates(self.protocol, &checked.controls, commit, effect) {
            return;
        }
        let name = |node: ProtocolNodeId| {
            self.protocol
                .declarations
                .get(node.0)
                .map(|declaration| declaration.name.name.clone())
                .unwrap_or_default()
        };
        self.refuse(ObligationCause::CommitPrecedesEffect {
            profile: self.profile.map(str::to_owned),
            compensation: form.name.name.clone(),
            commit: name(commit),
            effect: name(effect),
            span: anchor.span,
        });
    }

    fn template(
        &mut self,
        types: &Types,
        template: Template<'_>,
        captures: &mut Vec<CheckedCapture>,
        cx: &mut CheckContext<'_, Scope>,
    ) -> Option<CheckedCompensation> {
        let form = template.form;
        let own = template.types;
        let name = form.name.name.as_str();

        let effect = self.anchor(form, AnchorSite::CompensateFor);
        if let (Some(effect), Some(actual)) = (effect, &own.forward) {
            if let Some(expected) = self.effect_type(effect) {
                if expected != *actual {
                    self.refuse(ObligationCause::ForwardType {
                        compensation: name.to_owned(),
                        expected,
                        actual: actual.clone(),
                        span: form.forward.value_type.span,
                    });
                }
            }
        }

        let role = self
            .protocol
            .roles
            .iter()
            .position(|role| role.name.name == form.role.name);
        if role.is_none() {
            self.refuse(ObligationCause::MissingRole {
                compensation: name.to_owned(),
                name: form.role.name.clone(),
                span: form.role.span,
            });
        }

        let operation = self.operation(form);
        let frame_population = operation
            .as_ref()
            .and_then(|operation| self.frame_population(form, operation));

        if form.attempts.count == 0 {
            self.refuse(ObligationCause::ZeroAttempts {
                compensation: name.to_owned(),
                span: form.attempts.count_span,
            });
        }
        let attempt_type = self.attempt_type(form);
        if let Some(attempt) = &attempt_type {
            for binder in [&form.retry.earlier, &form.retry.later] {
                if let Some(actual) = self.resolve(&binder.value_type) {
                    if actual != *attempt {
                        self.refuse(ObligationCause::RetryBinder {
                            compensation: name.to_owned(),
                            binder: binder.name.name.clone(),
                            expected: attempt.clone(),
                            actual,
                            span: binder.value_type.span,
                        });
                    }
                }
            }
        }

        let within = match form.within.upper {
            IntervalUpper::Finite(upper) if form.within.lower <= upper => Some(CheckedDeadline {
                lower: form.within.lower,
                upper,
            }),
            IntervalUpper::Finite(upper) => {
                self.refuse(ObligationCause::Deadline {
                    compensation: name.to_owned(),
                    lower: form.within.lower,
                    upper: Some(upper),
                    span: form.within.span,
                });
                None
            }
            IntervalUpper::Open => {
                self.refuse(ObligationCause::Deadline {
                    compensation: name.to_owned(),
                    lower: form.within.lower,
                    upper: None,
                    span: form.within.span,
                });
                None
            }
        };

        let mut registration = Vec::new();
        for (position, (capture, declared)) in form
            .registration_captures
            .iter()
            .zip(&own.registration)
            .enumerate()
        {
            let site = self.site(
                types,
                ObligationStage::RegistrationCapture,
                Some(template),
                position,
            );
            registration.extend(self.capture(
                &site,
                capture,
                declared,
                (
                    CaptureStage::Registration,
                    CaptureOwner::Compensation(template.index),
                ),
                cx,
            ));
        }

        let site = self.site(types, ObligationStage::TriggerGuard, Some(template), 0);
        let guard = self.expression(&site, &form.guard, Goal::Boolean, cx);

        let mut activation = Vec::new();
        for (position, (capture, declared)) in form
            .activation_captures
            .iter()
            .zip(&own.activation)
            .enumerate()
        {
            let site = self.site(
                types,
                ObligationStage::ActivationCapture,
                Some(template),
                position,
            );
            activation.extend(self.capture(
                &site,
                capture,
                declared,
                (
                    CaptureStage::Activation,
                    CaptureOwner::Compensation(template.index),
                ),
                cx,
            ));
        }

        let site = self.site(types, ObligationStage::Retry, Some(template), 0);
        let relation = self.expression(&site, &form.retry.relation, Goal::Boolean, cx);

        let site = self.site(types, ObligationStage::Recovery, Some(template), 0);
        let predicate = self.expression(&site, &form.recover.predicate, Goal::Boolean, cx);

        let commit = match &form.commit {
            CompensationCommit::Never(_) => Some(CheckedCommit::Never),
            CompensationCommit::Node(_) => self
                .anchor(form, AnchorSite::CompensateCommit)
                .map(CheckedCommit::Node),
        };
        if let (Some(CheckedCommit::Node(commit)), Some(effect)) = (commit, effect) {
            self.commit_dominance(form, commit, effect);
        }

        let binder = |declared: &Option<ValueType>, name: &str| {
            declared.as_ref().map(|value_type| CheckedBinder {
                name: name.to_owned(),
                value_type: value_type.clone(),
            })
        };
        let build = move || -> Option<CheckedCompensation> {
            let registration_captures =
                (captures.len()..captures.len() + registration.len()).collect::<Vec<_>>();
            let activation_captures = (captures.len() + registration.len()
                ..captures.len() + registration.len() + activation.len())
                .collect::<Vec<_>>();
            let compensation = CheckedCompensation {
                name: name.to_owned(),
                effect: effect?,
                role: role?,
                operation: operation?,
                frame_population: frame_population?,
                forward: binder(&own.forward, &form.forward.name.name)?,
                registration_captures,
                trigger: binder(&own.trigger, &form.trigger.name.name)?,
                guard: guard?,
                activation_captures,
                within: within?,
                maximum_attempts: form.attempts.count,
                attempt_type: attempt_type?,
                retry: CheckedRetry {
                    earlier: binder(&own.earlier, &form.retry.earlier.name.name)?,
                    later: binder(&own.later, &form.retry.later.name.name)?,
                    relation: relation?,
                },
                commit: commit?,
                recover: CheckedRecovery {
                    binder: binder(&own.recovery, &form.recover.binder.name.name)?,
                    predicate: predicate?,
                },
            };
            captures.extend(registration);
            captures.extend(activation);
            Some(compensation)
        };
        build()
    }

    /// Resolves `M::X::op` as FR-114 resolves an attempt's operation.
    fn operation(&mut self, form: &CompensationForm) -> Option<ClauseOperation> {
        let name = form.name.name.as_str();
        let written = &form.operation;
        let Some((context, member)) = written.name.rsplit_once("::") else {
            self.refuse(ObligationCause::MissingOperation {
                compensation: name.to_owned(),
                name: written.name.clone(),
                span: written.span,
            });
            return None;
        };
        let context_span = Span {
            start: written.span.start,
            end: written.span.start.saturating_add(context.len()),
        };
        let member_span = Span {
            start: written.span.end.saturating_sub(member.len()),
            end: written.span.end,
        };
        let scope = self.input.scope;
        let Some(object) = scope.object_type_named(context) else {
            self.refuse(ObligationCause::MissingOperation {
                compensation: name.to_owned(),
                name: context.to_owned(),
                span: context_span,
            });
            return None;
        };
        match scope.operations().resolve(scope.types(), object, member) {
            OperationLookup::Declared {
                declaring,
                operation,
            } => Some(ClauseOperation {
                declaring,
                declaration: operation.clone(),
            }),
            OperationLookup::Missing => {
                self.refuse(ObligationCause::MissingOperation {
                    compensation: name.to_owned(),
                    name: member.to_owned(),
                    span: member_span,
                });
                None
            }
            OperationLookup::Ambiguous(_) => {
                self.refuse(ObligationCause::AmbiguousOperation {
                    compensation: name.to_owned(),
                    name: member.to_owned(),
                    span: member_span,
                });
                None
            }
        }
    }

    /// The population domain of the type declaring `operation`, resolved
    /// as FR-114 resolves an attempt's (`None` inside the result when the
    /// type shares no population). Two or more populations covering the
    /// type leave the frame's extent undecided and refuse
    /// `ambiguous_declaration`/`ambiguous-name` at the operation.
    fn frame_population(
        &mut self,
        form: &CompensationForm,
        operation: &ClauseOperation,
    ) -> Option<Option<PopulationDomain>> {
        match population_of(self.input.models, operation.declaring, self.scope().types()) {
            Ok(domain) => Some(domain),
            Err(populations) => {
                self.refuse(ObligationCause::AmbiguousPopulation {
                    compensation: form.name.name.clone(),
                    populations,
                    span: form.operation.span,
                });
                None
            }
        }
    }

    /// The attempt type `T` of `attempts n of T`: a record type or an event
    /// type (FR-381 "Activation, attempts and deadline").
    fn attempt_type(&mut self, form: &CompensationForm) -> Option<ValueType> {
        let name = form.name.name.clone();
        let record = &form.attempts.record;
        match resolve_form(self.scope(), record) {
            Err(error) => {
                self.refuse(ObligationCause::AttemptTypeName {
                    compensation: name,
                    fault: error.fault,
                    span: error.span,
                });
                None
            }
            Ok(value_type) if is_record(self.scope(), &value_type) => Some(value_type),
            Ok(_) => {
                self.refuse(ObligationCause::AttemptType {
                    compensation: name,
                    name: match &record.head {
                        TypeFormHead::Name(written) => Some(written.clone()),
                        TypeFormHead::Builtin(_)
                        | TypeFormHead::Collection(_)
                        | TypeFormHead::Population => None,
                    },
                    span: record.span,
                });
                None
            }
        }
    }

    /// Checks `form` through the clause checker over `site`'s names. A
    /// refusal is recorded and `None` returned.
    fn expression(
        &mut self,
        site: &Site,
        form: &ExpressionForm,
        goal: Goal<'_>,
        cx: &mut CheckContext<'_, Scope>,
    ) -> Option<CheckedBody> {
        let mut nodes = self.nodes;
        let typed = type_body(form, &site.bound, goal, self.input, &mut nodes, cx);
        self.nodes = nodes;
        let body = match typed {
            Ok(body) => body,
            Err(refusal) => {
                self.convert(site, form, goal, refusal, cx);
                return None;
            }
        };
        if site.stage.bars_model_state() {
            let reads = model_state_reads(&body, form, site.input);
            if !reads.is_empty() {
                for span in reads {
                    self.refuse(ObligationCause::ModelStateRead {
                        stage: site.stage,
                        owner: site.owner.clone(),
                        span,
                    });
                }
                return None;
            }
        }
        let observations = Observations::of(body.root(), Observation::Current, site.bound.len());
        let defined = Definedness::new(
            site.bound.len(),
            self.input.dispatch_tables,
            &self.input.scope.dispatch_operations,
        )
        .with_observations(&observations)
        .check(body.root());
        if let Err(refusal) = defined {
            self.convert(site, form, goal, refusal, cx);
            return None;
        }
        Some(body)
    }

    /// Records the clause checker's `refusal` of `form` at `site` as the
    /// refusal FR-381 states for it.
    fn convert(
        &mut self,
        site: &Site,
        form: &ExpressionForm,
        goal: Goal<'_>,
        refused: CheckRefusal,
        cx: &mut CheckContext<'_, Scope>,
    ) {
        let CheckRefusal { location, cause } = refused;
        let span = span_of(form, &location);
        if let (
            CheckCause::IllTyped(IllTypedCause::TypeMismatch),
            Goal::Declared { capture, declared },
        ) = (&cause, goal)
        {
            if location.depth() == 0 {
                let mut nodes = self.nodes;
                let actual = type_body(form, &site.bound, Goal::Infer, self.input, &mut nodes, cx)
                    .ok()
                    .map(|body| body.root().value_type().clone());
                if let Some(actual) = actual {
                    self.refuse(ObligationCause::CaptureType {
                        capture: capture.to_owned(),
                        expected: declared.clone(),
                        actual,
                        span,
                    });
                    return;
                }
            }
        }
        match cause {
            CheckCause::MissingName(name) if !is_qualified(&name) => {
                let read = self.universe.classify(&name, site);
                self.refuse(ObligationCause::StageRead {
                    name,
                    stage: site.stage,
                    owner: site.owner.clone(),
                    read,
                    span,
                });
            }
            cause if wraps(&cause) => self.refuse(ObligationCause::Clause {
                stage: site.stage,
                owner: site.owner.clone(),
                cause: Box::new(cause),
                span,
            }),
            // A reached limit, a cancellation or a fault keeps its own
            // cause, which the callers classify by kind.
            cause => self.refusals.push((span, CheckRefusal { location, cause })),
        }
    }
}

/// Whether the clause checker's `cause` is one a stage reports as its own
/// (FR-381 refusals for non-Boolean roots, undischarged obligations and the
/// like).
pub(super) fn wraps(cause: &CheckCause) -> bool {
    matches!(
        cause,
        CheckCause::IllTyped(_)
            | CheckCause::IdentityObservingForm(_)
            | CheckCause::NonBooleanRoot
            | CheckCause::WrongSnapshot(_)
            | CheckCause::MissingName(_)
            | CheckCause::AmbiguousName { .. }
            | CheckCause::Unproved(_)
            | CheckCause::UnanchoredResult { .. }
            | CheckCause::IntegerOutsideI128(_)
            | CheckCause::MalformedUuidLiteral { .. }
            | CheckCause::UnsupportedNavigation(_)
            | CheckCause::ProfileProhibitedOperator(_)
    )
}

/// Types `form` over `bound` as a state clause types its body (FR-104), so a
/// field read through an object reference is a model read.
pub(super) fn type_body(
    form: &ExpressionForm,
    bound: &[(String, ValueType)],
    goal: Goal<'_>,
    input: &ClauseDeclarations<'_>,
    nodes: &mut u64,
    cx: &mut CheckContext<'_, Scope>,
) -> Result<CheckedBody, CheckRefusal> {
    let mut typer = Typer::new(
        input.scope,
        input.signatures,
        input.checking_limits,
        nodes,
        ClauseKind::Invariant,
    )
    .with_cancel(input.cancel)
    .with_context(cx.reborrow(input.scope));
    bind_parameters(&mut typer, bound, input.location)?;
    typer.enter_state_clause(StateContext {
        parent_projections: BTreeMap::new(),
        kind: StateClauseKind::Invariant,
        operation: None,
        self_slot: NO_SLOT,
        result_slot: None,
    });
    match goal {
        Goal::Boolean => {
            let body = typer.infer(&form.expression, Some(&ValueType::Boolean), input.location)?;
            if body.root().value_type() == &ValueType::Boolean {
                Ok(body)
            } else {
                Err(CheckRefusal {
                    location: input.location.clone(),
                    cause: CheckCause::NonBooleanRoot,
                })
            }
        }
        Goal::Declared { declared, .. } => {
            typer.check_as(&form.expression, declared, input.location)
        }
        Goal::Infer => typer.infer(&form.expression, None, input.location),
    }
}

/// Whether the clause checker's missing name was written with a `::`
/// qualifier. The expression tree carries a name only as its spelling
/// (`ExprNode::Name`), so the qualifier is read from that spelling: a
/// qualified name is a missing declaration, not a stage's unreadable name.
#[ix_trace_rs::string_edge]
fn is_qualified(name: &str) -> bool {
    name.contains("::")
}

/// The span of the node `location` reaches in `form`, or the whole
/// expression's.
pub(super) fn span_of(form: &ExpressionForm, location: &Location) -> Span {
    form.spans
        .at(&location.path())
        .or_else(|| form.spans.at(&[]))
        .unwrap_or(Span { start: 0, end: 0 })
}

/// Every read of model state in a stage that bars it (FR-218 "Model
/// state"): a field read of an object binder, a field read through a
/// reference-typed value, an explicit dereference, a navigation, a
/// reachability, a lookup or population read and a dispatch call, and every
/// other read of the `over` input. Outermost reads only, in source order.
pub(super) fn model_state_reads(
    body: &CheckedBody,
    form: &ExpressionForm,
    input: Option<usize>,
) -> Vec<Span> {
    let mut spans = Vec::new();
    for node in body.root().descendants() {
        let location = node.location();
        match node.kind() {
            // `deref(r).f` is reported at the `deref(r)`.
            NodeKind::Attribute { derefed: true, .. } => {
                spans.push(span_of(form, &location.child(0)));
            }
            NodeKind::Attribute { .. }
            | NodeKind::Navigate { .. }
            | NodeKind::AllInstances { .. }
            | NodeKind::Lookup { .. }
            | NodeKind::Reaches { .. }
            | NodeKind::ReachesEnd { .. }
            | NodeKind::Dispatch { .. } => spans.push(span_of(form, location)),
            NodeKind::Local(slot) if Some(*slot) == input => spans.push(span_of(form, location)),
            _ => {}
        }
    }
    spans.sort_by_key(|span| (span.start, std::cmp::Reverse(span.end)));
    let mut outermost: Vec<Span> = Vec::new();
    for span in spans {
        if outermost.last().is_some_and(|kept| contains(*kept, span)) {
            continue;
        }
        outermost.push(span);
    }
    outermost
}

/// Whether `inner` lies inside `outer`.
fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// Whether `value_type` is a record type or an event type: the types an
/// attempt may be (FR-381 "Activation, attempts and deadline"). An object
/// type, a scalar, a collection, a tuple and a variant type are not.
fn is_record(scope: &Scope, value_type: &ValueType) -> bool {
    let ValueType::Composite(key) = value_type else {
        return false;
    };
    scope
        .types()
        .composite(*key)
        .is_some_and(|declaration| matches!(declaration.shape(), CompositeShape::Record(_)))
}

/// The refusal for a protocol whose declared types did not resolve while
/// `protocol_clause::content` refused none of them: a broken invariant of the
/// check stage, never an input defect.
pub(crate) fn unresolved_fault() -> CheckRefusal {
    CheckRefusal {
        location: Location::root(Origin::Expression),
        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
            quire_semantic_value::catalog::InternalFault::new(
                "check.protocol",
                "obligation-types-resolved",
            ),
        ))),
    }
}

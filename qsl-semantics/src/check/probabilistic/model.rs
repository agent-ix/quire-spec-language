// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185: the `random`, `reward` and `workload` declarations resolved,
//! typed and recorded on the checked graph.
//!
//! An operation is written `Type::op`, or `Alias::Type::op` to name the
//! model: `Type` is an object type's artifact id in a selected model, and
//! `op` resolves in that type's effective view (FR-104), so an inherited
//! operation is identified by its declaring type. A `random` or `reward`
//! operation resolves in every selected model, a workload weight only in the
//! workload's own model. A random parameter's values are typed against the
//! parameter's declared type and must each be a literal of it; a reward is
//! typed over the operation's parameters and must be an integer or a
//! quantity.

use std::collections::BTreeMap;

use qsl_forms::{ClauseKind, ExpressionForm, NameForm, RandomForm, RewardForm, WorkloadForm};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::Span;
use quire_exact::{EffectiveId, Identifier, Rational, Value, ValueType};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::{Location, Origin};

use super::super::check::{bind_parameters, Scope, Signatures, Typer};
use super::super::facts::Definedness;
use super::super::ir::{CheckedBody, CheckedLiteral, CheckedNode, DispatchTable, NodeKind};
use super::super::refusal::{CheckCause, CheckRefusal, KeyFault};
use super::super::state_clause::ClauseOperation;
use super::super::AdmittedModel;
use super::claim::CheckedProbabilisticClaim;
use super::{exact, refusal, ProbabilisticCause, ProbabilisticDeclaration, WeightOwner};
use crate::family::CheckContext;
use crate::model::key::DeclarationKey;
use crate::model::operation::{OperationDeclaration, OperationLookup};

/// An operation a random parameter, a reward or a workload weight names,
/// resolved (FR-185).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilisticOperation {
    model: String,
    key: DeclarationKey,
    declaring: EffectiveId,
    name: String,
}

impl ProbabilisticOperation {
    /// The `model` alias whose domain package declares the operation.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The operation's declaration key in its domain package: its identity.
    pub fn key(&self) -> &DeclarationKey {
        &self.key
    }

    /// The object type that declares the operation.
    pub fn declaring(&self) -> EffectiveId {
        self.declaring
    }

    /// The operation's name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// One value of a random parameter's distribution (FR-185 "Random
/// parameters").
#[derive(Clone, Debug, PartialEq)]
pub struct SupportValue {
    /// The value, of the parameter's type, compared by its canonical key.
    pub value: CheckedLiteral,
    /// Its declared weight, reduced, positive.
    pub weight: Rational,
    /// Its probability: the weight over the sum of the distribution's
    /// weights, in lowest terms.
    pub probability: Rational,
}

/// A checked random parameter: an operation parameter with its distribution
/// (FR-185 "Outputs").
#[derive(Clone, Debug, PartialEq)]
pub struct RandomParameter {
    /// The declaration's index among the unit's probabilistic
    /// declarations: its source order.
    declared: usize,
    operation: ProbabilisticOperation,
    parameter: String,
    position: usize,
    value_type: ValueType,
    support: Vec<SupportValue>,
}

impl RandomParameter {
    /// The operation the parameter belongs to.
    pub fn operation(&self) -> &ProbabilisticOperation {
        &self.operation
    }

    /// The parameter's name.
    pub fn parameter(&self) -> &str {
        &self.parameter
    }

    /// The parameter's position among the operation's parameters.
    pub fn position(&self) -> usize {
        self.position
    }

    /// The parameter's declared type.
    pub fn value_type(&self) -> &ValueType {
        &self.value_type
    }

    /// The distribution, in declared order: the parameter's finite support.
    pub fn support(&self) -> &[SupportValue] {
        &self.support
    }
}

/// A checked workload: one positive weight per operation of its model
/// (FR-185 "Workloads").
#[derive(Clone, Debug, PartialEq)]
pub struct Workload {
    /// The declaration's source order, as [`RandomParameter`]'s.
    declared: usize,
    name: String,
    model: String,
    weights: Vec<(ProbabilisticOperation, Rational)>,
}

impl Workload {
    /// The workload's declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The `model` alias the workload is declared on.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Each operation of the model with its weight, reduced, in declared
    /// order.
    pub fn weights(&self) -> &[(ProbabilisticOperation, Rational)] {
        &self.weights
    }
}

/// A checked reward: a named integer or quantity expression over an
/// operation's parameters (FR-185 "Rewards").
#[derive(Debug)]
pub struct Reward {
    /// The declaration's source order, as [`RandomParameter`]'s.
    declared: usize,
    name: String,
    operation: ProbabilisticOperation,
    parameters: Vec<(String, ValueType)>,
    body: CheckedBody,
    slots: usize,
}

impl Reward {
    /// The reward's declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The operation the reward is declared on.
    pub fn operation(&self) -> &ProbabilisticOperation {
        &self.operation
    }

    /// The reward's value type: an integer or a quantity type.
    pub fn value_type(&self) -> &ValueType {
        self.body.root().value_type()
    }

    /// The operation's parameters, each with its type, in slot order: the
    /// expression reads parameter `i` at slot `i` and nothing else of the
    /// operation.
    pub fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }

    /// The checked expression.
    pub fn expression(&self) -> CheckedNode<'_> {
        self.body.root()
    }

    /// The evaluation slot count.
    pub fn slots(&self) -> usize {
        self.slots
    }
}

/// Every checked `random`, `reward` and `workload` declaration of a package,
/// each in declaration order (FR-185 "Outputs").
#[derive(Debug, Default)]
pub struct CheckedProbabilisticModel {
    random_parameters: Vec<RandomParameter>,
    pub(super) rewards: Vec<Reward>,
    workloads: Vec<Workload>,
    pub(super) claims: Vec<CheckedProbabilisticClaim>,
}

impl CheckedProbabilisticModel {
    /// Every random parameter.
    pub fn random_parameters(&self) -> &[RandomParameter] {
        &self.random_parameters
    }

    /// The random parameter `parameter` of the operation `operation`.
    pub fn random_parameter(
        &self,
        operation: &DeclarationKey,
        parameter: &str,
    ) -> Option<&RandomParameter> {
        self.random_parameters.iter().find(|random| {
            random.operation.key == *operation && random.parameter.as_str() == parameter
        })
    }

    /// Every reward.
    pub fn rewards(&self) -> &[Reward] {
        &self.rewards
    }

    /// Every workload.
    pub fn workloads(&self) -> &[Workload] {
        &self.workloads
    }

    /// The workload declared `name`.
    pub fn workload(&self, name: &str) -> Option<&Workload> {
        self.workloads
            .iter()
            .find(|workload| workload.name.as_str() == name)
    }

    /// Every checked claim.
    pub fn claims(&self) -> &[CheckedProbabilisticClaim] {
        &self.claims
    }

    /// The claim named `name`.
    pub fn claim(&self, name: &str) -> Option<&CheckedProbabilisticClaim> {
        self.claims.iter().find(|claim| claim.name == name)
    }

    /// Whether the package declares none of the four.
    pub fn is_empty(&self) -> bool {
        self.random_parameters.is_empty()
            && self.rewards.is_empty()
            && self.workloads.is_empty()
            && self.claims.is_empty()
    }

    /// The first checked declaration in source order, with its feature:
    /// what S4 names when it refuses the package (FR-185 "Package
    /// emission"). A random parameter is named `operation(parameter)`, a
    /// reward, a workload and a claim by their declared names.
    pub fn first_declaration(&self) -> Option<(ProbabilisticFeature, String)> {
        let random = self.random_parameters.iter().map(|random| {
            (
                random.declared,
                ProbabilisticFeature::RandomParameter,
                format!("{}({})", random.operation.name, random.parameter),
            )
        });
        let rewards = self.rewards.iter().map(|reward| {
            (
                reward.declared,
                ProbabilisticFeature::Reward,
                reward.name.clone(),
            )
        });
        let workloads = self.workloads.iter().map(|workload| {
            (
                workload.declared,
                ProbabilisticFeature::Workload,
                workload.name.clone(),
            )
        });
        let claims = self.claims.iter().map(|claim| {
            (
                claim.declared,
                ProbabilisticFeature::Claim,
                claim.name.clone(),
            )
        });
        random
            .chain(rewards)
            .chain(workloads)
            .chain(claims)
            .min_by_key(|(declared, _, _)| *declared)
            .map(|(_, feature, name)| (feature, name))
    }
}

/// The kind of a probabilistic declaration, by its feature name (FR-185
/// "Package emission").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilisticFeature {
    /// A `random` declaration: `random-parameter`.
    RandomParameter,
    /// A `workload` declaration: `workload`.
    Workload,
    /// A `reward` declaration: `reward`.
    Reward,
    /// A `probabilistic` claim: `probabilistic-claim`.
    Claim,
}

impl ProbabilisticFeature {
    /// The feature name.
    pub fn name(self) -> &'static str {
        match self {
            Self::RandomParameter => "random-parameter",
            Self::Workload => "workload",
            Self::Reward => "reward",
            Self::Claim => "probabilistic-claim",
        }
    }
}

/// Which precondition reads a parameter (FR-185 "Random parameters").
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PreconditionSite {
    /// The unit's `pre` clause of this name.
    Clause(String),
    /// The domain package's `requires` clause at this position of its
    /// operation.
    Requires(usize),
}

/// One read of an operation parameter by one of the operation's checked
/// preconditions, located at the read.
#[derive(Clone, Debug)]
pub(crate) struct PreconditionRead {
    declaring: EffectiveId,
    operation: String,
    parameter: String,
    site: PreconditionSite,
    location: Location,
}

/// Every read of a parameter of `operation` in a precondition whose
/// parameters are `parameters` (the operation's last, after `self`) and
/// whose checked body is `body`.
pub(crate) fn precondition_reads(
    operation: &ClauseOperation,
    parameters: &[(String, ValueType)],
    body: CheckedNode<'_>,
    site: &PreconditionSite,
) -> Vec<PreconditionRead> {
    let declared = operation.declaration.parameters();
    let first = parameters.len().saturating_sub(declared.len());
    body.descendants()
        .into_iter()
        .filter_map(|node| match node.kind() {
            NodeKind::Local(slot) if *slot >= first => {
                let (parameter, _) = declared.get(slot - first)?;
                Some(PreconditionRead {
                    declaring: operation.declaring,
                    operation: operation.declaration.name().to_owned(),
                    parameter: parameter.clone(),
                    site: site.clone(),
                    location: node.location().clone(),
                })
            }
            _ => None,
        })
        .collect()
}

/// A located site of a probabilistic declaration, by the declaration's
/// index among the unit's probabilistic declarations.
#[derive(Clone, Copy, Debug)]
pub(super) enum Site {
    /// The whole declaration.
    Declaration,
    /// The `value` of a random parameter's distribution pair.
    Value(usize),
    /// The weight of a distribution pair or of a workload operation.
    Weight(usize),
    /// A reward's expression.
    Reward,
    /// The expression at this position of a claim's
    /// [`claim_expressions`](super::claim::claim_expressions).
    Expression(usize),
    /// The exact value at this position of a claim's
    /// [`claim_exact_values`](super::claim::claim_exact_values).
    Exact(usize),
    /// The operator at this arena index of a claim's temporal event.
    Operator(usize),
    /// A random declaration's parameter name.
    Parameter,
}

/// The `Origin::TypeDeclaration` site name of `site` of declaration
/// `declaration`. Spaced, so it names no declared type.
fn site_name(declaration: usize, site: Site) -> String {
    match site {
        Site::Declaration => format!("probabilistic {declaration}"),
        Site::Value(pair) => format!("probabilistic {declaration} value {pair}"),
        Site::Weight(pair) => format!("probabilistic {declaration} weight {pair}"),
        Site::Reward => format!("probabilistic {declaration} reward"),
        Site::Expression(position) => {
            format!("probabilistic {declaration} expression {position}")
        }
        Site::Exact(position) => format!("probabilistic {declaration} exact {position}"),
        Site::Operator(node) => format!("probabilistic {declaration} operator {node}"),
        Site::Parameter => format!("probabilistic {declaration} parameter"),
    }
}

pub(super) fn site_location(declaration: usize, site: Site) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: site_name(declaration, site),
    })
}

/// The spans of a site with no expression tree: its own span alone.
fn spanned(span: Span) -> Option<qsl_forms::ExpressionSpans> {
    qsl_forms::ExpressionSpans::new(span).ok()
}

/// The expression spans of every site of `declarations`, by site name, for
/// the unit's region map (FR-096).
pub(crate) fn site_spans(
    declarations: &[ProbabilisticDeclaration],
) -> impl Iterator<Item = (String, qsl_forms::ExpressionSpans)> + '_ {
    declarations
        .iter()
        .enumerate()
        .flat_map(|(index, declaration)| {
            let mut sites: Vec<(Site, Option<qsl_forms::ExpressionSpans>)> = Vec::new();
            match declaration {
                ProbabilisticDeclaration::Random(form) => {
                    sites.push((Site::Declaration, spanned(form.span)));
                    sites.push((Site::Parameter, spanned(form.parameter.span)));
                    for (pair, value) in form.support.iter().enumerate() {
                        sites.push((Site::Value(pair), Some(value.value.spans.clone())));
                        sites.push((Site::Weight(pair), spanned(value.weight.span)));
                    }
                }
                ProbabilisticDeclaration::Reward(form) => {
                    sites.push((Site::Reward, Some(form.expression.spans.clone())));
                }
                ProbabilisticDeclaration::Workload(form) => {
                    for (weight, written) in form.weights.iter().enumerate() {
                        sites.push((Site::Weight(weight), spanned(written.weight.span)));
                    }
                }
                ProbabilisticDeclaration::Claim(form) => {
                    for (position, expression) in super::claim::claim_expressions(form)
                        .into_iter()
                        .enumerate()
                    {
                        sites.push((Site::Expression(position), Some(expression.spans.clone())));
                    }
                    if let qsl_forms::ProbabilisticBody::Probability {
                        event: qsl_forms::EventForm::Temporal(formula),
                        ..
                    } = &form.body
                    {
                        for (id, node) in formula.iter() {
                            if let qsl_forms::TemporalNodeForm::Operator(operator) = node {
                                sites.push((
                                    Site::Operator(id.index()),
                                    spanned(operator.operator_span),
                                ));
                            }
                        }
                    }
                    for (position, value) in super::claim::claim_exact_values(form)
                        .into_iter()
                        .enumerate()
                    {
                        sites.push((Site::Exact(position), spanned(value.span)));
                    }
                }
            }
            sites
                .into_iter()
                .filter_map(move |(site, spans)| Some((site_name(index, site), spans?)))
        })
}

/// What the declarations are resolved and typed against.
pub(crate) struct ProbabilisticInput<'a> {
    pub(crate) scope: &'a Scope,
    pub(crate) signatures: &'a Signatures,
    pub(crate) dispatch_tables: &'a [DispatchTable],
    pub(crate) models: &'a [AdmittedModel],
    /// Each `model` alias's object types, by artifact id.
    pub(crate) model_objects: &'a BTreeMap<String, BTreeMap<Identifier, DeclarationKey>>,
    /// Each object type's effective identity, by its declaration key.
    pub(crate) object_types: &'a BTreeMap<DeclarationKey, EffectiveId>,
    /// Every read of an operation parameter by a checked precondition.
    pub(crate) preconditions: &'a [PreconditionRead],
    /// Every header profile selection of the unit.
    pub(crate) profiles: &'a [qsl_foundation::selection::ProfileSelection],
    pub(crate) limits: CheckingLimits,
    pub(crate) cancel: &'a quire_exact::Cancel,
}

/// Why an operation name resolves to no single operation.
pub(super) enum Unresolved {
    Missing,
    Ambiguous,
}

impl<'a> ProbabilisticInput<'a> {
    /// The operation `written` names: in the model `model` only when given,
    /// else in every selected model.
    pub(super) fn operation(
        &self,
        written: &NameForm,
        model: Option<&str>,
    ) -> Result<(ProbabilisticOperation, &'a OperationDeclaration), Unresolved> {
        let segments: Vec<&str> = written.name.split("::").collect();
        let (alias, object, operation) = match segments[..] {
            [alias, object, operation] => (Some(alias), object, operation),
            [object, operation] => (None, object, operation),
            _ => return Err(Unresolved::Missing),
        };
        let object = Identifier::new(object).map_err(|_| Unresolved::Missing)?;
        let candidates: Vec<(&String, &DeclarationKey)> = self
            .model_objects
            .iter()
            .filter(|(candidate, _)| {
                alias.is_none_or(|alias| candidate.as_str() == alias)
                    && model.is_none_or(|model| candidate.as_str() == model)
            })
            .filter_map(|(candidate, objects)| Some((candidate, objects.get(&object)?)))
            .collect();
        let (alias, key) = match candidates[..] {
            [] => return Err(Unresolved::Missing),
            [found] => found,
            _ => return Err(Unresolved::Ambiguous),
        };
        let context = *self.object_types.get(key).ok_or(Unresolved::Missing)?;
        let (declaring, declaration) =
            match self
                .scope
                .operations()
                .resolve(self.scope.types(), context, operation)
            {
                OperationLookup::Declared {
                    declaring,
                    operation,
                } => (declaring, operation),
                OperationLookup::Missing => return Err(Unresolved::Missing),
                OperationLookup::Ambiguous(_) => return Err(Unresolved::Ambiguous),
            };
        let resolved = ClauseOperation {
            declaring,
            declaration: declaration.clone(),
        };
        let key = self
            .models
            .iter()
            .filter(|candidate| candidate.alias() == Some(alias.as_str()))
            .find_map(|candidate| candidate.operation_key(&resolved))
            .ok_or(Unresolved::Missing)?;
        Ok((
            ProbabilisticOperation {
                model: alias.clone(),
                key: key.clone(),
                declaring,
                name: declaration.name().to_owned(),
            },
            declaration,
        ))
    }

    /// Every operation of the model `alias`, each with its spelling
    /// `Type::op`, in object type then declaration order.
    fn model_operations(&self, alias: &str) -> Vec<(EffectiveId, &'a str, String)> {
        let Some(objects) = self.model_objects.get(alias) else {
            return Vec::new();
        };
        let mut operations = Vec::new();
        for (artifact, key) in objects {
            let Some(&declaring) = self.object_types.get(key) else {
                continue;
            };
            for operation in self.scope.operations().declared_by(declaring) {
                operations.push((
                    declaring,
                    operation.name(),
                    format!("{}::{}", artifact.as_str(), operation.name()),
                ));
            }
        }
        operations
    }

    /// A typer over this input's scope, charging `nodes`.
    pub(super) fn typer<'t>(
        &'t self,
        nodes: &'t mut u64,
        cx: &'t mut CheckContext<'_, Scope>,
    ) -> Typer<'t>
    where
        'a: 't,
    {
        Typer::new(
            self.scope,
            self.signatures,
            self.limits,
            nodes,
            ClauseKind::Body,
        )
        .with_cancel(Some(self.cancel))
        .with_context(cx.reborrow(self.scope))
    }
}

/// The refusal of an operation name that resolves to no single operation.
pub(super) fn unresolved(unresolved: Unresolved, written: &NameForm) -> ProbabilisticCause {
    match unresolved {
        Unresolved::Missing => ProbabilisticCause::UnresolvedOperation {
            name: written.name.clone(),
            span: written.span,
        },
        Unresolved::Ambiguous => ProbabilisticCause::AmbiguousOperation {
            name: written.name.clone(),
            span: written.span,
        },
    }
}

/// An exact weight at `location`: a positive rational, or the refusal it is.
fn weight(
    value: &qsl_forms::ExactValueForm,
    ceiling: u64,
    location: Location,
    owner: impl FnOnce() -> WeightOwner,
) -> Result<Rational, CheckRefusal> {
    let span = value.span;
    let rational = exact::value(value, ceiling)
        .map_err(|fault| super::exact_refusal(fault, ceiling, span, location))?;
    if rational.is_zero() || rational.numerator().is_negative() {
        return Err(refusal(ProbabilisticCause::InvalidWeight {
            owner: owner(),
            span,
        }));
    }
    Ok(rational)
}

/// The literal value a checked support value is, admitted into its type's
/// integer interval when it is coerced into one.
pub(super) fn literal(node: CheckedNode<'_>) -> Option<CheckedLiteral> {
    match node.kind() {
        NodeKind::Literal(literal) => Some(literal.clone()),
        NodeKind::Coerce(operand, interval) => match node.at(*operand).kind() {
            NodeKind::Literal(CheckedLiteral(Value::Integer(value)))
                if interval.contains(value) =>
            {
                Some(CheckedLiteral(Value::Integer(value.clone())))
            }
            _ => None,
        },
        _ => None,
    }
}

/// A broken invariant of this check, never a refusal of the input.
fn fault(location: Location, invariant: &'static str) -> CheckRefusal {
    CheckRefusal {
        location,
        cause: CheckCause::InternalFault(Box::new(KeyFault::StageFault(
            qsl_foundation::InternalFault::new("S3", invariant),
        ))),
    }
}

/// The checks of one pass, and what they recorded.
pub(crate) struct Checked {
    pub(crate) model: CheckedProbabilisticModel,
    pub(crate) refusals: Vec<CheckRefusal>,
}

/// Checks the `random`, `reward` and `workload` declarations of
/// `declarations`, charging `nodes`; a claim is checked by
/// [`super::claim`].
pub(crate) fn check(
    declarations: &[ProbabilisticDeclaration],
    input: &ProbabilisticInput<'_>,
    cx: &mut CheckContext<'_, Scope>,
    nodes: &mut u64,
) -> Checked {
    let mut pass = Pass {
        input,
        model: CheckedProbabilisticModel::default(),
        distributions: Vec::new(),
        rewards: Vec::new(),
        workloads: Vec::new(),
        refusals: Vec::new(),
    };
    for (index, declaration) in declarations.iter().enumerate() {
        match declaration {
            ProbabilisticDeclaration::Random(form) => pass.random(index, form, cx, nodes),
            ProbabilisticDeclaration::Reward(form) => pass.reward(index, form, cx, nodes),
            ProbabilisticDeclaration::Workload(form) => pass.workload(index, form),
            ProbabilisticDeclaration::Claim(_) => {}
        }
    }
    // A claim names workloads and rewards declared anywhere in the unit, so
    // it is checked once every declaration is.
    for (index, declaration) in declarations.iter().enumerate() {
        if let ProbabilisticDeclaration::Claim(form) = declaration {
            pass.claim(index, form, cx, nodes);
        }
    }
    pass.preconditions();
    Checked {
        model: pass.model,
        refusals: pass.refusals,
    }
}

pub(super) struct Pass<'i, 'a> {
    pub(super) input: &'i ProbabilisticInput<'a>,
    pub(super) model: CheckedProbabilisticModel,
    /// Each (operation, parameter) a `random` declaration resolved to, so a
    /// second is a duplicate, and a precondition read of it refuses,
    /// whether or not its distribution was admitted.
    distributions: Vec<(ProbabilisticOperation, String)>,
    /// Each (operation, name) a `reward` declaration resolved to.
    rewards: Vec<(DeclarationKey, String)>,
    /// Each workload name declared so far, with its name span.
    workloads: Vec<(String, Span)>,
    pub(super) refusals: Vec<CheckRefusal>,
}

impl Pass<'_, '_> {
    fn refuse(&mut self, cause: ProbabilisticCause) {
        self.refusals.push(refusal(cause));
    }

    fn random(
        &mut self,
        index: usize,
        form: &RandomForm,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
    ) {
        let (operation, declaration) = match self.input.operation(&form.operation, None) {
            Ok(resolved) => resolved,
            Err(fault) => return self.refuse(unresolved(fault, &form.operation)),
        };
        let parameter = form.parameter.name.as_str();
        let Some((position, (_, value_type))) = declaration
            .parameters()
            .iter()
            .enumerate()
            .find(|(_, (name, _))| name.as_str() == parameter)
        else {
            return self.refuse(ProbabilisticCause::UnresolvedParameter {
                operation: form.operation.name.clone(),
                parameter: parameter.to_owned(),
                span: form.parameter.span,
            });
        };
        let names = || (form.operation.name.clone(), parameter.to_owned());
        if self
            .distributions
            .iter()
            .any(|(earlier, name)| earlier.key == operation.key && name.as_str() == parameter)
        {
            let (operation, parameter) = names();
            return self.refuse(ProbabilisticCause::DuplicateDistribution {
                operation,
                parameter,
                span: form.span,
            });
        }
        self.distributions
            .push((operation.clone(), parameter.to_owned()));
        let mut refusals: Vec<CheckRefusal> = Vec::new();
        let mut support: Vec<(CheckedLiteral, Rational)> = Vec::with_capacity(form.support.len());
        let mut seen: Vec<CheckedLiteral> = Vec::with_capacity(form.support.len());
        for (pair_index, pair) in form.support.iter().enumerate() {
            let weight = weight(
                &pair.weight,
                self.input.limits.rational_bits(),
                site_location(index, Site::Weight(pair_index)),
                || {
                    let (operation, parameter) = names();
                    WeightOwner::Parameter {
                        operation,
                        parameter,
                    }
                },
            );
            let value_span = pair.value.spans.at(&[]).unwrap_or(pair.span);
            let location = site_location(index, Site::Value(pair_index));
            let typed =
                self.input
                    .typer(nodes, cx)
                    .check_as(&pair.value.expression, value_type, &location);
            let value = match typed {
                // A support value is never a reference, whatever the declared
                // type (FR-185 "Random parameters"): a reference into a
                // population is an identity, which a distribution must not
                // weigh.
                Ok(body) => {
                    literal(body.root()).filter(|literal| !matches!(literal.0, Value::Reference(_)))
                }
                Err(refusal) if refusal.cause.code() == Code::IllTyped => None,
                Err(refusal) => {
                    refusals.extend(weight.err());
                    refusals.push(refusal);
                    continue;
                }
            };
            let Some(value) = value else {
                refusals.extend(weight.err());
                let (operation, parameter) = names();
                refusals.push(refusal(ProbabilisticCause::ValueOutsideType {
                    operation,
                    parameter,
                    span: value_span,
                }));
                continue;
            };
            if seen.contains(&value) {
                let (operation, parameter) = names();
                refusals.push(refusal(ProbabilisticCause::RepeatedSupportValue {
                    operation,
                    parameter,
                    span: value_span,
                }));
            }
            seen.push(value.clone());
            match weight {
                Ok(weight) => support.push((value, weight)),
                Err(refused) => refusals.push(refused),
            }
        }
        if !refusals.is_empty() || support.len() != form.support.len() {
            self.refusals.extend(refusals);
            return;
        }
        match self.probabilities(index, support) {
            Ok(support) => self.model.random_parameters.push(RandomParameter {
                declared: index,
                operation,
                parameter: parameter.to_owned(),
                position,
                value_type: value_type.clone(),
                support,
            }),
            Err(refused) => self.refusals.push(refused),
        }
    }

    /// Each value's probability: its weight over the sum of the weights.
    /// The running sum is bounded by the `s3.rational_bits` ceiling, as a written
    /// value is, measured after each addition, so in-limit weights with
    /// coprime denominators cannot grow it without bound; its refusal is
    /// located at the parameter it names. The handle is polled before each
    /// addition (FR-185 "Exact values").
    fn probabilities(
        &self,
        index: usize,
        support: Vec<(CheckedLiteral, Rational)>,
    ) -> Result<Vec<SupportValue>, CheckRefusal> {
        let location = site_location(index, Site::Declaration);
        let ceiling = self.input.limits.rational_bits();
        let mut total = Rational::from_integer(quire_exact::Integer::zero());
        for (_, weight) in &support {
            if self.input.cancel.poll() {
                return Err(super::super::check::cancelled_refusal(&location));
            }
            total = total.add(weight);
            let bits = total.max_part_bits();
            if bits > ceiling {
                return Err(super::limit_refusal(
                    super::Excess { bits, ceiling },
                    site_location(index, Site::Parameter),
                ));
            }
        }
        support
            .into_iter()
            .map(|(value, weight)| {
                if self.input.cancel.poll() {
                    return Err(super::super::check::cancelled_refusal(&location));
                }
                // `total` is a sum of positive weights, so it is not zero.
                let probability = weight.div(&total).ok_or_else(|| {
                    fault(location.clone(), "probabilistic-weight-sum-is-nonzero")
                })?;
                Ok(SupportValue {
                    value,
                    weight,
                    probability,
                })
            })
            .collect()
    }

    fn reward(
        &mut self,
        index: usize,
        form: &RewardForm,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
    ) {
        let reads = super::reward(form);
        let (operation, declaration) = match self.input.operation(&form.operation, None) {
            Ok(resolved) => resolved,
            Err(fault) => {
                self.refusals.extend(reads.into_iter().map(refusal));
                return self.refuse(unresolved(fault, &form.operation));
            }
        };
        let name = form.name.name.as_str();
        let declared = (operation.key.clone(), name.to_owned());
        if self.rewards.contains(&declared) {
            self.refusals.extend(reads.into_iter().map(refusal));
            return self.refuse(ProbabilisticCause::DuplicateReward {
                reward: name.to_owned(),
                operation: form.operation.name.clone(),
                span: form.span,
            });
        }
        self.rewards.push(declared);
        if !reads.is_empty() {
            self.refusals.extend(reads.into_iter().map(refusal));
            return;
        }
        let parameters = declaration.parameters().to_vec();
        let location = site_location(index, Site::Reward);
        match self.typed_reward(form, &parameters, &location, cx, nodes) {
            Ok((body, slots)) => {
                let typed = body.root().value_type();
                if !matches!(
                    typed,
                    ValueType::Integer | ValueType::Int(_) | ValueType::Quantity(_)
                ) {
                    return self.refuse(ProbabilisticCause::RewardType {
                        reward: name.to_owned(),
                        operation: form.operation.name.clone(),
                        span: expression_span(&form.expression, form.span),
                    });
                }
                self.model.rewards.push(Reward {
                    declared: index,
                    name: name.to_owned(),
                    operation,
                    parameters,
                    body,
                    slots,
                });
            }
            Err(refusal) => self.refusals.push(refusal),
        }
    }

    /// The reward's expression typed over the operation's parameters, with
    /// its definedness checked.
    fn typed_reward(
        &self,
        form: &RewardForm,
        parameters: &[(String, ValueType)],
        location: &Location,
        cx: &mut CheckContext<'_, Scope>,
        nodes: &mut u64,
    ) -> Result<(CheckedBody, usize), CheckRefusal> {
        let mut typer = self.input.typer(nodes, cx);
        bind_parameters(&mut typer, parameters, location)?;
        let body = typer.infer(&form.expression.expression, None, location)?;
        let slots = typer.slots();
        Definedness::new(
            parameters.len(),
            self.input.dispatch_tables,
            &self.input.scope.dispatch_operations,
        )
        .check(body.root())?;
        Ok((body, slots))
    }

    fn workload(&mut self, index: usize, form: &WorkloadForm) {
        let workload = form.name.name.as_str();
        let model = form.model.name.as_str();
        // A claim names a workload by its name alone (FR-186), so a name
        // names one workload of the package, whichever model it is on.
        if let Some((_, first)) = self
            .workloads
            .iter()
            .find(|(earlier, _)| earlier.as_str() == workload)
        {
            let first = *first;
            return self.refuse(ProbabilisticCause::DuplicateWorkload {
                workload: workload.to_owned(),
                span: form.name.span,
                related: [first, form.name.span],
            });
        }
        self.workloads.push((workload.to_owned(), form.name.span));
        if !self.input.model_objects.contains_key(model) {
            return self.refuse(ProbabilisticCause::UnresolvedModel {
                workload: workload.to_owned(),
                model: model.to_owned(),
                span: form.model.span,
            });
        }
        let mut refusals: Vec<CheckRefusal> = Vec::new();
        let mut weighted: Vec<DeclarationKey> = Vec::new();
        let mut weights: Vec<(ProbabilisticOperation, Rational)> = Vec::new();
        for (position, written) in form.weights.iter().enumerate() {
            let weight = weight(
                &written.weight,
                self.input.limits.rational_bits(),
                site_location(index, Site::Weight(position)),
                || WeightOwner::Workload {
                    workload: workload.to_owned(),
                    operation: written.operation.name.clone(),
                },
            );
            let operation = match self.input.operation(&written.operation, Some(model)) {
                Ok((operation, _)) => operation,
                Err(Unresolved::Missing) => {
                    refusals.extend(weight.err());
                    refusals.push(refusal(ProbabilisticCause::UnknownOperation {
                        workload: workload.to_owned(),
                        operation: written.operation.name.clone(),
                        span: written.operation.span,
                    }));
                    continue;
                }
                Err(fault @ Unresolved::Ambiguous) => {
                    refusals.extend(weight.err());
                    refusals.push(refusal(unresolved(fault, &written.operation)));
                    continue;
                }
            };
            if weighted.contains(&operation.key) {
                refusals.extend(weight.err());
                refusals.push(refusal(ProbabilisticCause::DuplicateWeight {
                    workload: workload.to_owned(),
                    operation: written.operation.name.clone(),
                    span: written.span,
                }));
                continue;
            }
            weighted.push(operation.key.clone());
            match weight {
                Ok(weight) => weights.push((operation, weight)),
                Err(refused) => refusals.push(refused),
            }
        }
        for (declaring, name, spelled) in self.input.model_operations(model) {
            let covered = form.weights.iter().any(|written| {
                self.input
                    .operation(&written.operation, Some(model))
                    .is_ok_and(|(operation, _)| {
                        operation.declaring == declaring && operation.name.as_str() == name
                    })
            });
            if !covered {
                refusals.push(refusal(ProbabilisticCause::IncompleteWorkload {
                    workload: workload.to_owned(),
                    operation: spelled,
                    span: form.span,
                }));
            }
        }
        if !refusals.is_empty() {
            self.refusals.extend(refusals);
            return;
        }
        self.model.workloads.push(Workload {
            declared: index,
            name: workload.to_owned(),
            model: model.to_owned(),
            weights,
        });
    }

    /// Every precondition read of a random parameter refuses at the read,
    /// whether or not the parameter's distribution was admitted.
    fn preconditions(&mut self) {
        for read in self.input.preconditions {
            let random = self.distributions.iter().any(|(operation, parameter)| {
                operation.declaring == read.declaring
                    && operation.name.as_str() == read.operation.as_str()
                    && parameter.as_str() == read.parameter.as_str()
            });
            if random {
                self.refusals.push(CheckRefusal {
                    location: read.location.clone(),
                    cause: CheckCause::Probabilistic(Box::new(
                        ProbabilisticCause::RandomReadInPrecondition {
                            operation: read.operation.clone(),
                            parameter: read.parameter.clone(),
                            precondition: read.site.clone(),
                        },
                    )),
                });
            }
        }
    }
}

/// The span of `expression`'s root, or `fallback`.
fn expression_span(expression: &ExpressionForm, fallback: Span) -> Span {
    expression.spans.at(&[]).unwrap_or(fallback)
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hyper product closure evidence and independent core checking (FR-163).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use filament_exact::{Cancel, ScalarLimits, ValueType};
use fsl_eval::automaton::PropertyAutomaton;
use fsl_eval::fairness::{Analysis, Constraint, EdgeLabel, Graph, Identity, Outcome, Strength};
use fsl_eval::hyper::{
    BodySense, CheckedStepMatch, ComponentStep, CopySwapInputs, HyperFinding, HyperInputs,
    HyperKey, HyperProduct, HyperState, HyperTransition, ParameterBinding, ProjectedInputs,
    ProjectedProduct, Property, WitnessInputs, WitnessProduct,
};
use fsl_eval::model_subject::ModelSubject;
use fsl_eval::simulation::model::{CheckedHyperLetters, ModelSystem};
use fsl_eval::simulation::{StateKey, SymmetryGroup, TransitionSystem};
use fsl_foundation::IdentityLimits;
use fsl_package::CheckedPackage;
use fsl_semantics::check::{CheckedHyperClause, CopyGroup, HyperForm};

use super::closure::{CertificateRejection, TemporalCertificateCheck};
use super::scc::{check_partition, CertificateState, Component, ComponentWitness, ProductGraph};
use super::{CertificateLocus, CertificateRule};
use crate::{IncompleteCause, ModelCheckLimit, ModelCheckLimits};

/// The full canonical tuple and property key, including HP-3 witness members.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProductStateKey(pub StateKey);

impl serde::Serialize for ProductStateKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        product_key_wire(&self.0)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

pub(super) fn product_key_wire(key: &StateKey) -> Result<serde_json::Value, ProductClosureRefusal> {
    use base64::Engine as _;
    use serde_json::json;
    let encode = |bytes: &[u8]| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    let value: serde_json::Value = serde_json::from_slice(key.as_bytes())
        .map_err(|_| ProductClosureRefusal::Input("product-key"))?;
    let pair = value
        .as_array()
        .filter(|pair| pair.len() == 2)
        .ok_or(ProductClosureRefusal::Input("product-key"))?;
    let states = |values: &serde_json::Value| -> Result<Vec<String>, ProductClosureRefusal> {
        values
            .as_array()
            .ok_or(ProductClosureRefusal::Input("component-keys"))?
            .iter()
            .map(|value| {
                serde_json::to_vec(value)
                    .map(|bytes| encode(&bytes))
                    .map_err(|_| ProductClosureRefusal::Input("component-key"))
            })
            .collect()
    };
    let property = pair[1]
        .as_array()
        .ok_or(ProductClosureRefusal::Input("property-key"))?;
    let mut written = json!({"states": states(&pair[0])?});
    if property.first().and_then(serde_json::Value::as_str) == Some("witnesses") {
        let mut witnesses = Vec::new();
        for member in property.iter().skip(1) {
            let tuple = member
                .as_array()
                .filter(|tuple| !tuple.is_empty())
                .ok_or(ProductClosureRefusal::Input("witness-key"))?;
            let automaton = tuple
                .last()
                .and_then(serde_json::Value::as_str)
                .ok_or(ProductClosureRefusal::Input("witness-automaton"))?;
            let components = serde_json::Value::Array(tuple[..tuple.len() - 1].to_vec());
            witnesses.push(json!({"states": states(&components)?, "automaton_state": encode(automaton.as_bytes())}));
        }
        written["witness_set"] = json!(witnesses);
    } else {
        let bytes = if property.first().and_then(serde_json::Value::as_str) == Some("after") {
            property
                .get(1)
                .and_then(serde_json::Value::as_str)
                .ok_or(ProductClosureRefusal::Input("automaton-key"))?
                .as_bytes()
                .to_vec()
        } else {
            serde_json::to_vec(&pair[1])
                .map_err(|_| ProductClosureRefusal::Input("property-key"))?
        };
        written["automaton_state"] = json!(encode(&bytes));
    }
    Ok(written)
}

impl CertificateState for ProductStateKey {
    fn locus(self) -> CertificateLocus {
        CertificateLocus::HyperProductState(self.0)
    }
}

/// One binding's closed product and its component witnesses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductClosureCertificate {
    /// The classified hyperproperty form.
    pub form: HyperForm,
    /// Parameters and admitted universe keys, in declaration order.
    pub instance: Vec<(String, String)>,
    /// The copy group used to canonicalize members, if one was applied.
    pub reduction: Option<CopyGroup>,
    /// Sorted, duplicate-free canonical product members.
    pub closure: Vec<ProductStateKey>,
    /// HP-2/HP-3 component partition and its independently checked witnesses.
    pub components: Option<Vec<Component<ProductStateKey>>>,
}

/// The original checked inputs that the core uses to reconstruct the product.
pub struct ProductClosureRequest<'a> {
    /// The checked program that owns every atom and match.
    pub package: &'a CheckedPackage,
    /// The clause being proved.
    pub clause: &'a CheckedHyperClause,
    /// One source model subject per distinct alias, in clause alias order.
    /// A unit that selects several models carries, per alias, the snapshot
    /// provision reduced to that alias's model.
    pub subjects: &'a [ModelSubject<'a>],
    /// Atom and match accounting limits.
    pub meter: ScalarLimits,
}

/// A malformed original input or an invariant failure while reconstructing.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProductClosureRefusal {
    /// A budget or cancellation stopped product reconstruction.
    #[error("hyper product certificate recomputation stopped")]
    Stopped(IncompleteCause),
    /// The supplied subject or binding does not admit.
    #[error("hyper certificate input does not admit: {0}")]
    Input(&'static str),
    /// The product could not be constructed or expanded.
    #[error("hyper certificate recomputation failed: {0}")]
    Recompute(&'static str),
}

/// Canonicalize a product key without a digest or search-location identity.
///
/// # Errors
///
/// Refuses a key that has no bounded canonical encoding.
pub fn product_state_key(key: &HyperKey) -> Result<ProductStateKey, ProductClosureRefusal> {
    let bytes = quire_canonical::to_vec(
        key,
        quire_canonical::Limits::new(IdentityLimits::default().input_bytes),
    )
    .map_err(|_| ProductClosureRefusal::Recompute("product-key"))?;
    Ok(ProductStateKey(StateKey::from_canonical_bytes(bytes)))
}

fn rejected(rule: CertificateRule, key: ProductStateKey) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: key.locus(),
    })
}

/// The system of each alias over its own model (FR-173 "Event-position form"),
/// as the engine built it: a subject whose simulation selects several models
/// carries the snapshot provision reduced to that alias's model.
///
/// # Errors
///
/// A subject that does not admit.
pub(crate) fn alias_systems<'a>(
    request: &ProductClosureRequest<'a>,
) -> Result<Vec<ModelSystem<'a>>, ProductClosureRefusal> {
    request
        .subjects
        .iter()
        .zip(request.clause.aliases())
        .map(|(subject, alias)| {
            ModelSystem::for_alias(subject.simulation.clone(), alias.as_str())
                .map_err(|_| ProductClosureRefusal::Input("model-subject"))
        })
        .collect()
}

/// Require one certificate for every admitted parameter binding, in order.
pub(crate) fn complete_scope(
    request: &ProductClosureRequest<'_>,
    certificates: &[ProductClosureCertificate],
) -> Result<(), ProductClosureRefusal> {
    let systems = alias_systems(request)?;
    let mut domains = Vec::new();
    for parameter in request.clause.parameters() {
        let slot = request
            .clause
            .aliases()
            .iter()
            .position(|alias| alias == &parameter.alias)
            .ok_or(ProductClosureRefusal::Input("parameter-alias"))?;
        let system = systems
            .get(slot)
            .ok_or(ProductClosureRefusal::Input("parameter-subject"))?;
        let instances = system
            .instances(&ValueType::Reference(parameter.population.object()))
            .map_err(|_| ProductClosureRefusal::Input("parameter-universe"))?;
        domains.push(
            instances
                .into_iter()
                .map(|instance| (parameter.name.clone(), instance.key))
                .collect::<Vec<_>>(),
        );
    }
    let count = domains
        .iter()
        .try_fold(1_usize, |count, domain| count.checked_mul(domain.len()))
        .ok_or(ProductClosureRefusal::Input("instance-count"))?;
    if count == 0 || count != certificates.len() {
        return Err(ProductClosureRefusal::Input("certificate-scope"));
    }
    for (index, certificate) in certificates.iter().enumerate() {
        let mut ordinal = index;
        let mut expected = Vec::with_capacity(domains.len());
        for domain in domains.iter().rev() {
            let position = ordinal % domain.len();
            ordinal /= domain.len();
            expected.push(
                domain
                    .get(position)
                    .ok_or(ProductClosureRefusal::Input("instance-index"))?
                    .clone(),
            );
        }
        expected.reverse();
        if expected != certificate.instance {
            return Err(ProductClosureRefusal::Input("certificate-scope"));
        }
    }
    Ok(())
}

/// Independently reconstruct initial members, successors and fairness witnesses.
///
/// # Errors
///
/// Returns a refusal when the original program or its model inputs do not admit.
pub fn check_product_closure(
    request: &ProductClosureRequest<'_>,
    certificate: &ProductClosureCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, ProductClosureRefusal> {
    match check(request, certificate, limits, cancel) {
        Err(ProductClosureRefusal::Stopped(cause)) => Ok(TemporalCertificateCheck::Stopped(cause)),
        result => result,
    }
}

fn check(
    request: &ProductClosureRequest<'_>,
    certificate: &ProductClosureCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, ProductClosureRefusal> {
    for pair in certificate.closure.windows(2) {
        if pair[0] >= pair[1] {
            return Ok(rejected(CertificateRule::Malformed, pair[1].clone()));
        }
    }
    let needs_components = matches!(
        certificate.form,
        HyperForm::Universal | HyperForm::ForallExistsSafety
    );
    if needs_components && certificate.components.is_none() {
        return Ok(TemporalCertificateCheck::Rejected(CertificateRejection {
            rule: CertificateRule::Malformed,
            at: CertificateLocus::CertificateMember(super::CertificateMember::Components),
        }));
    }
    if request.clause.form() != Some(&certificate.form)
        || request.subjects.len() != request.clause.aliases().len()
    {
        return Err(ProductClosureRefusal::Input("clause-or-subjects"));
    }
    let systems = alias_systems(request)?;
    let slot = |alias: &str| {
        request
            .clause
            .aliases()
            .iter()
            .position(|known| known.as_str() == alias)
            .ok_or(ProductClosureRefusal::Input("model-alias"))
    };
    let variable_systems = request
        .clause
        .quantifiers()
        .iter()
        .map(|quantifier| {
            systems
                .get(slot(quantifier.alias.as_str())?)
                .ok_or(ProductClosureRefusal::Input("model-alias"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut instance = Vec::new();
    if certificate.instance.len() != request.clause.parameters().len() {
        return Err(ProductClosureRefusal::Input("instance"));
    }
    for (parameter, (name, key)) in request
        .clause
        .parameters()
        .iter()
        .zip(&certificate.instance)
    {
        if name != &parameter.name {
            return Err(ProductClosureRefusal::Input("parameter"));
        }
        let system = systems
            .get(slot(parameter.alias.as_str())?)
            .ok_or(ProductClosureRefusal::Input("parameter-alias"))?;
        let admitted = system
            .instances(&ValueType::Reference(parameter.population.object()))
            .map_err(|_| ProductClosureRefusal::Input("parameter-universe"))?;
        let found = admitted
            .into_iter()
            .find(|instance| &instance.key == key)
            .ok_or(ProductClosureRefusal::Input("instance-key"))?;
        instance.push(ParameterBinding {
            parameter: name.clone(),
            instance: found,
        });
    }
    let letters =
        CheckedHyperLetters::new(request.package, request.clause, variable_systems.clone())
            .map_err(|_| ProductClosureRefusal::Input("letters"))?;
    let matcher = CheckedStepMatch::new(request.package, request.clause);
    let copy = certificate.reduction.as_ref();
    if let Some(copy) = copy {
        if copy != &fsl_eval::hyper::fairness_preserving_copy_group(request.clause) {
            return Err(ProductClosureRefusal::Input("copy-group"));
        }
    }
    let group = copy.map(fsl_eval::hyper::copy_symmetry);
    let variables: Vec<&str> = request
        .clause
        .quantifiers()
        .iter()
        .map(|q| q.variable.as_str())
        .collect();
    let mut product = Product::build(ProductBuild {
        clause: request.clause,
        instance: &instance,
        systems: variable_systems.clone(),
        letters: &letters,
        matcher: &matcher,
        copy_swap: copy.map(|group| CopySwapInputs {
            variables: variables.clone(),
            group,
        }),
        limits: *limits,
        meter: request.meter,
        cancel,
    })?;
    if product.start().is_err() {
        return Err(product_stop(&product, request, limits)?);
    }
    let members: BTreeSet<ProductStateKey> = certificate.closure.iter().cloned().collect();
    let mut pending = VecDeque::new();
    let mut seen = BTreeMap::new();
    for state in product.initial() {
        let state = canonical(&product, state, group.as_ref())?;
        let key = product_state_key(&product.key(&state))?;
        if !members.contains(&key) {
            return Ok(rejected(CertificateRule::InitialMissing, key));
        }
        if !seen.contains_key(&key) {
            seen.insert(key.clone(), seen.len());
            pending.push_back((key, state));
        }
    }
    if seen.is_empty() {
        return Err(ProductClosureRefusal::Input("no-initial-state"));
    }
    let mut retained = Vec::new();
    let mut edges = Vec::new();
    let mut undefined = None;
    let mut violating = None;
    while let Some((key, state)) = pending.pop_front() {
        if cancel.poll() {
            if let Some(source) = cancel.cause() {
                return Ok(TemporalCertificateCheck::Stopped(
                    IncompleteCause::Cancelled { source },
                ));
            }
        }
        if retained.len() >= limits.limits.max_states {
            return Ok(stopped(
                ModelCheckLimit::MaxStates,
                u64::try_from(limits.limits.max_states)
                    .map_err(|_| ProductClosureRefusal::Input("max-states"))?,
            ));
        }
        if matches!(
            state.property,
            Property::Undefined { .. } | Property::MatchUndefined { .. }
        ) {
            undefined.get_or_insert_with(|| key.clone());
        }
        if matches!(state.property, Property::Rejected) {
            violating.get_or_insert_with(|| key.clone());
        }
        let expansion = match product.successors(&state) {
            Ok(expansion) => expansion,
            Err(_) => return Err(product_stop(&product, request, limits)?),
        };
        if expansion
            .findings
            .iter()
            .any(|finding| !matches!(finding, HyperFinding::Enabled { .. }))
        {
            undefined.get_or_insert_with(|| key.clone());
        }
        let source = *seen
            .get(&key)
            .ok_or(ProductClosureRefusal::Recompute("source-key"))?;
        for (transition, next) in expansion.successors {
            if edges.len() >= limits.limits.max_transitions {
                return Ok(stopped(
                    ModelCheckLimit::MaxTransitions,
                    u64::try_from(limits.limits.max_transitions)
                        .map_err(|_| ProductClosureRefusal::Input("max-transitions"))?,
                ));
            }
            let next = canonical(&product, next, group.as_ref())?;
            let target_key = product_state_key(&product.key(&next))?;
            if !members.contains(&target_key) {
                return Ok(rejected(CertificateRule::SuccessorMissing, key));
            }
            let target = if let Some(target) = seen.get(&target_key) {
                *target
            } else {
                let target = seen.len();
                seen.insert(target_key.clone(), target);
                pending.push_back((target_key, next));
                target
            };
            edges.push((source, target, transition));
        }
        retained.push((key, state, expansion.findings));
    }
    // FR-163 checks the complete closure before letters, then the component
    // partition and witnesses before the form-specific violation rule.
    if let Some(key) = undefined {
        return Ok(rejected(CertificateRule::UndefinedMember, key));
    }
    let refs = retained
        .iter()
        .map(|(key, _, _)| key.clone())
        .collect::<Vec<_>>();
    let (graph, constraints) = fairness_graph(
        request.clause,
        &variable_systems,
        &retained,
        &edges,
        &product.automaton(),
    )?;
    let mut checked = ProductGraph::new();
    for (key, vertex) in refs.iter().zip(graph.vertices()) {
        let enabled = graph
            .enabled(vertex)
            .ok_or(ProductClosureRefusal::Recompute("checker-enabled"))?;
        checked
            .add_state(key.clone(), enabled.iter().copied())
            .map_err(|_| ProductClosureRefusal::Recompute("checker-vertex"))?;
    }
    for source in graph.vertices() {
        for edge in graph.edges_from(source) {
            checked
                .add_edge(source, edge.target, edge.label)
                .map_err(|_| ProductClosureRefusal::Recompute("checker-edge"))?;
        }
    }
    for accepting in graph.acceptance_sets() {
        checked
            .add_acceptance_set(accepting.iter().copied())
            .map_err(|_| ProductClosureRefusal::Recompute("checker-acceptance"))?;
    }
    if let Some(components) = &certificate.components {
        let result = check_partition(&checked, components, &constraints, cancel)
            .map_err(|_| ProductClosureRefusal::Recompute("empty-product"))?;
        if result != TemporalCertificateCheck::Accepted {
            return Ok(result);
        }
    }
    if let Some(key) = violating {
        return Ok(rejected(CertificateRule::ViolatingMember, key));
    }
    Ok(TemporalCertificateCheck::Accepted)
}

fn stopped(limit: ModelCheckLimit, value: u64) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Stopped(IncompleteCause::LimitReached { limit, value })
}

fn canonical(
    product: &Product<'_, '_>,
    state: HyperState,
    group: Option<&SymmetryGroup>,
) -> Result<HyperState, ProductClosureRefusal> {
    match group.and_then(|group| product.canonical(&state, group)) {
        Some(Ok((state, _))) => Ok(state),
        Some(Err(_)) => Err(ProductClosureRefusal::Recompute("canonicalization")),
        None => Ok(state),
    }
}

enum Product<'r, 's> {
    Universal(HyperProduct<'r, 's>),
    Witness(WitnessProduct<'r, 's>),
    Projected(ProjectedProduct<'r, 's>),
}

struct ProductBuild<'r, 's> {
    clause: &'r CheckedHyperClause,
    instance: &'r [ParameterBinding],
    systems: Vec<&'r ModelSystem<'s>>,
    letters: &'r CheckedHyperLetters<'r, 's>,
    matcher: &'r CheckedStepMatch<'r>,
    copy_swap: Option<CopySwapInputs<'r>>,
    limits: ModelCheckLimits,
    meter: ScalarLimits,
    cancel: &'r Cancel,
}

impl<'r, 's> Product<'r, 's> {
    fn build(inputs: ProductBuild<'r, 's>) -> Result<Self, ProductClosureRefusal> {
        let ProductBuild {
            clause,
            instance,
            systems,
            letters,
            matcher,
            copy_swap,
            limits,
            meter,
            cancel,
        } = inputs;
        let result = match clause.form() {
            Some(HyperForm::Universal | HyperForm::SingleExistential) => {
                HyperProduct::new(HyperInputs {
                    systems,
                    instance,
                    body: clause.body(),
                    profile: clause.profile(),
                    sense: if matches!(clause.form(), Some(HyperForm::SingleExistential)) {
                        BodySense::Itself
                    } else {
                        BodySense::Negated
                    },
                    letters,
                    matcher: (!clause.mu_universal().is_empty()).then_some(matcher),
                    max_automaton_states: limits.max_automaton_states,
                    meter,
                    cancel,
                    copy_swap,
                })
                .map(Self::Universal)
            }
            Some(HyperForm::ForallExistsSafety) => WitnessProduct::new(WitnessInputs {
                systems,
                instance,
                body: clause.body(),
                profile: clause.profile(),
                universal: clause.universals(),
                letters,
                universal_match: (!clause.mu_universal().is_empty()).then_some(matcher),
                existential_match: (!clause.mu_existential().is_empty()).then_some(matcher),
                max_automaton_states: limits.max_automaton_states,
                max_witness_set: limits.max_witness_set,
                meter,
                cancel,
            })
            .map(Self::Witness),
            Some(HyperForm::ProjectionAligned) => ProjectedProduct::new(ProjectedInputs {
                systems,
                instance,
                body: clause.body(),
                profile: clause.profile(),
                align_skip: clause.align_skip(),
                letters,
                matcher: (!clause.mu_universal().is_empty()).then_some(matcher),
                max_automaton_states: limits.max_automaton_states,
                meter,
                cancel,
            })
            .map(Self::Projected),
            _ => return Err(ProductClosureRefusal::Input("form")),
        };
        result.map_err(|error| match error {
            fsl_eval::automaton::AutomatonError::StateLimit { limit } => {
                ProductClosureRefusal::Stopped(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::MaxAutomatonStates,
                    value: limit,
                })
            }
            _ => ProductClosureRefusal::Recompute("automaton"),
        })
    }

    fn start(&mut self) -> Result<(), ProductClosureRefusal> {
        let result = match self {
            Self::Universal(product) => product.start(),
            Self::Witness(product) => product.start(),
            Self::Projected(product) => product.start(),
        };
        result.map_err(|_| ProductClosureRefusal::Recompute("initial-product"))
    }

    fn automaton(&self) -> std::cell::Ref<'_, PropertyAutomaton> {
        match self {
            Self::Universal(product) => product.automaton(),
            Self::Witness(product) => product.automaton(),
            Self::Projected(product) => product.automaton(),
        }
    }

    fn take_halt(&self) -> Option<fsl_eval::hyper::Halt> {
        match self {
            Self::Universal(product) => product.take_halt(),
            Self::Witness(product) => product.take_halt(),
            Self::Projected(product) => product.take_halt(),
        }
    }
}

fn product_stop(
    product: &Product<'_, '_>,
    request: &ProductClosureRequest<'_>,
    limits: &ModelCheckLimits,
) -> Result<ProductClosureRefusal, ProductClosureRefusal> {
    use fsl_eval::hyper::Halt;
    let stopped = |limit, value| {
        ProductClosureRefusal::Stopped(IncompleteCause::LimitReached { limit, value })
    };
    Ok(
        match product
            .take_halt()
            .ok_or(ProductClosureRefusal::Recompute("product-halt"))?
        {
            Halt::Automaton(value) => stopped(ModelCheckLimit::MaxAutomatonStates, value),
            Halt::WitnessSet(_) => stopped(ModelCheckLimit::MaxWitnessSet, limits.max_witness_set),
            Halt::Meter(counter) => stopped(
                ModelCheckLimit::EvaluationMeter { counter },
                crate::meter_bound(&request.meter, counter),
            ),
            Halt::Cancelled(source) => {
                ProductClosureRefusal::Stopped(IncompleteCause::Cancelled { source })
            }
            Halt::Expansion { component, stop } => match stop.limit {
                Some(fsl_eval::simulation::ExpansionLimit::EvaluationMeter(counter)) => stopped(
                    ModelCheckLimit::EvaluationMeter { counter },
                    crate::meter_bound(&request.meter, counter),
                ),
                Some(fsl_eval::simulation::ExpansionLimit::MaxCandidates) => {
                    let quantifier = request
                        .clause
                        .quantifiers()
                        .get(component)
                        .ok_or(ProductClosureRefusal::Recompute("halt-component"))?;
                    let slot = request
                        .clause
                        .aliases()
                        .iter()
                        .position(|alias| alias == &quantifier.alias)
                        .ok_or(ProductClosureRefusal::Recompute("halt-alias"))?;
                    let subject = request
                        .subjects
                        .get(slot)
                        .ok_or(ProductClosureRefusal::Recompute("halt-subject"))?;
                    stopped(
                        ModelCheckLimit::MaxCandidates,
                        subject.simulation.expansion.max_candidates,
                    )
                }
                None => ProductClosureRefusal::Recompute("expansion"),
            },
            Halt::AutomatonKey(_) | Halt::Atom(_) | Halt::Encoding(_) | Halt::Fault(_) => {
                ProductClosureRefusal::Recompute("product-halt")
            }
        },
    )
}

impl TransitionSystem for Product<'_, '_> {
    type State = HyperState;
    type TransitionId = HyperTransition;
    type Key = HyperKey;
    type Finding = HyperFinding;

    fn initial(&self) -> Vec<HyperState> {
        match self {
            Self::Universal(product) => product.initial(),
            Self::Witness(product) => product.initial(),
            Self::Projected(product) => product.initial(),
        }
    }

    fn key(&self, state: &HyperState) -> HyperKey {
        match self {
            Self::Universal(product) => product.key(state),
            Self::Witness(product) => product.key(state),
            Self::Projected(product) => product.key(state),
        }
    }

    fn canonical(
        &self,
        state: &HyperState,
        group: &SymmetryGroup,
    ) -> Option<
        Result<
            (HyperState, fsl_eval::simulation::Permutation),
            fsl_eval::simulation::ExpansionStop,
        >,
    > {
        match self {
            Self::Universal(product) => product.canonical(state, group),
            Self::Witness(_) | Self::Projected(_) => None,
        }
    }

    fn successors(
        &self,
        state: &HyperState,
    ) -> Result<
        fsl_eval::simulation::Expansion<HyperTransition, HyperState, HyperFinding>,
        fsl_eval::simulation::ExpansionStop,
    > {
        match self {
            Self::Universal(product) => product.successors(state),
            Self::Witness(product) => product.successors(state),
            Self::Projected(product) => product.successors(state),
        }
    }
}

/// One retained product member, its canonical key and recorded findings.
pub type HyperProductMember = (ProductStateKey, HyperState, Vec<HyperFinding>);

/// The retained graph and checked inputs from which the engine writes evidence.
pub struct ProductEvidence<'a, 's> {
    /// The hyper clause whose product was explored.
    pub clause: &'a CheckedHyperClause,
    /// One admitted model system for each trace variable.
    pub systems: &'a [&'a ModelSystem<'s>],
    /// The object parameter binding of this product.
    pub instance: &'a [ParameterBinding],
    /// Product states and findings, in canonical discovery order.
    pub members: &'a [HyperProductMember],
    /// Retained edges by source and target discovery index.
    pub edges: &'a [(usize, usize, HyperTransition)],
    /// The product's already translated property automaton.
    pub automaton: &'a PropertyAutomaton,
    /// The applied copy group, if any.
    pub reduction: Option<CopyGroup>,
    /// Cancellation of certificate production.
    pub cancel: &'a Cancel,
}

/// Produce a certificate from the engine's retained graph, without searching.
/// The result is only evidence to submit to `check_product_closure`.
///
/// # Errors
///
/// Refuses graphs that contain a fair accepting component or invalid edges.
pub fn product_certificate(
    evidence: ProductEvidence<'_, '_>,
) -> Result<ProductClosureCertificate, ProductClosureRefusal> {
    let ProductEvidence {
        clause,
        systems,
        instance,
        members,
        edges,
        automaton,
        reduction,
        cancel,
    } = evidence;
    let mut closure: Vec<_> = members.iter().map(|(key, _, _)| key.clone()).collect();
    closure.sort();
    if closure.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ProductClosureRefusal::Recompute("duplicate-member"));
    }
    let components = if matches!(
        clause.form(),
        Some(HyperForm::Universal | HyperForm::ForallExistsSafety)
    ) {
        let (graph, constraints) = fairness_graph(clause, systems, members, edges, automaton)?;
        let all = graph.vertices().collect();
        let analyses = fsl_eval::fairness::analyze(&graph, &all, &constraints, cancel)
            .map_err(|_| ProductClosureRefusal::Recompute("fairness"))?;
        let keys = members
            .iter()
            .map(|(key, _, _)| key.clone())
            .collect::<Vec<_>>();
        Some(components_of(&graph, &analyses, &keys)?)
    } else {
        None
    };
    Ok(ProductClosureCertificate {
        form: clause
            .form()
            .cloned()
            .ok_or(ProductClosureRefusal::Input("form"))?,
        instance: instance
            .iter()
            .map(|binding| (binding.parameter.clone(), binding.instance.key.clone()))
            .collect(),
        reduction,
        closure,
        components,
    })
}

fn components_of(
    graph: &Graph,
    analyses: &[Analysis],
    keys: &[ProductStateKey],
) -> Result<Vec<Component<ProductStateKey>>, ProductClosureRefusal> {
    let mut owner = BTreeMap::new();
    for (index, analysis) in analyses.iter().enumerate() {
        for vertex in analysis.vertices() {
            owner.insert(*vertex, index);
        }
    }
    let mut listed = BTreeSet::new();
    let mut components = Vec::new();
    while listed.len() < analyses.len() {
        let ready = analyses
            .iter()
            .enumerate()
            .filter(|(index, _)| !listed.contains(index))
            .filter(|(index, _)| {
                owner.iter().all(|(source, source_owner)| {
                    source_owner == index
                        || listed.contains(source_owner)
                        || !graph.edges_from(*source).iter().any(|edge| {
                            fsl_eval::fairness::is_live(edge, analyses[*source_owner].removed())
                                && owner.get(&edge.target) == Some(index)
                        })
                })
            })
            .min_by_key(|(_, analysis)| analysis.vertices().first().map(|vertex| vertex.index()))
            .map(|(index, _)| index)
            .ok_or(ProductClosureRefusal::Recompute("component-order"))?;
        let analysis = &analyses[ready];
        let index = |value| {
            u32::try_from(value).map_err(|_| ProductClosureRefusal::Recompute("component-index"))
        };
        let witness = match analysis.outcome() {
            Outcome::Trivial => ComponentWitness::Trivial,
            Outcome::MissingAcceptance { set } => {
                ComponentWitness::MissingAcceptance { set: index(*set)? }
            }
            Outcome::UnfairWeak { constraint } => ComponentWitness::UnfairWeak {
                constraint: index(*constraint)?,
            },
            Outcome::UnfairStrong { constraints, sub } => ComponentWitness::UnfairStrong {
                constraints: constraints
                    .iter()
                    .map(|value| index(*value))
                    .collect::<Result<_, _>>()?,
                sub: components_of(graph, sub, keys)?,
            },
            Outcome::UnfairVisibility { constraints, sub } => ComponentWitness::UnfairVisibility {
                constraints: constraints
                    .iter()
                    .map(|value| index(*value))
                    .collect::<Result<_, _>>()?,
                sub: components_of(graph, sub, keys)?,
            },
            Outcome::Fair => {
                return Err(ProductClosureRefusal::Recompute("fair-accepting-component"));
            }
        };
        let mut states = analysis
            .vertices()
            .iter()
            .map(|vertex| {
                keys.get(vertex.index())
                    .cloned()
                    .ok_or(ProductClosureRefusal::Recompute("component-member"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        states.sort();
        components.push(Component { states, witness });
        listed.insert(ready);
    }
    Ok(components)
}

fn fairness_graph(
    clause: &CheckedHyperClause,
    systems: &[&ModelSystem<'_>],
    members: &[HyperProductMember],
    edges: &[(usize, usize, HyperTransition)],
    automaton: &PropertyAutomaton,
) -> Result<(Graph, Vec<Constraint>), ProductClosureRefusal> {
    use filament_semantic_value::fairness::FairnessGranularity;
    use filament_semantic_value::model_transition::ModelTransition;
    let joints: BTreeSet<&HyperTransition> = edges.iter().map(|(_, _, edge)| edge).collect();
    let joint_ids: BTreeMap<&HyperTransition, Identity> = joints
        .iter()
        .enumerate()
        .map(|(index, joint)| {
            Ok((
                *joint,
                Identity::new(
                    u64::try_from(index)
                        .map_err(|_| ProductClosureRefusal::Recompute("joint-count"))?,
                ),
            ))
        })
        .collect::<Result<_, ProductClosureRefusal>>()?;
    let mut enabled = BTreeSet::<(usize, &ModelTransition)>::new();
    for (_, _, findings) in members {
        for finding in findings {
            if let HyperFinding::Enabled {
                component,
                transitions,
            } = finding
            {
                enabled.extend(
                    transitions
                        .iter()
                        .map(|transition| (*component, transition)),
                );
            }
        }
    }
    let enabled_ids: BTreeMap<(usize, &ModelTransition), Identity> = enabled
        .iter()
        .enumerate()
        .map(|(index, key)| {
            Ok((
                *key,
                Identity::new(
                    u64::try_from(
                        index
                            .checked_add(joint_ids.len())
                            .ok_or(ProductClosureRefusal::Recompute("identity-count"))?,
                    )
                    .map_err(|_| ProductClosureRefusal::Recompute("identity-count"))?,
                ),
            ))
        })
        .collect::<Result<_, ProductClosureRefusal>>()?;
    let mut graph = Graph::new();
    let mut vertices = Vec::new();
    for (_, _, findings) in members {
        let mut enabled = Vec::new();
        for finding in findings {
            if let HyperFinding::Enabled {
                component,
                transitions,
            } = finding
            {
                for transition in transitions {
                    enabled.push(
                        *enabled_ids
                            .get(&(*component, transition))
                            .ok_or(ProductClosureRefusal::Recompute("enabled-identity"))?,
                    );
                }
            }
        }
        vertices.push(graph.add_vertex(enabled));
    }
    for (source, target, transition) in edges {
        let label = if !transition.0.is_empty()
            && transition
                .0
                .iter()
                .all(|step| matches!(step, ComponentStep::Stutter))
        {
            EdgeLabel::Stutter
        } else {
            EdgeLabel::Transition(
                *joint_ids
                    .get(transition)
                    .ok_or(ProductClosureRefusal::Recompute("joint-identity"))?,
            )
        };
        graph
            .add_edge(
                *vertices
                    .get(*source)
                    .ok_or(ProductClosureRefusal::Recompute("edge-source"))?,
                *vertices
                    .get(*target)
                    .ok_or(ProductClosureRefusal::Recompute("edge-target"))?,
                label,
            )
            .map_err(|_| ProductClosureRefusal::Recompute("edge"))?;
    }
    let mut constraints = Vec::new();
    for (component, quantifier) in clause.quantifiers().iter().enumerate() {
        let system = systems
            .get(component)
            .ok_or(ProductClosureRefusal::Input("component-system"))?;
        for constraint in &quantifier.fairness {
            let operation = constraint.operation();
            let text = system
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or(ProductClosureRefusal::Input("fairness-operation"))?;
            let strength = Strength::from(constraint.kind());
            let mut covered = Vec::new();
            for ((owner, transition), identity) in &enabled_ids {
                if *owner != component || transition.operation() != text {
                    continue;
                }
                let mut identities = vec![*identity];
                identities.extend(joint_ids.iter().filter_map(|(joint, id)|
                    matches!(joint.0.get(component), Some(ComponentStep::Model(taken)) if taken == *transition)
                        .then_some(*id)));
                covered.push(identities);
            }
            match constraint.granularity() {
                FairnessGranularity::Whole => {
                    constraints.push(Constraint::new(strength, covered.into_iter().flatten()))
                }
                FairnessGranularity::Each => constraints.extend(
                    covered
                        .into_iter()
                        .map(|ids| Constraint::new(strength, ids)),
                ),
            }
        }
    }
    if matches!(clause.form(), Some(HyperForm::ForallExistsSafety)) {
        graph.add_acceptance_set(members.iter().enumerate().filter_map(|(index, (_, state, _))|
            matches!(&state.property, Property::Witnesses(witnesses) if witnesses.members().is_empty())
                .then(|| vertices[index])))
            .map_err(|_| ProductClosureRefusal::Recompute("witness-acceptance"))?;
    } else if matches!(clause.form(), Some(HyperForm::Universal)) {
        if automaton.acceptance_set_count() == 0 {
            graph
                .add_acceptance_set(members.iter().enumerate().filter_map(
                    |(index, (_, state, _))| {
                        matches!(state.property, Property::After(_)).then(|| vertices[index])
                    },
                ))
                .map_err(|_| ProductClosureRefusal::Recompute("live-acceptance"))?;
        }
        for set in 0..automaton.acceptance_set_count() {
            graph.add_acceptance_set(members.iter().enumerate().filter_map(|(index, (_, state, _))|
                matches!(&state.property, Property::After(at) if automaton.is_accepting_in(set, *at))
                    .then(|| vertices[index])))
                .map_err(|_| ProductClosureRefusal::Recompute("automaton-acceptance"))?;
        }
    }
    Ok((graph, constraints))
}

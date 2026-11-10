// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent gain–bias checking of the closed unweighted DTMC profile.

use super::*;
use quire_canonical::{Encode, Error, Sink, Writer};
use std::cell::RefCell;

/// Actual long-run checking phase entered under the caller's control.
#[cfg(feature = "test-support")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LongRunProbabilityPhase {
    /// Canonical document admission.
    Admission,
    /// Complete model/product key encoding.
    Key,
    /// Checked state predicate evaluation.
    Predicate,
    /// Qualified workload expansion.
    Workload,
    /// Positive-support component traversal.
    Components,
    /// Supplied versus independent membership comparison.
    Membership,
    /// Exact rational sums and equations.
    Arithmetic,
}

const PROFILE: &str = "quire.probability-objective/dtmc-long-run/v1";

/// Candidate evidence for one complete recurrent component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LongRunProbabilityComponent {
    /// Long-run fraction of the checked indicator in this component.
    pub value: Rational,
    /// Average original indicator reward, equal to `value`.
    pub gain: Rational,
    /// Signed biases in strictly ascending complete-key order, first bias zero.
    pub bias: Vec<ExactProbabilityValue>,
}

/// Canonical long-run document; construction alone grants no acceptance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LongRunProbabilityDocument {
    /// Structural checked claim identity.
    pub identity: ObligationIdentity,
    /// Exact reachable keys where the independently checked predicate holds.
    pub predicate_states: Vec<ProbabilityProductKey>,
    /// Exact long-run values on the whole original reachable graph.
    pub values: Vec<ExactProbabilityValue>,
    /// Every bottom component, ordered by its first complete bias key.
    pub components: Vec<LongRunProbabilityComponent>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Objective {
    profile: String,
    predicate_states: Vec<String>,
    optimization: String,
    fairness: Vec<Value>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Component {
    value: WireRational,
    gain: WireRational,
    bias: Vec<WireValue>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    identity: WireIdentity,
    objective: Objective,
    kind: String,
    values: Vec<WireValue>,
    components: Vec<Component>,
}

pub(super) fn has_profile(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    poll(cancel)?;
    if bytes.len() as u64 > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            bytes.len() as u64,
        ));
    }
    let parsed = quire_canonical::read_bounded(bytes, limits.bytes, limits.storage_bytes).map_err(
        |error| match error {
            quire_canonical::ReadError::Limit(limit) => exceeded(
                ProbabilityDocumentLimit::StorageBytes,
                limit.bound,
                limit.required,
            ),
            _ => ProbabilityDocumentError::Malformed,
        },
    )?;
    Ok(matches!(
        parsed
            .root()
            .get("objective")
            .and_then(|v| v.get("profile"))
            .map(|v| v.node()),
        Some(quire_canonical::Node::String(PROFILE))
    ))
}

/// Read canonical long-run bytes without trusting their component partition.
pub fn read_long_run_probability_certificate(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<LongRunProbabilityDocument, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    read(bytes, &mut cx)
}

fn read(
    bytes: &[u8],
    cx: &mut Context<'_>,
) -> Result<LongRunProbabilityDocument, ProbabilityDocumentError> {
    #[cfg(feature = "test-support")]
    cx.phase(LongRunProbabilityPhase::Admission)?;
    if !has_profile(bytes, cx.limits, cx.cancel)? {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    // Account wire strings, parsed rows and decoded key storage before serde growth.
    cx.reserve(bytes.len().saturating_mul(16))?;
    cx.work(bytes.len() as u64)?;
    let parsed = quire_canonical::read_bounded(bytes, cx.limits.bytes, cx.limits.storage_bytes)
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    if parsed.root().get("components").is_none() {
        return Err(ProbabilityDocumentError::MissingMember("components"));
    }
    drop(parsed);
    let wire: Document =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if canonical(&wire, cx.limits.bytes)? != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    if wire.kind != "long-run"
        || wire.objective.profile != PROFILE
        || wire.objective.optimization != "workload"
        || !wire.objective.fairness.is_empty()
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    cx.rows(wire.values.len())?;
    cx.rows(wire.objective.predicate_states.len())?;
    cx.rows(wire.components.len())?;
    let mut total = 0usize;
    for component in &wire.components {
        total = total.saturating_add(component.bias.len());
    }
    cx.rows(total)?;
    let node = qsl_foundation::node_wire::node_key_from_text(wire.identity.node_id.get())
        .map_err(|_| ProbabilityDocumentError::Malformed)?;
    let identity = ObligationIdentity::new(
        node,
        wire.identity.model.map(crate::ModelIdentity::new),
        wire.identity.ordinal,
    );
    let predicate_states = ordered(
        wire.objective
            .predicate_states
            .iter()
            .map(|key| decode_key(key, cx.limits))
            .collect::<Result<Vec<_>, _>>()?,
    )?;
    let values = read_rows(wire.values, true, cx)?;
    let mut components = Vec::new();
    for component in wire.components {
        cx.work(1)?;
        let value = ratio(
            component.value,
            cx.limits.scalar_bits,
            &mut cx.meter,
            cx.cancel,
        )?;
        let gain = ratio_checked(
            component.gain,
            cx.limits.scalar_bits,
            &mut cx.meter,
            cx.cancel,
            false,
        )?;
        let bias = read_rows(component.bias, false, cx)?;
        if bias.is_empty()
            || components
                .last()
                .is_some_and(|previous: &LongRunProbabilityComponent| {
                    previous.bias[0].state >= bias[0].state
                })
        {
            return Err(ProbabilityDocumentError::MalformedMember("components"));
        }
        components.push(LongRunProbabilityComponent { value, gain, bias });
    }
    Ok(LongRunProbabilityDocument {
        identity,
        predicate_states,
        values,
        components,
    })
}

fn read_rows(
    rows: Vec<WireValue>,
    probability: bool,
    cx: &mut Context<'_>,
) -> Result<Vec<ExactProbabilityValue>, ProbabilityDocumentError> {
    let mut values = Vec::new();
    for row in rows {
        cx.work(1)?;
        let state = decode_key(&row.state, cx.limits)?;
        if values
            .last()
            .is_some_and(|previous: &ExactProbabilityValue| previous.state >= state)
        {
            return Err(ProbabilityDocumentError::MalformedMember(if probability {
                "values"
            } else {
                "components"
            }));
        }
        let value = ratio_checked(
            row.value,
            cx.limits.scalar_bits,
            &mut cx.meter,
            cx.cancel,
            probability,
        )?;
        values.push(ExactProbabilityValue { state, value });
    }
    Ok(values)
}

struct Events<'a> {
    document: &'a LongRunProbabilityDocument,
    text: writer::Text<'a>,
}
impl Events<'_> {
    fn ratio<S: Sink + ?Sized>(
        &self,
        value: &Rational,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("numerator")?;
        self.text.integer(value.numerator(), writer)?;
        writer.name("denominator")?;
        self.text.integer(value.denominator(), writer)?;
        writer.end_object()
    }
    fn rows<S: Sink + ?Sized>(
        &self,
        rows: &[ExactProbabilityValue],
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        writer.begin_array()?;
        for row in rows {
            self.text.poll()?;
            writer.begin_object()?;
            writer.name("state")?;
            self.text.key(&row.state, writer, false)?;
            writer.name("value")?;
            self.ratio(&row.value, writer)?;
            writer.end_object()?;
        }
        writer.end_array()
    }
}
impl Encode for Events<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.text.poll()?;
        writer.begin_object()?;
        writer.name("identity")?;
        writer.begin_object()?;
        writer.name("node_id")?;
        qsl_foundation::node_wire::encode_node_key(self.document.identity.node(), writer)?;
        writer.name("model")?;
        match self.document.identity.model() {
            Some(model) => writer.string(model.as_str())?,
            None => writer.null()?,
        }
        writer.name("ordinal")?;
        writer.integer(i128::from(self.document.identity.ordinal()))?;
        writer.end_object()?;
        writer.name("objective")?;
        writer.begin_object()?;
        writer.name("profile")?;
        writer.string(PROFILE)?;
        writer.name("predicate_states")?;
        writer.begin_array()?;
        for (index, key) in self.document.predicate_states.iter().enumerate() {
            self.text.key(key, writer, index != 0)?;
        }
        writer.end_array()?;
        writer.name("optimization")?;
        writer.string("workload")?;
        writer.name("fairness")?;
        writer.begin_array()?;
        writer.end_array()?;
        writer.end_object()?;
        writer.name("kind")?;
        writer.string("long-run")?;
        writer.name("values")?;
        self.rows(&self.document.values, writer)?;
        writer.name("components")?;
        writer.begin_array()?;
        for component in &self.document.components {
            writer.begin_object()?;
            writer.name("value")?;
            self.ratio(&component.value, writer)?;
            writer.name("gain")?;
            self.ratio(&component.gain, writer)?;
            writer.name("bias")?;
            self.rows(&component.bias, writer)?;
            writer.end_object()?;
        }
        writer.end_array()?;
        writer.end_object()
    }
}

/// Write only the canonical, structurally admitted long-run representation.
pub fn write_long_run_probability_certificate(
    document: &LongRunProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    poll(cancel)?;
    let events = Events {
        document,
        text: writer::Text::new(limits, cancel),
    };
    let admission = RefCell::new(writer::Admission::new(limits.storage_bytes, cancel));
    let result = writer::encode_reserved(&events, limits.bytes, &admission);
    if let Some(error) = events.text.take_stop() {
        return Err(error);
    }
    let bytes = result?;
    read_long_run_probability_certificate(&bytes, limits, cancel)?;
    Ok(bytes)
}

struct Context<'a> {
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    meter: quire_exact::Meter,
    storage: u64,
    #[cfg(feature = "test-support")]
    observe: Option<&'a mut dyn FnMut(LongRunProbabilityPhase)>,
}
impl<'a> Context<'a> {
    fn new(limits: ProbabilityDocumentLimits, cancel: &'a Cancel) -> Self {
        Self {
            limits,
            cancel,
            meter: quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone()),
            storage: 0,
            #[cfg(feature = "test-support")]
            observe: None,
        }
    }
    #[cfg(feature = "test-support")]
    fn phase(&mut self, phase: LongRunProbabilityPhase) -> Result<(), ProbabilityDocumentError> {
        if let Some(observe) = self.observe.as_mut() {
            observe(phase);
        }
        poll(self.cancel)
    }
    fn rows(&self, n: usize) -> Result<(), ProbabilityDocumentError> {
        if n > self.limits.states {
            return Err(exceeded(
                ProbabilityDocumentLimit::States,
                self.limits.states as u64,
                n as u64,
            ));
        }
        Ok(())
    }
    fn reserve(&mut self, bytes: usize) -> Result<(), ProbabilityDocumentError> {
        poll(self.cancel)?;
        let actual = self.storage.saturating_add(bytes as u64);
        if actual > self.limits.storage_bytes {
            return Err(exceeded(
                ProbabilityDocumentLimit::StorageBytes,
                self.limits.storage_bytes,
                actual,
            ));
        }
        self.storage = actual;
        Ok(())
    }
    fn work(&mut self, n: u64) -> Result<(), ProbabilityDocumentError> {
        poll(self.cancel)?;
        self.meter
            .charge(
                quire_exact::Charge::new(quire_exact::ChargePoint::GraphEdge)
                    .work(Integer::from(n)),
            )
            .map_err(|error| match self.cancel.tripped() {
                Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                None => ProbabilityDocumentError::Accounting(Box::new(error)),
            })
    }
    fn arithmetic(
        &mut self,
        operation: quire_exact::RationalArithmetic<'_>,
    ) -> Result<Rational, ProbabilityDocumentError> {
        #[cfg(feature = "test-support")]
        self.phase(LongRunProbabilityPhase::Arithmetic)?;
        let value = arithmetic(operation, &mut self.meter, self.cancel)?;
        check_scalar(&value, self.limits)?;
        Ok(value)
    }
}

fn subject(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    cx: &mut Context<'_>,
) -> Result<(CheckerGraph, Bound), ProbabilityDocumentError> {
    use qsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
    if !std::ptr::eq(
        system.package().graph().probabilistic_model().claims(),
        claims,
    ) {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cx.cancel)?;
    let ProbForm::LongRunFraction {
        bound,
        predicate,
        weight: None,
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    if !matches!(claim.scheduler, ProbScheduler::Workload(_))
        || !claim.fairness.is_empty()
        || qsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(qsl_semantics::check::TemporalProfile::InfiniteTrace)
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|b| &b.reference);
    let workload = claim_workload(system, claim)?.ok_or(ProbabilityDocumentError::Unsupported)?;
    let limits = cx.limits;
    let cancel = cx.cancel;
    let cx = RefCell::new(cx);
    let stopped = RefCell::new(None);
    let key_of = |state: &ModelSimState| {
        let encoded = (|| {
            #[cfg(feature = "test-support")]
            cx.borrow_mut().phase(LongRunProbabilityPhase::Key)?;
            cx.borrow_mut().work(1)?;
            // Raw ModelSystem state-key construction owns its scratch allocation.
            let (bytes, _) = system.state_key(state).map_err(|_| {
                ProbabilityDocumentError::Graph(ExactProbabilityGraphError::ExpansionRefused)
            })?;
            let mut context = cx.borrow_mut();
            context.reserve(bytes.len())?;
            let remaining = limits.storage_bytes.saturating_sub(context.storage);
            let key = model_product_key(
                reference,
                &bytes,
                None,
                ProbabilityDocumentIoLimits {
                    bytes: limits.bytes,
                    storage_bytes: remaining,
                },
                cancel,
            )?;
            context.reserve(key.as_bytes().len())?;
            Ok(key)
        })();
        encoded.map_err(|error| {
            *stopped.borrow_mut() = Some(error);
            ExactProbabilityGraphError::ExpansionRefused
        })
    };
    let mut predicate_keys = BTreeSet::new();
    let mut issue = None;
    let graph = enumerate_model_graph_with_keys_and_reservation(
        system,
        None::<&qsl_semantics::check::CheckedPredicate>,
        reference,
        Some(workload),
        limits.states,
        limits.edges,
        cancel,
        false,
        &key_of,
        &mut |bytes| {
            let result = {
                let mut context = cx.borrow_mut();
                context.reserve(bytes).and_then(|()| context.work(1))
            };
            result.map_err(|error| {
                *stopped.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        #[cfg(feature = "test-support")]
        &mut |phase| {
            if phase == ProbabilityWitnessSemanticPhase::ExpansionEntered {
                if let Err(error) = cx.borrow_mut().phase(LongRunProbabilityPhase::Workload) {
                    *stopped.borrow_mut() = Some(error);
                }
            }
        },
        &mut |key, state| {
            let evaluated = (|| {
                #[cfg(feature = "test-support")]
                cx.borrow_mut().phase(LongRunProbabilityPhase::Predicate)?;
                cx.borrow_mut().work(1)?;
                let readings = system
                    .state_predicates_with_cancel(state, &[(predicate, reference)], cancel)
                    .map_err(|stop| {
                        ProbabilityDocumentError::Build(Box::new(
                            crate::ExactProbBuildIssue::Expansion(stop),
                        ))
                    })?;
                poll(cancel)?;
                match readings
                    .into_iter()
                    .next()
                    .ok_or(ProbabilityDocumentError::Malformed)?
                {
                    qsl_eval::simulation::model::PredicateReading::True => {
                        cx.borrow_mut().reserve(
                            key.as_bytes()
                                .len()
                                .saturating_add(std::mem::size_of::<ProbabilityProductKey>()),
                        )?;
                        predicate_keys.insert(key.clone());
                    }
                    qsl_eval::simulation::model::PredicateReading::False => {}
                    qsl_eval::simulation::model::PredicateReading::Undefined { cause, .. } => {
                        return Err(ProbabilityDocumentError::UndefinedPredicate(cause))
                    }
                    qsl_eval::simulation::model::PredicateReading::Refused(outcome) => {
                        return Err(ProbabilityDocumentError::Build(Box::new(
                            crate::ExactProbBuildIssue::Predicate(outcome),
                        )))
                    }
                }
                Ok(())
            })();
            evaluated.map_err(|error| {
                *stopped.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        &mut |error| issue = Some(error),
    );
    if let Some(error) = stopped.borrow_mut().take() {
        return Err(error);
    }
    if let Some(issue) = issue {
        return Err(ProbabilityDocumentError::Build(Box::new(issue)));
    }
    let mut graph = graph.map_err(|error| match error {
        ExactProbabilityGraphError::LimitReached => exceeded(
            ProbabilityDocumentLimit::States,
            limits.states as u64,
            (limits.states as u64).saturating_add(1),
        ),
        ExactProbabilityGraphError::EdgeLimitReached => exceeded(
            ProbabilityDocumentLimit::Edges,
            limits.edges as u64,
            (limits.edges as u64).saturating_add(1),
        ),
        other => other.into(),
    })?;
    for (key, state) in &mut graph.states {
        cx.borrow_mut().work(1)?;
        if predicate_keys.contains(key) {
            state.terminal = Some(true);
        }
    }
    Ok((graph, bound.clone()))
}

/// Build candidate transport with independently derived predicate keys.
/// Component membership and equations still require the independent checker.
pub fn long_run_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    values: Vec<ExactProbabilityValue>,
    components: Vec<LongRunProbabilityComponent>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<LongRunProbabilityDocument, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    let (graph, _) = subject(system, claims, item, &mut cx)?;
    Ok(LongRunProbabilityDocument {
        identity: item.clone(),
        predicate_states: target_keys(&graph),
        values,
        components,
    })
}

fn malformed(member: &'static str) -> ExactProbabilityCertificateCheck {
    ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
        rule: ProbabilityCertificateRule::Malformed,
        at: ProbabilityCertificateLocus::Member(member),
    })
}

pub(super) fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    check_with_context(system, claims, item, bytes, &mut cx)
}

/// Check with a phase observer while retaining the original caller controls.
#[cfg(feature = "test-support")]
pub fn check_long_run_probability_certificate_observing(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    observe: &mut dyn FnMut(LongRunProbabilityPhase),
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel);
    cx.observe = Some(observe);
    check_with_context(system, claims, item, bytes, &mut cx)
}

fn check_with_context(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    cx: &mut Context<'_>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let document = match read(bytes, cx) {
        Ok(document) => document,
        Err(ProbabilityDocumentError::MissingMember("components")) => {
            return Ok(malformed("components"))
        }
        Err(ProbabilityDocumentError::MalformedMember(member)) => return Ok(malformed(member)),
        Err(error) => return Err(error),
    };
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let (graph, bound) = subject(system, claims, item, cx)?;
    if document.predicate_states != target_keys(&graph) {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    check_graph(&graph, &document, &bound, cx).map(|mut check| {
        if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut check {
            accepted.document = Some(bytes.into());
        }
        check
    })
}

fn check_graph(
    graph: &CheckerGraph,
    document: &LongRunProbabilityDocument,
    bound: &Bound,
    cx: &mut Context<'_>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    cx.reserve(
        graph.states.len().saturating_mul(256).saturating_add(
            graph
                .states
                .values()
                .map(|s| s.choices.iter().map(Vec::len).sum::<usize>().max(1))
                .sum::<usize>()
                .saturating_mul(32),
        ),
    )?;
    let keys = graph.states.keys().collect::<Vec<_>>();
    let positions = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (*key, index))
        .collect::<BTreeMap<_, _>>();
    let mut edges = vec![Vec::new(); keys.len()];
    let mut reverse = vec![Vec::new(); keys.len()];
    for (i, key) in keys.iter().enumerate() {
        cx.work(1)?;
        let state = &graph.states[*key];
        if state.choices.is_empty() {
            edges[i].push(i);
            reverse[i].push(i);
            continue;
        }
        let [choice] = state.choices.as_slice() else {
            return Err(ProbabilityDocumentError::Unsupported);
        };
        let mut mass = rational(0);
        for edge in choice {
            cx.work(1)?;
            check_scalar(&edge.probability, cx.limits)?;
            if edge.probability < rational(0) {
                return Err(ProbabilityDocumentError::Malformed);
            }
            mass = cx.arithmetic(quire_exact::RationalArithmetic::Add(
                &mass,
                &edge.probability,
            ))?;
            if edge.probability > rational(0) {
                let next = *positions
                    .get(&edge.target)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                edges[i].push(next);
                reverse[next].push(i);
            }
        }
        if mass != rational(1) {
            return Err(ProbabilityDocumentError::Malformed);
        }
    }
    // Iterative Kosaraju: neither recursion depth nor producer SCC labels enter.
    let mut seen = vec![false; keys.len()];
    let mut order = Vec::new();
    for start in 0..keys.len() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![(start, 0)];
        while let Some((state, offset)) = stack.last_mut() {
            #[cfg(feature = "test-support")]
            cx.phase(LongRunProbabilityPhase::Components)?;
            cx.work(1)?;
            if let Some(&next) = edges[*state].get(*offset) {
                *offset += 1;
                if !seen[next] {
                    seen[next] = true;
                    stack.push((next, 0));
                }
            } else {
                let (state, _) = stack.pop().ok_or(ProbabilityDocumentError::Malformed)?;
                order.push(state);
            }
        }
    }
    let mut component_of = vec![usize::MAX; keys.len()];
    let mut components = Vec::new();
    for start in order.into_iter().rev() {
        if component_of[start] != usize::MAX {
            continue;
        }
        let index = components.len();
        let mut component = Vec::new();
        let mut stack = vec![start];
        component_of[start] = index;
        while let Some(state) = stack.pop() {
            cx.work(1)?;
            component.push(state);
            for &next in &reverse[state] {
                cx.work(1)?;
                if component_of[next] == usize::MAX {
                    component_of[next] = index;
                    stack.push(next);
                }
            }
        }
        components.push(component);
    }
    let mut bottom = vec![true; components.len()];
    for (state, successors) in edges.iter().enumerate() {
        for &next in successors {
            cx.work(1)?;
            if component_of[state] != component_of[next] {
                bottom[component_of[state]] = false;
            }
        }
    }
    let mut expected = vec![None; keys.len()];
    for (index, component) in components.iter().enumerate() {
        if bottom[index] {
            cx.work(component.len() as u64)?;
            let first = *component
                .iter()
                .min()
                .ok_or(ProbabilityDocumentError::Malformed)?;
            for &state in component {
                expected[state] = Some(first);
            }
        }
    }
    let mut offered = vec![None; keys.len()];
    let mut source = vec![None; keys.len()];
    for (index, component) in document.components.iter().enumerate() {
        let Some(first) = component.bias.first() else {
            return Ok(malformed("components"));
        };
        let Some(&first_index) = positions.get(&first.state) else {
            return Ok(malformed("components"));
        };
        for row in &component.bias {
            cx.work(1)?;
            let Some(&state) = positions.get(&row.state) else {
                return Ok(malformed("components"));
            };
            if offered[state].replace(first_index).is_some() {
                return Ok(malformed("components"));
            }
            source[state] = Some(index);
        }
    }
    for (state, key) in keys.iter().enumerate() {
        #[cfg(feature = "test-support")]
        cx.phase(LongRunProbabilityPhase::Membership)?;
        cx.work(1)?;
        if expected[state] != offered[state] {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                (*key).clone(),
            ));
        }
    }
    let mut values = BTreeMap::new();
    for row in &document.values {
        cx.work(1)?;
        if !graph.states.contains_key(&row.state) || values.insert(&row.state, &row.value).is_some()
        {
            return Ok(malformed("values"));
        }
    }
    if values.len() != keys.len() {
        return Ok(malformed("values"));
    }
    for (state, key) in keys.iter().enumerate() {
        cx.work(1)?;
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let current = &graph.states[*key];
        let expected_value = if let Some(index) = source[state] {
            let component = &document.components[index];
            let first = &component.bias[0];
            if *key == &first.state
                && (component.gain != component.value || first.value != rational(0))
            {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    (*key).clone(),
                ));
            }
            let bias = component
                .bias
                .binary_search_by(|row| row.state.cmp(key))
                .ok()
                .and_then(|index| component.bias.get(index))
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let mut sum = if current.terminal == Some(true) {
                rational(1)
            } else {
                rational(0)
            };
            sum = cx.arithmetic(quire_exact::RationalArithmetic::Subtract(
                &sum,
                &component.value,
            ))?;
            if current.choices.is_empty() {
                sum = cx.arithmetic(quire_exact::RationalArithmetic::Add(&sum, &bias.value))?;
            } else {
                for edge in &current.choices[0] {
                    cx.work(1)?;
                    let next = component
                        .bias
                        .binary_search_by(|row| row.state.cmp(&edge.target))
                        .ok()
                        .and_then(|index| component.bias.get(index))
                        .ok_or(ProbabilityDocumentError::Malformed)?;
                    let term = cx.arithmetic(quire_exact::RationalArithmetic::Multiply(
                        &edge.probability,
                        &next.value,
                    ))?;
                    sum = cx.arithmetic(quire_exact::RationalArithmetic::Add(&sum, &term))?;
                }
            }
            if sum != bias.value {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    (*key).clone(),
                ));
            }
            component.value.clone()
        } else {
            let mut sum = rational(0);
            for edge in &current.choices[0] {
                cx.work(1)?;
                let next = values
                    .get(&edge.target)
                    .ok_or(ProbabilityDocumentError::Malformed)?;
                let term = cx.arithmetic(quire_exact::RationalArithmetic::Multiply(
                    &edge.probability,
                    next,
                ))?;
                sum = cx.arithmetic(quire_exact::RationalArithmetic::Add(&sum, &term))?;
            }
            sum
        };
        if **value != expected_value {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                (*key).clone(),
            ));
        }
    }
    let initial = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    let threshold = match bound {
        Bound::AtLeast(v) | Bound::AtMost(v) => v,
    };
    check_scalar(threshold, cx.limits)?;
    if !match bound {
        Bound::AtLeast(v) => *initial >= v,
        Bound::AtMost(v) => *initial <= v,
    } {
        return Ok(rejected(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial.clone(),
        ));
    }
    poll(cx.cancel)?;
    Ok(ExactProbabilityCertificateCheck::Accepted(
        ExactProbabilityAcceptance {
            certificate: ExactProbabilityCertificate {
                values: document.values.clone(),
            },
            document: None,
            value: (*initial).clone(),
            side: match bound {
                Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
                Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
            },
        },
    ))
}

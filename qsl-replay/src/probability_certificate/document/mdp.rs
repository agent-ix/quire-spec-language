// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201's exact cyclic MDP values, independently checked against all actions.

use super::*;

/// Unchecked exact document with its closed wire-profile selection retained.
/// Existing document literals retain their original meaning and encoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilityProofDocument {
    document: ProbabilityCertificateDocument,
    cyclic_mdp: bool,
}

impl ProbabilityProofDocument {
    /// Borrow the untrusted identity, values, ranks and objective data.
    pub fn document(&self) -> &ProbabilityCertificateDocument {
        &self.document
    }

    /// Whether this document selects the cyclic MDP eventual proof profile.
    pub fn is_cyclic_mdp(&self) -> bool {
        self.cyclic_mdp
    }
}

/// Read a strict exact profile without discarding its wire discriminator.
pub fn read_probability_proof_document(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    let cyclic_mdp = profile_dispatch(bytes, limits, cancel)? == ProfileDispatch::CyclicMdp;
    let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let document = read_document_profile(bytes, limits, cancel, &mut meter, cyclic_mdp)?;
    Ok(ProbabilityProofDocument {
        document,
        cyclic_mdp,
    })
}

/// Write the shared canonical schema with the retained exact profile.
/// This produces unchecked evidence, never an acceptance token.
pub fn write_probability_proof_document(
    document: &ProbabilityProofDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let bytes = writer::encode_profile(&document.document, limits, cancel, document.cyclic_mdp)?;
    read_probability_proof_document(&bytes, limits, cancel)?;
    Ok(bytes)
}

/// Prefer existing eligible profiles before constructing cyclic MDP evidence.
/// All admission attempts share the same arithmetic and retained-storage scope.
pub fn probability_proof_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel, None);
    match construct_document(
        system,
        claims,
        item,
        certificate,
        limits,
        cancel,
        &mut cx.meter,
        &mut cx.storage,
    ) {
        Ok(document) => Ok(ProbabilityProofDocument {
            document,
            cyclic_mdp: false,
        }),
        Err(error) if may_retry(&error) => construct(system, claims, item, certificate, &mut cx),
        Err(error) => Err(error),
    }
}

fn may_retry(error: &ProbabilityDocumentError) -> bool {
    matches!(
        error,
        ProbabilityDocumentError::Unsupported
            | ProbabilityDocumentError::UndefinedPredicate(_)
            | ProbabilityDocumentError::Graph(ExactProbabilityGraphError::Cycle)
    ) || matches!(error, ProbabilityDocumentError::Build(issue)
            if matches!(issue.as_ref(), crate::ExactProbBuildIssue::Predicate(_)))
}

/// Construct this profile explicitly, including for a one-action DTMC subtype.
/// The independent checker must still verify every supplied value and rank.
pub fn cyclic_mdp_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    construct(
        system,
        claims,
        item,
        certificate,
        &mut Context::new(limits, cancel, None),
    )
}

/// Actual stages of the cyclic MDP checker, for default-off test observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CyclicMdpProbabilityPhase {
    /// Complete original model admission without evaluating the query predicate.
    OriginalAdmission,
    /// Initial reachable graph stopped at the first true predicate.
    StoppedGraph,
    /// Retaining offered value and progress rows.
    Rows,
    /// Exact sums over separate, source-reconstructed action distributions.
    ActionSums,
    /// Checking the positive-support closure of zero states.
    ZeroSupport,
    /// Checking every-action or tight-action progress.
    Progress,
    /// Deriving an unchecked producer progress witness.
    Ranks,
}

struct Context<'a> {
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    meter: quire_exact::Meter,
    storage: u64,
    identity: qsl_foundation::IdentityLimits,
    observe: Option<&'a mut dyn FnMut(CyclicMdpProbabilityPhase)>,
}
impl<'a> Context<'a> {
    fn new(
        limits: ProbabilityDocumentLimits,
        cancel: &'a Cancel,
        observe: Option<&'a mut dyn FnMut(CyclicMdpProbabilityPhase)>,
    ) -> Self {
        Self {
            limits,
            cancel,
            meter: quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone()),
            storage: 0,
            identity: qsl_foundation::IdentityLimits::default(),
            observe,
        }
    }
    fn tick(&mut self, phase: CyclicMdpProbabilityPhase) -> Result<(), ProbabilityDocumentError> {
        if let Some(observe) = self.observe.as_mut() {
            observe(phase);
        }
        super::super::reward::visit(self.cancel, &mut self.meter)
    }
    fn reserve(&mut self, bytes: usize) -> Result<(), ProbabilityDocumentError> {
        let mut admission = ProbabilityKeyAdmission::new(
            &mut self.meter,
            &mut self.storage,
            self.limits.storage_bytes,
            self.cancel,
        );
        quire_exact::Reservation::reserve(&mut admission, bytes)
            .map_err(|_| admission.refusal(qsl_eval::simulation::model::ModelKeyError::Denied))
    }
}

pub(super) fn member_shapes(
    document: &quire_canonical::Document,
    cancel: &Cancel,
    meter: &mut quire_exact::Meter,
) -> Result<(), ProbabilityDocumentError> {
    let root = document.root();
    for member in ["values", "ranking"] {
        let Some(quire_canonical::Node::Array(rows)) = root.get(member).map(|node| node.node())
        else {
            return Err(ProbabilityDocumentError::MalformedMember(member));
        };
        for row in rows {
            super::super::reward::visit(cancel, meter)?;
            let shape = matches!(row.node(), quire_canonical::Node::Object(fields) if fields.clone().count() == 2)
                && row.get("state").is_some();
            let payload = if member == "values" {
                row.get("value").is_some_and(|value| {
                    matches!(value.node(), quire_canonical::Node::Object(fields) if fields.clone().count() == 2)
                        && value.get("numerator").is_some() && value.get("denominator").is_some()
                })
            } else {
                row.get("rank").is_some()
            };
            if !shape || !payload {
                return Err(ProbabilityDocumentError::MalformedMember(member));
            }
        }
    }
    Ok(())
}

fn subject(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    cx: &mut Context<'_>,
) -> Result<(CheckerGraph, ProbabilityObjective, Bound), ProbabilityDocumentError> {
    use qsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
    if !std::ptr::eq(
        system.package().graph().probabilistic_model().claims(),
        claims,
    ) {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cx.cancel)?;
    let ProbForm::Reach {
        bound,
        target: ReachTarget::Eventually(predicate),
    } = &claim.form
    else {
        return Err(ProbabilityDocumentError::Unsupported);
    };
    if !matches!(claim.scheduler, ProbScheduler::Every)
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
    let (Bound::AtLeast(threshold) | Bound::AtMost(threshold)) = bound;
    if *threshold <= rational(0) || *threshold >= rational(1) {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|bound| &bound.reference);
    cx.tick(CyclicMdpProbabilityPhase::OriginalAdmission)?;
    let original = super::super::cyclic::enumerate_graph(
        system,
        None::<&qsl_semantics::check::CheckedPredicate>,
        reference,
        None,
        cx.limits,
        cx.cancel,
        cx.identity,
        &mut cx.meter,
        &mut cx.storage,
        false,
        false,
    )?;
    for state in original.states.values() {
        for choice in &state.choices {
            let mut mass = rational(0);
            for edge in choice {
                cx.tick(CyclicMdpProbabilityPhase::OriginalAdmission)?;
                if edge.probability < rational(0) {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                check_scalar(&edge.probability, cx.limits)?;
                mass = arithmetic(
                    quire_exact::RationalArithmetic::Add(&mass, &edge.probability),
                    &mut cx.meter,
                    cx.cancel,
                )?;
                check_scalar(&mass, cx.limits)?;
            }
            if mass != rational(1) {
                return Err(ProbabilityDocumentError::Malformed);
            }
        }
    }
    drop(original);
    cx.tick(CyclicMdpProbabilityPhase::StoppedGraph)?;
    let graph = super::super::cyclic::enumerate_graph(
        system,
        Some(predicate),
        reference,
        None,
        cx.limits,
        cx.cancel,
        cx.identity,
        &mut cx.meter,
        &mut cx.storage,
        true,
        true,
    )?;
    Ok((
        graph,
        claim_objective(&claim.scheduler, bound),
        bound.clone(),
    ))
}

type Values = BTreeMap<ProbabilityProductKey, Rational>;
type Sums = BTreeMap<ProbabilityProductKey, Vec<Rational>>;

fn rows(
    graph: &CheckerGraph,
    certificate: &ExactProbabilityCertificate,
    cx: &mut Context<'_>,
) -> Result<CertificateValues, ProbabilityDocumentError> {
    for row in &certificate.values {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        cx.reserve(
            std::mem::size_of::<(ProbabilityProductKey, Rational)>()
                .saturating_add(row.state.as_bytes().len())
                .saturating_add(
                    usize::try_from(
                        row.value
                            .numerator()
                            .magnitude_bits()
                            .saturating_add(row.value.denominator().magnitude_bits())
                            .div_ceil(8),
                    )
                    .unwrap_or(usize::MAX),
                ),
        )?;
        check_scalar(&row.value, cx.limits)?;
        if row.value < rational(0) || row.value > rational(1) {
            return Err(ProbabilityDocumentError::Malformed);
        }
    }
    Ok(certificate_values(graph, certificate, cx.cancel)?)
}

fn sums(
    graph: &CheckerGraph,
    values: &Values,
    cx: &mut Context<'_>,
) -> Result<Sums, ProbabilityDocumentError> {
    let mut sums = BTreeMap::new();
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::ActionSums)?;
        cx.reserve(
            std::mem::size_of::<(ProbabilityProductKey, Vec<Rational>)>()
                .saturating_add(key.as_bytes().len())
                .saturating_add(
                    state
                        .choices
                        .len()
                        .saturating_mul(std::mem::size_of::<Rational>()),
                ),
        )?;
        let mut actions = Vec::new();
        actions
            .try_reserve_exact(state.choices.len())
            .map_err(|_| {
                ProbabilityDocumentError::KeyEncoding(
                    qsl_eval::simulation::EncodingRefusal::Allocation {
                        requested: state
                            .choices
                            .len()
                            .saturating_mul(std::mem::size_of::<Rational>()),
                    },
                )
            })?;
        for (index, choice) in state.choices.iter().enumerate() {
            cx.tick(CyclicMdpProbabilityPhase::ActionSums)?;
            let sum = action_sum(
                key,
                index,
                choice,
                values,
                None,
                cx.limits,
                cx.cancel,
                &mut cx.meter,
                &super::super::reward::RewardControl::new(cx.cancel),
            )?;
            cx.reserve(
                usize::try_from(
                    sum.numerator()
                        .magnitude_bits()
                        .saturating_add(sum.denominator().magnitude_bits())
                        .div_ceil(8),
                )
                .unwrap_or(usize::MAX),
            )?;
            actions.push(sum);
        }
        sums.insert(key.clone(), actions);
    }
    Ok(sums)
}

fn equality_failure(
    graph: &CheckerGraph,
    values: &Values,
    sums: &Sums,
    objective: ProbabilityObjective,
    cx: &mut Context<'_>,
) -> Result<Option<ProbabilityProductKey>, ProbabilityDocumentError> {
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::ActionSums)?;
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let expected = match state.terminal {
            Some(target) => rational(i64::from(target)),
            None => {
                let choices = sums.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
                let mut expected: Option<Rational> = None;
                for choice in choices {
                    cx.tick(CyclicMdpProbabilityPhase::ActionSums)?;
                    expected = Some(match expected {
                        None => choice.clone(),
                        Some(previous) => match objective {
                            ProbabilityObjective::Minimum => previous.min(choice.clone()),
                            ProbabilityObjective::Maximum => previous.max(choice.clone()),
                            ProbabilityObjective::Workload => {
                                return Err(ProbabilityDocumentError::Unsupported)
                            }
                        },
                    });
                }
                expected.ok_or(ProbabilityDocumentError::Unsupported)?
            }
        };
        if *value != expected {
            return Ok(Some(key.clone()));
        }
    }
    Ok(None)
}

fn progress_failure(
    graph: &CheckerGraph,
    values: &Values,
    sums: &Sums,
    ranks: &BTreeMap<ProbabilityProductKey, u64>,
    objective: ProbabilityObjective,
    cx: &mut Context<'_>,
) -> Result<Option<(ProbabilityCertificateRule, ProbabilityProductKey)>, ProbabilityDocumentError> {
    for (key, state) in &graph.states {
        let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let rank = ranks.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        cx.tick(CyclicMdpProbabilityPhase::Progress)?;
        if state.terminal == Some(true) || *value == rational(0) {
            if *rank != 0 {
                return Ok(Some((ProbabilityCertificateRule::RankBroken, key.clone())));
            }
            if state.terminal.is_none() && *value == rational(0) {
                cx.tick(CyclicMdpProbabilityPhase::ZeroSupport)?;
                let mut closed = false;
                for choice in &state.choices {
                    let mut zero_support = true;
                    for edge in choice {
                        cx.tick(CyclicMdpProbabilityPhase::ZeroSupport)?;
                        if edge.probability > rational(0)
                            && values.get(&edge.target) != Some(&rational(0))
                        {
                            zero_support = false;
                        }
                    }
                    match objective {
                        ProbabilityObjective::Minimum => closed |= zero_support,
                        ProbabilityObjective::Maximum if !zero_support => {
                            return Ok(Some((
                                ProbabilityCertificateRule::InequalityFails,
                                key.clone(),
                            )))
                        }
                        ProbabilityObjective::Maximum => closed = true,
                        ProbabilityObjective::Workload => {
                            return Err(ProbabilityDocumentError::Unsupported)
                        }
                    }
                }
                if !closed {
                    return Ok(Some((
                        ProbabilityCertificateRule::InequalityFails,
                        key.clone(),
                    )));
                }
            }
            continue;
        }
        if state.terminal == Some(false) || *rank == 0 {
            return Ok(Some((ProbabilityCertificateRule::RankBroken, key.clone())));
        }
        let action_sums = sums.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
        let mut any = false;
        let mut all = true;
        for (index, choice) in state.choices.iter().enumerate() {
            let mut lower = false;
            for edge in choice {
                cx.tick(CyclicMdpProbabilityPhase::Progress)?;
                if edge.probability > rational(0)
                    && ranks.get(&edge.target).is_some_and(|next| next < rank)
                {
                    lower = true;
                }
            }
            all &= lower;
            if action_sums.get(index) == Some(value) {
                any |= lower;
            }
        }
        let progress = match objective {
            ProbabilityObjective::Minimum => all && !state.choices.is_empty(),
            ProbabilityObjective::Maximum => any,
            ProbabilityObjective::Workload => return Err(ProbabilityDocumentError::Unsupported),
        };
        if !progress {
            return Ok(Some((ProbabilityCertificateRule::RankBroken, key.clone())));
        }
    }
    Ok(None)
}

fn derive_ranks(
    graph: &CheckerGraph,
    values: &Values,
    sums: &Sums,
    objective: ProbabilityObjective,
    cx: &mut Context<'_>,
) -> Result<BTreeMap<ProbabilityProductKey, u64>, ProbabilityDocumentError> {
    let mut ranks = BTreeMap::new();
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::Ranks)?;
        if state.terminal == Some(true) || values.get(key) == Some(&rational(0)) {
            cx.reserve(std::mem::size_of::<(ProbabilityProductKey, u64)>() + key.as_bytes().len())?;
            ranks.insert(key.clone(), 0);
        }
    }
    while ranks.len() < graph.states.len() {
        let mut changed = false;
        for (key, state) in &graph.states {
            cx.tick(CyclicMdpProbabilityPhase::Ranks)?;
            if ranks.contains_key(key) {
                continue;
            }
            let value = values.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            let action_sums = sums.get(key).ok_or(ProbabilityDocumentError::Malformed)?;
            let mut candidate = None;
            let mut all = !state.choices.is_empty();
            for (index, choice) in state.choices.iter().enumerate() {
                let mut lower = None;
                for edge in choice {
                    cx.tick(CyclicMdpProbabilityPhase::Ranks)?;
                    if edge.probability > rational(0) {
                        if let Some(rank) = ranks.get(&edge.target) {
                            lower = Some(lower.map_or(*rank, |previous: u64| previous.min(*rank)));
                        }
                    }
                }
                match objective {
                    ProbabilityObjective::Minimum => match lower {
                        Some(rank) => {
                            candidate =
                                Some(candidate.map_or(rank, |previous: u64| previous.max(rank)))
                        }
                        None => all = false,
                    },
                    ProbabilityObjective::Maximum if action_sums.get(index) == Some(value) => {
                        if let Some(rank) = lower {
                            candidate =
                                Some(candidate.map_or(rank, |previous: u64| previous.min(rank)));
                        }
                    }
                    ProbabilityObjective::Maximum => {}
                    ProbabilityObjective::Workload => {
                        return Err(ProbabilityDocumentError::Unsupported)
                    }
                }
            }
            if (objective == ProbabilityObjective::Maximum || all) && candidate.is_some() {
                let rank = candidate
                    .ok_or(ProbabilityDocumentError::Malformed)?
                    .checked_add(1)
                    .filter(|rank| *rank <= MAX_COUNT)
                    .ok_or(exceeded(
                        ProbabilityDocumentLimit::Rank,
                        MAX_COUNT,
                        u64::MAX,
                    ))?;
                cx.reserve(
                    std::mem::size_of::<(ProbabilityProductKey, u64)>() + key.as_bytes().len(),
                )?;
                ranks.insert(key.clone(), rank);
                changed = true;
            }
        }
        if !changed {
            return Err(ProbabilityDocumentError::Unsupported);
        }
    }
    Ok(ranks)
}

fn admitted_vector<T>(
    count: usize,
    cx: &mut Context<'_>,
) -> Result<Vec<T>, ProbabilityDocumentError> {
    let requested = count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(ProbabilityDocumentError::KeySizeOverflow)?;
    cx.reserve(requested)?;
    let mut result = Vec::new();
    result.try_reserve_exact(count).map_err(|_| {
        ProbabilityDocumentError::KeyEncoding(qsl_eval::simulation::EncodingRefusal::Allocation {
            requested,
        })
    })?;
    Ok(result)
}

fn construct(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    cx: &mut Context<'_>,
) -> Result<ProbabilityProofDocument, ProbabilityDocumentError> {
    let (graph, objective, _) = subject(system, claims, item, cx)?;
    let values = match rows(&graph, certificate, cx)? {
        CertificateValues::Complete(values) => values,
        CertificateValues::Rejected(_) => {
            return Err(ProbabilityDocumentError::MalformedMember("values"))
        }
    };
    let action_sums = sums(&graph, &values, cx)?;
    if equality_failure(&graph, &values, &action_sums, objective, cx)?.is_some() {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    }
    let ranks = derive_ranks(&graph, &values, &action_sums, objective, cx)?;
    let mut ranking = admitted_vector(ranks.len(), cx)?;
    for row in ranks {
        cx.tick(CyclicMdpProbabilityPhase::Ranks)?;
        ranking.push(row);
    }
    let mut targets = admitted_vector(graph.states.len(), cx)?;
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        if state.terminal == Some(true) {
            cx.reserve(key.as_bytes().len())?;
            targets.push(key.clone());
        }
    }
    let mut offered = admitted_vector(values.len(), cx)?;
    for (state, value) in values {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        offered.push(ExactProbabilityValue { state, value });
    }
    Ok(ProbabilityProofDocument {
        cyclic_mdp: true,
        document: ProbabilityCertificateDocument {
            cyclic_dtmc: false,
            identity: item.clone(),
            targets,
            event: None,
            failures: Vec::new(),
            reward: None,
            optimization: objective,
            values: offered,
            ranking,
        },
    })
}

pub(super) fn check<'a>(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    observe: Option<&'a mut dyn FnMut(CyclicMdpProbabilityPhase)>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    check_with_context(
        system,
        claims,
        item,
        bytes,
        Context::new(limits, cancel, observe),
    )
}

fn check_with_context(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    mut cx: Context<'_>,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let limits = cx.limits;
    let cancel = cx.cancel;
    let document = match read_document_profile(bytes, limits, cancel, &mut cx.meter, true) {
        Ok(document) => document,
        Err(error) => return document_admission_error(error),
    };
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let (graph, objective, bound) = subject(system, claims, item, &mut cx)?;
    let mut targets = document.targets.iter();
    let mut target_match = true;
    for (key, state) in &graph.states {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        if state.terminal == Some(true) && targets.next() != Some(key) {
            target_match = false;
        }
    }
    if targets.next().is_some() {
        target_match = false;
    }
    if document.optimization != objective || !target_match {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let values = match rows(
        &graph,
        &ExactProbabilityCertificate {
            values: document.values,
        },
        &mut cx,
    )? {
        CertificateValues::Complete(values) => values,
        CertificateValues::Rejected(rejection) => {
            return Ok(ExactProbabilityCertificateCheck::Rejected(rejection))
        }
    };
    let mut ranks = BTreeMap::new();
    for (key, rank) in document.ranking {
        cx.tick(CyclicMdpProbabilityPhase::Rows)?;
        cx.reserve(std::mem::size_of::<(ProbabilityProductKey, u64)>() + key.as_bytes().len())?;
        if !graph.states.contains_key(&key) {
            return Ok(malformed_ranking());
        }
        ranks.insert(key, rank);
    }
    if ranks.len() != graph.states.len() {
        return Ok(malformed_ranking());
    }
    let action_sums = sums(&graph, &values, &mut cx)?;
    if let Some(key) = equality_failure(&graph, &values, &action_sums, objective, &mut cx)? {
        return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
    }
    if let Some((rule, key)) =
        progress_failure(&graph, &values, &action_sums, &ranks, objective, &mut cx)?
    {
        return Ok(rejected(rule, key));
    }
    let mut result = check_threshold(&graph, &values, &bound, limits, cancel)?;
    if let ExactProbabilityCertificateCheck::Accepted(accepted) = &mut result {
        accepted.document = Some(bytes.into());
    }
    Ok(result)
}

/// Observe real MDP action, closure and progress work without replacing cancellation.
#[cfg(feature = "test-support")]
#[allow(clippy::too_many_arguments)]
pub fn check_cyclic_mdp_probability_certificate_observing<'a>(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &'a Cancel,
    observe: &'a mut dyn FnMut(CyclicMdpProbabilityPhase),
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    check(system, claims, item, bytes, limits, cancel, Some(observe))
}

/// Exercise this profile's real owner identity ceiling, without changing wire limits.
#[cfg(feature = "test-support")]
#[allow(clippy::too_many_arguments)]
pub fn check_cyclic_mdp_probability_certificate_with_identity(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity: qsl_foundation::IdentityLimits,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut cx = Context::new(limits, cancel, None);
    cx.identity = identity;
    check_with_context(system, claims, item, bytes, cx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    fn key(n: u8) -> ProbabilityProductKey {
        ProbabilityProductKey(vec![n])
    }
    fn fraction(n: i64, d: i64) -> Rational {
        Rational::new(n.into(), d.into()).unwrap()
    }
    fn fixture(objective: ProbabilityObjective) -> (CheckerGraph, Values) {
        let edge = |state, n, d| ProbabilityEdge {
            target: key(state),
            probability: fraction(n, d),
        };
        let states = BTreeMap::from([
            (
                key(0),
                CheckerState {
                    terminal: None,
                    choices: vec![
                        vec![edge(3, 1, 2), edge(1, 1, 2)],
                        vec![edge(3, 1, 4), edge(1, 3, 4)],
                    ],
                },
            ),
            (
                key(1),
                CheckerState {
                    terminal: None,
                    choices: vec![vec![edge(2, 2, 3), edge(0, 1, 3)]],
                },
            ),
            (
                key(2),
                CheckerState {
                    terminal: Some(true),
                    choices: vec![],
                },
            ),
            (
                key(3),
                CheckerState {
                    terminal: Some(false),
                    choices: vec![],
                },
            ),
        ]);
        let (s, t) = match objective {
            ProbabilityObjective::Minimum => (fraction(2, 5), fraction(4, 5)),
            ProbabilityObjective::Maximum => (fraction(2, 3), fraction(8, 9)),
            _ => unreachable!(),
        };
        (
            CheckerGraph {
                initial: key(0),
                states,
            },
            BTreeMap::from([
                (key(0), s),
                (key(1), t),
                (key(2), rational(1)),
                (key(3), rational(0)),
            ]),
        )
    }
    #[trace("TC-1404", "FR-201-AC-40", "FR-201-AC-41")]
    #[test]
    fn equal_positive_ranks_progress_through_zero_and_attractor_witnesses_hold() {
        for objective in [ProbabilityObjective::Minimum, ProbabilityObjective::Maximum] {
            let (graph, values) = fixture(objective);
            let cancel = Cancel::new();
            let mut cx = Context::new(super::super::tests::limits(), &cancel, None);
            let sums = sums(&graph, &values, &mut cx).unwrap();
            assert_eq!(
                equality_failure(&graph, &values, &sums, objective, &mut cx).unwrap(),
                None
            );
            let offered = BTreeMap::from([(key(0), 1), (key(1), 1), (key(2), 0), (key(3), 0)]);
            assert_eq!(
                progress_failure(&graph, &values, &sums, &offered, objective, &mut cx).unwrap(),
                None
            );
            let derived = derive_ranks(&graph, &values, &sums, objective, &mut cx).unwrap();
            assert_eq!(
                progress_failure(&graph, &values, &sums, &derived, objective, &mut cx).unwrap(),
                None
            );
            let broken = BTreeMap::from([(key(0), 0), (key(1), 1), (key(2), 0), (key(3), 0)]);
            assert!(
                progress_failure(&graph, &values, &sums, &broken, objective, &mut cx)
                    .unwrap()
                    .is_some()
            );
        }
    }
    #[trace("TC-1404", "FR-201-AC-40", "FR-201-AC-41")]
    #[test]
    fn probability_zero_edges_do_not_progress_and_non_target_one_needs_positive_rank() {
        let cancel = Cancel::new();
        let mut cx = Context::new(super::super::tests::limits(), &cancel, None);
        let graph = CheckerGraph {
            initial: key(0),
            states: BTreeMap::from([
                (
                    key(0),
                    CheckerState {
                        terminal: None,
                        choices: vec![vec![
                            ProbabilityEdge {
                                target: key(0),
                                probability: rational(1),
                            },
                            ProbabilityEdge {
                                target: key(1),
                                probability: rational(0),
                            },
                        ]],
                    },
                ),
                (
                    key(1),
                    CheckerState {
                        terminal: Some(true),
                        choices: vec![],
                    },
                ),
            ]),
        };
        let values = BTreeMap::from([(key(0), rational(1)), (key(1), rational(1))]);
        let action_sums = sums(&graph, &values, &mut cx).unwrap();
        for objective in [ProbabilityObjective::Minimum, ProbabilityObjective::Maximum] {
            assert!(progress_failure(
                &graph,
                &values,
                &action_sums,
                &BTreeMap::from([(key(0), 1), (key(1), 0)]),
                objective,
                &mut cx
            )
            .unwrap()
            .is_some());
        }
        let mut graph = graph;
        graph.states.get_mut(&key(0)).unwrap().choices[0] = vec![ProbabilityEdge {
            target: key(1),
            probability: rational(1),
        }];
        let action_sums = sums(&graph, &values, &mut cx).unwrap();
        for objective in [ProbabilityObjective::Minimum, ProbabilityObjective::Maximum] {
            assert_eq!(
                progress_failure(
                    &graph,
                    &values,
                    &action_sums,
                    &BTreeMap::from([(key(0), 1), (key(1), 0)]),
                    objective,
                    &mut cx
                )
                .unwrap(),
                None
            );
            assert!(progress_failure(
                &graph,
                &values,
                &action_sums,
                &BTreeMap::from([(key(0), 0), (key(1), 0)]),
                objective,
                &mut cx
            )
            .unwrap()
            .is_some());
        }
    }
}

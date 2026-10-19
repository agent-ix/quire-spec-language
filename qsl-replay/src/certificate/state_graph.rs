// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-169 (ADR-022 GC-1 to GC-4): the certificate EN-1 returns with a
//! `Witnessed` or `Holds` state-graph outcome from an unreduced run, and
//! [`check_state_graph`], the core's checker of it.
//!
//! The checker reads nothing from the engine but the certificate. It
//! re-admits the subject through `ModelSystem` (FR-120), re-derives the
//! instance's `over` binding from the subject's universes, recomputes every
//! successor and evaluates every claim predicate with `qsl-eval`'s code. It
//! applies its rules in this order and rejects at the first failing state in
//! certificate order (ascending state-key digest):
//!
//! 1. `Malformed`: the closure is not strictly ascending in the state-key
//!    domain, or a rank or count entry is unsorted, duplicated, names no
//!    closure state or counts above 2, before any recomputation;
//! 2. `InitialMissing`: an initial state is not in the closure;
//! 3. `SuccessorMissing`: a closure state the checker reached has a successor
//!    outside the closure; `BadState`: its expansion evaluated a contract
//!    conjunction undefined or refused, so its successors are not known;
//! 4. `PredicateUndefined`: a claim predicate is undefined or refused at a
//!    reached state;
//! 5. by form: `possible` and `always possible` check ranks (`RankMissing`,
//!    `RankBroken`); `unique path` checks `H`'s order and counts
//!    (`OrderBroken`, `CountBroken`).
//!
//! Only states reached from the initial states are checked; an unreached
//! closure entry is never used, so it cannot help a false claim pass.
//!
//! **Counts.** A reached state with no count entry counts 0, so the states
//! with count 0 are closed under successors and hold no `to` node: none
//! reaches `to`. A state with count 0 or 1 that is no `to` node must have
//! every edge to a counted state go to an earlier state in the order, so its
//! count is well founded and exact; a count of 2 needs no order, since it
//! only refutes a `from` node and so is never trusted for acceptance. That
//! lets a cycle of `H` that no `from` node reaches be certified.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{ModelSimState, ModelSystem, PredicateReading};
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_semantics::check::{
    CheckedPredicateBody, CheckedStateGraphClaim, CheckedStatePredicate, PredicateRole,
    StateGraphForm,
};
use quire_exact::{Cancel, ObjectReference, ValueType};

use crate::certificate::check::{
    admit, expansion_halt, finish, poll, stopped, widen, Budgets, Checker, Halt, Step,
};
use crate::certificate::closure::{CertificateRejection, ClosureRefusal, TemporalCertificateCheck};
use crate::certificate::{CertificateLocus, CertificateRule};
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits};

/// FR-169's state-graph certificate of one item instance.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StateGraphCertificate {
    /// Every explored model state's `quire.simulation.state-key/v1` digest,
    /// strictly ascending.
    pub closure: Vec<DigestRecord>,
    /// `possible` and `always possible`: `(closure index, d)`, the state's
    /// distance to the nearest target state, for every state that reaches
    /// one; ascending by index.
    pub ranks: Vec<(u64, u32)>,
    /// `unique path`: `(closure index, count, order)` for every node of `H`:
    /// its path count to `to`, saturated at 2, and its position in a reverse
    /// topological order of `H`; ascending by index.
    pub counts: Vec<(u64, u8, u64)>,
}

/// What [`check_state_graph`] checks: the model subject, re-admitted by the
/// checker, the claim and the instance's `over` binding.
pub struct StateGraphCertificateRequest<'a> {
    /// The model subject (FR-125).
    pub subject: ModelSubject<'a>,
    /// The checked claim.
    pub claim: &'a CheckedStateGraphClaim,
    /// The object the instance binds the claim's `over` parameter to; `None`
    /// for a claim with no `over`.
    pub over: Option<ObjectReference>,
}

/// FR-169: check `certificate` for `request`'s claim instance under
/// `limits` and the subject's own evaluation meter and candidate bound,
/// polling `cancel` at every state.
///
/// # Errors
///
/// [`ClosureRefusal`] when the subject does not admit, a root is unbounded,
/// the `over` binding is no instance of the subject, the subject has no
/// initial state, or a state cannot be encoded. A refusal is never an
/// acceptance.
pub fn check_state_graph(
    request: &StateGraphCertificateRequest<'_>,
    certificate: &StateGraphCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, ClosureRefusal> {
    if let Some(rejection) = malformed(certificate) {
        return Ok(TemporalCertificateCheck::Rejected(rejection));
    }
    let simulation = request.subject.simulation.clone();
    let budgets = Budgets::of(&simulation);
    let system = admit(simulation)?;
    let over = bound_over(request, &system)?;
    if system.initial().is_empty() {
        return Err(ClosureRefusal::NoInitialState);
    }
    if certificate.closure.len() > limits.limits.max_states {
        return finish(Err(stopped(
            ModelCheckLimit::MaxStates,
            widen(limits.limits.max_states),
        )));
    }
    let predicates = predicates(request.claim.form());
    let check = Check {
        system: &system,
        certificate,
        limits,
        budgets,
        cancel,
        over: over.as_ref(),
        predicates: &predicates,
    };
    finish(check.run(request.claim.form()))
}

/// The request's `over` binding, when it is one of the subject's instances
/// of the claim's `over` type (FR-125).
fn bound_over(
    request: &StateGraphCertificateRequest<'_>,
    system: &ModelSystem<'_>,
) -> Result<Option<ObjectReference>, ClosureRefusal> {
    match (request.claim.over(), &request.over) {
        (None, None) => Ok(None),
        (Some(parameter), Some(reference)) => {
            let instances = system
                .instances(&ValueType::Reference(parameter.object))
                .map_err(|refusal| ClosureRefusal::NotSimulated(Box::new(refusal)))?;
            for instance in &instances {
                let bound = system
                    .bound_reference(parameter.object, instance)
                    .map_err(ClosureRefusal::Fault)?;
                if bound == *reference {
                    return Ok(Some(bound));
                }
            }
            Err(ClosureRefusal::NoInstance)
        }
        (None, Some(_)) | (Some(_), None) => Err(ClosureRefusal::NoInstance),
    }
}

/// The claim's written predicates, each with its role; an omitted `from`
/// holds everywhere and has none.
fn predicates(form: &StateGraphForm) -> Vec<(PredicateRole, &CheckedPredicateBody)> {
    let all = match form {
        StateGraphForm::Possible { target } => vec![(PredicateRole::Target, target)],
        StateGraphForm::AlwaysPossible { from, target } => {
            vec![(PredicateRole::From, from), (PredicateRole::Target, target)]
        }
        StateGraphForm::UniquePath { from, to } => {
            vec![(PredicateRole::From, from), (PredicateRole::To, to)]
        }
    };
    all.into_iter()
        .filter_map(|(role, predicate)| match predicate {
            CheckedStatePredicate::ConstantTrue => None,
            CheckedStatePredicate::Checked(body) => Some((role, body)),
        })
        .collect()
}

/// The first `Malformed` entry: closure order and domain, then ranks, then
/// counts.
fn malformed(certificate: &StateGraphCertificate) -> Option<CertificateRejection> {
    let reject = |at| CertificateRejection {
        rule: CertificateRule::Malformed,
        at,
    };
    let mut previous: Option<&DigestRecord> = None;
    for state in &certificate.closure {
        let ordered = previous.is_none_or(|before| before < state);
        if state.domain() != DigestDomain::SimulationStateKeyV1 || !ordered {
            return Some(reject(CertificateLocus::ModelState(*state)));
        }
        previous = Some(state);
    }
    let entry = |index: u64| match usize::try_from(index)
        .ok()
        .and_then(|at| certificate.closure.get(at))
    {
        Some(state) => CertificateLocus::ModelState(*state),
        None => CertificateLocus::Entry { index },
    };
    let indices = certificate
        .ranks
        .iter()
        .map(|(index, _)| (*index, false))
        .collect::<Vec<_>>();
    let counted = certificate
        .counts
        .iter()
        .map(|(index, count, _)| (*index, *count > 2))
        .collect::<Vec<_>>();
    for section in [indices, counted] {
        let mut previous: Option<u64> = None;
        for (index, over) in section {
            let in_range = usize::try_from(index).is_ok_and(|at| at < certificate.closure.len());
            let ordered = previous.is_none_or(|before| before < index);
            if !in_range || !ordered || over {
                return Some(reject(entry(index)));
            }
            previous = Some(index);
        }
    }
    None
}

fn reject(rule: CertificateRule, at: DigestRecord) -> Halt {
    Halt::Check(TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::ModelState(at),
    }))
}

/// A reached closure state: the successor of each of its edges, in
/// expansion order, one entry per edge.
struct Reached {
    state: ModelSimState,
    successors: Vec<DigestRecord>,
    /// Each written predicate's truth there.
    holds: Vec<(PredicateRole, bool)>,
}

impl Reached {
    /// Whether `role` holds; an unwritten predicate holds everywhere.
    fn holds(&self, role: PredicateRole) -> bool {
        self.holds
            .iter()
            .find(|(written, _)| *written == role)
            .is_none_or(|(_, holds)| *holds)
    }
}

struct Check<'c, 'a> {
    system: &'c ModelSystem<'a>,
    certificate: &'c StateGraphCertificate,
    limits: &'c ModelCheckLimits,
    budgets: Budgets,
    cancel: &'c Cancel,
    over: Option<&'c ObjectReference>,
    predicates: &'c [(PredicateRole, &'c CheckedPredicateBody)],
}

impl Check<'_, '_> {
    fn poll(&self) -> Step<()> {
        poll(self.cancel)
    }

    fn halt(&self, stop: &qsl_eval::simulation::ExpansionStop) -> Halt {
        expansion_halt(stop, self.budgets, Checker::StateGraph)
    }

    fn member(&self, state: &DigestRecord) -> bool {
        self.certificate.closure.binary_search(state).is_ok()
    }

    /// The closure state an entry's index names; `malformed` admitted it.
    fn state_at(&self, index: u64) -> Step<DigestRecord> {
        usize::try_from(index)
            .ok()
            .and_then(|at| self.certificate.closure.get(at))
            .copied()
            .ok_or_else(|| Checker::StateGraph.fault("admitted-entry-names-a-closure-state").into())
    }

    fn run(&self, form: &StateGraphForm) -> Step<()> {
        self.poll()?;
        let mut initial = BTreeMap::new();
        for state in self.system.initial() {
            let (_, digest) = self.system.state_key(&state)?;
            initial.entry(digest).or_insert(state);
        }
        if let Some(missing) = initial.keys().find(|digest| !self.member(digest)) {
            return Err(reject(CertificateRule::InitialMissing, *missing));
        }
        let mut reached = self.explore(&initial)?;
        self.read_predicates(reached.iter_mut())?;
        match form {
            StateGraphForm::Possible { .. } => {
                let ranks = self.ranks()?;
                if let Some(missing) = initial.keys().find(|digest| !ranks.contains_key(digest)) {
                    return Err(reject(CertificateRule::RankMissing, *missing));
                }
                self.check_ranks(&reached, &ranks)
            }
            StateGraphForm::AlwaysPossible { .. } => {
                let ranks = self.ranks()?;
                if let Some((missing, _)) = reached.iter().find(|(digest, state)| {
                    state.holds(PredicateRole::From) && !ranks.contains_key(digest)
                }) {
                    return Err(reject(CertificateRule::RankMissing, *missing));
                }
                self.check_ranks(&reached, &ranks)
            }
            StateGraphForm::UniquePath { .. } => self.check_counts(&reached),
        }
    }

    /// Every closure state reachable from `initial`, expanded, checking
    /// `SuccessorMissing` then `BadState` at the first failing state in
    /// certificate order.
    fn explore(
        &self,
        initial: &BTreeMap<DigestRecord, ModelSimState>,
    ) -> Step<BTreeMap<DigestRecord, Reached>> {
        let mut reached: BTreeMap<DigestRecord, Reached> = BTreeMap::new();
        let mut queue = VecDeque::new();
        let mut leaves = BTreeSet::new();
        let mut bad = BTreeSet::new();
        let mut edges: usize = 0;
        for (digest, state) in initial {
            self.admit(&reached)?;
            reached.insert(
                *digest,
                Reached {
                    state: state.clone(),
                    successors: Vec::new(),
                    holds: Vec::new(),
                },
            );
            queue.push_back(*digest);
        }
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            let state = reached
                .get(&at)
                .map(|entry| entry.state.clone())
                .ok_or_else(|| Checker::StateGraph.fault("queued-state-is-reached"))?;
            let expansion = self
                .system
                .successors(&state)
                .map_err(|stop| self.halt(&stop))?;
            if expansion.findings.iter().any(|finding| {
                finding.undefined_contract().is_some() || finding.is_refused_contract()
            }) {
                bad.insert(at);
            }
            let mut successors = Vec::with_capacity(expansion.successors.len());
            for (_, next) in expansion.successors {
                if edges >= self.limits.limits.max_transitions {
                    return Err(stopped(
                        ModelCheckLimit::MaxTransitions,
                        widen(self.limits.limits.max_transitions),
                    ));
                }
                edges = edges.saturating_add(1);
                let (_, digest) = self.system.state_key(&next)?;
                successors.push(digest);
                if !self.member(&digest) {
                    leaves.insert(at);
                    continue;
                }
                if !reached.contains_key(&digest) {
                    self.admit(&reached)?;
                    reached.insert(
                        digest,
                        Reached {
                            state: next,
                            successors: Vec::new(),
                            holds: Vec::new(),
                        },
                    );
                    queue.push_back(digest);
                }
            }
            if let Some(entry) = reached.get_mut(&at) {
                entry.successors = successors;
            }
        }
        if let Some(first) = leaves.first() {
            return Err(reject(CertificateRule::SuccessorMissing, *first));
        }
        if let Some(first) = bad.first() {
            return Err(reject(CertificateRule::BadState, *first));
        }
        Ok(reached)
    }

    /// Charge one more reached state against `max_states`.
    fn admit(&self, reached: &BTreeMap<DigestRecord, Reached>) -> Step<()> {
        if reached.len() >= self.limits.limits.max_states {
            return Err(stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            ));
        }
        Ok(())
    }

    /// Read every written predicate at every reached state, in certificate
    /// order: `PredicateUndefined` at the first where one has no value.
    fn read_predicates<'r>(
        &self,
        states: impl Iterator<Item = (&'r DigestRecord, &'r mut Reached)>,
    ) -> Step<()> {
        let bodies: Vec<_> = self
            .predicates
            .iter()
            .map(|(_, body)| (*body, self.over))
            .collect();
        for (digest, entry) in states {
            self.poll()?;
            let readings = self
                .system
                .state_predicates(&entry.state, &bodies)
                .map_err(|stop| self.halt(&stop))?;
            let mut holds = Vec::with_capacity(readings.len());
            for ((role, _), reading) in self.predicates.iter().zip(readings) {
                match reading {
                    PredicateReading::True => holds.push((*role, true)),
                    PredicateReading::False => holds.push((*role, false)),
                    PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => {
                        return Err(reject(CertificateRule::PredicateUndefined, *digest));
                    }
                }
            }
            entry.holds = holds;
        }
        Ok(())
    }

    fn ranks(&self) -> Step<BTreeMap<DigestRecord, u32>> {
        self.certificate
            .ranks
            .iter()
            .map(|(index, d)| Ok((self.state_at(*index)?, *d)))
            .collect()
    }

    /// `RankBroken`: a reached ranked state with `d = 0` where the target
    /// fails, or with `d > 0` and no successor ranked `d - 1`.
    fn check_ranks(
        &self,
        reached: &BTreeMap<DigestRecord, Reached>,
        ranks: &BTreeMap<DigestRecord, u32>,
    ) -> Step<()> {
        for (digest, state) in reached {
            let Some(d) = ranks.get(digest) else {
                continue;
            };
            let holds = match d.checked_sub(1) {
                None => state.holds(PredicateRole::Target),
                Some(next) => state
                    .successors
                    .iter()
                    .any(|successor| ranks.get(successor) == Some(&next)),
            };
            if !holds {
                return Err(reject(CertificateRule::RankBroken, *digest));
            }
        }
        Ok(())
    }

    /// `OrderBroken`, then `CountBroken`, over the reached states in
    /// certificate order (module docs, "Counts").
    fn check_counts(&self, reached: &BTreeMap<DigestRecord, Reached>) -> Step<()> {
        let counts: BTreeMap<DigestRecord, (u8, u64)> = self
            .certificate
            .counts
            .iter()
            .map(|(index, count, order)| Ok((self.state_at(*index)?, (*count, *order))))
            .collect::<Step<_>>()?;
        for (digest, state) in reached {
            let Some((count, order)) = counts.get(digest) else {
                continue;
            };
            if *count >= 2 || state.holds(PredicateRole::To) {
                continue;
            }
            let backward = state.successors.iter().any(|successor| {
                counts
                    .get(successor)
                    .is_some_and(|(_, later)| later >= order)
            });
            if backward {
                return Err(reject(CertificateRule::OrderBroken, *digest));
            }
        }
        for (digest, state) in reached {
            let count = counts.get(digest).map_or(0, |(count, _)| *count);
            let expected = if state.holds(PredicateRole::To) {
                1
            } else {
                state
                    .successors
                    .iter()
                    .map(|successor| counts.get(successor).map_or(0, |(count, _)| *count))
                    .fold(0u8, |sum, count| sum.saturating_add(count).min(2))
            };
            if count != expected || (state.holds(PredicateRole::From) && count != 1) {
                return Err(reject(CertificateRule::CountBroken, *digest));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn digest(byte: u8) -> DigestRecord {
        DigestRecord::mint(DigestDomain::SimulationStateKeyV1, [byte; 32])
    }

    /// FR-169's `Malformed` rule, applied before any recomputation: an
    /// unsorted or duplicated closure, a rank naming no closure state, an
    /// unsorted rank and a count above 2 each reject at their entry.
    #[trace("TC-644")]
    #[test]
    fn malformed_entries_reject_before_recomputation() {
        let closure = vec![digest(1), digest(2)];
        let at = |certificate: StateGraphCertificate| {
            malformed(&certificate).map(|rejection| (rejection.rule, rejection.at))
        };
        let malformed_at = |locus| Some((CertificateRule::Malformed, locus));
        assert_eq!(
            at(StateGraphCertificate {
                closure: vec![digest(2), digest(1)],
                ..StateGraphCertificate::default()
            }),
            malformed_at(CertificateLocus::ModelState(digest(1)))
        );
        assert_eq!(
            at(StateGraphCertificate {
                closure: vec![digest(1), digest(1)],
                ..StateGraphCertificate::default()
            }),
            malformed_at(CertificateLocus::ModelState(digest(1)))
        );
        assert_eq!(
            at(StateGraphCertificate {
                closure: closure.clone(),
                ranks: vec![(0, 1), (2, 0)],
                counts: Vec::new(),
            }),
            malformed_at(CertificateLocus::Entry { index: 2 })
        );
        assert_eq!(
            at(StateGraphCertificate {
                closure: closure.clone(),
                ranks: vec![(1, 0), (0, 1)],
                counts: Vec::new(),
            }),
            malformed_at(CertificateLocus::ModelState(digest(1)))
        );
        assert_eq!(
            at(StateGraphCertificate {
                closure: closure.clone(),
                ranks: Vec::new(),
                counts: vec![(1, 3, 0)],
            }),
            malformed_at(CertificateLocus::ModelState(digest(2)))
        );
        assert_eq!(
            at(StateGraphCertificate {
                closure,
                ranks: vec![(0, 1), (1, 0)],
                counts: vec![(0, 1, 1), (1, 2, 0)],
            }),
            None
        );
    }

    /// FR-169's new rules and loci on the wire: each rule's kebab-case
    /// spelling, a model-state locus as its digest record, and an entry
    /// locus as its decimal index.
    #[trace("TC-644")]
    #[test]
    fn state_graph_rules_and_loci_write_their_wire_spelling() {
        for (rule, spelling) in [
            (CertificateRule::Malformed, "malformed"),
            (CertificateRule::PredicateUndefined, "predicate-undefined"),
            (CertificateRule::RankMissing, "rank-missing"),
            (CertificateRule::RankBroken, "rank-broken"),
            (CertificateRule::OrderBroken, "order-broken"),
            (CertificateRule::CountBroken, "count-broken"),
        ] {
            assert_eq!(serde_json::to_value(rule).expect("serializes"), spelling);
        }
        assert_eq!(
            serde_json::to_value(CertificateLocus::ModelState(digest(1))).expect("serializes"),
            serde_json::json!({
                "domain": "quire.simulation.state-key/v1",
                "digest": "01".repeat(32),
            })
        );
        assert_eq!(
            serde_json::to_value(CertificateLocus::Entry { index: u64::MAX }).expect("serializes"),
            serde_json::json!({ "entry": u64::MAX.to_string() })
        );
    }
}

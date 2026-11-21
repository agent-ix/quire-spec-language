// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lower and upper fraction expectations in the mean-refutation role.
use super::*;

const PROFILE: &str = "filament.probability-objective/value-bounds/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBoundObjective {
    profile: String,
    base: WireObjective,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WirePolicy {
    state: String,
    choice: Box<serde_json::value::RawValue>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBound {
    identity: WireIdentity,
    objective: WireBoundObjective,
    kind: String,
    values: Vec<WireValue>,
    ranking: Option<Vec<WireRank>>,
    policy: Option<Vec<WirePolicy>>,
}

pub(super) fn is_bound(bytes: &[u8]) -> Result<bool, ProbabilityDocumentError> {
    #[derive(Deserialize)]
    struct Dispatch<'a> {
        #[serde(borrow)]
        kind: &'a str,
    }
    let dispatch: Dispatch<'_> =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    Ok(matches!(dispatch.kind, "lower" | "upper"))
}

pub(super) fn admit(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
) -> Result<(), ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    let admission = RefCell::new(writer::Admission::with_used(
        limits.storage_bytes,
        cancel,
        *storage,
    ));
    let parsed = quire_canonical::read_with_reservation(
        bytes,
        limits.bytes,
        &mut writer::Reserve(&admission),
    );
    *storage = admission.borrow().used();
    if let Some(error) = admission.borrow_mut().take_stop() {
        return Err(error);
    }
    let parsed = parsed.map_err(|_| ProbabilityDocumentError::Malformed)?;
    let canonical = writer::encode_reserved(&parsed, limits.bytes, &admission)?;
    *storage = admission.borrow().used();
    if canonical != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    Ok(())
}

pub(in crate::probability_certificate) fn admit_subsystem(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    if !is_bound(bytes)? {
        read_measure_probability_certificate(bytes, limits, cancel)?;
        return Ok(());
    }
    let mut storage = 0;
    admit(bytes, limits, cancel, &mut storage)?;
    let wire: WireBound =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if wire.objective.profile != PROFILE {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let document = read_wire(
        WireDocument {
            identity: wire.identity,
            objective: wire.objective.base,
            kind: "exact".into(),
            values: wire.values,
            ranking: wire.ranking.unwrap_or_default(),
            conditional: None,
        },
        limits,
        cancel,
        &mut meter,
        &mut storage,
        0,
    )?;
    validate_ranges(&document, limits, cancel, &mut meter)
}

fn policy_action(
    choice: &str,
    state: &State,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
) -> Result<Option<usize>, ProbabilityDocumentError> {
    poll(cancel)?;
    charge(
        storage,
        limits.storage_bytes,
        u64::try_from(choice.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(4),
    )?;
    let hinted = crate::Schedule::read_transition(
        choice.as_bytes(),
        quire_canonical::Limits::new(limits.bytes),
    )
    .map_err(|_| ProbabilityDocumentError::MalformedMember("policy"))?;
    Ok(state.actions.iter().position(|action| matches!(action, Some(crate::ProductAction::Identity(schedule)) if schedule == &hinted)))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn check(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    item: &ObligationIdentity,
    bytes: &[u8],
    scheduler: &[(ProbabilityProductKey, crate::ProductAction)],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeanRefutationCheck, ProbabilityDocumentError> {
    let mut storage = 0;
    admit(bytes, limits, cancel, &mut storage)?;
    charge(
        &mut storage,
        limits.storage_bytes,
        u64::try_from(bytes.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(4),
    )?;
    let wire: WireBound =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    if wire.objective.profile != PROFILE {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let lower = match wire.kind.as_str() {
        "lower" => true,
        "upper" => false,
        _ => return Err(ProbabilityDocumentError::MalformedMember("kind")),
    };
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let selected = plan(
        system,
        claim,
        MeasurePayoffKind::Fraction,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    // A refutation bound has the opposite direction to a proof of the claim.
    if !matches!(
        (&selected.bound, lower),
        (Bound::AtLeast(_), false) | (Bound::AtMost(_), true)
    ) {
        return Err(ProbabilityDocumentError::MalformedMember("kind"));
    }
    let needs_policy = matches!(
        (selected.objective.optimization, lower),
        (ProbabilityObjective::Minimum, false) | (ProbabilityObjective::Maximum, true)
    );
    if wire.ranking.is_some() != lower || (wire.policy.is_some() && !needs_policy) {
        return Err(ProbabilityDocumentError::MalformedMember(
            if wire.ranking.is_some() != lower {
                "ranking"
            } else {
                "policy"
            },
        ));
    }
    let document = read_wire(
        WireDocument {
            identity: wire.identity,
            objective: wire.objective.base,
            kind: "exact".into(),
            values: wire.values,
            ranking: wire.ranking.unwrap_or_default(),
            conditional: None,
        },
        limits,
        cancel,
        &mut meter,
        &mut storage,
        0,
    )?;
    if document.identity != *item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    if document.payoff != MeasurePayoffKind::Fraction {
        return Err(ProbabilityDocumentError::MalformedMember("kind"));
    }
    if document.objective != selected.objective {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    validate_ranges(&document, limits, cancel, &mut meter)?;
    let named = document
        .values
        .iter()
        .map(|row| row.state.clone())
        .collect::<BTreeSet<_>>();
    let graph = enumerate(
        system,
        claim,
        &selected,
        limits,
        cancel,
        &mut meter,
        &mut storage,
        lower.then_some(&named),
    )?;
    if graph.initials.len() != 1 {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let rejection = |rule, key| {
        MeanRefutationCheck::Rejected(ProbabilityCertificateRejection {
            rule,
            at: ProbabilityCertificateLocus::ProductState(key),
        })
    };
    for row in &document.values {
        charge(
            &mut storage,
            limits.storage_bytes,
            u64::try_from(
                std::mem::size_of::<ExactProbabilityValue>()
                    .saturating_add(row.state.as_bytes().len()),
            )
            .unwrap_or(u64::MAX),
        )?;
    }
    let values = document
        .values
        .iter()
        .map(|row| (row.state.clone(), row.value.clone()))
        .collect::<BTreeMap<_, _>>();
    if !values.contains_key(&graph.initial)
        || values.keys().any(|key| !graph.states.contains_key(key))
        || (!lower && values.len() != graph.states.len())
    {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    }
    if lower && document.ranking.iter().map(|row| &row.0).ne(values.keys()) {
        return Err(ProbabilityDocumentError::MalformedMember("ranking"));
    }
    let mut policy = BTreeMap::new();
    for row in wire.policy.unwrap_or_default() {
        let key = decode_state(&row.state, limits, cancel, &mut storage)?;
        if policy
            .last_key_value()
            .is_some_and(|(last, _)| last >= &key)
            || !values.contains_key(&key)
        {
            return Err(ProbabilityDocumentError::MalformedMember("policy"));
        }
        let state = graph
            .states
            .get(&key)
            .ok_or(ProbabilityDocumentError::MalformedMember("policy"))?;
        if state.terminal.is_some() {
            return Err(ProbabilityDocumentError::MalformedMember("policy"));
        }
        let Some(index) = policy_action(row.choice.get(), state, limits, cancel, &mut storage)?
        else {
            return Ok(rejection(ProbabilityCertificateRule::InequalityFails, key));
        };
        policy.insert(key, index);
    }
    let zero = super::super::rational(0);
    let mut selected_policy = Vec::new();
    for (key, value) in &values {
        poll(cancel)?;
        let state = graph
            .states
            .get(key)
            .ok_or(ProbabilityDocumentError::MalformedMember("values"))?;
        if lower {
            meter
                .charge(
                    filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge).work(
                        Integer::from(u64::from(
                            usize::BITS.saturating_sub(document.ranking.len().leading_zeros()),
                        )),
                    ),
                )
                .map_err(|error| ProbabilityDocumentError::Accounting(Box::new(error)))?;
        }
        if lower
            && document
                .ranking
                .binary_search_by(|row| row.0.cmp(key))
                .ok()
                .and_then(|index| document.ranking.get(index))
                .map(|row| row.1)
                != Some(state.rank)
        {
            return Ok(rejection(
                ProbabilityCertificateRule::RankBroken,
                key.clone(),
            ));
        }
        if let Some(terminal) = &state.terminal {
            if value != terminal {
                return Ok(rejection(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            }
            continue;
        }
        let choice_storage = storage;
        charge(
            &mut storage,
            limits.storage_bytes,
            u64::try_from(
                state
                    .actions
                    .len()
                    .saturating_mul(std::mem::size_of::<(Vec<u8>, usize)>()),
            )
            .unwrap_or(u64::MAX),
        )?;
        let mut choices = Vec::new();
        for (index, action) in state.actions.iter().enumerate() {
            let capacity = match action {
                Some(crate::ProductAction::Identity(schedule)) => schedule.as_bytes().len(),
                Some(crate::ProductAction::PostState(target)) => target.as_bytes().len(),
                Some(crate::ProductAction::TerminalStutter) | None => 0,
                Some(_) => return Err(ProbabilityDocumentError::Unsupported),
            }
            .saturating_add(32);
            charge(
                &mut storage,
                limits.storage_bytes,
                u64::try_from(capacity.saturating_mul(2)).unwrap_or(u64::MAX),
            )?;
            let bytes = action
                .as_ref()
                .map(crate::ProductAction::to_bytes)
                .transpose()
                .map_err(|_| ProbabilityDocumentError::Malformed)?
                .unwrap_or_default();
            choices.push((bytes, index));
        }
        choices.sort_by(|a, b| a.0.cmp(&b.0));
        let mut satisfying = None;
        for (_, index) in choices {
            if policy.get(key).is_some_and(|hint| *hint != index) {
                continue;
            }
            let choice = state
                .choices
                .get(index)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let mut sum = super::super::rational(0);
            for edge in choice {
                let target = values.get(&edge.target).unwrap_or(&zero);
                let term = arithmetic(
                    filament_exact::RationalArithmetic::Multiply(&edge.probability, target),
                    &mut meter,
                    cancel,
                )?;
                sum = arithmetic(
                    filament_exact::RationalArithmetic::Add(&sum, &term),
                    &mut meter,
                    cancel,
                )?;
            }
            let holds = if lower {
                ordered(Order::LessOrEqual, value, &sum, &mut meter, cancel)?
            } else {
                ordered(Order::LessOrEqual, &sum, value, &mut meter, cancel)?
            };
            if !holds && (!needs_policy || policy.contains_key(key)) {
                return Ok(rejection(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            }
            if holds && needs_policy {
                satisfying = Some(index);
                break;
            }
        }
        storage = choice_storage;
        if needs_policy {
            let Some(index) = satisfying else {
                return Ok(rejection(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            };
            let action = state
                .actions
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let action_bytes = match action {
                crate::ProductAction::Identity(schedule) => schedule.as_bytes().len(),
                crate::ProductAction::PostState(target) => target.as_bytes().len(),
                crate::ProductAction::TerminalStutter => 0,
                _ => return Err(ProbabilityDocumentError::Unsupported),
            };
            charge(
                &mut storage,
                limits.storage_bytes,
                u64::try_from(
                    key.as_bytes()
                        .len()
                        .saturating_add(action_bytes)
                        .saturating_add(std::mem::size_of::<(
                            ProbabilityProductKey,
                            crate::ProductAction,
                        )>()),
                )
                .unwrap_or(u64::MAX),
            )?;
            selected_policy.push((key.clone(), action.clone()));
        }
    }
    // The witness and certificate must describe the same scheduler wherever
    // the certificate chooses a semantic action.
    if selected.objective.optimization == ProbabilityObjective::Workload {
        if !scheduler.is_empty() {
            return Ok(MeanRefutationCheck::PolicyParity);
        }
    } else {
        let resolved = selected_policy
            .iter()
            .map(|(key, action)| (key, action))
            .collect::<BTreeMap<_, _>>();
        for (key, action) in scheduler {
            let Some(state) = graph.states.get(key) else {
                return Ok(MeanRefutationCheck::PolicyParity);
            };
            if !state
                .actions
                .iter()
                .any(|candidate| candidate.as_ref() == Some(action))
            {
                return Ok(MeanRefutationCheck::PolicyParity);
            }
            if resolved
                .get(key)
                .is_some_and(|selected| *selected != action)
            {
                return Ok(MeanRefutationCheck::PolicyParity);
            }
        }
    }
    let value = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::MalformedMember("values"))?;
    if !match &selected.bound {
        Bound::AtLeast(threshold) => ordered(Order::Less, value, threshold, &mut meter, cancel)?,
        Bound::AtMost(threshold) => ordered(Order::Greater, value, threshold, &mut meter, cancel)?,
    } {
        return Ok(rejection(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial,
        ));
    }
    poll(cancel)?;
    Ok(MeanRefutationCheck::AcceptedBounds(selected_policy))
}

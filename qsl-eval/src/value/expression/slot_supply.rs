// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-390 supply admission, shared by the checked package and the package
//! read from its bytes alone (FR-533).

use super::*;
use quire_exact::NodeKey;

/// The checked package and the slot selected through its view.
pub(super) struct CheckedHost<'a, 'p, 's> {
    pub(super) package: &'a CheckedPackage,
    pub(super) selected: &'a qsl_package::SelectedProtocolTerm<'p, 's>,
}

impl SlotHost for CheckedHost<'_, '_, '_> {
    fn types(&self) -> &quire_semantic_value::declaration::TypeEnvironment {
        self.package.graph().scope().types()
    }

    fn binder_level(&self, key: &NodeKey) -> usize {
        let graph = self.package.graph();
        graph
            .protocols()
            .iter()
            .filter_map(|protocol| graph.protocol_node_keys(&protocol.name))
            .flat_map(|keys| &keys.binders)
            .find(|binder| binder.node == *key)
            .map_or(usize::MAX, |binder| binder.level)
    }

    fn population_types(
        &self,
        node: &NodeKey,
    ) -> Option<(
        &quire_checked::key::DeclarationKey,
        &[quire_exact::EffectiveId],
    )> {
        self.package.graph().protocol_population_types(node)
    }

    fn holds_model(&self, model: &qsl_semantics::model::domain_package::DomainPackageRef) -> bool {
        self.package.graph().model_selections().contains(model)
    }

    fn observation_is_for_view(
        &self,
        observation: &qsl_package::AdmittedProtocolObservation<'_>,
    ) -> bool {
        observation.is_for_view(self.selected.root().view())
    }
}

/// One binder a slot reads, as supply admission sees it.
pub(super) struct SourceView<'s> {
    pub(super) index: usize,
    pub(super) value_type: &'s quire_exact::ValueType,
    pub(super) binder: &'s NodeKey,
}

/// What supply admission reads from the package whose slot it admits for:
/// the checked package, or a package read from its bytes alone (FR-533).
pub(super) trait SlotHost {
    fn types(&self) -> &quire_semantic_value::declaration::TypeEnvironment;
    /// The level of the protocol binder `key`, `usize::MAX` for a node that
    /// is none.
    fn binder_level(&self, key: &NodeKey) -> usize;
    fn population_types(
        &self,
        node: &NodeKey,
    ) -> Option<(
        &quire_checked::key::DeclarationKey,
        &[quire_exact::EffectiveId],
    )>;
    fn holds_model(&self, model: &qsl_semantics::model::domain_package::DomainPackageRef) -> bool;
    fn observation_is_for_view(
        &self,
        observation: &qsl_package::AdmittedProtocolObservation<'_>,
    ) -> bool;
}

/// The frame a slot's supply admits to.
pub(super) struct AdmittedSupply {
    pub(super) frame: Vec<Option<Value>>,
    pub(super) unavailable:
        std::collections::BTreeMap<usize, (NodeKey, quire_semantic_value::catalog::CatalogCode)>,
    pub(super) binder_slots: std::collections::BTreeMap<usize, NodeKey>,
}

/// FR-390 supply admission, in the order it states: duplicates, unknown and
/// missing binders, then each value's membership and reference provenance.
/// It charges nothing.
pub(super) fn admit_slot_supply(
    host: &dyn SlotHost,
    sources: &[SourceView<'_>],
    frame_len: usize,
    supply: &ProtocolSupply<'_>,
) -> Result<AdmittedSupply, CallFailure> {
    let sources = sources
        .iter()
        .map(|source| (source.index, source, source.binder.clone()))
        .collect::<Vec<_>>();
    let mut supplied = supply.binders.iter().collect::<Vec<_>>();
    supplied.sort_by_key(|(key, _)| (host.binder_level(key), key.clone()));
    for (position, (key, _)) in supplied.iter().enumerate() {
        if supplied[..position].iter().any(|(prior, _)| prior == key) {
            return Err(protocol_slot::resolve::refused(
                "invalid_runtime_input",
                "duplicate-member",
                CallAdmissionLocus::Binder(key.clone()),
            ));
        }
    }
    for (key, _) in &supplied {
        if !sources.iter().any(|(_, _, source)| *source == *key) {
            return Err(protocol_slot::resolve::refused(
                "invalid_runtime_input",
                "unknown-member",
                CallAdmissionLocus::Binder(key.clone()),
            ));
        }
    }
    for (_, _, key) in &sources {
        if !supplied.iter().any(|(candidate, _)| candidate == key) {
            return Err(protocol_slot::resolve::refused(
                "invalid_runtime_input",
                "missing-member",
                CallAdmissionLocus::Binder(key.clone()),
            ));
        }
    }
    let mut frame = vec![None; frame_len];
    let mut unavailable = std::collections::BTreeMap::new();
    let binder_slots = sources
        .iter()
        .map(|(index, _, binder)| (*index, binder.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (index, source, key) in sources {
        let Some((_, supplied)) = supply
            .binders
            .iter()
            .find(|(candidate, _)| *candidate == key)
        else {
            return Err(CallFailure::Fault(InternalFault::new(
                "FR-390",
                "admitted-source-is-supplied",
            )));
        };
        match &supplied.value {
            SuppliedValue::Value(value) => {
                if let Some(quire_semantic_value::admission::ObservationAdmission::Admitted(
                    observation,
                )) = supply.observation
                {
                    let occurrences = protocol_slot::reference_occurrences(value, host.types());
                    let offending = occurrences.iter().find(|(_, reference)| {
                        protocol_slot::foreign_to_observation(reference, observation)
                            || !observation
                                .environment()
                                .has_population_universe(reference.universe())
                    });
                    if let Some((_, reference)) = offending {
                        let required = match source.value_type {
                            quire_exact::ValueType::Reference(expected) => observation
                                .environment()
                                .population_universe_for_type(expected),
                            _ => None,
                        };
                        let supplied_universe = reference.universe();
                        return Err(protocol_slot::resolve::foreign_reference_refused(
                            required,
                            supplied_universe,
                            CallAdmissionLocus::Binder(key.clone()),
                        ));
                    }
                }
                let admitted =
                    host.types()
                        .admits(source.value_type, value)
                        .map_err(|failure| CallFailure::Storage {
                            parameter: index,
                            failure,
                        })?;
                if !admitted {
                    return Err(protocol_slot::resolve::refused(
                        "invalid_runtime_input",
                        if protocol_slot::resolve::shape_matches(
                            source.value_type,
                            value,
                            host.types(),
                        ) {
                            "invalid-value"
                        } else {
                            "wrong-value-kind"
                        },
                        CallAdmissionLocus::Binder(key),
                    ));
                }
                let occurrences = protocol_slot::reference_occurrences(value, host.types());
                for (position, metadata) in supplied.references.iter().enumerate() {
                    if supplied.references[..position]
                        .iter()
                        .any(|prior| prior.path == metadata.path)
                    {
                        return Err(CallFailure::Admission(Box::new(
                            CallAdmissionFailure::Refused(quire_eval::CallAdmissionRecord {
                                record: occurrence_record(
                                    "invalid_runtime_input",
                                    "duplicate-member",
                                    metadata.path.clone(),
                                ),
                                locus: CallAdmissionLocus::Binder(key),
                            }),
                        )));
                    }
                }
                for metadata in &supplied.references {
                    if !occurrences.iter().any(|(path, _)| *path == metadata.path) {
                        return Err(CallFailure::Admission(Box::new(
                            CallAdmissionFailure::Refused(quire_eval::CallAdmissionRecord {
                                record: occurrence_record(
                                    "invalid_runtime_input",
                                    "unknown-member",
                                    metadata.path.clone(),
                                ),
                                locus: CallAdmissionLocus::Binder(key),
                            }),
                        )));
                    }
                }
                for (path, _) in &occurrences {
                    if !supplied
                        .references
                        .iter()
                        .any(|metadata| metadata.path == *path)
                    {
                        return Err(CallFailure::Admission(Box::new(
                            CallAdmissionFailure::Refused(quire_eval::CallAdmissionRecord {
                                record: occurrence_record(
                                    "invalid_runtime_input",
                                    "missing-member",
                                    path.clone(),
                                ),
                                locus: CallAdmissionLocus::Binder(key),
                            }),
                        )));
                    }
                }
                for (path, reference) in &occurrences {
                    let Some(metadata) = supplied
                        .references
                        .iter()
                        .find(|metadata| metadata.path == *path)
                    else {
                        return Err(CallFailure::Fault(InternalFault::new(
                            "FR-390",
                            "reference-occurrence-metadata-is-matched",
                        )));
                    };
                    let Some((declaration, members)) = host.population_types(&metadata.population)
                    else {
                        return Err(protocol_slot::resolve::refused(
                            "invalid_model_binding",
                            "wrong-model-selection",
                            CallAdmissionLocus::Binder(key.clone()),
                        ));
                    };
                    if declaration.package != reference.object_type().original().package() {
                        return Err(protocol_slot::resolve::refused(
                            "invalid_model_binding",
                            "wrong-model-selection",
                            CallAdmissionLocus::Binder(key.clone()),
                        ));
                    }
                    if !members.iter().any(|member| {
                        host.types()
                            .conforms(reference.object_type().clone(), member.clone())
                    }) {
                        return Err(protocol_slot::resolve::refused(
                            "invalid_runtime_input",
                            "wrong-value-kind",
                            CallAdmissionLocus::Binder(key.clone()),
                        ));
                    }
                }
                if !occurrences.is_empty() {
                    let Some(observation_admission) = supply.observation else {
                        return Err(CallFailure::Admission(Box::new(
                            CallAdmissionFailure::Incomplete(quire_eval::CallAdmissionRecord {
                                record: occurrence_record(
                                    "unavailable_observation",
                                    "missing-required-artifact",
                                    occurrences[0].0.clone(),
                                ),
                                locus: CallAdmissionLocus::Binder(key),
                            }),
                        )));
                    };
                    let observation = match observation_admission {
                        quire_semantic_value::admission::ObservationAdmission::Admitted(
                            observation,
                        ) => observation,
                        quire_semantic_value::admission::ObservationAdmission::Refused(record) => {
                            return Err(CallFailure::Admission(Box::new(
                                CallAdmissionFailure::Refused(quire_eval::CallAdmissionRecord {
                                    record: record.clone(),
                                    locus: CallAdmissionLocus::Binder(key),
                                }),
                            )))
                        }
                        quire_semantic_value::admission::ObservationAdmission::Incomplete(
                            record,
                        ) => {
                            return Err(CallFailure::Admission(Box::new(
                                CallAdmissionFailure::Incomplete(quire_eval::CallAdmissionRecord {
                                    record: record.clone(),
                                    locus: CallAdmissionLocus::Binder(key),
                                }),
                            )))
                        }
                        quire_semantic_value::admission::ObservationAdmission::Fault(fault) => {
                            return Err(CallFailure::Fault(fault.clone()))
                        }
                    };
                    if !host.holds_model(observation.model_selection()) {
                        return Err(protocol_slot::resolve::refused(
                            "invalid_model_binding",
                            "wrong-model-selection",
                            CallAdmissionLocus::Binder(key),
                        ));
                    }
                    if !host.observation_is_for_view(observation) {
                        return Err(protocol_slot::resolve::refused(
                            "invalid_model_binding",
                            "wrong-model-selection",
                            CallAdmissionLocus::Binder(key),
                        ));
                    }
                    let mut seeds = Vec::with_capacity(occurrences.len());
                    for (path, reference) in occurrences {
                        let metadata = supplied
                            .references
                            .iter()
                            .find(|metadata| metadata.path == path)
                            .ok_or_else(|| {
                                CallFailure::Fault(InternalFault::new(
                                    "FR-390",
                                    "reference-occurrence-metadata-is-matched",
                                ))
                            })?;
                        if protocol_slot::foreign_to_observation(&reference, observation)
                            || !observation
                                .environment()
                                .has_population_universe(reference.universe())
                        {
                            let required = match source.value_type {
                                quire_exact::ValueType::Reference(expected) => observation
                                    .environment()
                                    .population_universe_for_type(expected),
                                _ => None,
                            };
                            return Err(protocol_slot::resolve::foreign_reference_refused(
                                required,
                                reference.universe(),
                                CallAdmissionLocus::Binder(key),
                            ));
                        }
                        seeds.push((reference, metadata.population.clone(), path));
                    }
                    if let Err(failure) =
                        qsl_semantics::model::observation::admit_protocol_reference_seeds(
                            host.types(),
                            observation.observation(),
                            &seeds,
                        )
                    {
                        return Err(protocol_reference_failure(observation, key, failure));
                    }
                }
                frame[index] = Some(value.clone());
            }
            SuppliedValue::Unavailable(cause) => {
                if let Some(metadata) = supplied.references.first() {
                    return Err(CallFailure::Admission(Box::new(
                        CallAdmissionFailure::Refused(quire_eval::CallAdmissionRecord {
                            record: occurrence_record(
                                "invalid_runtime_input",
                                "unknown-member",
                                metadata.path.clone(),
                            ),
                            locus: CallAdmissionLocus::Binder(key),
                        }),
                    )));
                }
                unavailable.insert(index, (key, *cause));
            }
        }
    }
    Ok(AdmittedSupply {
        frame,
        unavailable,
        binder_slots,
    })
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533 "Observation admission": the bare route's admission of a current
//! snapshot, over the type environment the package's own reads give.

use std::collections::BTreeMap;

use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::DomainPackageRef;
use qsl_semantics::model::intake::PackageInput;
use qsl_semantics::model::observation::{
    admit_current_snapshot, AdmissionFailure, DocumentRef, Observation, ObservationLimits,
};
use quire_semantic_value::admission::{AdmissionRecord, ObservationAdmission};
use quire_semantic_value::catalog::CatalogCode;

use super::BarePackage;
use crate::AdmittedProtocolObservation;

fn refused(catalog: CatalogCode, fields: BTreeMap<&'static str, String>) -> AdmissionRecord {
    AdmissionRecord {
        catalog,
        fields,
        path: Vec::new(),
        kernel: None,
    }
}

impl BarePackage<'_> {
    /// Admit one current snapshot for a model this package selects, over the
    /// package's own reads, its supply and the caller's domain package
    /// documents (FR-533 "Observation admission"). The returned authority
    /// belongs to this `BarePackage` value alone.
    pub fn admit_protocol_observation(
        &self,
        model: &DomainPackageRef,
        packages: &PackageInput,
        model_limits: ModelNormalizationLimits,
        snapshots: &BTreeMap<DocumentRef, Vec<u8>>,
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> ObservationAdmission<AdmittedProtocolObservation<'static>> {
        if !self.model_selections().contains(model) {
            return ObservationAdmission::Refused(refused(
                CatalogCode::new("invalid_model_binding", "wrong-model-selection"),
                BTreeMap::new(),
            ));
        }
        let unsupported = |reason: &str| {
            ObservationAdmission::Refused(refused(
                CatalogCode::new("unknown_required_feature", "unsupported-feature"),
                BTreeMap::from([
                    ("package", self.package().name().to_owned()),
                    ("reason", reason.to_owned()),
                ]),
            ))
        };
        if let Err(failure) = self.witness() {
            return match failure {
                crate::BareSelectionFailure::Identity(refusal) => unsupported(
                    &crate::reader_protocol_projection::UnsupportedReason::from(refusal).name(),
                ),
                _ => unsupported("missing-type-representation"),
            };
        }
        let Ok(world) = self.world() else {
            return unsupported("missing-type-representation");
        };
        if self.object_declarations().is_err() {
            return unsupported("missing-type-representation");
        }
        match admit_current_snapshot(
            core::slice::from_ref(model),
            world.types(),
            packages,
            model_limits,
            snapshots,
            selected,
            limits,
        ) {
            Ok((environment, usage)) => {
                let populations = environment.population_completeness().clone();
                ObservationAdmission::Admitted(AdmittedProtocolObservation::admitted_by_bare(
                    self.owner(),
                    model.clone(),
                    selected.clone(),
                    Observation {
                        identity: selected.clone(),
                        populations,
                        environment,
                        out_of_range: Vec::new(),
                    },
                    usage,
                ))
            }
            Err(AdmissionFailure::Refused(record)) => ObservationAdmission::Refused(*record),
            Err(AdmissionFailure::Incomplete(record)) => ObservationAdmission::Incomplete(*record),
            Err(AdmissionFailure::Fault(fault)) => ObservationAdmission::Fault(fault),
        }
    }
}

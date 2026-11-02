// SPDX-License-Identifier: AGPL-3.0-or-later
//! The in-process S4 evaluation view and the FR-106 authority it admits.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::DomainPackageRef;
use qsl_semantics::model::intake::PackageInput;
use qsl_semantics::model::observation::{
    admit_current_snapshot, AdmissionFailure, AdmissionUsage, DocumentRef, Observation,
    ObservationLimits,
};
use quire_exact::NodeKey;
use quire_semantic_value::admission::{AdmissionRecord, ObservationAdmission};
use quire_semantic_value::catalog::CatalogCode;
use quire_semantic_value::catalog::InternalFault;

use crate::CheckedPackage;

#[cfg(feature = "std")]
use crate::checked_v2::AdmittedPackageRead;

/// An S4 view tied to one successfully linked in-process package.
#[derive(Clone, Copy)]
pub struct AdmittedEvaluationView<'package> {
    package: &'package CheckedPackage,
    #[cfg(feature = "std")]
    reader: Option<&'package AdmittedPackageRead<'package>>,
}

impl<'package> AdmittedEvaluationView<'package> {
    pub(crate) fn in_process(package: &'package CheckedPackage) -> Self {
        Self {
            package,
            #[cfg(feature = "std")]
            reader: None,
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn reader(read: &'package AdmittedPackageRead<'package>) -> Self {
        Self {
            package: read.source_package(),
            reader: Some(read),
        }
    }

    /// Admit one current snapshot for a model selected by this package.
    #[allow(clippy::too_many_arguments)]
    pub fn admit_protocol_observation(
        &self,
        model: &DomainPackageRef,
        packages: &PackageInput,
        model_limits: ModelNormalizationLimits,
        snapshots: &BTreeMap<DocumentRef, Vec<u8>>,
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> ObservationAdmission<AdmittedProtocolObservation<'package>> {
        if !self.package.graph().model_selections().contains(model) {
            return ObservationAdmission::Refused(AdmissionRecord {
                catalog: CatalogCode::new("invalid_model_binding", "wrong-model-selection"),
                fields: BTreeMap::new(),
                path: Vec::new(),
                kernel: None,
            });
        }

        match admit_current_snapshot(
            core::slice::from_ref(model),
            self.package.graph().scope().types(),
            packages,
            model_limits,
            snapshots,
            selected,
            limits,
        ) {
            Ok((environment, usage)) => {
                let populations = environment.population_completeness().clone();
                ObservationAdmission::Admitted(AdmittedProtocolObservation {
                    package: self.package,
                    model_selection: model.clone(),
                    identity: selected.clone(),
                    observation: Observation {
                        identity: selected.clone(),
                        populations,
                        environment,
                        out_of_range: Vec::new(),
                    },
                    usage,
                })
            }
            Err(AdmissionFailure::Refused(record)) => ObservationAdmission::Refused(*record),
            Err(AdmissionFailure::Incomplete(record)) => ObservationAdmission::Incomplete(*record),
            Err(AdmissionFailure::Fault(fault)) => ObservationAdmission::Fault(fault),
        }
    }

    /// Resolve a checked node through this exact admitted view.
    ///
    /// A reader-backed view only returns handles for nodes present in its
    /// retained I2 graph and in the source package that emitted those bytes.
    /// The handle borrows this view, so it cannot be detached from its
    /// admission context.
    pub fn term_handle(&self, node: NodeKey) -> Option<TermHandle<'_, 'package>> {
        let present = {
            #[cfg(feature = "std")]
            if let Some(reader) = self.reader {
                reader.has_node(&node)
            } else {
                self.package
                    .graph()
                    .semantic_graph()
                    .node(node.clone())
                    .is_some()
            }
            #[cfg(not(feature = "std"))]
            {
                self.package
                    .graph()
                    .semantic_graph()
                    .node(node.clone())
                    .is_some()
            }
        };
        present.then_some(TermHandle { view: self, node })
    }

    /// Whether this view came from successful I2 admission of emitted bytes.
    #[cfg(feature = "std")]
    pub fn is_reader_backed(&self) -> bool {
        self.reader.is_some()
    }
}

/// An opaque checked-node selection tied to one [`AdmittedEvaluationView`].
#[derive(Clone)]
pub struct TermHandle<'view, 'package> {
    view: &'view AdmittedEvaluationView<'package>,
    node: NodeKey,
}

impl TermHandle<'_, '_> {
    /// The already-checked source package whose expression is selected by
    /// this view. Construction of the handle remains private to the view.
    pub fn package(&self) -> &CheckedPackage {
        self.view.package
    }

    /// The structural identity selected through the view.
    pub fn node(&self) -> &NodeKey {
        &self.node
    }
}

/// Why an admitted package could not expose its evaluation view.
#[derive(Debug)]
pub enum ViewFailure {
    /// An emitted source node was absent from the admitted I2 graph.
    ReaderSourceMismatch(InternalFault),
}

/// The successful, immutable current-snapshot authority for protocol slots.
/// Only [`AdmittedEvaluationView::admit_protocol_observation`] constructs it.
/// A caller cannot construct it from a plain observation:
///
/// ```compile_fail
/// use qsl_package::AdmittedProtocolObservation;
/// use qsl_semantics::model::observation::Observation;
/// fn forge(observation: Observation) -> AdmittedProtocolObservation<'static> {
///     observation.into()
/// }
/// ```
///
/// Its environment is read-only through the public accessors:
///
/// ```compile_fail
/// # use qsl_package::AdmittedProtocolObservation;
/// fn mutate(authority: &AdmittedProtocolObservation<'_>) {
///     authority.environment().set_place_presence(Default::default());
/// }
/// ```
#[derive(Clone, Debug)]
pub struct AdmittedProtocolObservation<'view> {
    package: &'view CheckedPackage,
    model_selection: DomainPackageRef,
    identity: DocumentRef,
    observation: Observation,
    usage: AdmissionUsage,
}

impl AdmittedProtocolObservation<'_> {
    /// Whether this authority was admitted by the exact in-process package
    /// instance now evaluating it.
    pub fn is_for_package(&self, package: &CheckedPackage) -> bool {
        core::ptr::eq(self.package, package)
    }

    /// FR-106 admission usage, separate from S6a evaluation charges.
    pub fn usage(&self) -> AdmissionUsage {
        self.usage
    }

    /// The admitted snapshot identity.
    pub fn identity(&self) -> &DocumentRef {
        &self.identity
    }

    /// The exact model selection used for admission.
    pub fn model_selection(&self) -> &DomainPackageRef {
        &self.model_selection
    }

    /// The admitted object environment.
    pub fn environment(&self) -> &qsl_semantics::model::object_environment::ObjectEnvironment {
        &self.observation.environment
    }

    /// The admitted completeness manifest keyed by population declaration
    /// identity.
    pub fn populations(&self) -> &BTreeMap<String, bool> {
        &self.observation.populations
    }

    /// The immutable FR-106 observation retained by this authority.
    pub fn observation(&self) -> &Observation {
        &self.observation
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! The in-process S4 evaluation view and the FR-106 authority it admits.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
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

use crate::protocol_selection::{self, ProtocolSlotDescriptor, Selected, SlotSelectionFailure};
use crate::CheckedPackage;

#[cfg(feature = "std")]
use crate::checked_v2::AdmittedPackageRead;

/// An S4 view tied to one successfully linked in-process package.
pub struct AdmittedEvaluationView<'package> {
    package: &'package CheckedPackage,
    identity: Arc<()>,
    #[cfg(feature = "std")]
    reader: Option<&'package AdmittedPackageRead<'package>>,
}

impl<'package> AdmittedEvaluationView<'package> {
    pub(crate) fn in_process(package: &'package CheckedPackage) -> Self {
        Self {
            package,
            identity: Arc::new(()),
            #[cfg(feature = "std")]
            reader: None,
        }
    }

    #[cfg(feature = "std")]
    pub(crate) fn reader(read: &'package AdmittedPackageRead<'package>) -> Self {
        Self {
            package: read.source_package(),
            identity: Arc::new(()),
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
                    view_identity: Arc::clone(&self.identity),
                    #[cfg(feature = "std")]
                    reader: self.reader,
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

    /// Select one checked protocol expression through this exact admitted view.
    pub fn select_protocol_slot(
        &self,
        descriptor: &ProtocolSlotDescriptor,
    ) -> Result<SelectedProtocolTerm<'_, 'package>, SlotSelectionFailure> {
        let Selected {
            body,
            frame,
            root_node,
        } = protocol_selection::select(self.package, descriptor)?;
        let present = {
            #[cfg(feature = "std")]
            if let Some(reader) = self.reader {
                reader.has_node(&root_node)
            } else {
                self.package
                    .graph()
                    .semantic_graph()
                    .node(root_node.clone())
                    .is_some()
            }
            #[cfg(not(feature = "std"))]
            {
                self.package
                    .graph()
                    .semantic_graph()
                    .node(root_node.clone())
                    .is_some()
            }
        };
        if !present {
            return Err(SlotSelectionFailure::MissingNode(root_node));
        }
        let root = TermHandle {
            view: self,
            node: root_node,
            body,
        };
        let frame_len = frame.len();
        let mut sources = frame
            .into_iter()
            .enumerate()
            .filter(|(index, _)| body.reads_local(*index))
            .map(|(index, source)| {
                let binder = source.node.ok_or_else(|| {
                    SlotSelectionFailure::Fault(InternalFault::new(
                        "FR-390",
                        "selected-protocol-source-has-a-keyed-binder",
                    ))
                })?;
                Ok(ProtocolSource {
                    index,
                    name: source.name,
                    level: source.level,
                    value_type: source.value_type,
                    binder,
                    location: body.root().location().clone(),
                })
            })
            .collect::<Result<Vec<_>, SlotSelectionFailure>>()?;
        sources
            .sort_by(|left, right| (left.level, &left.binder).cmp(&(right.level, &right.binder)));
        Ok(SelectedProtocolTerm {
            root,
            sources,
            frame_len,
        })
    }

    /// Whether this view came from successful I2 admission of emitted bytes.
    #[cfg(feature = "std")]
    pub fn is_reader_backed(&self) -> bool {
        self.reader.is_some()
    }
}

/// An opaque checked-node selection tied to one [`AdmittedEvaluationView`].
/// Its only constructor is the view's validated protocol-slot selector.
///
/// ```compile_fail
/// use qsl_package::{AdmittedEvaluationView, TermHandle};
/// use quire_exact::NodeKey;
/// fn forge<'a>(view: &'a AdmittedEvaluationView<'a>, node: NodeKey) -> TermHandle<'a, 'a> {
///     TermHandle { view, node, body: todo!() }
/// }
/// ```
///
/// ```compile_fail
/// use qsl_package::TermHandle;
/// use quire_exact::NodeKey;
/// fn from_node(node: NodeKey) { let _: TermHandle<'_, '_> = node.into(); }
/// ```
///
/// ```compile_fail
/// use qsl_package::TermHandle;
/// use qsl_semantics::check::CheckedNode;
/// fn from_checked(node: &CheckedNode<'_>) { let _ = TermHandle::from(node); }
/// ```
///
/// ```compile_fail
/// use qsl_package::TermHandle;
/// use qsl_semantics::check::CheckedBody;
/// fn from_term(body: &CheckedBody) { let _ = TermHandle::from(body); }
/// ```
///
/// ```compile_fail
/// use qsl_package::TermHandle;
/// use quire_contract_model::CheckedPackageV2;
/// fn from_graph(graph: &CheckedPackageV2) { let _ = TermHandle::from(graph); }
/// ```
///
/// ```compile_fail
/// use qsl_package::TermHandle;
/// use qsl_semantics::library::ImportView;
/// fn from_import(view: &ImportView) { let _ = TermHandle::from(view); }
/// ```
///
/// ```compile_fail
/// use qsl_package::TermHandle;
/// fn from_bytes(bytes: &[u8]) { let _ = TermHandle::from(bytes); }
/// ```
///
/// ```compile_fail
/// use qsl_package::TermHandle;
/// fn from_integer(value: usize) { let _ = TermHandle::from(value); }
/// ```
///
/// ```compile_fail
/// use qsl_package::{AdmittedEvaluationView, TermHandle};
/// fn from_view(view: &AdmittedEvaluationView<'_>) { let _ = TermHandle::from(view); }
/// ```
#[derive(Clone)]
pub struct TermHandle<'view, 'package> {
    view: &'view AdmittedEvaluationView<'package>,
    node: NodeKey,
    body: &'package qsl_semantics::check::CheckedBody,
}

impl TermHandle<'_, '_> {
    /// The already-checked source package whose expression is selected by
    /// this view. Construction of the handle remains private to the view.
    pub fn package(&self) -> &CheckedPackage {
        self.view.package
    }

    /// The checked expression selected by this handle.
    pub fn checked_body(&self) -> &qsl_semantics::check::CheckedBody {
        self.body
    }

    /// The exact evaluation view that validated this handle.
    pub fn view(&self) -> &AdmittedEvaluationView<'_> {
        self.view
    }

    /// The structural identity selected through the view.
    pub fn node(&self) -> &NodeKey {
        &self.node
    }
}

/// One binder source selected from the checked expression frame.
pub struct ProtocolSource {
    index: usize,
    name: String,
    level: usize,
    value_type: quire_exact::ValueType,
    binder: NodeKey,
    location: quire_semantic_value::location::Location,
}

impl ProtocolSource {
    /// The source's position in the checked frame.
    pub fn index(&self) -> usize {
        self.index
    }
    /// The authored binder name.
    pub fn name(&self) -> &str {
        &self.name
    }
    /// The admitted binder level.
    pub fn level(&self) -> usize {
        self.level
    }
    /// The binder's checked value type.
    pub fn value_type(&self) -> &quire_exact::ValueType {
        &self.value_type
    }
    /// The binder's admitted structural identity.
    pub fn binder(&self) -> &NodeKey {
        &self.binder
    }
    /// The checked expression's read location.
    pub fn location(&self) -> &quire_semantic_value::location::Location {
        &self.location
    }
}

/// The selected expression and its sources, all tied to one admitted view.
pub struct SelectedProtocolTerm<'view, 'package> {
    root: TermHandle<'view, 'package>,
    sources: Vec<ProtocolSource>,
    frame_len: usize,
}

impl SelectedProtocolTerm<'_, '_> {
    /// The selected opaque expression handle.
    pub fn root(&self) -> &TermHandle<'_, '_> {
        &self.root
    }
    /// The selected expression's ordered source descriptors.
    pub fn sources(&self) -> &[ProtocolSource] {
        &self.sources
    }
    /// The checked body represented by the opaque selected handle.
    pub fn checked_body(&self) -> &qsl_semantics::check::CheckedBody {
        self.root.checked_body()
    }
    /// The checked frame length, including unread local slots.
    pub fn frame_len(&self) -> usize {
        self.frame_len
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
#[derive(Clone)]
pub struct AdmittedProtocolObservation<'view> {
    package: &'view CheckedPackage,
    view_identity: Arc<()>,
    #[cfg(feature = "std")]
    reader: Option<&'view AdmittedPackageRead<'view>>,
    model_selection: DomainPackageRef,
    identity: DocumentRef,
    observation: Observation,
    usage: AdmissionUsage,
}

impl<'view> AdmittedProtocolObservation<'view> {
    /// Whether this authority was admitted by the exact in-process package
    /// instance now evaluating it.
    pub fn is_for_package(&self, package: &CheckedPackage) -> bool {
        core::ptr::eq(self.package, package)
    }

    /// Whether this authority was admitted through this exact evaluation view.
    pub fn is_for_view(&self, view: &AdmittedEvaluationView<'_>) -> bool {
        Arc::ptr_eq(&self.view_identity, &view.identity)
    }

    /// The admitted evaluation view, retaining its exact ownership identity.
    pub fn evaluation_view(&self) -> AdmittedEvaluationView<'view> {
        AdmittedEvaluationView {
            package: self.package,
            identity: Arc::clone(&self.view_identity),
            #[cfg(feature = "std")]
            reader: self.reader,
        }
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

impl core::fmt::Debug for AdmittedProtocolObservation<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("AdmittedProtocolObservation")
            .field("model_selection", &self.model_selection)
            .field("identity", &self.identity)
            .field("observation", &self.observation)
            .field("usage", &self.usage)
            .finish_non_exhaustive()
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! The in-process S4 evaluation view and the FR-106 authority it admits.

use alloc::collections::{BTreeMap, BTreeSet};
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

    /// Project one selected reader-backed protocol expression into the closed
    /// evaluator vocabulary. Wire terms and producer evidence remain private
    /// to qsl-package; callers receive only owned nodes, exact types, locations,
    /// equality decisions, and the admitted function table.
    #[cfg(feature = "std")]
    pub fn project_reader_protocol_root(
        &self,
        source_graph: &qsl_semantics::check::CheckedGraph,
        key: &NodeKey,
        sources: &[ProtocolSource],
        root_provenance: &quire_semantic_value::location::Location,
    ) -> Result<Option<crate::ReaderProtocolProjection>, InternalFault> {
        let Some(read) = self.reader else {
            return Err(InternalFault::new(
                "FR-390",
                "reader-view-has-admitted-graph",
            ));
        };
        let root = read.reader_node_id(key)?.ok_or_else(|| {
            InternalFault::new("FR-390", "reader-selected-protocol-node-is-retained")
        })?;
        let locals = sources
            .iter()
            .map(|source| {
                read.reader_node_id(source.binder())?
                    .map(|node| (node, source.index()))
                    .ok_or_else(|| {
                        InternalFault::new("FR-390", "reader-protocol-binder-is-retained")
                    })
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        crate::reader_protocol_projection::project_protocol_root(
            source_graph,
            read.admitted_graph(),
            read.emission(),
            &root,
            &locals,
            root_provenance.clone(),
            read.effective_limits().work,
        )
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
        #[cfg(feature = "std")]
        let reader_projection = if let Some(reader) = self.reader {
            let binders = frame
                .iter()
                .map(|source| source.node.clone())
                .collect::<Vec<_>>();
            // The admitted wire carries no semantic location for a leaf
            // root, and this projection does not yet pair the
            // `protocol_locations` rows with its visits (QSL-1625), so it
            // reports no location. It never takes one from the source
            // package's CheckedBody (FR-390).
            reader
                .protocol_reference_slot(&root_node, &binders)
                .map_err(SlotSelectionFailure::Fault)?
        } else {
            None
        };
        let root = TermHandle {
            view: self,
            node: root_node,
            body,
        };
        let frame_len = frame.len();
        let wanted: BTreeSet<&NodeKey> =
            frame.iter().filter_map(|slot| slot.node.as_ref()).collect();
        let mut first_occurrence = BTreeMap::new();
        for (key, _, location) in self.package.graph().occurrences() {
            if let Some(wanted_key) = wanted.get(&key) {
                first_occurrence
                    .entry((*wanted_key).clone())
                    .or_insert(location);
            }
        }
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
                let location = first_occurrence
                    .get(&binder)
                    .map(|location| (*location).clone())
                    .ok_or_else(|| {
                        SlotSelectionFailure::Fault(InternalFault::new(
                            "FR-390",
                            "selected-protocol-source-has-an-occurrence",
                        ))
                    })?;
                Ok(ProtocolSource {
                    index,
                    name: source.name,
                    level: source.level,
                    value_type: source.value_type,
                    binder,
                    location,
                })
            })
            .collect::<Result<Vec<_>, SlotSelectionFailure>>()?;
        sources
            .sort_by(|left, right| (left.level, &left.binder).cmp(&(right.level, &right.binder)));
        Ok(SelectedProtocolTerm {
            root,
            sources,
            frame_len,
            #[cfg(feature = "std")]
            reader_projection,
        })
    }

    /// Whether this view came from successful I2 admission of emitted bytes.
    #[cfg(feature = "std")]
    pub fn is_reader_backed(&self) -> bool {
        self.reader.is_some()
    }

    /// Whether two view values retain the same admission owner identity.
    pub fn is_same_view(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.identity, &other.identity)
    }

    /// Whether this view belongs to `package`.
    pub fn is_for_package(&self, package: &CheckedPackage) -> bool {
        core::ptr::eq(self.package, package)
    }
}

/// An opaque checked-node selection tied to one [`AdmittedEvaluationView`].
/// Its only constructor is the view's validated protocol-slot selector.
///
/// ```compile_fail,E0451
/// use qsl_package::{AdmittedEvaluationView, TermHandle};
/// use quire_exact::NodeKey;
/// fn forge<'a>(view: &'a AdmittedEvaluationView<'a>, node: NodeKey) -> TermHandle<'a, 'a> {
///     TermHandle { view, node, body: todo!() }
/// }
/// ```
///
/// ```compile_fail,E0277
/// use qsl_package::TermHandle;
/// use quire_exact::NodeKey;
/// fn from_node(node: NodeKey) { let _: TermHandle<'_, '_> = node.into(); }
/// ```
///
/// ```compile_fail,E0277
/// use qsl_package::TermHandle;
/// use qsl_semantics::check::CheckedNode;
/// fn from_checked(node: &CheckedNode<'_>) { let _: TermHandle<'_, '_> = node.into(); }
/// ```
///
/// ```compile_fail,E0277
/// use qsl_package::TermHandle;
/// use qsl_semantics::check::CheckedBody;
/// fn from_term(body: &CheckedBody) { let _: TermHandle<'_, '_> = body.into(); }
/// ```
///
/// ```compile_fail,E0277
/// use qsl_package::TermHandle;
/// use quire_contract_model::CheckedPackageV2;
/// fn from_graph(graph: &CheckedPackageV2) { let _: TermHandle<'_, '_> = graph.into(); }
/// ```
///
/// ```compile_fail,E0277
/// use qsl_package::TermHandle;
/// use qsl_semantics::library::ImportView;
/// fn from_import(view: &ImportView) { let _: TermHandle<'_, '_> = view.into(); }
/// ```
///
/// ```compile_fail,E0277
/// use qsl_package::TermHandle;
/// fn from_bytes(bytes: &[u8]) { let _: TermHandle<'_, '_> = bytes.into(); }
/// ```
///
/// ```compile_fail,E0277
/// use qsl_package::TermHandle;
/// fn from_integer(value: usize) { let _: TermHandle<'_, '_> = value.into(); }
/// ```
///
/// ```compile_fail,E0277
/// use qsl_package::{AdmittedEvaluationView, TermHandle};
/// fn from_view(view: &AdmittedEvaluationView<'_>) { let _: TermHandle<'_, '_> = view.into(); }
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

    /// The structural node identity selected by this view.
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
    /// The location of the binder's own first occurrence.
    pub fn location(&self) -> &quire_semantic_value::location::Location {
        &self.location
    }
}

/// The selected expression and its sources, all tied to one admitted view.
pub struct SelectedProtocolTerm<'view, 'package> {
    root: TermHandle<'view, 'package>,
    sources: Vec<ProtocolSource>,
    frame_len: usize,
    #[cfg(feature = "std")]
    reader_projection: Option<ReaderProtocolProjection>,
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
    /// Plain data projected from the retained reader graph for this slot.
    #[cfg(feature = "std")]
    pub fn reader_projection(&self) -> Option<&ReaderProtocolProjection> {
        self.reader_projection.as_ref()
    }
}

/// A protocol slot's reader-derived field and supplied local slot.
#[cfg(feature = "std")]
pub struct ReaderProtocolProjection {
    pub(crate) slot: usize,
    pub(crate) field: ReaderProtocolField,
    pub(crate) optional: bool,
    pub(crate) derefed: bool,
    pub(crate) value_type: quire_exact::ValueType,
}

/// The admitted field projection selected by a reader-backed protocol slot.
///
/// This is throwaway: it exists only until the reader returns canonical
/// `NodeRead`s (FR-390-AC-16, AC-50), after which the evaluator matches the
/// one `NodeKind` enum and this type is deleted.
#[cfg(feature = "std")]
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReaderProtocolField {
    /// A direct selected reference to one of the supplied frame slots.
    Local,
    /// A field on an admitted model object, with its exact effective owner.
    Attribute {
        /// The selected model object's effective identity.
        owner: quire_exact::EffectiveId,
        /// The field name admitted from the checked operation member.
        name: String,
    },
    /// A field on an admitted record value, with its exact selected declaration.
    RecordField {
        /// The record declaration identity joined from the selected identity row.
        declaration: quire_exact::NodeKey,
        /// The field name admitted from the checked operation member.
        name: String,
    },
}

#[cfg(feature = "std")]
impl ReaderProtocolProjection {
    /// The field operation admitted by the reader.
    pub fn field(&self) -> &ReaderProtocolField {
        &self.field
    }

    /// Whether the admitted field member type is optional.
    pub fn optional(&self) -> bool {
        self.optional
    }

    /// Whether the admitted operation retained an explicit dereference.
    pub fn derefed(&self) -> bool {
        self.derefed
    }

    /// The selected expression result type projected from the admitted graph.
    pub fn value_type(&self) -> &quire_exact::ValueType {
        &self.value_type
    }

    /// The selected expression's supplied frame slot admitted by the reader.
    pub fn slot(&self) -> usize {
        self.slot
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
/// ```compile_fail,E0277
/// use qsl_package::AdmittedProtocolObservation;
/// use qsl_semantics::model::observation::Observation;
/// fn forge(observation: Observation) -> AdmittedProtocolObservation<'static> {
///     observation.into()
/// }
/// ```
///
/// Its environment is read-only through the public accessors:
///
/// ```compile_fail,E0596
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

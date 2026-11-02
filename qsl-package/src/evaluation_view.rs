// SPDX-License-Identifier: AGPL-3.0-or-later
//! The in-process S4 evaluation view and the FR-106 authority it admits.

use alloc::collections::BTreeMap;
#[cfg(feature = "std")]
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::DomainPackageRef;
use qsl_semantics::model::intake::PackageInput;
use qsl_semantics::model::observation::{
    AdmissionFailure, AdmissionUsage, DocumentRef, Observation, ObservationLimits,
    admit_current_snapshot,
};
#[cfg(feature = "std")]
use quire_exact::OptionValue;
use quire_exact::{NodeKey, ValueType};
use quire_semantic_value::admission::{AdmissionRecord, ObservationAdmission};
use quire_semantic_value::catalog::CatalogCode;
use quire_semantic_value::catalog::InternalFault;
#[cfg(feature = "std")]
use quire_semantic_value::declaration::FieldRef;
use quire_semantic_value::location::Location;

#[cfg(feature = "std")]
use quire_checked::{NodeKind, node::RecordSlot};
#[cfg(feature = "std")]
use quire_contract_model::{CheckedLiteralValue, CheckedTerm};
#[cfg(feature = "std")]
use quire_exact::{Integer, Rational, Value};

use crate::CheckedPackage;
use qsl_semantics::check::CheckedNode;
use quire_checked::node::NodeRead;

#[cfg(feature = "std")]
use crate::V2Read;

#[cfg(feature = "std")]
use quire_contract_model::SelectedExactIdentities;

/// Plain selection data presented to the layer-4 view. It identifies a
/// protocol member but carries no node or evaluation authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolSlotDescriptor {
    /// The selected protocol node identity.
    pub node: NodeKey,
    /// The exact protocol member name.
    pub member: String,
    /// The list position for an indexed member.
    pub index: Option<usize>,
}

/// One admitted binder source selected by the view for a protocol expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolSource {
    binder: NodeKey,
    slot: usize,
    level: usize,
    value_type: ValueType,
    location: Location,
}

impl ProtocolSource {
    /// The exact checked identity of this source binder.
    pub fn binder(&self) -> &NodeKey {
        &self.binder
    }

    /// The expression frame slot used by this binder.
    pub fn slot(&self) -> usize {
        self.slot
    }

    /// The binder's canonical source level.
    pub fn level(&self) -> usize {
        self.level
    }

    /// The binder's checked static type.
    pub fn value_type(&self) -> &ValueType {
        &self.value_type
    }

    /// The selected expression's checked source location.
    pub fn location(&self) -> &Location {
        &self.location
    }
}

/// The exact selected protocol expression and its admitted input sources.
pub struct SelectedProtocolTerm<'view> {
    root: TermHandle<'view>,
    frame_slots: usize,
    sources: Vec<ProtocolSource>,
}

/// One preflighted callable body and its checked frame size.
pub struct CallableTerm<'view> {
    body: TermHandle<'view>,
    slots: usize,
}

impl<'view> CallableTerm<'view> {
    /// The owner-bound callable body handle.
    pub fn body(&self) -> &TermHandle<'view> {
        &self.body
    }

    /// The number of declared parameter slots in the checked body.
    pub fn slots(&self) -> usize {
        self.slots
    }
}

impl SelectedProtocolTerm<'_> {
    /// The owner-bound root expression handle.
    pub fn root(&self) -> &TermHandle<'_> {
        &self.root
    }

    /// Number of local slots in the checked expression frame.
    pub fn frame_slots(&self) -> usize {
        self.frame_slots
    }

    /// The selected expression's source binders in canonical order.
    pub fn sources(&self) -> &[ProtocolSource] {
        &self.sources
    }
}

#[derive(Clone)]
enum EvaluationSource<'package> {
    InProcess(&'package CheckedPackage),
    #[cfg(feature = "std")]
    Reader {
        read: &'package V2Read,
        identities: SelectedIdentityEvidence,
    },
}

/// QSL-owned authority wrapper for exact identities emitted by the model
/// admission stage. Its constructor stays crate-private until QSL-1377 wires
/// the selected-document, declaration, and field rows.
#[cfg(feature = "std")]
#[derive(Clone)]
pub struct SelectedIdentityEvidence {
    identities: SelectedExactIdentities,
    authoritative: bool,
}

#[cfg(feature = "std")]
impl SelectedIdentityEvidence {
    fn empty() -> Self {
        Self {
            identities: SelectedExactIdentities::empty(),
            authoritative: false,
        }
    }

    /// Retain the exact identity table assembled from QSL's admitted stage
    /// rows. The constructor is private to layer 4 so callers cannot assert
    /// identity joins from decoded terms or structural keys.
    fn selected(identities: SelectedExactIdentities) -> Self {
        Self {
            identities,
            authoritative: true,
        }
    }
}

#[cfg(feature = "std")]
#[derive(Clone)]
struct ReaderNode {
    operation: NodeKind<Vec<usize>, Value>,
    value_type: ValueType,
    location: Location,
}

#[cfg(feature = "std")]
#[derive(Clone)]
struct ProvisionalReaderCallable {
    body: quire_contract_model::CheckedNodeId,
    slots: usize,
}

/// An S4 view tied to one successfully linked package or successful I2 read.
#[derive(Clone)]
pub struct AdmittedEvaluationView<'package> {
    source: EvaluationSource<'package>,
    #[cfg(feature = "std")]
    reader_nodes: BTreeMap<
        (quire_contract_model::CheckedNodeId, Vec<usize>),
        Result<ReaderNode, crate::protocol_slot_select::ViewFailure>,
    >,
    #[cfg(feature = "std")]
    // IR-1210 classifies this graph-derived join as insufficient. Keep it
    // isolated as local prep and replace it with IR-1213's admitted read.
    provisional_reader_callables: BTreeMap<usize, ProvisionalReaderCallable>,
}

impl<'package> AdmittedEvaluationView<'package> {
    pub(crate) fn in_process(package: &'package CheckedPackage) -> Self {
        Self {
            source: EvaluationSource::InProcess(package),
            #[cfg(feature = "std")]
            reader_nodes: BTreeMap::new(),
            #[cfg(feature = "std")]
            provisional_reader_callables: BTreeMap::new(),
        }
    }

    #[cfg(feature = "std")]
    fn reader(
        read: &'package V2Read,
        evidence: SelectedIdentityEvidence,
    ) -> Result<Self, crate::protocol_slot_select::ViewFailure> {
        let (reader_nodes, provisional_reader_callables) =
            reader_projection(read, &evidence.identities)?;
        let view = Self {
            source: EvaluationSource::Reader {
                read,
                identities: evidence,
            },
            reader_nodes,
            provisional_reader_callables,
        };
        // Exercise every successful cached descriptor through the same public
        // NodeRead surface before this view can mint a reader handle.
        for (node, path) in view.reader_nodes.keys() {
            let handle = view.handle_reader_node(node.clone(), path.clone());
            view.read_node(&handle)
                .map_err(crate::protocol_slot_select::ViewFailure::Fault)?;
        }
        Ok(view)
    }

    fn in_process_package(&self) -> Option<&'package CheckedPackage> {
        match &self.source {
            EvaluationSource::InProcess(package) => Some(package),
            #[cfg(feature = "std")]
            EvaluationSource::Reader { .. } => None,
        }
    }

    /// Select one protocol expression and mint its opaque term handle only
    /// after the exact S4 member and binder identities have been validated.
    pub fn select_protocol_slot(
        &self,
        descriptor: &ProtocolSlotDescriptor,
    ) -> Result<SelectedProtocolTerm<'_>, crate::protocol_slot_select::ViewFailure> {
        let selected = match &self.source {
            EvaluationSource::InProcess(package) => {
                crate::protocol_slot_select::select(package, descriptor)?
            }
            #[cfg(feature = "std")]
            EvaluationSource::Reader { read, identities } => {
                if !identities.authoritative {
                    return Err(crate::protocol_slot_select::ViewFailure::Refused {
                        code: "invalid_model_binding",
                        cause: "missing-identity-witness",
                        locus: crate::protocol_slot_select::ViewLocus::Node(
                            descriptor.node.clone(),
                        ),
                    });
                }
                if !read.node_keys.values().any(|key| key == &descriptor.node) {
                    return Err(crate::protocol_slot_select::ViewFailure::Refused {
                        code: "missing_declaration",
                        cause: "missing-name",
                        locus: crate::protocol_slot_select::ViewLocus::Node(
                            descriptor.node.clone(),
                        ),
                    });
                }
                let (root_node, root_path, expression) =
                    select_reader_expression(read, descriptor)?;
                let location = reader_expression_location(read, &root_node, &descriptor.node)?;
                let mut binders = BTreeMap::new();
                collect_reader_binders(
                    read,
                    &expression,
                    &descriptor.node,
                    &mut binders,
                    &mut BTreeMap::new(),
                )?;
                let mut frame_slots = 0;
                let mut sources = Vec::with_capacity(binders.len());
                for (level, (binder_id, binder)) in binders {
                    let key = read.node_keys.get(&binder_id).cloned().ok_or_else(|| {
                        crate::protocol_slot_select::ViewFailure::Refused {
                            code: "invalid_model_binding",
                            cause: "missing-identity-witness",
                            locus: crate::protocol_slot_select::ViewLocus::Node(
                                descriptor.node.clone(),
                            ),
                        }
                    })?;
                    let projection = read
                        .admitted
                        .project_exact_type(&binder.semantic_type, &identities.identities, 10_000)
                        .map_err(|_| crate::protocol_slot_select::ViewFailure::Refused {
                            code: "invalid_model_binding",
                            cause: "missing-identity-witness",
                            locus: crate::protocol_slot_select::ViewLocus::Node(key.clone()),
                        })?;
                    frame_slots = frame_slots.max(level.saturating_add(1));
                    sources.push(ProtocolSource {
                        binder: key,
                        slot: level,
                        level,
                        value_type: projection.value_type().clone(),
                        location: location.clone(),
                    });
                }
                let root = self.handle_reader_node(root_node, root_path);
                return Ok(SelectedProtocolTerm {
                    root,
                    frame_slots,
                    sources,
                });
            }
        };
        let location = selected.body.root().location().clone();
        let root = self.handle_checked_node(selected.body.root());
        let frame_slots = selected.frame.len();
        let sources = selected
            .frame
            .into_iter()
            .enumerate()
            .filter(|(slot, _)| selected.body.reads_local(*slot))
            .filter_map(|(slot, source)| {
                source.node.map(|binder| ProtocolSource {
                    binder,
                    slot,
                    level: source.level,
                    value_type: source.value_type,
                    location: location.clone(),
                })
            })
            .collect();
        Ok(SelectedProtocolTerm {
            root,
            frame_slots,
            sources,
        })
    }

    /// Resolve a callable body reached from this view's selected term.
    /// Reader bodies are exposed only after their full typed closure was
    /// preflighted into this view.
    pub fn callable_term(
        &self,
        function: usize,
    ) -> Result<CallableTerm<'_>, crate::protocol_slot_select::ViewFailure> {
        let (body, slots) = match &self.source {
            EvaluationSource::InProcess(package) => {
                let callable = package.graph().function_state(function).ok_or_else(|| {
                    crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                        "FR-390",
                        "checked-callable-index-resolves",
                    ))
                })?;
                (TermNode::InProcess(callable.body), callable.slots)
            }
            #[cfg(feature = "std")]
            EvaluationSource::Reader { read, .. } => {
                let callable_nodes = read
                    .admitted
                    .graph()
                    .nodes
                    .iter()
                    .filter(|candidate| candidate.node_tag.as_ref() == "function")
                    .collect::<Vec<_>>();
                let Some(function_node) = callable_nodes.get(function) else {
                    return Err(crate::protocol_slot_select::ViewFailure::Fault(
                        InternalFault::new("FR-390", "reader-callable-index-resolves"),
                    ));
                };
                let node_key = read
                    .node_keys
                    .get(&function_node.node_id)
                    .cloned()
                    .ok_or_else(|| {
                        crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                            "FR-390",
                            "reader-callable-has-structural-key",
                        ))
                    })?;
                let callable = self
                    .provisional_reader_callables
                    .get(&function)
                    .ok_or_else(|| {
                        crate::protocol_slot_select::ViewFailure::Unsupported {
                            node: node_key,
                            cause: "reader-callable-body-not-preflighted",
                        }
                    })?;
                (
                    TermNode::Reader {
                        node: callable.body.clone(),
                        path: Vec::new(),
                    },
                    callable.slots,
                )
            }
        };
        Ok(CallableTerm {
            body: TermHandle {
                owner: self,
                node: body,
            },
            slots,
        })
    }

    /// Mint an opaque handle for a node in the selected checked body.
    ///
    /// This constructor is crate-local so protocol-slot selection remains the
    /// only public route to an evaluable expression handle.
    pub(crate) fn handle_checked_node<'view>(
        &'view self,
        node: CheckedNode<'package>,
    ) -> TermHandle<'view> {
        TermHandle {
            owner: self,
            node: TermNode::InProcess(node),
        }
    }

    #[cfg(feature = "std")]
    fn handle_reader_node<'view>(
        &'view self,
        node: quire_contract_model::CheckedNodeId,
        path: Vec<usize>,
    ) -> TermHandle<'view> {
        TermHandle {
            owner: self,
            node: TermNode::Reader { node, path },
        }
    }

    /// Read one checked node in this view's canonical operation vocabulary.
    /// A handle from another view is an invariant fault.
    pub fn read_node<'view>(
        &'view self,
        handle: &TermHandle<'view>,
    ) -> Result<NodeRead<'view, TermHandle<'view>>, InternalFault> {
        if !core::ptr::eq(self, handle.owner) {
            return Err(InternalFault::new("FR-390", "foreign-term-handle"));
        }
        let node = match &handle.node {
            TermNode::InProcess(node) => *node,
            #[cfg(feature = "std")]
            TermNode::Reader { node, path } => {
                let projected = self
                    .reader_nodes
                    .get(&(node.clone(), path.clone()))
                    .ok_or_else(|| InternalFault::new("FR-390", "reader-node-was-preflighted"))?
                    .as_ref()
                    .map_err(|_| {
                        InternalFault::new("FR-390", "reader-node-projection-was-admitted")
                    })?;
                let operation = projected.operation.map_ref(
                    |child_path| TermHandle {
                        owner: self,
                        node: TermNode::Reader {
                            node: node.clone(),
                            path: child_path.clone(),
                        },
                    },
                    |literal| literal,
                );
                return Ok(NodeRead {
                    operation,
                    value_type: &projected.value_type,
                    location: &projected.location,
                });
            }
        };
        Ok(NodeRead {
            operation: node.kind().map_ref(
                |id| self.handle_checked_node(node.at(*id)),
                |literal| &literal.0,
            ),
            value_type: node.value_type(),
            location: node.location(),
        })
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
        let Some(package) = self.in_process_package() else {
            return ObservationAdmission::Fault(quire_semantic_value::catalog::InternalFault::new(
                "FR-390",
                "reader-observation-admission-requires-reader-model-projection",
            ));
        };
        if !package.graph().model_selections().contains(model) {
            return ObservationAdmission::Refused(AdmissionRecord {
                catalog: CatalogCode::new("invalid_model_binding", "wrong-model-selection"),
                fields: BTreeMap::new(),
                path: Vec::new(),
                kernel: None,
            });
        }

        match admit_current_snapshot(
            core::slice::from_ref(model),
            package.graph().scope().types(),
            packages,
            model_limits,
            snapshots,
            selected,
            limits,
        ) {
            Ok((environment, usage)) => {
                let populations = environment.population_completeness().clone();
                ObservationAdmission::Admitted(AdmittedProtocolObservation {
                    package,
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
}

#[cfg(feature = "std")]
impl V2Read {
    /// Create a read-only evaluation view tied to this successful I2 read.
    ///
    /// QSL's selected identity rows are required before the reader can
    /// establish protocol and binding identity. Until that evidence is
    /// attached by the layer-4 admission path, return the typed refusal rather
    /// than expose a partial view.
    pub fn evaluation_view(
        &self,
    ) -> Result<AdmittedEvaluationView<'_>, crate::protocol_slot_select::ViewFailure> {
        let Some(node) = self.node_keys.values().next().cloned() else {
            return Err(crate::protocol_slot_select::ViewFailure::Fault(
                InternalFault::new("FR-390", "admitted-reader-has-package-identity-locus"),
            ));
        };
        Err(crate::protocol_slot_select::ViewFailure::Refused {
            code: "invalid_model_binding",
            cause: "missing-identity-witness",
            locus: crate::protocol_slot_select::ViewLocus::Node(node),
        })
    }

    /// Layer-4-only constructor used once the selected stage rows have been
    /// validated and retained by the owning model admission path.
    pub(crate) fn evaluation_view_with_evidence(
        &self,
        evidence: SelectedIdentityEvidence,
    ) -> Result<AdmittedEvaluationView<'_>, crate::protocol_slot_select::ViewFailure> {
        AdmittedEvaluationView::reader(self, evidence)
    }
}

#[cfg(feature = "std")]
fn reader_projection(
    read: &V2Read,
    identities: &SelectedExactIdentities,
) -> Result<
    (
        BTreeMap<
            (quire_contract_model::CheckedNodeId, Vec<usize>),
            Result<ReaderNode, crate::protocol_slot_select::ViewFailure>,
        >,
        BTreeMap<usize, ProvisionalReaderCallable>,
    ),
    crate::protocol_slot_select::ViewFailure,
> {
    let mut projected = BTreeMap::new();
    let mut callable_indices = BTreeSet::new();
    for descriptor in reader_protocol_descriptors(read) {
        let (root, root_path, term) =
            select_reader_expression(read, &descriptor).map_err(|failure| failure)?;
        let key = read.node_keys.get(&root).cloned().ok_or_else(|| {
            crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                "FR-390",
                "reader-expression-root-has-structural-key",
            ))
        })?;
        let location = reader_expression_location(read, &root, &descriptor.node)?;
        let mut pending = vec![(
            term,
            root_path,
            BTreeMap::<quire_contract_model::CheckedNodeId, ()>::new(),
        )];
        while let Some((term, path, mut visited)) = pending.pop() {
            let cache_key = (root.clone(), path.clone());
            if projected.contains_key(&cache_key) {
                continue;
            }
            let (effective, effective_node) = reader_expression_term(read, &term)?;
            if let Some(target) = effective_node.as_ref() {
                if visited.insert(target.clone(), ()).is_some() {
                    return Err(crate::protocol_slot_select::ViewFailure::Fault(
                        InternalFault::new("FR-390", "reader-expression-closure-is-acyclic"),
                    ));
                }
            }
            let owner_key = effective_node
                .as_ref()
                .and_then(|id| read.node_keys.get(id))
                .unwrap_or(&key);
            let result = project_reader_term(
                read,
                identities,
                owner_key,
                &effective,
                &path,
                Location::at(location.origin.clone(), &path),
            );
            let node = result?;
            if let NodeKind::Call { function, .. } = &node.operation {
                callable_indices.insert(*function);
            }
            let children = reader_expression_children(read, &effective);
            projected.insert(cache_key, Ok(node));
            for (position, child) in children.into_iter().rev() {
                let mut child_path = path.clone();
                child_path.push(position);
                pending.push((child, child_path, visited.clone()));
            }
        }
    }
    // IR-1210 row 2: graph fragments do not establish the admitted callable,
    // frame, and body join. Replace this whole index when IR-1213 lands.
    let mut provisional_reader_callables = BTreeMap::new();
    while let Some(function) = callable_indices
        .iter()
        .find(|function| !provisional_reader_callables.contains_key(*function))
        .copied()
    {
        let (root, path, term, slots, location) =
            provisional_reader_callable_source(read, function)?;
        let root_key = read.node_keys.get(&root).cloned().ok_or_else(|| {
            crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                "FR-390",
                "reader-callable-root-has-structural-key",
            ))
        })?;
        let mut pending = vec![(
            term,
            path,
            BTreeMap::<quire_contract_model::CheckedNodeId, ()>::new(),
        )];
        while let Some((term, path, mut visited)) = pending.pop() {
            let cache_key = (root.clone(), path.clone());
            if projected.contains_key(&cache_key) {
                continue;
            }
            let (effective, effective_node) = reader_expression_term(read, &term)?;
            if let Some(target) = effective_node.as_ref() {
                if visited.insert(target.clone(), ()).is_some() {
                    return Err(crate::protocol_slot_select::ViewFailure::Fault(
                        InternalFault::new("FR-390", "reader-function-closure-is-acyclic"),
                    ));
                }
            }
            let owner_key = effective_node
                .as_ref()
                .and_then(|id| read.node_keys.get(id))
                .unwrap_or(&root_key);
            let node = project_reader_term(
                read,
                identities,
                owner_key,
                &effective,
                &path,
                Location::at(location.origin.clone(), &path),
            )?;
            if let NodeKind::Call { function, .. } = &node.operation {
                callable_indices.insert(*function);
            }
            let children = reader_expression_children(read, &effective);
            projected.insert(cache_key, Ok(node));
            for (position, child) in children.into_iter().rev() {
                let mut child_path = path.clone();
                child_path.push(position);
                pending.push((child, child_path, visited.clone()));
            }
        }
        provisional_reader_callables.insert(
            function,
            ProvisionalReaderCallable { body: root, slots },
        );
    }
    Ok((projected, provisional_reader_callables))
}

#[cfg(feature = "std")]
fn provisional_reader_callable_source(
    read: &V2Read,
    function: usize,
) -> Result<
    (
        quire_contract_model::CheckedNodeId,
        Vec<usize>,
        CheckedTerm,
        usize,
        Location,
    ),
    crate::protocol_slot_select::ViewFailure,
> {
    use crate::protocol_slot_select::ViewFailure;
    let function_node = read
        .admitted
        .graph()
        .nodes
        .iter()
        .filter(|candidate| candidate.node_tag.as_ref() == "function")
        .nth(function)
        .ok_or_else(|| {
            ViewFailure::Fault(InternalFault::new(
                "FR-390",
                "reader-function-index-resolves",
            ))
        })?;
    let location_node = read
        .node_keys
        .get(&function_node.node_id)
        .cloned()
        .ok_or_else(|| {
            ViewFailure::Fault(InternalFault::new(
                "FR-390",
                "reader-function-has-structural-key",
            ))
        })?;
    let name = function_node
        .declaration
        .as_ref()
        .and_then(|declaration| declaration.qualified_name.last())
        .map(|segment| segment.to_string())
        .ok_or_else(|| ViewFailure::Unsupported {
            node: location_node.clone(),
            cause: "reader-callable-declaration-location-not-retained",
        })?;
    let expression = named_member(&function_node.body, "body")
        .and_then(|(_, body)| match body {
            CheckedTerm::Binding { value, .. } => match &**value {
                CheckedTerm::Reference { target } => Some(target.clone()),
                _ => None,
            },
            CheckedTerm::Reference { target } => Some(target.clone()),
            _ => None,
        })
        .ok_or_else(|| ViewFailure::Unsupported {
            node: location_node.clone(),
            cause: "reader-callable-body-binding-not-retained",
        })?;
    let (term, body_node) = reader_expression_term(
        read,
        &CheckedTerm::Reference {
            target: expression.clone(),
        },
    )?;
    let slots = binding_len(&function_node.body, "parameters");
    let expression_node = body_node.ok_or_else(|| {
        ViewFailure::Fault(InternalFault::new(
            "FR-390",
            "reader-function-body-is-admitted-expression",
        ))
    })?;
    let _ = reader_expression_location(read, &expression_node, &location_node)?;
    let location_index =
        provisional_reader_function_location_index(read, &function_node.node_id)?;
    Ok((
        expression,
        Vec::new(),
        term,
        slots,
        Location::root(quire_semantic_value::location::Origin::Body {
            function: name,
            index: location_index,
        }),
    ))
}

#[cfg(feature = "std")]
fn provisional_reader_function_location_index(
    read: &V2Read,
    selected: &quire_contract_model::CheckedNodeId,
) -> Result<usize, crate::protocol_slot_select::ViewFailure> {
    let mut locations = Vec::new();
    for node in read
        .admitted
        .graph()
        .nodes
        .iter()
        .filter(|candidate| candidate.node_tag.as_ref() == "function")
    {
        let key = read.node_keys.get(&node.node_id).ok_or_else(|| {
            crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                "FR-390",
                "reader-function-has-structural-key",
            ))
        })?;
        let Some((_, regions)) = read.source_map.occurrences(key).find(|(origin, regions)| {
            origin.origin().role().as_str() == "declaration" && !regions.is_empty()
        }) else {
            return Err(crate::protocol_slot_select::ViewFailure::Unsupported {
                node: key.clone(),
                cause: "reader-function-declaration-location-not-retained",
            });
        };
        let region = &regions[0];
        locations.push((
            region.source().authority().to_owned(),
            region.source().identity().to_owned(),
            region.start(),
            node.node_id.clone(),
        ));
    }
    locations.sort();
    locations
        .iter()
        .position(|(_, _, _, node)| node == selected)
        .ok_or_else(|| {
            match read.node_keys.get(selected).cloned() {
                Some(node) => crate::protocol_slot_select::ViewFailure::Unsupported {
                    node,
                    cause: "reader-function-declaration-order-not-retained",
                },
                None => crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                    "FR-390",
                    "reader-function-has-structural-key",
                )),
            }
        })
}

#[cfg(feature = "std")]
fn reader_protocol_descriptors(read: &V2Read) -> Vec<ProtocolSlotDescriptor> {
    let mut descriptors = Vec::new();
    for node in &read.admitted.graph().nodes {
        if node.node_tag.as_ref() != "protocol" {
            continue;
        }
        let Some(key) = read.node_keys.get(&node.node_id) else {
            continue;
        };
        let qname = node
            .declaration
            .as_ref()
            .map(|declaration| {
                declaration
                    .qualified_name
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut add = |member: &str, index: Option<usize>| {
            let descriptor = ProtocolSlotDescriptor {
                node: key.clone(),
                member: member.into(),
                index,
            };
            if select_reader_expression(read, &descriptor).is_ok() {
                descriptors.push(descriptor);
            }
        };
        match qname.as_slice() {
            [_] => {
                add("activation.guard", None);
                for index in 0..binding_len(&node.body, "captures") {
                    add("captures", Some(index));
                }
            }
            [_, "role", _] => add("replication.until", None),
            [_, "channel", _] => add("ordering.key", None),
            [_, "compensate", _] => {
                for index in 0..binding_len(&node.body, "registration_captures") {
                    add("registration_captures", Some(index));
                }
                add("trigger.guard", None);
                for index in 0..binding_len(&node.body, "activation_captures") {
                    add("activation_captures", Some(index));
                }
                add("retry.relation", None);
                add("recovery.predicate", None);
            }
            [_, "finish", _] => add("constraint", None),
            [_, "terminal"] => add("condition", None),
            [_, "run", ..] => {
                add("guard", None);
                add("condition", None);
                add("constraint", None);
                for index in 0..binding_len(&node.body, "related") {
                    add("related.from", Some(index));
                    add("related.to", Some(index));
                }
                for index in 0..binding_len(&node.body, "visible") {
                    add("visible", Some(index));
                }
                for index in 0..binding_len(&node.body, "cases") {
                    add("cases.guard", Some(index));
                }
            }
            _ => {}
        }
    }
    descriptors.sort_by(|left, right| {
        (&left.node, &left.member, left.index).cmp(&(&right.node, &right.member, right.index))
    });
    descriptors
}

#[cfg(feature = "std")]
fn binding_len(term: &CheckedTerm, name: &str) -> usize {
    named_member(term, name)
        .and_then(|(_, value)| match value {
            CheckedTerm::Aggregate { members } => Some(members.len()),
            _ => None,
        })
        .unwrap_or(0)
}

#[cfg(feature = "std")]
fn reader_expression_term(
    read: &V2Read,
    term: &CheckedTerm,
) -> Result<
    (CheckedTerm, Option<quire_contract_model::CheckedNodeId>),
    crate::protocol_slot_select::ViewFailure,
> {
    if let CheckedTerm::Binding { value, .. } = term {
        return Ok(((**value).clone(), None));
    }
    if let CheckedTerm::Reference { target } = term {
        let node = read
            .admitted
            .graph()
            .nodes
            .iter()
            .find(|node| node.node_id == *target)
            .ok_or_else(|| {
                crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                    "FR-390",
                    "admitted-expression-reference-resolves",
                ))
            })?;
        if node.node_tag.as_ref() == "expression" || node.node_tag.as_ref() == "value" {
            return Ok((node.body.clone(), Some(target.clone())));
        }
    }
    Ok((term.clone(), None))
}

#[cfg(feature = "std")]
fn reader_expression_children(_read: &V2Read, term: &CheckedTerm) -> Vec<(usize, CheckedTerm)> {
    let CheckedTerm::Application {
        operation,
        arguments,
        ..
    } = term
    else {
        return match term {
            CheckedTerm::Aggregate { members } => members
                .iter()
                .enumerate()
                .map(|(index, term)| (index, term.clone()))
                .collect(),
            _ => Vec::new(),
        };
    };
    if operation.identity() == Some("quire.op.record.project") {
        let member = operation.read().ok().and_then(|read| read.member.cloned());
        if matches!(
            member,
            Some(quire_contract_model::CheckedOperationMember::Field {
                read_form: Some(_),
                ..
            })
        ) {
            if let Some(CheckedTerm::Reference { target }) = arguments.first() {
                if let Some(CheckedTerm::Application {
                    operation,
                    arguments,
                    ..
                }) = _read
                    .admitted
                    .graph()
                    .nodes
                    .iter()
                    .find(|node| node.node_id == *target)
                    .map(|node| &node.body)
                {
                    if operation.identity() == Some("quire.op.model.deref") {
                        return arguments
                            .first()
                            .map(|term| vec![(0, term.clone())])
                            .unwrap_or_default();
                    }
                }
            }
        }
    }
    if operation.identity() == Some("quire.op.function.call") {
        return arguments
            .iter()
            .enumerate()
            .skip(1)
            .map(|(index, term)| (index, term.clone()))
            .collect();
    }
    arguments
        .iter()
        .enumerate()
        .map(|(index, term)| (index, term.clone()))
        .collect()
}

#[cfg(feature = "std")]
fn select_reader_expression(
    read: &V2Read,
    descriptor: &ProtocolSlotDescriptor,
) -> Result<
    (quire_contract_model::CheckedNodeId, Vec<usize>, CheckedTerm),
    crate::protocol_slot_select::ViewFailure,
> {
    use crate::protocol_slot_select::{ViewFailure, ViewLocus};
    let node = read
        .node_keys
        .iter()
        .find_map(|(id, key)| (key == &descriptor.node).then_some(id))
        .and_then(|id| {
            read.admitted
                .graph()
                .nodes
                .iter()
                .find(|node| &node.node_id == id)
        })
        .ok_or_else(|| ViewFailure::Refused {
            code: "missing_declaration",
            cause: "missing-name",
            locus: ViewLocus::Node(descriptor.node.clone()),
        })?;
    if node.node_tag.as_ref() != "protocol" {
        return Err(ViewFailure::Refused {
            code: "invalid_model_binding",
            cause: "malformed-declaration",
            locus: ViewLocus::Node(descriptor.node.clone()),
        });
    }
    let qname = node
        .declaration
        .as_ref()
        .map(|declaration| {
            declaration
                .qualified_name
                .iter()
                .map(AsRef::as_ref)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut root_path = Vec::new();
    let root = &node.body;
    let selected = match (qname.as_slice(), descriptor.member.as_str()) {
        ([_], "activation.guard") => named_path(root, &mut root_path, &["activation", "guard"]),
        ([_], "captures") => indexed_nested(
            root,
            &mut root_path,
            "captures",
            descriptor.index,
            "initializer",
        ),
        ([_, "terminal"], "condition") => named_path(root, &mut root_path, &["condition"]),
        ([_, "finish", _], "constraint") => named_path(root, &mut root_path, &["constraint"]),
        ([_, "role", _], "replication.until") => {
            named_path(root, &mut root_path, &["replication", "until"])
        }
        ([_, "channel", _], "ordering.key") => {
            named_path(root, &mut root_path, &["ordering", "key"])
        }
        ([_, "compensate", _], "registration_captures") => indexed_nested(
            root,
            &mut root_path,
            "registration_captures",
            descriptor.index,
            "initializer",
        ),
        ([_, "compensate", _], "trigger.guard") => {
            named_path(root, &mut root_path, &["trigger", "guard"])
        }
        ([_, "compensate", _], "activation_captures") => indexed_nested(
            root,
            &mut root_path,
            "activation_captures",
            descriptor.index,
            "initializer",
        ),
        ([_, "compensate", _], "retry.relation") => {
            named_path(root, &mut root_path, &["retry", "relation"])
        }
        ([_, "compensate", _], "recovery.predicate") => {
            named_path(root, &mut root_path, &["recovery", "predicate"])
        }
        ([_, "run", ..], "cases.guard") => {
            indexed_nested(root, &mut root_path, "cases", descriptor.index, "guard")
        }
        ([_, "run", ..], "visible") => {
            indexed_member(root, &mut root_path, "visible", descriptor.index)
        }
        ([_, "run", ..], "guard") => named_path(root, &mut root_path, &["guard"]),
        ([_, "run", ..], "condition") => named_path(root, &mut root_path, &["condition"]),
        ([_, "run", ..], "related.from") => {
            indexed_nested(root, &mut root_path, "related", descriptor.index, "from")
        }
        ([_, "run", ..], "related.to") => {
            indexed_nested(root, &mut root_path, "related", descriptor.index, "to")
        }
        ([_, "run", ..], "constraint") => named_path(root, &mut root_path, &["constraint"]),
        _ => None,
    }
    .ok_or_else(|| ViewFailure::Refused {
        code: "missing_declaration",
        cause: "missing-name",
        locus: ViewLocus::Member {
            name: descriptor.member.clone(),
            index: descriptor.index,
        },
    })?;
    let selected = if qname.get(1) == Some(&"terminal") && descriptor.member == "condition" {
        match selected {
            CheckedTerm::Aggregate { members } if members.len() == 1 => {
                root_path.push(0);
                &members[0]
            }
            value => value,
        }
    } else {
        selected
    };
    let (owner, path, selected) = match selected {
        CheckedTerm::Reference { target }
            if read
                .admitted
                .graph()
                .nodes
                .iter()
                .find(|candidate| candidate.node_id == *target)
                .is_some_and(|candidate| {
                    candidate.node_tag.as_ref() == "expression"
                        || candidate.node_tag.as_ref() == "value"
                }) =>
        {
            let target_node = read
                .admitted
                .graph()
                .nodes
                .iter()
                .find(|candidate| candidate.node_id == *target)
                .ok_or_else(|| {
                    ViewFailure::Fault(InternalFault::new(
                        "FR-390",
                        "admitted-expression-reference-resolves",
                    ))
                })?;
            (target.clone(), Vec::new(), target_node.body.clone())
        }
        term => (node.node_id.clone(), root_path, term.clone()),
    };
    Ok((owner, path, selected.clone()))
}

#[cfg(feature = "std")]
fn named_path<'a>(
    mut term: &'a CheckedTerm,
    path: &mut Vec<usize>,
    names: &[&str],
) -> Option<&'a CheckedTerm> {
    for name in names {
        let (index, value) = named_member(term, name)?;
        path.push(index);
        term = match value {
            CheckedTerm::Binding { value, .. } => {
                path.push(0);
                value
            }
            value => value,
        };
    }
    Some(term)
}

#[cfg(feature = "std")]
fn indexed_member<'a>(
    term: &'a CheckedTerm,
    path: &mut Vec<usize>,
    name: &str,
    index: Option<usize>,
) -> Option<&'a CheckedTerm> {
    let (position, value) = named_member(term, name)?;
    path.push(position);
    let value = match value {
        CheckedTerm::Binding { value, .. } => {
            path.push(0);
            &**value
        }
        value => value,
    };
    let CheckedTerm::Aggregate { members } = value else {
        return None;
    };
    let index = index?;
    path.push(index);
    members.get(index).map(|member| match member {
        CheckedTerm::Binding { value, .. } => {
            path.push(0);
            &**value
        }
        value => value,
    })
}

#[cfg(feature = "std")]
fn indexed_nested<'a>(
    term: &'a CheckedTerm,
    path: &mut Vec<usize>,
    name: &str,
    index: Option<usize>,
    nested: &str,
) -> Option<&'a CheckedTerm> {
    let item = indexed_member(term, path, name, index)?;
    let (position, value) = named_member(item, nested)?;
    path.push(position);
    match value {
        CheckedTerm::Binding { value, .. } => {
            path.push(0);
            Some(value)
        }
        value => Some(value),
    }
}

#[cfg(feature = "std")]
fn named_member<'a>(term: &'a CheckedTerm, name: &str) -> Option<(usize, &'a CheckedTerm)> {
    let members = match term {
        CheckedTerm::Application { arguments, .. } => arguments,
        CheckedTerm::Aggregate { members } => members,
        CheckedTerm::Binding {
            name: binding,
            value,
        } if binding.as_ref() == name => {
            return Some((0, value));
        }
        _ => return None,
    };
    members
        .iter()
        .enumerate()
        .find_map(|(index, member)| match member {
            CheckedTerm::Binding {
                name: binding,
                value,
            } if binding.as_ref() == name => Some((index, &**value)),
            _ => None,
        })
}

#[cfg(feature = "std")]
fn collect_reader_binders(
    read: &V2Read,
    term: &CheckedTerm,
    locus: &NodeKey,
    binders: &mut BTreeMap<
        usize,
        (
            quire_contract_model::CheckedNodeId,
            quire_contract_model::CheckedSemanticNodeV2,
        ),
    >,
    visited: &mut BTreeMap<quire_contract_model::CheckedNodeId, ()>,
) -> Result<(), crate::protocol_slot_select::ViewFailure> {
    match term {
        CheckedTerm::Reference { target } => {
            let Some(node) = read
                .admitted
                .graph()
                .nodes
                .iter()
                .find(|node| node.node_id == *target)
            else {
                return Err(crate::protocol_slot_select::ViewFailure::Fault(
                    InternalFault::new("FR-390", "admitted-reference-target-resolves"),
                ));
            };
            if node.semantic_form.as_ref() == "parameter" {
                let level_term = named_member(&node.body, "level")
                    .map(|(_, term)| term)
                    .ok_or_else(|| crate::protocol_slot_select::ViewFailure::Unsupported {
                        node: locus.clone(),
                        cause: "reader-parameter-level-missing",
                    })?;
                let level = integer_literal(level_term).ok_or_else(|| {
                    crate::protocol_slot_select::ViewFailure::Unsupported {
                        node: locus.clone(),
                        cause: "reader-parameter-level-not-integer",
                    }
                })?;
                if let Some((existing, _)) = binders.get(&level) {
                    if existing != target {
                        return Err(crate::protocol_slot_select::ViewFailure::Refused {
                            code: "invalid_model_binding",
                            cause: "malformed-declaration",
                            locus: crate::protocol_slot_select::ViewLocus::Node(locus.clone()),
                        });
                    }
                } else {
                    binders.insert(level, (target.clone(), node.clone()));
                }
            } else if (node.node_tag.as_ref() == "expression" || node.node_tag.as_ref() == "value")
                && visited.insert(target.clone(), ()).is_none()
            {
                collect_reader_binders(read, &node.body, locus, binders, visited)?;
            }
        }
        CheckedTerm::Application { arguments, .. } => {
            for argument in arguments {
                collect_reader_binders(read, argument, locus, binders, visited)?;
            }
        }
        CheckedTerm::Aggregate { members } => {
            for member in members {
                collect_reader_binders(read, member, locus, binders, visited)?;
            }
        }
        CheckedTerm::Binding { value, .. } => {
            collect_reader_binders(read, value, locus, binders, visited)?
        }
        _ => {}
    }
    Ok(())
}

#[cfg(feature = "std")]
fn integer_literal(term: &CheckedTerm) -> Option<usize> {
    let CheckedTerm::Literal { value, .. } = term else {
        return None;
    };
    value.as_str()?.parse().ok()
}

#[cfg(feature = "std")]
fn reader_expression_location(
    read: &V2Read,
    expression: &quire_contract_model::CheckedNodeId,
    protocol_node: &NodeKey,
) -> Result<Location, crate::protocol_slot_select::ViewFailure> {
    let projection_node = read
        .admitted
        .graph()
        .nodes
        .iter()
        .find(|candidate| &candidate.node_id == expression)
        .ok_or_else(|| {
            crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                "FR-390",
                "reader-expression-location-node-resolves",
            ))
        })?;
    let expression_occurrence = projection_node
        .occurrences
        .iter()
        .find(|occurrence| {
            occurrence.role == quire_contract_model::CheckedOccurrenceRole::Expression
        })
        .ok_or_else(|| crate::protocol_slot_select::ViewFailure::Unsupported {
            node: protocol_node.clone(),
            cause: "missing-expression-occurrence-for-reader-location",
        })?;
    let expression_key = read.node_keys.get(expression).ok_or_else(|| {
        crate::protocol_slot_select::ViewFailure::Refused {
            code: "invalid_model_binding",
            cause: "missing-identity-witness",
            locus: crate::protocol_slot_select::ViewLocus::Node(protocol_node.clone()),
        }
    })?;
    let source_occurrence = read
        .source_map
        .occurrences(expression_key)
        .any(|(key, regions)| {
            key.origin().role().as_str() == "expression"
                && key.origin().ordinal() == expression_occurrence.ordinal
                && !regions.is_empty()
        });
    if !source_occurrence {
        return Err(crate::protocol_slot_select::ViewFailure::Unsupported {
            node: protocol_node.clone(),
            cause: "reader-expression-source-occurrence-not-retained",
        });
    }
    Ok(Location::root(
        quire_semantic_value::location::Origin::Expression,
    ))
}

#[cfg(feature = "std")]
fn project_reader_term(
    read: &V2Read,
    identities: &SelectedExactIdentities,
    node: &NodeKey,
    term: &CheckedTerm,
    path: &[usize],
    location: Location,
) -> Result<ReaderNode, crate::protocol_slot_select::ViewFailure> {
    let unsupported = || crate::protocol_slot_select::ViewFailure::Unsupported {
        node: node.clone(),
        cause: "checked-term-has-no-canonical-node-read-projection",
    };
    let type_node = match term {
        CheckedTerm::Literal { type_id, .. } => type_id,
        CheckedTerm::Application { result_type, .. } => result_type,
        CheckedTerm::Reference { target } => {
            &read
                .admitted
                .graph()
                .nodes
                .iter()
                .find(|candidate| candidate.node_id == *target)
                .ok_or_else(|| {
                    crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                        "FR-390",
                        "admitted-reference-target-resolves",
                    ))
                })?
                .semantic_type
        }
        CheckedTerm::Aggregate { .. } => {
            let source_node = read
                .node_keys
                .iter()
                .find_map(|(id, key)| (key == node).then_some(id))
                .and_then(|id| {
                    read.admitted
                        .graph()
                        .nodes
                        .iter()
                        .find(|candidate| &candidate.node_id == id)
                })
                .ok_or_else(|| {
                    crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                        "FR-390",
                        "reader-value-node-has-admitted-type",
                    ))
                })?;
            &source_node.semantic_type
        }
        _ => return Err(unsupported()),
    };
    let projection = read
        .admitted
        .project_exact_type(type_node, identities, 10_000)
        .map_err(|error| {
            if error.code
                == quire_contract_model::ExactTypeProjectionErrorCode::UnavailableIdentityWitness
            {
                crate::protocol_slot_select::ViewFailure::Refused {
                    code: "invalid_model_binding",
                    cause: "missing-identity-witness",
                    locus: crate::protocol_slot_select::ViewLocus::Node(node.clone()),
                }
            } else {
                unsupported()
            }
        })?;
    let operation = match term {
        CheckedTerm::Literal { .. } => {
            let literal = term.literal_read().ok_or_else(unsupported)?;
            let value = match literal.value {
                CheckedLiteralValue::Boolean(value) => Value::Boolean(value),
                CheckedLiteralValue::Integer(value) => Value::Integer(Integer::from_big(value)),
                CheckedLiteralValue::Rational {
                    numerator,
                    denominator,
                } => {
                    let rational =
                        Rational::new(Integer::from_big(numerator), Integer::from_big(denominator))
                            .map_err(|_| unsupported())?;
                    Value::Rational(rational)
                }
                CheckedLiteralValue::String(value) => Value::String(value.to_owned()),
                CheckedLiteralValue::None => {
                    let ValueType::Option(payload) = projection.value_type() else {
                        return Err(unsupported());
                    };
                    OptionValue::none((**payload).clone())
                }
                _ => return Err(unsupported()),
            };
            NodeKind::Literal(value)
        }
        CheckedTerm::Application {
            operation,
            arguments,
            ..
        } => {
            let child = |index: usize| {
                if index >= arguments.len() {
                    return Err(unsupported());
                }
                let mut child_path = path.to_vec();
                child_path.push(index);
                Ok(child_path)
            };
            match operation.identity().unwrap_or_default() {
                "quire.op.control.if" if arguments.len() == 3 => NodeKind::If {
                    condition: child(0)?,
                    then: child(1)?,
                    otherwise: child(2)?,
                },
                "quire.op.boolean.not" if arguments.len() == 1 => NodeKind::Not(child(0)?),
                "quire.op.boolean.and" if arguments.len() == 2 => {
                    NodeKind::Connective(quire_checked::node::Connective::And, child(0)?, child(1)?)
                }
                "quire.op.boolean.or" if arguments.len() == 2 => {
                    NodeKind::Connective(quire_checked::node::Connective::Or, child(0)?, child(1)?)
                }
                "quire.op.boolean.implies" if arguments.len() == 2 => NodeKind::Connective(
                    quire_checked::node::Connective::Implies,
                    child(0)?,
                    child(1)?,
                ),
                "quire.op.integer.add" if arguments.len() == 2 => {
                    NodeKind::Arithmetic(quire_checked::node::Arithmetic::Add, child(0)?, child(1)?)
                }
                "quire.op.integer.sub" if arguments.len() == 2 => NodeKind::Arithmetic(
                    quire_checked::node::Arithmetic::Subtract,
                    child(0)?,
                    child(1)?,
                ),
                "quire.op.integer.mul" if arguments.len() == 2 => NodeKind::Arithmetic(
                    quire_checked::node::Arithmetic::Multiply,
                    child(0)?,
                    child(1)?,
                ),
                "quire.op.integer.negate" if arguments.len() == 1 => NodeKind::Negate(child(0)?),
                "quire.op.integer.lt" if arguments.len() == 2 => NodeKind::Order(
                    quire_exact::OrderingOperator::Less,
                    quire_checked::node::OrderedKind::Integers,
                    child(0)?,
                    child(1)?,
                ),
                "quire.op.integer.le" if arguments.len() == 2 => NodeKind::Order(
                    quire_exact::OrderingOperator::LessOrEqual,
                    quire_checked::node::OrderedKind::Integers,
                    child(0)?,
                    child(1)?,
                ),
                "quire.op.integer.gt" if arguments.len() == 2 => NodeKind::Order(
                    quire_exact::OrderingOperator::Greater,
                    quire_checked::node::OrderedKind::Integers,
                    child(0)?,
                    child(1)?,
                ),
                "quire.op.integer.ge" if arguments.len() == 2 => NodeKind::Order(
                    quire_exact::OrderingOperator::GreaterOrEqual,
                    quire_checked::node::OrderedKind::Integers,
                    child(0)?,
                    child(1)?,
                ),
                "quire.op.integer.eq"
                | "quire.op.integer.ne"
                | "quire.op.boolean.eq"
                | "quire.op.boolean.ne" => {
                    return Err(crate::protocol_slot_select::ViewFailure::Unsupported {
                        node: node.clone(),
                        cause: "checked-equality-certificate-not-exposed-by-ir-operation-read",
                    });
                }
                "quire.op.record.project" => {
                    if arguments.len() != 1 {
                        return Err(unsupported());
                    }
                    let Some(quire_contract_model::CheckedOperationMember::Field {
                        declaration,
                        name,
                        read_form,
                    }) = operation.read().ok().and_then(|read| read.member.cloned())
                    else {
                        return Err(unsupported());
                    };
                    let optional = matches!(projection.value_type(), ValueType::Option(_));
                    match read_form {
                        Some(form) => {
                            let owner_projection = read
                                .admitted
                                .project_exact_type(&declaration, identities, 10_000)
                                .map_err(|_| unsupported())?;
                            let ValueType::Reference(owner) = owner_projection.value_type() else {
                                return Err(unsupported());
                            };
                            let derefed = match form {
                                quire_contract_model::CheckedFieldReadForm::Bare => false,
                                quire_contract_model::CheckedFieldReadForm::ExplicitDeref => true,
                            };
                            NodeKind::Attribute {
                                reference: child(0)?,
                                field: FieldRef::new(owner.clone(), name.to_string()),
                                optional,
                                derefed,
                            }
                        }
                        None => {
                            let record = read
                                .admitted
                                .project_exact_type(&declaration, identities, 10_000)
                                .map_err(|_| unsupported())?;
                            let ValueType::Composite(record) = record.value_type() else {
                                return Err(unsupported());
                            };
                            NodeKind::Field {
                                operand: child(0)?,
                                field: quire_checked::node::RecordField::new(
                                    record.clone(),
                                    name.to_string(),
                                ),
                                optional,
                            }
                        }
                    }
                }
                "quire.op.function.call" => {
                    if arguments.is_empty() {
                        return Err(unsupported());
                    }
                    let CheckedTerm::Reference { target } = &arguments[0] else {
                        return Err(unsupported());
                    };
                    let function = read
                        .admitted
                        .graph()
                        .nodes
                        .iter()
                        .filter(|candidate| candidate.node_tag.as_ref() == "function")
                        .position(|candidate| candidate.node_id == *target)
                        .ok_or_else(unsupported)?;
                    let call_arguments = (1..arguments.len())
                        .map(child)
                        .collect::<Result<Vec<_>, _>>()?;
                    NodeKind::Call {
                        function,
                        arguments: call_arguments,
                    }
                }
                "quire.op.value.record_value" => {
                    return Err(crate::protocol_slot_select::ViewFailure::Refused {
                        code: "invalid_model_binding",
                        cause: "missing-identity-witness-for-record-construction",
                        locus: crate::protocol_slot_select::ViewLocus::Node(node.clone()),
                    });
                }
                _ => return Err(unsupported()),
            }
        }
        CheckedTerm::Reference { target } => {
            let target_node = read
                .admitted
                .graph()
                .nodes
                .iter()
                .find(|candidate| candidate.node_id == *target)
                .ok_or_else(|| {
                    crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                        "FR-390",
                        "admitted-reference-target-resolves",
                    ))
                })?;
            if target_node.semantic_form.as_ref() != "parameter" {
                return Err(unsupported());
            }
            let level = named_member(&target_node.body, "level")
                .and_then(|(_, term)| integer_literal(term))
                .ok_or_else(unsupported)?;
            NodeKind::Local(level)
        }
        CheckedTerm::Aggregate { members } => {
            let source_node = read
                .node_keys
                .iter()
                .find_map(|(id, key)| (key == node).then_some(id))
                .and_then(|id| {
                    read.admitted
                        .graph()
                        .nodes
                        .iter()
                        .find(|candidate| &candidate.node_id == id)
                })
                .ok_or_else(|| {
                    crate::protocol_slot_select::ViewFailure::Fault(InternalFault::new(
                        "FR-390",
                        "reader-value-node-has-admitted-form",
                    ))
                })?;
            if source_node.semantic_form.as_ref() != "record_value" {
                return Err(unsupported());
            }
            let record = projection.record_view().ok_or_else(unsupported)?;
            let mut slots = Vec::with_capacity(record.fields().len());
            for field in record.fields() {
                let Some(position) =
                    members
                        .iter()
                        .enumerate()
                        .find_map(|(position, member)| match member {
                            CheckedTerm::Binding { name, .. } if name.as_ref() == field.name() => {
                                Some(position)
                            }
                            _ => None,
                        })
                else {
                    if field.presence() == quire_exact::Presence::Optional {
                        slots.push(RecordSlot::Absent);
                        continue;
                    }
                    return Err(unsupported());
                };
                let mut field_path = path.to_vec();
                field_path.push(position);
                slots.push(RecordSlot::Present(field_path));
            }
            NodeKind::Record {
                declaration: record.record_key().clone(),
                slots,
            }
        }
        _ => return Err(unsupported()),
    };
    Ok(ReaderNode {
        operation,
        value_type: projection.value_type().clone(),
        location,
    })
}

/// An opaque expression-node handle whose lifetime and owner are the view
/// that minted it. It contains no authority to construct or retype a node.
#[derive(Clone)]
pub struct TermHandle<'view> {
    owner: &'view AdmittedEvaluationView<'view>,
    node: TermNode<'view>,
}

#[derive(Clone)]
enum TermNode<'view> {
    InProcess(CheckedNode<'view>),
    #[cfg(feature = "std")]
    Reader {
        node: quire_contract_model::CheckedNodeId,
        path: Vec<usize>,
    },
}

impl TermHandle<'_> {
    /// The source location of this selected expression node.
    pub fn location(&self) -> &quire_semantic_value::location::Location {
        match &self.node {
            TermNode::InProcess(node) => node.location(),
            #[cfg(feature = "std")]
            TermNode::Reader { node, path } => self
                .owner
                .reader_nodes
                .get(&(node.clone(), path.clone()))
                .and_then(|projected| projected.as_ref().ok())
                .map(|projected| &projected.location)
                .expect("reader handles are minted from preflighted nodes"),
        }
    }
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

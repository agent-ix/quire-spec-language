// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-485: in-process checked-closure acquisition and application through the replay facade.

use std::collections::BTreeSet;
use std::sync::Arc;

use qsl_eval::value::PackageClosure;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::SourceIdentity;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedExpression, CheckedNode, NodeKind};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::{
    Admission, BodyFactory, CallFailure, Evaluation, Progress, Registry, Session, Signature,
};
use quire_exact::{
    Identifier, Meter, NodeKey, Origin as OccurrenceOrigin, Role, UnionMember, Value, ValueType,
    VariantId,
};
use quire_semantic_value::declaration::CompositeShape;
use quire_semantic_value::location::Location;
use quire_semantic_value::object_closure::ObjectClosure;

use crate::admitted::{context_of, AdmittedContext, AdmittedRefusal};
use crate::bounds::ReplayLimits;
use crate::compile::{recompile_unit, CompiledPackage};
use crate::request::StageLimits;
use crate::spine::DependencyInput;
use crate::DigestRecord;

/// The checked form of a nominal declaration available through the closure facade.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NominalDeclarationForm {
    /// A record with ordered named fields.
    Record,
    /// A tuple with ordered zero-based positions.
    Tuple,
    /// A union with ordered, sealed variants.
    Union,
}

/// One checked member of a nominal declaration, in source order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NominalDeclarationMember {
    /// A record field and its exact admitted type.
    RecordField {
        /// The field's checked identifier.
        identifier: Identifier,
        /// The field's exact admitted value type.
        value_type: ValueType,
    },
    /// A tuple position and its exact admitted type.
    TuplePosition {
        /// The zero-based position in the checked tuple.
        position: usize,
        /// The position's exact admitted value type.
        value_type: ValueType,
    },
    /// A union member with the sealed identity, variant and ordered payload types.
    UnionMember {
        /// The checked sealed member binding; clone it to preserve its identity.
        member: UnionMember,
        /// The exact variant identity carried by values of this member.
        variant: VariantId,
        /// The member's payload types in declared order.
        payload: Vec<ValueType>,
    },
}

/// An immutable view of one owner-authored checked nominal declaration.
/// Consumers cannot forge one:
///
/// ```compile_fail
/// fn forge(package_id: qsl_replay::DigestRecord, node: quire_exact::NodeKey) {
///     let _ = qsl_replay::NominalDeclaration {
///         package_id,
///         node,
///         key: node,
///         qualified_name: vec![],
///         form: qsl_replay::NominalDeclarationForm::Record,
///         members: vec![],
///     };
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NominalDeclaration {
    package_id: DigestRecord,
    node: NodeKey,
    key: NodeKey,
    qualified_name: Vec<Identifier>,
    form: NominalDeclarationForm,
    members: Vec<NominalDeclarationMember>,
}

impl NominalDeclaration {
    /// The package that checked and owns this declaration.
    pub fn package_id(&self) -> DigestRecord {
        self.package_id
    }

    /// The exact checked `composite_type` node id.
    pub fn node(&self) -> NodeKey {
        self.node
    }

    /// The exact admitted kernel declaration key used by `ValueType::Composite`.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The checked qualified declaration name.
    pub fn qualified_name(&self) -> &[Identifier] {
        &self.qualified_name
    }

    /// The checked declaration form.
    pub fn form(&self) -> NominalDeclarationForm {
        self.form
    }

    /// Every checked member in its declared order.
    pub fn members(&self) -> &[NominalDeclarationMember] {
        &self.members
    }
}

/// Why a checked nominal declaration view could not be issued.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NominalDeclarationRefusal {
    /// The selection was issued by a different checked acquisition.
    #[error("selection belongs to another checked acquisition")]
    WrongAcquisition,
    /// The node is owned by exactly one other package in the closure.
    #[error("nominal declaration belongs to another package {0:?}")]
    Foreign(DigestRecord),
    /// No declaration reachable from this selected callable was found.
    #[error("unknown nominal declaration")]
    Unknown,
    /// The selected package owns the node, but it is not a composite declaration.
    #[error("node is not a nominal composite declaration")]
    WrongKind,
    /// Checked node metadata and the owner's admitted declaration disagree.
    #[error("checked nominal declaration data is inconsistent")]
    Inconsistent,
}

type Adapter = PackageClosure<Arc<CheckedPackage>, Arc<CheckedExpression>>;

/// An emitted package, its admitted tables, and its retained checked closure.
pub struct CheckedClosureCompilation {
    package: CompiledPackage,
    context: AdmittedContext,
    handle: CheckedClosureHandle,
}

impl CheckedClosureCompilation {
    /// The same emitted package that `compile_admitted` returns.
    pub fn package(&self) -> &CompiledPackage {
        &self.package
    }

    /// The same admitted tables that `compile_admitted` returns.
    pub fn context(&self) -> &AdmittedContext {
        &self.context
    }

    /// The retained checked closure, valid independently of these public values.
    pub fn handle(&self) -> &CheckedClosureHandle {
        &self.handle
    }

    /// Separate the three independently owned results.
    pub fn into_parts(self) -> (CompiledPackage, AdmittedContext, CheckedClosureHandle) {
        (self.package, self.context, self.handle)
    }
}

/// The checked package and exact linked dependency closure of one acquisition.
/// A consumer cannot reach the checked package through the facade:
///
/// ```compile_fail
/// fn inspect(handle: &qsl_replay::CheckedClosureHandle) {
///     let _ = &handle.package;
/// }
/// ```
#[derive(Clone)]
pub struct CheckedClosureHandle {
    package: Arc<CheckedPackage>,
    package_id: DigestRecord,
    acquisition: Arc<()>,
}

impl CheckedClosureHandle {
    /// The emitted unit's exact package identity.
    pub fn package_id(&self) -> DigestRecord {
        self.package_id
    }

    /// Look up a check-minted callable in the selected owner of this closure.
    pub fn lookup_callable(
        &self,
        owner_package_id: DigestRecord,
        wire_node_id: WireNodeId,
    ) -> Result<CallableSelection, CallableLookupRefusal> {
        let mut packages = vec![(self.package_id, self.package.as_ref())];
        let mut visited = BTreeSet::new();
        let mut selected = None;
        let mut other_owner = None;
        let mut ambiguous = false;
        while let Some((id, package)) = packages.pop() {
            if !visited.insert(id) {
                continue;
            }
            if id == owner_package_id {
                selected = Some(package);
            } else if has_node(package, wire_node_id) && other_owner.replace(id).is_some() {
                ambiguous = true;
            }
            packages.extend(
                package
                    .dependencies()
                    .iter()
                    .map(|(id, dependency)| (id.record(), dependency.as_ref())),
            );
        }
        let Some(package) = selected else {
            return Err(CallableLookupRefusal::Unknown);
        };
        {
            if let Some(key) = package
                .graph()
                .semantic_graph()
                .nodes()
                .map(|node| node.key())
                .find(|key| key.as_bytes() == wire_node_id.as_bytes())
            {
                if let Some(function) = package.graph().callable_by_identity(key) {
                    return Ok(CallableSelection {
                        owner_package_id,
                        key,
                        signature: Signature::new(
                            function
                                .parameters
                                .iter()
                                .map(|(_, ty)| ty.clone())
                                .collect::<Vec<_>>(),
                            function.result.clone(),
                        ),
                        acquisition: Arc::clone(&self.acquisition),
                    });
                }
                return Err(CallableLookupRefusal::WrongKind);
            }
        }
        match (other_owner, ambiguous) {
            (Some(owner), false) => Err(CallableLookupRefusal::Foreign(owner)),
            _ => Err(CallableLookupRefusal::Unknown),
        }
    }

    /// Read one checked nominal declaration reachable from `selection`'s
    /// signature, from the exact package that owns that callable. The view
    /// retains the check-minted node, declaration and union-member identities;
    /// it exposes no checked graph and carries no application authority.
    pub fn nominal_declaration(
        &self,
        selection: &CallableSelection,
        declaration: WireNodeId,
    ) -> Result<NominalDeclaration, NominalDeclarationRefusal> {
        if !Arc::ptr_eq(&self.acquisition, &selection.acquisition) {
            return Err(NominalDeclarationRefusal::WrongAcquisition);
        }
        let packages = self.closure_packages();
        let Some(owner) = packages
            .iter()
            .find(|(id, _)| *id == selection.owner_package_id)
            .map(|(_, package)| *package)
        else {
            return Err(NominalDeclarationRefusal::Unknown);
        };
        let Some(node) = owner
            .graph()
            .semantic_graph()
            .nodes()
            .find(|node| node.key().as_bytes() == declaration.as_bytes())
        else {
            let foreign: Vec<_> = packages
                .iter()
                .filter(|(id, package)| {
                    *id != selection.owner_package_id && has_node(package, declaration)
                })
                .map(|(id, _)| *id)
                .collect();
            return match foreign.as_slice() {
                [package] => Err(NominalDeclarationRefusal::Foreign(*package)),
                _ => Err(NominalDeclarationRefusal::Unknown),
            };
        };
        let graph = owner.graph();
        let Some(checked) = graph.checked_type_node(node.key()) else {
            return if node.node_tag() == qsl_semantics::check::NodeTag::CompositeType {
                Err(NominalDeclarationRefusal::Inconsistent)
            } else {
                Err(NominalDeclarationRefusal::WrongKind)
            };
        };
        let qsl_semantics::check::CheckedTypeNode::Composite {
            node: checked_node,
            declaration: key,
            form,
        } = checked
        else {
            return Err(NominalDeclarationRefusal::WrongKind);
        };
        if *checked_node != node.key()
            || node.node_tag() != qsl_semantics::check::NodeTag::CompositeType
        {
            return Err(NominalDeclarationRefusal::Inconsistent);
        }
        let Some(qualified_name) = node.declaration() else {
            return Err(NominalDeclarationRefusal::Inconsistent);
        };
        let Some(composite) = graph.scope().types().composite(*key) else {
            return Err(NominalDeclarationRefusal::Inconsistent);
        };
        let declaration_form = nominal_declaration_form(*form, composite.shape())?;
        let reachable = reachable_composites(graph, &selection.signature)
            .map_err(|()| NominalDeclarationRefusal::Inconsistent)?;
        if !reachable.contains(key) {
            return Err(NominalDeclarationRefusal::Unknown);
        }
        let members = match composite.shape() {
            CompositeShape::Record(fields) => fields
                .iter()
                .map(|field| {
                    let identifier = Identifier::new(field.name())
                        .map_err(|_| NominalDeclarationRefusal::Inconsistent)?;
                    Ok(NominalDeclarationMember::RecordField {
                        identifier,
                        value_type: field.value_type().clone(),
                    })
                })
                .collect::<Result<Vec<_>, NominalDeclarationRefusal>>()?,
            CompositeShape::Tuple(positions) => positions
                .iter()
                .enumerate()
                .map(
                    |(position, value_type)| NominalDeclarationMember::TuplePosition {
                        position,
                        value_type: value_type.clone(),
                    },
                )
                .collect(),
            CompositeShape::Union(members) => members
                .iter()
                .map(|member| {
                    let binding = member
                        .member()
                        .ok_or(NominalDeclarationRefusal::Inconsistent)?;
                    if binding.declaration() != *checked_node
                        || binding.identifier() != member.identifier()
                    {
                        return Err(NominalDeclarationRefusal::Inconsistent);
                    }
                    Ok(NominalDeclarationMember::UnionMember {
                        member: binding.clone(),
                        variant: binding.variant(),
                        payload: member.positions().to_vec(),
                    })
                })
                .collect::<Result<Vec<_>, NominalDeclarationRefusal>>()?,
        };
        Ok(NominalDeclaration {
            package_id: selection.owner_package_id,
            node: *checked_node,
            key: *key,
            qualified_name: qualified_name.to_vec(),
            form: declaration_form,
            members,
        })
    }

    /// Issue the checked source location of one nested `call` expression inside
    /// the selected caller's checked body, minted from this acquisition's
    /// retained closure. The result is a pure function of the acquisition, the
    /// caller, and the occurrence: it is unchanged by registration, Begin or
    /// sealing, and a refusal makes no registry change and no charge.
    pub fn nested_call_site(
        &self,
        caller: &CallableSelection,
        call: NestedCallOccurrence,
    ) -> Result<NestedCallSite, NestedCallSiteRefusal> {
        if !Arc::ptr_eq(&self.acquisition, &caller.acquisition) {
            return Err(NestedCallSiteRefusal::WrongAcquisition);
        }
        let packages = self.closure_packages();
        // A selection of this acquisition names an owner and body of this
        // closure; were either absent, no occurrence could be reached from it.
        let Some(owner) = packages
            .iter()
            .find(|(id, _)| *id == caller.owner_package_id)
            .map(|(_, package)| *package)
        else {
            return Err(NestedCallSiteRefusal::Unreachable);
        };
        let graph = owner.graph();
        let Some(node) = graph
            .semantic_graph()
            .nodes()
            .find(|node| node.key().as_bytes() == call.call_node.as_bytes())
        else {
            let owners: Vec<_> = packages
                .iter()
                .filter(|(id, package)| {
                    *id != caller.owner_package_id && has_node(package, call.call_node)
                })
                .map(|(id, _)| *id)
                .collect();
            return Err(foreign_or_unknown(&owners));
        };
        if !is_call_form(node.semantic_form()) {
            return Err(NestedCallSiteRefusal::WrongKind);
        }
        let Some(site) = graph.occurrence(
            node.key(),
            &OccurrenceOrigin::new(Role::new("expression"), call.ordinal),
        ) else {
            return Err(NestedCallSiteRefusal::UnknownOccurrence);
        };
        let Some(caller_function) = graph.function_by_identity(caller.key) else {
            return Err(NestedCallSiteRefusal::Unreachable);
        };
        if !body_holds_call_at(caller_function.body, site) {
            return Err(NestedCallSiteRefusal::Unreachable);
        }
        Ok(NestedCallSite {
            owner_package_id: caller.owner_package_id,
            location: site.clone(),
        })
    }

    /// Every distinct checked package of the closure with its exact id.
    fn closure_packages(&self) -> Vec<(DigestRecord, &CheckedPackage)> {
        let mut pending = vec![(self.package_id, self.package.as_ref())];
        let mut visited = BTreeSet::new();
        let mut packages = Vec::new();
        while let Some((id, package)) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            packages.push((id, package));
            pending.extend(
                package
                    .dependencies()
                    .iter()
                    .map(|(id, dependency)| (id.record(), dependency.as_ref())),
            );
        }
        packages
    }

    /// Create a fresh registry over the designated QSL adapter and leaf scheduler.
    pub fn registry(&self) -> ApplicationRegistry {
        ApplicationRegistry {
            acquisition: Arc::clone(&self.acquisition),
            inner: Registry::new(PackageClosure::new(Arc::clone(&self.package))),
        }
    }
}

fn has_node(package: &CheckedPackage, node: WireNodeId) -> bool {
    package
        .graph()
        .semantic_graph()
        .nodes()
        .any(|candidate| candidate.key().as_bytes() == node.as_bytes())
}

/// Collect the composite declaration keys reachable through one admitted
/// signature and its nested record, tuple and union members. Any broken link
/// in the checked owner's own descriptor makes the result inconsistent.
fn reachable_composites(
    graph: &qsl_semantics::check::CheckedGraph,
    signature: &Signature,
) -> Result<BTreeSet<NodeKey>, ()> {
    let mut pending = signature
        .parameters()
        .iter()
        .chain(std::iter::once(signature.result()))
        .cloned()
        .collect::<Vec<_>>();
    let mut reachable = BTreeSet::new();
    while let Some(value_type) = pending.pop() {
        match &value_type {
            ValueType::Option(payload) => pending.push((**payload).clone()),
            ValueType::Collection(collection) => pending.push(collection.element().clone()),
            // A map is type-only (it admits no value) but its value type can
            // still name a composite declaration, so the walk follows it.
            ValueType::Map(map) => pending.push(map.values().clone()),
            ValueType::Composite(key) => {
                if !reachable.insert(*key) {
                    continue;
                }
                let checked = graph
                    .checked_type_nodes()
                    .find(|type_node| {
                        matches!(type_node, qsl_semantics::check::CheckedTypeNode::Composite { declaration, .. } if *declaration == *key)
                    })
                    .ok_or(())?;
                let qsl_semantics::check::CheckedTypeNode::Composite {
                    node,
                    declaration,
                    form,
                } = checked
                else {
                    return Err(());
                };
                if *declaration != *key
                    || graph.semantic_graph().node(*node).is_none_or(|node| {
                        node.node_tag() != qsl_semantics::check::NodeTag::CompositeType
                    })
                {
                    return Err(());
                }
                let composite = graph.scope().types().composite(*key).ok_or(())?;
                if nominal_declaration_form(*form, composite.shape()).is_err() {
                    return Err(());
                }
                match composite.shape() {
                    CompositeShape::Record(fields) => {
                        pending.extend(fields.iter().map(|field| field.value_type().clone()));
                    }
                    CompositeShape::Tuple(positions) => pending.extend(positions.iter().cloned()),
                    CompositeShape::Union(members) => {
                        for member in members {
                            if member.member().is_none_or(|binding| {
                                binding.declaration() != *node
                                    || binding.identifier() != member.identifier()
                            }) {
                                return Err(());
                            }
                            pending.extend(member.positions().iter().cloned());
                        }
                    }
                }
            }
            ValueType::Boolean
            | ValueType::Integer
            | ValueType::Int(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Quantity(_)
            // A bounded quantity is a quantity leaf and names no composite.
            | ValueType::BoundedQuantity(_)
            | ValueType::Text(_)
            | ValueType::String
            | ValueType::Uuid
            | ValueType::Timestamp
            | ValueType::Enum(_)
            | ValueType::Reference(_)
            | ValueType::Population(_) => {}
        }
    }
    Ok(reachable)
}

fn nominal_declaration_form(
    form: qsl_semantics::check::CompositeForm,
    shape: &CompositeShape,
) -> Result<NominalDeclarationForm, NominalDeclarationRefusal> {
    if qsl_semantics::check::CompositeForm::from(shape) != form {
        return Err(NominalDeclarationRefusal::Inconsistent);
    }
    Ok(match form {
        qsl_semantics::check::CompositeForm::Record => NominalDeclarationForm::Record,
        qsl_semantics::check::CompositeForm::Tuple => NominalDeclarationForm::Tuple,
        qsl_semantics::check::CompositeForm::Union => NominalDeclarationForm::Union,
    })
}

/// The refusal for a node the caller's package lacks: its one actual owner, and
/// never an arbitrary one of several.
fn foreign_or_unknown(owners: &[DigestRecord]) -> NestedCallSiteRefusal {
    match owners {
        [owner] => NestedCallSiteRefusal::ForeignCall(*owner),
        _ => NestedCallSiteRefusal::UnknownCall,
    }
}

/// Whether the checked body holds a `call` or imported `call` node whose
/// checked location is exactly `site`, walking without recursion.
fn body_holds_call_at(root: CheckedNode<'_>, site: &Location) -> bool {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        let is_call = matches!(
            node.kind(),
            NodeKind::Call { .. } | NodeKind::ImportedCall { .. }
        );
        if is_call && node.location() == site {
            return true;
        }
        pending.extend(node.children());
    }
    false
}

/// Whether a checked node's FR-322 semantic form is the function application.
#[ix_trace_rs::string_edge]
fn is_call_form(form: &str) -> bool {
    form == "call"
}

/// A checker-issued callable bound to one acquisition, with no public constructor.
///
/// ```compile_fail
/// fn forge(selection: &qsl_replay::CallableSelection) {
///     let _ = selection.acquisition;
/// }
/// ```
#[derive(Clone)]
pub struct CallableSelection {
    owner_package_id: DigestRecord,
    key: NodeKey,
    signature: Signature,
    acquisition: Arc<()>,
}

impl CallableSelection {
    /// The package that owns this callable.
    pub fn owner_package_id(&self) -> DigestRecord {
        self.owner_package_id
    }
    /// The check-minted key, for generated-body diagnostics only.
    pub fn key(&self) -> NodeKey {
        self.key
    }
    /// The exact admitted signature.
    pub fn signature(&self) -> &Signature {
        &self.signature
    }
}

/// Why a callable lookup did not issue a selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CallableLookupRefusal {
    /// The owner or node is absent, or several other packages own the node.
    #[error("unknown callable")]
    Unknown,
    /// Exactly one other package in the closure owns the node.
    #[error("callable belongs to package {0:?}")]
    Foreign(DigestRecord),
    /// The selected package owns the node but it is not callable by name.
    #[error("node is not callable by name")]
    WrongKind,
}

/// The identity a CG consumer holds for one nested call: the checked `call`
/// node's wire id and the ordinal of its `expression` occurrence, the same
/// pair its checked-package source map records. It names nothing by itself;
/// [`CheckedClosureHandle::nested_call_site`] resolves it inside a selected caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NestedCallOccurrence {
    call_node: WireNodeId,
    ordinal: u64,
}

impl NestedCallOccurrence {
    /// Name the `ordinal`th `expression` occurrence of the `call_node`.
    pub fn new(call_node: WireNodeId, ordinal: u64) -> Self {
        Self { call_node, ordinal }
    }
}

/// The checked location of one nested call, issued by the retained closure and
/// constructible nowhere else.
///
/// ```compile_fail
/// fn forge(site: qsl_replay::NestedCallSite) {
///     let _ = site.location;
/// }
/// ```
///
/// The struct is exported, so a literal fails only because its fields are private:
///
/// ```compile_fail
/// fn forge(owner: qsl_replay::DigestRecord, location: quire_semantic_value::location::Location) {
///     let _ = qsl_replay::NestedCallSite { owner_package_id: owner, location };
/// }
/// ```
#[derive(Clone, Debug)]
pub struct NestedCallSite {
    owner_package_id: DigestRecord,
    location: Location,
}

impl NestedCallSite {
    /// The declaring package whose checked body holds the call.
    pub fn owner_package_id(&self) -> DigestRecord {
        self.owner_package_id
    }
    /// The exact checked location of the call expression, for the leaf
    /// `ChildRequest::site`.
    pub fn location(&self) -> &Location {
        &self.location
    }
}

/// Why no call site was issued.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NestedCallSiteRefusal {
    /// The caller selection was issued by another acquisition.
    #[error("caller selection belongs to another acquisition")]
    WrongAcquisition,
    /// No package owns the node, or several other packages do.
    #[error("unknown call node")]
    UnknownCall,
    /// Exactly one other package in the closure owns the node.
    #[error("call node belongs to package {0:?}")]
    ForeignCall(DigestRecord),
    /// The caller's package owns the node but it is not a function call.
    #[error("node is not a call")]
    WrongKind,
    /// The call node has no `expression` occurrence with this ordinal.
    #[error("call node has no such occurrence")]
    UnknownOccurrence,
    /// The occurrence is not a call within the selected caller's checked body
    /// (including a caller body the closure does not hold).
    #[error("call occurrence is not reachable from the caller body")]
    Unreachable,
}

/// Acquire and retain the checked closure from the one shared compilation path.
pub fn acquire_checked_closure<'a>(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    limits: StageLimits,
    replay_limits: ReplayLimits,
) -> Result<CheckedClosureCompilation, AdmittedRefusal> {
    let compiled = recompile_unit(
        source,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        replay_limits,
    )?;
    let package_id = compiled.emitted.package().package_id();
    let context =
        context_of(compiled.checked.package(), package_id).map_err(AdmittedRefusal::Fault)?;
    Ok(CheckedClosureCompilation {
        package: CompiledPackage::new(compiled.emitted),
        context,
        handle: CheckedClosureHandle {
            package: Arc::clone(compiled.checked.shared_package()),
            package_id: package_id.record(),
            acquisition: Arc::new(()),
        },
    })
}

/// The object closure as admitted by the existing QSL environment constructor.
pub struct ApplicationEnvironment(ObjectEnvironment);

impl ApplicationEnvironment {
    /// Use the supplied object closure without inventing population data.
    pub fn new(objects: ObjectClosure) -> Self {
        Self(ObjectEnvironment::new(objects))
    }
}

/// A leaf registration result or an acquisition-identity refusal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SelectionRefusal {
    /// The selection was issued by another acquisition.
    #[error("selection belongs to another acquisition")]
    WrongAcquisition,
}

/// A leaf Begin failure or an acquisition-identity refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ApplicationBeginRefusal {
    /// The selection was issued by another acquisition.
    #[error("selection belongs to another acquisition")]
    WrongAcquisition,
    /// The leaf refused admission before starting a session.
    #[error(transparent)]
    Call(#[from] CallFailure),
}

/// A binding set over precisely one retained acquisition.
pub struct ApplicationRegistry {
    acquisition: Arc<()>,
    inner: Registry<Adapter>,
}

impl ApplicationRegistry {
    /// Bind a generated body after checking acquisition identity.
    pub fn register(
        &mut self,
        selection: &CallableSelection,
        signature: &Signature,
        factory: impl BodyFactory + 'static,
    ) -> Result<Admission, SelectionRefusal> {
        if !Arc::ptr_eq(&self.acquisition, &selection.acquisition) {
            return Err(SelectionRefusal::WrongAcquisition);
        }
        Ok(self.inner.register(selection.key, signature, factory))
    }

    /// Begin a call under the leaf's admission and permanent sealing rules.
    pub fn begin_call<'a>(
        &mut self,
        selection: &CallableSelection,
        arguments: Vec<Value>,
        environment: &'a ApplicationEnvironment,
        meter: &'a mut Meter,
    ) -> Result<CheckedApplicationSession<'a>, ApplicationBeginRefusal> {
        if !Arc::ptr_eq(&self.acquisition, &selection.acquisition) {
            return Err(ApplicationBeginRefusal::WrongAcquisition);
        }
        self.inner
            .begin_call(selection.key, arguments, &environment.0, meter)
            .map(|inner| CheckedApplicationSession { inner })
            .map_err(ApplicationBeginRefusal::Call)
    }

    /// Whether a successful Begin permanently sealed this binding set.
    pub fn is_sealed(&self) -> bool {
        self.inner.is_sealed()
    }
}

/// A live leaf application whose meter belongs to its caller.
pub struct CheckedApplicationSession<'a> {
    inner: Session<'a, Adapter>,
}

impl<'a> CheckedApplicationSession<'a> {
    /// Take one leaf transition, consuming this session.
    pub fn step(self) -> CheckedApplicationProgress<'a> {
        match self.inner.step() {
            Progress::Runnable(inner) => CheckedApplicationProgress::Runnable(Self { inner }),
            Progress::Terminal(result) => CheckedApplicationProgress::Terminal(result),
        }
    }

    /// Drive this leaf session to its terminal result.
    pub fn run(self) -> Result<Evaluation, CallFailure> {
        self.inner.run()
    }
}

/// The runnable or terminal state of one leaf step.
#[must_use = "dropping a runnable session abandons the application"]
pub enum CheckedApplicationProgress<'a> {
    /// The same application can be stepped again.
    Runnable(CheckedApplicationSession<'a>),
    /// The application ended and its session was consumed.
    Terminal(Result<Evaluation, CallFailure>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::SuppliedLibrary;
    use ix_trace_rs::trace;
    use quire_eval::{Body, BodyFailure, ChildRequest, FamilyOutcome, Resume, Transition};
    use quire_exact::{Cancel, CancelCause, Integer, LimitKind, Outcome, ScalarLimits, ValueType};
    use quire_semantic_value::call::InputRefusal;
    use quire_semantic_value::location::Location;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const HEADER: &str =
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
    const FUNCTIONS: &str = "function f using v(x: Int[0, 9]): Integer pure { x }\nfunction g using v(x: Int[0, 9]): Integer pure { f(x) }\n";

    fn identity(name: &str) -> SourceIdentity {
        SourceIdentity::new("a", name, "git", "1")
    }
    fn acquire(text: &str, dependencies: &DependencyInput) -> CheckedClosureCompilation {
        acquire_checked_closure(
            identity("unit"),
            "unit.native",
            text.as_bytes(),
            [],
            dependencies,
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("checked acquisition")
    }
    fn wire(package: &CheckedPackage, name: &str) -> WireNodeId {
        WireNodeId::from_digest(
            *package
                .graph()
                .function_identity(name)
                .expect("function")
                .as_bytes(),
        )
    }
    fn meter(work_units: u64) -> Meter {
        let large = u64::from(u32::MAX);
        Meter::new(ScalarLimits {
            integer_bits: large,
            decimal_digits: large,
            scale_expansion: large,
            text_input_bytes: large,
            text_scalars: large,
            normalized_scalars: large,
            unit_edges: large,
            value_occurrences: large,
            work_units,
            result_units: large,
        })
    }
    fn int(value: u64) -> Value {
        Value::Integer(Integer::from(value))
    }
    fn completed(evaluation: &Evaluation) -> Option<u64> {
        match &evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => value.to_u64(),
            _ => None,
        }
    }

    #[derive(Clone)]
    struct Host {
        begun: Arc<AtomicUsize>,
        child: Option<NodeKey>,
    }
    impl BodyFactory for Host {
        fn begin(&self) -> Box<dyn Body> {
            self.begun.fetch_add(1, Ordering::SeqCst);
            Box::new(HostBody { child: self.child })
        }
    }
    struct HostBody {
        child: Option<NodeKey>,
    }
    impl Body for HostBody {
        fn resume(&mut self, input: Resume, _meter: &mut Meter) -> Result<Transition, BodyFailure> {
            Ok(match input {
                Resume::Start(arguments) => match self.child {
                    Some(callee) => Transition::Call(ChildRequest {
                        callee,
                        arguments,
                        site: Location::at(quire_semantic_value::location::Origin::Expression, &[]),
                    }),
                    None => Transition::Done(Evaluation {
                        outcome: FamilyOutcome::Evaluated(Outcome::Completed(int(7))),
                        location: None,
                        losses: vec![],
                    }),
                },
                Resume::Child(result) => Transition::Done(result),
                Resume::Again => Transition::Continue,
            })
        }
    }

    /// One compile yields equal public outputs and a retained owned closure.
    #[trace("TC-1263", "FR-485-AC-1", "FR-485-AC-8")]
    #[test]
    fn acquisition_retains_one_checked_closure() {
        let text =
            format!("{HEADER}import \"test/lib\" as l;\nenum Color {{ RED, BLUE }}\n{FUNCTIONS}");
        let input = DependencyInput::new([SuppliedLibrary {
            identity: "test/lib".to_owned(),
            source: identity("lib"),
            path: "lib.native".to_owned(),
            bytes: format!(
                "{HEADER}function imported using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n"
            )
            .into_bytes(),
        }])
        .expect("dependency input");
        let expected = crate::compile_admitted(
            identity("unit"),
            "unit.native",
            text.as_bytes(),
            [],
            &input,
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("admitted");
        let spine_runs = crate::execute::spine_run_count();
        let acquired = acquire(&text, &input);
        assert_eq!(crate::execute::spine_run_count(), spine_runs + 1);
        assert_eq!(acquired.package().bytes(), expected.package().bytes());
        assert_eq!(
            acquired.package().package_id(),
            expected.package().package_id()
        );
        assert_eq!(
            acquired.context().package_id(),
            expected.context().package_id()
        );
        assert_eq!(
            format!("{:?}", acquired.context()),
            format!("{:?}", expected.context())
        );
        let own = wire(&acquired.handle.package, "f");
        let (bytes, context, handle) = acquired.into_parts();
        drop((bytes, context, text, input));
        let selection = handle
            .lookup_callable(handle.package_id(), own)
            .expect("retained callable");
        assert_eq!(selection.key().as_bytes(), own.as_bytes());
        assert_eq!(selection.signature().parameters().len(), 1);
        let mut registry = handle.registry();
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        let result = registry
            .begin_call(&selection, vec![int(3)], &environment, &mut meter)
            .expect("begin")
            .run()
            .expect("run");
        assert_eq!(completed(&result), Some(3));
        assert!(meter.consumed(LimitKind::WorkUnits) > 0);
        use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
        let mut reference_meter = self::meter(1000);
        let reference = handle
            .package
            .call(
                &QualifiedName::unqualified("f").expect("function name"),
                vec![int(3)],
                &ObjectEnvironment::default(),
                &mut reference_meter,
            )
            .expect("reference call");
        assert_eq!(format!("{result:?}"), format!("{reference:?}"));
        for kind in LimitKind::ALL {
            assert_eq!(
                meter.consumed(kind),
                reference_meter.consumed(kind),
                "{kind:?}"
            );
        }
    }

    /// Owner selection precedes node lookup, including an imported callable.
    #[trace("TC-1263", "FR-485-AC-2")]
    #[test]
    fn owner_scoped_lookup_refuses_foreign_unknown_and_wrong_kind() {
        let library = SuppliedLibrary {
            identity: "test/lib".to_owned(),
            source: identity("lib"),
            path: "lib.native".to_owned(),
            bytes: format!(
                "{HEADER}function imported using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n"
            )
            .into_bytes(),
        };
        let lib_acquired = acquire_checked_closure(
            library.source.clone(),
            &library.path,
            &library.bytes,
            [],
            &DependencyInput::default(),
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("library");
        let lib_wire = wire(&lib_acquired.handle.package, "imported");
        let dependencies = DependencyInput::new([library]).expect("dependency input");
        let acquired = acquire(
            &format!("{HEADER}import \"test/lib\" as l;\n{FUNCTIONS}"),
            &dependencies,
        );
        let handle = acquired.handle();
        assert_eq!(
            handle.lookup_callable(handle.package_id(), lib_wire).err(),
            Some(CallableLookupRefusal::Foreign(
                lib_acquired.package().package_id()
            ))
        );
        let dependency = handle
            .lookup_callable(lib_acquired.package().package_id(), lib_wire)
            .expect("dependency callable");
        assert_eq!(
            dependency.owner_package_id(),
            lib_acquired.package().package_id()
        );
        assert_eq!(
            handle
                .lookup_callable(handle.package_id(), WireNodeId::from_digest([9; 32]))
                .err(),
            Some(CallableLookupRefusal::Unknown)
        );
        assert_eq!(
            handle
                .lookup_callable(
                    lib_acquired.package().package_id(),
                    wire(&handle.package, "f")
                )
                .err(),
            Some(CallableLookupRefusal::Foreign(handle.package_id()))
        );
        assert_eq!(
            handle
                .lookup_callable(
                    DigestRecord::mint(
                        qsl_foundation::digest::DigestDomain::PackageSemanticV2,
                        [8; 32]
                    ),
                    lib_wire
                )
                .err(),
            Some(CallableLookupRefusal::Unknown)
        );
        let mut registry = handle.registry();
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        let result = registry
            .begin_call(&dependency, vec![int(4)], &environment, &mut meter)
            .expect("begin imported")
            .run()
            .expect("run imported");
        assert!(matches!(
            result.outcome,
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
        ));
    }

    /// The facade identity check precedes the leaf's admission and seal checks.
    #[trace("TC-1263", "FR-485-AC-4", "FR-485-AC-5", "FR-485-AC-6")]
    #[test]
    fn bindings_sessions_and_acquisition_identity_follow_leaf_order() {
        let text = format!("{HEADER}{FUNCTIONS}");
        let first = acquire(&text, &DependencyInput::default());
        let second = acquire(&text, &DependencyInput::default());
        let own = wire(&first.handle.package, "f");
        let selection = first
            .handle()
            .lookup_callable(first.package().package_id(), own)
            .expect("selection");
        let foreign = second
            .handle()
            .lookup_callable(second.package().package_id(), own)
            .expect("same key second acquisition");
        assert_eq!(selection.key(), foreign.key());
        let mut registry = first.handle().registry();
        let begun = Arc::new(AtomicUsize::new(0));
        let host = Host {
            begun: Arc::clone(&begun),
            child: None,
        };
        assert_eq!(
            registry.register(&foreign, foreign.signature(), host.clone()),
            Err(SelectionRefusal::WrongAcquisition)
        );
        let wrong = Signature::new(vec![ValueType::Boolean], ValueType::Integer);
        assert_eq!(
            registry.register(&selection, &wrong, host.clone()),
            Ok(Admission::SignatureMismatch)
        );
        assert_eq!(
            registry.register(&selection, selection.signature(), host.clone()),
            Ok(Admission::Accepted)
        );
        assert_eq!(
            registry.register(&selection, selection.signature(), host.clone()),
            Ok(Admission::DuplicateBinding)
        );
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        assert!(matches!(
            registry.begin_call(&selection, vec![], &environment, &mut meter),
            Err(ApplicationBeginRefusal::Call(CallFailure::Input(
                InputRefusal::Arity { .. }
            )))
        ));
        assert!(matches!(
            registry.begin_call(
                &selection,
                vec![Value::Boolean(true)],
                &environment,
                &mut meter
            ),
            Err(ApplicationBeginRefusal::Call(CallFailure::Input(
                InputRefusal::WrongValueKind { parameter: 0 }
            )))
        ));
        assert!(!registry.is_sealed());
        assert_eq!(meter.admission_count(), 0);
        assert!(matches!(
            registry.begin_call(&foreign, vec![int(3)], &environment, &mut meter),
            Err(ApplicationBeginRefusal::WrongAcquisition)
        ));
        let session = registry
            .begin_call(&selection, vec![int(3)], &environment, &mut meter)
            .expect("begin");
        assert!(registry.is_sealed());
        assert_eq!(begun.load(Ordering::SeqCst), 0);
        drop(session);
        assert_eq!(begun.load(Ordering::SeqCst), 0);
        assert_eq!(
            registry.register(&foreign, foreign.signature(), host.clone()),
            Err(SelectionRefusal::WrongAcquisition)
        );
        assert!(matches!(
            registry.begin_call(&foreign, vec![int(3)], &environment, &mut meter),
            Err(ApplicationBeginRefusal::WrongAcquisition)
        ));
        assert!(registry.is_sealed());
        assert_eq!(meter.admission_count(), 0);
        assert_eq!(
            registry.register(&selection, selection.signature(), host),
            Ok(Admission::RegistrySealed)
        );
        let result = registry
            .begin_call(&selection, vec![int(4)], &environment, &mut meter)
            .expect("second begin")
            .run()
            .expect("host run");
        assert_eq!(completed(&result), Some(7));
        assert_eq!(begun.load(Ordering::SeqCst), 1);
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 1);
        let later = registry
            .begin_call(&selection, vec![int(5)], &environment, &mut meter)
            .expect("later begin")
            .run()
            .expect("later host run");
        assert_eq!(completed(&later), Some(7));
        assert_eq!(begun.load(Ordering::SeqCst), 2);
    }

    /// Refusals and cancellation never create a handle or charge at Begin.
    #[trace("TC-1263", "FR-485-AC-6", "FR-485-AC-7")]
    #[test]
    fn compile_refusals_and_cancelled_begin_preserve_leaf_results() {
        let malformed = "not a QSL unit";
        let input = DependencyInput::default();
        let expected = crate::compile_admitted(
            identity("unit"),
            "unit.native",
            malformed.as_bytes(),
            [],
            &input,
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .unwrap_err();
        let actual = acquire_checked_closure(
            identity("unit"),
            "unit.native",
            malformed.as_bytes(),
            [],
            &input,
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .err()
        .expect("refusal");
        assert_eq!(
            std::mem::discriminant(&actual),
            std::mem::discriminant(&expected)
        );
        let acquired = acquire(&format!("{HEADER}{FUNCTIONS}"), &input);
        let selected = acquired
            .handle()
            .lookup_callable(
                acquired.package().package_id(),
                wire(&acquired.handle.package, "f"),
            )
            .expect("selection");
        let mut registry = acquired.handle().registry();
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Requested);
        let mut meter = meter(1000).with_cancel(cancel);
        assert!(matches!(
            registry.begin_call(&selected, vec![int(3)], &environment, &mut meter),
            Err(ApplicationBeginRefusal::Call(CallFailure::Cancelled(
                CancelCause::Requested
            )))
        ));
        assert!(!registry.is_sealed());
        assert_eq!(meter.admission_count(), 0);
    }

    /// A private malformed selection reaches the leaf's own callable admission.
    #[trace("TC-1263", "FR-485-AC-2", "FR-485-AC-4")]
    #[test]
    fn public_lookup_and_private_malformed_selection_have_distinct_refusals() {
        let source = format!("{HEADER}enum Color {{ RED, BLUE }}\n{FUNCTIONS}");
        let acquired = acquire(&source, &DependencyInput::default());
        let handle = acquired.handle();
        let own = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "f"))
            .expect("own selection");
        let enum_key = acquired
            .context()
            .enums()
            .next()
            .expect("enum declaration")
            .key();
        assert_eq!(
            handle
                .lookup_callable(
                    handle.package_id(),
                    WireNodeId::from_digest(*enum_key.as_bytes())
                )
                .err(),
            Some(CallableLookupRefusal::WrongKind)
        );
        let name =
            crate::QualifiedName::new(vec![crate::Identifier::new("f").expect("identifier")])
                .expect("function name");
        let site = crate::call_site(
            identity("unit"),
            "unit.native",
            source.as_bytes(),
            [],
            &DependencyInput::default(),
            &name,
        )
        .expect("function site");
        let parameter = site.site.parameters[0].1;
        assert_eq!(
            handle.lookup_callable(handle.package_id(), parameter).err(),
            Some(CallableLookupRefusal::WrongKind)
        );
        let unrelated = acquire(
            &format!("{HEADER}function other using v(x: Int[0, 9]): Integer pure {{ x }}\n"),
            &DependencyInput::default(),
        );
        let missing_key = unrelated
            .handle
            .package
            .graph()
            .function_identity("other")
            .expect("other");
        let missing = WireNodeId::from_digest(*missing_key.as_bytes());
        assert_eq!(
            handle.lookup_callable(handle.package_id(), missing).err(),
            Some(CallableLookupRefusal::Unknown)
        );
        let mut malformed = own.clone();
        malformed.key = missing_key;
        let mut registry = handle.registry();
        let begun = Arc::new(AtomicUsize::new(0));
        assert_eq!(
            registry.register(
                &malformed,
                malformed.signature(),
                Host {
                    begun: Arc::clone(&begun),
                    child: None
                }
            ),
            Ok(Admission::CallableNotAdmitted)
        );
        assert_eq!(begun.load(Ordering::SeqCst), 0);
    }

    /// A registered body requests another callable through the one leaf session.
    #[trace("TC-1263", "FR-485-AC-5")]
    #[test]
    fn generated_body_child_request_uses_the_leaf_session() {
        let acquired = acquire(&format!("{HEADER}{FUNCTIONS}"), &DependencyInput::default());
        let handle = acquired.handle();
        let f = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "f"))
            .expect("f");
        let g = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "g"))
            .expect("g");
        let begun = Arc::new(AtomicUsize::new(0));
        let mut registry = handle.registry();
        assert_eq!(
            registry.register(
                &g,
                g.signature(),
                Host {
                    begun: Arc::clone(&begun),
                    child: Some(f.key())
                }
            ),
            Ok(Admission::Accepted)
        );
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        let result = registry
            .begin_call(&g, vec![int(3)], &environment, &mut meter)
            .expect("begin")
            .run()
            .expect("child application");
        assert_eq!(completed(&result), Some(3));
        assert_eq!(begun.load(Ordering::SeqCst), 1);
        assert!(meter.admission_count() >= 2, "root and child call charges");
    }

    /// Cancellation after Begin is a located incomplete outcome at the denied charge.
    #[trace("TC-1263", "FR-485-AC-6")]
    #[test]
    fn cancellation_after_begin_and_work_denial_preserve_leaf_accounting() {
        let dependencies = DependencyInput::new([SuppliedLibrary {
            identity: "test/lib".to_owned(),
            source: identity("lib"),
            path: "lib.native".to_owned(),
            bytes: format!(
                "{HEADER}function imported using v(x: Int[0, 9]): Boolean pure {{ x < 5 }}\n"
            )
            .into_bytes(),
        }])
        .expect("dependency input");
        let acquired = acquire(
            &format!("{HEADER}import \"test/lib\" as l;\n{FUNCTIONS}"),
            &dependencies,
        );
        let handle = acquired.handle();
        let selection = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "f"))
            .expect("f");
        let own_location = handle
            .package
            .graph()
            .function_by_identity(selection.key())
            .expect("own checked function")
            .body
            .location()
            .clone();
        let (imported_id, imported_package) = handle
            .package
            .dependencies()
            .iter()
            .next()
            .expect("retained checked dependency");
        let imported = handle
            .lookup_callable(imported_id.record(), wire(imported_package, "imported"))
            .expect("imported function");
        let imported_location = imported_package
            .graph()
            .function_by_identity(imported.key())
            .expect("imported checked function")
            .body
            .location()
            .clone();
        assert_ne!(own_location, imported_location);
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        for (callable, expected_location) in
            [(&selection, &own_location), (&imported, &imported_location)]
        {
            let cancel = Cancel::new();
            let mut cancelled_meter = meter(1000).with_cancel(cancel.clone());
            let mut registry = handle.registry();
            let session = registry
                .begin_call(callable, vec![int(3)], &environment, &mut cancelled_meter)
                .expect("begin");
            cancel.cancel(CancelCause::Requested);
            let result = session.run().expect("located cancelled outcome");
            assert!(matches!(
                result.outcome,
                FamilyOutcome::Evaluated(Outcome::Incomplete(_))
            ));
            assert_eq!(result.location.as_ref(), Some(expected_location));
            assert_eq!(cancelled_meter.admission_count(), 0);
            assert_eq!(cancel.tripped(), Some(CancelCause::Requested));

            let mut denied_meter = meter(0);
            let result = handle
                .registry()
                .begin_call(callable, vec![int(3)], &environment, &mut denied_meter)
                .expect("begin root work denial")
                .run()
                .expect("located root work denial");
            assert!(matches!(
                result.outcome,
                FamilyOutcome::Evaluated(Outcome::Incomplete(_))
            ));
            assert_eq!(result.location.as_ref(), Some(expected_location));
            assert_eq!(denied_meter.admission_count(), 0);
        }

        let mut baseline = meter(1000);
        let successful = handle
            .registry()
            .begin_call(&selection, vec![int(3)], &environment, &mut baseline)
            .expect("begin baseline")
            .run()
            .expect("baseline run");
        assert_eq!(completed(&successful), Some(3));
        let spend = baseline.consumed(LimitKind::WorkUnits);
        assert!(spend > 0);
        let mut limited = meter(spend - 1);
        let result = handle
            .registry()
            .begin_call(&selection, vec![int(3)], &environment, &mut limited)
            .expect("begin limited")
            .run()
            .expect("incomplete run");
        assert!(matches!(
            result.outcome,
            FamilyOutcome::Evaluated(Outcome::Incomplete(_))
        ));
        assert!(result.location.is_some());
        assert!(limited.consumed(LimitKind::WorkUnits) < spend);
        let mut exact = meter(spend);
        let result = handle
            .registry()
            .begin_call(&selection, vec![int(3)], &environment, &mut exact)
            .expect("begin exact")
            .run()
            .expect("exact run");
        assert_eq!(completed(&result), Some(3));
    }

    /// Dropping a runnable session does not resume a generated body again.
    #[trace("TC-1263", "FR-485-AC-6")]
    #[test]
    fn dropping_runnable_session_abandons_later_body_transition() {
        struct YieldingHost {
            begun: Arc<AtomicUsize>,
            started: Arc<AtomicUsize>,
            continued: Arc<AtomicUsize>,
        }
        struct YieldingBody {
            started: Arc<AtomicUsize>,
            continued: Arc<AtomicUsize>,
        }
        impl BodyFactory for YieldingHost {
            fn begin(&self) -> Box<dyn Body> {
                self.begun.fetch_add(1, Ordering::SeqCst);
                Box::new(YieldingBody {
                    started: Arc::clone(&self.started),
                    continued: Arc::clone(&self.continued),
                })
            }
        }
        impl Body for YieldingBody {
            fn resume(
                &mut self,
                input: Resume,
                _meter: &mut Meter,
            ) -> Result<Transition, BodyFailure> {
                match input {
                    Resume::Start(_) => {
                        self.started.fetch_add(1, Ordering::SeqCst);
                        Ok(Transition::Continue)
                    }
                    Resume::Again => {
                        self.continued.fetch_add(1, Ordering::SeqCst);
                        Ok(Transition::Done(Evaluation {
                            outcome: FamilyOutcome::Evaluated(Outcome::Completed(int(9))),
                            location: None,
                            losses: vec![],
                        }))
                    }
                    Resume::Child(_) => unreachable!("this body never requests a child"),
                }
            }
        }

        let acquired = acquire(&format!("{HEADER}{FUNCTIONS}"), &DependencyInput::default());
        let handle = acquired.handle();
        let selection = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "f"))
            .expect("f");
        let begun = Arc::new(AtomicUsize::new(0));
        let started = Arc::new(AtomicUsize::new(0));
        let continued = Arc::new(AtomicUsize::new(0));
        let mut registry = handle.registry();
        assert_eq!(
            registry.register(
                &selection,
                selection.signature(),
                YieldingHost {
                    begun: Arc::clone(&begun),
                    started: Arc::clone(&started),
                    continued: Arc::clone(&continued),
                },
            ),
            Ok(Admission::Accepted)
        );
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        let session = registry
            .begin_call(&selection, vec![int(3)], &environment, &mut meter)
            .expect("begin");
        let session = match session.step() {
            CheckedApplicationProgress::Runnable(session) => session,
            CheckedApplicationProgress::Terminal(_) => panic!("root entry must remain runnable"),
        };
        assert_eq!(begun.load(Ordering::SeqCst), 1);
        assert_eq!(started.load(Ordering::SeqCst), 0);
        let session = match session.step() {
            CheckedApplicationProgress::Runnable(session) => session,
            CheckedApplicationProgress::Terminal(_) => panic!("yield must remain runnable"),
        };
        assert_eq!(started.load(Ordering::SeqCst), 1);
        assert_eq!(continued.load(Ordering::SeqCst), 0);
        drop(session);
        assert_eq!(continued.load(Ordering::SeqCst), 0);
        assert_eq!(meter.admission_count(), 1);

        let result = registry
            .begin_call(&selection, vec![int(4)], &environment, &mut meter)
            .expect("second begin")
            .run()
            .expect("second run");
        assert_eq!(completed(&result), Some(9));
        assert_eq!(started.load(Ordering::SeqCst), 2);
        assert_eq!(continued.load(Ordering::SeqCst), 1);
    }

    /// Acquisition shares the exact refusal classification of compile_admitted.
    #[trace("TC-1263", "FR-485-AC-7")]
    #[test]
    fn acquisition_refusals_match_the_existing_compile_path() {
        use qsl_foundation::Setting;

        let valid = format!("{HEADER}{FUNCTIONS}");
        let same_owner = DependencyInput::new([SuppliedLibrary {
            identity: "test/same".to_owned(),
            source: SourceIdentity::new("a", "unit", "git", "2"),
            path: "same.native".to_owned(),
            bytes: HEADER.as_bytes().to_vec(),
        }])
        .expect("admissible input");
        let short: StageLimits = [(Setting::S1InputBytes, 4)].into_iter().collect();
        let above: StageLimits = [(Setting::S1InputBytes, 2 << 20)].into_iter().collect();
        let empty = DependencyInput::default();
        for (source, dependencies, stages) in [
            ("not a QSL unit", &empty, StageLimits::default()),
            (valid.as_str(), &same_owner, StageLimits::default()),
            (valid.as_str(), &empty, short),
            (valid.as_str(), &empty, above),
        ] {
            let expected = crate::compile_admitted(
                identity("unit"),
                "unit.native",
                source.as_bytes(),
                [],
                dependencies,
                stages.clone(),
                ReplayLimits::default(),
            )
            .expect_err("existing refusal");
            let actual = acquire_checked_closure(
                identity("unit"),
                "unit.native",
                source.as_bytes(),
                [],
                dependencies,
                stages,
                ReplayLimits::default(),
            )
            .err()
            .expect("facade refusal");
            assert_eq!(
                format!("{actual:?}"),
                format!("{expected:?}"),
                "source: {source}"
            );
        }
    }

    /// An absent population binding fails at Begin before any generated body runs.
    #[trace("TC-1263", "FR-485-AC-7")]
    #[test]
    fn unrecorded_population_refuses_before_application() {
        
        use quire_exact::PopulationId;

        let document = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tests/fixtures/spine-model.semantic-ir.json"
        ));
        let source = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
            profile v = \"quire.model.complete/v1\";\n\
            model M = \"acme/orders\";\n\
            function h using v(p: Population<M::Widget>[3]): Boolean pure {{ true }}\n"
        );
        let acquired = acquire_checked_closure(
            identity("unit"),
            "unit.native",
            source.as_bytes(),
            [document.as_slice()],
            &DependencyInput::default(),
            StageLimits::default(),
            ReplayLimits::default(),
        )
        .expect("model function compiles");
        let handle = acquired.handle();
        let selection = handle
            .lookup_callable(handle.package_id(), wire(&handle.package, "h"))
            .expect("h");
        let mut registry = handle.registry();
        let begun = Arc::new(AtomicUsize::new(0));
        assert_eq!(
            registry.register(
                &selection,
                selection.signature(),
                Host {
                    begun: Arc::clone(&begun),
                    child: None,
                }
            ),
            Ok(Admission::Accepted)
        );
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut meter = meter(1000);
        let result = registry.begin_call(
            &selection,
            vec![Value::Population(PopulationId::from_digest([7; 32]))],
            &environment,
            &mut meter,
        );
        assert!(matches!(
            result,
            Err(ApplicationBeginRefusal::Call(CallFailure::Input(
                InputRefusal::WrongValueKind { parameter: 0 }
            )))
        ));
        assert!(!registry.is_sealed());
        assert_eq!(meter.admission_count(), 0);
        assert_eq!(begun.load(Ordering::SeqCst), 0);
    }

    const SITE_FUNCTIONS: &str = "function f using v(x: Int[0, 9]): Int[0, 9] pure { x }\n\
        function g using v(x: Int[0, 9]): Int[0, 9] pure { f(x) }\n\
        function twice using v(x: Int[0, 9]): Int[0, 18] pure { f(x) + f(x) }\n\
        function k using v(x: Int[0, 9]): Int[0, 9] pure { l::leaf(x) }\n";
    const SITE_LIBRARY: &str = "function leaf using v(x: Int[0, 9]): Int[0, 9] pure { x }\n\
        function outer using v(x: Int[0, 9]): Int[0, 18] pure { leaf(x) + leaf(x) }\n";

    fn site_dependencies() -> DependencyInput {
        DependencyInput::new([SuppliedLibrary {
            identity: "test/lib".to_owned(),
            source: identity("lib"),
            path: "lib.native".to_owned(),
            bytes: format!("{HEADER}{SITE_LIBRARY}").into_bytes(),
        }])
        .expect("dependency input")
    }
    fn site_source() -> String {
        format!("{HEADER}import \"test/lib\" as l;\n{SITE_FUNCTIONS}")
    }

    /// Every call node of `package` with its checked ordinal-resolved sites that
    /// the facade issues for `caller`.
    fn issued(
        handle: &CheckedClosureHandle,
        package: &CheckedPackage,
        caller: &CallableSelection,
    ) -> Vec<(NestedCallOccurrence, NestedCallSite)> {
        let mut found = Vec::new();
        for node in package.graph().semantic_graph().nodes() {
            if !is_call_form(node.semantic_form()) {
                continue;
            }
            let wire = WireNodeId::from_digest(*node.key().as_bytes());
            for ordinal in 0..16 {
                let occurrence = NestedCallOccurrence::new(wire, ordinal);
                if let Ok(site) = handle.nested_call_site(caller, occurrence) {
                    found.push((occurrence, site));
                }
            }
        }
        found
    }
    /// The checked body's own call-expression locations, read without the facade.
    fn oracle(package: &CheckedPackage, callable: &CallableSelection) -> Vec<Location> {
        let mut pending = vec![
            package
                .graph()
                .function_by_identity(callable.key())
                .expect("checked caller")
                .body,
        ];
        let mut locations = Vec::new();
        while let Some(node) = pending.pop() {
            if matches!(
                node.kind(),
                NodeKind::Call { .. } | NodeKind::ImportedCall { .. }
            ) {
                locations.push(node.location().clone());
            }
            pending.extend(node.children());
        }
        locations.sort();
        locations
    }
    fn sorted(sites: &[(NestedCallOccurrence, NestedCallSite)]) -> Vec<Location> {
        let mut locations: Vec<_> = sites
            .iter()
            .map(|(_, site)| site.location().clone())
            .collect();
        locations.sort();
        locations
    }
    fn select(
        handle: &CheckedClosureHandle,
        owner: DigestRecord,
        package: &CheckedPackage,
        name: &str,
    ) -> CallableSelection {
        handle
            .lookup_callable(owner, wire(package, name))
            .expect("callable")
    }

    #[derive(Clone)]
    struct SitedHost {
        callee: Option<NodeKey>,
        site: Location,
    }
    impl BodyFactory for SitedHost {
        fn begin(&self) -> Box<dyn Body> {
            Box::new(SitedBody {
                callee: self.callee,
                site: self.site.clone(),
            })
        }
    }
    struct SitedBody {
        callee: Option<NodeKey>,
        site: Location,
    }
    impl Body for SitedBody {
        fn resume(&mut self, input: Resume, _meter: &mut Meter) -> Result<Transition, BodyFailure> {
            Ok(match (input, self.callee) {
                (Resume::Start(arguments), Some(callee)) => Transition::Call(ChildRequest {
                    callee,
                    arguments,
                    site: self.site.clone(),
                }),
                (Resume::Start(_), None) => Transition::Done(Evaluation {
                    outcome: FamilyOutcome::Evaluated(Outcome::Completed(int(7))),
                    location: None,
                    losses: vec![],
                }),
                (Resume::Child(result), _) => Transition::Done(result),
                (Resume::Again, _) => Transition::Continue,
            })
        }
    }

    /// Run `caller` with a generated body requesting `callee` at `site`, with the
    /// work limit that admits the root `function.call` and denies the nested one.
    fn denied_nested_call(
        handle: &CheckedClosureHandle,
        caller: &CallableSelection,
        callee: &CallableSelection,
        site: &Location,
    ) -> Evaluation {
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut root_only = meter(1000);
        let mut registry = handle.registry();
        assert_eq!(
            registry.register(
                caller,
                caller.signature(),
                SitedHost {
                    callee: None,
                    site: site.clone(),
                },
            ),
            Ok(Admission::Accepted)
        );
        registry
            .begin_call(caller, vec![int(3)], &environment, &mut root_only)
            .expect("begin root only")
            .run()
            .expect("root only");
        let root_spend = root_only.consumed(LimitKind::WorkUnits);
        assert!(root_spend > 0);

        let mut limited = meter(root_spend);
        let mut registry = handle.registry();
        assert_eq!(
            registry.register(
                caller,
                caller.signature(),
                SitedHost {
                    callee: Some(callee.key()),
                    site: site.clone(),
                },
            ),
            Ok(Admission::Accepted)
        );
        let result = registry
            .begin_call(caller, vec![int(3)], &environment, &mut limited)
            .expect("begin nested")
            .run()
            .expect("denied nested call");
        assert_eq!(limited.consumed(LimitKind::WorkUnits), root_spend);
        result
    }

    /// An own caller's nested own and imported calls resolve to the checked call
    /// expressions, and the leaf scheduler reports the denied nested charge there.
    #[trace("TC-1263", "FR-485-AC-9")]
    #[test]
    fn own_caller_call_sites_are_the_checked_call_expressions() {
        let acquired = acquire(&site_source(), &site_dependencies());
        let handle = acquired.handle();
        let own = handle.package_id();
        let package = Arc::clone(&handle.package);
        let f = select(handle, own, &package, "f");
        let twice = select(handle, own, &package, "twice");
        let k = select(handle, own, &package, "k");

        let sites = issued(handle, &package, &twice);
        assert_eq!(sites.len(), 2, "two nested call occurrences");
        assert_eq!(sorted(&sites), oracle(&package, &twice));
        let root = package
            .graph()
            .function_by_identity(twice.key())
            .expect("twice")
            .body
            .location()
            .clone();
        assert!(sites.iter().all(|(_, site)| site.location() != &root));
        assert_ne!(sites[0].1.location(), sites[1].1.location());
        for (_, site) in &sites {
            assert_eq!(site.owner_package_id(), own);
            assert!(!site.location().path().is_empty(), "a child path");
            assert!(matches!(
                &site.location().origin,
                quire_semantic_value::location::Origin::Body { function, index: 2 } if function == "twice"
            ));
            let result = denied_nested_call(handle, &twice, &f, site.location());
            assert!(matches!(
                result.outcome,
                FamilyOutcome::Evaluated(Outcome::Incomplete(_))
            ));
            assert_eq!(result.location.as_ref(), Some(site.location()));
        }

        let imported_calls = issued(handle, &package, &k);
        assert_eq!(imported_calls.len(), 1, "the imported call node");
        assert_eq!(sorted(&imported_calls), oracle(&package, &k));
        assert!(matches!(
            &imported_calls[0].1.location().origin,
            quire_semantic_value::location::Origin::Body { function, index: 3 } if function == "k"
        ));
    }

    /// An imported declaring caller's call site comes from the imported owner.
    #[trace("TC-1263", "FR-485-AC-10")]
    #[test]
    fn imported_caller_call_sites_come_from_the_imported_owner() {
        let acquired = acquire(&site_source(), &site_dependencies());
        let handle = acquired.handle();
        let (dependency_id, dependency) = handle
            .package
            .dependencies()
            .iter()
            .next()
            .expect("retained dependency");
        let dependency_id = dependency_id.record();
        let outer = select(handle, dependency_id, dependency, "outer");
        let leaf = select(handle, dependency_id, dependency, "leaf");

        let sites = issued(handle, dependency, &outer);
        assert_eq!(sites.len(), 2);
        assert_eq!(sorted(&sites), oracle(dependency, &outer));
        let own_functions: Vec<_> = ["f", "g", "twice", "k"]
            .iter()
            .map(|name| select(handle, handle.package_id(), &handle.package, name))
            .collect();
        for (_, site) in &sites {
            assert_eq!(site.owner_package_id(), dependency_id);
            assert_ne!(dependency_id, handle.package_id());
            assert!(matches!(
                &site.location().origin,
                quire_semantic_value::location::Origin::Body { function, index: 1 } if function == "outer"
            ));
            for own in &own_functions {
                assert!(!oracle(&handle.package, own).contains(site.location()));
            }
            let result = denied_nested_call(handle, &outer, &leaf, site.location());
            assert!(matches!(
                result.outcome,
                FamilyOutcome::Evaluated(Outcome::Incomplete(_))
            ));
            assert_eq!(result.location.as_ref(), Some(site.location()));
        }
    }

    /// Every refusal cause is distinct, typed, and made before any registry or meter change.
    #[trace("TC-1263", "FR-485-AC-11")]
    #[test]
    fn call_site_refusals_are_distinct_and_charge_nothing() {
        let acquired = acquire(&site_source(), &site_dependencies());
        let handle = acquired.handle();
        let own = handle.package_id();
        let package = Arc::clone(&handle.package);
        let (dependency_id, dependency) = package
            .dependencies()
            .iter()
            .next()
            .expect("retained dependency");
        let dependency_id = dependency_id.record();
        let g = select(handle, own, &package, "g");
        let outer = select(handle, dependency_id, dependency, "outer");
        let g_call = issued(handle, &package, &g)[0].0;
        let k = select(handle, own, &package, "k");
        let k_call = issued(handle, &package, &k)[0].0;
        let outer_call = issued(handle, dependency, &outer)[0].0;

        let second = acquire(&site_source(), &site_dependencies());
        let other = select(second.handle(), own, &second.handle().package, "g");

        let begun = Arc::new(AtomicUsize::new(0));
        let mut registry = handle.registry();
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let meter_before = meter(1000);
        let refusals = [
            handle.nested_call_site(&other, g_call),
            handle.nested_call_site(&g, outer_call),
            handle.nested_call_site(&outer, g_call),
            handle.nested_call_site(&g, k_call),
            handle.nested_call_site(&g, NestedCallOccurrence::new(wire(&package, "g"), 0)),
            handle.nested_call_site(
                &g,
                NestedCallOccurrence::new(WireNodeId::from_digest([7; 32]), 0),
            ),
            handle.nested_call_site(&g, NestedCallOccurrence::new(g_call.call_node, 99)),
        ]
        .map(|result| result.expect_err("refused"));
        assert_eq!(
            refusals,
            [
                NestedCallSiteRefusal::WrongAcquisition,
                NestedCallSiteRefusal::ForeignCall(dependency_id),
                NestedCallSiteRefusal::ForeignCall(own),
                NestedCallSiteRefusal::Unreachable,
                NestedCallSiteRefusal::WrongKind,
                NestedCallSiteRefusal::UnknownCall,
                NestedCallSiteRefusal::UnknownOccurrence,
            ]
        );
        for (index, left) in refusals.iter().enumerate() {
            assert!(refusals[index + 1..].iter().all(
                |right| right != left || matches!(left, NestedCallSiteRefusal::ForeignCall(_))
            ));
        }
        assert_eq!(meter_before.admission_count(), 0);
        assert!(!registry.is_sealed());
        assert_eq!(
            registry.register(
                &g,
                g.signature(),
                Host {
                    begun: Arc::clone(&begun),
                    child: None
                }
            ),
            Ok(Admission::Accepted)
        );
        assert_eq!(begun.load(Ordering::SeqCst), 0);
        let mut meter_after = meter(1000);
        registry
            .begin_call(&g, vec![int(3)], &environment, &mut meter_after)
            .expect("begin after refusals")
            .run()
            .expect("run");
        assert_eq!(begun.load(Ordering::SeqCst), 1);
    }

    /// The issued site does not change across registration, Begin and sealing of the same acquisition.
    #[trace("TC-1263", "FR-485-AC-12")]
    #[test]
    fn call_site_is_stable_across_registration_and_begin() {
        let acquired = acquire(&site_source(), &site_dependencies());
        let handle = acquired.handle();
        let package = Arc::clone(&handle.package);
        let own = handle.package_id();
        let g = select(handle, own, &package, "g");
        let f = select(handle, own, &package, "f");
        let (occurrence, before) = issued(handle, &package, &g).remove(0);
        let mut registry = handle.registry();
        assert_eq!(
            registry.register(
                &g,
                g.signature(),
                SitedHost {
                    callee: Some(f.key()),
                    site: before.location().clone(),
                },
            ),
            Ok(Admission::Accepted)
        );
        let registered = handle
            .nested_call_site(&g, occurrence)
            .expect("after register");
        let environment = ApplicationEnvironment::new(ObjectClosure::default());
        let mut work = meter(1000);
        let session = registry
            .begin_call(&g, vec![int(3)], &environment, &mut work)
            .expect("begin");
        assert!(registry.is_sealed());
        let begun = handle
            .nested_call_site(&g, occurrence)
            .expect("after begin");
        let cloned = handle
            .clone()
            .nested_call_site(&g, occurrence)
            .expect("clone");
        drop(session);
        for later in [&registered, &begun, &cloned] {
            assert_eq!(later.location(), before.location());
            assert_eq!(later.owner_package_id(), before.owner_package_id());
        }
    }

    /// A call node several other packages own is never attributed to one of them.
    /// Node keys are owner-scoped, and two dependencies sharing an owner are refused
    /// at acquisition, so no real closure holds one node in two packages; the branch
    /// is therefore tested on its pure decision.
    #[test]
    fn a_call_node_in_several_other_packages_is_unknown() {
        let first = DigestRecord::mint(
            qsl_foundation::digest::DigestDomain::PackageSemanticV2,
            [1; 32],
        );
        let second = DigestRecord::mint(
            qsl_foundation::digest::DigestDomain::PackageSemanticV2,
            [2; 32],
        );
        assert_eq!(
            foreign_or_unknown(&[first]),
            NestedCallSiteRefusal::ForeignCall(first)
        );
        assert_eq!(
            foreign_or_unknown(&[first, second]),
            NestedCallSiteRefusal::UnknownCall
        );
        assert_eq!(foreign_or_unknown(&[]), NestedCallSiteRefusal::UnknownCall);
    }

    #[test]
    fn reachable_composites_follows_map_values_and_treats_bounded_quantity_as_a_leaf() {
        let source = format!(
            "{HEADER}tuple Pair(Boolean, Integer);\nfunction consume using v(x: Pair): Boolean pure {{ true }}\n"
        );
        let acquired = acquire(&source, &DependencyInput::default());
        let handle = acquired.handle();
        let graph = handle.package.graph();
        let pair = graph
            .checked_type_nodes()
            .find_map(|type_node| match type_node {
                qsl_semantics::check::CheckedTypeNode::Composite { declaration, .. } => {
                    Some(*declaration)
                }
                _ => None,
            })
            .expect("the Pair composite declaration");
        let map = ValueType::Map(std::sync::Arc::new(quire_exact::MapType::new(
            ValueType::Composite(pair),
            None,
        )));
        let bounded = ValueType::BoundedQuantity(std::sync::Arc::new(
            quire_exact::BoundedQuantityType::new(
                quire_exact::UnitId::compound([5; 32]),
                None,
                None,
            )
            .expect("an unbounded quantity in the unit"),
        ));
        let through_map =
            reachable_composites(graph, &Signature::new(vec![map], ValueType::Boolean))
                .expect("a map of a checked composite is consistent");
        assert_eq!(through_map, BTreeSet::from([pair]));
        let leaf = reachable_composites(graph, &Signature::new(vec![bounded], ValueType::Boolean))
            .expect("a bounded quantity is a leaf");
        assert!(leaf.is_empty());
    }

    #[trace("TC-1263", "FR-485-AC-13", "FR-485-AC-14")]
    #[test]
    fn nominal_declaration_views_preserve_checked_members_and_identity() {
        use crate::{NominalDeclarationForm, NominalDeclarationMember};
        use quire_exact::{Presence, ValueType};
        use quire_semantic_value::declaration::{
            CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
            UnionMemberDeclaration,
        };

        let source = format!(
            "{HEADER}tuple Pair(Boolean, Integer);\nunion Shape {{ Empty, Circle(Integer), Boxed(Pair, Boolean) }}\nrecord Holder {{ flag: Boolean; pair: Pair; shape: Shape; }}\nfunction consume using v(x: Holder): Boolean pure {{ true }}\n"
        );
        let acquired = acquire(&source, &DependencyInput::default());
        let handle = acquired.handle();
        let graph = handle.package.graph();
        let selection = handle
            .lookup_callable(handle.package_id, wire(&handle.package, "consume"))
            .expect("the function selection is checked");
        let composite_id = |name: &str| {
            let node = graph
                .semantic_graph()
                .nodes()
                .find(|node| {
                    node.declaration()
                        .and_then(|name| name.last())
                        .is_some_and(|segment| segment.as_str() == name)
                })
                .expect("composite node");
            WireNodeId::from_digest(*node.key().as_bytes())
        };

        let holder_id = composite_id("Holder");
        let holder = handle
            .nominal_declaration(&selection, holder_id)
            .expect("the signature's record declaration is available");
        assert_eq!(holder.package_id(), handle.package_id());
        assert_eq!(holder.node().as_bytes(), holder_id.as_bytes());
        assert_eq!(holder.form(), NominalDeclarationForm::Record);
        assert_eq!(holder.qualified_name().last().unwrap().as_str(), "Holder");
        let [NominalDeclarationMember::RecordField {
            identifier: flag,
            value_type: ValueType::Boolean,
        }, NominalDeclarationMember::RecordField {
            identifier: pair_name,
            value_type: ValueType::Composite(pair_key),
        }, NominalDeclarationMember::RecordField {
            identifier: shape_name,
            value_type: ValueType::Composite(_),
        }] = holder.members()
        else {
            panic!("record members preserve their checked declaration order and types");
        };
        assert_eq!(flag.as_str(), "flag");
        assert_eq!(pair_name.as_str(), "pair");
        assert_eq!(shape_name.as_str(), "shape");

        let pair_id = composite_id("Pair");
        let pair = handle
            .nominal_declaration(&selection, pair_id)
            .expect("the nested tuple is reachable through Holder");
        assert_eq!(pair.form(), NominalDeclarationForm::Tuple);
        assert_eq!(
            pair.members(),
            &[
                NominalDeclarationMember::TuplePosition {
                    position: 0,
                    value_type: ValueType::Boolean,
                },
                NominalDeclarationMember::TuplePosition {
                    position: 1,
                    value_type: ValueType::Integer,
                },
            ]
        );

        let shape_id = composite_id("Shape");
        let shape = handle
            .nominal_declaration(&selection, shape_id)
            .expect("the nested union is reachable through Holder");
        assert_eq!(shape.form(), NominalDeclarationForm::Union);
        let [NominalDeclarationMember::UnionMember {
            member: empty_member,
            variant: empty_variant,
            payload: empty_payload,
        }, NominalDeclarationMember::UnionMember {
            member: circle_member,
            variant: circle_variant,
            payload: circle_payload,
        }, NominalDeclarationMember::UnionMember {
            member: boxed_member,
            variant: boxed_variant,
            payload: boxed_payload,
        }] = shape.members()
        else {
            panic!("union view retains each sealed member in declared order");
        };
        assert_eq!(empty_member.identifier().as_str(), "Empty");
        assert_eq!(empty_member.variant(), *empty_variant);
        assert!(empty_payload.is_empty());
        assert_eq!(circle_member.identifier().as_str(), "Circle");
        assert_eq!(circle_member.variant(), *circle_variant);
        assert_eq!(circle_payload, &[ValueType::Integer]);
        assert_eq!(boxed_member.identifier().as_str(), "Boxed");
        assert_eq!(boxed_member.variant(), *boxed_variant);
        assert_eq!(
            boxed_payload,
            &[ValueType::Composite(pair.key()), ValueType::Boolean]
        );

        // The returned checked record and tuple data is directly usable by the
        // nominal environment constructor, without inventing member data.
        let types = TypeEnvironment::new(
            [
                CompositeDeclaration::new(
                    pair.key(),
                    "Pair",
                    CompositeShape::Tuple(
                        pair.members()
                            .iter()
                            .map(|member| match member {
                                NominalDeclarationMember::TuplePosition { value_type, .. } => {
                                    value_type.clone()
                                }
                                _ => unreachable!("tuple view has tuple members"),
                            })
                            .collect(),
                    ),
                ),
                CompositeDeclaration::new(
                    holder.key(),
                    "Holder",
                    CompositeShape::Record(
                        holder
                            .members()
                            .iter()
                            .map(|member| match member {
                                NominalDeclarationMember::RecordField {
                                    identifier,
                                    value_type,
                                } => FieldDeclaration::new(
                                    identifier.as_str(),
                                    value_type.clone(),
                                    Presence::Required,
                                ),
                                _ => unreachable!("record view has record members"),
                            })
                            .collect(),
                    ),
                ),
                CompositeDeclaration::new(
                    shape.key(),
                    "Shape",
                    CompositeShape::Union(
                        shape
                            .members()
                            .iter()
                            .map(|member| {
                                let NominalDeclarationMember::UnionMember {
                                    member, payload, ..
                                } = member
                                else {
                                    unreachable!("union view has union members")
                                };
                                UnionMemberDeclaration::resolved(
                                    member.identifier().clone(),
                                    payload.clone(),
                                )
                            })
                            .collect(),
                    ),
                ),
            ],
            [],
        )
        .expect("checked facade data constructs a nominal environment");
        let mut types = types;
        types
            .seal_union_verified(
                shape.key(),
                shape.node(),
                shape
                    .members()
                    .iter()
                    .map(|member| {
                        let NominalDeclarationMember::UnionMember { member, .. } = member else {
                            unreachable!("union view has union members")
                        };
                        member.clone()
                    })
                    .collect(),
            )
            .expect("the retained union bindings seal the declared union");
        assert_eq!(types.composite(holder.key()).unwrap().name(), "Holder");
        assert_eq!(types.composite(shape.key()).unwrap().name(), "Shape");
        assert_eq!(*pair_key, pair.key());
        assert_eq!(holder.members().len(), 3);
    }

    #[trace("TC-1263", "FR-485-AC-13", "FR-485-AC-14")]
    #[test]
    fn nominal_declaration_views_are_acquisition_and_owner_bound() {
        let dependency = SuppliedLibrary {
            identity: "test/nominal".to_owned(),
            source: identity("nominal-lib"),
            path: "nominal-lib.native".to_owned(),
            bytes: format!(
                "{HEADER}record Imported {{ flag: Boolean; }}\nfunction imported using v(x: Imported): Boolean pure {{ true }}\n"
            )
            .into_bytes(),
        };
        let dependencies = DependencyInput::new([dependency]).expect("dependency");
        let source = format!(
            "{HEADER}import \"test/nominal\" as n;\nrecord OwnData {{ datum: Integer; }}\nfunction own_fn using v(x: OwnData): Boolean pure {{ true }}\n"
        );
        let acquired = acquire(&source, &dependencies);
        let handle = acquired.handle();
        let own_selection = handle
            .lookup_callable(handle.package_id, wire(&handle.package, "own_fn"))
            .expect("own callable");
        let own_id = handle
            .package
            .graph()
            .semantic_graph()
            .nodes()
            .find(|node| {
                node.declaration()
                    .and_then(|name| name.last())
                    .is_some_and(|name| name.as_str() == "OwnData")
            })
            .map(|node| WireNodeId::from_digest(*node.key().as_bytes()))
            .expect("own composite");
        assert!(handle.nominal_declaration(&own_selection, own_id).is_ok());

        let (dependency_id, dependency) = handle
            .package
            .dependencies()
            .iter()
            .next()
            .expect("dependency package");
        let dependency_record = dependency_id.record();
        let imported_node = dependency
            .graph()
            .semantic_graph()
            .nodes()
            .find(|node| {
                node.declaration()
                    .and_then(|name| name.last())
                    .is_some_and(|name| name.as_str() == "Imported")
            })
            .map(|node| WireNodeId::from_digest(*node.key().as_bytes()))
            .expect("imported composite");
        assert_eq!(
            handle.nominal_declaration(&own_selection, imported_node),
            Err(NominalDeclarationRefusal::Foreign(dependency_record))
        );
        let imported_selection = handle
            .lookup_callable(dependency_record, wire(dependency, "imported"))
            .expect("imported callable selection");
        let imported_view = handle
            .nominal_declaration(&imported_selection, imported_node)
            .expect("imported owner's declaration");
        assert_eq!(imported_view.package_id(), dependency_record);
        assert_ne!(imported_view.package_id(), handle.package_id());

        let function_node = wire(&handle.package, "own_fn");
        assert_eq!(
            handle.nominal_declaration(&own_selection, function_node),
            Err(NominalDeclarationRefusal::WrongKind)
        );
        assert_eq!(
            handle.nominal_declaration(&own_selection, WireNodeId::from_digest([0xA5; 32]),),
            Err(NominalDeclarationRefusal::Unknown)
        );
        let other_acquisition = acquire(&source, &dependencies);
        assert_eq!(
            other_acquisition
                .handle()
                .nominal_declaration(&own_selection, own_id),
            Err(NominalDeclarationRefusal::WrongAcquisition)
        );
    }

    #[trace("TC-1263", "FR-485-AC-14")]
    #[test]
    fn nominal_declaration_rejects_inconsistent_checked_form() {
        assert_eq!(
            nominal_declaration_form(
                qsl_semantics::check::CompositeForm::Union,
                &CompositeShape::Record(vec![]),
            ),
            Err(NominalDeclarationRefusal::Inconsistent)
        );
        assert_eq!(
            nominal_declaration_form(
                qsl_semantics::check::CompositeForm::Record,
                &CompositeShape::Tuple(vec![]),
            ),
            Err(NominalDeclarationRefusal::Inconsistent)
        );
        assert_eq!(
            nominal_declaration_form(
                qsl_semantics::check::CompositeForm::Tuple,
                &CompositeShape::Union(vec![]),
            ),
            Err(NominalDeclarationRefusal::Inconsistent)
        );
    }
}

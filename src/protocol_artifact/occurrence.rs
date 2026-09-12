// SPDX-License-Identifier: AGPL-3.0-only
//! FR-048: bounded runtime occurrence-key schemas over admitted protocol graphs.
//!
//! This module projects static key dimensions only. Concrete workflow identities
//! and repeat ordinals remain caller-owned runtime inputs.

use super::work::Work;
use super::{
    report, wire as w, AdmittedPackage, Dimension, Error, ExactInteger, Invalid, Limits,
    ProtocolNumber, Report,
};

/// Opaque read-only projection shared by version-exact admitted package types.
///
/// Callers cannot construct this view from freely decoded wire data. Version 2
/// can add its own crate-owned `From` implementation without changing this API
/// or the inherited protocol graph projection.
#[derive(Clone, Copy, Debug)]
pub struct AdmittedProtocolView<'a> {
    package: &'a w::Package,
}

impl<'a> From<&'a AdmittedPackage> for AdmittedProtocolView<'a> {
    fn from(value: &'a AdmittedPackage) -> Self {
        Self {
            package: value.package(),
        }
    }
}

/// One authored role slot and its separately supplied future-instance binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoleSlotSchema {
    role: w::Handle,
    instance_binding: u32,
    locus: w::Locus,
}

impl RoleSlotSchema {
    /// Declaration-local authored role handle.
    pub fn role(&self) -> &w::Handle {
        &self.role
    }

    /// Binding requirement that a consumer must satisfy with a role instance.
    pub fn instance_binding(&self) -> u32 {
        self.instance_binding
    }

    /// Original native source locus of the role declaration.
    pub fn locus(&self) -> &w::Locus {
        &self.locus
    }
}

/// Static role ownership of an authored control node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NodeRole {
    /// Structural orchestration has no invented participant owner.
    Structural,
    /// The source assigns this exact declaration-local role slot.
    Role(w::Handle),
}

/// One enclosing repeat dimension in a runtime occurrence key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepeatOrdinalSchema {
    /// A node in the repeat body uses an ordinal in `[0, upper_exclusive)`.
    Iteration {
        /// Authored repeat control introducing this dimension.
        repeat: w::Handle,
        /// Exact authored maximum; zero deliberately describes an empty domain.
        upper_exclusive: ExactInteger,
    },
    /// A node in the exhaustion branch uses the exact exhausted ordinal.
    Exhaustion {
        /// Authored repeat control introducing this dimension.
        repeat: w::Handle,
        /// Exact authored maximum at which exhaustion is selected.
        value: ExactInteger,
    },
}

/// Static key dimensions for one authored protocol control occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeOccurrenceSchema {
    node: w::Handle,
    original_node: u32,
    locus: w::Locus,
    role: NodeRole,
    repeats: Vec<RepeatOrdinalSchema>,
}

impl NodeOccurrenceSchema {
    /// Declaration-local authored control handle.
    pub fn node(&self) -> &w::Handle {
        &self.node
    }

    /// Original source arena ordinal retained by native emission.
    pub fn original_node(&self) -> u32 {
        self.original_node
    }

    /// Original native source locus of the control.
    pub fn locus(&self) -> &w::Locus {
        &self.locus
    }

    /// Exact source-owned role slot, or explicit structural ownership.
    pub fn role(&self) -> &NodeRole {
        &self.role
    }

    /// Enclosing repeat dimensions ordered from outermost to innermost.
    pub fn repeats(&self) -> &[RepeatOrdinalSchema] {
        &self.repeats
    }
}

/// Complete runtime occurrence-key schema for one admitted protocol declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OccurrenceKeySchema {
    declaration: u32,
    workflow_binding: u32,
    roles: Vec<RoleSlotSchema>,
    nodes: Vec<NodeOccurrenceSchema>,
}

impl OccurrenceKeySchema {
    /// Selected protocol declaration index.
    pub fn declaration(&self) -> u32 {
        self.declaration
    }

    /// Static requirement a consumer must satisfy with a workflow identity.
    pub fn workflow_binding(&self) -> u32 {
        self.workflow_binding
    }

    /// Authored role slots in their admitted table order.
    pub fn roles(&self) -> &[RoleSlotSchema] {
        &self.roles
    }

    /// Every authored control node in its admitted table order.
    pub fn nodes(&self) -> &[NodeOccurrenceSchema] {
        &self.nodes
    }

    /// Validate caller-owned runtime dimensions and retain them as one key.
    ///
    /// No workflow identity or ordinal is defaulted, generated, or read from the
    /// static artifact.
    pub fn bind<'a>(
        &self,
        workflow: WorkflowInstanceIdentity<'a>,
        node: &w::Handle,
        role: NodeRole,
        repeat_ordinals: &'a [ExactInteger],
    ) -> Result<OccurrenceKey<'a>, OccurrenceKeyError> {
        if node.declaration != self.declaration {
            return Err(OccurrenceKeyError::UnknownNode { node: node.clone() });
        }
        let index = usize::try_from(node.index)
            .map_err(|_| OccurrenceKeyError::UnknownNode { node: node.clone() })?;
        let selected = self
            .nodes
            .get(index)
            .filter(|selected| selected.node == *node)
            .ok_or_else(|| OccurrenceKeyError::UnknownNode { node: node.clone() })?;
        if selected.role != role {
            return Err(OccurrenceKeyError::WrongRole {
                expected: selected.role.clone(),
                actual: role,
            });
        }
        if selected.repeats.len() != repeat_ordinals.len() {
            return Err(OccurrenceKeyError::RepeatCount {
                expected: selected.repeats.len(),
                actual: repeat_ordinals.len(),
            });
        }
        for (dimension, (schema, ordinal)) in
            selected.repeats.iter().zip(repeat_ordinals).enumerate()
        {
            let value = ordinal.value();
            let valid = match schema {
                RepeatOrdinalSchema::Iteration {
                    upper_exclusive, ..
                } => value >= 0 && value < upper_exclusive.value(),
                RepeatOrdinalSchema::Exhaustion {
                    value: exhausted, ..
                } => value == exhausted.value(),
            };
            if !valid {
                return Err(OccurrenceKeyError::RepeatOrdinal {
                    dimension,
                    value: *ordinal,
                });
            }
        }
        Ok(OccurrenceKey {
            declaration: self.declaration,
            workflow,
            node: node.clone(),
            role: selected.role.clone(),
            repeat_ordinals,
        })
    }
}

/// Borrowed, nonempty workflow-instance identity supplied by a runtime caller.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WorkflowInstanceIdentity<'a>(&'a str);

impl<'a> WorkflowInstanceIdentity<'a> {
    /// Retain one caller-owned identity without interpreting its namespace.
    pub fn new(value: &'a str) -> Result<Self, OccurrenceKeyError> {
        if value.is_empty() {
            return Err(OccurrenceKeyError::EmptyWorkflowIdentity);
        }
        if value.len() > 4_096 {
            return Err(OccurrenceKeyError::WorkflowIdentityTooLong { bytes: value.len() });
        }
        Ok(Self(value))
    }

    /// Exact caller-supplied identity spelling.
    pub fn as_str(self) -> &'a str {
        self.0
    }
}

/// One validated runtime key borrowing every concrete caller-supplied value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OccurrenceKey<'a> {
    declaration: u32,
    workflow: WorkflowInstanceIdentity<'a>,
    node: w::Handle,
    role: NodeRole,
    repeat_ordinals: &'a [ExactInteger],
}

impl<'a> OccurrenceKey<'a> {
    /// Static protocol declaration component.
    pub fn declaration(&self) -> u32 {
        self.declaration
    }

    /// Concrete workflow identity supplied by the caller.
    pub fn workflow(&self) -> WorkflowInstanceIdentity<'a> {
        self.workflow
    }

    /// Exact authored node component.
    pub fn node(&self) -> &w::Handle {
        &self.node
    }

    /// Exact role-slot component, including explicit structural ownership.
    pub fn role(&self) -> &NodeRole {
        &self.role
    }

    /// Caller-supplied ordinals validated against outer-to-inner dimensions.
    pub fn repeat_ordinals(&self) -> &'a [ExactInteger] {
        self.repeat_ordinals
    }
}

/// Typed refusal for caller-supplied runtime key components.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum OccurrenceKeyError {
    /// Runtime workflow identity cannot be omitted.
    #[error("workflow instance identity is empty")]
    EmptyWorkflowIdentity,
    /// Runtime identity exceeds the selected protocol name bound.
    #[error("workflow instance identity is {bytes} bytes; maximum is 4096")]
    WorkflowIdentityTooLong {
        /// Offered UTF-8 byte length.
        bytes: usize,
    },
    /// The key selected no node in this declaration's schema.
    #[error("node is not present in the selected occurrence schema")]
    UnknownNode {
        /// Offered declaration-local handle.
        node: w::Handle,
    },
    /// The key substituted a different role owner.
    #[error("node role does not match the admitted schema")]
    WrongRole {
        /// Source-owned role requirement.
        expected: NodeRole,
        /// Caller-offered role requirement.
        actual: NodeRole,
    },
    /// The key omitted or added an enclosing repeat dimension.
    #[error("repeat ordinal count is {actual}; expected {expected}")]
    RepeatCount {
        /// Required enclosing repeat count.
        expected: usize,
        /// Offered ordinal count.
        actual: usize,
    },
    /// One ordinal lies outside its iteration or exhaustion domain.
    #[error("repeat ordinal at dimension {dimension} is outside its admitted domain")]
    RepeatOrdinal {
        /// Outer-to-inner dimension index.
        dimension: usize,
        /// Exact refused signed-64 ordinal.
        value: ExactInteger,
    },
}

#[derive(Clone, Copy)]
enum RepeatArm {
    Iteration(ExactInteger),
    Exhaustion(ExactInteger),
}

#[derive(Clone, Copy)]
struct Parent {
    node: usize,
    repeat: Option<RepeatArm>,
}

/// Derive a bounded static key schema from an admitted version-exact package.
///
/// The input conversion is crate-owned: freely decoded wire data cannot create
/// an [`AdmittedProtocolView`].
pub fn occurrence_key_schema<'a>(
    package: impl Into<AdmittedProtocolView<'a>>,
    declaration: u32,
    limits: Limits,
) -> Report<OccurrenceKeySchema> {
    let mut work = Work::new(limits);
    let view = package.into();
    let result = project(view.package, declaration, &mut work);
    report(work, result)
}

fn project(
    package: &w::Package,
    declaration_index: u32,
    work: &mut Work,
) -> Result<OccurrenceKeySchema, Error> {
    work.visit()?;
    let owner =
        usize::try_from(declaration_index).map_err(|_| Error::Invalid(Invalid::Reference))?;
    let declaration = package
        .declarations
        .get(owner)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    work.locus = Some(declaration.locus.clone());
    let w::Body::Protocol {
        activation,
        roles,
        channels,
        controls,
        run,
        ..
    } = &declaration.body
    else {
        return Err(Error::Invalid(Invalid::Owner));
    };

    let workflow_binding = workflow_binding(declaration, activation, owner, work)?;
    let role_slots = role_slots(roles, &declaration.bindings, owner, work)?;
    let parents = parents(controls, run, owner, work)?;
    let mut nodes = Vec::new();
    work.charge(Dimension::Entries, controls.len())?;
    nodes
        .try_reserve_exact(controls.len())
        .map_err(|_| Error::Allocation)?;
    for (index, control) in controls.iter().enumerate() {
        work.visit()?;
        work.locus = Some(control.locus.clone());
        let node = handle(owner, index)?;
        let role = node_role(
            controls,
            channels,
            roles.len(),
            &control.operation,
            owner,
            work,
        )?;
        let repeats = repeat_dimensions(&parents, index, owner, work)?;
        nodes.push(NodeOccurrenceSchema {
            node,
            original_node: control.original_node,
            locus: control.locus.clone(),
            role,
            repeats,
        });
    }
    Ok(OccurrenceKeySchema {
        declaration: declaration_index,
        workflow_binding,
        roles: role_slots,
        nodes,
    })
}

fn workflow_binding(
    declaration: &w::Declaration,
    activation: &w::Activation,
    owner: usize,
    work: &mut Work,
) -> Result<u32, Error> {
    work.visit()?;
    let anchor = match activation {
        w::Activation::Origin { anchor } | w::Activation::Each { anchor, .. } => anchor,
    };
    let anchor_index = local(anchor, owner, declaration.anchors.len())?;
    let binding = declaration.anchors[anchor_index]
        .binding
        .0
        .ok_or(Error::Invalid(Invalid::Binding))?;
    let requirement = declaration
        .bindings
        .get(usize::try_from(binding).map_err(|_| Error::Invalid(Invalid::Reference))?)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    if requirement.kind != w::BindingKind::WorkflowInstance {
        return Err(Error::Invalid(Invalid::Binding));
    }
    Ok(binding)
}

fn role_slots(
    roles: &[w::Role],
    bindings: &[w::BindingRequirement],
    owner: usize,
    work: &mut Work,
) -> Result<Vec<RoleSlotSchema>, Error> {
    work.charge(Dimension::Entries, roles.len())?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(roles.len())
        .map_err(|_| Error::Allocation)?;
    for (index, role) in roles.iter().enumerate() {
        work.visit()?;
        work.locus = Some(role.locus.clone());
        let role_handle = handle(owner, index)?;
        let binding = bindings
            .get(usize::try_from(role.instance).map_err(|_| Error::Invalid(Invalid::Reference))?)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        if binding.kind != w::BindingKind::RoleInstance
            || !matches!(&binding.subject, w::Subject::Role { role } if role == &role_handle)
        {
            return Err(Error::Invalid(Invalid::Binding));
        }
        result.push(RoleSlotSchema {
            role: role_handle,
            instance_binding: role.instance,
            locus: role.locus.clone(),
        });
    }
    Ok(result)
}

fn parents(
    controls: &[w::Control],
    run: &w::Handle,
    owner: usize,
    work: &mut Work,
) -> Result<Vec<Option<Parent>>, Error> {
    local(run, owner, controls.len())?;
    work.charge(Dimension::Entries, controls.len())?;
    let mut result = Vec::new();
    result
        .try_reserve_exact(controls.len())
        .map_err(|_| Error::Allocation)?;
    result.resize(controls.len(), None);
    for (parent, control) in controls.iter().enumerate() {
        work.visit()?;
        match &control.operation {
            w::ControlOperation::Sequence { children } => {
                for child in children {
                    set_parent(&mut result, child, owner, parent, None, work)?;
                }
            }
            w::ControlOperation::Choice { cases, .. } => {
                for case in cases {
                    set_parent(&mut result, &case.body, owner, parent, None, work)?;
                }
            }
            w::ControlOperation::Parallel { branches, .. } => {
                for branch in branches {
                    set_parent(&mut result, &branch.body, owner, parent, None, work)?;
                }
            }
            w::ControlOperation::Repeat {
                maximum,
                body,
                exhausted,
                ..
            } => {
                let maximum = exact_integer(maximum)?;
                set_parent(
                    &mut result,
                    body,
                    owner,
                    parent,
                    Some(RepeatArm::Iteration(maximum)),
                    work,
                )?;
                set_parent(
                    &mut result,
                    exhausted,
                    owner,
                    parent,
                    Some(RepeatArm::Exhaustion(maximum)),
                    work,
                )?;
            }
            w::ControlOperation::Await {
                event,
                then_body,
                timeout,
                ..
            } => {
                for child in [event, then_body, timeout] {
                    set_parent(&mut result, child, owner, parent, None, work)?;
                }
            }
            w::ControlOperation::Event { .. }
            | w::ControlOperation::Check { .. }
            | w::ControlOperation::Commit { .. } => {}
        }
    }
    let root = local(run, owner, controls.len())?;
    if result.get(root).and_then(|parent| *parent).is_some() {
        return Err(Error::Invalid(Invalid::Owner));
    }
    for (index, parent) in result.iter().enumerate() {
        work.visit()?;
        if index != root && parent.is_none() {
            return Err(Error::Invalid(Invalid::Owner));
        }
    }
    Ok(result)
}

fn set_parent(
    parents: &mut [Option<Parent>],
    child: &w::Handle,
    owner: usize,
    parent: usize,
    repeat: Option<RepeatArm>,
    work: &mut Work,
) -> Result<(), Error> {
    work.visit()?;
    let child = local(child, owner, parents.len())?;
    let slot = parents
        .get_mut(child)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    if slot
        .replace(Parent {
            node: parent,
            repeat,
        })
        .is_some()
    {
        return Err(Error::Invalid(Invalid::Owner));
    }
    Ok(())
}

fn repeat_dimensions(
    parents: &[Option<Parent>],
    mut node: usize,
    owner: usize,
    work: &mut Work,
) -> Result<Vec<RepeatOrdinalSchema>, Error> {
    let mut reversed = Vec::new();
    let mut depth = 0usize;
    while let Some(parent) = parents
        .get(node)
        .ok_or(Error::Invalid(Invalid::Reference))?
    {
        work.visit()?;
        depth = depth
            .checked_add(1)
            .ok_or(Error::Invalid(Invalid::StructuralInteger))?;
        if depth > parents.len() {
            return Err(Error::Invalid(Invalid::Cycle));
        }
        work.charge(Dimension::Depth, depth)?;
        if let Some(repeat) = parent.repeat {
            work.charge(Dimension::Entries, 1)?;
            reversed.try_reserve(1).map_err(|_| Error::Allocation)?;
            let repeat_handle = handle(owner, parent.node)?;
            reversed.push(match repeat {
                RepeatArm::Iteration(upper_exclusive) => RepeatOrdinalSchema::Iteration {
                    repeat: repeat_handle,
                    upper_exclusive,
                },
                RepeatArm::Exhaustion(value) => RepeatOrdinalSchema::Exhaustion {
                    repeat: repeat_handle,
                    value,
                },
            });
        }
        node = parent.node;
    }
    reversed.reverse();
    Ok(reversed)
}

fn node_role(
    controls: &[w::Control],
    channels: &[w::Channel],
    roles: usize,
    operation: &w::ControlOperation,
    owner: usize,
    work: &mut Work,
) -> Result<NodeRole, Error> {
    work.visit()?;
    let role = match operation {
        w::ControlOperation::Choice { owner, .. }
        | w::ControlOperation::Repeat { owner, .. }
        | w::ControlOperation::Commit { owner, .. } => Some(owner.clone()),
        w::ControlOperation::Event { event, .. } => match event {
            w::Event::Send { channel } => {
                let channel = channels
                    .get(local(channel, owner, channels.len())?)
                    .ok_or(Error::Invalid(Invalid::Reference))?;
                Some(channel.from.clone())
            }
            w::Event::Receive { channel, .. } => {
                let channel = channels
                    .get(local(channel, owner, channels.len())?)
                    .ok_or(Error::Invalid(Invalid::Reference))?;
                Some(channel.to.clone())
            }
            w::Event::Attempt { owner, .. } | w::Event::Event { owner, .. } => Some(owner.clone()),
            w::Event::Effect { attempt, .. } => {
                let attempt_index = local(attempt, owner, controls.len())?;
                let attempt = controls
                    .get(attempt_index)
                    .ok_or(Error::Invalid(Invalid::Reference))?;
                let w::ControlOperation::Event {
                    event: w::Event::Attempt { owner, .. },
                    ..
                } = &attempt.operation
                else {
                    return Err(Error::Invalid(Invalid::Control));
                };
                Some(owner.clone())
            }
        },
        w::ControlOperation::Sequence { .. }
        | w::ControlOperation::Parallel { .. }
        | w::ControlOperation::Await { .. }
        | w::ControlOperation::Check { .. } => None,
    };
    if let Some(role) = role {
        local(&role, owner, roles)?;
        Ok(NodeRole::Role(role))
    } else {
        Ok(NodeRole::Structural)
    }
}

fn exact_integer(value: &w::Integer) -> Result<ExactInteger, Error> {
    match value.checked().map_err(Error::Numeric)? {
        ProtocolNumber::Integer(value) => Ok(value),
        ProtocolNumber::Rational(_) => Err(Error::Invalid(Invalid::WrongNumericKind)),
    }
}

fn handle(owner: usize, index: usize) -> Result<w::Handle, Error> {
    Ok(w::Handle {
        declaration: u32::try_from(owner)
            .map_err(|_| Error::Invalid(Invalid::StructuralInteger))?,
        index: u32::try_from(index).map_err(|_| Error::Invalid(Invalid::StructuralInteger))?,
    })
}

fn local(value: &w::Handle, owner: usize, count: usize) -> Result<usize, Error> {
    let declaration =
        usize::try_from(value.declaration).map_err(|_| Error::Invalid(Invalid::Reference))?;
    let index = usize::try_from(value.index).map_err(|_| Error::Invalid(Invalid::Reference))?;
    if declaration != owner || index >= count {
        return Err(Error::Invalid(Invalid::Reference));
    }
    Ok(index)
}

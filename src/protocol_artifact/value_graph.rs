// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: original value dependencies shared by native emission and wire admission.

use super::{validate, wire::*, work::Work, Dimension, Error, Invalid};

/// Bound to one declaration; it cannot silently retain another owner's edges.
pub(super) struct ValueGraph<'a> {
    pub owner: usize,
    pub declaration: &'a Declaration,
    edges: Vec<Vec<usize>>,
}

impl<'a> ValueGraph<'a> {
    pub fn new(owner: usize, declaration: &'a Declaration, work: &mut Work) -> Result<Self, Error> {
        work.charge(Dimension::Entries, declaration.values.len())?;
        let mut graph = Self {
            owner,
            declaration,
            edges: vec![Vec::new(); declaration.values.len()],
        };
        for (index, value) in declaration.values.iter().enumerate() {
            work.locus = Some(value.locus.clone());
            work.visit()?;
            match &value.origin {
                Origin::Independent {} => {}
                Origin::Anchor { anchor } => {
                    graph.anchor(anchor, work)?;
                }
                Origin::Selected { value } => {
                    let target = graph.local(value, declaration.values.len(), work)?;
                    // A self-selection records mixed provenance, not evaluation.
                    if target != index {
                        graph.push(index, target, work)?;
                    }
                }
            }
            let mut child = |handle: &Handle, work: &mut Work| {
                let target = graph.local(handle, declaration.values.len(), work)?;
                graph.push(index, target, work)
            };
            match &value.operation {
                ValueOperation::Boolean { .. }
                | ValueOperation::Number { .. }
                | ValueOperation::Text { .. }
                | ValueOperation::Enum { .. } => {}
                ValueOperation::Read { binder } => {
                    // Resolve the authored initializer even when its syntax is
                    // outside the expression that reads the immutable binder.
                    let at = local(owner, binder, declaration.binders.len(), work)?;
                    let binder = declaration
                        .binders
                        .get(at)
                        .ok_or(Error::Invalid(Invalid::Reference))?;
                    if let Some(initializer) = &binder.initializer.0 {
                        child(initializer, work)?;
                    }
                }
                ValueOperation::Group { value }
                | ValueOperation::Unary { value, .. }
                | ValueOperation::Pre { value, .. } => child(value, work)?,
                ValueOperation::Field { base, .. } => child(base, work)?,
                ValueOperation::Binary { left, right, .. } => {
                    child(left, work)?;
                    child(right, work)?;
                }
                ValueOperation::If {
                    condition,
                    then_value,
                    else_value,
                } => {
                    child(condition, work)?;
                    child(then_value, work)?;
                    child(else_value, work)?;
                }
                ValueOperation::Let {
                    initializer, body, ..
                } => {
                    child(initializer, work)?;
                    child(body, work)?;
                }
                ValueOperation::Call { arguments, .. } => {
                    // Arguments belong to this declaration; the callee's own
                    // graph does not supply a caller observation or capture.
                    for argument in arguments {
                        child(argument, work)?;
                    }
                }
                ValueOperation::Size { collection, .. } => child(collection, work)?,
                ValueOperation::Contains { collection, member } => {
                    child(collection, work)?;
                    child(member, work)?;
                }
                ValueOperation::Query {
                    collection, body, ..
                } => {
                    child(collection, work)?;
                    child(body, work)?;
                }
                ValueOperation::Parent { reference, .. } => child(reference, work)?,
                ValueOperation::Reaches { start, target, .. } => {
                    child(start, work)?;
                    child(target, work)?;
                }
            }
        }
        validate::acyclic(&graph.edges, work)?;
        work.locus = Some(declaration.locus.clone());
        Ok(graph)
    }

    pub fn local(&self, handle: &Handle, count: usize, work: &mut Work) -> Result<usize, Error> {
        local(self.owner, handle, count, work)
    }

    pub fn anchor(&self, handle: &Handle, work: &mut Work) -> Result<&'a Anchor, Error> {
        let index = self.local(handle, self.declaration.anchors.len(), work)?;
        self.declaration
            .anchors
            .get(index)
            .ok_or(Error::Invalid(Invalid::Reference))
    }

    pub fn targets(&self, index: usize) -> Result<&[usize], Error> {
        self.edges
            .get(index)
            .map(Vec::as_slice)
            .ok_or(Error::Invalid(Invalid::Reference))
    }

    fn push(&mut self, from: usize, to: usize, work: &mut Work) -> Result<(), Error> {
        work.charge(Dimension::Entries, 1)?;
        self.edges
            .get_mut(from)
            .ok_or(Error::Invalid(Invalid::Reference))?
            .push(to);
        Ok(())
    }
}

fn local(owner: usize, handle: &Handle, count: usize, work: &mut Work) -> Result<usize, Error> {
    work.visit()?;
    if handle.declaration as usize != owner {
        return Err(Error::Invalid(Invalid::Owner));
    }
    let index = handle.index as usize;
    if index >= count {
        return Err(Error::Invalid(Invalid::Reference));
    }
    Ok(index)
}

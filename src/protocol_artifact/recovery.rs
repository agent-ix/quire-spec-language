// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: one recovery-to-population attribution rule for both artifact paths.

use std::collections::BTreeSet;

use super::{value_graph::ValueGraph, wire::*, work::Work, Dimension, Error, Invalid};

/// Select existing declaration-owned pairs, without creating model authorities.
/// The model validator separately checks their exact nominal exports/contracts.
pub(super) fn population_members(
    graph: &ValueGraph<'_>,
    value: &Compensation,
    subject: &Handle,
    work: &mut Work,
) -> Result<BTreeSet<u32>, Error> {
    let declaration = graph.declaration;
    let Body::Protocol { compensations, .. } = &declaration.body else {
        return Err(Error::Invalid(Invalid::Owner));
    };
    let at = graph.local(subject, compensations.len(), work)?;
    let original = compensations
        .get(at)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    if !std::ptr::eq(original, value) {
        return Err(Error::Invalid(Invalid::Owner));
    }
    let recovery = graph.local(&value.recovery, declaration.binders.len(), work)?;
    let recovery = declaration
        .binders
        .get(recovery)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    let anchor = graph.anchor(&recovery.anchor, work)?;
    if recovery.kind != BinderKind::Recovery
        || anchor.kind != AnchorKind::Recovery
        || anchor.owner.0.as_ref() != Some(subject)
    {
        return Err(Error::Invalid(Invalid::Binding));
    }
    let start = graph.local(&value.recover, declaration.values.len(), work)?;
    work.charge(Dimension::Entries, 2)?;
    let mut anchors = BTreeSet::from([recovery.anchor.index]);
    let mut pending = vec![start];
    let mut visited = BTreeSet::new();
    while let Some(index) = pending.pop() {
        work.visit()?;
        if visited.contains(&index) {
            continue;
        }
        work.charge(Dimension::Entries, 1)?;
        visited.insert(index);
        let node = declaration
            .values
            .get(index)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        work.locus = Some(node.locus.clone());
        if let Origin::Anchor { anchor } = &node.origin {
            let original = graph.anchor(anchor, work)?;
            if matches!(
                original.kind,
                AnchorKind::Registration
                    | AnchorKind::CompensationActivation
                    | AnchorKind::Retry
                    | AnchorKind::Recovery
            ) && original.owner.0.as_ref() != Some(subject)
            {
                return Err(Error::Invalid(Invalid::Binding));
            }
            if !anchors.contains(&anchor.index) {
                work.charge(Dimension::Entries, 1)?;
                anchors.insert(anchor.index);
            }
        }
        for target in graph.targets(index)? {
            work.visit()?;
            if !visited.contains(target) {
                work.charge(Dimension::Entries, 1)?;
                pending.push(*target);
            }
        }
    }
    work.locus = Some(value.locus.clone());
    pairs(graph, &anchors, work)
}

fn pairs(
    graph: &ValueGraph<'_>,
    anchors: &BTreeSet<u32>,
    work: &mut Work,
) -> Result<BTreeSet<u32>, Error> {
    let bindings = &graph.declaration.bindings;
    let mut populations = BTreeSet::new();
    for (index, binding) in bindings.iter().enumerate() {
        work.visit()?;
        if binding.kind != BindingKind::Population
            || !matches!(binding.subject, Subject::Declaration { declaration } if declaration as usize == graph.owner)
            || !anchors.contains(&binding.anchor.index)
        {
            continue;
        }
        graph.anchor(&binding.anchor, work)?;
        if binding.model.0.is_none() || binding.value_type.0.is_none() {
            return Err(Error::Invalid(Invalid::Binding));
        }
        work.charge(Dimension::Entries, 1)?;
        populations.insert(u32::try_from(index).map_err(|_| Error::Invalid(Invalid::Reference))?);
    }
    let mut paired = BTreeSet::new();
    let mut members = BTreeSet::new();
    for (index, closure) in bindings.iter().enumerate() {
        work.visit()?;
        if closure.kind != BindingKind::Closure {
            continue;
        }
        let [population] = closure.requires.as_slice() else {
            continue;
        };
        work.visit()?;
        if !populations.contains(population) {
            continue;
        }
        let original = bindings
            .get(*population as usize)
            .ok_or(Error::Invalid(Invalid::Binding))?;
        if paired.contains(population) {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
        if closure.model != original.model
            || closure.value_type != original.value_type
            || closure.subject != original.subject
            || closure.anchor != original.anchor
            || closure.scope != original.scope
        {
            return Err(Error::Invalid(Invalid::Binding));
        }
        work.charge(Dimension::Entries, 3)?;
        paired.insert(*population);
        members.insert(*population);
        members.insert(u32::try_from(index).map_err(|_| Error::Invalid(Invalid::Reference))?);
    }
    if paired.len() != populations.len() {
        return Err(Error::Invalid(Invalid::Binding));
    }
    Ok(members)
}

/// Native attachment runs after original values and binder initializers exist.
pub(super) fn attach(
    owner: usize,
    declaration: &mut Declaration,
    work: &mut Work,
) -> Result<(), Error> {
    let Body::Protocol { compensations, .. } = &declaration.body else {
        return Ok(());
    };
    if compensations.is_empty() {
        return Ok(());
    }
    let graph = ValueGraph::new(owner, declaration, work)?;
    let mut selected = Vec::new();
    for (index, value) in compensations.iter().enumerate() {
        work.locus = Some(value.locus.clone());
        work.visit()?;
        let subject = Handle {
            declaration: u32::try_from(owner).map_err(|_| Error::Invalid(Invalid::Owner))?,
            index: u32::try_from(index).map_err(|_| Error::Invalid(Invalid::Reference))?,
        };
        let members = population_members(&graph, value, &subject, work)?;
        work.charge(Dimension::Entries, 1)?;
        selected.push((index, members));
    }
    let Body::Protocol { compensations, .. } = &mut declaration.body else {
        return Err(Error::Invalid(Invalid::Owner));
    };
    for (index, members) in selected {
        work.visit()?;
        let value = compensations
            .get_mut(index)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        work.locus = Some(value.locus.clone());
        work.charge(Dimension::Entries, members.len())?;
        value.recovery_bindings.extend(members);
        work.charge(Dimension::References, value.recovery_bindings.len())?;
        value.recovery_bindings.sort_unstable();
    }
    work.locus = Some(declaration.locus.clone());
    Ok(())
}

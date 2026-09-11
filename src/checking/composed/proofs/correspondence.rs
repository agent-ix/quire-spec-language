// SPDX-License-Identifier: AGPL-3.0-only
//! Authored source/clause selection reuses the native formal-source admission.

use super::work::{Dimension as D, Work};
use super::*;
use crate::checking::composed::{sources::Correspondence, work::Dimension as SourceWork};
use crate::linking::composed::models::ModelTarget;
use crate::syntax::{composed as c, ClauseKind};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Mappings {
    pub declarations: BTreeMap<DeclarationId, Result<AuthoredBinding, CorrespondenceError>>,
}

impl Mappings {
    pub(super) fn new(
        types: &TypeReport<'_, '_>,
        offered: &[CheckBindings],
        work: &mut Work,
        site: Site,
    ) -> Result<Self, Exhaustion> {
        let mut sources = Vec::new();
        for input in offered {
            work.charge(D::Records, 1, site)?;
            work.charge(
                D::Bytes,
                input.source.identity().document().as_str().len(),
                site,
            )?;
            // Source is Arc-backed; only the formal identity is copied here.
            sources.push(input.source.clone());
        }
        let correspondence =
            Correspondence::collect(types.binding(), &sources, |dimension, amount| {
                let dimension = match dimension {
                    SourceWork::Bytes => D::Bytes,
                    SourceWork::Constraints => D::Types,
                    SourceWork::Records => D::Records,
                    SourceWork::Declarations
                    | SourceWork::Expressions
                    | SourceWork::Edges
                    | SourceWork::Normalization
                    | SourceWork::Depth => unreachable!(
                        "source correspondence charges only bytes, lookups and records"
                    ),
                };
                work.charge(dimension, amount, site)
            })?;
        let mut names = Vec::new();
        let mut qualified = BTreeMap::new();
        let mut duplicate = BTreeSet::new();
        for (source, input) in offered.iter().enumerate() {
            work.charge(D::Records, 1, site)?;
            let mut local = BTreeMap::new();
            for (clause, binding) in input.clauses.iter().enumerate() {
                work.charge(D::Records, 1, site)?;
                work.charge(D::Types, 1, site)?;
                let metadata = binding.name.len()
                    + binding.requirement.package().as_str().len()
                    + binding.requirement.requirement().as_str().len()
                    + binding.clause.as_str().len()
                    + point_bytes(&binding.execution_point);
                work.charge(D::Bytes, metadata, site)?;
                let at = AuthoredBinding { source, clause };
                if let Some(previous) = local.insert(binding.name.as_str(), at) {
                    work.charge(D::Records, 2, site)?;
                    duplicate.insert((previous.source, previous.clause));
                    duplicate.insert((source, clause));
                }
                if let Some(previous) =
                    qualified.insert((&binding.requirement, &binding.clause), at)
                {
                    work.charge(D::Records, 2, site)?;
                    duplicate.insert((previous.source, previous.clause));
                    duplicate.insert((source, clause));
                }
            }
            names.push(local);
        }
        let namespace = types.binding().namespace();
        let mut declarations = BTreeMap::new();
        for bound in types.binding().declarations() {
            let id = bound.declaration();
            let entry = namespace.declaration(id).expect("retained binding owner");
            let syntax = namespace.syntax(id).expect("original syntax");
            let local_site = Site {
                declaration: id,
                unit: entry.unit(),
                expression: None,
                span: syntax.span,
            };
            work.charge(D::Types, 1, local_site)?;
            work.charge(D::Records, 1, local_site)?;
            let selected = if let Some(source) = correspondence.index(entry.unit()) {
                work.charge(D::Bytes, syntax.name.value.len(), local_site)?;
                if let Some(&at) = names[source].get(syntax.name.value.as_str()) {
                    if duplicate.contains(&(at.source, at.clause)) {
                        Err(CorrespondenceError::DuplicateDeclaration)
                    } else if !execution(
                        types,
                        id,
                        &offered[source].clauses[at.clause].execution_point,
                        work,
                        local_site,
                    )? {
                        Err(CorrespondenceError::ExecutionPoint)
                    } else {
                        Ok(at)
                    }
                } else {
                    Err(CorrespondenceError::MissingDeclaration)
                }
            } else {
                // Source correspondence deliberately returns no selection for
                // duplicates/conflicts. Retain their input without inventing one.
                let mut candidates = 0;
                for input in offered {
                    work.charge(D::Types, 1, local_site)?;
                    let original = namespace.unit(entry.unit()).expect("unit").source();
                    let candidate = input.source.source();
                    work.charge(
                        D::Bytes,
                        original.identity().identity.len()
                            + original.identity().revision.len()
                            + candidate.identity().identity.len()
                            + candidate.identity().revision.len(),
                        local_site,
                    )?;
                    if candidate.identity() == original.identity() {
                        candidates += 1;
                    }
                }
                Err(match candidates {
                    0 => CorrespondenceError::MissingSource,
                    1 => CorrespondenceError::ForeignSource,
                    _ => CorrespondenceError::DuplicateSource,
                })
            };
            declarations.insert(id, selected);
        }
        // Extra mappings refuse declarations from that source, rather than
        // silently turning a complete authored correspondence into a partial one.
        for (source, input) in offered.iter().enumerate() {
            for binding in &input.clauses {
                work.charge(D::Types, 1, site)?;
                work.charge(D::Bytes, binding.name.len(), site)?;
                let mut found = false;
                for &id in namespace.lookup(&binding.name) {
                    work.charge(D::Types, 1, site)?;
                    let unit = namespace.declaration(id).expect("lookup").unit();
                    found |= correspondence.index(unit) == Some(source);
                }
                if !found {
                    for (&id, selected) in &mut declarations {
                        work.charge(D::Types, 1, site)?;
                        let unit = namespace.declaration(id).expect("owner").unit();
                        if correspondence.index(unit) == Some(source) {
                            *selected = Err(CorrespondenceError::ForeignDeclaration);
                        }
                    }
                }
            }
        }
        Ok(Self { declarations })
    }
}

fn point_bytes(point: &ir::ExecutionPoint) -> usize {
    match point {
        ir::ExecutionPoint::Initialization { name } => name.as_str().len(),
        ir::ExecutionPoint::Handler { name } => name.as_str().len(),
        ir::ExecutionPoint::Pre { operation } | ir::ExecutionPoint::Post { operation } => {
            operation.as_str().len()
        }
    }
}
fn execution(
    types: &TypeReport<'_, '_>,
    id: DeclarationId,
    point: &ir::ExecutionPoint,
    work: &mut Work,
    site: Site,
) -> Result<bool, Exhaustion> {
    let syntax = types.binding().namespace().syntax(id).expect("syntax");
    match (&syntax.kind, point) {
        (
            c::DeclarationKind::State {
                kind: ClauseKind::Precondition,
                ..
            },
            ir::ExecutionPoint::Pre { operation },
        )
        | (
            c::DeclarationKind::State {
                kind: ClauseKind::Postcondition,
                ..
            },
            ir::ExecutionPoint::Post { operation },
        ) => {
            if let Some(exports) = types.binding().exports().get(id.index()) {
                for occurrence in &exports.occurrences {
                    work.charge(D::Types, 1, site)?;
                    if let ModelTarget::Operation(bound) = &occurrence.target {
                        work.charge(
                            D::Bytes,
                            operation.as_str().len() + bound.role().anchor.as_str().len(),
                            site,
                        )?;
                        return Ok(&bound.role().anchor == operation);
                    }
                }
            }
            Ok(false)
        }
        (
            c::DeclarationKind::Predicate { .. }
            | c::DeclarationKind::Temporal { .. }
            | c::DeclarationKind::Protocol(_)
            | c::DeclarationKind::State {
                kind: ClauseKind::Invariant,
                ..
            },
            ir::ExecutionPoint::Initialization { .. } | ir::ExecutionPoint::Handler { .. },
        ) => Ok(true),
        _ => Ok(false),
    }
}

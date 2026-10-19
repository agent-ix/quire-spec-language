// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218 "Requirements": the S3 check of a protocol's `requires temporal T;`
//! members.
//!
//! Each written name resolves once, among the unit's own declarations: to a
//! `temporal` clause, which the checked requirement binds by its position
//! among the unit's temporal clauses (the node key S4 mints for that
//! position, as a protocol reference binds its target, FR-092), never by
//! the name. A name no declaration of the unit holds, an import or model
//! alias included, refuses `missing_declaration`/`missing-name`; a
//! declaration of another kind refuses `ill_typed`/`operator-ineligible`,
//! naming the kind. A clause required twice is recorded once, at its first
//! requirement.
//!
//! A required clause that reaches the requiring protocol through a protocol
//! reference, directly or through the requirements of the protocols it
//! names, closes a definition cycle, refused `invalid_package`/
//! `definition-cycle` at the requirement. The references are read from the
//! clause's form: a `protocol P` names the unit's first protocol written
//! `P`, as the clause checker resolves it.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{ExprNode, ExpressionForm, ProtocolDeclarationForm, TemporalNodeForm};
use qsl_foundation::Span;
use quire_semantic_value::location::{Location, Origin};

use super::refusal::{CheckCause, CheckRefusal, ProtocolAnchorCause};
use super::temporal_clause::TemporalDeclaration;

/// One checked `requires temporal T;` member (FR-218 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedRequirement {
    /// The required clause's position among the unit's `temporal` clauses,
    /// in declaration order.
    pub temporal: usize,
    /// The span of the first requirement naming it.
    pub span: Span,
}

/// The unit's declarations a requirement name may resolve to: its
/// `temporal` clauses by name, and the kind of every other declaration.
pub(crate) struct UnitDeclarations<'u> {
    temporal: BTreeMap<&'u str, usize>,
    other: BTreeMap<&'u str, &'static str>,
    protocols: BTreeMap<&'u str, usize>,
}

impl<'u> UnitDeclarations<'u> {
    /// The unit's temporal clauses, its protocols and each other
    /// declaration's name with its kind's keyword. A name declared twice
    /// keeps its first.
    pub(crate) fn new(
        temporal: &'u [TemporalDeclaration],
        protocols: &'u [ProtocolDeclarationForm],
        other: impl IntoIterator<Item = (&'u str, &'static str)>,
    ) -> Self {
        let mut declarations = Self {
            temporal: BTreeMap::new(),
            other: BTreeMap::new(),
            protocols: BTreeMap::new(),
        };
        for (position, clause) in temporal.iter().enumerate() {
            declarations
                .temporal
                .entry(clause.form.name.name.as_str())
                .or_insert(position);
        }
        for (position, protocol) in protocols.iter().enumerate() {
            declarations
                .protocols
                .entry(protocol.name.name.as_str())
                .or_insert(position);
            declarations
                .other
                .entry(protocol.name.name.as_str())
                .or_insert("protocol");
        }
        for (name, kind) in other {
            declarations.other.entry(name).or_insert(kind);
        }
        declarations
    }
}

fn refusal(cause: ProtocolAnchorCause) -> CheckRefusal {
    CheckRefusal {
        // `region::DeclarationRegions::refusal_region` reads the cause's own
        // span.
        location: Location::root(Origin::Expression),
        cause: CheckCause::ProtocolAnchor(Box::new(cause)),
    }
}

/// The requirements of the `index`-th protocol of `protocols`, each bound to
/// its clause and in the order of each clause's first requirement, or the
/// refusals in source order.
pub(crate) fn check(
    index: usize,
    protocols: &[ProtocolDeclarationForm],
    temporal: &[TemporalDeclaration],
    declarations: &UnitDeclarations<'_>,
) -> Result<Vec<CheckedRequirement>, Vec<CheckRefusal>> {
    let Some(protocol) = protocols.get(index) else {
        return Ok(Vec::new());
    };
    let mut checked: Vec<CheckedRequirement> = Vec::new();
    let mut refusals = Vec::new();
    for requirement in &protocol.requirements {
        let name = &requirement.temporal;
        let Some(&clause) = declarations.temporal.get(name.name.as_str()) else {
            refusals.push(refusal(match declarations.other.get(name.name.as_str()) {
                Some(kind) => ProtocolAnchorCause::RequirementKind {
                    name: name.name.clone(),
                    kind,
                    span: name.span,
                },
                None => ProtocolAnchorCause::MissingRequirement {
                    name: name.name.clone(),
                    span: name.span,
                },
            }));
            continue;
        };
        if reaches(clause, index, protocols, temporal, declarations) {
            refusals.push(refusal(ProtocolAnchorCause::RequirementCycle {
                protocol: protocol.name.name.clone(),
                temporal: name.name.clone(),
                span: name.span,
            }));
            continue;
        }
        if !checked.iter().any(|seen| seen.temporal == clause) {
            checked.push(CheckedRequirement {
                temporal: clause,
                span: name.span,
            });
        }
    }
    if refusals.is_empty() {
        Ok(checked)
    } else {
        Err(refusals)
    }
}

/// Whether the temporal clause `start` reaches the protocol `target`: a
/// clause reaches each protocol its expressions reference, and a protocol
/// reaches each clause it requires. Each node is visited once.
fn reaches(
    start: usize,
    target: usize,
    protocols: &[ProtocolDeclarationForm],
    temporal: &[TemporalDeclaration],
    declarations: &UnitDeclarations<'_>,
) -> bool {
    let mut seen_clauses = BTreeSet::from([start]);
    let mut seen_protocols = BTreeSet::new();
    let mut pending = vec![start];
    while let Some(clause) = pending.pop() {
        let Some(declaration) = temporal.get(clause) else {
            continue;
        };
        for referenced in referenced_protocols(declaration, declarations) {
            if referenced == target {
                return true;
            }
            if !seen_protocols.insert(referenced) {
                continue;
            }
            let Some(protocol) = protocols.get(referenced) else {
                continue;
            };
            for requirement in &protocol.requirements {
                if let Some(&next) = declarations
                    .temporal
                    .get(requirement.temporal.name.as_str())
                {
                    if seen_clauses.insert(next) {
                        pending.push(next);
                    }
                }
            }
        }
    }
    false
}

/// The protocols the expressions of `clause` reference: its captures'
/// values and its formula's `holds` conditions.
fn referenced_protocols(
    clause: &TemporalDeclaration,
    declarations: &UnitDeclarations<'_>,
) -> Vec<usize> {
    let expressions = clause
        .form
        .captures
        .iter()
        .map(|capture| &capture.value)
        .chain(
            clause
                .form
                .formula
                .nodes()
                .iter()
                .filter_map(|node| match node {
                    TemporalNodeForm::Holds { condition, .. } => Some(condition),
                    TemporalNodeForm::Constant { .. } | TemporalNodeForm::Operator(_) => None,
                }),
        );
    expressions
        .flat_map(|form: &ExpressionForm| form.expression.iter())
        .filter_map(|node| match node.node() {
            ExprNode::ProtocolReference { target } => {
                declarations.protocols.get(target.as_str()).copied()
            }
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests;

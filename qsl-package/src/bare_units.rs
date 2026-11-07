// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533 "What the bare view reads from the package": the definitions of the
//! source-owned units the package's quantities name, read from the package's
//! own unit and dimension nodes through Contract IR's
//! `read_unit_definition` and admitted by the same `admit_unit_graph` the
//! source route admits its units through, so that a bytes-only evaluation
//! resolves a quantity's unit exactly as the source route does.

use qsl_semantics::value::{
    admit_unit_graph, DimensionPreimage, NodeOwner, OwnerSelection, OwnerSubject, UnitPreimage,
};
use quire_contract_model::nominal_reads::{
    NominalDeclarationRow, NominalIdentity, NominalReadCode,
};
use quire_contract_model::{
    CheckedNodeOwner, CheckedPackageV2, CheckedRational, NativeJoinBudget, SelectedSourceUnitRow,
};
use quire_exact::{Integer, NodeKey, Rational};
use quire_semantic_value::quantity::UnitTable;

use crate::checked_v2::wire_graph;

/// Why the selected source-unit rows are no unit table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnitDefinitionRefusal {
    /// IR's nominal read refused with its own code; `LimitReached` is the
    /// caller's native allowance running out.
    Read(NominalReadCode),
    /// The read definitions are not an admitted unit graph.
    Graph,
    /// A selected row names no node of the package.
    Unjoined,
}

/// A checked rational as `Rational`, only when its spelling is the reduced one:
/// a preimage carries canonical spellings, so an unreduced spelling is refused
/// rather than silently normalised.
fn rational(value: &CheckedRational) -> Option<Rational> {
    let parsed = Rational::new(
        value.numerator.parse::<Integer>().ok()?,
        value.denominator.parse::<Integer>().ok()?,
    )
    .ok()?;
    (parsed.numerator().to_string() == *value.numerator
        && parsed.denominator().to_string() == *value.denominator)
        .then_some(parsed)
}

/// The preimage owner of a source-owned node; only a source owns a unit or
/// dimension the bare route reads.
fn owner(package: &CheckedPackageV2, owner: Option<&CheckedNodeOwner>) -> Option<NodeOwner> {
    match owner? {
        CheckedNodeOwner::Source {
            authority,
            identity,
        } => Some(NodeOwner::Source(OwnerSubject::new(
            &**authority,
            &**identity,
            package.name(),
        ))),
        CheckedNodeOwner::ModelRoot { .. } | CheckedNodeOwner::Model { .. } => None,
    }
}

fn name(declaration: Option<&quire_contract_model::CheckedDeclaration>) -> Option<Vec<String>> {
    Some(
        declaration?
            .qualified_name
            .iter()
            .map(ToString::to_string)
            .collect(),
    )
}

/// The unit table of the source-owned units `rows` select, charged to
/// `budget`. A selected row that IR already joined to the package's nodes is
/// read; the read and the graph's admission are IR's and the source route's,
/// not a second implementation.
pub(crate) fn read_unit_table(
    package: &CheckedPackageV2,
    rows: &[SelectedSourceUnitRow],
    budget: &mut NativeJoinBudget,
) -> Result<UnitTable, UnitDefinitionRefusal> {
    use UnitDefinitionRefusal::{Graph, Read, Unjoined};
    if rows.is_empty() {
        return Ok(UnitTable::default());
    }
    let wire = wire_graph(package).map_err(|_| Graph)?;
    let key = |id: &quire_contract_model::CheckedNodeId| -> Result<NodeKey, _> {
        wire.key(id).map_err(|_| Graph)
    };
    let mut declared = Vec::new();
    for row in rows {
        let node = package
            .graph()
            .nodes
            .iter()
            .find(|node| node.node_id == row.structural_node)
            .ok_or(Unjoined)?;
        let (Some(owner), Some(declaration)) = (node.owner.as_ref(), node.declaration.as_ref())
        else {
            return Err(Unjoined);
        };
        declared.push(NominalDeclarationRow {
            node: &row.structural_node,
            owner,
            declaration,
            identity: NominalIdentity::Unit(&row.unit_id),
        });
    }
    let requested: Vec<_> = rows.iter().map(|row| row.structural_node.clone()).collect();
    let read = package
        .read_unit_definition(&requested, &declared, budget)
        .map_err(|refusal| Read(refusal.code))?;
    let mut owners = Vec::new();
    let mut dimensions = Vec::new();
    for dimension in &read.dimensions {
        let mut terms = Vec::new();
        for term in dimension.terms {
            terms.push((
                key(&term.dimension_node_id)?,
                term.exponent.parse::<Integer>().map_err(|_| Graph)?,
            ));
        }
        terms.sort_by(|left, right| left.0.cmp(&right.0));
        let owner = owner(package, dimension.node.owner.as_ref())
            .ok_or(Read(NominalReadCode::UnsupportedForm))?;
        let qualified = name(dimension.node.declaration.as_ref()).ok_or(Graph)?;
        let preimage = if dimension.base {
            DimensionPreimage::new(owner.clone(), qualified, terms)
        } else {
            DimensionPreimage::new_derived(owner.clone(), qualified, terms)
        }
        .map_err(|_| Graph)?;
        owners.push(owner);
        dimensions.push((preimage, key(&dimension.node.node_id)?));
    }
    let mut units = Vec::new();
    for unit in &read.units {
        let owner = owner(package, unit.node.owner.as_ref())
            .ok_or(Read(NominalReadCode::UnsupportedForm))?;
        let target = unit.target.map(|target| key(&target.node_id)).transpose()?;
        let preimage = UnitPreimage::new(
            owner.clone(),
            name(unit.node.declaration.as_ref()).ok_or(Graph)?,
            key(&unit.dimension.node_id)?,
            target,
            &rational(unit.scale).ok_or(Graph)?,
            &rational(unit.offset).ok_or(Graph)?,
        )
        .map_err(|_| Graph)?;
        // The row's identity is a claim of the producer: it must be the key
        // the table holds the unit by, or a quantity naming it would not
        // resolve.
        let unit_key = key(&unit.node.node_id)?;
        if *unit.identity != quire_exact::UnitId::Declared(unit_key.clone()) {
            return Err(Graph);
        }
        owners.push(owner);
        units.push((preimage, unit_key));
    }
    let graph =
        admit_unit_graph(dimensions, units, &OwnerSelection::new(owners)).map_err(|_| Graph)?;
    Ok(UnitTable::declared(&graph))
}

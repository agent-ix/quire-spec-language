// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533 "What the bare view reads from the package": the definitions of the
//! source-owned units the package's quantities name, read from the package's
//! own unit and dimension nodes through Contract IR's
//! `read_unit_definition`, so that a bytes-only evaluation resolves a
//! quantity's unit exactly as the source route does.

use std::collections::BTreeMap;

use quire_contract_model::nominal_reads::{
    NominalDeclarationRow, NominalIdentity, NominalReadCode,
};
use quire_contract_model::{CheckedPackageV2, CheckedRational, NativeJoinBudget};
use quire_exact::{Integer, NodeKey, Rational};
use quire_semantic_value::quantity::UnitTable;
use quire_semantic_value::unit::{DimensionNode, NominalDeclaration, UnitGraph, UnitNode};

use crate::checked_v2::wire_graph;

/// Why the package's unit definitions are not a unit table: the owner's
/// closed read code, or a definition that is no admitted unit graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnitDefinitionRefusal {
    /// IR's nominal read refused with this code.
    Read(NominalReadCode),
    /// The read definitions are not an admitted unit graph.
    Graph,
}

fn rational(value: &CheckedRational) -> Option<Rational> {
    Rational::new(
        value.numerator.parse::<Integer>().ok()?,
        value.denominator.parse::<Integer>().ok()?,
    )
    .ok()
}

fn declaration(name: &[Box<str>]) -> NominalDeclaration {
    NominalDeclaration {
        qualified_declaration: name.iter().map(|part| part.to_string()).collect(),
        ..NominalDeclaration::default()
    }
}

/// The unit table of the source-owned units `rows` select, charged to
/// `budget`. `Ok(Err(_))` is a refusal of the definitions; an exhausted
/// budget is reported by `budget` and returned as `Err(true)`.
pub(crate) fn read_unit_table(
    package: &CheckedPackageV2,
    rows: &[quire_contract_model::SelectedSourceUnitRow],
    budget: &mut NativeJoinBudget,
) -> Result<UnitTable, UnitDefinitionRefusal> {
    if rows.is_empty() {
        return Ok(UnitTable::default());
    }
    let graph = wire_graph(package).map_err(|_| UnitDefinitionRefusal::Graph)?;
    let key = |id: &quire_contract_model::CheckedNodeId| -> Result<NodeKey, UnitDefinitionRefusal> {
        graph.key(id).map_err(|_| UnitDefinitionRefusal::Graph)
    };
    let mut declared = Vec::new();
    for row in rows {
        let node = package
            .graph()
            .nodes
            .iter()
            .find(|node| node.node_id == row.structural_node)
            .ok_or(UnitDefinitionRefusal::Read(NominalReadCode::MissingNode))?;
        let (Some(owner), Some(declaration)) = (node.owner.as_ref(), node.declaration.as_ref())
        else {
            return Err(UnitDefinitionRefusal::Read(NominalReadCode::ForeignWitness));
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
        .map_err(|refusal| UnitDefinitionRefusal::Read(refusal.code))?;
    let mut dimensions = BTreeMap::new();
    for dimension in &read.dimensions {
        let mut terms = Vec::new();
        for term in dimension.terms {
            terms.push((
                key(&term.dimension_node_id)?,
                term.exponent
                    .parse::<Integer>()
                    .map_err(|_| UnitDefinitionRefusal::Graph)?,
            ));
        }
        terms.sort_by(|left, right| left.0.cmp(&right.0));
        let name = declaration(
            &dimension
                .node
                .declaration
                .as_ref()
                .ok_or(UnitDefinitionRefusal::Graph)?
                .qualified_name,
        );
        let node = if dimension.base {
            DimensionNode::checked(terms, name)
        } else {
            DimensionNode::checked_derived(terms, name)
        }
        .map_err(|_| UnitDefinitionRefusal::Graph)?;
        dimensions.insert(key(&dimension.node.node_id)?, node);
    }
    let mut units = BTreeMap::new();
    for unit in &read.units {
        let target = unit.target.map(|target| key(&target.node_id)).transpose()?;
        let name = declaration(
            &unit
                .node
                .declaration
                .as_ref()
                .ok_or(UnitDefinitionRefusal::Graph)?
                .qualified_name,
        );
        let node = UnitNode::checked(
            key(&unit.dimension.node_id)?,
            target,
            rational(unit.scale).ok_or(UnitDefinitionRefusal::Graph)?,
            rational(unit.offset).ok_or(UnitDefinitionRefusal::Graph)?,
            name,
        )
        .map_err(|_| UnitDefinitionRefusal::Graph)?;
        let unit_key = key(&unit.node.node_id)?;
        // The row's identity is a claim of the producer: it must be the key
        // the table holds the unit by, or a quantity naming it would not
        // resolve.
        if *unit.identity != quire_exact::UnitId::Declared(unit_key.clone()) {
            return Err(UnitDefinitionRefusal::Graph);
        }
        units.insert(unit_key, node);
    }
    let graph = UnitGraph::from_checked_nodes(&dimensions, &units)
        .map_err(|_| UnitDefinitionRefusal::Graph)?;
    Ok(UnitTable::declared(&graph))
}

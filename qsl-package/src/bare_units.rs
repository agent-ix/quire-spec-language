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
fn owner(package_name: &str, owner: Option<&CheckedNodeOwner>) -> Option<NodeOwner> {
    match owner? {
        CheckedNodeOwner::Source {
            authority,
            identity,
        } => Some(NodeOwner::Source(OwnerSubject::new(
            &**authority,
            &**identity,
            package_name,
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
            .resolved_graph()
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
        // A dimension no selected source owns is not in the owner selection.
        let owner = owner(package.name(), dimension.node.owner.as_ref()).ok_or(Graph)?;
        let qualified = name(dimension.node.declaration.as_ref()).ok_or(Graph)?;
        let preimage = if dimension.base {
            DimensionPreimage::new(owner, qualified, terms)
        } else {
            DimensionPreimage::new_derived(owner, qualified, terms)
        }
        .map_err(|_| Graph)?;
        dimensions.push((preimage, key(&dimension.node.node_id)?));
    }
    let mut units = Vec::new();
    for unit in &read.units {
        let owner = owner(package.name(), unit.node.owner.as_ref()).ok_or(Graph)?;
        let target = unit.target.map(|target| key(&target.node_id)).transpose()?;
        let preimage = UnitPreimage::new(
            owner,
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
        units.push((preimage, unit_key));
    }
    // The owners the package selected: its locked raw sources.
    let selected = OwnerSelection::new(package.lock().sources.iter().map(|source| {
        NodeOwner::Source(OwnerSubject::new(
            &*source.authority,
            &*source.identity,
            package.name(),
        ))
    }));
    admit(dimensions, units, &selected)
}

/// The unit table of the dimension and unit nodes, admitted as the source
/// route admits its units.
fn admit(
    dimensions: Vec<(DimensionPreimage, NodeKey)>,
    units: Vec<(UnitPreimage, NodeKey)>,
    owners: &OwnerSelection,
) -> Result<UnitTable, UnitDefinitionRefusal> {
    let graph =
        admit_unit_graph(dimensions, units, owners).map_err(|_| UnitDefinitionRefusal::Graph)?;
    Ok(UnitTable::declared(&graph))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn source_owner(identity: &str) -> NodeOwner {
        NodeOwner::Source(OwnerSubject::new("a", identity, "pkg"))
    }

    /// R's `Money` and `U`, owned by `owner`, with their keys.
    fn money(owner: &NodeOwner) -> (DimensionPreimage, NodeKey, UnitPreimage, NodeKey) {
        let dimension = DimensionPreimage::new(owner.clone(), vec!["Money".to_owned()], Vec::new())
            .expect("a dimension");
        let dimension_key = dimension.key().expect("a key");
        let unit = UnitPreimage::new(
            owner.clone(),
            vec!["U".to_owned()],
            dimension_key.clone(),
            None,
            &Rational::from_integer(Integer::from(1_i64)),
            &Rational::from_integer(Integer::from(0_i64)),
        )
        .expect("a unit");
        let unit_key = unit.key().expect("a key");
        (dimension, dimension_key, unit, unit_key)
    }

    /// FR-533-AC-19: the unit table admits the units of a selected owner and
    /// refuses a unit whose owner the package did not select.
    #[trace("TC-1333", "FR-533-AC-19")]
    #[test]
    fn a_unit_of_an_unselected_owner_is_no_unit_graph() {
        let owned = source_owner("u");
        let (dimension, dimension_key, unit, unit_key) = money(&owned);
        let parts = || {
            (
                vec![(dimension.clone(), dimension_key.clone())],
                vec![(unit.clone(), unit_key.clone())],
            )
        };
        let (dimensions, units) = parts();
        assert!(admit(dimensions, units, &OwnerSelection::new([owned.clone()])).is_ok());
        let (dimensions, units) = parts();
        assert_eq!(
            admit(
                dimensions,
                units,
                &OwnerSelection::new([source_owner("other")])
            )
            .unwrap_err(),
            UnitDefinitionRefusal::Graph
        );
    }

    /// FR-533-AC-19: only a source owns a unit or dimension the bare route
    /// reads; a node of a model owner has no owner in the selection, so a
    /// dimension (or unit) it owns is refused as no unit graph.
    #[trace("TC-1333", "FR-533-AC-19")]
    #[test]
    fn a_node_no_source_owns_has_no_selectable_owner() {
        let source = CheckedNodeOwner::Source {
            authority: "a".into(),
            identity: "u".into(),
        };
        assert_eq!(owner("pkg", Some(&source)), Some(self::source_owner("u")));
        for other in [
            CheckedNodeOwner::ModelRoot {
                identity: "m".into(),
            },
            CheckedNodeOwner::Model {
                identity: "m".into(),
                node: "n".into(),
            },
        ] {
            assert_eq!(owner("pkg", Some(&other)), None);
        }
        assert_eq!(owner("pkg", None), None);
    }

    /// FR-533-AC-19: a conversion spelled unreduced is refused, not normalised.
    #[trace("TC-1333", "FR-533-AC-19")]
    #[test]
    fn an_unreduced_conversion_spelling_is_refused() {
        let spelled = |numerator: &str, denominator: &str| CheckedRational {
            numerator: numerator.into(),
            denominator: denominator.into(),
        };
        assert!(rational(&spelled("1", "2")).is_some());
        assert!(rational(&spelled("2", "4")).is_none());
        assert!(rational(&spelled("0", "1")).is_some());
        assert!(rational(&spelled("0", "3")).is_none());
    }
}

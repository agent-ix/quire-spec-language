// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "The time dimension and its units": which units of the checked
//! package's unit graph (FR-091) are time units, and the exact scale of each
//! to its root, so a time quantity converts exactly to another time unit
//! (QSpec FR-142).
//!
//! The time dimension is the base dimension node declared with the name
//! `Time`, and nothing else; the time units are the units typed by that
//! node. A unit name is read in FR-230's order: the graph has a time base,
//! then the unit is found by name, then it is typed by the time dimension,
//! then its composed offset is zero. FR-233 reads an interval bound through
//! it; the `time` member's own reading waits on FR-230's facet gate.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use quire_exact::{Integer, NodeKey, Rational};
use quire_semantic_value::quantity::UnitTable;
use quire_semantic_value::unit::{NominalUnitForm, UnitGraph};

use crate::check::assemble::{DimensionEquivalence, EquivalenceClasses};
use crate::check::lowering::model_declaration_key;
use crate::check::lowering::AdmittedModel;
use crate::check::node_key::NodeTag;
use crate::check::temporal_clause::TimeNominalNode;
use crate::model::domain_package::PackageUnitRecord;
use crate::value::{ModelSubject, NodeOwner, OwnerSubject};

use super::refusal::TimeUnitCause;

/// The name FR-230 fixes for the time dimension's base node.
const TIME_DIMENSION: &str = "Time";

/// One declared unit, as FR-230 reads it.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeclaredUnit {
    node: Option<NodeKey>,
    dimension: Option<NodeKey>,
    scale: Rational,
    offset: Rational,
}

/// The time units of one checked package's unit graph.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TimeUnits {
    /// The base dimension node named `Time`, if the graph declares one.
    time: Option<NodeKey>,
    /// Base `Time` node of each selected package, keyed by its QSL owner name.
    time_dimensions: BTreeMap<String, NodeKey>,
    /// Every declared unit by its declared name.
    units: BTreeMap<String, DeclaredUnit>,
    /// Every locally admitted nominal dimension/unit node by key.
    nominal_nodes: BTreeMap<NodeKey, TimeNominalNode>,
    /// The dimension equivalence classes (QSpec FR-142 "Effect"), through
    /// which a quantity of one `Time` converts to an equivalent `Time`.
    classes: EquivalenceClasses,
}

impl TimeUnits {
    /// The time units of `graph`, whose dimension and unit node keys are
    /// `keys` (the assembler's `nominal_spans` keys). The one place a
    /// declared dimension's spelling is read for FR-230's `Time` base.
    #[ix_trace_rs::string_edge]
    pub fn new(
        graph: &UnitGraph,
        keys: impl IntoIterator<Item = NodeKey>,
        models: &[AdmittedModel],
        equivalences: &[DimensionEquivalence],
        source: &qsl_foundation::source::provenance::RawSourceRef,
        package_name: &str,
    ) -> Self {
        let source_owner = NodeOwner::Source(OwnerSubject::new(
            source.authority(),
            source.identity(),
            package_name,
        ));
        let table = UnitTable::declared(graph);
        let mut time = None;
        let mut time_dimensions = BTreeMap::new();
        let mut units = BTreeMap::new();
        let mut nominal_nodes = BTreeMap::new();
        for key in keys {
            let Some(node) = table.nominal_node(&key) else {
                continue;
            };
            let dependencies = match &node.form {
                NominalUnitForm::Dimension { terms } => {
                    terms.iter().map(|(key, _)| key.clone()).collect()
                }
                NominalUnitForm::Unit {
                    dimension, target, ..
                } => std::iter::once(dimension.clone())
                    .chain(target.iter().cloned())
                    .collect(),
            };
            nominal_nodes.insert(
                key.clone(),
                TimeNominalNode {
                    key: key.clone(),
                    form: match &node.form {
                        NominalUnitForm::Dimension { .. } => "dimension",
                        NominalUnitForm::Unit { .. } => "unit",
                    }
                    .to_owned(),
                    qualified_declaration: node.declaration.qualified_declaration.clone(),
                    owner: source_owner.clone(),
                    dependencies,
                },
            );
            let [name] = node.declaration.qualified_declaration.as_slice() else {
                continue;
            };
            match &node.form {
                NominalUnitForm::Dimension { terms } => {
                    if terms.is_empty() && name == TIME_DIMENSION {
                        time = Some(key.clone());
                    }
                }
                NominalUnitForm::Unit { .. } => {
                    if let Some(unit) = graph.unit(key.clone()) {
                        units.insert(
                            name.clone(),
                            DeclaredUnit {
                                node: Some(key.clone()),
                                dimension: Some(unit.dimension_node()),
                                scale: unit.canonical().scale().clone(),
                                offset: unit.canonical().offset().clone(),
                            },
                        );
                    }
                }
            }
        }
        for model in models {
            let package = model.selection();
            let dimension = model
                .dimensions
                .iter()
                .find(|dimension| dimension.base && dimension.name == TIME_DIMENSION);
            let time_node =
                dimension.and_then(|dimension| scalar_type_key(&dimension.key, "dimension"));
            let time_key = time_node.clone();
            if let Some(time_key) = time_node {
                nominal_nodes.insert(
                    time_key.clone(),
                    TimeNominalNode {
                        key: time_key,
                        form: "dimension".to_owned(),
                        qualified_declaration: vec![TIME_DIMENSION.to_owned()],
                        owner: NodeOwner::Model(ModelSubject {
                            identity: package.identity.clone(),
                            node: dimension
                                .map(|dimension| dimension.key.node.clone())
                                .unwrap_or_default(),
                        }),
                        dependencies: Vec::new(),
                    },
                );
            }
            let prefix = model.alias().unwrap_or(&package.identity);
            if let Some(time_key) = time_key.clone() {
                time_dimensions.insert(prefix.to_owned(), time_key);
            }
            for unit in &model.units {
                let Some((scale, offset)) = package_unit_scale(unit, &model.units) else {
                    continue;
                };
                let package_nodes = dimension.and_then(|dimension| {
                    let time_key = time_key.clone()?;
                    (unit.dimension == dimension.key).then(|| {
                        package_unit_nominal_nodes(unit, &model.units, time_key, &package.identity)
                    })?
                });
                if let Some(package_nodes) = &package_nodes {
                    for node in package_nodes {
                        nominal_nodes.insert(node.key.clone(), node.clone());
                    }
                }
                let node = package_nodes
                    .as_ref()
                    .and_then(|nodes| nodes.last().map(|node| node.key.clone()));
                let dimension_key = dimension
                    .zip(time_key.clone())
                    .filter(|(dimension, _)| unit.dimension == dimension.key)
                    .map(|(_, key)| key);
                units.insert(
                    format!("{prefix}::{}", unit.name),
                    DeclaredUnit {
                        node,
                        dimension: dimension_key,
                        scale,
                        offset,
                    },
                );
            }
        }
        Self {
            time,
            time_dimensions,
            units,
            nominal_nodes,
            classes: EquivalenceClasses::new(equivalences),
        }
    }

    /// The exact scale from the time unit `name` to its root
    /// (`root = scale × name`), or why `name` is not a time unit, in
    /// FR-230's order.
    pub fn scale(&self, name: &str) -> Result<&Rational, TimeUnitCause> {
        let owner_time = name
            .split_once("::")
            .and_then(|(owner, _)| self.time_dimensions.get(owner).cloned());
        let time = owner_time
            .or(self.time.clone())
            .ok_or(TimeUnitCause::NoTimeDimension {
                name: TIME_DIMENSION,
            })?;
        let unit = self
            .units
            .get(name)
            .ok_or_else(|| TimeUnitCause::UnknownUnit {
                name: name.to_owned(),
            })?;
        if unit.dimension != Some(time) {
            return Err(TimeUnitCause::NotTimeDimension {
                name: name.to_owned(),
            });
        }
        if !unit.offset.is_zero() {
            return Err(TimeUnitCause::AffineUnit {
                name: name.to_owned(),
                offset: unit.offset.clone(),
            });
        }
        Ok(&unit.scale)
    }

    /// The exact nominal dimension node for a successfully resolved time
    /// unit, used by the checked timed-model root.
    pub fn dimension(&self, name: &str) -> Result<NodeKey, TimeUnitCause> {
        self.scale(name)?;
        let unit = self
            .units
            .get(name)
            .ok_or_else(|| TimeUnitCause::UnknownUnit {
                name: name.to_owned(),
            })?;
        unit.dimension
            .clone()
            .ok_or(TimeUnitCause::NotTimeDimension {
                name: name.to_owned(),
            })
    }

    /// The time unit `name` as a conversion target: the reciprocal of its
    /// root scale. `Ok(None)` only if the scale is zero, which the unit
    /// graph refuses at admission (an invariant, not a refusal).
    pub fn target(&self, name: &str) -> Result<Option<TimeScale>, TimeUnitCause> {
        let scale = self.scale(name)?;
        let dimension = self.dimension(name)?;
        let unit = self
            .units
            .get(name)
            .and_then(|unit| unit.node.clone())
            .ok_or_else(|| TimeUnitCause::NotTimeDimension {
                name: name.to_owned(),
            })?;
        let nominal_nodes = self.nominal_dependency_closure([dimension.clone(), unit.clone()]);
        Ok(Rational::from_integer(Integer::one())
            .div(scale)
            .map(|reciprocal| TimeScale {
                reciprocal,
                dimension,
                unit,
                nominal_nodes,
            }))
    }

    fn nominal_dependency_closure(
        &self,
        roots: impl IntoIterator<Item = NodeKey>,
    ) -> Vec<TimeNominalNode> {
        fn visit(
            key: NodeKey,
            nodes: &BTreeMap<NodeKey, TimeNominalNode>,
            seen: &mut BTreeSet<NodeKey>,
            ordered: &mut Vec<TimeNominalNode>,
        ) {
            if !seen.insert(key.clone()) {
                return;
            }
            let Some(node) = nodes.get(&key) else {
                return;
            };
            for dependency in &node.dependencies {
                visit(dependency.clone(), nodes, seen, ordered);
            }
            ordered.push(node.clone());
        }
        let mut seen = BTreeSet::new();
        let mut ordered = Vec::new();
        for key in roots {
            visit(key, &self.nominal_nodes, &mut seen, &mut ordered);
        }
        ordered
    }

    /// `magnitude` of the time unit `from` converted exactly to `to`
    /// (QSpec FR-142). `from` must be a unit of `to`'s own time dimension or
    /// of one a dimension equivalence relates to it: a unit of another
    /// owner's `Time` is another dimension (QSpec FR-142 "Dimensions of
    /// different owners"). Across an equivalence the value passes through
    /// the class root: `from`'s roots, times the size of its dimension's
    /// root in class roots, divided by the size of `to`'s.
    pub fn convert(
        &self,
        magnitude: &Rational,
        from: &str,
        to: &TimeScale,
    ) -> Result<Rational, TimeUnitCause> {
        let scale = self.scale(from)?;
        let (from_class, from_size) = self.classes.class(self.dimension(from)?);
        let (to_class, to_size) = self.classes.class(to.dimension.clone());
        let other = || TimeUnitCause::OtherTimeDimension {
            name: from.to_owned(),
        };
        if from_class != to_class {
            return Err(other());
        }
        let roots = magnitude
            .mul(scale)
            .mul(&from_size)
            .div(&to_size)
            .ok_or_else(other)?;
        Ok(roots.mul(&to.reciprocal))
    }
}

/// The exact scale and composed offset of an FCD unit along its target chain.
pub(crate) fn package_unit_scale(
    start: &PackageUnitRecord,
    units: &[PackageUnitRecord],
) -> Option<(Rational, Rational)> {
    let mut current = start;
    let mut scale = Rational::from_integer(Integer::one());
    let mut offset = Rational::from_integer(Integer::zero());
    let mut seen = BTreeSet::new();
    loop {
        if !seen.insert(current.key.clone()) {
            return None;
        }
        let edge_scale = rational(&current.scale)?;
        let edge_offset = rational(&current.offset)?;
        offset = edge_scale.mul(&offset).add(&edge_offset);
        scale = edge_scale.mul(&scale);
        let Some(target) = &current.target else {
            return Some((scale, offset));
        };
        current = units.iter().find(|unit| unit.key == *target)?;
    }
}

/// The key of a model package's dimension or unit node: package the
/// declaring model package, path `["scalar_type", form, <FCD identity>]` (the
/// identity opaque, never split) and ordinal 0.
fn scalar_type_key(key: &quire_checked::key::DeclarationKey, form: &str) -> Option<NodeKey> {
    crate::check::declared_node_key(
        key.package.clone(),
        vec!["scalar_type".to_owned(), form.to_owned(), key.node.clone()],
        0,
    )
    .ok()
}

/// The nominal nodes of an admitted package unit and its target chain.
fn package_unit_nominal_nodes(
    start: &PackageUnitRecord,
    units: &[PackageUnitRecord],
    dimension: NodeKey,
    identity: &str,
) -> Option<Vec<TimeNominalNode>> {
    fn resolve(
        unit: &PackageUnitRecord,
        units: &[PackageUnitRecord],
        dimension: NodeKey,
        identity: &str,
        seen: &mut BTreeSet<quire_checked::key::DeclarationKey>,
    ) -> Option<Vec<TimeNominalNode>> {
        if !seen.insert(unit.key.clone()) {
            return None;
        }
        let target_nodes = match &unit.target {
            Some(key) => {
                let target = units.iter().find(|candidate| candidate.key == *key)?;
                resolve(target, units, dimension.clone(), identity, seen)?
            }
            None => Vec::new(),
        };
        let target = target_nodes.last().map(|node| node.key.clone());
        rational(&unit.scale)?;
        rational(&unit.offset)?;
        let key = model_declaration_key(&unit.key.package, NodeTag::Model, "unit", &unit.key.node)
            .ok()?;
        let mut nodes = target_nodes;
        nodes.push(TimeNominalNode {
            key,
            form: "unit".to_owned(),
            qualified_declaration: vec![unit.name.clone()],
            owner: NodeOwner::Model(ModelSubject {
                identity: identity.to_owned(),
                node: unit.key.node.clone(),
            }),
            dependencies: std::iter::once(dimension).chain(target).collect(),
        });
        Some(nodes)
    }

    resolve(start, units, dimension, identity, &mut BTreeSet::new())
}

/// A canonical FCD integer or `n/d` coefficient as an exact rational.
pub(crate) fn rational(spelling: &str) -> Option<Rational> {
    let (numerator, denominator) = spelling.split_once('/').unwrap_or((spelling, "1"));
    Rational::new(
        Integer::from_str(numerator).ok()?,
        Integer::from_str(denominator).ok()?,
    )
    .ok()
}

/// A time unit as a conversion target ([`TimeUnits::target`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeScale {
    /// `1 / scale` of the unit to its root.
    reciprocal: Rational,
    /// The exact nominal dimension node of this unit.
    dimension: NodeKey,
    /// The exact nominal node key of the selected time unit.
    unit: NodeKey,
    /// Nominal unit-graph closure required by the timed root.
    nominal_nodes: Vec<TimeNominalNode>,
}

impl TimeScale {
    /// The unit's nominal dimension node.
    pub fn dimension(&self) -> NodeKey {
        self.dimension.clone()
    }

    /// The unit's nominal node key.
    pub fn unit(&self) -> NodeKey {
        self.unit.clone()
    }

    /// Nominal dimension/unit nodes needed by this conversion target.
    pub fn nominal_nodes(&self) -> &[TimeNominalNode] {
        &self.nominal_nodes
    }
}

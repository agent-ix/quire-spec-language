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

use crate::check::lowering::AdmittedModel;
use crate::check::node_key::nominal_key;
use crate::model::domain_package::PackageUnitRecord;
use crate::value::semantic_node::{ModelSubject, NodeOwner};
use crate::value::unit::DimensionPreimage;
use qsl_foundation::IdentityLimits;

use super::refusal::TimeUnitCause;

/// The name FR-230 fixes for the time dimension's base node.
const TIME_DIMENSION: &str = "Time";

/// One declared unit, as FR-230 reads it.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeclaredUnit {
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
        identity: IdentityLimits,
    ) -> Self {
        let table = UnitTable::declared(graph);
        let mut time = None;
        let mut time_dimensions = BTreeMap::new();
        let mut units = BTreeMap::new();
        for key in keys {
            let Some(node) = table.nominal_node(key.as_bytes()) else {
                continue;
            };
            let [name] = node.declaration.qualified_declaration.as_slice() else {
                continue;
            };
            match &node.form {
                NominalUnitForm::Dimension { terms } => {
                    if terms.is_empty() && name == TIME_DIMENSION {
                        time = Some(key);
                    }
                }
                NominalUnitForm::Unit { .. } => {
                    if let Some(unit) = graph.unit(key) {
                        units.insert(
                            name.clone(),
                            DeclaredUnit {
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
            let time_key = dimension.and_then(|dimension| {
                let owner = NodeOwner::Model(ModelSubject {
                    identity: package.identity.clone(),
                    node: dimension.key.node.clone(),
                });
                let preimage =
                    DimensionPreimage::new(owner, vec![dimension.name.clone()], vec![]).ok()?;
                nominal_key(&preimage, identity).ok()
            });
            let prefix = model.alias().unwrap_or(&package.identity);
            if let Some(time_key) = time_key {
                time_dimensions.insert(prefix.to_owned(), time_key);
            }
            for unit in &model.units {
                let Some((scale, offset)) = package_unit_scale(unit, &model.units) else {
                    continue;
                };
                let dimension_key = dimension
                    .zip(time_key)
                    .filter(|(dimension, _)| unit.dimension == dimension.key)
                    .map(|(_, key)| key);
                units.insert(
                    format!("{prefix}::{}", unit.name),
                    DeclaredUnit {
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
        }
    }

    /// The exact scale from the time unit `name` to its root
    /// (`root = scale × name`), or why `name` is not a time unit, in
    /// FR-230's order.
    pub fn scale(&self, name: &str) -> Result<&Rational, TimeUnitCause> {
        let owner_time = name
            .split_once("::")
            .and_then(|(owner, _)| self.time_dimensions.get(owner).copied());
        let time = owner_time
            .or(self.time)
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
        unit.dimension.ok_or(TimeUnitCause::NotTimeDimension {
            name: name.to_owned(),
        })
    }

    /// The time unit `name` as a conversion target: the reciprocal of its
    /// root scale. `Ok(None)` only if the scale is zero, which the unit
    /// graph refuses at admission (an invariant, not a refusal).
    pub fn target(&self, name: &str) -> Result<Option<TimeScale>, TimeUnitCause> {
        let scale = self.scale(name)?;
        let dimension = self.dimension(name)?;
        Ok(Rational::from_integer(Integer::one())
            .div(scale)
            .map(|reciprocal| TimeScale {
                reciprocal,
                dimension,
            }))
    }

    /// `magnitude` of the time unit `from` converted exactly to `to`
    /// (QSpec FR-142).
    pub fn convert(
        &self,
        magnitude: &Rational,
        from: &str,
        to: &TimeScale,
    ) -> Result<Rational, TimeUnitCause> {
        Ok(magnitude.mul(self.scale(from)?).mul(&to.reciprocal))
    }
}

/// The exact scale and composed offset of an FCD unit along its target chain.
fn package_unit_scale(
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

/// A canonical FCD integer or `n/d` coefficient as an exact rational.
fn rational(spelling: &str) -> Option<Rational> {
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
}

impl TimeScale {
    /// The unit's nominal dimension node.
    pub fn dimension(&self) -> NodeKey {
        self.dimension
    }
}

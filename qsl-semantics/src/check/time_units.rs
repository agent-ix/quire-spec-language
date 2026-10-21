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

use std::collections::BTreeMap;

use quire_exact::{Integer, NodeKey, Rational};
use quire_semantic_value::quantity::UnitTable;
use quire_semantic_value::unit::{NominalUnitForm, UnitGraph};

use super::refusal::TimeUnitCause;

/// The name FR-230 fixes for the time dimension's base node.
const TIME_DIMENSION: &str = "Time";

/// One declared unit, as FR-230 reads it.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DeclaredUnit {
    dimension: NodeKey,
    scale: Rational,
    offset: Rational,
}

/// The time units of one checked package's unit graph.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TimeUnits {
    /// The base dimension node named `Time`, if the graph declares one.
    time: Option<NodeKey>,
    /// Every declared unit by its declared name.
    units: BTreeMap<String, DeclaredUnit>,
}

impl TimeUnits {
    /// The time units of `graph`, whose dimension and unit node keys are
    /// `keys` (the assembler's `nominal_spans` keys). The one place a
    /// declared dimension's spelling is read for FR-230's `Time` base.
    #[qsl_attrs::string_edge]
    pub fn new(graph: &UnitGraph, keys: impl IntoIterator<Item = NodeKey>) -> Self {
        let table = UnitTable::declared(graph);
        let mut time = None;
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
                                dimension: unit.dimension_node(),
                                scale: unit.canonical().scale().clone(),
                                offset: unit.canonical().offset().clone(),
                            },
                        );
                    }
                }
            }
        }
        Self { time, units }
    }

    /// The exact scale from the time unit `name` to its root
    /// (`root = scale × name`), or why `name` is not a time unit, in
    /// FR-230's order.
    pub fn scale(&self, name: &str) -> Result<&Rational, TimeUnitCause> {
        let time = self.time.ok_or(TimeUnitCause::NoTimeDimension)?;
        let unit = self
            .units
            .get(name)
            .ok_or_else(|| TimeUnitCause::UnknownUnit {
                name: name.to_owned(),
            })?;
        if unit.dimension != time {
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

    /// The time unit `name` as a conversion target: the reciprocal of its
    /// root scale. `Ok(None)` only if the scale is zero, which the unit
    /// graph refuses at admission (an invariant, not a refusal).
    pub fn target(&self, name: &str) -> Result<Option<TimeScale>, TimeUnitCause> {
        let scale = self.scale(name)?;
        Ok(Rational::from_integer(Integer::one())
            .div(scale)
            .map(|reciprocal| TimeScale { reciprocal }))
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

/// A time unit as a conversion target ([`TimeUnits::target`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeScale {
    /// `1 / scale` of the unit to its root.
    reciprocal: Rational,
}

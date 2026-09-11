// SPDX-License-Identifier: AGPL-3.0-only
//! Exact source correspondence shared with the eventual authored proof seam.

use super::work::{Dimension, Exhaustion, Work};
use super::Site;
use crate::formal_source::FormalSource;
use crate::linking::composed::{binding, models::ModelInput, UnitId};
use crate::Source;

pub(super) struct Correspondence<'s> {
    offered: &'s [FormalSource],
    units: Vec<Option<usize>>,
}

fn bytes(source: &FormalSource) -> usize {
    source.source().text().len()
        + source.source().path().len()
        + source.source().identity().identity.len()
        + source.source().identity().revision.len()
        + source.identity().document().as_str().len()
}

fn same(left: &Source, right: &Source) -> bool {
    left.identity() == right.identity()
        && left.path() == right.path()
        && left.digest() == right.digest()
}

impl<'s> Correspondence<'s> {
    pub(super) fn new(
        binding: &binding::Report<'_>,
        offered: &'s [FormalSource],
        work: &mut Work,
        site: Site,
    ) -> Result<Self, Exhaustion> {
        for source in offered {
            work.charge(Dimension::Bytes, bytes(source), site)?;
        }
        let mut units = Vec::new();
        for unit in binding.namespace().units() {
            work.charge(Dimension::Records, 1, site)?;
            let mut selected = None;
            let mut invalid = false;
            for (index, source) in offered.iter().enumerate() {
                work.charge(Dimension::Constraints, 1, site)?;
                work.charge(
                    Dimension::Bytes,
                    bytes(source) - source.source().text().len()
                        + unit.source().identity().identity.len()
                        + unit.source().identity().revision.len()
                        + unit.source().path().len(),
                    site,
                )?;
                if same(source.source(), unit.source()) {
                    if selected.replace(index).is_some() {
                        invalid = true;
                    }
                    for (other_index, other) in offered.iter().enumerate() {
                        work.charge(Dimension::Constraints, 1, site)?;
                        work.charge(
                            Dimension::Bytes,
                            source.identity().document().as_str().len()
                                + other.identity().document().as_str().len(),
                            site,
                        )?;
                        if index != other_index && source.identity() == other.identity() {
                            invalid = true;
                        }
                    }
                    if let Some(models) = binding.models() {
                        for input in models.inputs() {
                            work.charge(Dimension::Constraints, 1, site)?;
                            match input {
                                ModelInput::Native(model) => {
                                    work.charge(
                                        Dimension::Bytes,
                                        source.identity().document().as_str().len()
                                            + model.source().identity().document().as_str().len(),
                                        site,
                                    )?;
                                    if source.identity() == model.source().identity()
                                        && !same(source.source(), model.source().source())
                                    {
                                        invalid = true;
                                    }
                                }
                                ModelInput::UnsupportedProducer { .. } => {}
                            }
                        }
                    }
                }
            }
            units.push(if invalid { None } else { selected });
        }
        Ok(Self { offered, units })
    }
    pub(super) fn get(&self, unit: UnitId) -> Option<&'s FormalSource> {
        self.units
            .get(unit.index())
            .copied()
            .flatten()
            .map(|index| &self.offered[index])
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact source correspondence shared with the eventual authored proof seam.

use super::work::{Dimension, Exhaustion, Work};
use super::Site;
use crate::formal_source::FormalSource;
use crate::linking::composed::{binding, UnitId};
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
        Self::collect(binding, offered, |dimension, amount| {
            work.charge(dimension, amount, site)
        })
    }
    // Share correspondence checks with the proof stage's source-owned meter.
    pub(super) fn collect<E>(
        binding: &binding::Report<'_>,
        offered: &'s [FormalSource],
        mut charge: impl FnMut(Dimension, usize) -> Result<(), E>,
    ) -> Result<Self, E> {
        for source in offered {
            charge(Dimension::Bytes, bytes(source))?;
        }
        let mut units = Vec::new();
        for unit in binding.namespace().units() {
            charge(Dimension::Records, 1)?;
            let mut selected = None;
            let mut invalid = false;
            for (index, source) in offered.iter().enumerate() {
                charge(Dimension::Constraints, 1)?;
                charge(
                    Dimension::Bytes,
                    bytes(source) - source.source().text().len()
                        + unit.source().identity().identity.len()
                        + unit.source().identity().revision.len()
                        + unit.source().path().len(),
                )?;
                if same(source.source(), unit.source()) {
                    if selected.replace(index).is_some() {
                        invalid = true;
                    }
                    for (other_index, other) in offered.iter().enumerate() {
                        charge(Dimension::Constraints, 1)?;
                        charge(
                            Dimension::Bytes,
                            source.identity().document().as_str().len()
                                + other.identity().document().as_str().len(),
                        )?;
                        if index != other_index && source.identity() == other.identity() {
                            invalid = true;
                        }
                    }
                    if let Some(models) = binding.models() {
                        for input in models.inputs() {
                            charge(Dimension::Constraints, 1)?;
                            if let Some(model) = (*input).native_model() {
                                charge(
                                    Dimension::Bytes,
                                    source.identity().document().as_str().len()
                                        + model.source().identity().document().as_str().len(),
                                )?;
                                if source.identity() == model.source().identity()
                                    && !same(source.source(), model.source().source())
                                {
                                    invalid = true;
                                }
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
    pub(super) fn index(&self, unit: UnitId) -> Option<usize> {
        self.units.get(unit.index()).copied().flatten()
    }
}

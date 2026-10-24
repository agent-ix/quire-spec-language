// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-025 MA-5 and TSO-1: the one cell a field location names, read and
//! written in place, for a memory model that keeps a shared location's value
//! apart from the model state.

use super::walk::{absent_rows, rows_of, Row};
use super::{ModelSimState, ModelSystem};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::footprint::Location;
use quire_checked::key::DeclarationKey;
use quire_exact::Value;

/// One field of one object of a model state: the population and key that
/// hold the object, and the field's declaration key and name.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Cell {
    /// The population that holds the object.
    pub population: DeclarationKey,
    /// The object's key in it.
    pub key: String,
    /// The field's declaration key.
    pub field: DeclarationKey,
    /// The field's declared name.
    pub name: String,
}

impl ModelSimState {
    /// The value `cell` holds; `None` when the object is absent or leaves
    /// the field absent.
    pub fn cell(&self, cell: &Cell) -> Option<&Value> {
        self.state
            .populations
            .get(&cell.population)?
            .get(&cell.key)?
            .fields
            .get(&cell.field)
    }

    /// This state with `cell` holding `value`; an absent object is left
    /// absent, and `None` leaves the field absent.
    pub(crate) fn set_cell(&mut self, cell: &Cell, value: Option<Value>) {
        let Some(object) = self
            .state
            .populations
            .get_mut(&cell.population)
            .and_then(|objects| objects.get_mut(&cell.key))
        else {
            return;
        };
        match value {
            Some(value) => {
                object.fields.insert(cell.field.clone(), value);
            }
            None => {
                object.fields.remove(&cell.field);
            }
        }
    }
}

impl ModelSystem<'_> {
    /// The cell the field location `location` names at `state`: the object
    /// the state holds, or a universe key could name, with that reference.
    /// `None` for a location that is not one object's field, or whose object
    /// no row names.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the state cannot be walked.
    pub(crate) fn cell_of(
        &self,
        state: &ModelSimState,
        location: &Location,
    ) -> Result<Option<Cell>, ExpansionStop> {
        let Location::Field { object, field } = location else {
            return Ok(None);
        };
        let mut rows: Vec<Row> = rows_of(self, &state.state)?;
        rows.extend(absent_rows(self, state));
        let Some(row) = rows.into_iter().find(|row| row.reference == *object) else {
            return Ok(None);
        };
        let name = self
            .fields
            .values()
            .flatten()
            .find(|declared| declared.key == *field)
            .map(|declared| declared.name.clone());
        Ok(name.map(|name| Cell {
            population: row.population,
            key: row.key,
            field: field.clone(),
            name,
        }))
    }
}

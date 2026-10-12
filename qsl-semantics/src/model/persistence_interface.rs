// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 persistence-interface binder.

use crate::model::intake::{meaning, unsupported_at};
use crate::model::normalize::ModelRefusal;

use super::meaning_dispatch::MeaningBindingContext;

pub(super) fn bind(context: &mut MeaningBindingContext<'_, '_>) -> Result<(), ModelRefusal> {
    Err(unsupported_at(
        context.type_value,
        context.at,
        meaning::PERSISTENCE_INTERFACE,
    ))
}

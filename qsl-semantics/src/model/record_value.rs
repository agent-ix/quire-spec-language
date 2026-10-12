// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 record-value-type binder.

use crate::model::intake::read_record_value_type;
use crate::model::normalize::ModelRefusal;

use super::meaning_dispatch::MeaningBindingContext;

pub(super) fn bind(context: &mut MeaningBindingContext<'_, '_>) -> Result<(), ModelRefusal> {
    read_record_value_type(
        context.package,
        context.type_value,
        context.node,
        context.at,
        context.type_meanings,
        context.records,
        context.pending,
    )
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 tuple value type binder.
#![allow(
    clippy::result_large_err,
    reason = "intake refusal path follows the model's typed refusal contract"
)]

use crate::model::intake::read_tuple_value_type;
use crate::model::normalize::Refusals;

use super::meaning_dispatch::MeaningBindingContext;

pub(super) fn bind(context: &mut MeaningBindingContext<'_, '_>) -> Result<(), Refusals> {
    read_tuple_value_type(
        context.package,
        context.type_value,
        context.node,
        context.at,
        context.construct_shape,
        context.meanings_by_node,
        context.records,
    )
}

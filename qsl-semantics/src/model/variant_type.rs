// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 variant-type binder.
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, the typed-failure convention of the checked-term result categories"
)]

use crate::model::intake::read_variant_type;
use crate::model::normalize::Refusals;

use super::meaning_dispatch::MeaningBindingContext;

pub(super) fn bind(context: &mut MeaningBindingContext<'_, '_>) -> Result<(), Refusals> {
    read_variant_type(
        context.package,
        context.type_value,
        context.node,
        context.at,
        context.construct_shape,
        context.meanings_by_node,
        context.records,
    )
}

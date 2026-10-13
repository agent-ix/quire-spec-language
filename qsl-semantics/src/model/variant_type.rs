// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 variant-type binder.
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use crate::model::intake::read_variant_type;
use crate::model::normalize::ModelRefusal;

use super::meaning_dispatch::MeaningBindingContext;

pub(super) fn bind(context: &mut MeaningBindingContext<'_, '_>) -> Result<(), ModelRefusal> {
    let Err(mut found) = read_variant_type(
        context.package,
        context.type_value,
        context.node,
        context.at,
        context.construct_shape,
        context.records,
    ) else {
        return Ok(());
    };
    // `found` is never empty; the binder reports one refusal and the rest
    // ride in `more_refusals`, in report order.
    let first = found.remove(0);
    context.more_refusals.append(&mut found);
    Err(first)
}

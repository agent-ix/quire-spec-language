// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 persistence-interface binder.

use crate::model::intake::{read_persistence_interface, NodeCtx};
use crate::model::normalize::ModelRefusal;

use super::meaning_dispatch::MeaningBindingContext;

pub(super) fn bind(context: &mut MeaningBindingContext<'_, '_>) -> Result<(), ModelRefusal> {
    let node_context = NodeCtx::new(context.type_value, context.at);
    read_persistence_interface(
        context.package,
        context.node,
        &node_context,
        context.type_meanings,
        context.records,
        context.pending,
    )
}

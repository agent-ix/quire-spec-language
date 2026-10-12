// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-208 event-type binder.

use crate::model::domain_package::{DomainPackageRecord, EventTypeRecord};
use crate::model::intake::{read_record_value_type, NodeCtx};
use crate::model::key::DeclarationKey;
use crate::model::normalize::ModelRefusal;

use super::meaning_dispatch::MeaningBindingContext;

pub(super) fn bind(context: &mut MeaningBindingContext<'_, '_>) -> Result<(), ModelRefusal> {
    let ctx = NodeCtx::new(context.type_value, context.at);
    let identity_fields = ctx.identity_keys(context.package, "identityFields")?;
    let supertypes = ctx.identity_keys(context.package, "supertypes")?;
    let occurrence = ctx.str_field("occurrenceField")?;

    // Reuse the shared field, operation and clause readers, but do not let
    // the record-value reader reinterpret event-only identity/supertype
    // properties. The event record below retains those authored properties.
    let mut shared = context.type_value.clone();
    let object = shared.as_object_mut().expect("admitted type is an object");
    object.remove("identityFields");
    object.remove("supertypes");
    read_record_value_type(
        context.package,
        &shared,
        context.node,
        context.at,
        context.type_meanings,
        context.records,
        context.pending,
    )?;
    let key = DeclarationKey {
        package: context.package.to_owned(),
        node: context.node.to_owned(),
    };
    let Some(record) = context.records.iter_mut().find(
        |record| matches!(record, DomainPackageRecord::RecordValueType(value) if value.key == key),
    ) else {
        unreachable!("shared record reader adds the temporary type record")
    };
    *record = DomainPackageRecord::EventType(EventTypeRecord {
        key,
        identity_fields,
        occurrence_field: Some(DeclarationKey {
            package: context.package.to_owned(),
            node: occurrence.to_owned(),
        }),
        supertypes,
    });
    Ok(())
}

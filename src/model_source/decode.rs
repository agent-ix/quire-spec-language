// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-025: bounded entry decoding with original declaration occurrences.

use super::wire::{
    Enumeration, Field, Object, Operation, Record, RuleModel, Scalar, ScalarV1, Value,
};
use super::{EntryKind, ModelSourceCause, ModelSourceLimits, Result};
use crate::formal_source::FormalSource;
use crate::located_json::{self, Located};
use crate::native_model::NativeModelProfile;
use qsl_foundation::serde_object::Object as JsonObject;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::value::RawValue;

pub(super) struct DecodedModel {
    pub(super) license: String,
    pub(super) package: String,
    pub(super) requirement: String,
    pub(super) revision: u64,
    pub(super) scalars: Vec<Located<Scalar>>,
    pub(super) records: Vec<Located<DecodedRecord>>,
    pub(super) enums: Vec<Located<DecodedEnum>>,
    pub(super) values: Vec<Located<Value>>,
    pub(super) objects: Vec<Located<Object>>,
    pub(super) operations: Vec<Located<DecodedOperation>>,
}

pub(super) struct DecodedRecord {
    pub(super) name: String,
    pub(super) fields: Vec<Located<Field>>,
}

pub(super) struct DecodedEnum {
    pub(super) name: String,
    pub(super) variants: Vec<Located<String>>,
}

fn decode_items<T: DeserializeOwned>(
    source: &FormalSource,
    items: Vec<&RawValue>,
) -> Result<Vec<Located<T>>> {
    items
        .into_iter()
        .map(|raw| decode_record(source, raw))
        .collect()
}

pub(super) fn decode_model(
    source: &FormalSource,
    limits: ModelSourceLimits,
    profile: NativeModelProfile,
) -> Result<DecodedModel> {
    let JsonObject(input): JsonObject<RuleModel<'_>> =
        located_json::read(source, limits.source_bytes)?;
    let mut budget = Budget::new(limits.entries);
    for count in [
        input.scalars.len(),
        input.records.len(),
        input.enums.len(),
        input.values.len(),
        input.objects.len(),
        input.operations.len(),
    ] {
        budget.take(count, EntryKind::Declaration)?;
    }
    let mut records = Vec::new();
    for raw in input.records {
        let Located {
            value: record,
            source: span,
        } = decode_record::<Record<'_>>(source, raw)?;
        budget.take(record.fields.len(), EntryKind::Field)?;
        records.push(Located {
            value: DecodedRecord {
                name: record.name,
                fields: decode_items(source, record.fields)?,
            },
            source: span,
        });
    }
    let mut enums = Vec::new();
    for raw in input.enums {
        let Located {
            value: enumeration,
            source: span,
        } = decode_record::<Enumeration<'_>>(source, raw)?;
        budget.take(enumeration.variants.len(), EntryKind::Variant)?;
        enums.push(Located {
            value: DecodedEnum {
                name: enumeration.name,
                variants: enumeration
                    .variants
                    .into_iter()
                    .map(|raw| located_json::decode(source, raw).map_err(ModelSourceCause::from))
                    .collect::<Result<_>>()?,
            },
            source: span,
        });
    }
    let mut operations = Vec::new();
    for raw in input.operations {
        let Located {
            value,
            source: span,
        } = decode_record::<Operation<'_>>(source, raw)?;
        // Charge each byte-bounded raw group before decoding its strings/tuples,
        // and finish this operation before inspecting the next.
        let parameters = decode_values(value.parameters, &mut budget, EntryKind::Parameter)?;
        let fields = decode_values(value.frame.fields, &mut budget, EntryKind::FrameField)?;
        let created = decode_values(value.frame.created, &mut budget, EntryKind::Created)?;
        let deleted = decode_values(value.frame.deleted, &mut budget, EntryKind::Deleted)?;
        operations.push(Located {
            value: DecodedOperation {
                name: value.name,
                context: value.context,
                anchor: value.anchor,
                parameters,
                result: value.result,
                frame: DecodedFrame {
                    fields,
                    created,
                    deleted,
                },
            },
            source: span,
        });
    }
    Ok(DecodedModel {
        license: input.license,
        package: input.package,
        requirement: input.requirement,
        revision: input.revision,
        scalars: match profile {
            NativeModelProfile::V1 => decode_items::<ScalarV1>(source, input.scalars)?
                .into_iter()
                .map(|Located { value, source }| Located {
                    value: value.into(),
                    source,
                })
                .collect(),
            NativeModelProfile::V2 => decode_items(source, input.scalars)?,
        },
        records,
        enums,
        values: decode_items(source, input.values)?,
        objects: decode_items(source, input.objects)?,
        operations,
    })
}

pub(super) struct DecodedOperation {
    pub(super) name: String,
    pub(super) context: String,
    pub(super) anchor: String,
    pub(super) parameters: Vec<String>,
    pub(super) result: Option<String>,
    pub(super) frame: DecodedFrame,
}

pub(super) struct DecodedFrame {
    pub(super) fields: Vec<(String, String)>,
    pub(super) created: Vec<String>,
    pub(super) deleted: Vec<String>,
}

struct Budget {
    maximum: usize,
    remaining: usize,
}

impl Budget {
    fn new(maximum: usize) -> Self {
        Self {
            maximum,
            remaining: maximum,
        }
    }

    fn take(&mut self, requested: usize, kind: EntryKind) -> Result<()> {
        self.remaining =
            self.remaining
                .checked_sub(requested)
                .ok_or(ModelSourceCause::Entries {
                    kind,
                    requested,
                    remaining: self.remaining,
                    maximum: self.maximum,
                })?;
        Ok(())
    }
}

fn decode_values<T: DeserializeOwned>(
    items: Vec<&RawValue>,
    budget: &mut Budget,
    kind: EntryKind,
) -> Result<Vec<T>> {
    budget.take(items.len(), kind)?;
    items
        .into_iter()
        .map(|raw| {
            serde_json::from_str(raw.get()).map_err(|error| ModelSourceCause::Decode(error.into()))
        })
        .collect()
}

fn decode_record<'de, T: Deserialize<'de>>(
    source: &FormalSource,
    raw: &'de RawValue,
) -> Result<Located<T>> {
    let Located {
        value: JsonObject(value),
        source,
    } = located_json::decode::<JsonObject<T>>(source, raw)?;
    Ok(Located { value, source })
}

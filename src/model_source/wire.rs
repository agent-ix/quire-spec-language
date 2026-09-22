// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-025: closed wire shapes, borrowing undecoded entry groups from the source.

use qsl_foundation::serde_object::{deserialize_empty_object, from_object as deserialize_object};
use quire_contract_ir as ir;
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RuleModel<'a> {
    pub(super) license: String,
    pub(super) package: String,
    pub(super) requirement: String,
    pub(super) revision: u64,
    #[serde(borrow)]
    pub(super) scalars: Vec<&'a RawValue>,
    #[serde(borrow)]
    pub(super) records: Vec<&'a RawValue>,
    #[serde(default, borrow)]
    pub(super) enums: Vec<&'a RawValue>,
    #[serde(borrow)]
    pub(super) values: Vec<&'a RawValue>,
    #[serde(borrow)]
    pub(super) objects: Vec<&'a RawValue>,
    #[serde(borrow)]
    pub(super) operations: Vec<&'a RawValue>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ScalarV1 {
    Integer {
        name: String,
        minimum: i64,
        maximum: i64,
        unit: Option<String>,
    },
    Text {
        name: String,
        max_scalars: u32,
    },
}

/// The /2 scalar extension; all other source sections share the /1 wire types.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Scalar {
    Integer {
        name: String,
        minimum: i64,
        maximum: i64,
        unit: Option<String>,
    },
    Text {
        name: String,
        max_scalars: u32,
    },
    Rational {
        name: String,
        numerator_minimum: i64,
        numerator_maximum: i64,
        maximum_denominator: u64,
        unit: Option<String>,
    },
}

impl From<ScalarV1> for Scalar {
    fn from(value: ScalarV1) -> Self {
        match value {
            ScalarV1::Integer {
                name,
                minimum,
                maximum,
                unit,
            } => Self::Integer {
                name,
                minimum,
                maximum,
                unit,
            },
            ScalarV1::Text { name, max_scalars } => Self::Text { name, max_scalars },
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record<'a> {
    pub(super) name: String,
    #[serde(borrow)]
    pub(super) fields: Vec<&'a RawValue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Field {
    pub(super) name: String,
    #[serde(rename = "type")]
    #[serde(deserialize_with = "deserialize_object")]
    pub(super) ty: Type,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Enumeration<'a> {
    pub(super) name: String,
    #[serde(borrow)]
    pub(super) variants: Vec<&'a RawValue>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Value {
    pub(super) name: String,
    pub(super) kind: ir::ValueDeclarationKind,
    #[serde(rename = "type")]
    #[serde(deserialize_with = "deserialize_object")]
    pub(super) ty: Type,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Type {
    #[serde(deserialize_with = "deserialize_empty_object")]
    Boolean,
    Scalar {
        name: String,
    },
    Record {
        name: String,
    },
    Enum {
        name: String,
    },
    Option {
        #[serde(deserialize_with = "deserialize_object")]
        value: Box<Type>,
    },
    Sequence {
        maximum: u32,
        #[serde(deserialize_with = "deserialize_object")]
        value: Box<Type>,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Object {
    pub(super) record: String,
    pub(super) reference: String,
    pub(super) identity_field: String,
    pub(super) universe: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Operation<'a> {
    pub(super) name: String,
    pub(super) context: String,
    pub(super) anchor: String,
    #[serde(borrow)]
    pub(super) parameters: Vec<&'a RawValue>,
    pub(super) result: Option<String>,
    #[serde(borrow, deserialize_with = "deserialize_object")]
    pub(super) frame: FrameData<'a>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FrameData<'a> {
    #[serde(borrow)]
    pub(super) fields: Vec<&'a RawValue>,
    #[serde(borrow)]
    pub(super) created: Vec<&'a RawValue>,
    #[serde(borrow)]
    pub(super) deleted: Vec<&'a RawValue>,
}

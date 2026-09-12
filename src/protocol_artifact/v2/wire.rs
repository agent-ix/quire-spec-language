// SPDX-License-Identifier: AGPL-3.0-only
//! FR-050: the closed version-2 temporal binding delta.

use crate::protocol_artifact::wire as v1;
use crate::serde_object::from_object;
use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

/// Exact clock configuration selected for one temporal declaration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum ClockConfiguration {
    /// One admitted semantic-event sequence authority.
    #[serde(rename = "event_position")]
    EventPosition {
        /// Exact sequence-authority name.
        sequence_authority: String,
    },
    /// Exact epoch, positive period and unit for a fixed sample sequence.
    #[serde(rename = "fixed_sample")]
    FixedSample {
        /// Exact origin in the selected unit domain.
        epoch: v1::Number,
        /// Positive reduced sampling period.
        period: v1::Number,
        /// Exact clock-unit name shared by epoch and period.
        unit: String,
    },
    /// Unit of admitted timestamps in one finite window.
    #[serde(rename = "timestamped_event")]
    TimestampedEvent {
        /// Exact timestamp-unit name.
        timestamp_unit: String,
    },
}

impl<'de> Deserialize<'de> for ClockConfiguration {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "kind", deny_unknown_fields)]
        enum Fields {
            #[serde(rename = "event_position")]
            EventPosition { sequence_authority: String },
            #[serde(rename = "fixed_sample")]
            FixedSample {
                epoch: v1::Number,
                period: v1::Number,
                unit: String,
            },
            #[serde(rename = "timestamped_event")]
            TimestampedEvent { timestamp_unit: String },
        }
        Ok(match from_object(decoder)? {
            Fields::EventPosition { sequence_authority } => {
                Self::EventPosition { sequence_authority }
            }
            Fields::FixedSample {
                epoch,
                period,
                unit,
            } => Self::FixedSample {
                epoch,
                period,
                unit,
            },
            Fields::TimestampedEvent { timestamp_unit } => {
                Self::TimestampedEvent { timestamp_unit }
            }
        })
    }
}

/// One temporal declaration's exact definition and clock selection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TemporalBinding {
    /// Index of the selected temporal declaration.
    pub declaration: u32,
    /// Definition index, equal to the declaration's profile index.
    pub definition: u32,
    /// Exact configuration alternative required by that definition.
    pub clock: ClockConfiguration,
}

impl<'de> Deserialize<'de> for TemporalBinding {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            declaration: u32,
            definition: u32,
            clock: ClockConfiguration,
        }
        let Fields {
            declaration,
            definition,
            clock,
        } = from_object(decoder)?;
        Ok(Self {
            declaration,
            definition,
            clock,
        })
    }
}

/// Version-2 package. `inherited` is serialized flat before the required delta.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    /// Complete version-1-shaped package graph with version-2 headers.
    pub inherited: v1::Package,
    /// Complete declaration-indexed temporal selection table.
    pub temporal_bindings: Vec<TemporalBinding>,
}

impl Serialize for Package {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let p = &self.inherited;
        let mut object = serializer.serialize_struct("CompiledProtocolPackage", 19)?;
        object.serialize_field("wire", &p.wire)?;
        object.serialize_field("media", &p.media)?;
        object.serialize_field("schema", &p.schema)?;
        object.serialize_field("type", &p.package_type)?;
        object.serialize_field("encoding", &p.encoding)?;
        object.serialize_field("numeric", &p.numeric)?;
        object.serialize_field("contract", &p.contract)?;
        object.serialize_field("producer", &p.producer)?;
        object.serialize_field("baseline", &p.baseline)?;
        object.serialize_field("language", &p.language)?;
        object.serialize_field("package_definition", &p.package_definition)?;
        object.serialize_field("features", &p.features)?;
        object.serialize_field("sources", &p.sources)?;
        object.serialize_field("dependencies", &p.dependencies)?;
        object.serialize_field("definitions", &p.definitions)?;
        object.serialize_field("models", &p.models)?;
        object.serialize_field("types", &p.types)?;
        object.serialize_field("declarations", &p.declarations)?;
        object.serialize_field("temporal_bindings", &self.temporal_bindings)?;
        object.end()
    }
}

impl<'de> Deserialize<'de> for Package {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            wire: String,
            media: String,
            schema: String,
            #[serde(rename = "type")]
            package_type: String,
            encoding: String,
            numeric: String,
            contract: v1::ArtifactRef,
            producer: v1::Producer,
            baseline: v1::ArtifactRef,
            language: v1::Language,
            package_definition: u32,
            features: v1::Features,
            sources: Vec<v1::Source>,
            dependencies: Vec<v1::Dependency>,
            definitions: Vec<v1::Definition>,
            models: Vec<v1::Model>,
            types: Vec<v1::Type>,
            declarations: Vec<v1::Declaration>,
            temporal_bindings: Vec<TemporalBinding>,
        }
        let fields: Fields = from_object(decoder)?;
        Ok(Self {
            inherited: v1::Package {
                wire: fields.wire,
                media: fields.media,
                schema: fields.schema,
                package_type: fields.package_type,
                encoding: fields.encoding,
                numeric: fields.numeric,
                contract: fields.contract,
                producer: fields.producer,
                baseline: fields.baseline,
                language: fields.language,
                package_definition: fields.package_definition,
                features: fields.features,
                sources: fields.sources,
                dependencies: fields.dependencies,
                definitions: fields.definitions,
                models: fields.models,
                types: fields.types,
                declarations: fields.declarations,
            },
            temporal_bindings: fields.temporal_bindings,
        })
    }
}

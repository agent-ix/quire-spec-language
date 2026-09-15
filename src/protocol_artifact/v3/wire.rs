// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-054: the closed version-3 control-to-temporal mapping delta.

use crate::protocol_artifact::{v2, wire as v1};
use crate::serde_object::from_object;
use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};

/// One declaration-local event control's exact authored temporal selection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ActivationMapping {
    /// Exact in-graph control handle; this is not a source position or name.
    pub control: v1::Handle,
    /// Exact selected temporal declaration index.
    pub temporal_declaration: u32,
}

impl<'de> Deserialize<'de> for ActivationMapping {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            control: v1::Handle,
            temporal_declaration: u32,
        }
        let Fields {
            control,
            temporal_declaration,
        } = from_object(decoder)?;
        Ok(Self {
            control,
            temporal_declaration,
        })
    }
}

/// Version-3 package. The frozen v2 fields are serialized flat before its delta.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Package {
    /// Complete v1-shaped graph, with version-3 headers.
    pub inherited: v1::Package,
    /// The retained complete v2 temporal selection table.
    pub temporal_bindings: Vec<v2::wire::TemporalBinding>,
    /// Complete authored control-to-temporal activation table.
    pub activation_mappings: Vec<ActivationMapping>,
}

impl Serialize for Package {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let p = &self.inherited;
        let mut object = serializer.serialize_struct("CompiledProtocolPackage", 20)?;
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
        object.serialize_field("activation_mappings", &self.activation_mappings)?;
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
            temporal_bindings: Vec<v2::wire::TemporalBinding>,
            activation_mappings: Vec<ActivationMapping>,
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
            activation_mappings: fields.activation_mappings,
        })
    }
}

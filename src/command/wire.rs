// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026/027: closed command records; existing constructors own identifier validity.

use crate::checking::ClauseBinding;
use crate::runtime::{
    ExecutionSelection, InvocationRef, ObjectIdentity, ObservationSelection, SnapshotRef,
};
use crate::serde_object::{deserialize_objects, from_object};
use quire_contract_ir as ir;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// The selected request shape owns its admitted envelope format.
pub(super) trait RequestKind: serde::de::DeserializeOwned {
    const FORMAT: crate::wire_format::WireFormat;
}

impl RequestKind for Request {
    const FORMAT: crate::wire_format::WireFormat = crate::wire_format::WireFormat::RunRequest;
}

impl RequestKind for CompileRequest {
    const FORMAT: crate::wire_format::WireFormat = crate::wire_format::WireFormat::CompileRequest;
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Envelope<'a> {
    pub format: String,
    #[serde(borrow)]
    pub request: &'a RawValue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompileRequest {
    #[serde(deserialize_with = "deserialize_objects")]
    pub models: Vec<Model>,
    #[serde(deserialize_with = "from_object")]
    pub program: Program,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    #[serde(deserialize_with = "deserialize_objects")]
    pub models: Vec<Model>,
    #[serde(deserialize_with = "from_object")]
    pub program: Program,
    #[serde(deserialize_with = "deserialize_objects")]
    pub snapshots: Vec<FileSelection<SnapshotRef>>,
    #[serde(deserialize_with = "deserialize_objects")]
    pub invocations: Vec<FileSelection<InvocationRef>>,
    #[serde(deserialize_with = "from_object")]
    pub selection: Selection,
    #[serde(default, deserialize_with = "from_object")]
    pub limits: Limits,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceFile {
    pub file: String,
    pub identity: String,
    pub revision: String,
    pub digest: String,
    pub document: String,
    pub formal_revision: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Model {
    pub format: String,
    #[serde(deserialize_with = "from_object")]
    pub source: SourceFile,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Program {
    #[serde(deserialize_with = "from_object")]
    pub source: SourceFile,
    #[serde(deserialize_with = "deserialize_objects")]
    pub clauses: Vec<Binding>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub(super) struct FileSelection<T> {
    pub file: String,
    #[serde(deserialize_with = "from_object")]
    pub reference: T,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Owner {
    pub package: String,
    pub requirement: String,
    pub revision: u64,
}

impl Owner {
    pub fn bind(&self) -> Result<ir::RequirementRef, ir::Diagnostic> {
        ir::RequirementRef::parse(&self.package, &self.requirement, self.revision)
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Point {
    Initialization { name: String },
    Handler { name: String },
    Pre { operation: String },
    Post { operation: String },
}

impl Point {
    fn bind(&self) -> Result<ir::ExecutionPoint, ir::Diagnostic> {
        Ok(match self {
            Self::Initialization { name } => ir::ExecutionPoint::Initialization {
                name: ir::AnchorName::new(name)?,
            },
            Self::Handler { name } => ir::ExecutionPoint::Handler {
                name: ir::AnchorName::new(name)?,
            },
            Self::Pre { operation } => ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new(operation)?,
            },
            Self::Post { operation } => ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new(operation)?,
            },
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub name: String,
    #[serde(deserialize_with = "from_object")]
    pub owner: Owner,
    pub clause: String,
    #[serde(deserialize_with = "from_object")]
    pub point: Point,
}

impl Binding {
    pub fn bind(&self) -> Result<ClauseBinding, ir::Diagnostic> {
        Ok(ClauseBinding {
            name: self.name.clone(),
            requirement: self.owner.bind()?,
            clause: ir::ClauseId::new(&self.clause)?,
            execution_point: self.point.bind()?,
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Observation {
    Current {
        #[serde(deserialize_with = "from_object")]
        snapshot: SnapshotRef,
        #[serde(deserialize_with = "from_object")]
        self_object: ObjectIdentity,
    },
    Invocation {
        #[serde(deserialize_with = "from_object")]
        invocation: InvocationRef,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    #[serde(deserialize_with = "from_object")]
    pub owner: Owner,
    pub clause: String,
    #[serde(deserialize_with = "from_object")]
    pub observation: Observation,
}

impl Selection {
    pub fn bind(&self) -> Result<ExecutionSelection, ir::Diagnostic> {
        Ok(ExecutionSelection {
            requirement: self.owner.bind()?,
            clause: ir::ClauseId::new(&self.clause)?,
            observation: match &self.observation {
                Observation::Current {
                    snapshot,
                    self_object,
                } => ObservationSelection::Current {
                    snapshot: snapshot.clone(),
                    self_object: self_object.clone(),
                },
                Observation::Invocation { invocation } => ObservationSelection::Invocation {
                    invocation: invocation.clone(),
                },
            },
        })
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct Limits {
    pub validation_work: Option<usize>,
    pub expression_steps: Option<usize>,
}

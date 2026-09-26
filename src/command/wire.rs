// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-026–028, FR-031: closed records; existing constructors own identifier validity.

use crate::checking::ClauseBinding;
use crate::runtime::{
    ExecutionSelection, InvocationRef, ObjectIdentity, ObservationSelection, SnapshotRef,
};
use qsl_foundation::serde_object::{deserialize_objects, from_object};
use quire_contract_ir as ir;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// The selected request shape owns its admitted envelope format.
pub(super) trait RequestKind: serde::de::DeserializeOwned {
    const FORMAT: qsl_foundation::wire_format::WireFormat;
}

impl RequestKind for Request {
    const FORMAT: qsl_foundation::wire_format::WireFormat =
        qsl_foundation::wire_format::WireFormat::RunRequest;
}

impl RequestKind for CompileRequest {
    const FORMAT: qsl_foundation::wire_format::WireFormat =
        qsl_foundation::wire_format::WireFormat::CompileRequest;
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
    /// A `1-draft` program's supplied libraries (FR-027, ADR-015 D-1); none
    /// when absent.
    #[serde(default, deserialize_with = "deserialize_objects")]
    pub libraries: Vec<Library>,
}

/// One supplied library: its identity, version and source selection.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Library {
    pub identity: String,
    pub version: String,
    #[serde(deserialize_with = "from_object")]
    pub source: SourceFile,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    #[serde(default, deserialize_with = "selected_package")]
    pub package: Option<SelectedPackage>,
    #[serde(deserialize_with = "deserialize_objects")]
    pub models: Vec<Model>,
    #[serde(deserialize_with = "from_object")]
    pub program: Program,
    // FR-026's native-only members. A `0-draft` program (or one declaring
    // no edition) requires each; a `1-draft` program admits none of them
    // (FR-100), so each is optional here and the edition-specific
    // requirement is applied after the program's declared edition is read
    // (`RunSelection::new`/`native_only`).
    #[serde(default, deserialize_with = "deserialize_objects_opt")]
    pub snapshots: Option<Vec<FileSelection<SnapshotRef>>>,
    #[serde(default, deserialize_with = "deserialize_objects_opt")]
    pub invocations: Option<Vec<FileSelection<InvocationRef>>>,
    #[serde(default, deserialize_with = "from_object_opt")]
    pub selection: Option<Selection>,
    #[serde(default, deserialize_with = "from_object")]
    pub limits: Limits,
    /// FR-100: a `1-draft` program's named call. Absent for a `0-draft`
    /// program.
    #[serde(default, deserialize_with = "from_object_opt")]
    pub call: Option<Call>,
    /// FR-100/ADR-015 D-1: a `1-draft` program's supplied libraries; none
    /// when absent. A `0-draft` program admits none (FR-026).
    #[serde(default, deserialize_with = "deserialize_objects")]
    pub libraries: Vec<Library>,
}

// Present only when the field itself is present; `#[serde(default)]` alone
// supplies `None` when the JSON key is absent (`from_object`/
// `deserialize_objects` are only ever invoked for a present key).
fn from_object_opt<'de, D: serde::Deserializer<'de>, T: serde::Deserialize<'de>>(
    decoder: D,
) -> Result<Option<T>, D::Error> {
    from_object(decoder).map(Some)
}

fn deserialize_objects_opt<'de, D: serde::Deserializer<'de>, T: serde::Deserialize<'de>>(
    decoder: D,
) -> Result<Option<Vec<T>>, D::Error> {
    deserialize_objects(decoder).map(Some)
}

/// The `call` member of a `1-draft` native-run/1 request (FR-100): which
/// function to call, its arguments, and an optional `work_units` limit.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Call {
    pub function: String,
    #[serde(deserialize_with = "deserialize_objects")]
    pub arguments: Vec<Argument>,
    /// FR-100: a JSON integer from 0 to `u64::MAX` when present. A present
    /// `null` is not admitted: unlike `from_object_opt`/
    /// `deserialize_objects_opt` (wrapped so `#[serde(default)]` alone
    /// covers absence), this deserializes `u64` directly -- so serde only
    /// ever calls it for a present key, and a present `null` fails `u64`'s
    /// own visitor rather than silently becoming the default.
    #[serde(default, deserialize_with = "work_units")]
    pub work_units: Option<u64>,
}

fn work_units<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<Option<u64>, D::Error> {
    u64::deserialize(decoder).map(Some)
}

/// One `{parameter, value}` argument of a [`Call`] (FR-100): `value` is a
/// JSON integer in the signed 64-bit range, read as `i64` so an out-of-range
/// or non-integer JSON value refuses at decode (`invalid-request`), never
/// reaching `qsl_replay::spine::run`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Argument {
    pub parameter: String,
    pub value: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SelectedPackage {
    pub file: String,
    pub digest: String,
}

// Omission selects compilation; an explicitly supplied value must be an object.
fn selected_package<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<SelectedPackage>, D::Error> {
    from_object(decoder).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SourceFile {
    pub file: String,
    pub digest: String,
    #[serde(flatten)]
    pub identity: Identity,
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
    #[cfg(feature = "quire-extraction")]
    #[serde(default, deserialize_with = "extraction")]
    pub extraction: Option<Extraction>,
    #[serde(deserialize_with = "from_object")]
    pub source: SourceFile,
    /// A `0-draft` program's authored clause bindings, required (FR-026); a
    /// `1-draft` program carries none (FR-100), so any presence at all --
    /// including an empty array -- is "carrying" `clauses` and refuses.
    #[serde(default, deserialize_with = "deserialize_objects_opt")]
    pub clauses: Option<Vec<Binding>>,
}

#[cfg(feature = "quire-extraction")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Extraction {
    #[serde(deserialize_with = "from_object")]
    pub body: Identity,
}

#[cfg(feature = "quire-extraction")]
fn extraction<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<Extraction>, D::Error> {
    from_object(decoder).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Identity {
    pub authority: String,
    pub identity: String,
    pub revision_namespace: String,
    pub revision: String,
    pub document: String,
    pub formal_revision: u64,
}

impl Identity {
    pub fn bind(&self) -> Result<crate::formal_source::SourceIdentities, ir::Diagnostic> {
        Ok(crate::formal_source::SourceIdentities {
            native: qsl_foundation::SourceIdentity::new(
                self.authority.clone(),
                self.identity.clone(),
                self.revision_namespace.clone(),
                self.revision.clone(),
            ),
            formal: ir::SourceIdentity::new(
                ir::SourceDocumentId::new(&self.document)?,
                ir::SourceRevision::new(self.formal_revision)?,
            ),
        })
    }
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

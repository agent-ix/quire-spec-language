// SPDX-License-Identifier: AGPL-3.0-only
//! FR-015/041: explicit native profiles over source-bound, validated IR declarations.

mod admission;
mod artifact;

use quire_contract_ir::{AnchorName, DeclarationEnvironment, SourceSpan, SymbolName};
use serde::Serialize;
use std::sync::Arc;

use crate::{formal_source::FormalSource, ByteDigest, Code, Diagnostic, Phase};

/// Explicit producer semantics retained in the immutable native artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum NativeModelProfile {
    /// Historical integer/text model admission.
    V1,
    /// Historical roles plus exact IR rational scalar representations.
    V2,
}

impl NativeModelProfile {
    /// Exact artifact profile spelling, independent of source format selection.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1 => "native-state-model/1",
            Self::V2 => "native-state-model/2",
        }
    }
}

/// A selector outside the explicitly supported native model profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("unsupported native model profile")]
pub struct UnknownNativeModelProfile;

impl TryFrom<&str> for NativeModelProfile {
    type Error = UnknownNativeModelProfile;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

impl std::str::FromStr for NativeModelProfile {
    type Err = UnknownNativeModelProfile;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        [Self::V1, Self::V2]
            .into_iter()
            .find(|profile| profile.as_str() == value)
            .ok_or(UnknownNativeModelProfile)
    }
}

/// Complete explicit native-role inventory; no runtime population is included.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct NativeRoles {
    /// Nominal primitive identities and their exact declaration sites.
    pub scalars: Vec<ScalarRole>,
    /// Object and opaque-reference carrier mappings.
    pub objects: Vec<ObjectRole>,
    /// Invocation, result and frame mappings.
    pub operations: Vec<OperationRole>,
}

/// One nominal scalar and the sites sharing its representation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScalarRole {
    /// Explicit scalar identity under the environment owner.
    pub name: SymbolName,
    /// Exact authored role occurrence.
    pub source: SourceSpan,
    /// Native primitive semantics beyond the IR representation.
    pub kind: ScalarKind,
    /// Nonempty, unique primitive declaration sites.
    pub sites: Vec<ScalarSite>,
}

/// A primitive leaf reached through optional/collection wrappers at one site.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScalarSite {
    /// A declared state or input value.
    Value {
        /// Value name.
        name: SymbolName,
    },
    /// A field in one declared record.
    Field {
        /// Containing record.
        record: SymbolName,
        /// Field name.
        field: SymbolName,
    },
}

/// Admitted nominal scalar semantics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScalarKind {
    /// Bounds and signed/reject policy come from the actual IR sites.
    Integer {
        /// Exact authored unit, with no conversions.
        unit: Unit,
    },
    /// Exact numerator/denominator bounds come from the actual IR sites; /2 only.
    Rational {
        /// Exact authored unit, with no conversions.
        unit: Unit,
    },
    /// Text bounded in Unicode scalar values.
    Text {
        /// Inclusive authored maximum, including zero for ordinary text.
        max_scalars: u32,
    },
}

/// Exact scalar unit identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    /// No dimension.
    Dimensionless,
    /// Explicit named dimension.
    Named(SymbolName),
}

/// Identity-bearing object and its opaque text-reference carrier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ObjectRole {
    /// Object payload record.
    pub record: SymbolName,
    /// Distinct record containing exactly one nonoptional text ID field.
    pub reference: SymbolName,
    /// The reference carrier's ID field.
    pub identity_field: SymbolName,
    /// Explicit object universe identity.
    pub universe: SymbolName,
    /// Original authored role occurrence.
    pub source: SourceSpan,
}

/// Explicit operation semantics; a pure function signature is not an operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OperationRole {
    /// Object context record.
    pub context: SymbolName,
    /// Operation name within the context.
    pub name: SymbolName,
    /// Explicit execution anchor.
    pub anchor: AnchorName,
    /// Original authored operation occurrence.
    pub source: SourceSpan,
    /// Ordered IR Input declarations captured at pre.
    pub parameters: Vec<SymbolName>,
    /// Optional IR Input declaration captured at post.
    pub result: Option<SymbolName>,
    /// Explicit effect frame, including an intentionally empty frame.
    pub frame: Frame,
}

/// Native effects permitted for the selected operation.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Frame {
    /// Allowed changed fields as (object record, field) pairs.
    pub fields: Vec<(SymbolName, SymbolName)>,
    /// Object types allowed to gain instances.
    pub created: Vec<SymbolName>,
    /// Object types allowed to lose instances.
    pub deleted: Vec<SymbolName>,
}

/// Caller-lowered model admission and artifact-content limits.
#[derive(Clone, Copy, Debug)]
pub struct ModelLimits {
    /// Total native roles, at most 10,000.
    pub roles: usize,
    /// Aggregate scalar sites, frame entries and parameters, at most 10,000.
    pub entries: usize,
    /// Formal declaration/type nodes, at most 10,000.
    pub nodes: usize,
    /// Nested formal value-type levels, at most 64.
    pub depth: usize,
    /// Emitted artifact content, at most 1 MiB.
    pub artifact_bytes: usize,
}

impl Default for ModelLimits {
    fn default() -> Self {
        Self {
            roles: 10_000,
            entries: 10_000,
            nodes: 10_000,
            depth: 64,
            artifact_bytes: 1_048_576,
        }
    }
}

impl ModelLimits {
    fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            roles: self.roles.min(hard.roles),
            entries: self.entries.min(hard.entries),
            nodes: self.nodes.min(hard.nodes),
            depth: self.depth.min(hard.depth),
            artifact_bytes: self.artifact_bytes.min(hard.artifact_bytes),
        }
    }
}

/// An immutable admitted native model; population membership is still unproved.
#[derive(Clone, Debug)]
pub struct NativeModel {
    inner: Arc<NativeModelData>,
}

#[derive(Debug)]
struct NativeModelData {
    profile: NativeModelProfile,
    source: FormalSource,
    environment: DeclarationEnvironment,
    roles: NativeRoles,
    artifact: Vec<u8>,
    digest: ByteDigest,
}

impl NativeModel {
    /// Admit the historical /1 declarations/roles and bind their exact artifact.
    ///
    /// # Errors
    /// Returns a located invalid-model, unsupported-profile or resource refusal
    /// without exposing a partially admitted model.
    pub fn new(
        source: FormalSource,
        environment: DeclarationEnvironment,
        roles: NativeRoles,
        limits: ModelLimits,
    ) -> Result<Self, Box<Diagnostic>> {
        Self::new_with_profile(NativeModelProfile::V1, source, environment, roles, limits)
    }

    /// Admit the explicitly selected profile without inferring it from declarations.
    ///
    /// # Errors
    /// Refuses invalid roles, unsupported representations and exhausted limits
    /// atomically, retaining the original model source.
    pub fn new_with_profile(
        profile: NativeModelProfile,
        source: FormalSource,
        environment: DeclarationEnvironment,
        mut roles: NativeRoles,
        limits: ModelLimits,
    ) -> Result<Self, Box<Diagnostic>> {
        let limits = limits.bounded();
        admission::check(profile, &source, &environment, &roles, limits)?;
        normalize(&mut roles);
        let artifact = artifact::encode(
            profile,
            &source,
            &environment,
            &roles,
            limits.artifact_bytes,
        )?;
        let digest = ByteDigest::of(&artifact);
        Ok(Self {
            inner: Arc::new(NativeModelData {
                profile,
                source,
                environment,
                roles,
                artifact,
                digest,
            }),
        })
    }

    /// Actual admitted profile, also encoded in the artifact bytes.
    pub fn profile(&self) -> NativeModelProfile {
        self.inner.profile
    }
    /// Exact original model source and explicit formal identity.
    pub fn source(&self) -> &FormalSource {
        &self.inner.source
    }
    /// Existing validated formal declarations, retaining original provenance.
    pub fn environment(&self) -> &DeclarationEnvironment {
        &self.inner.environment
    }
    /// Admitted roles in deterministic declaration order.
    pub fn roles(&self) -> &NativeRoles {
        &self.inner.roles
    }
    /// Exact profile-bearing native model artifact content.
    pub fn artifact_bytes(&self) -> &[u8] {
        &self.inner.artifact
    }
    /// Raw SHA-256 of the artifact, distinct from IR semantic identity.
    pub fn digest(&self) -> ByteDigest {
        self.inner.digest
    }
}

fn failure(source: &FormalSource, code: Code, message: impl Into<String>) -> Box<Diagnostic> {
    crate::diagnostic::error(source.source(), code, Phase::Link, 0, 0, message)
}

fn normalize(roles: &mut NativeRoles) {
    roles.scalars.sort_by(|a, b| a.name.cmp(&b.name));
    for role in &mut roles.scalars {
        role.sites.sort();
    }
    roles.objects.sort_by(|a, b| a.record.cmp(&b.record));
    roles
        .operations
        .sort_by(|a, b| (&a.context, &a.name).cmp(&(&b.context, &b.name)));
    for operation in &mut roles.operations {
        operation.frame.fields.sort();
        operation.frame.created.sort();
        operation.frame.deleted.sort();
    }
}

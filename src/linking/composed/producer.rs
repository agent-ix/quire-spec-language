// SPDX-License-Identifier: AGPL-3.0-only
//! Typed Producer 1.2 correspondence admission for compiler model inputs.

use std::{borrow::Cow, collections::BTreeSet};

use agent_ix_baseline_producer as filament;

use super::binding_work::{Dimension, Exhaustion, Limits, Work};
use super::models::AdmittedProducerModel;
use crate::{native_model::NativeModel, protocol_artifact::wire, ByteDigest};

/// Existing compiled-contract revision shape, reused without another namespace schema.
pub type ProducerRevision = wire::Revision;

/// Existing domain-qualified selected digest, reused without another digest schema.
pub type ProducerDigest = wire::SelectedDigest;

/// One immutable producer-side selected object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerObjectSelection {
    /// Object identity.
    pub identity: Box<str>,
    /// Namespaced immutable revision.
    pub revision: ProducerRevision,
    /// Canonical producer-object digest.
    pub digest: ProducerDigest,
}

/// One immutable producer configuration selection.
///
/// Producer interface 1.2 gives the configuration an identity and canonical
/// digest, but no revision member. Keeping a separate type prevents callers
/// from fabricating a configuration revision to fit the model/profile shape.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerConfigurationSelection {
    /// Configuration identity.
    pub identity: Box<str>,
    /// Canonical configuration-document digest.
    pub digest: ProducerDigest,
}

/// One immutable producer bundle key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerBundleSelection {
    /// Bundle identity.
    pub identity: Box<str>,
    /// Namespaced immutable revision.
    pub revision: ProducerRevision,
    /// Canonical producer-bundle digest.
    pub digest: ProducerDigest,
}

/// One native artifact or definition selected by raw bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeArtifactSelection {
    /// Native identity.
    pub identity: Box<str>,
    /// Namespaced immutable revision.
    pub revision: ProducerRevision,
    /// Raw native-byte digest in its distinct domain.
    pub digest: ProducerDigest,
}

/// Producer-authored side of one correspondence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrespondenceProducer {
    /// Producer object kind.
    pub kind: Box<str>,
    /// Producer authority.
    pub authority: Box<str>,
    /// Exact selected producer object.
    pub selection: ProducerObjectSelection,
}

/// Closed static producer export kinds consumed by this adapter.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ProducerExportKind {
    /// Component declaration.
    Component,
    /// Relationship endpoint declaration.
    Endpoint,
    /// Relationship declaration.
    Relationship,
    /// Enumeration declaration.
    Enum,
    /// Record field declaration.
    Field,
    /// Native object declaration.
    Object,
    /// Native operation declaration.
    Operation,
    /// Native record declaration.
    Record,
    /// Native reference declaration.
    Reference,
    /// Native scalar declaration.
    Scalar,
    /// Enumeration variant declaration.
    Variant,
}

impl ProducerExportKind {
    /// Whether this export names a native type rather than a producer record or member.
    pub const fn is_type(self) -> bool {
        matches!(
            self,
            Self::Enum
                | Self::Object
                | Self::Record
                | Self::Reference
                | Self::Scalar
                | Self::Variant
        )
    }

    /// The existing compiled-protocol export kind for this producer mapping.
    pub(crate) const fn wire_kind(self) -> wire::ExportKind {
        match self {
            Self::Component => wire::ExportKind::Component,
            Self::Endpoint => wire::ExportKind::Endpoint,
            Self::Relationship => wire::ExportKind::Relationship,
            Self::Enum => wire::ExportKind::Enum,
            Self::Field => wire::ExportKind::Field,
            Self::Object => wire::ExportKind::Object,
            Self::Operation => wire::ExportKind::Operation,
            Self::Record => wire::ExportKind::Record,
            Self::Reference => wire::ExportKind::Reference,
            Self::Scalar => wire::ExportKind::Scalar,
            Self::Variant => wire::ExportKind::Variant,
        }
    }
}

/// The declaration members supplying the first and second `related by` operands.
/// Every admitted direction retains source first and target second; direction
/// changes traversal authority only.
pub(crate) fn relationship_operands(
    declaration: &filament::RelationshipDeclaration,
) -> (
    &filament::RelationshipEndpoint,
    &filament::RelationshipEndpoint,
) {
    match declaration.semantics.direction {
        filament::RelationshipDirection::SourceToTarget
        | filament::RelationshipDirection::TargetToSource
        | filament::RelationshipDirection::Bidirectional
        | filament::RelationshipDirection::Undirected => (&declaration.source, &declaration.target),
    }
}

/// One exported producer identity and its exact native path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerExportSelection {
    /// Producer export kind.
    pub kind: ProducerExportKind,
    /// Stable producer export identity.
    pub identity: Box<str>,
    /// Producer object owning the export.
    pub producer_object_identity: Box<str>,
    /// Ordered native export path.
    pub path: Vec<Box<str>>,
    /// Exact producer source locus.
    pub locus: wire::ForeignLocus,
}

/// Complete producer/native correspondence selected for one model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerCorrespondenceSelection {
    /// Producer-declared binding relation identity.
    pub relation_identity: Box<str>,
    /// Producer object named by the relation.
    pub producer: CorrespondenceProducer,
    /// Native model artifact named by the relation.
    pub native: NativeArtifactSelection,
    /// Exact native definition closure.
    pub definitions: Vec<NativeArtifactSelection>,
    /// Required definition identities; absence never means discovery.
    pub required_definitions: BTreeSet<Box<str>>,
    /// Configuration that authorized the relation.
    pub configuration_identity: Box<str>,
    /// Complete producer export mapping.
    pub exports: Vec<ProducerExportSelection>,
}

/// The complete static producer selection supplied by a caller adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerCompatibilitySelection {
    /// Producer interface version.
    pub interface_version: Box<str>,
    /// Exact producer bundle key.
    pub bundle: ProducerBundleSelection,
    /// Exact producer model selection.
    pub model: ProducerObjectSelection,
    /// Exact producer profile selection.
    pub profile: ProducerObjectSelection,
    /// Exact producer configuration selection.
    pub configuration: ProducerConfigurationSelection,
    /// Exact producer/native correspondence.
    pub correspondence: ProducerCorrespondenceSelection,
}

/// Caller-supplied offered producer selection paired with actual native bytes.
#[derive(Clone, Copy, Debug)]
pub struct ProducerCompatibilityInput<'a> {
    /// Offered static producer selection.
    pub selection: &'a ProducerCompatibilitySelection,
    /// Constructor-admitted native model whose bytes the offer claims.
    pub native: &'a NativeModel,
}

/// A distinct refusal for every independently selected producer axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProducerModelRefusal {
    /// Producer selection intake exceeded an independently bounded compiler resource.
    #[error("producer selection resource exhausted: {0:?}")]
    ResourceExhausted(Exhaustion),
    /// Interface version differs or is unsupported.
    #[error("producer interface selection differs")]
    Interface,
    /// Bundle identity, revision or digest differs.
    #[error("producer bundle selection differs")]
    Bundle,
    /// Model identity, revision or digest differs.
    #[error("producer model selection differs")]
    Model,
    /// Profile identity, revision or digest differs.
    #[error("producer profile selection differs")]
    Profile,
    /// Configuration identity, revision or digest differs.
    #[error("producer configuration selection differs")]
    Configuration,
    /// The relation's producer selection differs from the selected model.
    #[error("producer correspondence selection differs")]
    Correspondence,
    /// The native definition closure is missing, duplicated or surplus.
    #[error("native definition closure differs")]
    DefinitionClosure,
    /// The export mapping is incomplete, foreign, malformed or duplicated.
    #[error("producer export mapping differs")]
    Exports,
    /// A producer canonical digest uses the wrong domain or spelling.
    #[error("producer canonical digest is invalid")]
    ProducerDigest,
    /// A native digest uses the wrong domain or spelling.
    #[error("native raw-byte digest is invalid")]
    NativeDigest,
    /// The correspondence's native raw-byte digest does not seal the model bytes.
    #[error("native model bytes differ")]
    NativeBytes,
}

/// Convert one admitted Filament producer bundle into A's existing typed
/// compatibility selection for the exact native artifact identity requested by
/// the caller.
///
/// The producer bundle has already passed Filament's indivisible admission.
/// This adapter only selects one model/native pair and translates its members;
/// it does not deserialize producer JSON or reconstruct omitted meaning.
pub fn adapt_filament_producer(
    bundle: &filament::AdmittedStaticBundle,
    native_artifact_identity: &str,
) -> Result<ProducerCompatibilitySelection, ProducerModelRefusal> {
    let mut matching = bundle.correspondences().iter().filter(|correspondence| {
        correspondence.producer.identity == bundle.model().model_identity
            && correspondence.native.identity == native_artifact_identity
    });
    let correspondence = matching
        .next()
        .ok_or(ProducerModelRefusal::Correspondence)?;
    if matching.next().is_some() {
        return Err(ProducerModelRefusal::Correspondence);
    }

    Ok(ProducerCompatibilitySelection {
        interface_version: bundle.interface_version().into(),
        bundle: ProducerBundleSelection {
            identity: bundle.bundle_identity().into(),
            revision: revision(bundle.bundle_revision()),
            digest: digest(bundle.digest()),
        },
        model: ProducerObjectSelection {
            identity: bundle.model().model_identity.as_str().into(),
            revision: revision(&bundle.model().model_revision),
            digest: digest(&bundle.model().digest),
        },
        profile: ProducerObjectSelection {
            identity: bundle.profile().profile_identity.as_str().into(),
            revision: revision(&bundle.profile().profile_revision),
            digest: digest(&bundle.profile().digest),
        },
        configuration: ProducerConfigurationSelection {
            identity: bundle
                .configuration()
                .configuration_identity
                .as_str()
                .into(),
            digest: digest(&bundle.configuration().digest),
        },
        correspondence: ProducerCorrespondenceSelection {
            relation_identity: correspondence.binding_relation_identity.as_str().into(),
            producer: CorrespondenceProducer {
                kind: correspondence.producer.object_kind.as_str().into(),
                authority: correspondence.producer.authority.as_str().into(),
                selection: ProducerObjectSelection {
                    identity: correspondence.producer.identity.as_str().into(),
                    revision: revision(&correspondence.producer.revision),
                    digest: digest(&correspondence.producer.digest),
                },
            },
            native: native_artifact(&correspondence.native),
            definitions: correspondence
                .native_definition_closure
                .iter()
                .map(native_artifact)
                .collect(),
            required_definitions: correspondence
                .required_native_definition_identities
                .iter()
                .map(|identity| identity.as_str().into())
                .collect(),
            configuration_identity: correspondence
                .configuration_identity
                .as_deref()
                .ok_or(ProducerModelRefusal::Correspondence)?
                .into(),
            exports: correspondence
                .exports
                .iter()
                .map(export)
                .collect::<Result<_, _>>()?,
        },
    })
}

/// Admit one indivisibly admitted Filament bundle against an independently
/// supplied expected selection and the actual constructor-admitted native
/// model bytes.
pub fn admit_filament_producer_model<'a>(
    bundle: &'a filament::AdmittedStaticBundle,
    native_artifact_identity: &str,
    native: &'a NativeModel,
    expected: &ProducerCompatibilitySelection,
) -> Result<AdmittedProducerModel<'a>, ProducerModelRefusal> {
    admit_filament_producer_model_with_limits(
        bundle,
        native_artifact_identity,
        native,
        expected,
        Limits::default(),
    )
}

/// Admit one Filament bundle under caller-lowered, hard-clamped compiler intake
/// limits.
pub fn admit_filament_producer_model_with_limits<'a>(
    bundle: &'a filament::AdmittedStaticBundle,
    native_artifact_identity: &str,
    native: &'a NativeModel,
    expected: &ProducerCompatibilitySelection,
    limits: Limits,
) -> Result<AdmittedProducerModel<'a>, ProducerModelRefusal> {
    let actual = adapt_filament_producer(bundle, native_artifact_identity)?;
    admit_producer_model_selection(Cow::Owned(actual), native, expected, limits, Some(bundle))
}

/// Admit one caller-adapted Producer 1.2 selection as authority for a native model.
pub fn admit_producer_model<'a>(
    offered: ProducerCompatibilityInput<'a>,
    expected: &ProducerCompatibilitySelection,
) -> Result<AdmittedProducerModel<'a>, ProducerModelRefusal> {
    admit_producer_model_with_limits(offered, expected, Limits::default())
}

/// Admit one producer selection under caller-lowered, hard-clamped intake limits.
pub fn admit_producer_model_with_limits<'a>(
    offered: ProducerCompatibilityInput<'a>,
    expected: &ProducerCompatibilitySelection,
    limits: Limits,
) -> Result<AdmittedProducerModel<'a>, ProducerModelRefusal> {
    admit_producer_model_selection(
        Cow::Borrowed(offered.selection),
        offered.native,
        expected,
        limits,
        None,
    )
}

fn admit_producer_model_selection<'a>(
    actual: Cow<'a, ProducerCompatibilitySelection>,
    native: &'a NativeModel,
    expected: &ProducerCompatibilitySelection,
    limits: Limits,
    filament: Option<&'a filament::AdmittedStaticBundle>,
) -> Result<AdmittedProducerModel<'a>, ProducerModelRefusal> {
    let mut work = Work::new(limits);
    charge_selection(actual.as_ref(), &mut work)?;
    charge_selection(expected, &mut work)?;
    validate_selection(actual.as_ref())?;
    if let Some(refusal) = selection_difference(actual.as_ref(), expected) {
        return Err(refusal);
    }
    if actual.correspondence.producer.kind.as_ref() != "model"
        || actual.correspondence.producer.authority.is_empty()
        || actual.correspondence.producer.selection != actual.model
        || actual.correspondence.configuration_identity != actual.configuration.identity
    {
        return Err(ProducerModelRefusal::Correspondence);
    }
    if actual
        .correspondence
        .exports
        .iter()
        .any(|export| export.producer_object_identity != actual.model.identity)
    {
        return Err(ProducerModelRefusal::Exports);
    }
    if actual.correspondence.native.digest.value != native.digest().to_string() {
        return Err(ProducerModelRefusal::NativeBytes);
    }

    Ok(AdmittedProducerModel::new(native, actual, filament))
}

fn revision(value: &filament::Revision) -> ProducerRevision {
    ProducerRevision {
        namespace: value.namespace.clone(),
        value: value.value.clone(),
    }
}

fn digest(value: &filament::DigestSelection) -> ProducerDigest {
    ProducerDigest {
        domain: value.domain.clone(),
        version: value.version.clone(),
        algorithm: value.algorithm.clone(),
        value: value.value.clone(),
    }
}

fn native_artifact(value: &filament::NativeArtifactReference) -> NativeArtifactSelection {
    NativeArtifactSelection {
        identity: value.identity.as_str().into(),
        revision: revision(&value.revision),
        digest: digest(&value.raw_byte_digest),
    }
}

fn export(value: &filament::ExportRecord) -> Result<ProducerExportSelection, ProducerModelRefusal> {
    let locus = value.locus.as_ref().ok_or(ProducerModelRefusal::Exports)?;
    Ok(ProducerExportSelection {
        kind: export_kind(value.kind),
        identity: value.export_identity.as_str().into(),
        producer_object_identity: value.producer_object_identity.as_str().into(),
        path: value
            .export_path
            .iter()
            .map(|part| part.as_str().into())
            .collect(),
        locus: foreign_locus(locus)?,
    })
}

fn export_kind(value: filament::ExportKind) -> ProducerExportKind {
    match value {
        filament::ExportKind::Component => ProducerExportKind::Component,
        filament::ExportKind::Endpoint => ProducerExportKind::Endpoint,
        filament::ExportKind::Relationship => ProducerExportKind::Relationship,
        filament::ExportKind::Enum => ProducerExportKind::Enum,
        filament::ExportKind::Field => ProducerExportKind::Field,
        filament::ExportKind::Object => ProducerExportKind::Object,
        filament::ExportKind::Operation => ProducerExportKind::Operation,
        filament::ExportKind::Record => ProducerExportKind::Record,
        filament::ExportKind::Reference => ProducerExportKind::Reference,
        filament::ExportKind::Scalar => ProducerExportKind::Scalar,
        filament::ExportKind::Variant => ProducerExportKind::Variant,
    }
}

fn foreign_locus(
    value: &filament::SourceLocus,
) -> Result<wire::ForeignLocus, ProducerModelRefusal> {
    Ok(wire::ForeignLocus {
        source: wire::ArtifactRef {
            ref_version: value.source.ref_version.clone(),
            kind: artifact_kind(value.source.kind),
            authority: value.source.authority.clone(),
            identity: value.source.identity.clone(),
            revision: revision(&value.source.revision),
            digest: value
                .source
                .digest
                .as_str()
                .parse()
                .map_err(|_| ProducerModelRefusal::Exports)?,
            wire: wire::Wire {
                identity: value.source.wire.identity.clone(),
                version: value.source.wire.version.clone(),
            },
        },
        formal: wire::Formal {
            document: value.formal.document.clone(),
            revision: revision(&value.formal.revision),
        },
        span: wire::Span {
            start: value.span.start,
            end: value.span.end,
        },
    })
}

fn artifact_kind(value: filament::ArtifactKind) -> wire::ArtifactKind {
    match value {
        filament::ArtifactKind::Binding => wire::ArtifactKind::Binding,
        filament::ArtifactKind::DependencyClosure => wire::ArtifactKind::DependencyClosure,
        filament::ArtifactKind::Environment => wire::ArtifactKind::Environment,
        filament::ArtifactKind::ExecutableProjection => wire::ArtifactKind::ExecutableProjection,
        filament::ArtifactKind::FaultModel => wire::ArtifactKind::FaultModel,
        filament::ArtifactKind::GeneratedArtifact => wire::ArtifactKind::GeneratedArtifact,
        filament::ArtifactKind::Invocation => wire::ArtifactKind::Invocation,
        filament::ArtifactKind::LinkedPackage => wire::ArtifactKind::LinkedPackage,
        filament::ArtifactKind::ModelLock => wire::ArtifactKind::ModelLock,
        filament::ArtifactKind::ModelManifest => wire::ArtifactKind::ModelManifest,
        filament::ArtifactKind::ModelPackage => wire::ArtifactKind::ModelPackage,
        filament::ArtifactKind::Observation => wire::ArtifactKind::Observation,
        filament::ArtifactKind::Oracle => wire::ArtifactKind::Oracle,
        filament::ArtifactKind::Property => wire::ArtifactKind::Property,
        filament::ArtifactKind::ReviewDisposition => wire::ArtifactKind::ReviewDisposition,
        filament::ArtifactKind::ReviewProcedure => wire::ArtifactKind::ReviewProcedure,
        filament::ArtifactKind::RunArtifact => wire::ArtifactKind::RunArtifact,
        filament::ArtifactKind::Snapshot => wire::ArtifactKind::Snapshot,
        filament::ArtifactKind::Source => wire::ArtifactKind::Source,
        filament::ArtifactKind::Trace => wire::ArtifactKind::Trace,
    }
}

fn charge_selection(
    selection: &ProducerCompatibilitySelection,
    work: &mut Work,
) -> Result<(), ProducerModelRefusal> {
    fn text(work: &mut Work, value: &str) -> Result<(), ProducerModelRefusal> {
        work.charge(Dimension::Bytes, value.len())
            .map_err(ProducerModelRefusal::ResourceExhausted)
    }
    text(work, &selection.interface_version)?;
    for selected in [
        (
            &selection.bundle.identity,
            &selection.bundle.revision,
            &selection.bundle.digest,
        ),
        (
            &selection.model.identity,
            &selection.model.revision,
            &selection.model.digest,
        ),
        (
            &selection.profile.identity,
            &selection.profile.revision,
            &selection.profile.digest,
        ),
    ] {
        work.charge(Dimension::Bindings, 1)
            .map_err(ProducerModelRefusal::ResourceExhausted)?;
        for value in [
            selected.0.as_ref(),
            selected.1.namespace.as_str(),
            selected.1.value.as_str(),
            selected.2.domain.as_str(),
            selected.2.version.as_str(),
            selected.2.algorithm.as_str(),
            selected.2.value.as_str(),
        ] {
            text(work, value)?;
        }
    }
    work.charge(Dimension::Bindings, 1)
        .map_err(ProducerModelRefusal::ResourceExhausted)?;
    for value in [
        selection.configuration.identity.as_ref(),
        selection.configuration.digest.domain.as_str(),
        selection.configuration.digest.version.as_str(),
        selection.configuration.digest.algorithm.as_str(),
        selection.configuration.digest.value.as_str(),
    ] {
        text(work, value)?;
    }
    let correspondence = &selection.correspondence;
    work.charge(Dimension::Bindings, 3)
        .map_err(ProducerModelRefusal::ResourceExhausted)?;
    for value in [
        correspondence.relation_identity.as_ref(),
        correspondence.producer.kind.as_ref(),
        correspondence.producer.authority.as_ref(),
        correspondence.producer.selection.identity.as_ref(),
        correspondence
            .producer
            .selection
            .revision
            .namespace
            .as_str(),
        correspondence.producer.selection.revision.value.as_str(),
        correspondence.producer.selection.digest.domain.as_str(),
        correspondence.producer.selection.digest.version.as_str(),
        correspondence.producer.selection.digest.algorithm.as_str(),
        correspondence.producer.selection.digest.value.as_str(),
        correspondence.native.identity.as_ref(),
        correspondence.native.revision.namespace.as_str(),
        correspondence.native.revision.value.as_str(),
        correspondence.native.digest.domain.as_str(),
        correspondence.native.digest.version.as_str(),
        correspondence.native.digest.algorithm.as_str(),
        correspondence.native.digest.value.as_str(),
        correspondence.configuration_identity.as_ref(),
    ] {
        text(work, value)?;
    }
    for definition in &correspondence.definitions {
        work.charge(Dimension::Definitions, 1)
            .map_err(ProducerModelRefusal::ResourceExhausted)?;
        work.charge(Dimension::References, 1)
            .map_err(ProducerModelRefusal::ResourceExhausted)?;
        for value in [
            definition.identity.as_ref(),
            definition.revision.namespace.as_str(),
            definition.revision.value.as_str(),
            definition.digest.domain.as_str(),
            definition.digest.version.as_str(),
            definition.digest.algorithm.as_str(),
            definition.digest.value.as_str(),
        ] {
            text(work, value)?;
        }
    }
    for identity in &correspondence.required_definitions {
        work.charge(Dimension::Definitions, 1)
            .map_err(ProducerModelRefusal::ResourceExhausted)?;
        text(work, identity)?;
    }
    for export in &correspondence.exports {
        work.charge(Dimension::References, 1)
            .map_err(ProducerModelRefusal::ResourceExhausted)?;
        work.charge(Dimension::Bindings, 1)
            .map_err(ProducerModelRefusal::ResourceExhausted)?;
        work.charge(Dimension::Bindings, export.path.len())
            .map_err(ProducerModelRefusal::ResourceExhausted)?;
        text(work, &export.identity)?;
        text(work, &export.producer_object_identity)?;
        for part in &export.path {
            text(work, part)?;
        }
        let locus = &export.locus;
        for value in [
            locus.source.ref_version.as_str(),
            locus.source.authority.as_str(),
            locus.source.identity.as_str(),
            locus.source.revision.namespace.as_str(),
            locus.source.revision.value.as_str(),
            locus.source.wire.identity.as_str(),
            locus.source.wire.version.as_str(),
            locus.formal.document.as_str(),
            locus.formal.revision.namespace.as_str(),
            locus.formal.revision.value.as_str(),
        ] {
            text(work, value)?;
        }
    }
    Ok(())
}

pub(crate) fn selection_difference(
    actual: &ProducerCompatibilitySelection,
    expected: &ProducerCompatibilitySelection,
) -> Option<ProducerModelRefusal> {
    if actual.interface_version != expected.interface_version {
        Some(ProducerModelRefusal::Interface)
    } else if actual.bundle != expected.bundle {
        Some(ProducerModelRefusal::Bundle)
    } else if actual.model != expected.model {
        Some(ProducerModelRefusal::Model)
    } else if actual.profile != expected.profile {
        Some(ProducerModelRefusal::Profile)
    } else if actual.configuration != expected.configuration {
        Some(ProducerModelRefusal::Configuration)
    } else if actual.correspondence.relation_identity != expected.correspondence.relation_identity
        || actual.correspondence.producer != expected.correspondence.producer
        || actual.correspondence.native != expected.correspondence.native
        || actual.correspondence.configuration_identity
            != expected.correspondence.configuration_identity
    {
        Some(ProducerModelRefusal::Correspondence)
    } else if actual.correspondence.definitions != expected.correspondence.definitions
        || actual.correspondence.required_definitions
            != expected.correspondence.required_definitions
    {
        Some(ProducerModelRefusal::DefinitionClosure)
    } else if actual.correspondence.exports != expected.correspondence.exports {
        Some(ProducerModelRefusal::Exports)
    } else {
        None
    }
}

pub(crate) fn validate_selection(
    actual: &ProducerCompatibilitySelection,
) -> Result<(), ProducerModelRefusal> {
    if actual.interface_version.as_ref() != "1.2.0" {
        return Err(ProducerModelRefusal::Interface);
    }
    if actual.bundle.identity.is_empty() || invalid_revision(&actual.bundle.revision) {
        return Err(ProducerModelRefusal::Bundle);
    }
    validate_producer_digest(&actual.bundle.digest)?;
    if actual.model.identity.is_empty() || invalid_revision(&actual.model.revision) {
        return Err(ProducerModelRefusal::Model);
    }
    validate_producer_digest(&actual.model.digest)?;
    if actual.profile.identity.is_empty() || invalid_revision(&actual.profile.revision) {
        return Err(ProducerModelRefusal::Profile);
    }
    validate_producer_digest(&actual.profile.digest)?;
    if actual.configuration.identity.is_empty() {
        return Err(ProducerModelRefusal::Configuration);
    }
    validate_producer_digest(&actual.configuration.digest)?;
    if actual.correspondence.relation_identity.is_empty()
        || actual.correspondence.producer.kind.as_ref() != "model"
        || actual.correspondence.producer.authority.is_empty()
        || actual.correspondence.native.identity.is_empty()
        || invalid_revision(&actual.correspondence.native.revision)
    {
        return Err(ProducerModelRefusal::Correspondence);
    }
    validate_native_digest(&actual.correspondence.native.digest)?;

    let mut definitions = BTreeSet::new();
    for definition in &actual.correspondence.definitions {
        if definition.identity.is_empty() || invalid_revision(&definition.revision) {
            return Err(ProducerModelRefusal::DefinitionClosure);
        }
        validate_native_digest(&definition.digest)?;
        if !definitions.insert(definition.identity.as_ref()) {
            return Err(ProducerModelRefusal::DefinitionClosure);
        }
    }
    let required = actual
        .correspondence
        .required_definitions
        .iter()
        .map(AsRef::as_ref)
        .collect::<BTreeSet<_>>();
    if definitions != required {
        return Err(ProducerModelRefusal::DefinitionClosure);
    }
    if actual.correspondence.exports.is_empty() {
        return Err(ProducerModelRefusal::Exports);
    }
    let mut identities = BTreeSet::new();
    let mut mappings = BTreeSet::new();
    for export in &actual.correspondence.exports {
        if export.identity.is_empty()
            || export.producer_object_identity.is_empty()
            || export.path.is_empty()
            || export.path.iter().any(|part| part.is_empty())
            || export.locus.source.ref_version.is_empty()
            || export.locus.source.authority.is_empty()
            || export.locus.source.identity.is_empty()
            || export.locus.source.revision.namespace.is_empty()
            || export.locus.source.revision.value.is_empty()
            || export.locus.source.wire.identity.is_empty()
            || export.locus.source.wire.version.is_empty()
            || export.locus.formal.document.is_empty()
            || export.locus.formal.revision.namespace.is_empty()
            || export.locus.formal.revision.value.is_empty()
            || export.locus.span.start > export.locus.span.end
            || !identities.insert(export.identity.as_ref())
            || !mappings.insert((export.kind, export.path.as_slice()))
        {
            return Err(ProducerModelRefusal::Exports);
        }
    }
    Ok(())
}

fn invalid_revision(revision: &ProducerRevision) -> bool {
    revision.namespace.is_empty() || revision.value.is_empty()
}

fn validate_producer_digest(digest: &ProducerDigest) -> Result<(), ProducerModelRefusal> {
    if digest.algorithm != "sha256"
        || digest.domain != "filament-canonical-json-1"
        || digest.version != "1"
        || digest.value.parse::<ByteDigest>().is_err()
    {
        return Err(ProducerModelRefusal::ProducerDigest);
    }
    Ok(())
}

fn validate_native_digest(digest: &ProducerDigest) -> Result<(), ProducerModelRefusal> {
    if digest.algorithm != "sha256"
        || digest.domain != "quire-native-bytes-1"
        || digest.version != "1"
        || digest.value.parse::<ByteDigest>().is_err()
    {
        return Err(ProducerModelRefusal::NativeDigest);
    }
    Ok(())
}

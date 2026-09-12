// SPDX-License-Identifier: AGPL-3.0-only
//! Typed Producer 1.2 correspondence admission for compiler model inputs.

use std::collections::BTreeSet;

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
    pub configuration: ProducerObjectSelection,
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
    let actual = offered.selection;
    let mut work = Work::new(limits);
    charge_selection(actual, &mut work)?;
    charge_selection(expected, &mut work)?;
    validate_selection(actual)?;
    if let Some(refusal) = selection_difference(actual, expected) {
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
    if actual.correspondence.native.digest.value != offered.native.digest().to_string() {
        return Err(ProducerModelRefusal::NativeBytes);
    }

    Ok(AdmittedProducerModel::new(
        offered.native,
        actual,
        actual.bundle.identity.clone(),
        actual.model.identity.clone(),
        actual.profile.identity.clone(),
        actual.configuration.identity.clone(),
        actual.correspondence.relation_identity.clone(),
    ))
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
        (
            &selection.configuration.identity,
            &selection.configuration.revision,
            &selection.configuration.digest,
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
    if actual.configuration.identity.is_empty() || invalid_revision(&actual.configuration.revision)
    {
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

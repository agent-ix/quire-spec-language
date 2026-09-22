// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-020: ordered selection, explicit authority, real reconstruction and comparison.

use super::{
    encoding, intake, view, wire, NativePackage, NativePackageRef, PackageCause, PackageError,
    PackageLimits, PackagePathSegment, PackageStage, PackageUsage,
};
use crate::checking::{check, CheckBindings, CheckLimits};
use crate::native_model::NativeModel;
use crate::{link_native, parse, Limits, LinkLimits};
use qsl_foundation::{ByteDigest, Code, Diagnostic};
use std::collections::{BTreeMap, BTreeSet};

const KNOWN_FEATURES: [&str; 19] = [
    "boolean",
    "integer",
    "text",
    "enumeration",
    "structural-record",
    "object",
    "reference",
    "option",
    "sequence",
    "integer-arithmetic",
    "comparison",
    "boolean-control",
    "conditional",
    "let",
    "quantification",
    "reachability",
    "pre-observation",
    "precondition",
    "postcondition",
];

/// Independently selected package and native frontend ceilings.
#[derive(Clone, Copy, Debug, Default)]
pub struct PackageReadLimits {
    /// Limits for every package pass; counters are independent.
    pub package: PackageLimits,
    /// Actual native parser limits.
    pub syntax: Limits,
    /// Actual native inventory and linker limits.
    pub link: LinkLimits,
    /// Actual native type/proof checker limits.
    pub check: CheckLimits,
}

/// Consumer feature admission; extra strings cannot enable unknown features.
#[derive(Clone, Debug)]
pub struct PackageSupport {
    /// Explicit set of supported required features.
    pub features: BTreeSet<String>,
}
impl Default for PackageSupport {
    fn default() -> Self {
        Self {
            features: KNOWN_FEATURES.into_iter().map(String::from).collect(),
        }
    }
}

struct Request {
    usage: PackageUsage,
    stage: PackageStage,
}
impl Request {
    fn error(
        &self,
        code: Code,
        path: Vec<PackagePathSegment>,
        message: &'static str,
    ) -> Box<PackageError> {
        Box::new(PackageError {
            code,
            stage: self.stage,
            path,
            usage: self.usage.clone(),
            message,
            cause: None,
        })
    }
    fn native(&self, cause: Box<Diagnostic>) -> Box<PackageError> {
        let mut error = self.error(
            cause.code,
            Vec::new(),
            "native compiler reconstruction refused",
        );
        error.cause = Some(PackageCause::Native(cause));
        error
    }
    fn linking(&self, cause: Box<crate::linking::LinkingError>) -> Box<PackageError> {
        let mut error = self.error(
            cause.diagnostic.code,
            Vec::new(),
            "native compiler reconstruction refused",
        );
        error.cause = Some(PackageCause::Linking(cause));
        error
    }
    fn checking(&self, cause: Box<crate::checking::CheckingError>) -> Box<PackageError> {
        let mut error = self.error(
            cause.diagnostic.code,
            Vec::new(),
            "native compiler reconstruction refused",
        );
        error.cause = Some(PackageCause::Checking(cause));
        error
    }
    fn decoded<T>(
        &mut self,
        result: Result<(T, super::PackagePassUsage), intake::Failure>,
        recognition: bool,
    ) -> Result<T, Box<PackageError>> {
        let measured = match &result {
            Ok((_, usage)) => *usage,
            Err(error) => error.usage,
        };
        if recognition {
            self.usage.recognition = Some(measured);
        } else {
            self.usage.decode = Some(measured);
        }
        result.map(|(value, _)| value).map_err(|failure| {
            let mut error = self.error(
                failure.code,
                failure.path,
                "native package JSON intake refused",
            );
            error.cause = Some(PackageCause::Json(failure.cause));
            error
        })
    }
}
fn path(fields: &[&str]) -> Vec<PackagePathSegment> {
    fields
        .iter()
        .map(|field| PackagePathSegment::Field((*field).into()))
        .collect()
}
fn clause_path(index: usize, field: &str) -> Vec<PackagePathSegment> {
    vec![
        PackagePathSegment::Field("clauses".into()),
        PackagePathSegment::Index(index),
        PackagePathSegment::Field(field.into()),
    ]
}

fn select(
    request: &Request,
    wire: &mut wire::Manifest,
    support: &PackageSupport,
) -> Result<(), Box<PackageError>> {
    let expected = view::Semantics::selected();
    let actual = &wire.semantics;
    for (field, actual, expected, code) in [
        (
            "language",
            actual.language.as_str(),
            expected.language,
            Code::UnknownLanguage,
        ),
        (
            "edition",
            actual.edition.as_str(),
            expected.edition,
            Code::UnknownEdition,
        ),
        (
            "syntax_profile",
            actual.syntax_profile.as_str(),
            expected.syntax_profile,
            Code::UnknownProfile,
        ),
        (
            "model_profile",
            actual.model_profile.as_str(),
            expected.model_profile,
            Code::UnknownProfile,
        ),
        (
            "checking_contract",
            actual.checking_contract.as_str(),
            expected.checking_contract,
            Code::UnknownProfile,
        ),
        (
            "ir_revision",
            actual.ir_revision.as_str(),
            expected.ir_revision,
            Code::UnknownProfile,
        ),
    ] {
        if actual != expected {
            return Err(request.error(
                code,
                path(&["semantics", field]),
                "unsupported package semantic selection",
            ));
        }
    }
    for (field, actual, expected) in [
        (
            "base_definition",
            &actual.base_definition,
            expected.base_definition,
        ),
        (
            "rules_definition",
            &actual.rules_definition,
            expected.rules_definition,
        ),
    ] {
        if actual.revision != expected.revision {
            return Err(request.error(
                Code::UnknownProfile,
                path(&["semantics", field, "revision"]),
                "unsupported semantic definition revision",
            ));
        }
        if actual.digest.0.to_string() != expected.digest {
            return Err(request.error(
                Code::UnknownProfile,
                path(&["semantics", field, "digest"]),
                "unsupported semantic definition bytes",
            ));
        }
    }
    for (field, actual, expected) in [
        (
            "domain",
            wire.canonical_identity.domain.as_str(),
            "quire.native.bound-package",
        ),
        ("version", wire.canonical_identity.version.as_str(), "v1"),
        (
            "algorithm",
            wire.canonical_identity.algorithm.as_str(),
            "sha256",
        ),
    ] {
        if actual != expected {
            return Err(request.error(
                Code::UnknownProfile,
                path(&["canonical_identity", field]),
                "unsupported canonical identity selection",
            ));
        }
    }
    let mut seen = BTreeSet::new();
    for (index, feature) in wire.required_features.iter().enumerate() {
        let location = vec![
            PackagePathSegment::Field("required_features".into()),
            PackagePathSegment::Index(index),
        ];
        if !seen.insert(feature) {
            return Err(request.error(
                Code::InvalidPackage,
                location,
                "duplicate required feature",
            ));
        }
        if !KNOWN_FEATURES.contains(&feature.as_str()) || !support.features.contains(feature) {
            return Err(request.error(
                Code::UnknownRequiredFeature,
                location,
                "consumer does not support required feature",
            ));
        }
    }
    wire.required_features.sort();
    Ok(())
}

fn authority(
    request: &Request,
    wire: &wire::Manifest,
    bindings: &CheckBindings,
    limits: LinkLimits,
) -> Result<(), Box<PackageError>> {
    let source = bindings.source.source();
    for (field, matches, code) in [
        (
            "identity",
            wire.source.identity.0 == source.identity().identity,
            Code::InvalidModelBinding,
        ),
        (
            "revision",
            wire.source.revision.0 == source.identity().revision,
            Code::InvalidModelBinding,
        ),
        (
            "digest",
            wire.source.digest.0 == source.digest(),
            Code::StaleDependency,
        ),
        (
            "formal",
            wire.source.formal.native() == *bindings.source.identity(),
            Code::InvalidModelBinding,
        ),
    ] {
        if !matches {
            return Err(request.error(
                code,
                path(&["source", field]),
                "package source differs from external authority",
            ));
        }
    }
    if bindings.clauses.len() > limits.clauses.min(LinkLimits::default().clauses) {
        return Err(request.error(
            Code::ResourceExhausted,
            path(&["clauses"]),
            "external authored binding count exceeds limit",
        ));
    }
    if bindings.clauses.len() != wire.clauses.len() {
        return Err(request.error(
            Code::InvalidModelBinding,
            path(&["clauses"]),
            "external authored binding count differs",
        ));
    }
    let mut by_name = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for binding in &bindings.clauses {
        if by_name.insert(binding.name.as_str(), binding).is_some()
            || !identities.insert((&binding.requirement, &binding.clause))
        {
            return Err(request.error(
                Code::InvalidModelBinding,
                path(&["clauses"]),
                "external authored bindings are not unique",
            ));
        }
    }
    let mut names = BTreeSet::new();
    let mut selected = BTreeSet::new();
    for (index, clause) in wire.clauses.iter().enumerate() {
        let owner = clause.owner.native();
        if !names.insert(clause.name.0.as_str())
            || !selected.insert((owner.clone(), &clause.clause))
        {
            return Err(request.error(
                Code::InvalidModelBinding,
                clause_path(index, "name"),
                "wire authored bindings are not unique",
            ));
        }
        let Some(binding) = by_name.get(clause.name.0.as_str()) else {
            return Err(request.error(
                Code::InvalidModelBinding,
                clause_path(index, "name"),
                "wire clause has no external authored binding",
            ));
        };
        for (field, matches) in [
            ("owner", owner == binding.requirement),
            ("clause", clause.clause == binding.clause),
            (
                "execution_point",
                clause.execution_point.native() == binding.execution_point,
            ),
        ] {
            if !matches {
                return Err(request.error(
                    Code::InvalidModelBinding,
                    clause_path(index, field),
                    "wire authored binding differs from external authority",
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn read<'model>(
    bytes: &[u8],
    expected: NativePackageRef,
    bindings: CheckBindings,
    models: &'model [NativeModel],
    support: &PackageSupport,
    limits: PackageReadLimits,
) -> Result<NativePackage<'model>, Box<PackageError>> {
    let package_limits = limits.package.bounded();
    let mut request = Request {
        usage: PackageUsage::default(),
        stage: PackageStage::Decode,
    };
    if bytes.len() > package_limits.artifact_bytes {
        return Err(request.error(
            Code::ResourceExhausted,
            Vec::new(),
            "offered package byte limit",
        ));
    }
    request.usage.admitted_input_bytes = bytes.len();
    if ByteDigest::of(bytes) != expected.digest() {
        return Err(request.error(
            Code::StaleDependency,
            Vec::new(),
            "package bytes do not match selected digest",
        ));
    }
    if std::str::from_utf8(bytes).is_err() {
        return Err(request.error(Code::InvalidUtf8, Vec::new(), "package bytes are not UTF-8"));
    }
    let recognized: intake::Recognized =
        request.decoded(intake::decode(bytes, package_limits), true)?;
    let Some(format) = recognized.0 else {
        return Err(request.error(
            Code::InvalidPackage,
            path(&["format"]),
            "package format must be a present string",
        ));
    };
    if format != view::FORMAT {
        return Err(request.error(
            Code::UnknownWire,
            path(&["format"]),
            "unsupported package wire format",
        ));
    }
    let mut wire: wire::Manifest =
        request.decoded(intake::decode_wire(bytes, package_limits), false)?;
    select(&request, &mut wire, support)?;
    request.stage = PackageStage::Rebind;
    authority(&request, &wire, &bindings, limits.link)?;
    let source = bindings.source.source();
    let unit = parse(
        source.identity().clone(),
        source.path(),
        source.text().as_bytes(),
        limits.syntax,
    )
    .map_err(|cause| request.native(cause))?;
    let linked = link_native(unit, models, limits.link).map_err(|cause| request.linking(cause))?;
    let checked = check(linked, bindings, limits.check).map_err(|cause| request.checking(cause))?;
    request.usage.checking = Some(*checked.usage());
    request.stage = PackageStage::Compare;
    let mut reconstructed = NativePackage::new(checked, package_limits).map_err(|mut error| {
        error.stage = PackageStage::Compare;
        error.usage.admitted_input_bytes = request.usage.admitted_input_bytes;
        error.usage.recognition = request.usage.recognition;
        error.usage.decode = request.usage.decode;
        error.usage.checking = request.usage.checking;
        error
    })?;
    request.usage.derive = reconstructed.usage.derive;
    request.usage.canonical = reconstructed.usage.canonical;
    request.usage.encode = reconstructed.usage.encode;
    let comparison = encoding::compare(&wire, &reconstructed.bytes, package_limits);
    request.usage.compare = Some(match &comparison {
        Ok(usage) => *usage,
        Err(failure) => failure.usage,
    });
    comparison.map_err(|failure| {
        let mut error = request.error(
            failure.code,
            failure.path,
            "package claims differ from actual reconstruction",
        );
        error.cause = Some(PackageCause::Json(failure.cause));
        error
    })?;
    reconstructed.bytes = bytes.to_vec();
    reconstructed.digest = expected.digest();
    reconstructed.usage = request.usage;
    Ok(reconstructed)
}

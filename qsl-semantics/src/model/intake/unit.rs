// SPDX-License-Identifier: AGPL-3.0-or-later
//! I1 for one source unit (ADR-011 §2.1 I1, FR-056): the unit's `model`
//! declarations, each admitted under FR-154 against the package input, read
//! into domain package records and normalized into the effective view E3
//! resolves the unit's model type names against.
//!
//! A declaration selecting a domain package spells its digest
//! `sha256-jcs:<hex>`. A `sha256:<hex>` digest selects a compiled-model
//! artifact, which has no admitted form on this path (FR-056-CON-4), so it
//! refuses at its declaration.

use std::collections::BTreeMap;

use qsl_foundation::selection::{ModelDigest, ModelSelection};
use qsl_foundation::{Code, IntakeLimits, Span};
use quire_exact::Cancel;

use super::{admit_located, read_records, PackageDocument};
use crate::model::accounting::{Incomplete, Meter, ModelNormalizationLimits};
use crate::model::domain_package::{DomainPackage, DomainPackageRef};
use crate::model::key::{raw_bytes_digest, SHA256_JCS_DIGEST_DOMAIN};
use crate::model::normalize::{normalize_packages, BatchDenial, EffectiveView, Refusals};

/// One `model` declaration of a unit, admitted at I1: its alias, the span
/// of its declaration and its domain package's effective view.
#[derive(Clone, Debug)]
pub struct SelectedModel {
    /// The declaration's alias, the `M` of `M::T`.
    pub alias: String,
    /// The span of the `model` declaration.
    pub span: Span,
    /// The admitted domain package's effective view.
    pub view: EffectiveView,
    originals: OriginalInventory,
}

/// Read-only handles into one actually admitted source document. Inventory
/// entries name original declaration nodes, including scalar declarations;
/// none are derived effective members or inputs to an effective identity.
#[derive(Clone, Debug)]
struct OriginalInventory {
    document: std::sync::Arc<serde_json::Value>,
    nodes: BTreeMap<crate::model::key::DeclarationKey, String>,
}

impl OriginalInventory {
    fn empty() -> Self {
        Self { document: std::sync::Arc::new(serde_json::Value::Null), nodes: BTreeMap::new() }
    }

    fn admitted(package: &str, document: PackageDocument) -> Self {
        let document = std::sync::Arc::new(document.tree);
        let mut nodes = BTreeMap::new();
        let mut pending = Vec::new();
        for collection in ["types", "populations"] {
            if let Some(entries) = document.get(collection).and_then(serde_json::Value::as_array) {
                pending.extend(entries.iter().enumerate().map(|(ordinal, node)|
                    (node, format!("/{collection}/{ordinal}"))));
            }
        }
        while let Some((node, pointer)) = pending.pop() {
            if let Some(identity) = node.get("identity").and_then(serde_json::Value::as_str) {
                nodes.insert(crate::model::key::DeclarationKey {
                    package: package.to_owned(), node: identity.to_owned(),
                }, pointer.clone());
            }
            for collection in ["fields", "operations", "params", "relationships", "constraints", "pre", "post"] {
                if let Some(entries) = node.get(collection).and_then(serde_json::Value::as_array) {
                    pending.extend(entries.iter().enumerate().map(|(ordinal, child)|
                        (child, format!("{pointer}/{collection}/{ordinal}"))));
                }
            }
        }
        Self { document, nodes }
    }

    fn node(&self, key: &crate::model::key::DeclarationKey) -> Option<&serde_json::Value> {
        self.document.pointer(self.nodes.get(key)?)
    }
}

impl SelectedModel {
    /// This key's own original admitted node. No fallback to an effective
    /// ancestor, another selection, or a caller-supplied origin is made.
    pub fn original_node(&self, key: &crate::model::key::DeclarationKey) -> Option<&serde_json::Value> {
        self.originals.node(key)
    }

    /// Every original declaration key, ascending by its full package/key.
    pub fn original_keys(&self) -> impl Iterator<Item = &crate::model::key::DeclarationKey> {
        self.originals.nodes.keys()
    }
}

/// One model-admission operation. Only intake constructs a nonempty owner;
/// assembly consumes it and carries its meter into checking.
#[derive(Debug)]
pub struct SelectedModels {
    entries: Vec<SelectedModel>,
    meter: Meter,
}

impl Default for SelectedModels {
    fn default() -> Self {
        Self { entries: Vec::new(), meter: Meter::new(ModelNormalizationLimits::default()) }
    }
}

impl std::ops::Deref for SelectedModels {
    type Target = [SelectedModel];
    fn deref(&self) -> &Self::Target { &self.entries }
}

impl SelectedModels {
    /// Links a one-package test fixture with its genuine admission meter.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_link_dispatch(
        self,
        original: &crate::model::key::DeclarationKey,
        closure: crate::model::dispatch::GeneralizationClosure,
    ) -> crate::model::dispatch::LinkCheckOutcome {
        let (entries, mut meter) = self.into_parts();
        assert_eq!(entries.len(), 1, "this fixture links one genuinely admitted package");
        crate::model::dispatch::link_dispatch(&entries[0].view, original, closure, &mut meter)
    }

    pub(crate) fn into_parts(self) -> (Vec<SelectedModel>, Meter) { (self.entries, self.meter) }

    /// Read-only accounting evidence from the actual operation owner.
    #[cfg(any(test, feature = "test-support"))]
    pub fn consumed(&self, kind: crate::model::accounting::LimitKind) -> u64 {
        self.meter.consumed(kind)
    }

    /// A bounded test fixture admits all packages in one genuine operation.
    /// Duplicate selected identities deliberately reach the checker's own
    /// invariant refusal; no owner or meter is appended after admission.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixtures(mut offered: Vec<(String, Span, DomainPackage)>,
        limits: ModelNormalizationLimits) -> Result<Self, UnitIntakeRefusal> {
        offered.sort_by(|left, right| {
            let left = &left.2.model_selection;
            let right = &right.2.model_selection;
            left.identity.as_bytes().cmp(right.identity.as_bytes())
                .then_with(|| left.digest.cmp(&right.digest))
        });
        let packages: Vec<_> = offered.iter().map(|(_, _, package)| package.clone()).collect();
        let mut meter = Meter::new(limits);
        let views = normalize_packages(&packages, &mut meter).map_err(|failure| {
            let cause = |ordinal: usize, cause| UnitIntakeRefusal {
                alias: offered[ordinal].0.clone(), span: offered[ordinal].1,
                cause, additional: Vec::new(),
            };
            match failure {
                BatchDenial::Incomplete(ordinal, incomplete) =>
                    cause(ordinal, UnitIntakeCause::Limit(incomplete)),
                BatchDenial::Refused(refusals) => {
                    let mut refusals = refusals.into_iter().map(|(ordinal, refusals)|
                        cause(ordinal, UnitIntakeCause::Refused(refusals)));
                    let mut first = refusals.next().expect("a refused batch has a refusing selection");
                    first.additional.extend(refusals);
                    first
                }
            }
        })?;
        Ok(Self { entries: views.into_iter().zip(offered).map(|(view, (alias, span, _))|
            SelectedModel { alias, span, view, originals: OriginalInventory::empty() }).collect(), meter })
    }

    /// A fixture uses the same private normalization producer and owns its
    /// actual admitted meter; it cannot pair an arbitrary view with a meter.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(alias: &str, span: Span, package: DomainPackage,
        limits: ModelNormalizationLimits) -> Result<Self, UnitIntakeRefusal> {
        let mut meter = Meter::new(limits);
        let packages = [package];
        let views = normalize_packages(&packages, &mut meter).map_err(|failure| {
            let cause = match failure {
                BatchDenial::Refused(mut refusals) => UnitIntakeCause::Refused(refusals.remove(0).1),
                BatchDenial::Incomplete(_, incomplete) => UnitIntakeCause::Limit(incomplete),
            };
            UnitIntakeRefusal { alias: alias.to_owned(), span, cause, additional: Vec::new() }
        })?;
        Ok(Self { entries: views.into_iter().map(|view| SelectedModel {
            alias: alias.to_owned(), span, view, originals: OriginalInventory::empty(),
        }).collect(), meter })
    }
}

/// Why I1 admitted no domain package for one `model` declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnitIntakeCause {
    /// The declaration selects a compiled-model artifact by its raw-byte
    /// `sha256:` digest, not a domain package by its `sha256-jcs:` one.
    ArtifactDigest,
    /// FR-154 admission, the record reader or normalization refused, with
    /// every refusal the refusing step reports.
    Refused(Refusals),
    /// Normalization reached a `ModelNormalizationLimitsV1` ceiling.
    Limit(Incomplete),
    /// The record reader refused with no refusal: a broken invariant of
    /// intake, never a property of the package.
    Invariant,
}

impl UnitIntakeCause {
    /// This cause's catalog code: the first model refusal's own, and
    /// `stage_limit_exceeded` for a normalization ceiling.
    pub fn code(&self) -> Code {
        match self {
            Self::ArtifactDigest => Code::InvalidModelBinding,
            Self::Refused(refusals) => refusals[0].code,
            Self::Limit(_) => Code::StageLimitExceeded,
            Self::Invariant => Code::RuntimeInvariant,
        }
    }
}

/// I1's refusal: the `model` declaration it concerns and why.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitIntakeRefusal {
    /// The declaration's alias.
    pub alias: String,
    /// The span of the `model` declaration.
    pub span: Span,
    /// The typed cause.
    pub cause: UnitIntakeCause,
    /// Other selections' failures from this same completed stage, in
    /// canonical selection order. An unfinished stage carries none.
    pub additional: Vec<UnitIntakeRefusal>,
}

/// No byte limit: how [`package_input`] reads a document to key it.
const UNBOUNDED: IntakeLimits = IntakeLimits {
    input_bytes: u64::MAX,
};

/// The package input FR-056 names: each supplied domain package document
/// keyed by its `sha256-jcs` digest. A document that does not parse is keyed
/// by the SHA-256 of its raw bytes, the digest FR-154 check 3 takes of such
/// bytes, so a selection of it refuses at admission rather than as a
/// missing package. Each document is parsed here to key it, under no byte
/// limit, and again when a selection admits it under the caller's
/// `intake.input_bytes`, so a document over that limit refuses naming it
/// and is never mistaken for a missing package.
pub fn package_input<'a>(
    documents: impl IntoIterator<Item = &'a [u8]>,
) -> BTreeMap<[u8; 32], Vec<u8>> {
    documents
        .into_iter()
        .map(|bytes| {
            let digest = PackageDocument::parse(bytes, UNBOUNDED)
                .map_or_else(|_| raw_bytes_digest(bytes), |document| document.jcs_digest);
            (digest, bytes.to_vec())
        })
        .collect()
}

/// Admit each of a unit's `model` declarations, in source order, against
/// `packages` (see [`package_input`]): FR-154 admission (one version of
/// each identity), [`read_records`], then normalization under `limits`. It
/// stops at the first declaration that does not admit.
pub fn admit_unit(
    selections: &[ModelSelection],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
) -> Result<SelectedModels, UnitIntakeRefusal> {
    admit_unit_with_cancel(selections, packages, limits, &Cancel::new())
}

/// [`admit_unit`] under the caller's [`Cancel`] handle, polled at every
/// normalization charge (FR-276).
pub fn admit_unit_with_cancel(
    selections: &[ModelSelection],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
    cancel: &Cancel,
) -> Result<SelectedModels, UnitIntakeRefusal> {
    let refuse = |selection: &ModelSelection, cause| UnitIntakeRefusal {
        alias: selection.alias.clone(),
        span: selection.span,
        cause,
        additional: Vec::new(),
    };
    let mut offered = Vec::with_capacity(selections.len());
    for selection in selections {
        let ModelDigest::DomainPackage(digest) = selection.model.digest() else {
            return Err(refuse(selection, UnitIntakeCause::ArtifactDigest));
        };
        offered.push((
            selection,
            DomainPackageRef {
                identity: selection.model.identity().to_owned(),
                version: selection.model.version().to_owned(),
                digest: digest.as_bytes(),
            },
        ));
    }
    offered.sort_by(|(_, a), (_, b)| a.identity.as_bytes().cmp(b.identity.as_bytes())
        .then_with(|| a.digest.cmp(&b.digest)));
    let admitted = admit_located(
        &offered,
        |(_, offered_ref)| offered_ref,
        SHA256_JCS_DIGEST_DOMAIN,
        packages,
        limits.intake,
    )
    .map_err(|((selection, _), refusal)| {
        refuse(
            selection,
            UnitIntakeCause::Refused(Refusals::new(refusal, Vec::new())),
        )
    })?;
    let mut owners = Vec::with_capacity(admitted.len());
    let mut selected_packages = Vec::with_capacity(admitted.len());
    let mut originals = Vec::with_capacity(admitted.len());
    let mut reader_failures = Vec::new();
    for ((selection, _), package_ref, document) in admitted {
        match read_records(&package_ref.identity, &document) {
            Ok(records) => {
                owners.push(selection);
                originals.push(OriginalInventory::admitted(&package_ref.identity, document));
                selected_packages.push(DomainPackage::new(package_ref, records));
            }
            Err(refusals) => {
                let cause = Refusals::try_from(refusals)
                    .map_or(UnitIntakeCause::Invariant, UnitIntakeCause::Refused);
                reader_failures.push(refuse(selection, cause));
            }
        }
    }
    if !reader_failures.is_empty() {
        let mut first = reader_failures.remove(0);
        first.additional = reader_failures;
        return Err(first);
    }
    let mut meter = Meter::new(limits).with_cancel(cancel.clone());
    let views = normalize_packages(&selected_packages, &mut meter).map_err(|failure| {
        match failure {
            BatchDenial::Incomplete(ordinal, incomplete) => refuse(owners[ordinal], UnitIntakeCause::Limit(incomplete)),
            BatchDenial::Refused(refusals) => {
                let mut failures = refusals.into_iter().map(|(ordinal, refusals)|
                    refuse(owners[ordinal], UnitIntakeCause::Refused(refusals)));
                let mut first = failures.next().expect("a refused stage has a refusal");
                first.additional = failures.collect();
                first
            }
        }
    })?;
    Ok(SelectedModels {
        entries: owners.into_iter().zip(views).zip(originals).map(|((selection, view), originals)| SelectedModel {
            alias: selection.alias.clone(), span: selection.span, view, originals,
        }).collect(), meter,
    })
}

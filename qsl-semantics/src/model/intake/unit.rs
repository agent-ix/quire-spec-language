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
            for collection in ["fields", "operations", "params", "relationships", "constraints", "clauses", "pre", "post"] {
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
        let views = normalize_packages(&packages, &mut meter)
            .and_then(|views| { check_systems_packages(&views, &mut meter)?; Ok(views) }).map_err(|failure| {
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
        let views = normalize_packages(&packages, &mut meter)
            .and_then(|views| { check_systems_packages(&views, &mut meter)?; Ok(views) }).map_err(|failure| {
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

/// The completed systems stage shares the normalization operation's meter
/// and index. No failures from an unfinished stage escape a denied charge.
fn check_systems_packages(views: &[EffectiveView], meter: &mut Meter) -> Result<(), BatchDenial> {
    use crate::model::domain_package::DomainPackageRecord;
    use crate::model::systems::{classify, check_connection, check_allocation,
        Kind, ConnectionCheckOutcome, ConnectionOutcome, AllocationCheckOutcome};
    let mut failures = Vec::new();
    for (ordinal, view) in views.iter().enumerate() {
        let classification = classify(view.domain_package(), meter)
            .map_err(|incomplete| BatchDenial::Incomplete(ordinal, incomplete))?;
        let mut refused = classification.refusals.clone();
        let mut connections: Vec<_> = view.domain_package().records.iter().filter_map(|record| {
            if let DomainPackageRecord::Relationship(edge) = record {
                (classification.actual_kind(&edge.key) == Kind::Connection).then_some(&edge.key)
            } else { None }
        }).collect();
        connections.sort();
        for key in connections {
            match check_connection(view.model_index(), &classification, key, meter) {
                ConnectionCheckOutcome::Incomplete(incomplete) => return Err(BatchDenial::Incomplete(ordinal, incomplete)),
                ConnectionCheckOutcome::Refused(refusal) => refused.push(refusal),
                ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(conditions)) =>
                    refused.extend(conditions.into_iter().map(|condition| crate::model::normalize::ModelRefusal {
                        code: condition.code, cause: condition.cause, detail: condition.detail,
                    })),
                ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted) => {},
            }
        }
        let mut allocations: Vec<_> = view.domain_package().records.iter().filter_map(|record| {
            if let DomainPackageRecord::Allocation(edge) = record { Some(&edge.key) } else { None }
        }).collect();
        allocations.sort();
        for key in allocations {
            match check_allocation(&classification, key, meter) {
                AllocationCheckOutcome::Incomplete(incomplete) => return Err(BatchDenial::Incomplete(ordinal, incomplete)),
                AllocationCheckOutcome::Refused(refusal) => refused.push(refusal),
                AllocationCheckOutcome::Admitted => {},
            }
        }
        if !refused.is_empty() {
            failures.push((ordinal, Refusals::try_from(refused).expect("a completed failing stage has refusals")));
        }
    }
    if failures.is_empty() { Ok(()) } else { Err(BatchDenial::Refused(failures)) }
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
    let views = normalize_packages(&selected_packages, &mut meter)
        .and_then(|views| { check_systems_packages(&views, &mut meter)?; Ok(views) }).map_err(|failure| {
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

#[cfg(test)]
mod systems_operation_controls {
    use super::*;
    use crate::model::accounting::{ChargePoint, LimitKind};
    use crate::model::normalize::{ModelRefusalCause, normalize_packages};
    use ix_trace_rs::trace;
    use serde_json::{json, Value};

    const PACKAGE: &str = "example/systems";
    const DIGEST: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
    fn key(name: &str) -> String { format!("ix://{PACKAGE}/{name}") }
    fn document(wrong_port: bool, reverse: bool) -> Vec<u8> {
        let origin = json!({"source": {"sourceIdentity": key("spec"), "path": "systems.md", "startLine": 1, "startColumn": 1}});
        let multiplicity = json!({"lower": 1, "upper": 1, "ordered": false, "unique": true});
        let node = |name: &str, kind: &str, extra: Value| {
            let mut value = json!({"identity": key(name), "displayName": name,
                "kind": {"module": PACKAGE, "name": kind}, "roles": [], "origin": origin,
                "constraints": [], "extensions": [], "unknownPolicy": "reject", "fields": [], "operations": [], "relationships": []});
            value.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone()); value
        };
        let construct = |name: &str, meaning: &str| json!({"kind": {"module": PACKAGE, "name": name},
            "moduleVersion": "1.0.0", "manifestDigest": DIGEST,
            "construct": {"identity": "none", "shape": "record", "members": {}, "meaning": meaning}});
        let mut types = vec![
            node("Composite", "object", json!({"supertypes": []})),
            node("Thing", "object", json!({"supertypes": []})),
            node("I", "interface", json!({"supertypes": [], "featureOrder": []})),
            node("ISub", "interface", json!({"supertypes": [key("I")], "featureOrder": []})),
            node("Part", "part", json!({"owner": key("Composite"), "declaredType": key("Thing"), "multiplicity": multiplicity})),
            node("Out", "port", json!({"owner": key(if wrong_port { "Composite" } else { "Part" }),
                "interfaceType": key(if wrong_port { "Thing" } else { "ISub" }), "direction": "out", "multiplicity": multiplicity})),
            node("In", "port", json!({"owner": key("Part"), "interfaceType": key("I"), "direction": "in", "multiplicity": multiplicity})),
            node("Link", "connection", json!({"sourceEnd": {"type": key("Out"), "multiplicity": multiplicity},
                "targetEnd": {"type": key("In"), "multiplicity": multiplicity}, "flowDirection": "source-to-target"})),
        ];
        if reverse { types.reverse(); }
        serde_json::to_vec(&json!({"contractVersion": "2.0.0",
            "source": {"identity": key("spec"), "version": "1.0.0", "dialect": "spec-bundle", "digest": DIGEST},
            "package": {"identity": PACKAGE, "version": "1.0.0", "manifestDigest": DIGEST,
                "mappingVersions": [], "profileVersions": [], "lockDigest": DIGEST},
            "occurrences": [], "extensions": [], "populations": [], "types": types,
            "constructs": [construct("object", super::super::meaning::OBJECT_TYPE),
                construct("interface", super::super::meaning::SYSTEMS_INTERFACE), construct("part", super::super::meaning::SYSTEMS_PART),
                construct("port", super::super::meaning::SYSTEMS_PORT), construct("connection", super::super::meaning::SYSTEMS_CONNECTION)]})).unwrap()
    }
    fn selection(bytes: &[u8]) -> (qsl_forms::ParsedUnit, BTreeMap<[u8; 32], Vec<u8>>) {
        let packages = package_input([bytes]);
        let digest = crate::model::key::hex(packages.keys().next().unwrap());
        let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nmodel M = {PACKAGE:?} version \"1.0.0\" digest \"sha256-jcs:{digest}\";\nfunction noop using v(): Boolean pure {{ true }}\n");
        let parsed = qsl_cst::parse(qsl_foundation::SourceIdentity::new("test", "systems", "fixture", "1"),
            "unit.native", text.as_bytes(), qsl_cst::Limits::default()).unwrap();
        assert!(parsed.diagnostics().is_empty());
        (qsl_forms::build_unit(&parsed).unwrap(), packages)
    }
    fn normalization_prefix(bytes: &[u8]) -> u64 {
        let document = PackageDocument::parse(bytes, UNBOUNDED).unwrap();
        let package_identity = document.tree["package"]["identity"].as_str().unwrap();
        let records = read_records(package_identity, &document).unwrap();
        let package = DomainPackage::new(DomainPackageRef { identity: package_identity.to_owned(), version: "1.0.0".to_owned(), digest: document.jcs_digest }, records);
        let mut meter = Meter::new(ModelNormalizationLimits::UNLIMITED);
        normalize_packages(&[package], &mut meter).unwrap();
        meter.consumed(LimitKind::WorkUnits)
    }

    #[trace("QSpec-TC-196", "QSpec-TC-213", "FR-082-AC-8")]
    #[test]
    fn actual_intake_charges_interfaces_first_and_connection_type_facts_once() {
        for reverse in [false, true] {
            let bytes = document(false, reverse);
            let prefix = normalization_prefix(&bytes);
            let (unit, packages) = selection(&bytes);
            let selected = admit_unit(&unit.selections().models, &packages, ModelNormalizationLimits::UNLIMITED).unwrap();
            // Two interfaces, one part, two ports, one connection; then
            // direction1 + actual ISub qualify/inherit2 + multiplicity1.
            assert_eq!(selected.consumed(LimitKind::WorkUnits), prefix + 6 + 4);
            assert_eq!(selected[0].original_node(&crate::model::key::DeclarationKey { package: PACKAGE.to_owned(), node: key("Out") }).unwrap()["origin"]["source"]["path"], "systems.md");
        }
    }
    #[trace("QSpec-TC-196", "QSpec-TC-213", "FR-082-AC-8")]
    #[test]
    fn actual_intake_preserves_all_port_kind_failures_and_connection_cascade() {
        for reverse in [false, true] {
            let bytes = document(true, reverse);
            let (unit, packages) = selection(&bytes);
            let refused = admit_unit(&unit.selections().models, &packages, ModelNormalizationLimits::UNLIMITED)
                .expect_err("all applicable kind failures precede source checking");
            assert!(refused.additional.is_empty());
            let UnitIntakeCause::Refused(refusals) = refused.cause else { panic!("expected systems kind refusals"); };
            assert_eq!(refusals.len(), 3);
            for refusal in refusals.iter() {
                assert_eq!(refusal.code, Code::InvalidModelBinding);
                assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
            }
            assert_eq!(refusals.iter().map(|refusal| refusal.detail.as_str()).collect::<Vec<_>>(), vec![
                "ix://example/systems/Out: required kind Part, actual kind none",
                "ix://example/systems/Out: required kind Interface, actual kind none",
                "source end of ix://example/systems/Link: required kind Port, actual kind none",
            ]);
        }
    }
    #[trace("QSpec-TC-213", "FR-082-AC-8")]
    #[test]
    fn actual_intake_denies_interface_kind_or_type_condition_without_partial_failures() {
        let bytes = document(false, false);
        let prefix = normalization_prefix(&bytes);
        for (bound, consumed, next, point) in [
            (prefix + 1, prefix + 1, 1, ChargePoint::SystemsKind),
            (prefix + 8, prefix + 7, 2, ChargePoint::SystemsConnectionCondition),
        ] {
            let (unit, packages) = selection(&bytes);
            let refused = admit_unit(&unit.selections().models, &packages, ModelNormalizationLimits {
                work_units: bound, ..ModelNormalizationLimits::UNLIMITED }).expect_err("actual stage work must fit the next full charge");
            assert!(refused.additional.is_empty());
            assert_eq!(refused.cause, UnitIntakeCause::Limit(Incomplete { limit_kind: LimitKind::WorkUnits,
                limit: bound, consumed, next_charge: next, charge_point: point }));
        }
    }

    #[trace("QSpec-TC-213", "FR-082-AC-8")]
    #[test]
    fn actual_systems_stage_reports_all_models_canonically_and_discards_unfinished_failures() {
        for reverse in [false, true] {
            for deny in [false, true] {
                let first = document(true, false);
                let second = String::from_utf8(document(!deny, true)).unwrap()
                    .replace(PACKAGE, "example/systems-z").into_bytes();
                let packages = package_input([first.as_slice(), second.as_slice()]);
                let digest_first = crate::model::key::hex(&PackageDocument::parse(&first, UNBOUNDED).unwrap().jcs_digest);
                let digest_second = crate::model::key::hex(&PackageDocument::parse(&second, UNBOUNDED).unwrap().jcs_digest);
                let mut selections = vec![format!("model Z = {PACKAGE:?} version \"1.0.0\" digest \"sha256-jcs:{digest_first}\";"),
                    format!("model A = \"example/systems-z\" version \"1.0.0\" digest \"sha256-jcs:{digest_second}\";")];
                if reverse { selections.reverse(); }
                let text = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n{}\nfunction noop using v(): Boolean pure {{ true }}\n", selections.join("\n"));
                let parsed = qsl_cst::parse(qsl_foundation::SourceIdentity::new("test", "systems", "fixture", "2"),
                    "unit.native", text.as_bytes(), qsl_cst::Limits::default()).unwrap();
                assert!(parsed.diagnostics().is_empty());
                let built = qsl_forms::build_unit(&parsed).unwrap();
                let prefix = normalization_prefix(&first) + normalization_prefix(&second);
                let bound = if deny { prefix + 7 } else { u64::MAX };
                let refused = admit_unit(&built.selections().models, &packages, ModelNormalizationLimits {
                    work_units: bound, ..ModelNormalizationLimits::UNLIMITED }).expect_err("systems stage completes all selected models or exposes only its first unavailable charge");
                if deny {
                    assert_eq!(refused.alias, "A");
                    assert!(refused.additional.is_empty());
                    assert_eq!(refused.cause, UnitIntakeCause::Limit(Incomplete { limit_kind: LimitKind::WorkUnits,
                        limit: bound, consumed: bound, next_charge: 1, charge_point: ChargePoint::SystemsKind }));
                } else {
                    assert_eq!(refused.alias, "Z", "selection identity order precedes aliases/source order");
                    assert_eq!(refused.additional.len(), 1);
                    assert_eq!(refused.additional[0].alias, "A");
                    for (failure, package) in [(&refused, PACKAGE), (&refused.additional[0], "example/systems-z")] {
                        let UnitIntakeCause::Refused(refusals) = &failure.cause else { panic!("expected completed systems failures"); };
                        assert_eq!(refusals.len(), 3);
                        assert!(refusals.iter().all(|refusal| refusal.code == Code::InvalidModelBinding && refusal.cause == ModelRefusalCause::WrongExport));
                        assert_eq!(refusals.iter().map(|refusal| refusal.detail.clone()).collect::<Vec<_>>(), vec![
                            format!("ix://{package}/Out: required kind Part, actual kind none"),
                            format!("ix://{package}/Out: required kind Interface, actual kind none"),
                            format!("source end of ix://{package}/Link: required kind Port, actual kind none"),
                        ]);
                    }
                }
            }
        }
    }


    #[trace("QSpec-TC-213", "FR-082-AC-8")]
    #[test]
    fn actual_intake_ancestor_denial_stays_in_normalization_and_direct_walk_keeps_exact_payload() {
        let bytes = document(false, false);
        let (unit, packages) = selection(&bytes);
        let refusal = admit_unit(&unit.selections().models, &packages, ModelNormalizationLimits {
            ancestor_steps: 0, ..ModelNormalizationLimits::UNLIMITED }).expect_err("normalization encounters this edge before systems can run");
        assert!(refusal.additional.is_empty());
        let expected = Incomplete { limit_kind: LimitKind::AncestorSteps, limit: 0,
            consumed: 0, next_charge: 1, charge_point: ChargePoint::ModelAncestorEdge };
        assert_eq!(refusal.cause, UnitIntakeCause::Limit(expected));
        let selected = admit_unit(&unit.selections().models, &packages, ModelNormalizationLimits::UNLIMITED).unwrap();
        let declaration = |name| crate::model::key::DeclarationKey { package: PACKAGE.to_owned(), node: key(name) };
        // This is an explicitly direct walk seam control on the real shared
        // index, not a fictitious reachable systems-stage meter denial.
        assert_eq!(selected[0].view.model_index().conformance_walk(&declaration("ISub"), &declaration("I"), 0), Err(expected));
        assert_eq!(selected[0].view.model_index().conformance_walk(&declaration("ISub"), &declaration("I"), 1), Ok(true));
    }

}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! Model-layer inputs: a Semantic IR 2.0.0 document with a caller-chosen
//! number of object types, carried through FR-154 model intake
//! (`intake::admit` -> `intake::read_records` -> `DomainPackage::new`) into
//! normalization, population admission and `allInstances`.
//!
//! The document is authored in QSpec's own identity form
//! (`ix://<package>/<name>`), the same form
//! `qsl-semantics/tests/it/model_intake.rs`'s
//! `a_qspec_conformant_document_admits_reads_and_classifies` uses: a
//! document lifted by FCD's `lift` from a spec bundle is refused by
//! `read_records` at the pinned FCD revision (FCD's `ix://<pkg>/type/<id>`
//! identity form; see `intake.rs`'s module doc), so it cannot build a large
//! `DomainPackage` today. Everything after the bytes -- the admission
//! table, `agent-ix-semantic-ir`'s validator, the per-node reader -- is the
//! production intake path.
//!
//! ## Shape
//!
//! `types` object types, each with one native `Boolean` field:
//!
//! - a supertype chain `C0 <- C1 <- ... <- C{depth}` (`C{i}` declares
//!   `supertypes: [C{i-1}]`), so `C{depth}` has `depth` proper ancestors;
//! - `types - depth - 1` filler types `F{i}`, each declaring
//!   `supertypes: [C0]`, so the whole package is one connected component
//!   (one object universe);
//! - one closed population `Pop` whose member type is `C0`.
//!
//! [`population_document`] fills `Pop` with objects whose most-specific
//! type is the chain's deepest type, so an `allInstances<C0>` query walks
//! all `depth` generalization steps per member.

use std::collections::BTreeMap;

use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::conformance::{resolve_redefinition_target, RedefinitionTargetOutcome};
use qsl_semantics::model::dispatch::GeneralizationClosure;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, OperationEffect,
};
use qsl_semantics::model::intake::{admit, meaning, read_records, PackageDocument};
use qsl_semantics::model::key::{DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use qsl_semantics::model::normalize::{normalize, EffectiveView, ModelRefusal, NormalizeOutcome};
use qsl_semantics::model::population::PopulationBinding;
use qsl_semantics::model::population::{
    admit_binding, admit_invocation, AdmissionMeter, AdmissionOutcome, InvocationContext,
    InvocationDelta, PopulationAdmissionLimits, PopulationDocument, PopulationMember,
};
use qsl_semantics::value::model_query::{evaluate_all_instances, ModelQueryHalt};
use quire_exact::{CardinalityBound, CollectionKind, CollectionType, Meter, ValueType};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// The benchmark domain package's identity.
pub const PACKAGE: &str = "bench/model";

/// The package version every selection names.
pub const VERSION: &str = "1.0.0";

const PLACEHOLDER_DIGEST: &str =
    "sha256:0000000000000000000000000000000000000000000000000000000000000000";

/// A model document's size parameters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelShape {
    /// Object types declared (each also declares one field member record).
    pub types: usize,
    /// Proper ancestors of the chain's deepest type (`A` in F6's O(N x A)).
    pub depth: usize,
}

impl ModelShape {
    /// The filler type count: every declared type not on the chain.
    fn fillers(self) -> usize {
        self.types.saturating_sub(self.depth + 1)
    }
}

/// `ix://bench/model/<name>`: QSpec's identity form for a package node.
pub fn node(name: &str) -> String {
    format!("ix://{PACKAGE}/{name}")
}

/// The declaration key of the package node named `name`.
pub fn key(name: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: node(name),
    }
}

/// The chain's `index`-th type's name, `C{index}`.
pub fn chain_type(index: usize) -> String {
    format!("C{index}")
}

fn origin(identity: &str) -> Value {
    json!({
        "generated": {
            "generatorIdentity": identity,
            "generatorVersion": "1.0.0",
            "inputIdentities": [identity],
        }
    })
}

fn construct(name: &str, meaning: &str) -> Value {
    json!({
        "kind": {"module": PACKAGE, "name": name},
        "moduleVersion": "1.0.0",
        "manifestDigest": PLACEHOLDER_DIGEST,
        "construct": {
            "identity": "none",
            "shape": "record",
            "members": {},
            "meaning": meaning,
        },
    })
}

fn object_type(name: &str, supertypes: &[String]) -> Value {
    let identity = node(name);
    let field = format!("{identity}/flag");
    json!({
        "identity": identity,
        "displayName": name,
        "kind": {"module": PACKAGE, "name": "object_type"},
        "roles": [],
        "origin": origin(&identity),
        "constraints": [],
        "extensions": [],
        "unknownPolicy": "reject",
        "supertypes": supertypes,
        "fields": [{
            "identity": field,
            "name": "flag",
            "typeRef": "ix://quire/native/Boolean",
            "presence": "optional",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 0, "ordered": false, "unique": true},
            "origin": origin(&field),
        }],
        "operations": [],
    })
}

/// The Semantic IR 2.0.0 document for `shape`, as the exact bytes intake
/// admits: `serde_json`'s compact output, whose key order is already
/// sorted, so these bytes are their own JCS form.
pub fn document(shape: ModelShape) -> Vec<u8> {
    let mut types = Vec::with_capacity(shape.types.max(shape.depth + 1));
    for index in 0..=shape.depth {
        let supertypes = match index.checked_sub(1) {
            Some(parent) => vec![node(&chain_type(parent))],
            None => Vec::new(),
        };
        types.push(object_type(&chain_type(index), &supertypes));
    }
    for index in 0..shape.fillers() {
        types.push(object_type(&format!("F{index}"), &[node(&chain_type(0))]));
    }
    let population = node("Pop");
    let document = json!({
        "contractVersion": "2.0.0",
        "source": {
            "identity": node("spec"),
            "version": VERSION,
            "dialect": "spec-bundle",
            "digest": PLACEHOLDER_DIGEST,
        },
        "package": {
            "identity": PACKAGE,
            "version": VERSION,
            "manifestDigest": PLACEHOLDER_DIGEST,
            "mappingVersions": [],
            "profileVersions": [],
            "lockDigest": PLACEHOLDER_DIGEST,
        },
        "occurrences": [],
        "extensions": [],
        "constructs": [
            construct("object_type", meaning::OBJECT_TYPE),
            construct("population", meaning::POPULATION),
        ],
        "types": types,
        "populations": [{
            "identity": population,
            "displayName": "Pop",
            "kind": {"module": PACKAGE, "name": "population"},
            "members": [node(&chain_type(0))],
            "extent": "closed",
            "origin": origin(&population),
        }],
    });
    serde_json::to_vec(&document).expect("a json! value always serializes")
}

/// A domain package selection and the byte map FR-154's admission table
/// reads it from.
#[derive(Clone, Debug)]
pub struct Offer {
    /// The offered `{identity, version, digest}` selection.
    pub selection: DomainPackageRef,
    /// The package bytes, keyed by their SHA-256-over-JCS digest.
    pub bytes_by_digest: BTreeMap<[u8; 32], Vec<u8>>,
}

/// Offer `document` under its own identity, version and digest.
pub fn offer(document: Vec<u8>) -> Offer {
    let digest: [u8; 32] = Sha256::digest(&document).into();
    Offer {
        selection: DomainPackageRef {
            identity: PACKAGE.to_owned(),
            version: VERSION.to_owned(),
            digest,
        },
        bytes_by_digest: BTreeMap::from([(digest, document)]),
    }
}

/// Which intake stage refused.
#[derive(Clone, Debug)]
pub enum IntakeFailure {
    /// FR-154's four-check admission table (`intake::admit`) refused.
    Admit(Box<ModelRefusal>),
    /// The per-node reader (`intake::read_records`) refused these nodes.
    Read(Vec<ModelRefusal>),
}

// Every call into `model::intake` goes through the three functions below,
// so an intake signature change is one edit here, not one per bench.

/// FR-154's admission table (`intake::admit`) over `offer`: the selection
/// and the package document it parsed.
pub fn admit_offer(
    offer: &Offer,
) -> Result<(DomainPackageRef, PackageDocument), Box<ModelRefusal>> {
    admit(
        &offer.selection,
        SHA256_JCS_DIGEST_DOMAIN,
        &offer.bytes_by_digest,
    )
    .map_err(Box::new)
}

/// Parse package bytes the way `admit` does (`PackageDocument::parse`).
pub fn parse_document(bytes: &[u8]) -> Result<PackageDocument, Box<ModelRefusal>> {
    PackageDocument::parse(bytes).map_err(Box::new)
}

/// The per-node reader (`intake::read_records`) over an admitted document.
pub fn read(document: &PackageDocument) -> Result<Vec<DomainPackageRecord>, Vec<ModelRefusal>> {
    read_records(PACKAGE, document)
}

/// FR-154 intake: [`admit_offer`], then [`read`], then `DomainPackage::new`.
pub fn intake(offer: &Offer) -> Result<DomainPackage, IntakeFailure> {
    let (selection, document) = admit_offer(offer).map_err(IntakeFailure::Admit)?;
    let records = read(&document).map_err(IntakeFailure::Read)?;
    Ok(DomainPackage::new(selection, records))
}

/// FR-150 normalization of `domain_package` under unlimited limits.
///
/// # Panics
///
/// Panics when normalization does not complete: every generated shape is a
/// valid, acyclic, single-component package.
pub fn view(domain_package: &DomainPackage) -> EffectiveView {
    match normalize(domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("the generated package normalizes: {other:?}"),
    }
}

/// A population document of `members` objects `o0` .. `o{members - 1}`,
/// each of the chain's deepest type `C{depth}`.
pub fn population_document(shape: ModelShape, members: usize) -> PopulationDocument {
    let deepest = key(&chain_type(shape.depth));
    PopulationDocument {
        model_identity: PACKAGE.to_owned(),
        members: (0..members)
            .map(|index| PopulationMember {
                object: format!("o{index}"),
                type_identity: deepest.clone(),
                field_values: Vec::new(),
            })
            .collect(),
    }
}

/// FR-153 binding admission of `document` into `Pop`, with a closed
/// subtype closure and a declared maximum of `document`'s own size.
pub fn admit_population(
    domain_package: &DomainPackage,
    view: &EffectiveView,
    document: &PopulationDocument,
) -> AdmissionOutcome {
    let mut meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    admit_binding(
        domain_package,
        view,
        document,
        &key("Pop"),
        GeneralizationClosure::Closed,
        Some(u64::try_from(document.members.len()).unwrap_or(u64::MAX)),
        &mut meter,
    )
}

/// FR-153 invocation admission of `document` as both the pre and the post
/// population of one invocation whose frame changes nothing (an empty
/// effect, no created or deleted object): the admission work of
/// [`admit_population`] done at both instants, plus a frame check that
/// passes.
pub fn admit_unchanged_invocation(
    domain_package: &DomainPackage,
    view: &EffectiveView,
    document: &PopulationDocument,
) -> AdmissionOutcome {
    let population = key("Pop");
    let effect = OperationEffect::default();
    let mut pre_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let mut post_meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    admit_invocation(
        InvocationContext {
            domain_package,
            view,
            population: &population,
            subtype_closure: GeneralizationClosure::Closed,
            declared_maximum: Some(u64::try_from(document.members.len()).unwrap_or(u64::MAX)),
        },
        document,
        document,
        &InvocationDelta {
            effect: &effect,
            declared_created: &[],
            declared_deleted: &[],
        },
        &mut pre_meter,
        &mut post_meter,
    )
}

/// A generated package, its effective view and an admitted `Pop` binding of
/// `members` objects of the chain's deepest type.
///
/// # Panics
///
/// Panics when intake or admission refuses: every generated shape is valid.
pub fn admitted(shape: ModelShape, members: usize) -> (DomainPackage, PopulationBinding) {
    let domain_package =
        intake(&offer(document(shape))).expect("the generated document passes intake");
    let effective = view(&domain_package);
    match admit_population(
        &domain_package,
        &effective,
        &population_document(shape, members),
    ) {
        AdmissionOutcome::Admitted(binding) => (domain_package, binding),
        other => panic!("the generated population admits: {other:?}"),
    }
}

/// `allInstances<C0>` through the evaluator-facing bridge
/// (`value::model_query::evaluate_all_instances`), which resolves `C0`'s
/// effective identity through a reverse catalog it rebuilds per query.
///
/// # Panics
///
/// Panics when `binding`'s catalog does not name `C0`.
pub fn query_all_instances_of_root(
    binding: &PopulationBinding,
) -> Result<quire_exact::Value, ModelQueryHalt> {
    let root = *binding
        .type_catalog()
        .get(&key(&chain_type(0)))
        .expect("C0 is a declared object type");
    let maximum = binding.declared_maximum().unwrap_or(u64::MAX);
    let collection = CollectionType::new(
        CollectionKind::Set,
        ValueType::Reference(root),
        CardinalityBound::new(0, maximum).expect("0 <= any maximum"),
    );
    let mut meter = Meter::new(crate::check::SCALAR_UNLIMITED);
    evaluate_all_instances(binding, &collection, &mut meter)
}

/// `resolve_redefinition_target` for the chain root's field: a model
/// conformance entry point that builds `ConformanceIndex` over the whole
/// package on every call.
pub fn resolve_root_field_redefinition(
    domain_package: &DomainPackage,
) -> Result<RedefinitionTargetOutcome, Box<ModelRefusal>> {
    let field = DeclarationKey {
        package: PACKAGE.to_owned(),
        node: format!("{}/flag", node(&chain_type(0))),
    };
    resolve_redefinition_target(
        domain_package,
        &field,
        ModelNormalizationLimits::UNLIMITED.ancestor_steps,
    )
    .map_err(Box::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smallest_shape_admits_and_queries() {
        let shape = ModelShape { types: 1, depth: 0 };
        let (domain_package, binding) = admitted(shape, 1);
        assert_eq!(domain_package.records.len(), 3);
        assert!(query_all_instances_of_root(&binding).is_ok());
        assert!(resolve_root_field_redefinition(&domain_package).is_ok());
        let effective = view(&domain_package);
        assert!(matches!(
            admit_unchanged_invocation(&domain_package, &effective, &population_document(shape, 1)),
            AdmissionOutcome::Admitted(_)
        ));
    }
}

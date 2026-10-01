// SPDX-License-Identifier: AGPL-3.0-or-later
//! `library::bundle` tests (TC-491). They hold the scenarios that were backed
//! by the retired source-package resolver (QSpec FR-131-AC-1 to AC-3,
//! FR-339-AC-3), now over `link_bundle`, whose roots are exact `DefinitionRef`s and so need no
//! parsed source.
use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_cst::CompleteCause;
use qsl_foundation::diagnostic::LimitKind;
use qsl_foundation::selection::{DefinitionRef, MAX_SELECTED_DEFINITIONS};
use qsl_foundation::Code;

use super::bundle::{
    link_bundle, BundleRefusal, CapabilityId, Definition, DefinitionCatalog, DefinitionRole, Facet,
    LinkedBundle, PackageError, PackageLimitKind, PackageLimits, ReaderAuthority, ResolutionCause,
};

const READER_AUTHORITY: ReaderAuthority = ReaderAuthority::fixture();

const COMPLETE_DEFINITION_ROLES: [(&str, DefinitionRole); 9] = [
    ("quire.source.complete/v1", DefinitionRole::Source),
    (
        "quire.value.complete/v1",
        DefinitionRole::ValueModelExpression,
    ),
    ("quire.temporal.complete/v1", DefinitionRole::Temporal),
    ("quire.observation.complete/v1", DefinitionRole::Observation),
    ("quire.protocol.complete/v1", DefinitionRole::Protocol),
    ("quire.runtime.complete/v1", DefinitionRole::Runtime),
    ("quire.method.complete/v1", DefinitionRole::MethodPlan),
    ("quire.backend.complete/v1", DefinitionRole::Backend),
    ("quire.tooling.complete/v1", DefinitionRole::ToolingEvidence),
];

fn inventory() -> BTreeSet<CapabilityId> {
    CapabilityId::complete_inventory().into_iter().collect()
}

fn definition(
    identity: &str,
    version: &str,
    role: DefinitionRole,
    dependencies: BTreeSet<DefinitionRef>,
    capabilities: BTreeSet<CapabilityId>,
    bytes: &[u8],
) -> Definition {
    Definition::from_exact_bytes(
        &READER_AUTHORITY,
        identity,
        version,
        role,
        dependencies,
        capabilities,
        bytes,
    )
    .unwrap()
}

/// The nine complete-V1 facet definitions; the first carries the whole
/// 176-capability inventory.
fn complete_definitions() -> Vec<Definition> {
    COMPLETE_DEFINITION_ROLES
        .into_iter()
        .enumerate()
        .map(|(index, (identity, role))| {
            definition(
                identity,
                "1",
                role,
                BTreeSet::new(),
                if index == 0 {
                    inventory()
                } else {
                    BTreeSet::new()
                },
                format!("fixture complete definition artifact {index}\n").as_bytes(),
            )
        })
        .collect()
}

fn reinterpret_definition(
    definition: &Definition,
    role: DefinitionRole,
    dependencies: BTreeSet<DefinitionRef>,
    capabilities: BTreeSet<CapabilityId>,
) -> Definition {
    self::definition(
        definition.exact().identity(),
        definition.exact().version(),
        role,
        dependencies,
        capabilities,
        definition.exact_bytes(),
    )
}

fn roots(definitions: &[Definition]) -> Vec<DefinitionRef> {
    definitions
        .iter()
        .map(|definition| definition.exact().clone())
        .collect()
}

/// Link the first `root_count` definitions as roots over a catalog of all of
/// them.
fn link_roots(
    definitions: &[Definition],
    root_count: usize,
    limits: PackageLimits,
) -> Result<LinkedBundle, BundleRefusal> {
    link_bundle(
        &roots(&definitions[..root_count]),
        &DefinitionCatalog::new(definitions.to_vec()).unwrap(),
        limits,
    )
}

fn link_complete(definitions: &[Definition]) -> LinkedBundle {
    link_roots(definitions, 9, PackageLimits::default()).unwrap()
}

/// FR-111-AC-7: the refusal's (code, cause) pair is one the catalog lists.
fn assert_catalogued(refusal: &BundleRefusal) {
    assert!(
        refusal.cause_tag.is_cause_of(refusal.code),
        "{:?} is not a cause of {:?}",
        refusal.cause_tag,
        refusal.code
    );
}

#[trace("TC-491", "FR-111-AC-1", "FR-131-AC-1", "FR-131-AC-2")]
#[test]
fn capability_inventory_round_trips_through_one_shared_family_authority() {
    let inventory = CapabilityId::complete_inventory();
    assert_eq!(inventory.len(), 176);
    for capability in &inventory {
        assert_eq!(
            CapabilityId::complete(capability.as_str()).unwrap(),
            *capability
        );
    }
    for outside in [
        "V1-SRC-000",
        "V1-SRC-016",
        "V1-TYPE-032",
        "V1-TOOL-011",
        "V1-OTHER-001",
    ] {
        assert!(matches!(
            CapabilityId::complete(outside),
            Err(PackageError::UnknownCapability(value)) if value == outside
        ));
    }
}

#[trace("TC-491", "FR-111-AC-5", "FR-131-AC-2")]
#[test]
fn definition_digest_is_the_exact_artifact_byte_digest() {
    let bytes = b"exact definition bytes\n";
    let definition = definition(
        "fixed",
        "1",
        DefinitionRole::Runtime,
        BTreeSet::new(),
        BTreeSet::new(),
        bytes,
    );
    assert_eq!(definition.exact_bytes(), bytes);
    assert_eq!(
        definition.exact().digest().digest().to_string(),
        "sha256:8a942381b82e9165c44d9b427a128d53d1241c2353eb9fbb9bfe0c445b10ce72"
    );
}

#[trace(
    "TC-491",
    "FR-111-AC-1",
    "FR-131-AC-1",
    "FR-131-AC-2",
    "FR-131-AC-3",
    "FR-339-AC-3"
)]
#[test]
fn roots_covering_the_facets_and_capabilities_link() {
    let definitions = complete_definitions();
    let linked = link_complete(&definitions);
    assert_eq!(linked.definitions().len(), definitions.len());
    assert_eq!(linked.bundle().capabilities().len(), 176);
    assert_eq!(linked.bundle().facets().len(), Facet::all().len());
    assert_eq!(linked.limits(), PackageLimits::default());
    for definition in &definitions {
        assert!(linked.definitions().contains_key(definition.exact()));
    }
}

#[trace("TC-491", "FR-111-AC-2", "FR-131-AC-2", "FR-131-AC-3")]
#[test]
fn roots_the_catalog_does_not_hold_exactly_refuse_naming_their_index() {
    let definitions = complete_definitions();

    // Unknown: the catalog lacks the root's identity (root 0).
    let unknown = link_bundle(
        &roots(&definitions),
        &DefinitionCatalog::new(definitions[1..].to_vec()).unwrap(),
        PackageLimits::default(),
    )
    .unwrap_err();
    assert_eq!(unknown.code, Code::UnknownProfile);
    assert_eq!(unknown.cause_tag, ResolutionCause::UnsupportedSelection);
    assert_eq!(unknown.root, Some(0));
    assert!(matches!(unknown.cause, PackageError::MissingDefinition(_)));
    assert_catalogued(&unknown);

    // Another version of the root's identity.
    let mut stale_definitions = definitions.clone();
    stale_definitions[1] = definition(
        definitions[1].exact().identity(),
        "2",
        DefinitionRole::ValueModelExpression,
        BTreeSet::new(),
        BTreeSet::new(),
        b"stale value definition artifact",
    );
    let stale = link_bundle(
        &roots(&definitions),
        &DefinitionCatalog::new(stale_definitions).unwrap(),
        PackageLimits::default(),
    )
    .unwrap_err();
    assert_eq!(stale.code, Code::StaleDependency);
    assert_eq!(stale.cause_tag, ResolutionCause::RevisionMismatch);
    assert_eq!(stale.root, Some(1));
    assert_catalogued(&stale);

    // The same version with other bytes.
    let mut rebytes = definitions.clone();
    rebytes[2] = definition(
        definitions[2].exact().identity(),
        definitions[2].exact().version(),
        DefinitionRole::Temporal,
        BTreeSet::new(),
        BTreeSet::new(),
        b"same version, other definition bytes",
    );
    let digest_only = link_bundle(
        &roots(&definitions),
        &DefinitionCatalog::new(rebytes).unwrap(),
        PackageLimits::default(),
    )
    .unwrap_err();
    assert_eq!(digest_only.code, Code::StaleDependency);
    assert_eq!(digest_only.cause_tag, ResolutionCause::ByteDigestMismatch);
    assert_eq!(digest_only.root, Some(2));
    assert_catalogued(&digest_only);
}

#[trace("TC-491", "FR-111-AC-2", "FR-131-AC-2")]
#[test]
fn a_dependency_edge_to_an_absent_definition_refuses_missing_selection() {
    let mut definitions = complete_definitions();
    let absent = definition(
        "quire.absent.complete/v1",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::new(),
        BTreeSet::new(),
        b"absent from the catalog",
    );
    definitions[3] = definition(
        definitions[3].exact().identity(),
        "1",
        DefinitionRole::Observation,
        BTreeSet::from([absent.exact().clone()]),
        BTreeSet::new(),
        b"observation root with a missing dependency",
    );
    let missing = link_roots(&definitions, 9, PackageLimits::default()).unwrap_err();
    assert_eq!(missing.code, Code::MissingImport);
    assert!(matches!(missing.cause, PackageError::MissingDefinition(_)));
    assert_eq!(missing.cause_tag, ResolutionCause::MissingSelection);
    assert_eq!(missing.root, Some(3));
    assert_catalogued(&missing);
}

#[trace("TC-491", "FR-111-AC-3", "FR-131-AC-2")]
#[test]
fn one_identity_selected_at_two_exact_selections_refuses_naming_both() {
    let left = definition(
        "acme.transitive",
        "2",
        DefinitionRole::MethodPlan,
        BTreeSet::new(),
        BTreeSet::new(),
        b"earlier transitive definition version two",
    );
    let right = definition(
        "acme.transitive",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::new(),
        BTreeSet::new(),
        b"later transitive definition version one",
    );
    let mut definitions = complete_definitions();
    definitions[2] = definition(
        definitions[2].exact().identity(),
        "1",
        DefinitionRole::Temporal,
        BTreeSet::from([left.exact().clone()]),
        BTreeSet::new(),
        b"temporal root selecting earlier version two",
    );
    definitions[3] = definition(
        definitions[3].exact().identity(),
        "1",
        DefinitionRole::Observation,
        BTreeSet::from([right.exact().clone()]),
        BTreeSet::new(),
        b"observation root selecting later version one",
    );
    definitions.extend([left.clone(), right.clone()]);

    // A root and a reached definition.
    let conflict = link_roots(&definitions, 9, PackageLimits::default()).unwrap_err();
    assert_eq!(conflict.code, Code::AmbiguousDeclaration);
    assert_eq!(conflict.cause_tag, ResolutionCause::ConflictingAuthority);
    assert_eq!(conflict.root, Some(3));
    let PackageError::ConflictingDefinitions(details) = conflict.cause else {
        panic!("expected a typed transitive definition conflict")
    };
    assert_eq!(details.first, *left.exact());
    assert_eq!(details.first_root, 2);
    assert_eq!(details.second, *right.exact());

    // Two roots.
    let both = link_bundle(
        &[left.exact().clone(), right.exact().clone()],
        &DefinitionCatalog::new(definitions).unwrap(),
        PackageLimits::default(),
    )
    .unwrap_err();
    assert_eq!(both.code, Code::AmbiguousDeclaration);
    assert_eq!(both.cause_tag, ResolutionCause::ConflictingAuthority);
    assert_eq!(both.root, Some(1));
    let PackageError::ConflictingDefinitions(details) = &both.cause else {
        panic!("expected a typed two-root conflict")
    };
    assert_eq!(details.first, *left.exact());
    assert_eq!(details.first_root, 0);
    assert_eq!(details.second, *right.exact());
    assert_catalogued(&both);
}

#[trace("TC-491", "FR-111-AC-3", "FR-131-AC-2")]
#[test]
fn a_dependency_cycle_refuses_naming_the_cycle_in_path_order() {
    let a_ref = definition(
        "acme.cycle.a",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::new(),
        BTreeSet::new(),
        b"cycle a",
    )
    .exact()
    .clone();
    let b_ref = definition(
        "acme.cycle.b",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::new(),
        BTreeSet::new(),
        b"cycle b",
    )
    .exact()
    .clone();
    // A definition's reference digests its bytes only, so the two can name
    // each other.
    let a = definition(
        "acme.cycle.a",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::from([b_ref.clone()]),
        BTreeSet::new(),
        b"cycle a",
    );
    let b = definition(
        "acme.cycle.b",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::from([a_ref.clone()]),
        BTreeSet::new(),
        b"cycle b",
    );
    let refusal = link_bundle(
        std::slice::from_ref(&a_ref),
        &DefinitionCatalog::new(vec![a, b]).unwrap(),
        PackageLimits::default(),
    )
    .unwrap_err();
    assert_eq!(refusal.code, Code::InvalidPackage);
    assert_eq!(refusal.cause_tag, ResolutionCause::DefinitionCycle);
    assert_eq!(refusal.root, Some(0));
    assert_eq!(
        refusal.cause,
        PackageError::DefinitionCycle(vec![a_ref.clone(), b_ref, a_ref])
    );
    assert_catalogued(&refusal);
}

#[trace(
    "TC-491",
    "FR-111-AC-1",
    "FR-111-AC-4",
    "FR-131-AC-1",
    "FR-131-AC-2",
    "FR-131-AC-3",
    "FR-339-AC-3"
)]
#[test]
fn a_missing_facet_or_capability_refuses_and_no_backend_input_exists() {
    let definitions = complete_definitions();
    let linked = link_complete(&definitions);
    assert_eq!(linked.bundle().capabilities().len(), 176);
    assert_eq!(CapabilityId::complete_inventory().len(), 176);

    // The bundle's identity is fixed by its definitions: a broader or a
    // narrower known-backend set only decides `validate_known_capabilities`.
    let identity = linked.bundle().identity();
    assert_eq!(
        linked.bundle().validate_known_capabilities(&inventory()),
        Ok(())
    );
    let narrower: BTreeSet<_> = inventory().into_iter().skip(1).collect();
    assert!(matches!(
        linked.bundle().validate_known_capabilities(&narrower),
        Err(PackageError::UnknownCapability(_))
    ));
    assert_eq!(link_complete(&definitions).bundle().identity(), identity);

    let incomplete: Vec<_> = definitions
        .iter()
        .filter(|definition| definition.exact().identity() != "quire.observation.complete/v1")
        .cloned()
        .collect();
    let refusal = link_roots(&incomplete, incomplete.len(), PackageLimits::default()).unwrap_err();
    assert_eq!(
        refusal.cause,
        PackageError::MissingFacet(Facet::Observation)
    );
    assert_eq!(
        (refusal.code, refusal.cause_tag),
        (Code::InvalidPackage, ResolutionCause::FeatureSetMismatch)
    );
    assert_eq!(refusal.root, None);
    assert_catalogued(&refusal);

    let mut fewer = definitions.clone();
    let dropped = inventory().into_iter().next().unwrap();
    fewer[0] = reinterpret_definition(
        &definitions[0],
        DefinitionRole::Source,
        BTreeSet::new(),
        inventory().into_iter().skip(1).collect(),
    );
    let refusal = link_roots(&fewer, 9, PackageLimits::default()).unwrap_err();
    assert_eq!(refusal.cause, PackageError::MissingCapability(dropped));
    assert_eq!(
        (refusal.code, refusal.cause_tag),
        (
            Code::UnknownRequiredFeature,
            ResolutionCause::UnsupportedFeature
        )
    );
    assert_catalogued(&refusal);
}

#[trace("TC-491", "FR-111-AC-5", "FR-131-AC-2")]
#[test]
fn semantic_identity_binds_typed_definition_interpretation() {
    let definitions = complete_definitions();
    let baseline = link_complete(&definitions).bundle().identity();

    let mut role_variant = definitions.clone();
    role_variant[0] = reinterpret_definition(
        &definitions[0],
        DefinitionRole::ValueModelExpression,
        BTreeSet::new(),
        inventory(),
    );
    role_variant[1] = reinterpret_definition(
        &definitions[1],
        DefinitionRole::Source,
        BTreeSet::new(),
        BTreeSet::new(),
    );

    let mut dependency_variant = definitions.clone();
    dependency_variant[0] = reinterpret_definition(
        &definitions[0],
        DefinitionRole::Source,
        BTreeSet::from([definitions[1].exact().clone()]),
        inventory(),
    );

    let mut capability_variant = definitions.clone();
    capability_variant[0] = reinterpret_definition(
        &definitions[0],
        DefinitionRole::Source,
        BTreeSet::new(),
        BTreeSet::new(),
    );
    capability_variant[1] = reinterpret_definition(
        &definitions[1],
        DefinitionRole::ValueModelExpression,
        BTreeSet::new(),
        inventory(),
    );

    let mut bytes_variant = definitions.clone();
    bytes_variant[8] = definition(
        definitions[8].exact().identity(),
        "1",
        DefinitionRole::ToolingEvidence,
        BTreeSet::new(),
        BTreeSet::new(),
        b"other tooling bytes",
    );

    for variant in [
        &role_variant,
        &dependency_variant,
        &capability_variant,
        &bytes_variant,
    ] {
        assert_ne!(link_complete(variant).bundle().identity(), baseline);
    }
    for variant in [&role_variant, &dependency_variant, &capability_variant] {
        assert!(definitions
            .iter()
            .zip(variant)
            .all(|(left, right)| left.exact() == right.exact()));
    }
}

/// The FR-131 half of the former compiled-model test: a definition root that
/// names a compiled model's identity is a root the catalog lacks, so it
/// refuses `unsupported-selection`; model selections resolve only through I1
/// (FR-056), never here.
#[trace("TC-491", "FR-111-AC-2", "FR-131-AC-1", "FR-131-AC-2")]
#[test]
fn a_root_naming_a_compiled_models_identity_is_not_a_definition_root() {
    let definitions = complete_definitions();
    let model_named = definition(
        "acme.compiled-model/reading",
        "1",
        DefinitionRole::ValueModelExpression,
        BTreeSet::new(),
        BTreeSet::new(),
        br#"{"kind":"compiled-model","exports":["Reading"]}"#,
    );
    let mut selected = roots(&definitions);
    selected.push(model_named.exact().clone());
    let refusal = link_bundle(
        &selected,
        &DefinitionCatalog::new(definitions).unwrap(),
        PackageLimits::default(),
    )
    .unwrap_err();
    assert_eq!(refusal.code, Code::UnknownProfile);
    assert_eq!(refusal.cause_tag, ResolutionCause::UnsupportedSelection);
    assert_eq!(refusal.root, Some(9));
    assert_catalogued(&refusal);
}

#[trace("TC-491", "FR-111-AC-6", "FR-131-AC-2")]
#[test]
fn catalog_and_link_resource_limits_have_exact_boundaries() {
    let definitions = complete_definitions();
    let artifact_bytes: usize = definitions
        .iter()
        .map(|definition| definition.exact_bytes().len())
        .sum();
    let exact = PackageLimits {
        definitions: definitions.len(),
        dependency_edges: 0,
        depth: 1,
        artifact_bytes,
        single_artifact_bytes: artifact_bytes,
    };
    assert!(DefinitionCatalog::with_limits(definitions.clone(), exact).is_ok());
    assert_eq!(
        DefinitionCatalog::with_limits(
            definitions.clone(),
            PackageLimits {
                definitions: definitions.len() - 1,
                ..exact
            },
        )
        .unwrap_err(),
        PackageError::ResourceLimit {
            kind: PackageLimitKind::Definitions,
            limit: definitions.len() - 1,
            actual: None,
        }
    );
    assert_eq!(
        DefinitionCatalog::with_limits(
            definitions.clone(),
            PackageLimits {
                artifact_bytes: artifact_bytes - 1,
                ..exact
            },
        )
        .unwrap_err(),
        PackageError::ResourceLimit {
            kind: PackageLimitKind::ArtifactBytes,
            limit: artifact_bytes - 1,
            actual: None,
        }
    );

    let catalog = DefinitionCatalog::new(definitions.clone()).unwrap();
    let selected = roots(&definitions);
    let link_exact = PackageLimits {
        artifact_bytes,
        ..PackageLimits::default()
    };
    assert!(link_bundle(&selected, &catalog, link_exact).is_ok());
    let artifact_refusal = link_bundle(
        &selected,
        &catalog,
        PackageLimits {
            artifact_bytes: artifact_bytes - 1,
            ..link_exact
        },
    )
    .unwrap_err();
    assert_eq!(artifact_refusal.code, Code::ResourceExhausted);
    assert_eq!(
        artifact_refusal.cause,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::ArtifactBytes,
            limit: artifact_bytes - 1,
            actual: None,
        }
    );
    assert_eq!(
        artifact_refusal.cause_tag,
        ResolutionCause::InsufficientNextCharge
    );
    assert_catalogued(&artifact_refusal);

    let refusal = link_bundle(
        &selected,
        &catalog,
        PackageLimits {
            definitions: 1,
            ..PackageLimits::default()
        },
    )
    .unwrap_err();
    assert_eq!(refusal.code, Code::ResourceExhausted);
    assert_eq!(
        refusal.cause,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::Definitions,
            limit: 1,
            actual: None,
        }
    );
}

/// `PackageLimits::bounded()` no longer clamps a caller-supplied
/// `definitions` ceiling down to [`PackageLimits::default`] (ADR-011 §7.3;
/// NFR-001 "an implementation ceiling is not a domain bound"): a catalog
/// with more definitions than the default (`MAX_SELECTED_DEFINITIONS`, 4096)
/// admits under a caller-raised ceiling that the default itself refuses.
#[trace("TC-491", "FR-111-AC-6", "FR-131-AC-2")]
#[test]
fn a_caller_raised_definitions_ceiling_admits_a_catalog_the_default_refuses() {
    let over_default = MAX_SELECTED_DEFINITIONS + 1;
    let definitions: Vec<Definition> = (0..over_default)
        .map(|i| {
            definition(
                &format!("acme.bulk.{i}"),
                "1",
                DefinitionRole::MethodPlan,
                BTreeSet::new(),
                BTreeSet::new(),
                format!("definition body {i}").as_bytes(),
            )
        })
        .collect();

    assert!(
        matches!(
            DefinitionCatalog::with_limits(definitions.clone(), PackageLimits::default()),
            Err(PackageError::ResourceLimit {
                kind: PackageLimitKind::Definitions,
                ..
            })
        ),
        "the default definitions ceiling must refuse a catalog past it"
    );

    let raised = PackageLimits {
        definitions: over_default,
        ..PackageLimits::default()
    };
    assert!(
        DefinitionCatalog::with_limits(definitions, raised).is_ok(),
        "a caller-raised definitions ceiling must admit what the default refuses"
    );
}

/// Reaching a *caller-raised* `definitions` ceiling (not just
/// the default) still refuses, naming the limit kind
/// ([`PackageLimitKind::Definitions`]) and the caller's own configured
/// bound.
#[trace("TC-491", "FR-111-AC-6", "FR-131-AC-2")]
#[test]
fn reaching_a_caller_raised_definitions_ceiling_refuses_naming_the_kind_and_bound() {
    let raised = PackageLimits {
        definitions: MAX_SELECTED_DEFINITIONS + 10,
        ..PackageLimits::default()
    };
    let definitions: Vec<Definition> = (0..=raised.definitions)
        .map(|i| {
            definition(
                &format!("acme.bulk.{i}"),
                "1",
                DefinitionRole::MethodPlan,
                BTreeSet::new(),
                BTreeSet::new(),
                format!("definition body {i}").as_bytes(),
            )
        })
        .collect();

    let refusal = DefinitionCatalog::with_limits(definitions, raised).unwrap_err();
    assert_eq!(
        refusal,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::Definitions,
            limit: raised.definitions,
            actual: None,
        },
        "must name the raised ceiling actually in force, not the original default"
    );
    assert_eq!(
        refusal.to_string(),
        format!(
            "package resource limit exceeded: definitions (limit {})",
            raised.definitions
        )
    );
}

#[trace("TC-491", "FR-111-AC-6", "FR-131-AC-1", "FR-131-AC-2")]
#[test]
fn dependency_edge_and_depth_limits_admit_exactly_and_refuse_one_below() {
    let leaf = definition(
        "acme.chain.leaf",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::new(),
        BTreeSet::new(),
        b"chain leaf",
    );
    let middle = definition(
        "acme.chain.middle",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::from([leaf.exact().clone()]),
        BTreeSet::new(),
        b"chain middle",
    );
    let mut definitions = complete_definitions();
    definitions[0] = definition(
        definitions[0].exact().identity(),
        "1",
        DefinitionRole::Source,
        BTreeSet::from([middle.exact().clone()]),
        inventory(),
        b"source root with depth-two dependency chain",
    );
    definitions.extend([middle, leaf]);
    let exact = PackageLimits {
        definitions: 11,
        dependency_edges: 2,
        depth: 3,
        ..PackageLimits::default()
    };
    let linked = link_roots(&definitions, 9, exact).unwrap();
    assert_eq!(
        linked.limits(),
        exact,
        "a link records exactly the limits it was checked against"
    );
    assert_eq!(linked.definitions().len(), 11);
    for name in ["acme.chain.middle", "acme.chain.leaf"] {
        assert!(linked
            .definitions()
            .keys()
            .any(|exact| exact.identity() == name));
    }
    // The closure holds 11 definitions: one fewer refuses, at the definition
    // that would be the 11th (root 8).
    let over = link_roots(
        &definitions,
        9,
        PackageLimits {
            definitions: 10,
            ..exact
        },
    )
    .unwrap_err();
    assert_eq!(over.code, Code::ResourceExhausted);
    assert_eq!(
        over.cause,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::Definitions,
            limit: 10,
            actual: None,
        }
    );
    assert_eq!(over.cause_tag, ResolutionCause::InsufficientNextCharge);
    assert_eq!(over.root, Some(8));
    let raised = PackageLimits {
        definitions: PackageLimits::default().definitions + 1,
        dependency_edges: PackageLimits::default().dependency_edges + 1,
        depth: PackageLimits::default().depth + 1,
        artifact_bytes: PackageLimits::default().artifact_bytes + 1,
        single_artifact_bytes: PackageLimits::default().single_artifact_bytes + 1,
    };
    assert_eq!(
        link_roots(&definitions, 9, raised).unwrap().limits(),
        raised,
        "a caller-raised limit is recorded as given, never clamped to the default"
    );
    let edges = link_roots(
        &definitions,
        9,
        PackageLimits {
            dependency_edges: 1,
            ..exact
        },
    )
    .unwrap_err();
    assert_eq!(
        edges.cause,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::DependencyEdges,
            limit: 1,
            actual: None,
        }
    );
    assert_eq!(edges.code, Code::ResourceExhausted);
    assert_eq!(edges.root, Some(0));
    let depth_refusal =
        link_roots(&definitions, 9, PackageLimits { depth: 2, ..exact }).unwrap_err();
    assert_eq!(
        depth_refusal.cause,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::Depth,
            limit: 2,
            actual: Some(3),
        }
    );
    // The graph's own depth ceiling is the one `PackageLimitKind`
    // that maps cleanly onto the catalog's `stage_limit_exceeded`, unlike
    // `Definitions`/`DependencyEdges`/`ArtifactBytes` above, which stay
    // `resource_exhausted`.
    assert_eq!(depth_refusal.code, Code::StageLimitExceeded);
    assert_eq!(
        depth_refusal.cause_tag,
        ResolutionCause::StageLimit(LimitKind::NestingDepth)
    );
    assert_catalogued(&depth_refusal);
}

/// Every `ResolutionCause` paired with the layer-1 `CompleteCause` of the
/// same name. The macro builds an exhaustive `match` over the listed
/// variants with no `_` arm, so a variant left out of the list fails to
/// compile (E0004), and the list is the one the test iterates.
macro_rules! resolution_causes {
    ($($variant:ident),+ $(,)?) => {{
        fn covered(cause: ResolutionCause) {
            match cause {
                $(ResolutionCause::$variant => {})+
                // The one payload-bearing variant, covered
                // separately below rather than through this macro's
                // bare-identifier list.
                ResolutionCause::StageLimit(_) => {}
            }
        }
        vec![$({
            covered(ResolutionCause::$variant);
            (ResolutionCause::$variant, CompleteCause::$variant)
        }),+]
    }};
}

/// Layer 3's resolution cause tags are a subset of layer 1's complete cause
/// vocabulary: each spells the same catalog tag and is admitted for exactly
/// the same codes as the `qsl_cst::CompleteCause` of the same name.
#[trace("TC-491", "FR-111-AC-7", "FR-131-AC-2")]
#[test]
fn resolution_causes_match_the_complete_cause_catalog() {
    let pairs = resolution_causes![
        UnsupportedSelection,
        RevisionMismatch,
        ByteDigestMismatch,
        InsufficientNextCharge,
        MissingSelection,
        ConflictingAuthority,
        DuplicateMember,
        InvalidValue,
        DefinitionCycle,
        FeatureSetMismatch,
        UnknownFeature,
        UnsupportedFeature,
    ];
    for (resolution, complete) in pairs {
        assert_eq!(resolution.as_str(), complete.as_str());
        for code in Code::all() {
            assert_eq!(
                resolution.is_cause_of(*code),
                complete.is_cause_of(*code),
                "{resolution:?} and {complete:?} disagree on {code:?}"
            );
        }
    }
    // `StageLimit`'s payload carries the kind, so it is checked
    // directly rather than through the macro's bare-identifier list, over
    // every kind the catalog admits (not only `NestingDepth`, the one the
    // package graph itself produces).
    for kind in [
        LimitKind::InputBytes,
        LimitKind::NestingDepth,
        LimitKind::NodeCount,
        LimitKind::WorkBudget,
    ] {
        let resolution = ResolutionCause::StageLimit(kind);
        let complete = CompleteCause::StageLimit(kind);
        assert_eq!(resolution.as_str(), complete.as_str());
        for code in Code::all() {
            assert_eq!(
                resolution.is_cause_of(*code),
                complete.is_cause_of(*code),
                "{resolution:?} and {complete:?} disagree on {code:?}"
            );
        }
    }
}

/// A catalog holding one exact definition twice refuses at construction
/// (`invalid_package`/`duplicate-member`).
#[trace("TC-491", "FR-111-AC-7", "FR-131-AC-2")]
#[test]
fn a_catalog_holding_one_exact_definition_twice_refuses() {
    let definitions = complete_definitions();
    let error =
        DefinitionCatalog::new(vec![definitions[0].clone(), definitions[0].clone()]).unwrap_err();
    assert_eq!(error, PackageError::DuplicateDefinition);
}

/// The size of one definition is a caller limit
/// (`PackageLimits::single_artifact_bytes`), not a fixed 1 MiB ceiling refused
/// as invalid bytes. An artifact one byte past the default is built, refused
/// at catalog admission naming the kind and the bound, and admitted once the
/// caller raises the limit to its size.
#[trace("TC-491", "FR-111-AC-6", "FR-131-AC-2")]
#[test]
fn single_artifact_bytes_is_a_caller_limit_naming_its_bound() {
    let default = PackageLimits::default();
    let oversized = vec![b'x'; default.single_artifact_bytes + 1];
    let large = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        "acme.large",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::new(),
        BTreeSet::new(),
        &oversized,
    )
    .expect("size is a catalog limit, not an invalid-bytes refusal");
    let expected = PackageError::ResourceLimit {
        kind: PackageLimitKind::SingleArtifactBytes,
        limit: default.single_artifact_bytes,
        actual: None,
    };
    assert_eq!(
        DefinitionCatalog::with_limits(vec![large.clone()], default).unwrap_err(),
        expected
    );
    assert_eq!(
        expected.to_string(),
        format!(
            "package resource limit exceeded: single_artifact_bytes (limit {})",
            default.single_artifact_bytes
        )
    );

    let raised = PackageLimits {
        single_artifact_bytes: oversized.len(),
        ..default
    };
    assert!(DefinitionCatalog::with_limits(vec![large], raised).is_ok());
}

/// A link checks `single_artifact_bytes` under its own limits,
/// whatever limits the catalog was built under.
#[trace("TC-491", "FR-111-AC-6", "FR-131-AC-2")]
#[test]
fn a_link_enforces_its_own_single_artifact_bytes() {
    let default = PackageLimits::default();
    let mut definitions = complete_definitions();
    definitions[0] = definition(
        definitions[0].exact().identity(),
        "1",
        DefinitionRole::Source,
        BTreeSet::new(),
        inventory(),
        &vec![b'x'; default.single_artifact_bytes + 1],
    );
    let raised = PackageLimits {
        single_artifact_bytes: default.single_artifact_bytes + 1,
        ..default
    };
    let catalog = DefinitionCatalog::with_limits(definitions.clone(), raised).unwrap();
    let selected = roots(&definitions);

    let refusal = link_bundle(&selected, &catalog, default).unwrap_err();
    assert_eq!(refusal.code, Code::ResourceExhausted);
    assert_eq!(
        refusal.cause,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::SingleArtifactBytes,
            limit: default.single_artifact_bytes,
            actual: None,
        }
    );
    assert_catalogued(&refusal);
    assert_eq!(
        link_bundle(&selected, &catalog, raised).unwrap().limits(),
        raised
    );
}

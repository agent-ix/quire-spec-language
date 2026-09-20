// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeSet;

use crate::complete::{
    self, resolve_source_package, CapabilityId, CompleteCause, CompleteCode, Definition,
    DefinitionCatalog, DefinitionDigest, DefinitionRef, DefinitionRole, Facet, ModelArtifact,
    ModelCatalog, PackageError, PackageLimits, ProfileCatalog, ReaderAuthority, SourceDigest,
};
use crate::{Limits, SourceIdentity};
use ix_trace_rs::trace;

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

const READER_AUTHORITY: ReaderAuthority = ReaderAuthority::crate_owned();

fn definition_ref(identity: &str, digit: char) -> DefinitionRef {
    DefinitionRef::new(
        identity,
        "1",
        DefinitionDigest::parse(&format!("sha256:{}", digit.to_string().repeat(64))).unwrap(),
    )
    .unwrap()
}

fn complete_definitions() -> Vec<Definition> {
    COMPLETE_DEFINITION_ROLES
        .into_iter()
        .enumerate()
        .map(|(index, (identity, role))| {
            let exact_bytes = format!("fixture complete definition artifact {index}\n");
            Definition::from_exact_bytes(
                &READER_AUTHORITY,
                identity,
                "1",
                role,
                BTreeSet::new(),
                if index == 0 {
                    CapabilityId::complete_inventory().into_iter().collect()
                } else {
                    BTreeSet::new()
                },
                exact_bytes.as_bytes(),
            )
            .unwrap()
        })
        .collect()
}

fn reinterpret_definition(
    definition: &Definition,
    role: DefinitionRole,
    dependencies: BTreeSet<DefinitionRef>,
    capabilities: BTreeSet<CapabilityId>,
) -> Definition {
    Definition::from_exact_bytes(
        &READER_AUTHORITY,
        definition.exact().identity(),
        definition.exact().version(),
        role,
        dependencies,
        capabilities,
        definition.exact_bytes(),
    )
    .unwrap()
}

fn compiled_model() -> ModelArtifact {
    ModelArtifact::from_exact_bytes(
        &READER_AUTHORITY,
        "acme.compiled-model/reading",
        "1",
        br#"{"kind":"compiled-model","exports":["Reading"]}"#,
    )
    .unwrap()
}

fn resolved_source(definitions: &[Definition], model: &ModelArtifact) -> complete::ParsedSource {
    let profiles = definitions
        .iter()
        .enumerate()
        .map(|(index, definition)| {
            format!(
                "profile P{index} = \"{}\" version \"{}\" digest \"{}\";\n",
                definition.exact().identity(),
                definition.exact().version(),
                definition.exact().digest().digest()
            )
        })
        .collect::<String>();
    let source = format!(
        concat!(
            "language \"ix:native\" edition \"1-draft\";\n{profiles}",
            "import \"{}\" version \"{}\" digest \"{}\" as Base;\n",
            "model M = \"{}\" version \"{}\" digest \"{}\";\n",
            "record R {{ datum: Integer; }}\n",
            "record S {{ datum: Integer; }}"
        ),
        definitions[0].exact().identity(),
        definitions[0].exact().version(),
        definitions[0].exact().digest().digest(),
        model.exact().identity(),
        model.exact().version(),
        model.exact().digest().digest(),
        profiles = profiles,
    );
    complete::parse(
        SourceIdentity {
            identity: "test:resolved-package".into(),
            revision: "r1".into(),
        },
        "resolved.native",
        source.as_bytes(),
        Limits::default(),
    )
    .unwrap()
}

fn resolved_package(definitions: &[Definition]) -> complete::ResolvedSourcePackage {
    let model = compiled_model();
    resolve_source_package(
        std::sync::Arc::new(resolved_source(definitions, &model)),
        &DefinitionCatalog::new(definitions.to_vec()).unwrap(),
        &ModelCatalog::new(vec![model]).unwrap(),
        PackageLimits::default(),
    )
    .unwrap()
}

fn resolve_fixture(
    parsed: std::sync::Arc<complete::ParsedSource>,
    catalog: &DefinitionCatalog,
    model: &ModelArtifact,
) -> Result<complete::ResolvedSourcePackage, complete::PackageRefusal> {
    resolve_source_package(
        parsed,
        catalog,
        &ModelCatalog::new(vec![model.clone()]).unwrap(),
        PackageLimits::default(),
    )
}

fn assert_refusal_authority(refusal: &complete::PackageRefusal, parsed: &complete::ParsedSource) {
    assert_eq!(refusal.authority.identity, *parsed.source().identity());
    assert_eq!(refusal.authority.path, parsed.source().path());
    assert_eq!(refusal.authority.digest.digest(), parsed.source().digest());
    assert!(
        refusal.cause_tag.is_cause_of(refusal.code),
        "{:?} is not a cause of {:?}",
        refusal.cause_tag,
        refusal.code
    );
}

#[trace("TC-180", "FR-131-AC-1", "FR-131-AC-2", "FR-131-AC-3", "FR-339-AC-3")]
#[test]
fn exact_multi_profile_graph_closes_and_preserves_source_authority() {
    let definitions = complete_definitions();
    let model = compiled_model();
    let catalog = DefinitionCatalog::new(definitions.clone()).unwrap();
    let parsed = std::sync::Arc::new(resolved_source(&definitions, &model));
    assert_eq!(parsed.selections().profiles.len(), definitions.len());
    let package = resolve_fixture(parsed.clone(), &catalog, &model).unwrap();
    assert_eq!(package.definitions().len(), definitions.len());
    assert_eq!(package.models().len(), 1);
    assert_eq!(package.bundle().capabilities().len(), 176);
    // Source authority is `ResolvedSourcePackage`'s own accessor (FR-131,
    // FR-339): SEAM-5's now-deleted structural lowering (ADR-011 §7.3 M-3a;
    // FR-067-AC-5, AC-6) only ever cloned it from here, so asserting on
    // `package.authority()` directly keeps this coverage unchanged.
    assert_eq!(package.authority().identity, *parsed.source().identity());
    assert_eq!(package.authority().path, parsed.source().path());
    assert_eq!(
        package.authority().digest.digest(),
        parsed.source().digest()
    );
    assert_eq!(
        SourceDigest::of(b"abc").digest().to_string(),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[trace("TC-180", "FR-131-AC-2", "FR-131-AC-3")]
#[test]
fn exact_profile_resolution_refuses_unknown_stale_and_missing_dependencies() {
    let definitions = complete_definitions();
    let model = compiled_model();
    let parsed = std::sync::Arc::new(resolved_source(&definitions, &model));
    let unknown_catalog = DefinitionCatalog::new(definitions[1..].to_vec()).unwrap();
    let unknown = resolve_fixture(parsed.clone(), &unknown_catalog, &model).unwrap_err();
    assert_eq!(unknown.code, CompleteCode::UnknownProfile);
    assert_eq!(unknown.cause_tag, CompleteCause::UnsupportedSelection);

    let broken_text = format!(
        "{}\nrecord Broken {{ value: Integer }}",
        parsed.source().text()
    );
    let broken = std::sync::Arc::new(
        complete::parse(
            SourceIdentity {
                identity: "test:broken-source".into(),
                revision: "r1".into(),
            },
            "broken.native",
            broken_text.as_bytes(),
            Limits::default(),
        )
        .unwrap(),
    );
    let inadmissible = resolve_fixture(broken.clone(), &unknown_catalog, &model).unwrap_err();
    assert_eq!(
        (inadmissible.code, inadmissible.cause_tag),
        (CompleteCode::InvalidSyntax, CompleteCause::UnexpectedToken)
    );
    assert_refusal_authority(&inadmissible, broken.as_ref());
    assert_eq!(unknown.span, parsed.selections().profiles[0].identity_span);
    assert_refusal_authority(&unknown, parsed.as_ref());

    let mut stale_definitions = definitions.clone();
    stale_definitions[0] = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        definitions[0].exact().identity(),
        "2",
        DefinitionRole::Source,
        BTreeSet::new(),
        CapabilityId::complete_inventory().into_iter().collect(),
        b"stale source definition artifact",
    )
    .unwrap();
    let stale = resolve_fixture(
        parsed.clone(),
        &DefinitionCatalog::new(stale_definitions).unwrap(),
        &model,
    )
    .unwrap_err();
    assert_eq!(stale.code, CompleteCode::StaleDependency);
    assert_eq!(stale.cause_tag, CompleteCause::RevisionMismatch);
    assert_refusal_authority(&stale, parsed.as_ref());

    let mut rebytes = definitions.clone();
    rebytes[0] = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        definitions[0].exact().identity(),
        definitions[0].exact().version(),
        DefinitionRole::Source,
        BTreeSet::new(),
        CapabilityId::complete_inventory().into_iter().collect(),
        b"same version, other source definition bytes",
    )
    .unwrap();
    let digest_only = resolve_fixture(
        parsed.clone(),
        &DefinitionCatalog::new(rebytes).unwrap(),
        &model,
    )
    .unwrap_err();
    assert_eq!(digest_only.code, CompleteCode::StaleDependency);
    assert_eq!(digest_only.cause_tag, CompleteCause::ByteDigestMismatch);
    assert_refusal_authority(&digest_only, parsed.as_ref());

    let stale_source = resolved_source(&definitions, &model)
        .source()
        .text()
        .replacen("version \"1\"", "version \"stale\"", 1);
    let stale_parsed = complete::parse_with_catalog(
        SourceIdentity {
            identity: "test:stale-profile-parse".into(),
            revision: "r1".into(),
        },
        "stale.native",
        stale_source.as_bytes(),
        &ProfileCatalog::new(
            definitions
                .iter()
                .map(|definition| definition.exact().clone())
                .collect(),
        )
        .unwrap(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        stale_parsed.diagnostics()[0].code,
        CompleteCode::StaleDependency
    );

    let mut missing_dependency = definitions.clone();
    missing_dependency[0] = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        definitions[0].exact().identity(),
        "1",
        DefinitionRole::Source,
        BTreeSet::from([definition_ref("quire.absent.complete/v1", 'a')]),
        CapabilityId::complete_inventory().into_iter().collect(),
        b"source definition with missing dependency",
    )
    .unwrap();
    let missing_source = std::sync::Arc::new(resolved_source(&missing_dependency, &model));
    let missing = resolve_fixture(
        missing_source.clone(),
        &DefinitionCatalog::new(missing_dependency).unwrap(),
        &model,
    )
    .unwrap_err();
    assert_eq!(missing.code, CompleteCode::MissingImport);
    assert!(matches!(missing.cause, PackageError::MissingDefinition(_)));
    assert_eq!(missing.cause_tag, CompleteCause::MissingSelection);
    assert_refusal_authority(&missing, missing_source.as_ref());
}

#[trace("TC-180", "FR-131-AC-2")]
#[test]
fn dependency_closure_refuses_logical_conflicts_and_duplicate_aliases() {
    let left = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        "acme.transitive",
        "2",
        DefinitionRole::MethodPlan,
        BTreeSet::new(),
        BTreeSet::new(),
        b"earlier transitive definition version two",
    )
    .unwrap();
    let right = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        "acme.transitive",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::new(),
        BTreeSet::new(),
        b"later transitive definition version one",
    )
    .unwrap();
    let mut definitions = complete_definitions();
    definitions[2] = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        definitions[2].exact().identity(),
        "1",
        DefinitionRole::Temporal,
        BTreeSet::from([left.exact().clone()]),
        BTreeSet::new(),
        b"temporal root selecting earlier version two",
    )
    .unwrap();
    definitions[3] = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        definitions[3].exact().identity(),
        "1",
        DefinitionRole::Observation,
        BTreeSet::from([right.exact().clone()]),
        BTreeSet::new(),
        b"observation root selecting later version one",
    )
    .unwrap();
    let model = compiled_model();
    let parsed = std::sync::Arc::new(resolved_source(&definitions, &model));
    let first_span = parsed.selections().profiles[2].identity_span;
    let second_span = parsed.selections().profiles[3].identity_span;
    definitions.extend([left, right]);
    let conflict = resolve_fixture(
        parsed,
        &DefinitionCatalog::new(definitions.clone()).unwrap(),
        &model,
    )
    .unwrap_err();
    assert_eq!(conflict.code, CompleteCode::AmbiguousDeclaration);
    assert_eq!(conflict.cause_tag, CompleteCause::ConflictingAuthority);
    assert_eq!(conflict.span, second_span);
    let PackageError::ConflictingDefinitions(details) = conflict.cause else {
        panic!("expected a typed transitive definition conflict")
    };
    assert_eq!(details.first_span, first_span);

    let duplicate_source = resolved_source(&definitions[..9], &model)
        .source()
        .text()
        .replacen("P1", "P0", 1);
    let parsed = complete::parse(
        SourceIdentity {
            identity: "test:duplicate-alias".into(),
            revision: "r1".into(),
        },
        "duplicate.native",
        duplicate_source.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let duplicate = resolve_fixture(
        std::sync::Arc::new(parsed),
        &DefinitionCatalog::new(definitions[..9].to_vec()).unwrap(),
        &model,
    )
    .unwrap_err();
    assert_eq!(duplicate.code, CompleteCode::AmbiguousDeclaration);
    assert_eq!(duplicate.cause_tag, CompleteCause::AmbiguousName);
    assert!(matches!(
        duplicate.cause,
        PackageError::DuplicateAlias { .. }
    ));
}

#[trace("TC-180", "FR-131-AC-1", "FR-131-AC-2", "FR-131-AC-3", "FR-339-AC-3")]
#[test]
fn complete_bundle_is_closed_and_backend_authority_free() {
    let definitions = complete_definitions();
    let model = compiled_model();
    let linked = resolved_package(&definitions).bundle().clone();
    assert_eq!(linked.capabilities().len(), 176);
    assert_eq!(CapabilityId::complete_inventory().len(), 176);
    assert_eq!(linked.facets().len(), Facet::all().len());

    let incomplete: Vec<_> = definitions
        .iter()
        .filter(|definition| definition.exact().identity() != "quire.observation.complete/v1")
        .cloned()
        .collect();
    let refusal = resolve_fixture(
        std::sync::Arc::new(resolved_source(&incomplete, &model)),
        &DefinitionCatalog::new(incomplete).unwrap(),
        &model,
    )
    .unwrap_err();
    assert_eq!(
        refusal.cause,
        PackageError::MissingFacet(Facet::Observation)
    );
    assert_eq!(
        (refusal.code, refusal.cause_tag),
        (
            CompleteCode::InvalidPackage,
            CompleteCause::FeatureSetMismatch
        )
    );

    type ParseFn = fn(
        SourceIdentity,
        String,
        &[u8],
        Limits,
    ) -> Result<complete::ParsedSource, Box<complete::CompleteDiagnostic>>;
    let _: ParseFn = complete::parse;

    // These public types are the complete parser/resolver authority surface;
    // neither admits backend installation or capability state as an input.
    let resolver: fn(
        std::sync::Arc<complete::ParsedSource>,
        &DefinitionCatalog,
        &ModelCatalog,
        PackageLimits,
    ) -> Result<complete::ResolvedSourcePackage, complete::PackageRefusal> = resolve_source_package;
    let source = resolved_source(&definitions, &model);
    let admitted = resolver(
        std::sync::Arc::new(source.clone()),
        &DefinitionCatalog::new(definitions).unwrap(),
        &ModelCatalog::new(vec![model]).unwrap(),
        PackageLimits::default(),
    )
    .unwrap();
    assert_eq!(source.cst().render(), admitted.parsed().cst().render());
}

#[trace("TC-180", "FR-131-AC-1", "FR-131-AC-2")]
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

#[trace("TC-180", "FR-131-AC-2")]
#[test]
fn definition_digest_is_the_exact_artifact_byte_digest() {
    let bytes = b"exact definition bytes\n";
    let definition = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        "fixed",
        "1",
        DefinitionRole::Runtime,
        BTreeSet::new(),
        BTreeSet::new(),
        bytes,
    )
    .unwrap();
    assert_eq!(definition.exact_bytes(), bytes);
    assert_eq!(
        definition.exact().digest().digest().to_string(),
        "sha256:8a942381b82e9165c44d9b427a128d53d1241c2353eb9fbb9bfe0c445b10ce72"
    );
}

#[trace("TC-180", "FR-131-AC-2")]
#[test]
fn semantic_identity_binds_typed_definition_interpretation() {
    let definitions = complete_definitions();
    let baseline = resolved_package(&definitions).bundle().identity();

    let mut role_variant = definitions.clone();
    role_variant[0] = reinterpret_definition(
        &definitions[0],
        DefinitionRole::ValueModelExpression,
        BTreeSet::new(),
        CapabilityId::complete_inventory().into_iter().collect(),
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
        CapabilityId::complete_inventory().into_iter().collect(),
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
        CapabilityId::complete_inventory().into_iter().collect(),
    );

    for variant in [&role_variant, &dependency_variant, &capability_variant] {
        assert!(definitions
            .iter()
            .zip(variant)
            .all(|(left, right)| left.exact() == right.exact()));
        assert_ne!(resolved_package(variant).bundle().identity(), baseline);
    }
}

#[trace("TC-180", "FR-131-AC-1", "FR-131-AC-2")]
#[test]
fn compiled_models_cannot_substitute_for_or_be_substituted_by_definitions() {
    let mut definitions = complete_definitions();
    let model = compiled_model();
    definitions.push(
        Definition::from_exact_bytes(
            &READER_AUTHORITY,
            model.exact().identity(),
            model.exact().version(),
            DefinitionRole::ValueModelExpression,
            BTreeSet::new(),
            BTreeSet::new(),
            model.exact_bytes(),
        )
        .unwrap(),
    );
    let parsed = std::sync::Arc::new(resolved_source(&definitions[..9], &model));
    let refusal = resolve_source_package(
        parsed.clone(),
        &DefinitionCatalog::new(definitions).unwrap(),
        &ModelCatalog::default(),
        PackageLimits::default(),
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::InvalidModelBinding);
    assert!(matches!(refusal.cause, PackageError::MissingModel(_)));
    assert_eq!(refusal.cause_tag, CompleteCause::WrongModelSelection);

    let stale_model = ModelArtifact::from_exact_bytes(
        &READER_AUTHORITY,
        model.exact().identity(),
        "2",
        b"changed compiled model document",
    )
    .unwrap();
    let stale = resolve_source_package(
        parsed,
        &DefinitionCatalog::new(complete_definitions()).unwrap(),
        &ModelCatalog::new(vec![stale_model]).unwrap(),
        PackageLimits::default(),
    )
    .unwrap_err();
    assert!(matches!(stale.cause, PackageError::StaleModel(_)));
    assert!(stale.cause_tag.is_cause_of(stale.code));
}

#[trace("TC-180", "FR-131-AC-2")]
#[test]
fn catalog_and_resolution_resource_limits_have_exact_boundaries() {
    let definitions = complete_definitions();
    let artifact_bytes = definitions
        .iter()
        .map(|definition| definition.exact_bytes().len())
        .sum();
    let exact = PackageLimits {
        definitions: definitions.len(),
        dependency_edges: 0,
        depth: 1,
        artifact_bytes,
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
        PackageError::ResourceLimit
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
        PackageError::ResourceLimit
    );

    let model = compiled_model();
    let parsed = std::sync::Arc::new(resolved_source(&definitions, &model));
    let catalog = DefinitionCatalog::new(definitions.clone()).unwrap();
    let models = ModelCatalog::new(vec![model.clone()]).unwrap();
    let resolved_artifact_bytes = artifact_bytes + model.exact_bytes().len();
    let resolution_exact = PackageLimits {
        artifact_bytes: resolved_artifact_bytes,
        ..PackageLimits::default()
    };
    assert!(resolve_source_package(parsed.clone(), &catalog, &models, resolution_exact).is_ok());
    let artifact_refusal = resolve_source_package(
        parsed.clone(),
        &catalog,
        &models,
        PackageLimits {
            artifact_bytes: resolved_artifact_bytes - 1,
            ..resolution_exact
        },
    )
    .unwrap_err();
    assert_eq!(artifact_refusal.code, CompleteCode::ResourceExhausted);
    assert_eq!(artifact_refusal.cause, PackageError::ResourceLimit);
    assert_eq!(
        artifact_refusal.cause_tag,
        CompleteCause::InsufficientNextCharge
    );
    assert_refusal_authority(&artifact_refusal, parsed.as_ref());

    let refusal = resolve_source_package(
        parsed,
        &catalog,
        &models,
        PackageLimits {
            definitions: 1,
            ..PackageLimits::default()
        },
    )
    .unwrap_err();
    assert_eq!(refusal.code, CompleteCode::ResourceExhausted);
    assert_eq!(refusal.cause, PackageError::ResourceLimit);
}

#[trace("TC-180", "FR-131-AC-1", "FR-131-AC-2")]
#[test]
fn dependency_edge_and_depth_limits_admit_exactly_and_refuse_one_below() {
    let leaf = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        "acme.chain.leaf",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::new(),
        BTreeSet::new(),
        b"chain leaf",
    )
    .unwrap();
    let middle = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        "acme.chain.middle",
        "1",
        DefinitionRole::MethodPlan,
        BTreeSet::from([leaf.exact().clone()]),
        BTreeSet::new(),
        b"chain middle",
    )
    .unwrap();
    let mut definitions = complete_definitions();
    definitions[0] = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        definitions[0].exact().identity(),
        "1",
        DefinitionRole::Source,
        BTreeSet::from([middle.exact().clone()]),
        CapabilityId::complete_inventory().into_iter().collect(),
        b"source root with depth-two dependency chain",
    )
    .unwrap();
    let model = compiled_model();
    let parsed = std::sync::Arc::new(resolved_source(&definitions, &model));
    definitions.extend([middle, leaf]);
    let catalog = DefinitionCatalog::new(definitions).unwrap();
    let models = ModelCatalog::new(vec![model]).unwrap();
    let exact = PackageLimits {
        definitions: 12,
        dependency_edges: 2,
        depth: 3,
        ..PackageLimits::default()
    };
    assert!(resolve_source_package(parsed.clone(), &catalog, &models, exact).is_ok());
    assert_eq!(
        resolve_source_package(
            parsed.clone(),
            &catalog,
            &models,
            PackageLimits {
                dependency_edges: 1,
                ..exact
            },
        )
        .unwrap_err()
        .cause,
        PackageError::ResourceLimit
    );
    assert_eq!(
        resolve_source_package(
            parsed,
            &catalog,
            &models,
            PackageLimits { depth: 2, ..exact },
        )
        .unwrap_err()
        .cause,
        PackageError::ResourceLimit
    );
}

#[trace("TC-180", "FR-131-AC-2")]
#[test]
fn selection_validation_locates_each_invalid_component_for_every_declaration_kind() {
    let valid_digest = format!("sha256:{}", "a".repeat(64));
    let parsed_digest = DefinitionDigest::parse(&valid_digest).unwrap();
    assert_eq!(
        DefinitionRef::new("", "1", parsed_digest).unwrap_err(),
        PackageError::InvalidDefinitionIdentity
    );
    assert_eq!(
        DefinitionRef::new("acme/definition", "", parsed_digest).unwrap_err(),
        PackageError::InvalidDefinitionVersion
    );

    for declaration_kind in ["profile", "import", "model"] {
        for invalid_component in ["identity", "version", "digest"] {
            let identity = if invalid_component == "identity" {
                ""
            } else {
                "acme/definition"
            };
            let version = if invalid_component == "version" {
                ""
            } else {
                "1"
            };
            let digest = if invalid_component == "digest" {
                "not-a-digest"
            } else {
                &valid_digest
            };
            let declaration = match declaration_kind {
                "profile" => format!(
                    "profile Complete = \"{identity}\" version \"{version}\" digest \"{digest}\";"
                ),
                "import" => format!(
                    "import \"{identity}\" version \"{version}\" digest \"{digest}\" as Base;"
                ),
                "model" => {
                    format!("model M = \"{identity}\" version \"{version}\" digest \"{digest}\";")
                }
                _ => unreachable!("closed declaration-kind test table"),
            };
            let valid_profile = (declaration_kind != "profile").then(|| {
                format!(
                    "profile Complete = \"acme/profile\" version \"1\" digest \"{valid_digest}\";\n"
                )
            });
            let source = format!(
                "language \"ix:native\" edition \"1-draft\";\n{}{declaration}\nrecord R {{ datum: Integer; }}",
                valid_profile.as_deref().unwrap_or_default(),
            );
            let invalid_value = match invalid_component {
                "identity" => identity,
                "version" => version,
                "digest" => digest,
                _ => unreachable!("closed invalid-component test table"),
            };
            let invalid_literal = format!("\"{invalid_value}\"");
            let expected_start = source.find(&invalid_literal).unwrap();
            let parsed = complete::parse(
                SourceIdentity {
                    identity: format!("test:{declaration_kind}-{invalid_component}"),
                    revision: "r1".into(),
                },
                "invalid-selection.native",
                source.as_bytes(),
                Limits::default(),
            )
            .unwrap();

            assert!(!parsed.is_admissible());
            assert_eq!(parsed.diagnostics().len(), 1);
            let diagnostic = &parsed.diagnostics()[0];
            assert_eq!(
                diagnostic.code,
                if invalid_component == "digest" {
                    CompleteCode::InvalidDigest
                } else {
                    CompleteCode::InvalidIdentifier
                }
            );
            assert_eq!(diagnostic.span.start.byte, expected_start);
            assert_eq!(
                diagnostic.span.end.byte,
                expected_start + invalid_literal.len()
            );
            assert!(diagnostic.message.contains(invalid_component));
        }
    }
}

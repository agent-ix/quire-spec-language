// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 package resolution over parsed source (TC-180), through the
//! shipped caller step `command::resolve_parsed_source`, which refuses an
//! inadmissible parse and hands layer-3 `complete::resolve_source_package`
//! the authority and selections of one admitted parse (QSL-181). Gated on
//! `test-support` for `ReaderAuthority::fixture`.
use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_cst::{parse, CompleteCause, CompleteDiagnostic, Limits, ParsedSource};
use qsl_foundation::selection::{
    DefinitionDigest, DefinitionRef, ProfileCatalog, MAX_SELECTED_DEFINITIONS,
};
use qsl_foundation::{Code, SourceIdentity};
use qsl_semantics::complete::{
    resolve_source_package, CapabilityId, Definition, DefinitionCatalog, DefinitionRole, Facet,
    ModelArtifact, ModelCatalog, PackageError, PackageLimitKind, PackageLimits, PackageRefusal,
    ReaderAuthority, ResolutionCause, ResolvedSourcePackage, SourceAuthority, SourceDigest,
};
use quire_spec_language::command::{resolve_parsed_source, SourcePackageRefusal};
use quire_spec_language::complete;

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

const READER_AUTHORITY: ReaderAuthority = ReaderAuthority::fixture();

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

fn parse_fixture(identity: &str, path: &str, source: &str) -> ParsedSource {
    parse(
        SourceIdentity {
            identity: identity.into(),
            revision: "r1".into(),
        },
        path,
        source.as_bytes(),
        Limits::default(),
    )
    .unwrap()
}

fn resolved_source(definitions: &[Definition], model: &ModelArtifact) -> ParsedSource {
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
    parse_fixture("test:resolved-package", "resolved.native", &source)
}

/// The shipped caller step over an admissible fixture: its outcome is a
/// resolved package or a layer-3 resolution refusal.
fn resolve_parsed(
    parsed: &ParsedSource,
    catalog: &DefinitionCatalog,
    models: &ModelCatalog,
    limits: PackageLimits,
) -> Result<ResolvedSourcePackage, PackageRefusal> {
    match resolve_parsed_source(parsed, catalog, models, limits) {
        Ok(package) => Ok(package),
        Err(SourcePackageRefusal::Resolution(refusal)) => Err(refusal),
        Err(refusal) => panic!("an admissible fixture was refused before resolution: {refusal}"),
    }
}

fn resolved_package(definitions: &[Definition]) -> ResolvedSourcePackage {
    let model = compiled_model();
    resolve_parsed(
        &resolved_source(definitions, &model),
        &DefinitionCatalog::new(definitions.to_vec()).unwrap(),
        &ModelCatalog::new(vec![model]).unwrap(),
        PackageLimits::default(),
    )
    .unwrap()
}

fn resolve_fixture(
    parsed: &ParsedSource,
    catalog: &DefinitionCatalog,
    model: &ModelArtifact,
) -> Result<ResolvedSourcePackage, PackageRefusal> {
    resolve_parsed(
        parsed,
        catalog,
        &ModelCatalog::new(vec![model.clone()]).unwrap(),
        PackageLimits::default(),
    )
}

fn assert_refusal_authority(refusal: &PackageRefusal, parsed: &ParsedSource) {
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
    let parsed = resolved_source(&definitions, &model);
    assert_eq!(parsed.selections().profiles.len(), definitions.len());
    let package = resolve_fixture(&parsed, &catalog, &model).unwrap();
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
    let parsed = resolved_source(&definitions, &model);
    let unknown_catalog = DefinitionCatalog::new(definitions[1..].to_vec()).unwrap();
    let unknown = resolve_fixture(&parsed, &unknown_catalog, &model).unwrap_err();
    assert_eq!(unknown.code, Code::UnknownProfile);
    assert_eq!(unknown.cause_tag, ResolutionCause::UnsupportedSelection);

    // An inadmissible parse is refused before resolution, with the parse's
    // own first diagnostic and the refused source's own authority.
    let broken_text = format!(
        "{}\nrecord Broken {{ value: Integer }}",
        parsed.source().text()
    );
    let broken = parse_fixture("test:broken-source", "broken.native", &broken_text);
    let refused = resolve_parsed_source(
        &broken,
        &unknown_catalog,
        &ModelCatalog::new(vec![model.clone()]).unwrap(),
        PackageLimits::default(),
    )
    .unwrap_err();
    let SourcePackageRefusal::Inadmissible {
        authority,
        diagnostic,
    } = refused
    else {
        panic!("expected an inadmissible-source refusal, got {refused:?}")
    };
    assert_eq!(
        (diagnostic.code, diagnostic.cause),
        (Code::InvalidSyntax, CompleteCause::UnexpectedToken)
    );
    assert_eq!(*diagnostic, broken.diagnostics()[0]);
    assert_eq!(*authority, SourceAuthority::of(broken.source()));
    assert_eq!(authority.identity.identity, "test:broken-source");
    assert_eq!(authority.path, "broken.native");
    assert_eq!(unknown.span, parsed.selections().profiles[0].identity_span);
    assert_refusal_authority(&unknown, &parsed);

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
        &parsed,
        &DefinitionCatalog::new(stale_definitions).unwrap(),
        &model,
    )
    .unwrap_err();
    assert_eq!(stale.code, Code::StaleDependency);
    assert_eq!(stale.cause_tag, ResolutionCause::RevisionMismatch);
    assert_refusal_authority(&stale, &parsed);

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
    let digest_only =
        resolve_fixture(&parsed, &DefinitionCatalog::new(rebytes).unwrap(), &model).unwrap_err();
    assert_eq!(digest_only.code, Code::StaleDependency);
    assert_eq!(digest_only.cause_tag, ResolutionCause::ByteDigestMismatch);
    assert_refusal_authority(&digest_only, &parsed);

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
    assert_eq!(stale_parsed.diagnostics()[0].code, Code::StaleDependency);

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
    let missing_source = resolved_source(&missing_dependency, &model);
    let missing = resolve_fixture(
        &missing_source,
        &DefinitionCatalog::new(missing_dependency).unwrap(),
        &model,
    )
    .unwrap_err();
    assert_eq!(missing.code, Code::MissingImport);
    assert!(matches!(missing.cause, PackageError::MissingDefinition(_)));
    assert_eq!(missing.cause_tag, ResolutionCause::MissingSelection);
    assert_refusal_authority(&missing, &missing_source);
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
    let parsed = resolved_source(&definitions, &model);
    let first_span = parsed.selections().profiles[2].identity_span;
    let second_span = parsed.selections().profiles[3].identity_span;
    definitions.extend([left, right]);
    let conflict = resolve_fixture(
        &parsed,
        &DefinitionCatalog::new(definitions.clone()).unwrap(),
        &model,
    )
    .unwrap_err();
    assert_eq!(conflict.code, Code::AmbiguousDeclaration);
    assert_eq!(conflict.cause_tag, ResolutionCause::ConflictingAuthority);
    assert_eq!(conflict.span, second_span);
    let PackageError::ConflictingDefinitions(details) = conflict.cause else {
        panic!("expected a typed transitive definition conflict")
    };
    assert_eq!(details.first_span, first_span);

    let duplicate_source = resolved_source(&definitions[..9], &model)
        .source()
        .text()
        .replacen("P1", "P0", 1);
    let parsed = parse_fixture(
        "test:duplicate-alias",
        "duplicate.native",
        &duplicate_source,
    );
    let duplicate = resolve_fixture(
        &parsed,
        &DefinitionCatalog::new(definitions[..9].to_vec()).unwrap(),
        &model,
    )
    .unwrap_err();
    assert_eq!(duplicate.code, Code::AmbiguousDeclaration);
    assert_eq!(duplicate.cause_tag, ResolutionCause::AmbiguousName);
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
        &resolved_source(&incomplete, &model),
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
        (Code::InvalidPackage, ResolutionCause::FeatureSetMismatch)
    );

    type ParseFn =
        fn(SourceIdentity, String, &[u8], Limits) -> Result<ParsedSource, Box<CompleteDiagnostic>>;
    let _: ParseFn = parse;

    // These public types are the complete parser/resolver authority surface;
    // neither admits backend installation or capability state as an input.
    let _: fn(
        SourceAuthority,
        &qsl_foundation::selection::SourceSelections,
        &DefinitionCatalog,
        &ModelCatalog,
        PackageLimits,
    ) -> Result<ResolvedSourcePackage, PackageRefusal> = resolve_source_package;
    let source = resolved_source(&definitions, &model);
    let caller_step: fn(
        &ParsedSource,
        &DefinitionCatalog,
        &ModelCatalog,
        PackageLimits,
    ) -> Result<ResolvedSourcePackage, SourcePackageRefusal> = resolve_parsed_source;
    let admitted = caller_step(
        &source,
        &DefinitionCatalog::new(definitions).unwrap(),
        &ModelCatalog::new(vec![model]).unwrap(),
        PackageLimits::default(),
    )
    .unwrap();
    // The caller step derives the authority and the selections from the one
    // parse it is given: the package names this source and holds exactly
    // this source's selected definitions and models.
    assert_eq!(admitted.authority().identity, *source.source().identity());
    assert_eq!(admitted.authority().path, source.source().path());
    assert_eq!(
        admitted.authority().digest.digest(),
        source.source().digest()
    );
    let selected_definitions: BTreeSet<DefinitionRef> = source
        .selections()
        .profiles
        .iter()
        .map(|selection| selection.definition.clone())
        .chain(
            source
                .selections()
                .imports
                .iter()
                .map(|selection| selection.definition.clone()),
        )
        .collect();
    assert_eq!(
        admitted
            .definitions()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        selected_definitions
    );
    assert_eq!(
        admitted.models().keys().cloned().collect::<Vec<_>>(),
        source
            .selections()
            .models
            .iter()
            .map(|selection| selection.model.clone())
            .collect::<Vec<_>>()
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
    let parsed = resolved_source(&definitions[..9], &model);
    let refusal = resolve_parsed(
        &parsed,
        &DefinitionCatalog::new(definitions).unwrap(),
        &ModelCatalog::default(),
        PackageLimits::default(),
    )
    .unwrap_err();
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert!(matches!(refusal.cause, PackageError::MissingModel(_)));
    assert_eq!(refusal.cause_tag, ResolutionCause::WrongModelSelection);

    let stale_model = ModelArtifact::from_exact_bytes(
        &READER_AUTHORITY,
        model.exact().identity(),
        "2",
        b"changed compiled model document",
    )
    .unwrap();
    let stale = resolve_parsed(
        &parsed,
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
        PackageError::ResourceLimit {
            kind: PackageLimitKind::Definitions,
            limit: definitions.len() - 1,
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
        }
    );

    let model = compiled_model();
    let parsed = resolved_source(&definitions, &model);
    let catalog = DefinitionCatalog::new(definitions.clone()).unwrap();
    let models = ModelCatalog::new(vec![model.clone()]).unwrap();
    let resolved_artifact_bytes = artifact_bytes + model.exact_bytes().len();
    let resolution_exact = PackageLimits {
        artifact_bytes: resolved_artifact_bytes,
        ..PackageLimits::default()
    };
    assert!(resolve_parsed(&parsed, &catalog, &models, resolution_exact).is_ok());
    let artifact_refusal = resolve_parsed(
        &parsed,
        &catalog,
        &models,
        PackageLimits {
            artifact_bytes: resolved_artifact_bytes - 1,
            ..resolution_exact
        },
    )
    .unwrap_err();
    assert_eq!(artifact_refusal.code, Code::ResourceExhausted);
    assert_eq!(
        artifact_refusal.cause,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::ArtifactBytes,
            limit: resolved_artifact_bytes - 1,
        }
    );
    assert_eq!(
        artifact_refusal.cause_tag,
        ResolutionCause::InsufficientNextCharge
    );
    assert_refusal_authority(&artifact_refusal, &parsed);

    let refusal = resolve_parsed(
        &parsed,
        &catalog,
        &models,
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
        }
    );
}

/// QSL-199: `PackageLimits::bounded()` no longer clamps a caller-supplied
/// `definitions` ceiling down to [`PackageLimits::default`] (ADR-011 §7.3;
/// NFR-001 "an implementation ceiling is not a domain bound"): a catalog
/// with more definitions than the default (`MAX_SELECTED_DEFINITIONS`, 4096)
/// admits under a caller-raised ceiling that the default itself refuses.
#[trace("TC-180", "FR-131-AC-2")]
#[test]
fn a_caller_raised_definitions_ceiling_admits_a_catalog_the_default_refuses() {
    let over_default = MAX_SELECTED_DEFINITIONS + 1;
    let definitions: Vec<Definition> = (0..over_default)
        .map(|i| {
            Definition::from_exact_bytes(
                &READER_AUTHORITY,
                format!("acme.bulk.{i}"),
                "1",
                DefinitionRole::MethodPlan,
                BTreeSet::new(),
                BTreeSet::new(),
                format!("definition body {i}").as_bytes(),
            )
            .unwrap()
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

/// QSL-199 AC-3: reaching a *caller-raised* `definitions` ceiling (not just
/// the default) still refuses, naming the limit kind
/// ([`PackageLimitKind::Definitions`]) and the caller's own configured
/// bound.
#[trace("TC-180", "FR-131-AC-2")]
#[test]
fn reaching_a_caller_raised_definitions_ceiling_refuses_naming_the_kind_and_bound() {
    let raised = PackageLimits {
        definitions: MAX_SELECTED_DEFINITIONS + 10,
        ..PackageLimits::default()
    };
    let definitions: Vec<Definition> = (0..=raised.definitions)
        .map(|i| {
            Definition::from_exact_bytes(
                &READER_AUTHORITY,
                format!("acme.bulk.{i}"),
                "1",
                DefinitionRole::MethodPlan,
                BTreeSet::new(),
                BTreeSet::new(),
                format!("definition body {i}").as_bytes(),
            )
            .unwrap()
        })
        .collect();

    let refusal = DefinitionCatalog::with_limits(definitions, raised).unwrap_err();
    assert_eq!(
        refusal,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::Definitions,
            limit: raised.definitions,
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
    let parsed = resolved_source(&definitions, &model);
    definitions.extend([middle, leaf]);
    let catalog = DefinitionCatalog::new(definitions).unwrap();
    let models = ModelCatalog::new(vec![model]).unwrap();
    let exact = PackageLimits {
        definitions: 12,
        dependency_edges: 2,
        depth: 3,
        ..PackageLimits::default()
    };
    assert_eq!(
        resolve_parsed(&parsed, &catalog, &models, exact)
            .unwrap()
            .effective_limits(),
        exact,
        "a resolution records exactly the limits it was checked against"
    );
    let raised = PackageLimits {
        definitions: PackageLimits::default().definitions + 1,
        dependency_edges: PackageLimits::default().dependency_edges + 1,
        depth: PackageLimits::default().depth + 1,
        artifact_bytes: PackageLimits::default().artifact_bytes + 1,
    };
    assert_eq!(
        resolve_parsed(&parsed, &catalog, &models, raised)
            .unwrap()
            .effective_limits(),
        raised,
        "a caller-raised limit is recorded as given, never clamped to the default"
    );
    assert_eq!(
        resolve_parsed(
            &parsed,
            &catalog,
            &models,
            PackageLimits {
                dependency_edges: 1,
                ..exact
            },
        )
        .unwrap_err()
        .cause,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::DependencyEdges,
            limit: 1,
        }
    );
    assert_eq!(
        resolve_parsed(
            &parsed,
            &catalog,
            &models,
            PackageLimits { depth: 2, ..exact },
        )
        .unwrap_err()
        .cause,
        PackageError::ResourceLimit {
            kind: PackageLimitKind::Depth,
            limit: 2,
        }
    );
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
#[trace("TC-180", "FR-131-AC-2")]
#[test]
fn resolution_causes_match_the_complete_cause_catalog() {
    let pairs = resolution_causes![
        UnsupportedSelection,
        RevisionMismatch,
        ByteDigestMismatch,
        InsufficientNextCharge,
        MissingSelection,
        AmbiguousName,
        ConflictingAuthority,
        DuplicateMember,
        InvalidValue,
        DefinitionCycle,
        FeatureSetMismatch,
        UnknownFeature,
        UnsupportedFeature,
        WrongModelSelection,
        ConflictingBinding,
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
}

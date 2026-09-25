// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-148 (FR-036-AC-9, FR-056-AC-8): the composed model linker binds native
//! model references to the declarations of a domain package admitted from
//! filament-core-data#173's architecture bundle.
//!
//! Every package here runs the real path: the bundle goes through
//! `qsl_semantics::model::intake::lift_document` (FCD's `lift`, which calls
//! quire-rs extraction with the bundle's real module manifests), the lifted
//! bytes through FR-154 admission and `read_records`, and the records through
//! FR-152 classification. No semantic context or model JSON is hand-built.
//!
//! The bundle is read at test time from the pinned
//! `agent-ix-extraction-frontend` checkout and copied into a temporary
//! directory, where each test edits it as TC-148 directs. The unedited
//! bundle does not admit whole at this pin, for reasons owned outside this
//! linker (asserted by `qsl-semantics`' `model_intake` tests):
//!
//! - `Pump`/`Sys`/`Tank.id` are typed `UUID`, which is not a QSL native value
//!   type (FCD fixture `spec/model/Pump.md:21`, `Sys.md:20`, `Tank.md:20`;
//!   PLAT-836).
//! - `Flow2`'s inline relationship is identified
//!   `ix://agent-ix/architecture/relationship/Flow2-specializes-Flow`, not
//!   `<owner>/<name>` (FCD `crates/extraction-frontend/src/identity.rs:223`).
//!
//! [`architecture_bundle`] therefore types the three identity fields
//! `Boolean` and drops `Flow2`. `Count`, a record value type, and every
//! part, port, connection and allocation of the bundle are kept as authored.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use ix_trace_rs::trace;
use qsl_foundation::{Source, SourceIdentity};
use qsl_semantics::model::accounting::{Meter, ModelNormalizationLimits};
use qsl_semantics::model::admitted::{AdmittedPackage, DeclarationKind};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, PortDirection,
};
use qsl_semantics::model::index::ModelIndex;
use qsl_semantics::model::intake::{admit, lift_document, read_records};
use qsl_semantics::model::key::{DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use qsl_semantics::model::systems::{
    check_connection, classify, ConnectionCheckOutcome, ConnectionOutcome,
};
use quire_spec_language::linking::composed::binding_work::{Limits as BindingLimits, Work};
use quire_spec_language::linking::composed::models::{
    bind_models, ImportRefusal, ModelBindings, ModelErrorKind, ModelInput, ModelTarget,
};
use quire_spec_language::linking::composed::{
    admit_namespace, ExpectedSource, SourceInventory, SyntaxNamespace, WorkLimits,
};
use quire_spec_language::Limits;
use sha2::{Digest, Sha256};

const PACKAGE: &str = "agent-ix/architecture";

/// `crates/extraction-frontend/fixtures` in the pinned FCD checkout, found
/// through `cargo metadata` so the revision is named once, in `Cargo.lock`.
fn fcd_fixtures_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let output = Command::new(env!("CARGO"))
            .args(["metadata", "--format-version", "1", "--locked", "--offline"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("run Cargo's locked offline metadata resolver");
        assert!(
            output.status.success(),
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("decode cargo metadata");
        let manifest = metadata["packages"]
            .as_array()
            .expect("metadata packages array")
            .iter()
            .find(|package| package["name"] == "agent-ix-extraction-frontend")
            .expect("agent-ix-extraction-frontend is a locked dependency")["manifest_path"]
            .as_str()
            .expect("manifest_path is a string")
            .to_owned();
        Path::new(&manifest)
            .parent()
            .expect("manifest has a parent directory")
            .join("fixtures")
    })
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("create bundle directory");
    for entry in std::fs::read_dir(from).expect("read bundle directory") {
        let entry = entry.expect("bundle entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("entry type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy bundle file");
        }
    }
}

pub(crate) fn edit(path: &Path, from: &str, to: &str) {
    let text = std::fs::read_to_string(path).expect("read bundle file");
    assert!(
        text.contains(from),
        "{}: {from:?} not found",
        path.display()
    );
    std::fs::write(path, text.replace(from, to)).expect("write bundle file");
}

/// The architecture bundle, copied and edited as the module docs describe,
/// then `extra` applied to the copy.
pub(crate) fn architecture_bundle(extra: impl FnOnce(&Path)) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let source = fcd_fixtures_dir().join("architecture");
    copy_tree(&source.join("spec"), &dir.path().join("spec"));
    std::fs::copy(source.join("modules.json"), dir.path().join("modules.json"))
        .expect("copy modules.json");
    let spec = dir.path().join("spec");
    for entity in ["model/Pump.md", "model/Sys.md", "model/Tank.md"] {
        edit(&spec.join(entity), "| id | UUID |", "| id | Boolean |");
    }
    std::fs::remove_file(spec.join("systems/Flow2.md")).expect("drop Flow2");
    extra(dir.path());
    dir
}

/// Lifts `bundle` through FCD's real pipeline and returns its bytes (the
/// lift emits RFC 8785 bytes).
fn lift(bundle: &Path) -> Vec<u8> {
    let fixtures = fcd_fixtures_dir();
    let modules = [
        "modules/spec-objects-business",
        "modules/edge-vocabulary",
        "modules/spec-objects-architecture",
    ]
    .map(|root| fixtures.join(root));
    lift_document(bundle, &modules).expect("the edited bundle lifts")
}

/// FR-154 admission, `read_records` and FR-152 classification over the
/// lifted bundle, selected under its own identity, version and digest.
fn admitted(bundle: &Path) -> AdmittedPackage {
    admitted_with_bytes(bundle).0
}

/// [`admitted`], with the lifted document bytes the package was admitted from.
pub(crate) fn admitted_with_bytes(bundle: &Path) -> (AdmittedPackage, Vec<u8>) {
    admit_bytes(lift(bundle))
}

/// Lifts `bundle`, adds one FR-153 population declaration per `(name,
/// member types)` of `populations`, each `closed`, then admits it; returns
/// the package and its document bytes. With no population the lifted bytes
/// are admitted as they are.
///
/// The FCD frontend at this pin refuses a population `Members` table
/// (`agent-ix-extraction-frontend` `constructs.rs`, "a population `Members`
/// table", which no construct lowers), so each population is added to the
/// real lifted document: a `populations[]` node, under one `constructs[]`
/// entry for `spec-objects-business`' `population` construct with the
/// module version and manifest digest the lift recorded for that module.
/// The document is re-encoded as RFC 8785 bytes, so its raw-byte and
/// `sha256-jcs` digests agree as the lift's own do.
pub(crate) fn admitted_with_populations(
    bundle: &Path,
    populations: &[(&str, &[&str])],
) -> (AdmittedPackage, Vec<u8>) {
    admitted_with(bundle, &[], populations)
}

/// [`admitted_with_populations`], with one object type added per
/// `(subtype, supertype)` of `subtypes`: a copy of the lifted `Sys` type
/// node under the subtype's identity and a generated origin, with no field
/// of its own, the supertype as its only supertype and the supertype's `id`
/// as its identity field. A `specializes` edge in the bundle would lift to an
/// inline relationship FCD identifies outside `<owner>/<name>` (PLAT-1064,
/// see the module docs), so the subtype is added to the lifted document.
pub(crate) fn admitted_with(
    bundle: &Path,
    subtypes: &[(&str, &str)],
    populations: &[(&str, &[&str])],
) -> (AdmittedPackage, Vec<u8>) {
    let bytes = lift(bundle);
    if populations.is_empty() && subtypes.is_empty() {
        return admit_bytes(bytes);
    }
    let mut document: serde_json::Value = serde_json::from_slice(&bytes).expect("lifted JSON");
    let types = document["types"].as_array_mut().expect("types array");
    let template = types
        .iter()
        .find(|node| node["identity"] == format!("ix://{PACKAGE}/Sys"))
        .expect("the lift declares Sys")
        .clone();
    for (subtype, supertype) in subtypes {
        let mut node = template.clone();
        node["identity"] = format!("ix://{PACKAGE}/{subtype}").into();
        node["displayName"] = (*subtype).into();
        node["fields"] = serde_json::json!([]);
        // Its identity is the one it inherits.
        node["identityFields"] = serde_json::json!([format!("ix://{PACKAGE}/{supertype}/id")]);
        node["origin"] = serde_json::json!({"generated": {
            "generatorIdentity": format!("ix://{PACKAGE}/{subtype}"),
            "generatorVersion": "1.0.0",
            "inputIdentities": [format!("ix://{PACKAGE}/{subtype}")],
        }});
        node["supertypes"] = serde_json::json!([format!("ix://{PACKAGE}/{supertype}")]);
        types.push(node);
    }
    types.sort_by(|left, right| left["identity"].as_str().cmp(&right["identity"].as_str()));
    if populations.is_empty() {
        let limits = quire_canonical::Limits::new(1 << 24, quire_canonical::Limits::MAX_DEPTH)
            .expect("limits");
        return admit_bytes(quire_canonical::to_vec(&document, limits).expect("RFC 8785 bytes"));
    }
    let constructs = document["constructs"]
        .as_array_mut()
        .expect("constructs array");
    let business = constructs
        .iter()
        .find(|entry| entry["kind"]["module"] == "agent-ix/spec-objects-business")
        .expect("the lift records spec-objects-business")
        .clone();
    constructs.push(serde_json::json!({
        "kind": {"module": "agent-ix/spec-objects-business", "name": "population"},
        "moduleVersion": business["moduleVersion"],
        "manifestDigest": business["manifestDigest"],
        "construct": {
            "identity": "none",
            "shape": "record",
            "members": {},
            "meaning": "quire.meaning.model.population/v1",
        },
    }));
    document["populations"] = populations
        .iter()
        .map(|(name, members)| {
            let identity = format!("ix://{PACKAGE}/{name}");
            serde_json::json!({
                "identity": identity,
                "displayName": name,
                "kind": {"module": "agent-ix/spec-objects-business", "name": "population"},
                "members": members
                    .iter()
                    .map(|member| format!("ix://{PACKAGE}/{member}"))
                    .collect::<Vec<_>>(),
                "extent": "closed",
                "origin": {"generated": {
                    "generatorIdentity": identity,
                    "generatorVersion": "1.0.0",
                    "inputIdentities": [identity],
                }},
            })
        })
        .collect();
    let limits =
        quire_canonical::Limits::new(1 << 24, quire_canonical::Limits::MAX_DEPTH).expect("limits");
    admit_bytes(quire_canonical::to_vec(&document, limits).expect("RFC 8785 bytes"))
}

/// FR-154 admission, `read_records` and FR-152 classification over `bytes`,
/// RFC 8785 document bytes selected under their own identity, version and
/// digest.
fn admit_bytes(bytes: Vec<u8>) -> (AdmittedPackage, Vec<u8>) {
    let digest = Sha256::digest(&bytes).into();
    let document: serde_json::Value = serde_json::from_slice(&bytes).expect("lifted JSON");
    let offered = DomainPackageRef {
        identity: document["package"]["identity"]
            .as_str()
            .expect("package identity")
            .to_owned(),
        version: document["package"]["version"]
            .as_str()
            .expect("package version")
            .to_owned(),
        digest,
    };
    let (selection, document) = admit(
        &offered,
        SHA256_JCS_DIGEST_DOMAIN,
        &BTreeMap::from([(digest, bytes.clone())]),
    )
    .expect("the selection matches the lifted package");
    let records = read_records(&selection.identity, &document)
        .expect("every IR node of the edited bundle reads with no refusal");
    let package = AdmittedPackage::admit(
        DomainPackage::new(selection, records),
        &mut Meter::new(ModelNormalizationLimits::default()),
    )
    .expect("every declaration classifies with no refusal");
    (package, bytes)
}

fn key(artifact: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: format!("ix://{PACKAGE}/{artifact}"),
    }
}

pub(crate) fn hex(digest: &[u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

const DECLARATIONS: &str =
    "predicate FieldRule using S (): Boolean { M::Pump::id = M::Pump::id }\n\
protocol Plumbing using S over (view: M::Sys) on origin {\n\
    role Operator on M::Sys;\n\
    role Outlet on M::pump_out;\n\
    relationship Pipe = M::pipe;\n\
    run sequence Main { check Valid using S { true }; }\n\
    finish Closed as (closed: M::Sys) { true };\n\
}\n";

/// One native unit importing `M` under `digest`, with the field, Port (a
/// role) and Connection references of TC-148 step 3, plus `Unrelated`, which names no
/// model.
fn program(identity: &str, version: &str, digest: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"test:unresolved-definition\" version \"1\" digest \"unresolved\";\n\
         model M = \"{identity}\" version \"{version}\" digest \"{digest}\";\n\
         {DECLARATIONS}\
         predicate Unrelated using S (flag: Boolean): Boolean {{ flag }}\n"
    )
}

fn with_namespace(text: &str, test: impl FnOnce(&SyntaxNamespace)) {
    let source = Source::read(
        SourceIdentity {
            authority: "test".into(),
            identity: "plumbing".into(),
            revision_namespace: "test".into(),
            revision: "selected".into(),
        },
        "plumbing.native".to_owned(),
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .expect("source reads");
    let inventory = SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: vec![ExpectedSource {
            authority: "plumbing".into(),
            identity: source.identity().clone(),
            digest: source.digest(),
        }],
    };
    let sources = [source];
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(report.issues().is_empty(), "{:?}", report.issues());
    test(report.namespace().expect("namespace admitted"));
}

fn work() -> Work {
    Work::new(BindingLimits::default())
}

/// The domain declarations each named native declaration bound, as
/// `(key node, kind)`, and its refusal kinds.
fn resolved(
    bindings: &ModelBindings<'_>,
    namespace: &SyntaxNamespace,
    name: &str,
) -> (Vec<(String, DeclarationKind)>, Vec<ModelErrorKind>) {
    let report = bindings
        .resolve_declaration(namespace, namespace.lookup(name)[0], &mut work())
        .expect("declaration handle");
    assert!(report.complete, "{name}: model traversal finished");
    let declarations = report
        .occurrences
        .iter()
        .filter_map(|occurrence| match &occurrence.target {
            ModelTarget::Declaration(bound) => Some((bound.key().node.clone(), bound.kind())),
            ModelTarget::Type(_) | ModelTarget::Operation(_) => None,
        })
        .collect();
    let refusals = report
        .refusals
        .iter()
        .map(|error| error.kind.clone())
        .collect();
    (declarations, refusals)
}

/// TC-148 steps 1 and 2: the bundle's ports keep owning part, direction,
/// interface type and multiplicity, `pipe` is admitted under FR-152's
/// connection rule, and a port moved to another part keeps its key and
/// reports its new owner.
#[trace("TC-148", "FR-056-AC-8")]
#[test]
fn architecture_ports_and_connection_admit_through_the_real_lift() {
    let bundle = architecture_bundle(|_| {});
    let package = admitted(bundle.path());

    let port = |package: &AdmittedPackage, artifact: &str| {
        let declaration = package.declaration(artifact).expect("port declared");
        assert_eq!(declaration.kind, DeclarationKind::Port);
        assert_eq!(declaration.key, &key(artifact));
        let DomainPackageRecord::Endpoint(endpoint) = declaration.record else {
            panic!("{artifact}: a Port reads as an endpoint record")
        };
        endpoint.clone()
    };
    let pump_out = port(&package, "pump_out");
    assert_eq!(pump_out.owning_component, key("sys_pump"));
    assert_eq!(pump_out.direction, Some(PortDirection::Out));
    assert_eq!(pump_out.value_type, key("Flow"));
    assert_eq!(
        (pump_out.multiplicity.lower, pump_out.multiplicity.upper),
        (1, Some(1))
    );
    let tank_in = port(&package, "tank_in");
    assert_eq!(tank_in.owning_component, key("sys_tank"));
    assert_eq!(tank_in.direction, Some(PortDirection::In));
    assert_eq!(tank_in.value_type, key("Flow"));
    assert_eq!(
        (tank_in.multiplicity.lower, tank_in.multiplicity.upper),
        (1, Some(1))
    );

    // `Count` admits as authored: a record value type owning `value`.
    let count = package.declaration("Count").expect("Count declared");
    assert_eq!(count.kind, DeclarationKind::RecordValueType);
    let value = package.member(count.key, "value").expect("Count/value");
    assert_eq!(value.kind, DeclarationKind::Field);

    let pipe = package.declaration("pipe").expect("pipe declared");
    assert_eq!(pipe.kind, DeclarationKind::Connection);
    assert_eq!(pipe.key, &key("pipe"));
    let mut meter = Meter::new(ModelNormalizationLimits::default());
    let systems = classify(package.package(), &mut meter).expect("classifies");
    let outcome = check_connection(
        &ModelIndex::build(package.package().clone()),
        &systems,
        &key("pipe"),
        &mut meter,
    );
    assert_eq!(
        outcome,
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted)
    );

    let moved = architecture_bundle(|root| {
        edit(
            &root.join("spec/systems/pump_out.md"),
            "| sys_pump | out |",
            "| sys_tank | out |",
        );
    });
    let moved = admitted(moved.path());
    let moved_port = port(&moved, "pump_out");
    assert_eq!(moved_port.key, pump_out.key);
    assert_eq!(moved_port.owning_component, key("sys_tank"));
}

/// TC-148 step 3: a field, a Port and a Connection of the admitted package
/// bind to their exact declaration keys and kinds.
#[trace("TC-148", "FR-036-AC-9", "FR-056-AC-8")]
#[test]
fn native_references_bind_to_domain_declaration_keys_and_kinds() {
    let bundle = architecture_bundle(|_| {});
    let package = admitted(bundle.path());
    let selection = package.selection().clone();
    let inputs = [ModelInput::Domain(&package)];
    let text = program(
        &selection.identity,
        &selection.version,
        &format!("sha256-jcs:{}", hex(&selection.digest)),
    );
    with_namespace(&text, |namespace| {
        let bindings = bind_models(namespace, &inputs, &mut work());
        assert!(bindings.complete());
        assert_eq!(bindings.imports()[0].selection, Ok(0));

        let field = format!("ix://{PACKAGE}/Pump/id");
        assert_eq!(
            resolved(&bindings, namespace, "FieldRule"),
            (
                vec![
                    (field.clone(), DeclarationKind::Field),
                    (field, DeclarationKind::Field)
                ],
                vec![]
            )
        );
        let (plumbing, refusals) = resolved(&bindings, namespace, "Plumbing");
        assert!(refusals.is_empty(), "{refusals:?}");
        assert!(plumbing.contains(&(key("pump_out").node, DeclarationKind::Port)));
        assert!(plumbing.contains(&(key("pipe").node, DeclarationKind::Connection)));
        assert!(plumbing.contains(&(key("Sys").node, DeclarationKind::ObjectType)));
        assert_eq!(
            resolved(&bindings, namespace, "Unrelated"),
            (vec![], vec![])
        );

        // Each bound occurrence carries the selection it was admitted under.
        let report = bindings
            .resolve_declaration(namespace, namespace.lookup("Plumbing")[0], &mut work())
            .expect("declaration handle");
        assert!(!report.occurrences.is_empty());
        for occurrence in &report.occurrences {
            let ModelTarget::Declaration(bound) = &occurrence.target else {
                panic!("Plumbing binds only domain declarations")
            };
            assert_eq!(bound.selection(), &selection);
        }
    });
}

/// Refusals at the same reference sites: a name the package does not
/// declare, a member that does not exist, and a kind the site cannot take --
/// a member or a Port at a type site, a Connection as a role, a Port as a
/// relationship.
#[trace("TC-148", "FR-036-AC-9")]
#[test]
fn missing_and_wrong_kind_domain_references_refuse_located() {
    let bundle = architecture_bundle(|_| {});
    let package = admitted(bundle.path());
    let selection = package.selection().clone();
    let inputs = [ModelInput::Domain(&package)];
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"test:unresolved-definition\" version \"1\" digest \"unresolved\";\n\
         model M = \"{}\" version \"{}\" digest \"sha256-jcs:{}\";\n\
         predicate MissingType using S (port: M::drain_out): Boolean {{ true }}\n\
         predicate MissingField using S (): Boolean {{ M::Pump::speed = M::Pump::speed }}\n\
         predicate MemberAsType using S (): Boolean {{ M::pump_out::direction = M::pump_out::direction }}\n\
         predicate PortAsType using S (port: M::pump_out): Boolean {{ true }}\n\
         protocol ConnectionAsRole using S over (view: M::Sys) on origin {{\n\
             role Operator on M::pipe;\n\
             run sequence Main {{ check Valid using S {{ true }}; }}\n\
             finish Closed as (closed: M::Sys) {{ true }};\n\
         }}\n\
         protocol NotAConnection using S over (view: M::Sys) on origin {{\n\
             role Operator on M::Sys;\n\
             relationship Pipe = M::pump_out;\n\
             run sequence Main {{ check Valid using S {{ true }}; }}\n\
             finish Closed as (closed: M::Sys) {{ true }};\n\
         }}\n",
        selection.identity,
        selection.version,
        hex(&selection.digest)
    );
    with_namespace(&text, |namespace| {
        let bindings = bind_models(namespace, &inputs, &mut work());
        for (name, cause) in [
            ("MissingType", ModelErrorKind::MissingExport),
            ("MissingField", ModelErrorKind::MissingExport),
            ("MemberAsType", ModelErrorKind::WrongExportKind),
            ("PortAsType", ModelErrorKind::WrongExportKind),
            ("ConnectionAsRole", ModelErrorKind::WrongExportKind),
            ("NotAConnection", ModelErrorKind::WrongExportKind),
        ] {
            let (_, refusals) = resolved(&bindings, namespace, name);
            assert!(!refusals.is_empty(), "{name} refuses");
            assert!(
                refusals.iter().all(|refusal| *refusal == cause),
                "{name}: {refusals:?}"
            );
        }
    });
}

/// TC-148 step 4: a same-shaped package under another identity, a changed
/// selected digest, and the right digest spelled in the raw-byte domain each
/// refuse every declaration that depends on the import, while `Unrelated`
/// stays bound.
#[trace("TC-148", "FR-036-AC-9")]
#[test]
fn substituted_package_or_changed_digest_refuses_only_dependents() {
    let bundle = architecture_bundle(|_| {});
    let package = admitted(bundle.path());
    let other = architecture_bundle(|root| {
        edit(
            &root.join("spec/spec.md"),
            "name: architecture",
            "name: plumbing",
        );
    });
    let other = admitted(other.path());
    assert_eq!(other.selection().identity, "agent-ix/plumbing");
    assert!(
        other.declaration("pump_out").is_some(),
        "the substitute declares a same-shaped pump_out"
    );
    let selection = package.selection().clone();
    let mut changed = selection.digest;
    changed[0] ^= 1;

    for (inputs, digest, refusal) in [
        (
            [ModelInput::Domain(&other)],
            format!("sha256-jcs:{}", hex(&selection.digest)),
            ImportRefusal::MissingPackage,
        ),
        (
            [ModelInput::Domain(&package)],
            format!("sha256-jcs:{}", hex(&changed)),
            ImportRefusal::StaleSelection,
        ),
        (
            [ModelInput::Domain(&package)],
            format!("sha256:{}", hex(&selection.digest)),
            ImportRefusal::DigestDomainMismatch,
        ),
    ] {
        let text = program(&selection.identity, &selection.version, &digest);
        with_namespace(&text, |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            assert_eq!(bindings.imports()[0].selection, Err(refusal.clone()));
            for name in ["FieldRule", "Plumbing"] {
                let (declarations, refusals) = resolved(&bindings, namespace, name);
                assert!(declarations.is_empty(), "{name}: {declarations:?}");
                assert!(
                    !refusals.is_empty()
                        && refusals
                            .iter()
                            .all(|kind| *kind == ModelErrorKind::RefusedImport { import: 0 }),
                    "{name} ({refusal:?}): {refusals:?}"
                );
            }
            assert_eq!(
                resolved(&bindings, namespace, "Unrelated"),
                (vec![], vec![])
            );
        });
    }
}

/// FR-036-AC-9 at the package report: under the admitted selection every
/// declaration's names resolve; with the selected digest changed, each
/// declaration naming the import refuses, `Caller`, which only calls one of
/// them, refuses as its dependent, and `Unrelated` stays bound.
#[trace("TC-148", "FR-036-AC-9")]
#[test]
fn changed_digest_refuses_dependents_in_the_package_report() {
    use quire_spec_language::linking::composed::binding::{self, Disposition, Refusal};
    use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
    use quire_spec_language::linking::composed::definitions::Inventory;

    let bundle = architecture_bundle(|_| {});
    let package = admitted(bundle.path());
    let selection = package.selection().clone();
    let inputs = [ModelInput::Domain(&package)];
    let (definitions, rules) = crate::composed_binding::artifacts();
    let selected = Inventory {
        edition: R::Edition.selection(),
        definitions: &definitions,
        rules: &rules,
    };
    let profile = R::StateQueries.selection();
    let mut changed = selection.digest;
    changed[31] ^= 1;
    for (digest, bound) in [(selection.digest, true), (changed, false)] {
        let text = format!(
            "language \"ix:native\" edition \"1-draft\";\n\
             profile S = \"{}\" version \"{}\" digest \"{}\";\n\
             model M = \"{}\" version \"{}\" digest \"sha256-jcs:{}\";\n\
             predicate FieldRule using S (): Boolean {{ M::Pump::id = M::Pump::id }}\n\
             predicate PumpRule using S (pump: M::Pump): Boolean {{ true }}\n\
             predicate Caller using S (): Boolean {{ FieldRule() }}\n\
             predicate Unrelated using S (flag: Boolean): Boolean {{ flag }}\n",
            profile.identity,
            profile.revision,
            profile.digest,
            selection.identity,
            selection.version,
            hex(&digest)
        );
        with_namespace(&text, |namespace| {
            let report = binding::bind(namespace, &selected, &inputs, BindingLimits::default());
            assert!(report.complete(), "{:?}", report.exhaustion());
            let disposition = |name: &str| report.disposition(namespace.lookup(name)[0]);
            let expected = if bound {
                Disposition::NamesResolved
            } else {
                Disposition::Refused
            };
            for name in ["FieldRule", "PumpRule", "Caller"] {
                assert_eq!(
                    disposition(name),
                    Some(expected),
                    "{name}: {:?}",
                    report.declarations()[namespace.lookup(name)[0].index()].refusals()
                );
            }
            assert_eq!(disposition("Unrelated"), Some(Disposition::NamesResolved));
            if !bound {
                let field_rule = namespace.lookup("FieldRule")[0];
                let caller = namespace.lookup("Caller")[0];
                assert!(report.declarations()[caller.index()].refusals().iter().any(
                    |cause| matches!(
                        cause,
                        Refusal::Dependency { target, .. } if *target == field_rule
                    )
                ));
            }
        });
    }
}

/// The reverse digest-domain substitution: a native model selected under a
/// `sha256-jcs:` spelling of its own artifact digest refuses
/// `DigestDomainMismatch` rather than binding.
#[trace("TC-148", "FR-036-AC-9")]
#[test]
fn native_model_selected_in_the_jcs_domain_refuses() {
    let model = crate::support::native_rule_model::parts().model();
    let inputs = [ModelInput::Native(&model)];
    let owner = model.environment().owner();
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"test:unresolved-definition\" version \"1\" digest \"unresolved\";\n\
         model M = \"{}\" version \"{}\" digest \"sha256-jcs:{:x}\";\n\
         predicate Rule using S (item: M::Node): Boolean {{ true }}\n",
        owner.package().as_str(),
        owner.revision().get(),
        model.digest()
    );
    with_namespace(&text, |namespace| {
        let bindings = bind_models(namespace, &inputs, &mut work());
        assert_eq!(
            bindings.imports()[0].selection,
            Err(ImportRefusal::DigestDomainMismatch)
        );
        assert_eq!(
            resolved(&bindings, namespace, "Rule").1,
            [ModelErrorKind::RefusedImport { import: 0 }]
        );
    });
}

/// Both same-shaped packages supplied: each alias binds only its own
/// selection, and every bound key stays under that selection's identity.
#[trace("TC-148", "FR-036-AC-9")]
#[test]
fn two_same_shaped_packages_keep_keys_under_their_selection() {
    let bundle = architecture_bundle(|_| {});
    let package = admitted(bundle.path());
    let other = architecture_bundle(|root| {
        edit(
            &root.join("spec/spec.md"),
            "name: architecture",
            "name: plumbing",
        );
    });
    let other = admitted(other.path());
    let inputs = [ModelInput::Domain(&other), ModelInput::Domain(&package)];
    let (a, b) = (package.selection(), other.selection());
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"test:unresolved-definition\" version \"1\" digest \"unresolved\";\n\
         model M = \"{}\" version \"{}\" digest \"sha256-jcs:{}\";\n\
         model N = \"{}\" version \"{}\" digest \"sha256-jcs:{}\";\n\
         predicate FromM using S (): Boolean {{ M::Pump::id = M::Pump::id }}\n\
         predicate FromN using S (): Boolean {{ N::Pump::id = N::Pump::id }}\n",
        a.identity,
        a.version,
        hex(&a.digest),
        b.identity,
        b.version,
        hex(&b.digest)
    );
    with_namespace(&text, |namespace| {
        let bindings = bind_models(namespace, &inputs, &mut work());
        assert_eq!(bindings.imports()[0].selection, Ok(1));
        assert_eq!(bindings.imports()[1].selection, Ok(0));
        for (name, identity) in [("FromM", &a.identity), ("FromN", &b.identity)] {
            let report = bindings
                .resolve_declaration(namespace, namespace.lookup(name)[0], &mut work())
                .expect("declaration handle");
            assert!(report.refusals.is_empty(), "{name}: {:?}", report.refusals);
            assert_eq!(report.occurrences.len(), 2);
            for occurrence in &report.occurrences {
                let ModelTarget::Declaration(bound) = &occurrence.target else {
                    panic!("{name} binds a domain declaration")
                };
                assert_eq!(&bound.key().package, identity);
                assert_eq!(bound.key().node, format!("ix://{identity}/Pump/id"));
                assert_eq!(&bound.selection().identity, identity);
            }
        }
    });
}

/// The composed type checker types domain declarations (FR-042-AC-11's
/// checking prerequisite): a parameter typed by a domain object type,
/// Interface or record value type is typed, and a field access reads the
/// field's value type. A field whose value type has no native type here
/// (`Count.value` is an `Integer`), a field the type does not declare, and
/// equality of domain values each refuse with their own cause, while
/// `Unrelated` stays typed.
#[trace("TC-148", "FR-036-AC-9", "FR-042-AC-11")]
#[test]
fn domain_typed_declarations_type_check() {
    use quire_spec_language::checking::composed::{
        self, CauseKind, Prerequisite, TypeDisposition, TypeLimits,
    };
    use quire_spec_language::checking::NativeType;
    use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;

    let bundle = architecture_bundle(|_| {});
    let package = admitted(bundle.path());
    let selection = package.selection().clone();
    let profile = R::StateQueries.selection();
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"{}\" version \"{}\" digest \"{}\";\n\
         model M = \"{}\" version \"{}\" digest \"sha256-jcs:{}\";\n\
         predicate PumpUp using S (pump: M::Pump): Boolean {{ pump.id }}\n\
         predicate Flowing using S (flow: M::Flow, tally: M::Count): Boolean {{ true }}\n\
         predicate CountValue using S (tally: M::Count): Boolean {{ tally.value = tally.value }}\n\
         predicate Missing using S (pump: M::Pump): Boolean {{ pump.speed }}\n\
         predicate SamePump using S (a: M::Pump, b: M::Pump): Boolean {{ a = b }}\n\
         predicate Mixed using S (pump: M::Pump, tally: M::Count): Boolean {{ pump = tally }}\n\
         predicate Unrelated using S (flag: Boolean): Boolean {{ flag }}\n",
        profile.identity,
        profile.revision,
        profile.digest,
        selection.identity,
        selection.version,
        hex(&selection.digest)
    );
    let source = Source::read(
        SourceIdentity {
            authority: "test".into(),
            identity: "pumps".into(),
            revision_namespace: "test".into(),
            revision: "selected".into(),
        },
        "pumps.native".to_owned(),
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .expect("source reads");
    let sources = [source];
    let formal = crate::support::composed_types::formal_sources(&sources);
    let inputs = [ModelInput::Domain(&package)];
    crate::support::composed_types::with_binding_inputs(
        &sources,
        &inputs,
        BindingLimits::default(),
        |binding| {
            let namespace = binding.namespace();
            let report = composed::admit_types(binding, &formal, TypeLimits::default());
            assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
            let id = |name: &str| namespace.lookup(name)[0];
            let causes = |name: &str| -> Vec<CauseKind> {
                report
                    .declaration(id(name))
                    .expect("typed record")
                    .causes()
                    .iter()
                    .map(|cause| cause.kind.clone())
                    .collect()
            };
            for name in ["PumpUp", "Flowing", "Unrelated"] {
                assert_eq!(
                    report.disposition(id(name)),
                    Some(TypeDisposition::Typed),
                    "{name}: {:?}",
                    causes(name)
                );
            }
            // `pump` is typed by the `Pump` declaration under the admitted
            // selection, and `pump.id` by the field's `Boolean`.
            let pump_up = report.declaration(id("PumpUp")).expect("typed record");
            let Some(NativeType::Domain(pump)) = &pump_up.binders()[0].ty else {
                panic!("pump is a domain type: {:?}", pump_up.binders()[0].ty)
            };
            assert_eq!(pump.declaration.key, &key("Pump"));
            assert_eq!(pump.package.selection(), &selection);
            assert!(pump_up
                .nodes()
                .iter()
                .any(|node| node.ty == Some(NativeType::Boolean)));
            for (name, cause) in [
                (
                    "CountValue",
                    CauseKind::UnsupportedPrerequisite(Prerequisite::DomainRepresentation),
                ),
                ("Missing", CauseKind::InvalidField),
                ("SamePump", CauseKind::ForbiddenOperator),
                ("Mixed", CauseKind::TypeMismatch),
            ] {
                assert_eq!(
                    report.disposition(id(name)),
                    Some(TypeDisposition::Refused),
                    "{name}"
                );
                assert!(causes(name).contains(&cause), "{name}: {:?}", causes(name));
            }
        },
    );
}

/// A test-authored record value type written into the temp bundle copy, with
/// one `Boolean` field per `(name, multiplicity)`.
pub(crate) fn record_value_type(root: &Path, name: &str, fields: &[(&str, &str, &str)]) {
    let rows: String = fields
        .iter()
        .map(|(field, ty, multiplicity)| format!("| {field} | {ty} | {multiplicity} | |\n"))
        .collect();
    std::fs::write(
        root.join(format!("spec/model/{name}.md")),
        format!(
            "---\nid: {name}\ntitle: {name}\nobject: value_object\ntype: FR\nname: {name}\n---\n\n\
             # {name}: {name}\n\n## Description\n\nA test-authored record value type.\n\n\
             ## Properties\n\n| Field | Type | Multiplicity | Constraints |\n\
             |-------|------|--------------|-------------|\n{rows}"
        ),
    )
    .expect("write record value type");
}

/// The checker's field multiplicity rules over record value types written
/// into the bundle: `0..1` types as an option and `0..3` as a sequence of at
/// most 3, while `0..*` (unbounded) and `1..3` (a lower bound other than 0 or
/// the single value) refuse as an unsupported domain representation. A read
/// through a field typed by another record value type types its inner field
/// (`o.inner.ok`).
#[trace("TC-148", "FR-042-AC-11")]
#[test]
fn domain_field_multiplicities_and_nested_reads_type_check() {
    use quire_spec_language::checking::composed::{
        self, CauseKind, Prerequisite, TypeDisposition, TypeLimits,
    };
    use quire_spec_language::checking::NativeType;
    use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;

    let bundle = architecture_bundle(|root| {
        record_value_type(
            root,
            "Gauge",
            &[
                ("maybe", "Boolean", "0..1"),
                ("few", "Boolean", "0..3"),
                ("many", "Boolean", "0..*"),
                ("some", "Boolean", "1..3"),
            ],
        );
        record_value_type(root, "Inner", &[("ok", "Boolean", "1")]);
        record_value_type(root, "Outer", &[("inner", "Inner", "1")]);
    });
    let package = admitted(bundle.path());
    let selection = package.selection().clone();
    let profile = R::StateQueries.selection();
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"{}\" version \"{}\" digest \"{}\";\n\
         model M = \"{}\" version \"{}\" digest \"sha256-jcs:{}\";\n\
         predicate Maybe using S (g: M::Gauge): Boolean {{ present(g.maybe) }}\n\
         predicate Few using S (g: M::Gauge): Boolean {{ let few = g.few in true }}\n\
         predicate Many using S (g: M::Gauge): Boolean {{ let many = g.many in true }}\n\
         predicate Some using S (g: M::Gauge): Boolean {{ let some = g.some in true }}\n\
         predicate Nested using S (o: M::Outer): Boolean {{ o.inner.ok }}\n",
        profile.identity,
        profile.revision,
        profile.digest,
        selection.identity,
        selection.version,
        hex(&selection.digest)
    );
    let source = Source::read(
        SourceIdentity {
            authority: "test".into(),
            identity: "gauges".into(),
            revision_namespace: "test".into(),
            revision: "selected".into(),
        },
        "gauges.native".to_owned(),
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .expect("source reads");
    let sources = [source];
    let formal = crate::support::composed_types::formal_sources(&sources);
    let inputs = [ModelInput::Domain(&package)];
    crate::support::composed_types::with_binding_inputs(
        &sources,
        &inputs,
        BindingLimits::default(),
        |binding| {
            let namespace = binding.namespace();
            let report = composed::admit_types(binding, &formal, TypeLimits::default());
            assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
            let id = |name: &str| namespace.lookup(name)[0];
            let declaration = |name: &str| report.declaration(id(name)).expect("typed record");
            let causes = |name: &str| -> Vec<CauseKind> {
                declaration(name)
                    .causes()
                    .iter()
                    .map(|cause| cause.kind.clone())
                    .collect()
            };
            let has_node = |name: &str, expected: &dyn Fn(&NativeType<'_>) -> bool| {
                declaration(name)
                    .nodes()
                    .iter()
                    .any(|node| node.ty.as_ref().is_some_and(expected))
            };
            for name in ["Maybe", "Few", "Nested"] {
                assert_eq!(
                    report.disposition(id(name)),
                    Some(TypeDisposition::Typed),
                    "{name}: {:?}",
                    causes(name)
                );
            }
            assert!(has_node("Maybe", &|ty| matches!(
                ty,
                NativeType::Option(inner) if **inner == NativeType::Boolean
            )));
            assert!(has_node("Few", &|ty| matches!(
                ty,
                NativeType::Sequence { element, maximum: 3 } if **element == NativeType::Boolean
            )));
            // `o.inner` is the `Inner` record value type; `o.inner.ok` its
            // Boolean field.
            assert!(has_node("Nested", &|ty| matches!(
                ty,
                NativeType::Domain(inner) if inner.declaration.key == &key("Inner")
            )));
            for name in ["Many", "Some"] {
                assert_eq!(
                    report.disposition(id(name)),
                    Some(TypeDisposition::Refused),
                    "{name}"
                );
                assert!(
                    causes(name).contains(&CauseKind::UnsupportedPrerequisite(
                        Prerequisite::DomainRepresentation
                    )),
                    "{name}: {:?}",
                    causes(name)
                );
            }
        },
    );
}

/// Appends six operations with parameters or a result to the temp copy's
/// `Pump`: `fill(level: Reading, forced: Boolean): Boolean`,
/// `swap(peer: Pump)`, `meter(amount: Integer)`, `total(): Integer`, and
/// `latch(result: Boolean): Boolean` and `hold(self: Boolean)`, whose
/// parameters spell a clause's ambient slots. The
/// bundle's own only operation, `run`, takes no parameters and returns
/// nothing. `Reading` is written by the caller.
pub(crate) fn pump_operations(root: &Path) {
    let pump = root.join("spec/model/Pump.md");
    let mut text = std::fs::read_to_string(&pump).expect("read Pump");
    text.push_str(
        "\n### fill\n\nFill the pump to a reading.\n\n\
         | Param | Type | Multiplicity | Constraints |\n\
         |-------|------|--------------|-------------|\n\
         | level | Reading | 1 | |\n\
         | forced | Boolean | 1 | |\n\n\
         Returns: Boolean [1]\n\n\
         ### swap\n\nSwap with a peer pump.\n\n\
         | Param | Type | Multiplicity | Constraints |\n\
         |-------|------|--------------|-------------|\n\
         | peer | Pump | 1 | |\n\n\
         ### meter\n\nMeter an amount.\n\n\
         | Param | Type | Multiplicity | Constraints |\n\
         |-------|------|--------------|-------------|\n\
         | amount | Integer | 1 | |\n\n\
         ### total\n\nThe pumped total.\n\n\
         Returns: Integer [1]\n\n\
         ### latch\n\nLatch a result.\n\n\
         | Param | Type | Multiplicity | Constraints |\n\
         |-------|------|--------------|-------------|\n\
         | result | Boolean | 1 | |\n\n\
         Returns: Boolean [1]\n\n\
         ### hold\n\nHold itself.\n\n\
         | Param | Type | Multiplicity | Constraints |\n\
         |-------|------|--------------|-------------|\n\
         | self | Boolean | 1 | |\n",
    );
    std::fs::write(&pump, text).expect("write Pump");
}

/// A state clause over a domain operation binds its declared parameters as
/// invocation parameters, in the admitted member's order, and, in a
/// postcondition, its
/// declared result, each typed from the admitted operation member: `level`
/// by the `Reading` record value type, `forced` and `result` by `Boolean`.
/// As for a native operation, a precondition reads a Boolean parameter, and
/// a record-typed parameter, observed at the invocation input, is read in a
/// postcondition; read in a precondition it refuses as `InvalidPreSelection`.
/// A parameter or result typed `Integer` has no native type here and refuses
/// as an unsupported domain representation, whether or not the body reads
/// it, while a precondition of `total` binds no result and types. A name the
/// operation does not declare, `result` in a precondition, and a parameter
/// named `result` or `self`, refuse at scope resolution.
#[trace("TC-121", "FR-042-AC-14")]
#[test]
fn domain_operation_parameters_and_result_bind_and_type_check() {
    use quire_spec_language::checking::composed::{
        self, CauseKind, Prerequisite, TypeDisposition, TypeLimits,
    };
    use quire_spec_language::checking::NativeType;
    use quire_spec_language::linking::composed::definition_source::RegisteredDefinition as R;
    use quire_spec_language::linking::composed::scopes::{
        Anchor, BinderKind, ScopeDisposition, ScopeIssue,
    };

    let bundle = architecture_bundle(|root| {
        record_value_type(root, "Reading", &[("ok", "Boolean", "1")]);
        pump_operations(root);
    });
    let package = admitted(bundle.path());
    let selection = package.selection().clone();
    let profile = R::StateQueries.selection();
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"{}\" version \"{}\" digest \"{}\";\n\
         model M = \"{}\" version \"{}\" digest \"sha256-jcs:{}\";\n\
         pre FillReady using S on M::Pump::fill {{ forced }}\n\
         post Filled using S on M::Pump::fill {{ result and level.ok and forced }}\n\
         pre LevelFirst using S on M::Pump::fill {{ level.ok }}\n\
         pre Metered using S on M::Pump::meter {{ true }}\n\
         post Totalled using S on M::Pump::total {{ true }}\n\
         pre TotalReady using S on M::Pump::total {{ true }}\n\
         pre Undeclared using S on M::Pump::fill {{ amount }}\n\
         pre EarlyResult using S on M::Pump::fill {{ result }}\n\
         post Latched using S on M::Pump::latch {{ result }}\n\
         pre Held using S on M::Pump::hold {{ true }}\n",
        profile.identity,
        profile.revision,
        profile.digest,
        selection.identity,
        selection.version,
        hex(&selection.digest)
    );
    let source = Source::read(
        SourceIdentity {
            authority: "test".into(),
            identity: "fills".into(),
            revision_namespace: "test".into(),
            revision: "selected".into(),
        },
        "fills.native".to_owned(),
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .expect("source reads");
    let sources = [source];
    let formal = crate::support::composed_types::formal_sources(&sources);
    let inputs = [ModelInput::Domain(&package)];
    crate::support::composed_types::with_binding_inputs(
        &sources,
        &inputs,
        BindingLimits::default(),
        |binding| {
            let namespace = binding.namespace();
            let scopes = binding.scopes().expect("scopes resolved");
            let report = composed::admit_types(binding, &formal, TypeLimits::default());
            assert!(report.exhaustion().is_none(), "{:?}", report.exhaustion());
            let id = |name: &str| namespace.lookup(name)[0];
            let scope = |name: &str| scopes.declaration(id(name)).expect("scope record");
            let causes = |name: &str| -> Vec<CauseKind> {
                report
                    .declaration(id(name))
                    .expect("typed record")
                    .causes()
                    .iter()
                    .map(|cause| cause.kind.clone())
                    .collect()
            };
            for name in ["FillReady", "Filled", "TotalReady"] {
                assert_eq!(
                    report.disposition(id(name)),
                    Some(TypeDisposition::Typed),
                    "{name}: {:?}",
                    causes(name)
                );
            }
            // Each binder of `name` of `kind`, with its name, anchor and type.
            let binders = |name: &str, kind: BinderKind| {
                let typed = report.declaration(id(name)).expect("typed record");
                scope(name)
                    .binders
                    .iter()
                    .enumerate()
                    .filter(|(_, binder)| binder.kind == kind)
                    .map(|(index, binder)| {
                        let ty = typed
                            .binders()
                            .iter()
                            .find(|typed| typed.binder == index)
                            .and_then(|typed| typed.ty.clone());
                        (binder.name.clone(), binder.anchor, ty)
                    })
                    .collect::<Vec<_>>()
            };
            let is_reading = |ty: &Option<NativeType<'_>>| {
                matches!(ty, Some(NativeType::Domain(reading))
                    if reading.declaration.key == &key("Reading"))
            };
            for name in ["FillReady", "Filled"] {
                // The lift orders an operation's parameters by identity.
                let [(Some(forced), Anchor::InvocationInput, Some(NativeType::Boolean)), (Some(level), Anchor::InvocationInput, level_ty)] =
                    &binders(name, BinderKind::InvocationParameter)[..]
                else {
                    panic!("{name}: two invocation parameters")
                };
                assert_eq!((level.as_str(), forced.as_str()), ("level", "forced"));
                assert!(is_reading(level_ty), "{name}: {level_ty:?}");
            }
            assert!(binders("FillReady", BinderKind::ResultValue).is_empty());
            assert!(binders("TotalReady", BinderKind::ResultValue).is_empty());
            let [(None, Anchor::InvocationPost, Some(NativeType::Boolean))] =
                &binders("Filled", BinderKind::ResultValue)[..]
            else {
                panic!("Filled: one Boolean result")
            };

            assert_eq!(
                report.disposition(id("LevelFirst")),
                Some(TypeDisposition::Refused)
            );
            assert!(causes("LevelFirst").contains(&CauseKind::InvalidPreSelection));
            for name in ["Metered", "Totalled"] {
                assert_eq!(
                    report.disposition(id(name)),
                    Some(TypeDisposition::Refused),
                    "{name}"
                );
                assert!(
                    causes(name).contains(&CauseKind::UnsupportedPrerequisite(
                        Prerequisite::DomainRepresentation
                    )),
                    "{name}: {:?}",
                    causes(name)
                );
            }

            let undeclared = scope("Undeclared");
            assert_eq!(undeclared.disposition(), ScopeDisposition::Refused);
            let [ScopeIssue::MissingValue { name, .. }] = undeclared.issues.as_slice() else {
                panic!("one missing value: {:?}", undeclared.issues)
            };
            assert_eq!(name, "amount");
            let early = scope("EarlyResult");
            assert_eq!(early.disposition(), ScopeDisposition::Refused);
            assert!(
                matches!(
                    early.issues.as_slice(),
                    [ScopeIssue::AmbientUnavailable { .. }]
                ),
                "{:?}",
                early.issues
            );
            // A parameter named `result` or `self` refuses where it is bound,
            // at the operation selector, rather than shadowing the ambient slot.
            for (name, operation) in [("Latched", "latch"), ("Held", "hold")] {
                let reserved = scope(name);
                assert_eq!(reserved.disposition(), ScopeDisposition::Refused);
                let [ScopeIssue::ReservedBinder { span }] = reserved.issues.as_slice() else {
                    panic!("{name}: one reserved binder: {:?}", reserved.issues)
                };
                let unit = namespace.unit(reserved.unit).expect("unit");
                assert_eq!(unit.source().slice(*span), Some(operation));
            }
            for name in ["Undeclared", "EarlyResult", "Latched", "Held"] {
                assert_ne!(
                    report.disposition(id(name)),
                    Some(TypeDisposition::Typed),
                    "{name}"
                );
            }
        },
    );
}

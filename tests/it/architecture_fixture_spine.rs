// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1308 row 17 (FR-036-AC-9, FR-056-AC-8, FR-513-AC-8): the FCD #173
//! architecture bundle enters through spine I2, is selected by a native model
//! declaration, and its field, Port and Connection records retain their
//! selected identities through S3. Every step runs on the spine
//! (`qsl_replay::spine` parse, select, check), never on a composed path.
//!
//! The bundle is read at test time from the locked
//! `agent-ix-extraction-frontend` checkout and copied to a temporary root, so
//! edits never touch the shared checkout. The unedited bundle does not admit
//! whole: `Pump`/`Sys`/`Tank.id` are typed `UUID`, which is not a QSL native
//! value type, and `Flow2`'s inline relationship is not identified
//! `<owner>/<name>`. [`architecture_bundle`] types the three identity fields
//! `Boolean` and drops `Flow2`; every part, port, connection and allocation
//! is kept as authored.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::DependencyInput;
use qsl_semantics::model::accounting::{Meter, ModelNormalizationLimits};
use qsl_semantics::model::domain_package::{DomainPackageRecord, PortDirection};
use qsl_semantics::model::index::ModelIndex;
use qsl_semantics::model::intake::lift_document;
use qsl_semantics::model::systems::{
    check_connection, classify, ConnectionCheckOutcome, ConnectionOutcome,
};
use quire_checked::key::DeclarationKey;
use quire_exact::Cancel;

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
            std::fs::copy(entry.path(), &target).expect("copy canonical bundle file");
        }
    }
}

fn write_overlay(path: &Path, bytes: &[u8]) {
    if std::fs::symlink_metadata(path).is_ok() {
        std::fs::remove_file(path).expect("detach canonical fixture file");
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create overlay parent");
    }
    std::fs::write(path, bytes).expect("write overlay file");
}

fn edit(path: &Path, from: &str, to: &str) {
    let text = std::fs::read_to_string(path).expect("read bundle file");
    assert!(
        text.contains(from),
        "{}: {from:?} not found",
        path.display()
    );
    write_overlay(path, text.replace(from, to).as_bytes());
}

/// A temp copy of the canonical architecture fixture, then `extra` applied
/// to the copy.
fn architecture_bundle(extra: impl FnOnce(&Path)) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let source = fcd_fixtures_dir().join("architecture");
    copy_tree(&source, dir.path());
    assert!(
        dir.path().join("spec/spec.md").is_file(),
        "canonical fixture root did not copy spec/spec.md from {}",
        source.display()
    );
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
        "modules/fixture-domain",
        "modules/fixture-edges",
        "modules/fixture-systems",
    ]
    .map(|root| fixtures.join(root));
    lift_document(bundle, &modules).expect("the edited bundle lifts")
}

fn key(artifact: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: format!("ix://{PACKAGE}/{artifact}"),
    }
}

/// TC-1308 row 17: the pinned FCD #173 architecture bundle enters through
/// spine I2, is selected by a native model declaration, and its field, Port
/// and Connection records retain their selected identities through S3.
#[trace("TC-1308", "FR-036-AC-9", "FR-056-AC-8", "FR-513-AC-8")]
#[test]
fn architecture_fixture_binds_references_through_spine_i2() {
    let bundle = architecture_bundle(|_| {});
    let bytes = lift(bundle.path());
    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()])
        .expect("the lifted FCD package enters I2 intake");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"quire.value.complete/v1\";\n\
         profile P = \"quire.protocol.finite-global/v1\";\n\
         model M = \"{PACKAGE}\";\n\
         pre FieldRule using S on M::Pump::run {{ self.id }}\n\
         protocol Plumbing using P over (view: M::Sys) on origin {{\n\
             role Operator on M::Sys;\n\
             role Outlet on M::pump_out;\n\
             relationship Pipe = M::pipe;\n\
             run sequence Main {{ check Valid using S {{ true }}; }}\n\
             finish Closed as (closed: M::Sys) {{ true }};\n\
         }}\n\
         predicate Unrelated using S (flag: Boolean): Boolean {{ flag }}\n"
    );
    let source = SourceIdentity::new("agent-ix", "test:tc-1308-fcd-i2");
    let cancel = Cancel::new();
    let limits = qsl_replay::spine::SpineLimits::default();
    let failure = |failure| {
        qsl_replay::spine::refusal_or_fault(failure).expect("no internal front-end fault")
    };
    let parsed = qsl_replay::spine::parse(
        &qsl_replay::spine::ParseRequest {
            source: &source,
            path: "architecture.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(failure)
    .expect("the native unit parses")
    .into_value();
    let models = qsl_replay::spine::select(&parsed, &packages, limits.model, &cancel)
        .map_err(failure)
        .expect("the fixture is selected by model M")
        .into_value();
    let selected_model = models
        .models()
        .iter()
        .find(|model| model.alias == "M")
        .expect("selected M model");
    let selected_package = selected_model.view.domain_package();
    let endpoint = |artifact: &str| {
        selected_package
            .records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::Endpoint(endpoint)
                    if endpoint.key.node == key(artifact).node =>
                {
                    Some(endpoint)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("selected model is missing endpoint {artifact}"))
    };
    let pump_out = endpoint("pump_out");
    assert_eq!(pump_out.owning_component, key("sys_pump"));
    assert_eq!(pump_out.direction, Some(PortDirection::Out));
    assert_eq!(pump_out.value_type, key("Flow"));
    assert_eq!(
        (pump_out.multiplicity.lower, pump_out.multiplicity.upper),
        (1, Some(1))
    );
    let tank_in = endpoint("tank_in");
    assert_eq!(tank_in.owning_component, key("sys_tank"));
    assert_eq!(tank_in.direction, Some(PortDirection::In));
    assert_eq!(tank_in.value_type, key("Flow"));
    assert_eq!(
        (tank_in.multiplicity.lower, tank_in.multiplicity.upper),
        (1, Some(1))
    );
    let selected_domain = selected_package.clone();
    let mut meter = Meter::new(ModelNormalizationLimits::default());
    let systems = classify(&selected_domain, &mut meter).expect("selected systems classify");
    assert_eq!(
        check_connection(
            &ModelIndex::build(selected_domain),
            &systems,
            &key("pipe"),
            &mut meter,
        ),
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted)
    );
    let checked = qsl_replay::spine::check(
        &parsed,
        "tc-1308-fcd-i2".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(failure)
    .expect("field, Port and Connection references check on the spine")
    .into_value();

    let selections = checked.package().graph().model_selections();
    assert_eq!(selections.len(), 1);
    assert_eq!(selections[0].identity.as_str(), PACKAGE);
    let plumbing = checked
        .package()
        .graph()
        .protocols()
        .iter()
        .find(|protocol| protocol.name == "Plumbing")
        .expect("Plumbing checked protocol");
    let outlet = plumbing
        .roles
        .iter()
        .find(|role| role.name == "Outlet")
        .expect("Outlet role binds to the selected Port");
    assert_eq!(outlet.model_member.as_ref(), Some(&key("pump_out")));
    assert_eq!(
        outlet.model_selection.as_deref(),
        Some(PACKAGE),
        "a Port role binds the selection of the model that owns the Port"
    );
    let operator = plumbing
        .roles
        .iter()
        .find(|role| role.name == "Operator")
        .expect("Operator role");
    assert_eq!(operator.model_member, None);
    assert_eq!(
        operator.model_selection, None,
        "a qualified role that is not a Port keeps the whole-name lookup and binds no selection"
    );
    assert_eq!(
        plumbing.relationships[0].key,
        key("pipe"),
        "the protocol relationship retains the connection declaration key"
    );
    assert!(
        checked
            .package()
            .graph()
            .field_domain(
                &quire_exact::Identifier::new("M").unwrap(),
                &quire_exact::Identifier::new("Pump").unwrap(),
                &quire_exact::Identifier::new("id").unwrap(),
            )
            .expect("field binding has no internal fault")
            .is_some(),
        "the field read binds to the selected Pump.id domain key"
    );
    let emitted = qsl_package::emit_checked(checked.package(), qsl_package::EmitLimits::fixture())
        .expect("checked unit emits");
    assert!(
        !emitted.package().bytes().is_empty(),
        "the checked package carrying the bindings emits"
    );
    // QSL-1579: the checked-package round trip keeps each role's selected
    // model type. A Port role is on the Port's owning object type; the Port
    // key itself has no carrier on the current CheckedPackageV2 role layout.
    let read = crate::emit_checked_v2::read_back(emitted.package().bytes(), &[bytes.as_slice()]);
    let in_process = checked
        .package()
        .protocol_occurrence_schemas()
        .expect("in-process occurrence schema");
    let read_back = qsl_package::protocol_occurrence_schemas_from_read(&read)
        .expect("read-back occurrence schema");
    assert_eq!(in_process, read_back, "in-process/read-back parity");
    let role_model = |name: &str| {
        read_back
            .iter()
            .flat_map(|occurrence| &occurrence.schema.roles)
            .find(|role| role.path.last().map(String::as_str) == Some(name))
            .unwrap_or_else(|| panic!("read-back role {name}"))
            .model
            .clone()
    };
    let sys = quire_exact::NodeKey::new(
        PACKAGE,
        ["model", "object_type", &format!("ix://{PACKAGE}/Sys")]
            .into_iter()
            .map(|segment| quire_exact::PathSegment::Text(segment.to_owned()))
            .collect(),
        0,
    )
    .expect("a structural key");
    assert_eq!(
        role_model("Outlet"),
        sys,
        "the Port role reads back on the Port's owning object type"
    );
    assert_eq!(
        role_model("Operator"),
        sys,
        "the whole-type role reads back on the same object type, so only a \
         role `model_member` carrier can tell the Port role apart"
    );
    assert!(
        checked
            .package()
            .graph()
            .protocols()
            .iter()
            .any(|protocol| protocol.name == "Plumbing"),
        "the checked protocol retains the fixture's role and relationship bindings"
    );
}

/// `text` through the spine's parse, select and check over `packages`.
fn spine_check(
    text: &str,
    packages: &qsl_semantics::model::intake::PackageInput,
) -> Result<qsl_replay::spine::CheckedUnit, Box<qsl_replay::spine::CompileRefusal>> {
    let source = SourceIdentity::new("agent-ix", "test:fcd-refusals");
    let cancel = Cancel::new();
    let limits = qsl_replay::spine::SpineLimits::default();
    let fault = |failure| {
        qsl_replay::spine::refusal_or_fault(failure).expect("no internal front-end fault")
    };
    let parsed = qsl_replay::spine::parse(
        &qsl_replay::spine::ParseRequest {
            source: &source,
            path: "refusals.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(fault)?
    .into_value();
    let models = qsl_replay::spine::select(&parsed, packages, limits.model, &cancel)
        .map_err(fault)?
        .into_value();
    Ok(qsl_replay::spine::check(
        &parsed,
        "fcd-refusals".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(fault)?
    .into_value())
}

fn unit(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"quire.model.complete/v1\";\n\
         model M = \"{PACKAGE}\";\n{declarations}"
    )
}

/// A systems part, port or allocation is retained for model-reference
/// binding but is not a value type: used where a type is written, each
/// refuses at the assembler, located on the written name.
#[trace("TC-1308", "FR-036-AC-9")]
#[test]
fn systems_records_and_missing_names_are_not_value_types_on_the_spine() {
    let bundle = architecture_bundle(|_| {});
    let bytes = lift(bundle.path());
    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()])
        .expect("the lifted FCD package enters I2 intake");
    // A part, a Port and an allocation (kinds the type site cannot take) and
    // a name the package does not declare.
    for record in ["sys_pump", "pump_out", "pump_alloc", "drain_out"] {
        let text = unit(&format!(
            "predicate AsType using S (x: M::{record}): Boolean {{ true }}\n"
        ));
        let refusal = spine_check(&text, &packages)
            .err()
            .unwrap_or_else(|| panic!("M::{record} is not a value type"));
        let qsl_replay::spine::CompileRefusal::Assembly {
            refusal,
            region: Some(region),
        } = refusal.as_ref()
        else {
            panic!("M::{record} refuses at the assembler: {refusal:?}");
        };
        let written = format!("M::{record}");
        assert!(
            matches!(
                refusal.errors.as_slice(),
                [qsl_semantics::check::AssemblyError {
                    cause: qsl_semantics::check::AssemblyCause::UnresolvedTypeName { name },
                    ..
                }] if *name == written
            ),
            "M::{record} is an unresolved type name: {refusal:?}"
        );
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        assert_eq!(text.get(start..end), Some(written.as_str()));
    }
}

/// A member the declared object type does not have refuses as a typing
/// failure while the declared member checks; and a package under another
/// identity does not satisfy the `model` selection, which refuses the unit at
/// intake, located on the `model` declaration, where the same unit over the
/// selected package checks.
#[trace("TC-148", "FR-036-AC-9")]
#[test]
fn missing_members_and_substituted_packages_refuse() {
    let bundle = architecture_bundle(|_| {});
    let bytes = lift(bundle.path());
    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()])
        .expect("the lifted FCD package enters I2 intake");
    let read = |member: &str| {
        unit(&format!(
            "predicate Read using S (p: Reference<M::Pump>): Boolean \
             {{ deref(p).{member} = deref(p).{member} }}\n\
             predicate Unrelated using S (flag: Boolean): Boolean {{ flag }}\n"
        ))
    };
    spine_check(&read("id"), &packages).expect("the declared field binds");
    let refusal = spine_check(&read("speed"), &packages)
        .err()
        .unwrap_or_else(|| panic!("a member Pump does not declare refuses"));
    assert!(
        matches!(*refusal, qsl_replay::spine::CompileRefusal::Check { .. }),
        "{refusal:?}"
    );

    let other = architecture_bundle(|root| {
        edit(
            &root.join("spec/spec.md"),
            "name: architecture",
            "name: plumbing",
        );
    });
    let other_bytes = lift(other.path());
    let substitute = qsl_semantics::model::intake::package_input([other_bytes.as_slice()])
        .expect("the same-shaped package under another identity enters intake");
    let text = read("id");
    let refusal = spine_check(&text, &substitute).err().unwrap_or_else(|| {
        panic!("a same-shaped package under another identity is not the selection")
    });
    let qsl_replay::spine::CompileRefusal::Intake {
        refusal,
        region: Some(region),
    } = refusal.as_ref()
    else {
        panic!("the selection refuses at intake: {refusal:?}");
    };
    assert_eq!(refusal.identity, PACKAGE);
    let start = usize::try_from(region.start()).unwrap();
    let end = usize::try_from(region.end()).unwrap();
    assert_eq!(
        text.get(start..end),
        Some(format!("model M = \"{PACKAGE}\";").as_str())
    );
}

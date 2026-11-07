// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1062 (FR-383-AC-62): a role on a systems Port keeps the Port's package
//! and declaration identity through checked-package emission and the strict
//! Contract IR read, and a role on a model type or on the whole model carries
//! no such identity. The fixture is FCD's architecture bundle, lifted through
//! the real pipeline and selected by a native model declaration.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::DependencyInput;
use qsl_semantics::model::intake::{lift_document, package_input};
use quire_exact::Cancel;

const PACKAGE: &str = "agent-ix/architecture";

/// `crates/extraction-frontend/fixtures` of the locked FCD checkout.
fn fcd_fixtures_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let output = Command::new(env!("CARGO"))
            .args(["metadata", "--format-version", "1", "--locked", "--offline"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("run Cargo's locked offline metadata resolver");
        assert!(output.status.success(), "cargo metadata failed");
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

/// The architecture bundle lifted to document bytes. `Pump`, `Sys` and `Tank`
/// type their `id` as `UUID`, which is not a QSL native value type, and
/// `Flow2` is identified in a form the reader refuses, so the copy retypes the
/// three ids `Boolean` and drops `Flow2`; every Port stays as authored.
fn lifted_bundle() -> Vec<u8> {
    let fixtures = fcd_fixtures_dir();
    let dir = tempfile::tempdir().expect("tempdir");
    copy_tree(&fixtures.join("architecture"), dir.path());
    let spec = dir.path().join("spec");
    for entity in ["model/Pump.md", "model/Sys.md", "model/Tank.md"] {
        let path = spec.join(entity);
        let text = std::fs::read_to_string(&path).expect("read entity");
        assert!(text.contains("| id | UUID |"), "{entity}");
        // Detach a possible link to the shared checkout before writing.
        std::fs::remove_file(&path).expect("detach entity");
        std::fs::write(&path, text.replace("| id | UUID |", "| id | Boolean |"))
            .expect("write entity");
    }
    std::fs::remove_file(spec.join("systems/Flow2.md")).expect("drop Flow2");
    let modules = [
        "modules/fixture-domain",
        "modules/fixture-edges",
        "modules/fixture-systems",
    ]
    .map(|root| fixtures.join(root));
    lift_document(dir.path(), &modules).expect("the bundle lifts")
}

#[trace("FR-383-AC-62", "TC-1062")]
#[test]
fn a_port_role_keeps_its_port_identity_through_emission_and_read_back() {
    let bytes = lifted_bundle();
    let packages = package_input([bytes.as_slice()]).expect("the lifted package enters I2");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile S = \"quire.value.complete/v1\";\n\
         profile P = \"quire.protocol.finite-global/v1\";\n\
         model M = \"{PACKAGE}\";\n\
         protocol Plumbing using P over (view: M::Sys) on origin {{\n\
             role Operator on M::Sys;\n\
             role Outlet on M::pump_out;\n\
             role Inlet on M::tank_in;\n\
             run sequence Main {{ check Valid using S {{ true }}; }}\n\
             finish Closed as (closed: M::Sys) {{ true }};\n\
         }}\n"
    );
    let source = SourceIdentity::new("agent-ix", "test:tc-1062-port-role");
    let cancel = Cancel::new();
    let limits = qsl_replay::spine::SpineLimits::default();
    let failure = |failure| {
        qsl_replay::spine::refusal_or_fault(failure).expect("no internal front-end fault")
    };
    let parsed = qsl_replay::spine::parse(
        &qsl_replay::spine::ParseRequest {
            source: &source,
            path: "port-role.native",
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
        .expect("model M is selected")
        .into_value();
    let checked = qsl_replay::spine::check(
        &parsed,
        "tc-1062-port-role".to_owned(),
        &models,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(failure)
    .expect("the roles check on the spine")
    .into_value();

    let plumbing = checked
        .package()
        .graph()
        .protocols()
        .iter()
        .find(|protocol| protocol.name == "Plumbing")
        .expect("Plumbing is checked");
    let checked_port = |name: &str| {
        plumbing
            .roles
            .iter()
            .find(|role| role.name == name)
            .unwrap_or_else(|| panic!("checked role {name}"))
            .model_member
            .clone()
    };
    let port = |artifact: &str| quire_checked::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: format!("ix://{PACKAGE}/{artifact}"),
    };
    assert_eq!(checked_port("Outlet"), Some(port("pump_out")));
    assert_eq!(checked_port("Inlet"), Some(port("tank_in")));
    assert_eq!(checked_port("Operator"), None);

    let emitted = qsl_package::emit_checked(checked.package(), qsl_package::EmitLimits::fixture())
        .expect("the checked unit emits");
    let read = crate::emit_checked_v2::read_back(emitted.package().bytes(), &[bytes.as_slice()]);
    let schemas = qsl_package::protocol_occurrence_schemas_from_read(&read)
        .expect("read-back occurrence schema");
    let in_process = checked
        .package()
        .protocol_occurrence_schemas()
        .expect("in-process occurrence schema");
    assert_eq!(in_process, schemas, "in-process and read-back agree");
    let role = |name: &str| {
        schemas
            .iter()
            .flat_map(|occurrence| &occurrence.schema.roles)
            .find(|role| role.path.last().map(String::as_str) == Some(name))
            .unwrap_or_else(|| panic!("read-back role {name}"))
            .clone()
    };
    let member = |artifact: &str| {
        Some(qsl_package::RoleModelMember {
            package: PACKAGE.to_owned(),
            node: format!("ix://{PACKAGE}/{artifact}"),
        })
    };
    let (outlet, inlet, operator) = (role("Outlet"), role("Inlet"), role("Operator"));
    assert_eq!(outlet.model_member, member("pump_out"));
    assert_eq!(inlet.model_member, member("tank_in"));
    assert_eq!(operator.model_member, None);
    assert_eq!(outlet.model, operator.model, "both are on Sys");
    assert_ne!(outlet.key, inlet.key, "each Port role is its own node");
}

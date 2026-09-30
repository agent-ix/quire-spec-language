// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-326 (ADR-011 §2.3 "Proof-stage acceptance", FB-10): the QSL-owned
//! claimed-module discharge-count read-back and mutation control for the
//! function-application Kani exemplar (ARCH-30/QSL-22; evidence: CG's
//! `tc_027_a_routed_scalar_harness_verifies`, CG PRs #181/#183, a real
//! Kani 0.67.0 Verified run and a real Falsified counterexample run).
//!
//! Peter's ruling (QSL-20 comment, 2026-09-29): CG's own architecture
//! correctly removed the claimed-module proof gate from
//! `quire-contract-codegen` per CG's no-tracking/no-pins policy. FB-10
//! assigns that check to "§2.3 gate owners" (QSL-20/#219), not CG. This
//! module is that QSL-owned spine test, scoped to the function-application
//! exemplar only (QSL-21's frame-effect proofs over an object's pre/post
//! population, exercised by `configversion_backends.rs`, are already Done
//! and out of scope here).
//!
//! Reuses `tests/it/configversion_backends.rs`'s technique for running real
//! `cargo kani` in a throwaway single-crate workspace with no
//! `rust-version` pin, sidestepping the Kani-0.67.0-vs-1.98 toolchain
//! conflict tracked separately as QSL-130 (the unrelated `quire-exact`
//! kernel gate; out of scope here).
//!
//! The exemplar: `amount < 1000 implies amount + 1 <= 1000`, a
//! function-application invariant over one scalar input -- applying `+` and
//! a comparison directly to a value (QSL-22's shape), rather than QSL-21's
//! object pre/post frame. `generate_kani_bundle`'s compiled postcondition
//! (`oracle_populationrule_7_population_rule`) renders the clause's
//! `implies` connective as a call to
//! `quire_contract_runtime::operators::implies_short_circuit` -- the one
//! real, externally-versioned helper the compiled proof expectation
//! depends on (confirmed by inspecting the generated source; asserted
//! directly in `mutating_the_compiled_postconditions_connective_helper_fails_the_proof`
//! below). This clause's ABI derives no subject-consumed result (FR-015):
//! the postcondition is a pure fact over the input, with no operation
//! output for a hand-written subject to produce, so there is no helper
//! *shared with the subject side* to mutate under ADR-011 §2.3 rule 3 --
//! an empty set, which ADR-011 §2.3 states is stated as empty. The
//! mutation control below instead satisfies rule 2 ("At least one
//! mutation control injected inside that module turns the gate red"): it
//! mutates the shared connective helper the compiled expectation calls and
//! shows the exact same, unmutated subject flips from Verified to
//! Falsified. This closes the gap the ticket names: nothing before this
//! test exercised whether that generated module's own logic -- as opposed
//! to a hand-written subject -- is load-bearing.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use ix_trace_rs::trace;
use qsl_replay::{ProofCategory, TerminalRecord, TerminalValue};
use quire_contract_ir_historical as backend_ir;
use quire_spec_language::{
    lowering::{lower_for, LoweringLimits, ProjectionTarget},
    native_model::NativeModel,
    package::{NativePackage, PackageLimits},
    syntax::ClauseKind,
};
use serde_json::json;

use crate::support::runtime_setup as setup;

const TARGET: ProjectionTarget = ProjectionTarget::IntegerIrV1;
const RUNTIME_GIT_URL: &str = "https://github.com/agent-ix/quire-contract-runtime";

/// The one claimed module for this exemplar's one proposition (ADR-011
/// §2.3 rule 1): the compiled postcondition oracle Kani discharges checks
/// against.
const CLAIMED_MODULES: [&str; 1] = ["PopulationRule@7/population_rule"];

/// `quire_contract_runtime::operators::implies_short_circuit`'s exact
/// current source, matched before mutating it so a future upstream edit
/// fails loudly here instead of silently mutating nothing.
const CONNECTIVE_HELPER_HEALTHY: &str = "pub fn implies_short_circuit<R: From<bool>>(antecedent: bool, consequent: impl FnOnce() -> R) -> R {\n    if antecedent {\n        consequent()\n    } else {\n        R::from(true)\n    }\n}";

/// The mutation control: unconditionally false, ignoring both operands.
const CONNECTIVE_HELPER_MUTATED: &str = "pub fn implies_short_circuit<R: From<bool>>(_antecedent: bool, _consequent: impl FnOnce() -> R) -> R {\n    R::from(false)\n}";

fn model() -> NativeModel {
    setup::authored_model(|model| {
        model["values"].as_array_mut().unwrap().push(json!({
            "name": "amount",
            "kind": "state",
            "type": {"kind": "scalar", "name": "Version"},
        }));
    })
}

fn package(models: &[NativeModel]) -> NativePackage<'_> {
    NativePackage::new(
        setup::checked_kind(
            models,
            "amount < 1000 implies amount + 1 <= 1000",
            ClauseKind::Invariant,
        ),
        PackageLimits::default(),
    )
    .unwrap()
}

fn population_rule_clause(bound: &backend_ir::BoundPackage) -> &backend_ir::BoundClause {
    bound
        .clauses()
        .iter()
        .find(|clause| clause.identity().clause().as_str() == "population_rule")
        .unwrap()
}

fn kani_bundle(
    bound_clause: &backend_ir::BoundClause,
) -> quire_contract_codegen::KaniArtifactBundle {
    let true_source = backend_ir::Expression::new(
        backend_ir::ExpressionKind::BooleanLiteral { value: true },
        bound_clause.source().clone(),
    );
    let precondition = bound_clause
        .environment()
        .check_expression(
            &true_source,
            &backend_ir::ValueType::Boolean,
            bound_clause.anchor(),
            true,
        )
        .expect("canonical absent precondition is a zero-dependency Boolean true");
    let precondition_clause = backend_ir::ClauseId::new("amount_absent_precondition").unwrap();
    quire_contract_codegen::generate_kani_bundle(&quire_contract_codegen::KaniRequest {
        requirement: bound_clause.identity().requirement(),
        precondition_clause: &precondition_clause,
        postcondition_clause: bound_clause.identity().clause(),
        precondition: &precondition,
        postcondition: bound_clause.expression(),
        proof_id: "qsl-326-function-application",
        subject_path: "crate::subject",
        unwind: 2,
        solver: quire_contract_codegen::KaniSolver::Cadical,
        dependencies: &[],
    })
    .unwrap()
}

/// Runs `bundle` plus `subject` under real `cargo kani`, in a throwaway
/// single-crate workspace with no `rust-version` pin
/// (`tests/it/configversion_backends.rs`'s technique). `runtime_patch`,
/// when given, overrides the `quire-contract-runtime` dependency with a
/// local, mutated copy -- the ADR-011 §2.3 rule 2 mutation control.
fn execute_kani(
    bundle: &quire_contract_codegen::KaniArtifactBundle,
    subject: &str,
    runtime_patch: Option<&Path>,
) -> std::process::Output {
    let graph: quire_contract_codegen::ProofDependencyGraph =
        serde_json::from_str(&bundle.proof_graph.contents).unwrap();
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("src/lib.rs"),
        format!("{}\n{subject}\n", bundle.rust.contents),
    )
    .unwrap();
    let patch = runtime_patch
        .map(|path| {
            format!(
                "\n[patch.\"{RUNTIME_GIT_URL}\"]\nquire-contract-runtime = {{ path = {path:?} }}\n"
            )
        })
        .unwrap_or_default();
    fs::write(
        directory.path().join("Cargo.toml"),
        format!(
            // QSL-327: quire-contract-runtime's own `#[cfg(kani)]` internal
            // proof harness (`verification/kani.rs`) unconditionally imports
            // `crate::exact`, so a `cargo kani` build needs the "exact"
            // feature enabled regardless of what the generated oracle
            // itself needs (see `tests/it/configversion_backends.rs`).
            "[package]\nname = \"function-application-kani\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\nquire-contract-runtime = {{ git = \"{RUNTIME_GIT_URL}\", rev = \"{}\", features = [\"exact\"] }}\n\n[workspace]\n{patch}",
            quire_contract_codegen::RUNTIME_REVISION
        ),
    )
    .unwrap();
    fs::write(
        directory.path().join("build.rs"),
        "fn main() { println!(\"cargo:rustc-check-cfg=cfg(kani)\"); }\n",
    )
    .unwrap();
    Command::new("cargo")
        .arg("kani")
        .args(&graph.options)
        .env(
            "CARGO_TARGET_DIR",
            directory.path().join("target-codex-backends"),
        )
        .env("CARGO_NET_OFFLINE", "true")
        .current_dir(directory.path())
        .output()
        .expect("pinned cargo-kani must execute")
}

/// The generated Kani obligation's SUCCESS-check discharge count, read from
/// Kani's own `** <failed> of <total> failed[ (<n> unreachable)]` summary
/// line (Kani has no machine-readable verdict; this is the same prose CG's
/// own `kani_transcript.rs` parses). UNREACHABLE and UNDETERMINED checks
/// are excluded, matching ADR-011 §2.3 rule 1.
fn discharged_success_checks(output: &str) -> u64 {
    for line in output.lines() {
        let Some(rest) = line.trim().strip_prefix("** ") else {
            continue;
        };
        let (counts, qualifier) = match rest.split_once(" (") {
            Some((counts, paren)) => (counts, paren.trim_end_matches(')')),
            None => (rest, ""),
        };
        let Some(counts) = counts.strip_suffix(" failed") else {
            continue;
        };
        let Some((failed, total)) = counts.split_once(" of ") else {
            continue;
        };
        let (Ok(failed), Ok(total)) = (failed.parse::<u64>(), total.parse::<u64>()) else {
            continue;
        };
        let excluded = if qualifier.contains("unreachable") || qualifier.contains("undetermined") {
            qualifier
                .split_whitespace()
                .next()
                .and_then(|count| count.parse::<u64>().ok())
                .unwrap_or(0)
        } else {
            0
        };
        return total.saturating_sub(failed).saturating_sub(excluded);
    }
    panic!("Kani transcript has no '** <failed> of <total> failed' summary line:\n{output}");
}

fn cargo_home() -> PathBuf {
    env::var_os("CARGO_HOME").map_or_else(
        || {
            PathBuf::from(env::var_os("HOME").expect("HOME is set for the test process"))
                .join(".cargo")
        },
        PathBuf::from,
    )
}

/// Locates `quire-contract-runtime`'s already-fetched git checkout for
/// `revision` in cargo's global cache (populated the first time any test in
/// this suite builds a throwaway crate against it, offline runs thereafter
/// reuse it -- the same assumption `execute_kani`'s own `--offline`-style
/// `CARGO_NET_OFFLINE=true` already makes).
fn cached_runtime_checkout(revision: &str) -> PathBuf {
    let checkouts = cargo_home().join("git").join("checkouts");
    let short = &revision[..7];
    let entries = fs::read_dir(&checkouts).unwrap_or_else(|error| {
        panic!("cargo git checkout cache {checkouts:?} is unavailable: {error}")
    });
    for entry in entries {
        let candidate = entry.unwrap().path().join(short);
        let manifest_path = candidate.join("Cargo.toml");
        let Ok(manifest) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        if manifest.contains("name = \"quire-contract-runtime\"") {
            return candidate;
        }
    }
    panic!(
        "quire-contract-runtime@{revision} is not in the local cargo git cache at {checkouts:?}; \
         run a test that depends on it (e.g. configversion_backends) first"
    );
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A local copy of `quire-contract-runtime`'s sources with
/// `operators::implies_short_circuit` mutated to unconditionally return
/// `false`, and its manifest's `[workspace]` section (with its
/// `measurement/footprint` member) stripped, so this copy is a plain crate
/// with no nested workspace and carries no `rust-toolchain.toml` (QSL-130).
fn mutated_runtime_copy(revision: &str) -> tempfile::TempDir {
    let source = cached_runtime_checkout(revision);
    let directory = tempfile::tempdir().unwrap();
    // Keep the original manifest (features, dependencies, lints) so the
    // "exact" feature this dependency is built with (QSL-327) still
    // resolves; drop only `[workspace]`, so this copy is a plain crate with
    // no nested workspace and no `rust-toolchain.toml` pin (avoids QSL-130).
    let manifest = fs::read_to_string(source.join("Cargo.toml")).unwrap();
    let workspace_start = manifest
        .find("[workspace]")
        .expect("quire-contract-runtime's Cargo.toml declares [workspace]");
    let workspace_end = manifest[workspace_start..]
        .find("\n\n")
        .map(|offset| workspace_start + offset + 2)
        .unwrap_or(manifest.len());
    let manifest = format!(
        "{}{}",
        &manifest[..workspace_start],
        &manifest[workspace_end..]
    );
    assert!(
        !manifest.contains("[workspace]"),
        "failed to strip [workspace] from the copied manifest"
    );
    fs::write(directory.path().join("Cargo.toml"), manifest).unwrap();
    copy_tree(&source.join("src"), &directory.path().join("src"));
    // `src/lib.rs` pulls its `#[cfg(kani)]` internal proof harness in from a
    // sibling directory via `#[path = "../verification/kani.rs"]`.
    copy_tree(
        &source.join("verification"),
        &directory.path().join("verification"),
    );
    let operators_path = directory.path().join("src/operators.rs");
    let original = fs::read_to_string(&operators_path).unwrap();
    assert!(
        original.contains(CONNECTIVE_HELPER_HEALTHY),
        "quire-contract-runtime's operators::implies_short_circuit no longer matches this \
         mutation control's expected source; update CONNECTIVE_HELPER_HEALTHY/_MUTATED for the \
         new implementation"
    );
    fs::write(
        &operators_path,
        original.replace(CONNECTIVE_HELPER_HEALTHY, CONNECTIVE_HELPER_MUTATED),
    )
    .unwrap();
    directory
}

const HEALTHY_SUBJECT: &str = "pub fn subject(_amount_current: i64) {}";

/// ADR-011 §2.3 rule 1: per claimed module, the prover transcript shows at
/// least one discharged SUCCESS check, and reading `TerminalValue::Proved`
/// back against the claimed-module list gives `ProofCategory::Success`.
/// `qsl_replay::proof_result::TerminalValue::Proved{success_checks}`
/// already carries this per item; nothing before this test read it back
/// against a claimed-module list for the function-application exemplar.
#[test]
#[trace("FR-069-AC-1")]
fn healthy_run_discharges_a_success_check_for_every_claimed_module() {
    let models = [model()];
    let native = package(&models);
    let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
    let consumer = backend_ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap();
    let bundle = kani_bundle(population_rule_clause(&consumer));

    let healthy = execute_kani(&bundle, HEALTHY_SUBJECT, None);
    let output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&healthy.stdout),
        String::from_utf8_lossy(&healthy.stderr)
    );
    assert!(
        healthy.status.success() && output.contains("VERIFICATION:- SUCCESSFUL"),
        "function-application proof did not verify:\n{output}"
    );
    let success_checks = u32::try_from(discharged_success_checks(&output)).unwrap();

    for module in CLAIMED_MODULES {
        let record = TerminalRecord::new(module, TerminalValue::Proved { success_checks });
        assert_eq!(
            record.value().category(),
            ProofCategory::Success,
            "claimed module {module}'s discharge count did not read back as success"
        );
        let TerminalValue::Proved {
            success_checks: discharged,
        } = record.value()
        else {
            unreachable!("record was constructed as Proved above")
        };
        assert!(
            discharged > 0,
            "claimed module {module} discharged zero SUCCESS checks (ADR-011 §2.3: unreached)"
        );
    }
}

/// ADR-011 §2.3 rule 2: a mutation control injected inside the claimed
/// module turns the gate red. `implies_short_circuit` is the one real,
/// externally-versioned helper the compiled postcondition
/// (`oracle_populationrule_7_population_rule`) calls to render this
/// clause's `implies`; mutating it, with the exact same unmutated subject,
/// must flip the discharge from Verified to Falsified. This is the check
/// the ticket's gap describes: the existing healthy/violating *subject*
/// pair never exercises whether the generated module's own logic is
/// load-bearing, so a broken connective helper -- one that always answers
/// `false`, i.e. a constant-`false` oracle -- would otherwise pass
/// unnoticed as long as no test ever mutates it.
#[test]
#[trace("FR-069-AC-1")]
fn mutating_the_compiled_postconditions_connective_helper_fails_the_proof() {
    let models = [model()];
    let native = package(&models);
    let projection = lower_for(&native, TARGET, LoweringLimits::default()).unwrap();
    let consumer = backend_ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap();
    let bundle = kani_bundle(population_rule_clause(&consumer));
    assert!(
        bundle
            .rust
            .contents
            .contains("quire_contract_runtime::operators::implies_short_circuit("),
        "the compiled postcondition no longer calls the connective helper this mutation targets; \
         update this test for the new generated shape"
    );

    let unmutated = execute_kani(&bundle, HEALTHY_SUBJECT, None);
    assert!(
        unmutated.status.success(),
        "control run must verify before mutating:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&unmutated.stdout),
        String::from_utf8_lossy(&unmutated.stderr)
    );

    let mutated_runtime = mutated_runtime_copy(quire_contract_codegen::RUNTIME_REVISION);
    let mutated = execute_kani(&bundle, HEALTHY_SUBJECT, Some(mutated_runtime.path()));
    let output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&mutated.stdout),
        String::from_utf8_lossy(&mutated.stderr)
    );
    assert!(
        !mutated.status.success() && output.contains("VERIFICATION:- FAILED"),
        "mutating quire_contract_runtime::operators::implies_short_circuit did not fail the \
         proof -- the connective helper is not load-bearing for this exemplar:\n{output}"
    );
}

// SPDX-License-Identifier: AGPL-3.0-only
//! IT-008: actual generated Rust and measured coverage against native execution.

// This shared fixture module also serves operation/graph tests in other binaries.
#[allow(dead_code)]
#[path = "support/runtime_setup.rs"]
mod setup;

use std::{fs, path::PathBuf, process::Command};

use ix_trace_rs::trace;
use quire_contract_codegen as codegen;
use quire_contract_ir_backend as backend_ir;
use quire_spec_language::lowering::{lower, LoweringLimits};
use quire_spec_language::package::{NativePackage, PackageLimits};
use quire_spec_language::runtime::{
    evaluate, validate, EvaluationLimits, EvaluationOutcome, ImplicationEventKind, QualifiedName,
    ValidationLimits, ValueBinding, ValueId, ValueNode,
};
use serde_json::{json, Value};

fn run(command: &mut Command) -> Vec<u8> {
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("cannot run {command:?}: {error}"));
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
#[trace("TC-094", "FR-009-AC-5")]
fn generated_truth_matches_all_assignments_and_retains_unsupported_coverage() {
    run_cases(false);
}

#[test]
#[trace("TC-094", "FR-009-AC-5")]
#[ignore = "required LC04 activation gate: codegen 240fad84 rejects LLVM 3.1.0; C handoff on CO01"]
fn required_generated_activation_parity() {
    run_cases(true);
}

fn run_cases(require_activation: bool) {
    // Left-antecedent implication, outer implication, then right-consequent
    // implication: this catches source-map order accidentally using preorder.
    const EXPRESSION: &str = "(a implies b) implies (c implies (not a or not b and c))";
    let models = [setup::authored_model(|model| {
        for name in ["a", "b", "c"] {
            model["values"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":name,"kind":"state","type":{"kind":"boolean"}}));
        }
    })];
    let package = NativePackage::new(
        setup::checked(&models, EXPRESSION),
        PackageLimits::default(),
    )
    .unwrap();
    let projection = lower(&package, LoweringLimits::default()).unwrap();
    // Consume bytes through the real older consumer, without changing production
    // IR types or treating a Serde value conversion as a binding check.
    let consumer = backend_ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap();
    assert_eq!(
        consumer.digest().to_string(),
        projection.bound().digest().to_string()
    );
    let generated = codegen::generate_bound_oracles(
        &consumer,
        codegen::AttestationContext {
            // Explicit synthetic context: generation bodies are not sealed attestations.
            record_digest: &"0".repeat(64),
            candidate_revision: &"0".repeat(40),
        },
    )
    .unwrap();
    let codegen::BoundOracleGeneration::Generated(generated_clauses) = &generated else {
        panic!("complete executable population required")
    };
    assert_eq!(generated_clauses.clauses().len(), 2);
    let root = tempfile::tempdir().unwrap();
    let source_root = root.path().join("package");
    codegen::write_bundle_atomic(generated_clauses.bundle(), &source_root).unwrap();
    let mut program = String::from("// SPDX-License-Identifier: AGPL-3.0-only\n");
    let mut calls = Vec::new();
    for (index, clause) in generated_clauses.clauses().iter().enumerate() {
        let artifact = &clause.bundle().rust;
        let syntax = syn::parse_file(&artifact.contents).unwrap();
        let functions = syntax
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function) if matches!(function.vis, syn::Visibility::Public(_)) => {
                    Some(function)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(functions.len(), 1);
        let function = functions[0];
        let arguments = function
            .sig
            .inputs
            .iter()
            .map(|argument| {
                let syn::FnArg::Typed(argument) = argument else {
                    panic!("free oracle function")
                };
                let syn::Pat::Ident(name) = argument.pat.as_ref() else {
                    panic!("named Boolean parameter")
                };
                match name.ident.to_string().as_str() {
                    "a_current" => "a",
                    "b_current" => "b",
                    "c_current" => "c",
                    name => panic!("unexpected generated parameter {name}"),
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        program.push_str(&format!(
            "#[allow(dead_code)] #[path = {:?}] mod clause_{index};\n",
            artifact.path.strip_prefix("src/").unwrap()
        ));
        calls.push(format!(
            "clause_{index}::{}({arguments})",
            function.sig.ident
        ));
        let map: Vec<codegen::SourceRegion> =
            serde_json::from_str(&clause.bundle().source_map.contents).unwrap();
        assert!(map
            .iter()
            .all(|region| region.package_id == "example/runtime-rules"
                && region.requirement_id == "PopulationRule"
                && region.requirement_revision == 7
                && region.clause_id == clause.identity().clause().as_str()));
    }
    program.push_str(&format!("fn main() {{ let mut args = std::env::args().skip(1); let a: bool = args.next().unwrap().parse().unwrap(); let b: bool = args.next().unwrap().parse().unwrap(); let c: bool = args.next().unwrap().parse().unwrap(); println!(\"[{{}},{{}}]\", {}, {}); }}\n", calls[0], calls[1]));
    fs::write(source_root.join("src/main.rs"), program).unwrap();
    fs::write(
        source_root.join("Cargo.toml"),
        include_bytes!("fixtures/native-lowering/Cargo.toml"),
    )
    .unwrap();
    fs::write(
        source_root.join("Cargo.lock"),
        include_bytes!("fixtures/native-lowering/Cargo.lock"),
    )
    .unwrap();

    let version = String::from_utf8(run(Command::new("rustc").arg("-vV"))).unwrap();
    assert!(
        version.starts_with("rustc 1.98.1 "),
        "qualification requires the adopted compiler: {version}"
    );
    let host = version
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .unwrap();
    let sysroot =
        String::from_utf8(run(Command::new("rustc").args(["--print", "sysroot"]))).unwrap();
    let llvm = PathBuf::from(sysroot.trim())
        .join("lib/rustlib")
        .join(host)
        .join("bin");
    for tool in ["llvm-cov", "llvm-profdata"] {
        assert!(
            llvm.join(tool).is_file(),
            "install llvm-tools-preview for Rust 1.98.1"
        );
    }
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/lc04-generated");
    run(Command::new(env!("CARGO"))
        .args(["build", "--locked", "--offline", "-j", "1", "--target-dir"])
        .arg(&target)
        .current_dir(&source_root)
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env("RUSTFLAGS", "-C instrument-coverage"));
    let executable = target.join("debug/native-boolean-parity");
    assert_eq!(
        String::from_utf8(run(
            Command::new(env!("CARGO")).args(["llvm-cov", "--version"])
        ))
        .unwrap()
        .trim(),
        "cargo-llvm-cov 0.9.0"
    );
    // Isolate this run's measured binary and profiles from the reusable build
    // cache so stale executables/profiles cannot participate in the report.
    let report_target = root.path().join("coverage");
    fs::create_dir_all(report_target.join("debug")).unwrap();
    fs::copy(
        &executable,
        report_target.join("debug/native-boolean-parity"),
    )
    .unwrap();
    // Re-read the actual published bytes used by compilation, including maps and
    // unsealed generation bodies, before asking the existing coverage consumer.
    let artifacts = generated_clauses
        .bundle()
        .artifacts()
        .iter()
        .map(|artifact| {
            (
                artifact.path.clone(),
                fs::read(source_root.join(&artifact.path)).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let artifacts = artifacts
        .iter()
        .map(|(path, bytes)| codegen::ArtifactBytes { path, bytes })
        .collect::<Vec<_>>();
    for bits in 0..8 {
        let [a, b, c] = [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0];
        // Direct independent truth/control equations for this fixed fixture.
        let antecedent = !a || b;
        let expected = !antecedent || !c || !a || !b && c;
        let expected_entries = [
            u64::from(a),
            u64::from(antecedent),
            u64::from(antecedent && c),
        ];
        let mut draft = setup::draft(&models[0]);
        for (name, value) in [("a", a), ("b", b), ("c", c)] {
            let id = ValueId::new(u32::try_from(draft.arena.len()).unwrap());
            draft.arena.push(ValueNode::Boolean { value });
            draft.values.push(ValueBinding {
                declaration: QualifiedName {
                    model: models[0].environment().owner().clone(),
                    name: setup::symbol(name),
                },
                value: id,
            });
        }
        let snapshot = setup::snapshot(draft);
        let selected = setup::selection(&models[0], snapshot.reference());
        let context = validate(
            package.checked(),
            setup::input(snapshot),
            selected,
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        let native = evaluate(&context, EvaluationLimits::default(), || false);
        assert_eq!(
            native.outcome(),
            &EvaluationOutcome::Completed(expected),
            "assignment {bits}"
        );
        let mut entries = [0_u64; 3];
        let unit = package.checked().linked().unit();
        for event in native
            .events()
            .iter()
            .filter(|event| event.kind == ImplicationEventKind::ConsequentEntered)
        {
            let expression = unit.expression(event.implication).unwrap();
            let ordinal = match &unit.source().text()[expression.span.start..expression.span.end] {
                "a implies b" => 0,
                EXPRESSION => 1,
                "c implies (not a or not b and c)" => 2,
                other => panic!("foreign implication source: {other}"),
            };
            entries[ordinal] += 1;
        }
        assert_eq!(
            entries, expected_entries,
            "native activation assignment {bits}"
        );

        let raw = report_target.join("case.profraw");
        let output = run(Command::new(&executable)
            .args([a.to_string(), b.to_string(), c.to_string()])
            .env("LLVM_PROFILE_FILE", &raw));
        assert_eq!(
            serde_json::from_slice::<Vec<bool>>(&output).unwrap(),
            [true, expected]
        );
        let export = run(Command::new(env!("CARGO"))
            .args(["llvm-cov", "report", "--json", "--locked", "--offline"])
            .current_dir(&source_root)
            .env("CARGO_LLVM_COV_TARGET_DIR", &report_target)
            .env("LLVM_COV", llvm.join("llvm-cov"))
            .env("LLVM_PROFDATA", llvm.join("llvm-profdata"))
            .env("LLVM_PROFDATA_FLAGS", "-num-threads=1")
            .env("CARGO_BUILD_JOBS", "1"));
        fs::remove_file(&raw).unwrap();
        let report = codegen::analyze_bound_coverage(
            &consumer,
            &generated,
            codegen::BoundCoverageInputs {
                source_root: source_root.to_str().unwrap(),
                artifacts: &artifacts,
                llvm_export: Some(&export),
            },
        );
        let report: Value = serde_json::from_slice(&report.to_json_bytes().unwrap()).unwrap();
        if !require_activation {
            // Retain the real downstream refusal while truth parity progresses.
            // The separate required activation gate stays visibly pending.
            assert_eq!(
                report["state"], "unsupported",
                "update the qualification when the backend changes: {report}"
            );
            assert_eq!(report["diagnostics"][0]["code"], "unsupported_profile");
            assert_eq!(report["diagnostics"][0]["message"], "expected cargo-llvm-cov 0.9.0 / llvm.coverage.json.export 3.0.1; found 0.9.0 / llvm.coverage.json.export 3.1.0");
            continue;
        }
        assert_eq!(report["state"], "complete", "assignment {bits}: {report}");
        assert_eq!(report["provenance"], "unqualified");
        let clauses = report["clauses"].as_array().unwrap();
        assert_eq!(clauses.len(), 2);
        assert!(clauses.iter().all(|clause| clause["evaluation_count"] == 1));
        let counts = clauses[1]["consequents"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["count"].as_u64().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(counts, entries, "generated activation assignment {bits}");
    }
}

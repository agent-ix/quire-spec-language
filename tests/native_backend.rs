// SPDX-License-Identifier: AGPL-3.0-or-later
//! IT-008: actual generated Rust and measured coverage against native execution.

// This shared fixture module also serves operation/graph tests in other binaries.
#[allow(dead_code)]
#[path = "support/runtime_setup.rs"]
mod setup;

use std::{fs, path::Path, path::PathBuf, process::Command};

use ix_trace_rs::trace;
use quire_contract_codegen as codegen;
use quire_contract_ir_historical as backend_ir;
use quire_spec_language::lowering::{lower, LoweringLimits};
use quire_spec_language::package::{NativePackage, PackageLimits};
use quire_spec_language::runtime::{
    evaluate, validate, EvaluationLimits, EvaluationOutcome, ImplicationEventKind, QualifiedName,
    ValidationLimits, ValueBinding, ValueId, ValueNode,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
struct Llvm31Export {
    #[serde(rename = "type")]
    kind: String,
    version: String,
    cargo_llvm_cov: Llvm31Producer,
    data: Vec<Llvm31Data>,
}

#[derive(Deserialize)]
struct Llvm31Producer {
    version: String,
    manifest_path: String,
}

#[derive(Deserialize)]
struct Llvm31Data {
    files: Vec<Llvm31File>,
}

#[derive(Deserialize)]
struct Llvm31File {
    filename: String,
    segments: Vec<(u32, u32, u64, bool, bool, bool)>,
}

impl Llvm31Export {
    fn parse(bytes: &[u8], source_root: &Path) -> Self {
        assert!(bytes.len() <= codegen::MAX_COVERAGE_BYTES);
        let export: Self = serde_json::from_slice(bytes).unwrap();
        assert_eq!(export.kind, "llvm.coverage.json.export");
        assert_eq!(export.version, "3.1.0");
        assert_eq!(export.cargo_llvm_cov.version, "0.9.0");
        assert_eq!(
            Path::new(&export.cargo_llvm_cov.manifest_path),
            source_root.join("Cargo.toml")
        );
        assert!(!export.data.is_empty());
        export
    }

    fn probe_count(&self, source_root: &Path, region: &codegen::SourceRegion) -> u64 {
        let expected = source_root.join(&region.artifact_path);
        let file = self
            .data
            .iter()
            .flat_map(|data| &data.files)
            .find(|file| Path::new(&file.filename) == expected)
            .unwrap_or_else(|| panic!("missing generated coverage file {}", expected.display()));
        let probe = region.probe.expect("generated semantic probe");
        let start = (probe.line, probe.start_column);
        let end = (probe.line, probe.end_column);
        file.segments
            .windows(2)
            .find_map(|pair| {
                let segment = pair[0];
                let next = pair[1];
                ((segment.0, segment.1) <= start
                    && end <= (next.0, next.1)
                    && segment.3
                    && !segment.5)
                    .then_some(segment.2)
            })
            .unwrap_or_else(|| {
                panic!(
                    "unmeasured generated probe {}:{}:{}",
                    region.artifact_path, probe.line, probe.start_column
                )
            })
    }
}

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
fn generated_proptest_and_old_profile_activation_match_reference() {
    run_cases();
}

fn run_cases() {
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
    let unit = package.checked().linked().unit();
    assert_eq!(unit.language().value, "ix:native");
    assert_eq!(unit.edition().value, "0-draft");
    assert_eq!(
        unit.source().text().lines().nth(1),
        Some("profile \"state-finite/0-draft\";")
    );
    let projection = lower(&package, LoweringLimits::default()).unwrap();
    assert_eq!(projection.profile(), "boolean-oracle/v1");
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
    let strategy = codegen::generate_i64_strategy(&codegen::StrategyRequest {
        requirement: generated_clauses.clauses()[0].identity().requirement(),
        strategy_id: "boolean-oracle-v1-complete-domain",
        constraint: codegen::StrategyConstraint::Membership {
            values: &[0, 1, 2, 3, 4, 5, 6, 7],
        },
        campaign: codegen::StrategyCampaign::Broad,
        attestation: codegen::AttestationContext {
            record_digest: &"0".repeat(64),
            candidate_revision: &"0".repeat(40),
        },
    })
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let source_root = root.path().join("package");
    codegen::write_bundle_atomic(generated_clauses.bundle(), &source_root).unwrap();
    fs::write(
        source_root.join(&strategy.rust.path),
        &strategy.rust.contents,
    )
    .unwrap();
    let mut program = String::from("// SPDX-License-Identifier: AGPL-3.0-or-later\n");
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
    let strategy_syntax = syn::parse_file(&strategy.rust.contents).unwrap();
    let strategy_function = strategy_syntax
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if matches!(function.vis, syn::Visibility::Public(_)) => {
                Some(function.sig.ident.to_string())
            }
            _ => None,
        })
        .expect("generated public strategy function");
    program.push_str(&format!(
        "#[path = {:?}] mod boolean_oracle_v1_domain;\n",
        strategy.rust.path.strip_prefix("src/").unwrap()
    ));
    program.push_str(&format!(
        r#"fn evaluate(a: bool, b: bool, c: bool) -> [bool; 2] {{ [{}, {}] }}
fn main() {{
    let values = std::env::args().skip(1).collect::<Vec<_>>();
    if values.is_empty() {{
        use proptest::strategy::Strategy as _;
        let mut runner = proptest::test_runner::TestRunner::deterministic();
        let strategy = boolean_oracle_v1_domain::{strategy_function}();
        runner.run(&strategy, |case| {{
            proptest::prop_assert!(!case.expected.expects_rejection());
            proptest::prop_assert_eq!(case.related, None);
            let bits = u8::try_from(case.primary).unwrap();
            let [a, b, c] = [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0];
            let [constant, rule] = evaluate(a, b, c);
            println!("{{bits}} {{constant}} {{rule}}");
            Ok(())
        }}).unwrap();
        return;
    }}
    assert_eq!(values.len(), 3);
    let a: bool = values[0].parse().unwrap();
    let b: bool = values[1].parse().unwrap();
    let c: bool = values[2].parse().unwrap();
    println!("{{:?}}", evaluate(a, b, c));
}}
"#,
        calls[0], calls[1]
    ));
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
    let property_raw = root.path().join("property.profraw");
    let property_output = run(Command::new(&executable).env("LLVM_PROFILE_FILE", &property_raw));
    fs::remove_file(property_raw).unwrap();
    let mut property_truth = [None; 8];
    for line in String::from_utf8(property_output).unwrap().lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        assert_eq!(fields.len(), 3);
        let bits = fields[0].parse::<usize>().unwrap();
        assert!(bits < property_truth.len());
        assert!(fields[1].parse::<bool>().unwrap());
        let truth = fields[2].parse::<bool>().unwrap();
        if let Some(previous) = property_truth[bits] {
            assert_eq!(previous, truth, "generated proptest assignment {bits}");
        }
        property_truth[bits] = Some(truth);
    }
    assert!(property_truth.iter().all(Option::is_some));
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
    for (bits, property_expected) in property_truth.into_iter().enumerate() {
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
        assert_eq!(
            property_expected,
            Some(expected),
            "generated proptest assignment {bits}"
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
        let exact_profile = Llvm31Export::parse(&export, &source_root);
        let rule_map: Vec<codegen::SourceRegion> =
            serde_json::from_str(&generated_clauses.clauses()[1].bundle().source_map.contents)
                .unwrap();
        let generated_entries = rule_map[2..]
            .iter()
            .map(|region| exact_profile.probe_count(&source_root, region))
            .collect::<Vec<_>>();
        assert_eq!(
            generated_entries, entries,
            "generated activation assignment {bits}"
        );
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
        // Preserve the downstream capability boundary: this named fixture can
        // qualify LLVM 3.1.0, but the reusable pinned reader has not adopted it.
        assert_eq!(
            report["state"], "unsupported",
            "update the qualification when the backend changes: {report}"
        );
        assert_eq!(report["diagnostics"][0]["code"], "unsupported_profile");
        let message = report["diagnostics"][0]["message"].as_str().unwrap();
        assert!(message.contains("3.0.1") && message.contains("3.1.0"));
    }
}

// SPDX-License-Identifier: AGPL-3.0-only
//! IT-010: actual ConfigVersion parity across native and generated numeric backends.

#[allow(dead_code)]
#[path = "../examples/config-version/fixtures.rs"]
mod config;

use std::{cell::Cell, env, fmt::Write as _, fs, path::Path, path::PathBuf, process::Command};

use ix_trace_rs::trace;
use quire_contract_codegen as codegen;
use quire_contract_ir as ir;
use quire_spec_language::{
    checking::{check, CheckBindings, CheckLimits, ClauseBinding},
    formal_source::FormalSource,
    link_native,
    lowering::{lower_for, LoweringCode, LoweringLimits, ProjectionTarget},
    native_model::NativeModel,
    package::{NativePackage, PackageLimits},
    parse,
    runtime::{
        self, ArtifactLimits, ExecutionLimits, ExecutionSelection, FieldBinding, Invocation,
        InvocationDraft, ModelBinding, ObjectEntry, ObjectIdentity, ObservationSelection,
        Population, QualifiedName, RuntimeInput, Snapshot, SnapshotDraft, ValueId, ValueNode,
    },
    Limits, LinkLimits, SourceIdentity,
};
use serde_json::Value;
use sha2::Digest as _;

const CODEGEN_REVISION: &str = "5e2a6a994d2107f36294078ad202467a4c66bb75";
const IR_REVISION: &str = "04eb6f849c03be23177d373549c6c272551f957d";
const KANI_SHA256: &str = "7f143a251d11c7e6e232bbf2cbccf56f9ce66a5f0107eeb3008698e6715f55d9";
const DOMAIN: std::ops::RangeInclusive<i64> = 0..=1000;
const CORPUS: [i64; 4] = [-1, 0, 1000, 1001];

fn symbol(name: &str) -> ir::SymbolName {
    ir::SymbolName::new(name).expect("static ConfigVersion symbol")
}

fn compile_config<'m>(
    directory: &Path,
    models: &'m [NativeModel],
    case: config::Case,
) -> NativePackage<'m> {
    config::write(directory, &models[0], case).unwrap();
    let job: Value =
        serde_json::from_slice(&fs::read(directory.join("request.json")).unwrap()).unwrap();
    let request = &job["request"];
    let program = &request["program"];
    let source = &program["source"];
    let unit = parse(
        SourceIdentity {
            identity: source["identity"].as_str().unwrap().into(),
            revision: source["revision"].as_str().unwrap().into(),
        },
        "program.native",
        &fs::read(directory.join("program.native")).unwrap(),
        Limits::default(),
    )
    .unwrap();
    let formal = FormalSource::new(
        unit.source().clone(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new(source["document"].as_str().unwrap()).unwrap(),
            ir::SourceRevision::new(source["formal_revision"].as_u64().unwrap()).unwrap(),
        ),
    );
    let selected = &program["clauses"][0];
    let owner = &selected["owner"];
    let requirement = ir::RequirementRef::parse(
        owner["package"].as_str().unwrap(),
        owner["requirement"].as_str().unwrap(),
        owner["revision"].as_u64().unwrap(),
    )
    .unwrap();
    let clause = ir::ClauseId::new(selected["clause"].as_str().unwrap()).unwrap();
    let execution_point: ir::ExecutionPoint =
        serde_json::from_value(selected["point"].clone()).unwrap();
    let checked = check(
        link_native(unit, models, LinkLimits::default()).unwrap(),
        CheckBindings {
            source: formal,
            clauses: vec![ClauseBinding {
                name: selected["name"].as_str().unwrap().into(),
                requirement,
                clause,
                execution_point,
            }],
        },
        CheckLimits::default(),
    )
    .unwrap();
    NativePackage::new(checked, PackageLimits::default()).unwrap()
}

fn object(model: &NativeModel, key: &str) -> ObjectIdentity {
    ObjectIdentity {
        model: model.environment().owner().clone(),
        record: symbol("ConfigVersion"),
        universe: symbol("config_history"),
        key: key.into(),
    }
}

fn push(arena: &mut Vec<ValueNode>, value: ValueNode) -> ValueId {
    let id = ValueId::new(u32::try_from(arena.len()).unwrap());
    arena.push(value);
    id
}

fn snapshot(
    model: &NativeModel,
    observation: ir::StateObservation,
    version: i64,
    identity: &str,
) -> Snapshot {
    let mut arena = Vec::new();
    let root_version = push(&mut arena, ValueNode::Integer { value: 1 });
    let root_parent = push(&mut arena, ValueNode::Absent);
    let child_version = push(&mut arena, ValueNode::Integer { value: version });
    let parent_reference = push(
        &mut arena,
        ValueNode::Reference {
            identity: object(model, "root"),
        },
    );
    let child_parent = push(
        &mut arena,
        ValueNode::Present {
            value: parent_reference,
        },
    );
    let fields = |version, parent| {
        vec![
            FieldBinding {
                name: symbol("versionNumber"),
                value: version,
            },
            FieldBinding {
                name: symbol("parent"),
                value: parent,
            },
        ]
    };
    Snapshot::new(
        SourceIdentity {
            identity: identity.into(),
            revision: "1".into(),
        },
        SnapshotDraft {
            observation,
            models: vec![ModelBinding {
                model: model.environment().owner().clone(),
                digest: model.digest(),
            }],
            populations: vec![Population {
                model: model.environment().owner().clone(),
                record: symbol("ConfigVersion"),
                universe: symbol("config_history"),
                complete: true,
                objects: vec![
                    ObjectEntry {
                        key: "root".into(),
                        fields: fields(root_version, root_parent),
                    },
                    ObjectEntry {
                        key: "child".into(),
                        fields: fields(child_version, child_parent),
                    },
                ],
            }],
            values: Vec::new(),
            arena,
        },
        ArtifactLimits::default(),
    )
    .unwrap()
}

fn runtime_pair(
    native: &NativePackage<'_>,
    model: &NativeModel,
    pre_version: i64,
    post_version: i64,
) -> (RuntimeInput, ExecutionSelection) {
    let stem = format!("ix://it-010/config-version/{pre_version}/{post_version}");
    let pre = snapshot(
        model,
        ir::StateObservation::Pre,
        pre_version,
        &format!("{stem}/pre"),
    );
    let post = snapshot(
        model,
        ir::StateObservation::Post,
        post_version,
        &format!("{stem}/post"),
    );
    let invocation = Invocation::new(
        SourceIdentity {
            identity: format!("{stem}/invocation"),
            revision: "1".into(),
        },
        InvocationDraft {
            models: vec![ModelBinding {
                model: model.environment().owner().clone(),
                digest: model.digest(),
            }],
            context: QualifiedName {
                model: model.environment().owner().clone(),
                name: symbol("ConfigVersion"),
            },
            operation: symbol("attemptUpdate"),
            anchor: ir::AnchorName::new("attemptUpdate").unwrap(),
            self_object: object(model, "child"),
            pre: pre.reference(),
            post: post.reference(),
            parameters: Vec::new(),
            result: Some(ValueId::new(0)),
            created: Vec::new(),
            deleted: Vec::new(),
            arena: vec![ValueNode::Boolean { value: true }],
        },
        ArtifactLimits::default(),
    )
    .unwrap();
    let binding = native
        .checked()
        .clauses()
        .iter()
        .find(|clause| clause.binding().name == "VersionUnchanged")
        .unwrap()
        .binding();
    let selection = ExecutionSelection {
        requirement: binding.requirement.clone(),
        clause: binding.clause.clone(),
        observation: ObservationSelection::Invocation {
            invocation: invocation.reference(),
        },
    };
    (
        RuntimeInput {
            snapshots: vec![pre, post],
            invocations: vec![invocation],
        },
        selection,
    )
}

fn native_verdict(
    native: &NativePackage<'_>,
    model: &NativeModel,
    pre_version: i64,
    post_version: i64,
) -> Option<bool> {
    let (input, selection) = runtime_pair(native, model, pre_version, post_version);
    runtime::execute(native, input, selection, ExecutionLimits::default(), || {
        false
    })
    .truth()
}

fn attestation() -> codegen::AttestationContext<'static> {
    codegen::AttestationContext {
        record_digest: "0000000000000000000000000000000000000000000000000000000000000000",
        candidate_revision: codegen::IR_CANDIDATE_REVISION,
    }
}

fn public_function(source: &str) -> &syn::ItemFn {
    let syntax = syn::parse_file(source).unwrap();
    let function = syntax.items.into_iter().find_map(|item| match item {
        syn::Item::Fn(function) if matches!(function.vis, syn::Visibility::Public(_)) => {
            Some(function)
        }
        _ => None,
    });
    Box::leak(Box::new(function.expect("one public generated function")))
}

struct OracleExecutable {
    _directory: tempfile::TempDir,
    path: PathBuf,
    calls: Cell<u64>,
}

impl OracleExecutable {
    fn compile(source: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("src")).unwrap();
        fs::write(directory.path().join("src/oracle.rs"), source).unwrap();
        let function = public_function(source);
        let arguments = function
            .sig
            .inputs
            .iter()
            .map(|argument| {
                let syn::FnArg::Typed(argument) = argument else {
                    panic!("generated oracle is a free function")
                };
                let syn::Pat::Ident(name) = argument.pat.as_ref() else {
                    panic!("generated oracle argument is named")
                };
                let name = name.ident.to_string();
                if name.ends_with("_pre") {
                    "pre"
                } else if name.ends_with("_post") {
                    "post"
                } else {
                    panic!("unexpected ConfigVersion oracle argument {name}")
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        fs::write(
            directory.path().join("src/main.rs"),
            format!(
                "#![deny(warnings)]\n#[allow(dead_code)] #[path = \"oracle.rs\"] mod oracle;\nfn main() {{ let mut values = std::env::args().skip(1); let pre: i64 = values.next().unwrap().parse().unwrap(); let post: i64 = values.next().unwrap().parse().unwrap(); assert!(values.next().is_none()); println!(\"{{}}\", oracle::{}({arguments})); }}\n",
                function.sig.ident
            ),
        )
        .unwrap();
        fs::write(
            directory.path().join("Cargo.toml"),
            format!(
                "[package]\nname = \"configversion-oracle\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\nquire-contract-runtime = {{ git = \"https://github.com/agent-ix/quire-contract-runtime\", rev = \"{}\" }}\n\n[workspace]\n",
                codegen::RUNTIME_REVISION
            ),
        )
        .unwrap();
        let output = Command::new(env!("CARGO"))
            .args(["build", "--offline", "--quiet", "--target-dir"])
            .arg(directory.path().join("target-codex-backends"))
            .env("RUSTFLAGS", "-Dwarnings")
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "generated oracle did not compile:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let path = directory
            .path()
            .join("target-codex-backends/debug")
            .join(format!("configversion-oracle{}", env::consts::EXE_SUFFIX));
        Self {
            _directory: directory,
            path,
            calls: Cell::new(0),
        }
    }

    fn bounded_verdict(&self, pre: i64, post: i64) -> Option<bool> {
        if !DOMAIN.contains(&pre) || !DOMAIN.contains(&post) {
            return None;
        }
        self.calls.set(self.calls.get() + 1);
        let output = Command::new(&self.path)
            .args([pre.to_string(), post.to_string()])
            .output()
            .unwrap();
        assert!(output.status.success());
        Some(String::from_utf8(output.stdout).unwrap().trim() == "true")
    }
}

fn generated_oracle(
    bound: &ir::BoundPackage,
) -> (&ir::BoundClause, codegen::GeneratedBoundOracles) {
    let generated = codegen::generate_bound_oracles(bound, attestation()).unwrap();
    let codegen::BoundOracleGeneration::Generated(generated) = generated else {
        panic!("ConfigVersion has one executable state comparison")
    };
    let clause = bound
        .clauses()
        .iter()
        .find(|clause| clause.identity().clause().as_str() == "version_unchanged")
        .unwrap();
    (clause, generated)
}

fn generated_item(source: &str, prefix: &str) -> String {
    source
        .lines()
        .find_map(|line| {
            line.starts_with(prefix).then(|| {
                line.split_whitespace()
                    .nth(2)
                    .expect("generated public item name")
                    .split(['(', '<', ' ', '{', ':'])
                    .next()
                    .unwrap()
                    .to_owned()
            })
        })
        .unwrap_or_else(|| panic!("generated source has no item beginning {prefix:?}"))
}

fn sampled_runner(source: &str) -> String {
    source
        .lines()
        .find_map(|line| {
            line.strip_prefix("pub fn bound_campaign_")
                .filter(|tail| tail.contains("_run<Strategy>"))
                .map(|tail| format!("bound_campaign_{}", tail.split('<').next().unwrap()))
        })
        .expect("generated sampled campaign runner")
}

fn census_runner(source: &str) -> String {
    source
        .lines()
        .find_map(|line| {
            line.strip_prefix("pub fn bound_campaign_")
                .filter(|tail| tail.contains("_run_census("))
                .map(|tail| format!("bound_campaign_{}", tail.split('(').next().unwrap()))
        })
        .expect("generated boundary census runner")
}

fn instrumented_strategy_source(source: &str) -> String {
    let oracle = generated_item(source, "pub fn oracle_");
    let syntax = syn::parse_file(source).unwrap();
    let function = syntax
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == oracle => Some(function),
            _ => None,
        })
        .unwrap();
    let arguments = function
        .sig
        .inputs
        .iter()
        .map(|argument| {
            let syn::FnArg::Typed(argument) = argument else {
                panic!("generated oracle is a free function")
            };
            let syn::Pat::Ident(name) = argument.pat.as_ref() else {
                panic!("generated oracle argument is named")
            };
            name.ident.to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(arguments.len(), 2);
    let mut instrumented = source.replacen(
        &format!("pub fn {oracle}("),
        &format!("fn {oracle}_uninstrumented("),
        1,
    );
    writeln!(
        instrumented,
        r#"
thread_local! {{
    static IT_010_OBSERVATIONS: core::cell::RefCell<Vec<(i64, i64)>> = const {{ core::cell::RefCell::new(Vec::new()) }};
}}

/// IT-010 instrumented call preserving the generated oracle body.
pub fn {oracle}({first}: i64, {second}: i64) -> bool {{
    IT_010_OBSERVATIONS.with(|values| values.borrow_mut().push(({first}, {second})));
    {oracle}_uninstrumented({first}, {second})
}}

/// Drain the exact values observed by this generated campaign.
pub fn take_it_010_observations() -> Vec<(i64, i64)> {{
    IT_010_OBSERVATIONS.with(|values| core::mem::take(&mut *values.borrow_mut()))
}}
"#,
        first = arguments[0],
        second = arguments[1],
    )
    .unwrap();
    instrumented
}

#[test]
#[trace("IT-010-SC-01")]
fn locked_backend_graph_has_one_reviewed_ir_and_pinned_kani() {
    let lock = include_str!("../Cargo.lock");
    assert!(lock.contains(&format!("rev={CODEGEN_REVISION}")));
    assert_eq!(lock.matches("name = \"quire-contract-ir\"").count(), 1);
    assert!(lock.contains(&format!("rev={IR_REVISION}")));
    assert_eq!(codegen::IR_CANDIDATE_REVISION, IR_REVISION);
    assert_eq!(codegen::RUNTIME_REVISION.len(), 40);
    assert_eq!(codegen::KANI_BACKEND_VERSION, "0.67.0");
    let version = Command::new("cargo")
        .args(["kani", "--version"])
        .output()
        .expect("cargo-kani 0.67.0 must be installed");
    assert!(version.status.success());
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        "cargo-kani 0.67.0"
    );
    let executable = env::var_os("PATH")
        .into_iter()
        .flat_map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .map(|directory| directory.join(format!("cargo-kani{}", env::consts::EXE_SUFFIX)))
        .find(|candidate| candidate.is_file())
        .expect("cargo-kani executable is on PATH");
    let digest = sha2::Sha256::digest(fs::read(executable).unwrap());
    assert_eq!(format!("{digest:x}"), KANI_SHA256);
}

#[test]
#[trace("IT-010-SC-02", "IT-010-SC-03")]
fn compiled_state_oracle_and_native_execute_agree_on_complete_corpus() {
    let models = [config::model().unwrap()];
    let directory = tempfile::tempdir().unwrap();
    let native = compile_config(directory.path(), &models, config::Case::Unchanged);
    let projection = lower_for(
        &native,
        ProjectionTarget::StateScalarIrV1,
        LoweringLimits::default(),
    )
    .unwrap();
    let strict = ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap();
    assert_eq!(&strict, projection.bound());
    let (bound_clause, generated) = generated_oracle(&strict);
    let generated_clause = generated
        .clauses()
        .iter()
        .find(|clause| clause.identity() == bound_clause.identity())
        .unwrap();
    assert_eq!(
        generated_clause.expression_digest(),
        bound_clause.expression_digest()
    );
    let map: Vec<codegen::SourceRegion> =
        serde_json::from_str(&generated_clause.bundle().source_map.contents).unwrap();
    assert!(map.iter().all(|region| {
        region.package_id == "example/config-version"
            && region.requirement_id == "VersionUnchanged"
            && region.requirement_revision == 1
            && region.clause_id == "version_unchanged"
    }));
    let oracle = OracleExecutable::compile(&generated_clause.bundle().rust.contents);
    for pre in CORPUS {
        for post in CORPUS {
            let calls = oracle.calls.get();
            let generated = oracle.bounded_verdict(pre, post);
            let native_result = native_verdict(&native, &models[0], pre, post);
            assert_eq!(generated, native_result, "pre={pre}, post={post}");
            if DOMAIN.contains(&pre) && DOMAIN.contains(&post) {
                assert_eq!(generated, Some(pre == post));
                assert_eq!(oracle.calls.get(), calls + 1);
            } else {
                assert_eq!(generated, None);
                assert_eq!(
                    oracle.calls.get(),
                    calls,
                    "invalid input invoked Boolean oracle"
                );
            }
        }
    }
}

#[test]
#[trace("IT-010-SC-02", "IT-010-SC-04")]
fn generated_proptest_populations_execute_in_domain_with_zero_discards() {
    let models = [config::model().unwrap()];
    let fixture = tempfile::tempdir().unwrap();
    let native = compile_config(fixture.path(), &models, config::Case::Unchanged);
    let projection = lower_for(
        &native,
        ProjectionTarget::StateScalarIrV1,
        LoweringLimits::default(),
    )
    .unwrap();
    let bound = ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap();
    let clause = bound
        .clauses()
        .iter()
        .find(|clause| clause.identity().clause().as_str() == "version_unchanged")
        .unwrap();

    let generated_crate = tempfile::tempdir().unwrap();
    fs::create_dir(generated_crate.path().join("src")).unwrap();
    let mut root =
        String::from("#![deny(warnings)]\n//! Executed IT-010 strategy populations.\n\n");
    let mut checks = String::from("#[cfg(test)]\nmod checks {\n");
    for (module, population) in [
        ("satisfying", codegen::BoundStrategyPopulation::Satisfying),
        ("violating", codegen::BoundStrategyPopulation::Violating),
        ("broad", codegen::BoundStrategyPopulation::Broad),
        ("boundary", codegen::BoundStrategyPopulation::Boundary),
    ] {
        let generated = codegen::generate_bound_strategy(&codegen::BoundStrategyRequest {
            package: &bound,
            clause: clause.identity(),
            population,
            minimum_accepted_cases: 1,
            minimum_rejected_cases: 0,
            maximum_discarded_cases: 0,
            attestation: attestation(),
        })
        .unwrap();
        fs::write(
            generated_crate.path().join(format!("src/{module}.rs")),
            instrumented_strategy_source(&generated.rust.contents),
        )
        .unwrap();
        writeln!(
            root,
            "#[allow(dead_code)] #[path = \"{module}.rs\"] pub mod {module};"
        )
        .unwrap();
        if population == codegen::BoundStrategyPopulation::Boundary {
            let runner = census_runner(&generated.rust.contents);
            writeln!(
                checks,
                r#"
    #[test]
    fn boundary_census_executes_exact_in_domain_cases() {{
        let mut report = quire_contract_runtime::CampaignReport::new(
            quire_contract_runtime::ContractIdentity::new(
                quire_contract_runtime::RequirementId::new("VersionUnchanged"),
                quire_contract_runtime::RevisionId::new("1"),
            ),
        );
        let summary = super::boundary::{runner}(&mut report).unwrap();
        let observed = super::boundary::take_it_010_observations();
        assert_eq!(observed.len(), usize::try_from(summary.attempted).unwrap());
        assert!(observed.iter().all(|(left, right)| (0..=1000).contains(left) && (0..=1000).contains(right)));
        assert_eq!(summary.accepted, summary.attempted);
        assert!(summary.failed > 0 && summary.failed < summary.attempted);
        assert_eq!(summary.discarded, 0);
        assert_eq!(summary.discard_rate(), Some((0, summary.attempted)));
    }}
"#
            )
            .unwrap();
        } else {
            let strategy = generated_item(&generated.rust.contents, "pub fn bound_strategy_");
            let runner = sampled_runner(&generated.rust.contents);
            let failure_assertion = match population {
                codegen::BoundStrategyPopulation::Satisfying => "assert_eq!(summary.failed, 0);",
                codegen::BoundStrategyPopulation::Violating => {
                    "assert_eq!(summary.failed, summary.attempted);"
                }
                codegen::BoundStrategyPopulation::Broad => {
                    "assert!(summary.failed > 0 && summary.failed < summary.attempted);"
                }
                codegen::BoundStrategyPopulation::Boundary => unreachable!(),
            };
            writeln!(
                checks,
                r#"
    #[test]
    fn {module}_sampled_campaign_is_in_domain_and_reports_zero_discards() {{
        let config = proptest::test_runner::Config {{
            cases: 256,
            max_global_rejects: 0,
            failure_persistence: None,
            ..proptest::test_runner::Config::default()
        }};
        let mut runner = proptest::test_runner::TestRunner::new_with_rng(
            config,
            proptest::test_runner::TestRng::deterministic_rng(
                proptest::test_runner::RngAlgorithm::ChaCha,
            ),
        );
        let strategy = super::{module}::{strategy}();
        let mut report = quire_contract_runtime::CampaignReport::new(
            quire_contract_runtime::ContractIdentity::new(
                quire_contract_runtime::RequirementId::new("VersionUnchanged"),
                quire_contract_runtime::RevisionId::new("1"),
            ),
        );
        let summary = super::{module}::{runner}(&mut runner, &strategy, &mut report).unwrap();
        let observed = super::{module}::take_it_010_observations();
        assert_eq!(observed.len(), 256);
        assert!(observed.iter().all(|(left, right)| (0..=1000).contains(left) && (0..=1000).contains(right)));
        assert_eq!(summary.attempted, 256);
        assert_eq!(summary.accepted, 256);
        assert_eq!(summary.rejected, 0);
        {failure_assertion}
        assert_eq!(summary.discarded, 0);
        assert_eq!(summary.discard_rate(), Some((0, 256)));
    }}
"#
            )
            .unwrap();
        }
    }
    checks.push_str("}\n");
    root.push_str(&checks);
    fs::write(generated_crate.path().join("src/lib.rs"), root).unwrap();
    fs::write(
        generated_crate.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"configversion-strategies\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\nproptest = {{ version = \"=1.5.0\", default-features = false, features = [\"std\"] }}\nquire-contract-runtime = {{ git = \"https://github.com/agent-ix/quire-contract-runtime\", rev = \"{}\" }}\n\n[workspace]\n",
            codegen::RUNTIME_REVISION
        ),
    )
    .unwrap();
    let output = Command::new(env!("CARGO"))
        .args(["test", "--offline", "--quiet", "--target-dir"])
        .arg(generated_crate.path().join("target-codex-backends"))
        .env("RUSTFLAGS", "-Dwarnings")
        .current_dir(generated_crate.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "generated strategy campaigns failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn config_kani_bundle(clause: &ir::BoundClause) -> codegen::KaniArtifactBundle {
    let true_source = ir::Expression::new(
        ir::ExpressionKind::BooleanLiteral { value: true },
        clause.source().clone(),
    );
    let precondition = clause
        .environment()
        .check_expression(&true_source, &ir::ValueType::Boolean, clause.anchor(), true)
        .expect("canonical absent precondition is a zero-dependency Boolean true");
    assert!(precondition.dependencies().is_empty());
    let precondition_clause = ir::ClauseId::new("version_unchanged_absent_precondition").unwrap();
    codegen::generate_kani_bundle(&codegen::KaniRequest {
        requirement: clause.identity().requirement(),
        precondition_clause: &precondition_clause,
        postcondition_clause: clause.identity().clause(),
        precondition: &precondition,
        postcondition: clause.expression(),
        proof_id: "it-010-config-version",
        subject_path: "crate::subject",
        backend_version: codegen::KANI_BACKEND_VERSION,
        backend_executable_sha256: KANI_SHA256,
        unwind: 2,
        solver: codegen::KaniSolver::Cadical,
        dependencies: &[],
        attestation: attestation(),
    })
    .unwrap()
}

fn execute_kani(bundle: &codegen::KaniArtifactBundle, subject: &str) -> std::process::Output {
    let graph: codegen::ProofDependencyGraph =
        serde_json::from_str(&bundle.proof_graph.contents).unwrap();
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("src")).unwrap();
    fs::write(
        directory.path().join("src/lib.rs"),
        format!("{}\n{subject}\n", bundle.rust.contents),
    )
    .unwrap();
    fs::write(
        directory.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"configversion-kani\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\nquire-contract-runtime = {{ git = \"https://github.com/agent-ix/quire-contract-runtime\", rev = \"{}\" }}\n\n[workspace]\n",
            codegen::RUNTIME_REVISION
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

fn playback_i64(output: &str) -> Option<i64> {
    let tail = output
        .split_once("let concrete_vals: Vec<Vec<u8>> = vec![")?
        .1;
    let row = tail.split_once("vec![")?.1.split_once(']')?.0;
    let bytes = row
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let bytes: [u8; 8] = bytes.try_into().ok()?;
    Some(i64::from_le_bytes(bytes))
}

#[test]
#[trace("IT-010-SC-01", "IT-010-SC-02", "IT-010-SC-05")]
fn pinned_kani_proves_identity_and_counterexample_replays_natively() {
    let models = [config::model().unwrap()];
    let fixture = tempfile::tempdir().unwrap();
    let native = compile_config(fixture.path(), &models, config::Case::Unchanged);
    let projection = lower_for(
        &native,
        ProjectionTarget::StateScalarIrV1,
        LoweringLimits::default(),
    )
    .unwrap();
    let bound = ir::BoundPackage::from_json_bytes(projection.bytes()).unwrap();
    let clause = bound
        .clauses()
        .iter()
        .find(|clause| clause.identity().clause().as_str() == "version_unchanged")
        .unwrap();
    let bundle = config_kani_bundle(clause);
    let graph: codegen::ProofDependencyGraph =
        serde_json::from_str(&bundle.proof_graph.contents).unwrap();
    assert_eq!(graph.proof_id, "it-010-config-version");
    assert_eq!(graph.requirement_id, "VersionUnchanged");
    assert_eq!(graph.requirement_revision, 1);
    assert_eq!(graph.adapter_profile, codegen::KANI_ADAPTER_PROFILE);
    assert_eq!(graph.backend_version, "0.67.0");
    assert_eq!(graph.backend_executable_sha256, KANI_SHA256);
    assert!(graph.dependencies.is_empty());
    assert_eq!(graph.subject_arguments.len(), 1);
    assert_eq!(graph.subject_results.len(), 1);
    for binding in graph.subject_arguments.iter().chain(&graph.subject_results) {
        let bounds = binding.integer_bounds.as_ref().unwrap();
        assert_eq!((bounds.minimum, bounds.maximum), (0, 1000));
    }
    let harness = graph.options[5].clone();
    assert_eq!(
        graph.options,
        [
            "-Z".to_owned(),
            "function-contracts".to_owned(),
            "-Z".to_owned(),
            "concrete-playback".to_owned(),
            "--harness".to_owned(),
            harness,
            "--exact".to_owned(),
            "--unwind".to_owned(),
            "2".to_owned(),
            "--solver".to_owned(),
            "cadical".to_owned(),
            "--output-format".to_owned(),
            "regular".to_owned(),
            "--concrete-playback".to_owned(),
            "print".to_owned(),
        ]
    );

    let healthy = execute_kani(
        &bundle,
        "/// Identity ConfigVersion subject.\npub fn subject(version_pre: i64) -> i64 { version_pre }",
    );
    assert!(
        healthy.status.success(),
        "identity proof failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&healthy.stdout),
        String::from_utf8_lossy(&healthy.stderr)
    );

    let changed = execute_kani(
        &bundle,
        "/// Always-changed, still in-domain ConfigVersion subject.\npub fn subject(version_pre: i64) -> i64 { if version_pre == 1000 { 999 } else { version_pre + 1 } }",
    );
    assert!(
        !changed.status.success(),
        "violating subject unexpectedly proved"
    );
    let output = format!(
        "{}\n{}",
        String::from_utf8_lossy(&changed.stdout),
        String::from_utf8_lossy(&changed.stderr)
    );
    let counterexample = playback_i64(&output)
        .unwrap_or_else(|| panic!("Kani emitted no decodable concrete playback:\n{output}"));
    assert!(DOMAIN.contains(&counterexample));
    let post = if counterexample == 1000 {
        999
    } else {
        counterexample + 1
    };
    assert!(DOMAIN.contains(&post));
    assert_eq!(
        native_verdict(&native, &models[0], counterexample, post),
        Some(false)
    );
    assert_eq!(playback_i64("let concrete_vals = vec![];"), None);
}

#[test]
#[trace("IT-010-SC-06")]
fn object_and_graph_clauses_refuse_at_exact_authored_loci_without_artifacts() {
    let models = [config::model().unwrap()];
    let root = tempfile::tempdir().unwrap();
    for (case, clause, spelling) in [
        (config::Case::Healthy, "parent_order", "present"),
        (config::Case::Cycle, "no_cycle", "reaches"),
    ] {
        let native = compile_config(&root.path().join(case.id()), &models, case);
        let error = lower_for(
            &native,
            ProjectionTarget::StateScalarIrV1,
            LoweringLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, LoweringCode::Unsupported);
        assert_eq!(error.clause.as_ref().unwrap().clause().as_str(), clause);
        let span = error.source.expect("unsupported native expression locus");
        let source = &native.checked().bindings().source;
        assert!(source.source().slice(span).unwrap().contains(spelling));
        assert!(
            error.upstream.is_empty(),
            "refusal occurs before IR binding"
        );
    }
}

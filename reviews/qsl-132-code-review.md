---
id: SR-4929
title: "QSL-132 immutable static code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@69cc01a64ff54fec97d3b73f459206f309eee46c; Makefile; xtask/src/ci_clean_build_tests.rs; xtask/src/ci_feature_lane_tests.rs; xtask/src/ci_feature_lane_tests/cargo_double.rs; xtask/src/lib.rs; QSL-132; NFR-002; NFR-005; FR-042-AC-15; FR-050-AC-8"
review_set: subset
---

## Summary

Ticket: QSL-132. Code-review includes the complete installed rust-review lane. Three incremental findings and two inherited findings remain open.

Exact immutable head 69cc01a64ff54fec97d3b73f459206f309eee46c; fetched and live origin/main b24dbda01d56843cd14d8cab9233ec3cff67535f; parent26d0ea94ecf1ba38ad33e0a85e5ababda9db7075 / PR679. Local source is clean and remote PR head matches. Full three-dot scope: five files, 416 insertions/12 deletions. Incremental parent diff: four files, 349 insertions/10 deletions. No applicable AssuranceProfile found. AGENTS.md, CLAUDE.md, README.md, CONTRIBUTING.md, LICENSE-DECISION.md, NFR-002/NFR-005, ticket and existing coder/reviewer evidence were read. New Rust fixtures retain AGPL-3.0-or-later; no dependency, workflow, product-source or licensing change.

Static-only dispatch: no formatter, build, test, Make dry-run, helper, native validator, aggregate, source edit, push, merge or descendant was run. Artifacts are NOT Quire-validated. Quoin runtime/module metadata was not invoked; marker values explicitly say unverified. Repository computed matrix, stale tags and evidence-store status remain pending. Spec-review and its sub-analyses are inapplicable: no spec/plan diff.

## Verdict

CONDITIONAL — findings require dispositions; static diff examination is complete, execution/schema checks are deferred. 

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Incremental: shell interpolation of caller roots changes literal metacharacter paths in both feature lanes. | Makefile:166 |
| FND-002 | low | Incremental: the static OnceLock retains its TempDir and compiled Cargo double after each test process exits. | xtask/src/ci_feature_lane_tests.rs:64 |
| FND-003 | low | Incremental: four new routing tests lack the canonical trace attributes required by NFR-005. | xtask/src/ci_feature_lane_tests.rs:159 |
| FND-004 | medium | Inherited QSL-209 SR-4922 FND-001: clean-build caller paths are still interpolated unsafely into shell double quotes. | Makefile:192 |
| FND-005 | low | Inherited parent trace low from QSL-190 SR-2449 FND-003: both clean-build tests retain legacy doc-comment tags. | xtask/src/ci_clean_build_tests.rs:40 |

## Finding details

FND-001 (incremental, high confidence): A literal target root /tmp/owned`printf x` becomes /tmp/ownedx/ci-default-features after Make substitutes it into shell double quotes; the all-feature lane behaves identically. A literal double quote can break the assignment. Space-only names work, but these valid path bytes do not remain data. Command-line lane overrides share the same defect. Derive through safe shell-variable expansion or full shell quoting and preserve the literal bytes. Static trace only; no reproducer executed.

FND-002 (incremental, high confidence): TempDir cleanup depends on Drop; static variables are not dropped at process exit. Every fresh test process compiles into another random directory under the owned target and leaves the executable behind. Both aggregate feature lanes invoke new processes. This is avoidable test-artifact accumulation, not a request to delete retained warm caches. Use an explicitly owned lifetime or deterministic reusable test artifact; preserve process safety.

FND-003 (incremental, high confidence): The complete-command, explicit-override, repeated-switch and aggregate tests have #[test] only. Only the fifth test imports and uses the existing macro. The missing tags leave these regressions outside declared trace bindings and violate the repository's documented convention. Select real owning TC/AC identities; do not tag unrelated writer semantics solely to silence tooling.

FND-004 (inherited, high confidence): Both inherited --target-dir arguments preserve the same backtick/quote path-fidelity defect at this exact head. The parse invocation inherits the original root and can therefore route differently. No parent fix/disposition is established by this review.

FND-005 (inherited, high confidence): The inherited file has /// Trace: FR-042-AC-15, FR-050-AC-8 above both tests, contrary to NFR-005's canonical imported trace-attribute convention. This is the known inherited low, not a QSL-190 code change present on PR683.

## Coverage

Full examined units and bindings (including clean units, excerpts are verbatim fragments):

```yaml
{
  "scope": [
    {
      "id": "QSL-132 done when",
      "path": "Linear QSL-132",
      "role": "examined",
      "excerpt": "Running `make ci` twice in succession from a warm target dir produces the same result both times, and no lane can execute an artifact built under different features."
    },
    {
      "id": "Makefile::default-root",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "CI_DEFAULT_TARGET_DIR = $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)/ci-default-features"
    },
    {
      "id": "Makefile::all-root",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "CI_ALL_TARGET_DIR = $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)/ci-all-features"
    },
    {
      "id": "Makefile::default-recipe",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "CARGO_TARGET_DIR=\"$(CI_DEFAULT_TARGET_DIR)\" cargo test --locked --workspace"
    },
    {
      "id": "Makefile::all-recipe",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "CARGO_TARGET_DIR=\"$(CI_ALL_TARGET_DIR)\" cargo test --locked --workspace --all-features"
    },
    {
      "id": "Makefile::clean-recipe",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "cargo build --locked --workspace --no-default-features --target-dir \"$(CI_CLEAN_TARGET_DIR)\""
    },
    {
      "id": "Makefile::ci",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "ci: check-no-committed-binaries check-index-completeness ci-default-features ci-all-features ci-clean-build seam-probe string-edge route-lint checked-input cargo-deny-bans ci-docs arch-lint-canonical-encoder arch-lint-api-surface-qsl arch-lint-qualified-core"
    },
    {
      "id": "xtask::cargo_double",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "static DIRECTORY: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();"
    },
    {
      "id": "xtask::Recipe::run",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn run(&self, targets: &[&str], root: Option<&str>, overrides: &[&str]) -> String {"
    },
    {
      "id": "xtask::every_feature_recipe_uses_its_child_and_preserves_complete_argv",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn every_feature_recipe_uses_its_child_and_preserves_complete_argv() {"
    },
    {
      "id": "xtask::explicit_roots_and_lane_overrides_are_one_quoted_argument",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn explicit_roots_and_lane_overrides_are_one_quoted_argument() {"
    },
    {
      "id": "xtask::repeated_switches_in_both_directions_keep_each_lane_directory",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn repeated_switches_in_both_directions_keep_each_lane_directory() {"
    },
    {
      "id": "xtask::aggregate_reaches_default_then_all_features_without_changing_recipe_order",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn aggregate_reaches_default_then_all_features_without_changing_recipe_order() {"
    },
    {
      "id": "xtask::clean_and_core_tooling_retain_their_own_target_and_features",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn clean_and_core_tooling_retain_their_own_target_and_features() {"
    },
    {
      "id": "cargo_double::main",
      "path": "xtask/src/ci_feature_lane_tests/cargo_double.rs",
      "role": "examined",
      "excerpt": "writeln!(log, \"{target}\\t{}\", args.join(\"\\t\")).expect(\"record invocation\");"
    },
    {
      "id": "xtask::assert_clean_build_recipe",
      "path": "xtask/src/ci_clean_build_tests.rs",
      "role": "examined",
      "excerpt": ".args([\"--no-print-directory\", \"-rR\", \"-n\", \"ci-clean-build\"])"
    },
    {
      "id": "xtask::clean_build_defaults_to_target_clean",
      "path": "xtask/src/ci_clean_build_tests.rs",
      "role": "examined",
      "excerpt": "fn clean_build_defaults_to_target_clean() {"
    },
    {
      "id": "xtask::clean_build_uses_the_callers_target_root",
      "path": "xtask/src/ci_clean_build_tests.rs",
      "role": "examined",
      "excerpt": "fn clean_build_uses_the_callers_target_root() {"
    },
    {
      "id": "xtask::registration",
      "path": "xtask/src/lib.rs",
      "role": "examined",
      "excerpt": "#[cfg(test)]\nmod ci_feature_lane_tests;"
    },
    {
      "id": "NFR-002",
      "path": "spec/non-functional/NFR-002-reproduce-native-builds.md",
      "role": "examined",
      "excerpt": "When the recorded native build command is run, the repository shall build without a Node or JVM runtime dependency."
    },
    {
      "id": "NFR-005::Verification",
      "path": "spec/non-functional/NFR-005-rust-verification-paths.md",
      "role": "examined",
      "excerpt": "Trace actual test functions to TC and AC identities\nwith imported `ix_trace_rs::trace` and canonical `#[trace(\"TC-...\", \"FR-...-AC-...\")]`\nattributes under the installed Quire/module grammar. Legacy doc-comment tags\nare not the convention for new tests."
    },
    {
      "id": "FR-042-AC-15",
      "path": "spec/functional/FR-042-publish-compiled-protocol-artifacts.md",
      "role": "examined",
      "excerpt": "`protocol_artifact::handoff::write_v1(directory)`, in the normal (non-dev) dependency surface behind the `handoff-writer` feature and adding no dependency, lets a downstream crate compile the authored recipe fresh and write a complete handoff"
    },
    {
      "id": "FR-050-AC-8",
      "path": "spec/functional/FR-050-publish-authenticated-temporal-artifacts.md",
      "role": "examined",
      "excerpt": "`protocol_artifact::handoff::write_v2(directory)` is FR-042-AC-15's `/2` counterpart: behind the same `handoff-writer` feature, it writes the complete authenticated handoff"
    },
    {
      "id": "extracted_command::extraction_request_obeys_the_build_feature",
      "path": "tests/it/extracted_command.rs",
      "role": "examined",
      "excerpt": "fn extraction_request_obeys_the_build_feature() {"
    }
  ],
  "bindings": [
    {
      "test_id": "xtask::clean_build_defaults_to_target_clean",
      "ac_id": "FR-042-AC-15",
      "trace": "correct",
      "qualification": "supplementary recipe-preservation only; no writer semantic qualification"
    },
    {
      "test_id": "xtask::clean_build_defaults_to_target_clean",
      "ac_id": "FR-050-AC-8",
      "trace": "correct",
      "qualification": "supplementary recipe-preservation only; no writer semantic qualification"
    },
    {
      "test_id": "xtask::clean_build_uses_the_callers_target_root",
      "ac_id": "FR-042-AC-15",
      "trace": "correct",
      "qualification": "supplementary recipe-preservation only; no writer semantic qualification"
    },
    {
      "test_id": "xtask::clean_build_uses_the_callers_target_root",
      "ac_id": "FR-050-AC-8",
      "trace": "correct",
      "qualification": "supplementary recipe-preservation only; no writer semantic qualification"
    },
    {
      "test_id": "xtask::clean_and_core_tooling_retain_their_own_target_and_features",
      "ac_id": "FR-042-AC-15",
      "trace": "correct",
      "qualification": "supplementary recipe-preservation only; no writer semantic qualification"
    },
    {
      "test_id": "xtask::clean_and_core_tooling_retain_their_own_target_and_features",
      "ac_id": "FR-050-AC-8",
      "trace": "correct",
      "qualification": "supplementary recipe-preservation only; no writer semantic qualification"
    },
    {
      "test_id": "extracted_command::extraction_request_obeys_the_build_feature",
      "ac_id": "FR-031-AC-3",
      "trace": "correct",
      "qualification": "existing real CLI feature-enabled/disabled control; context inspected"
    }
  ]
}
```

## Existing execution evidence and review checks

Existing evidence, read rather than independently rerun: batches/qsl-132-{cheap,focused,shared-path-red,warm-extraction,ci}.log and qsl-132-feature-targets-evidence.md. Focused five routing plus two inherited controls passed; shared-child removal failed the intended complete isolation assertion (exit101), restored controls passed (exit0). Real CLI uses CARGO_BIN_EXE_quire-spec with an explicit cwd. Default/all/default/all/default runs passed 1/12/1/12/1 = 27 executions, zero ignored, through separate executable directories. These are focused executions, not 27 distinct tests or full aggregate acceptance.

One existing full make ci passed on this exact unchanged head: 03:54:47 to06:22:11 UTC, 2h27m24s plus80s lock wait, exit0. Root integrations default888/all909 passed with2 existing ignored in each; xtask101 passed each. All nine feature Cargo commands/argv and standalone/default feature distinctions are preserved. Clean build/writer check, parse, non-feature tools, docs, dependency bans and architecture checks retained their original recipes/order. No skips or narrowed checks added. The aggregate routing test deliberately omits unrelated targets with -o: this focused reachability oracle is not full aggregate evidence. Real Cargo remains authoritative for compilation; the process double measures only Make routing and actual argv/env. Historical wrong-feature cause is unproven, and the routing mutant does not reproduce it.

Relative roots are interpreted from Make's explicit working directory; the new tests launch a copied real Makefile under an isolated directory. The Rust compiler helper resolves its owned target against the actual workspace. No changed async, wire, integer conversion or production panic/unsafe surface exists. Test unwraps are assertion setup. Native helper's tab-delimited recording is sufficient for the fixed argv and examined path cases, and should not be mistaken for a general lossless protocol. Helper duplication searches over xtask/tools/tests found no reusable process-seam helper in this head. Parent190 fixtures are a separate branch, not source silently imported into this review.

Two-successive-warm aggregate acceptance remains unmet. Fix/disposition/custody, lead-coordinated final aggregate, parent-safe landing and closeout remain outstanding. Any custody/source head change prevents claiming the existing aggregate and later aggregate are unchanged-head successive acceptance. Driver688 fixes and671/685 priorities precede L0 gates. Preserve132-target; no cleanup is requested.

## Remaining validation

See validation-request.md for a lead-scheduled isolated reviewer batch. No command has been queued. Full matrix interpretation and complete repository reverse-gap audit require follow-up before a full gap-analysis result is claimed.

## Dispositions

No disposition pass yet. All findings above remain open; the same reviewer retains future disposition work. Known parent findings remain under their original SR identities as well as the explicit inherited rows here.

## Disposition pass 1

Reviewed head: b5c5e7fe9927598387aff11b0d85ee8889ceac08. Verdict: CONDITIONAL. Fixed outcomes below mean source-level resolution, with execution qualification pending; still-open rows require further action. Original findings, scope, excerpts and historical verdict are unchanged.

Round1 static inspection at exact b5c5e7fe9927598387aff11b0d85ee8889ceac08. Clean local head and GitHub PR683 head match; live/fetched origin/main is now9da86d0f553956d11a49e23cdb93794c86389e4b; merge-base remains b24dbda01d56843cd14d8cab9233ec3cff67535f. Full main three-dot diff is seven files,1007 insertions/12 deletions, including inherited209 and verbatim original SR custody. Incremental fix commit changes Makefile and feature fixture and adds the two original SR files. Parent209 source is unchanged. Main advanced independently through PR690; no rebase or copy was made by this reviewer.

Full qsl-132-review-fix-round1-report.md and the complete actual changed source were read. All nine feature commands/arguments/order remain intact. Quoted exported shell-variable transport corrects the reviewed shell reparse defect. Per-fixture TempDir ownership replaces static storage; ordinary subprocess completion precedes drop. Metacharacter/environment/override cases and safe copied-Makefile controls provide meaningful source-level oracles; no execution outcome is inferred. The new mutant's NFR-only marker is the same unresolved trace issue as FND-003. No distinct new defect was found in this static pass.

The coder's formatting/old-binary selection preflight4151640 was queued but no executed receipt is available for this disposition. Old binary listing cannot establish six current tests or current teardown/mutation behavior. No new-head formatting/lint/compile/focused-test/helper/matrix/schema or aggregate PASS is claimed. Original69cc aggregate and27 extraction executions are historical prior-head receipts only. Two unchanged-source warm aggregates are still required, not waived. No reviewer command was executed or queued, no lock taken, no public GitHub findings, source edit, push, merge, parent-source copy or descendant work occurred. Same reviewer retains future dispositions.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | b5c5e7fe9927598387aff11b0d85ee8889ceac08 |
| FND-002 | fixed | b5c5e7fe9927598387aff11b0d85ee8889ceac08 |
| FND-003 | still-open | The four original tests now use #[trace("NFR-005")] (lines161,193,280,304), and the new mutant test repeats it at244. NFR-005 Verification lines33-37 explicitly requires actual TC and AC identities; its metric is inspection of Rust verification language and it declares no CI routing acceptance criterion. A requirement-level language-policy marker supplies neither a TC nor a behavioral AC. The source-backed minimum is real existing owning TC/AC bindings if applicable; otherwise a narrowly scoped normative CI-routing criterion and linked TC describing lane isolation, literal caller-path/override fidelity and preserved command/feature order, with canonical imported attributes mapped to assertions. Retain runtime extraction qualification separately. Do not invent IDs or tag unrelated writer/extraction ACs just to obtain a binding. This is the original trace finding, including the same defect on the new test; no duplicate finding allocated. |
| FND-004 | still-open | At the reviewed head, Makefile192-193 still interpolates $(CI_CLEAN_TARGET_DIR) directly into shell double quotes. The parent codex3 fix is pending separate qualification/integration per dispatch, and is not present in this candidate. No parent source was copied or parent disposition inferred. |
| FND-005 | still-open | Both inherited clean tests still carry legacy /// Trace: FR-042-AC-15, FR-050-AC-8 at lines41,49, with no imported canonical trace attributes. NFR-00533-37 and README canonical convention still apply. Pending parent fixes do not resolve this exact head. |

```yaml
{
  "round": 1,
  "reviewed": "b5c5e7fe9927598387aff11b0d85ee8889ceac08",
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "fix_sha": "b5c5e7fe9927598387aff11b0d85ee8889ceac08",
      "after_excerpt": "export CI_DEFAULT_TARGET_DIR = $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)/ci-default-features\nexport CI_ALL_TARGET_DIR = $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)/ci-all-features\n\tCARGO_TARGET_DIR=\"$$CI_DEFAULT_TARGET_DIR\" cargo test --locked --workspace\n\tCARGO_TARGET_DIR=\"$$CI_ALL_TARGET_DIR\" cargo test --locked --workspace --all-features",
      "path": "Makefile",
      "lines": "164-178",
      "evidence": "Static source disposition only. All nine recipes now expand quoted shell variables from exported path data; the shell does not reparse expansion contents. Complete argv/feature flags/order are unchanged. No new-head execution PASS."
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "fix_sha": "b5c5e7fe9927598387aff11b0d85ee8889ceac08",
      "after_excerpt": "fn cargo_double() -> tempfile::TempDir {\nstruct Recipe {\n    directory: tempfile::TempDir,\n    // All child processes have been joined before this owner is dropped.\n    cargo_directory: tempfile::TempDir,\n}\n    let helper_directory = recipe.cargo_directory.path().to_path_buf();\n    assert!(helper_directory.join(\"cargo\").is_file());\n    drop(recipe);\n    assert!(!helper_directory.exists(), \"fixture teardown removes its compiled helper\");",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "lines": "64-110,186-189",
      "evidence": "Verbatim separate source fragments. Static OnceLock removed; each Recipe owns the helper TempDir, synchronous output() joins processes, and fixture drop releases it. Teardown assertion exists but has not executed at this head."
    },
    {
      "fnd": "FND-003",
      "outcome": "still-open",
      "reason": "The four original tests now use #[trace(\"NFR-005\")] (lines161,193,280,304), and the new mutant test repeats it at244. NFR-005 Verification lines33-37 explicitly requires actual TC and AC identities; its metric is inspection of Rust verification language and it declares no CI routing acceptance criterion. A requirement-level language-policy marker supplies neither a TC nor a behavioral AC. The source-backed minimum is real existing owning TC/AC bindings if applicable; otherwise a narrowly scoped normative CI-routing criterion and linked TC describing lane isolation, literal caller-path/override fidelity and preserved command/feature order, with canonical imported attributes mapped to assertions. Retain runtime extraction qualification separately. Do not invent IDs or tag unrelated writer/extraction ACs just to obtain a binding. This is the original trace finding, including the same defect on the new test; no duplicate finding allocated.",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "lines": "160-162,192-194,243-245,279-281,303-305",
      "current_excerpt": "#[test]\n#[trace(\"NFR-005\")]\nfn every_feature_recipe_uses_its_child_and_preserves_complete_argv() {"
    },
    {
      "fnd": "FND-004",
      "outcome": "still-open",
      "reason": "At the reviewed head, Makefile192-193 still interpolates $(CI_CLEAN_TARGET_DIR) directly into shell double quotes. The parent codex3 fix is pending separate qualification/integration per dispatch, and is not present in this candidate. No parent source was copied or parent disposition inferred.",
      "path": "Makefile",
      "lines": "189-194",
      "current_excerpt": "cargo build --locked --workspace --no-default-features --target-dir \"$(CI_CLEAN_TARGET_DIR)\""
    },
    {
      "fnd": "FND-005",
      "outcome": "still-open",
      "reason": "Both inherited clean tests still carry legacy /// Trace: FR-042-AC-15, FR-050-AC-8 at lines41,49, with no imported canonical trace attributes. NFR-00533-37 and README canonical convention still apply. Pending parent fixes do not resolve this exact head.",
      "path": "xtask/src/ci_clean_build_tests.rs",
      "lines": "40-51",
      "current_excerpt": "/// Provenance: QSL-209; recipe wiring only, real compilation is checked separately.\n/// Trace: FR-042-AC-15, FR-050-AC-8\n#[test]\nfn clean_build_defaults_to_target_clean() {"
    }
  ],
  "original_findings": [
    {
      "fnd": "FND-001",
      "severity": "medium",
      "confidence": "high",
      "check": "code-bug",
      "artifact_id": "Makefile::default-recipe",
      "related_ids": [
        "Makefile::all-recipe"
      ],
      "path": "Makefile",
      "lines": "166-177",
      "excerpt": "CARGO_TARGET_DIR=\"$(CI_DEFAULT_TARGET_DIR)\" cargo test --locked --workspace",
      "finding": "Incremental: shell interpolation of caller roots changes literal metacharacter paths in both feature lanes.",
      "origin": "incremental",
      "details": "A literal target root /tmp/owned`printf x` becomes /tmp/ownedx/ci-default-features after Make substitutes it into shell double quotes; the all-feature lane behaves identically. A literal double quote can break the assignment. Space-only names work, but these valid path bytes do not remain data. Command-line lane overrides share the same defect. Derive through safe shell-variable expansion or full shell quoting and preserve the literal bytes. Static trace only; no reproducer executed.",
      "method": "code-review"
    },
    {
      "fnd": "FND-002",
      "severity": "low",
      "confidence": "high",
      "check": "other",
      "artifact_id": "xtask::cargo_double",
      "related_ids": [],
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "lines": "64-94",
      "excerpt": "static DIRECTORY: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();",
      "finding": "Incremental: the static OnceLock retains its TempDir and compiled Cargo double after each test process exits.",
      "origin": "incremental",
      "details": "TempDir cleanup depends on Drop; static variables are not dropped at process exit. Every fresh test process compiles into another random directory under the owned target and leaves the executable behind. Both aggregate feature lanes invoke new processes. This is avoidable test-artifact accumulation, not a request to delete retained warm caches. Use an explicitly owned lifetime or deterministic reusable test artifact; preserve process safety.",
      "method": "code-review"
    },
    {
      "fnd": "FND-003",
      "severity": "low",
      "confidence": "high",
      "check": "other",
      "artifact_id": "xtask::every_feature_recipe_uses_its_child_and_preserves_complete_argv",
      "related_ids": [
        "NFR-005::Verification"
      ],
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "lines": "159-255",
      "excerpt": "fn every_feature_recipe_uses_its_child_and_preserves_complete_argv() {",
      "finding": "Incremental: four new routing tests lack the canonical trace attributes required by NFR-005.",
      "origin": "incremental",
      "details": "The complete-command, explicit-override, repeated-switch and aggregate tests have #[test] only. Only the fifth test imports and uses the existing macro. The missing tags leave these regressions outside declared trace bindings and violate the repository's documented convention. Select real owning TC/AC identities; do not tag unrelated writer semantics solely to silence tooling.",
      "method": "code-review"
    },
    {
      "fnd": "FND-004",
      "severity": "medium",
      "confidence": "high",
      "check": "code-bug",
      "artifact_id": "Makefile::clean-recipe",
      "related_ids": [
        "Makefile::default-recipe"
      ],
      "path": "Makefile",
      "lines": "192-193",
      "excerpt": "cargo build --locked --workspace --no-default-features --target-dir \"$(CI_CLEAN_TARGET_DIR)\"",
      "finding": "Inherited QSL-209 SR-4922 FND-001: clean-build caller paths are still interpolated unsafely into shell double quotes.",
      "origin": "inherited",
      "details": "Both inherited --target-dir arguments preserve the same backtick/quote path-fidelity defect at this exact head. The parse invocation inherits the original root and can therefore route differently. No parent fix/disposition is established by this review.",
      "method": "code-review"
    },
    {
      "fnd": "FND-005",
      "severity": "low",
      "confidence": "high",
      "check": "other",
      "artifact_id": "xtask::clean_build_defaults_to_target_clean",
      "related_ids": [
        "NFR-005::Verification"
      ],
      "path": "xtask/src/ci_clean_build_tests.rs",
      "lines": "40-50",
      "excerpt": "fn clean_build_defaults_to_target_clean() {",
      "finding": "Inherited parent trace low from QSL-190 SR-2449 FND-003: both clean-build tests retain legacy doc-comment tags.",
      "origin": "inherited",
      "details": "The inherited file has /// Trace: FR-042-AC-15, FR-050-AC-8 above both tests, contrary to NFR-005's canonical imported trace-attribute convention. This is the known inherited low, not a QSL-190 code change present on PR683.",
      "method": "code-review"
    }
  ],
  "validation": {
    "mode": "static",
    "new_head_execution": "not run/no PASS",
    "artifact": "not Quire-validated",
    "matrix": "not run",
    "aggregate": "not queued; TWO warm acceptance open"
  },
  "new_findings": [],
  "run": "qsl132-pr683-round1-fa8f725615a9a8",
  "original_scope": [
    {
      "id": "QSL-132 done when",
      "path": "Linear QSL-132",
      "role": "examined",
      "excerpt": "Running `make ci` twice in succession from a warm target dir produces the same result both times, and no lane can execute an artifact built under different features."
    },
    {
      "id": "Makefile::default-root",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "CI_DEFAULT_TARGET_DIR = $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)/ci-default-features"
    },
    {
      "id": "Makefile::all-root",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "CI_ALL_TARGET_DIR = $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)/ci-all-features"
    },
    {
      "id": "Makefile::default-recipe",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "CARGO_TARGET_DIR=\"$(CI_DEFAULT_TARGET_DIR)\" cargo test --locked --workspace"
    },
    {
      "id": "Makefile::all-recipe",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "CARGO_TARGET_DIR=\"$(CI_ALL_TARGET_DIR)\" cargo test --locked --workspace --all-features"
    },
    {
      "id": "Makefile::clean-recipe",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "cargo build --locked --workspace --no-default-features --target-dir \"$(CI_CLEAN_TARGET_DIR)\""
    },
    {
      "id": "Makefile::ci",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "ci: check-no-committed-binaries check-index-completeness ci-default-features ci-all-features ci-clean-build seam-probe string-edge route-lint checked-input cargo-deny-bans ci-docs arch-lint-canonical-encoder arch-lint-api-surface-qsl arch-lint-qualified-core"
    },
    {
      "id": "xtask::cargo_double",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "static DIRECTORY: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();"
    },
    {
      "id": "xtask::Recipe::run",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn run(&self, targets: &[&str], root: Option<&str>, overrides: &[&str]) -> String {"
    },
    {
      "id": "xtask::every_feature_recipe_uses_its_child_and_preserves_complete_argv",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn every_feature_recipe_uses_its_child_and_preserves_complete_argv() {"
    },
    {
      "id": "xtask::explicit_roots_and_lane_overrides_are_one_quoted_argument",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn explicit_roots_and_lane_overrides_are_one_quoted_argument() {"
    },
    {
      "id": "xtask::repeated_switches_in_both_directions_keep_each_lane_directory",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn repeated_switches_in_both_directions_keep_each_lane_directory() {"
    },
    {
      "id": "xtask::aggregate_reaches_default_then_all_features_without_changing_recipe_order",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn aggregate_reaches_default_then_all_features_without_changing_recipe_order() {"
    },
    {
      "id": "xtask::clean_and_core_tooling_retain_their_own_target_and_features",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn clean_and_core_tooling_retain_their_own_target_and_features() {"
    },
    {
      "id": "cargo_double::main",
      "path": "xtask/src/ci_feature_lane_tests/cargo_double.rs",
      "role": "examined",
      "excerpt": "writeln!(log, \"{target}\\t{}\", args.join(\"\\t\")).expect(\"record invocation\");"
    },
    {
      "id": "xtask::assert_clean_build_recipe",
      "path": "xtask/src/ci_clean_build_tests.rs",
      "role": "examined",
      "excerpt": ".args([\"--no-print-directory\", \"-rR\", \"-n\", \"ci-clean-build\"])"
    },
    {
      "id": "xtask::clean_build_defaults_to_target_clean",
      "path": "xtask/src/ci_clean_build_tests.rs",
      "role": "examined",
      "excerpt": "fn clean_build_defaults_to_target_clean() {"
    },
    {
      "id": "xtask::clean_build_uses_the_callers_target_root",
      "path": "xtask/src/ci_clean_build_tests.rs",
      "role": "examined",
      "excerpt": "fn clean_build_uses_the_callers_target_root() {"
    },
    {
      "id": "xtask::registration",
      "path": "xtask/src/lib.rs",
      "role": "examined",
      "excerpt": "#[cfg(test)]\nmod ci_feature_lane_tests;"
    },
    {
      "id": "NFR-002",
      "path": "spec/non-functional/NFR-002-reproduce-native-builds.md",
      "role": "examined",
      "excerpt": "When the recorded native build command is run, the repository shall build without a Node or JVM runtime dependency."
    },
    {
      "id": "NFR-005::Verification",
      "path": "spec/non-functional/NFR-005-rust-verification-paths.md",
      "role": "examined",
      "excerpt": "Trace actual test functions to TC and AC identities\nwith imported `ix_trace_rs::trace` and canonical `#[trace(\"TC-...\", \"FR-...-AC-...\")]`\nattributes under the installed Quire/module grammar. Legacy doc-comment tags\nare not the convention for new tests."
    },
    {
      "id": "FR-042-AC-15",
      "path": "spec/functional/FR-042-publish-compiled-protocol-artifacts.md",
      "role": "examined",
      "excerpt": "`protocol_artifact::handoff::write_v1(directory)`, in the normal (non-dev) dependency surface behind the `handoff-writer` feature and adding no dependency, lets a downstream crate compile the authored recipe fresh and write a complete handoff"
    },
    {
      "id": "FR-050-AC-8",
      "path": "spec/functional/FR-050-publish-authenticated-temporal-artifacts.md",
      "role": "examined",
      "excerpt": "`protocol_artifact::handoff::write_v2(directory)` is FR-042-AC-15's `/2` counterpart: behind the same `handoff-writer` feature, it writes the complete authenticated handoff"
    },
    {
      "id": "extracted_command::extraction_request_obeys_the_build_feature",
      "path": "tests/it/extracted_command.rs",
      "role": "examined",
      "excerpt": "fn extraction_request_obeys_the_build_feature() {"
    }
  ],
  "scope": [
    {
      "id": "Makefile::feature-routing",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "export CI_DEFAULT_TARGET_DIR = $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)/ci-default-features\nexport CI_ALL_TARGET_DIR = $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)/ci-all-features"
    },
    {
      "id": "xtask::fixture-ownership",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "    cargo_directory: tempfile::TempDir,"
    },
    {
      "id": "xtask::current-trace",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "#[trace(\"NFR-005\")]"
    },
    {
      "id": "NFR-005::Verification",
      "path": "spec/non-functional/NFR-005-rust-verification-paths.md",
      "role": "examined",
      "excerpt": "Rust tool. Trace actual test functions to TC and AC identities\nwith imported `ix_trace_rs::trace` and canonical `#[trace(\"TC-...\", \"FR-...-AC-...\")]`"
    },
    {
      "id": "Makefile::clean-recipe",
      "path": "Makefile",
      "role": "examined",
      "excerpt": "cargo build --locked --workspace --no-default-features --target-dir \"$(CI_CLEAN_TARGET_DIR)\""
    },
    {
      "id": "xtask::inherited-clean-oracle",
      "path": "xtask/src/ci_clean_build_tests.rs",
      "role": "examined",
      "excerpt": ".args([\"--no-print-directory\", \"-rR\", \"-n\", \"ci-clean-build\"])"
    },
    {
      "id": "xtask::metacharacter-controls",
      "path": "xtask/src/ci_feature_lane_tests.rs",
      "role": "examined",
      "excerpt": "fn routing_oracle_rejects_safe_path_and_command_mutations() {"
    }
  ]
}
```

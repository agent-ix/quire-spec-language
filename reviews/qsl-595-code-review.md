---
id: SR-20261105101078
title: "Code and Rust review of QSL-595 LC7 QSL builds no binary"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-595 LC7; PR #2465 head 939ef1538; .github/workflows/ci.yml, Cargo.toml, Makefile, README.md, src/cli.rs, src/main.rs, tests/fixtures/checked-closure-consumer/src/lib.rs, tests/it/cli.rs, tests/it/compile_command.rs, tests/it/config_version_spine.rs, tests/it/deep_sources.rs, tests/it/main.rs, tests/it/no_binary.rs, tests/it/spine_run.rs, xtask/src/ci_clean_build_tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: reviews
---

## Summary

Ticket: QSL-595. Code review with the rust-review lane folded in, scoped to the PR #2465 diff (15 files, +52/-3823). The deletion itself is clean: `src/main.rs`, `src/cli.rs`, the `[[bin]] quire-spec` table and `default-run` are gone, and `cargo metadata` lists no bin target on the root package. The CI edits only drop the step that ran the deleted binary.

The exact `.github/workflows/ci.yml` diff, which is the whole workflow change:

```
@@ -47,7 +47,5 @@ jobs:
         run: RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps --all-features
       - name: Clean build
         run: cargo build --locked --workspace --no-default-features --target-dir target/clean
-      - name: Parse example
-        run: cargo run --locked --no-default-features -- parse test:parent fixture:1 tests/fixtures/parent.native
       - name: Registry bans (cargo-deny)
         run: cargo deny --workspace check bans
```

The Makefile `ci-clean-build` change removes only the matching `cargo run ... parse` line, and `xtask/src/ci_clean_build_tests.rs` drops the same argument list from its expected recipe.

The `checked-closure-consumer` move from `main.rs` to `lib.rs` is required by FR-287-AC-1. That workspace member lives under `tests/fixtures/`, which is not a developer-tool path, so its old `src/main.rs` was an auto-detected bin target. The conversion keeps the TC-1263 / FR-485-AC-3 test `checked_closure_external_consumer_runs`, which still calls `run()` and passes.

## Verdict

**CHANGES REQUESTED (not mergeable).** Two high findings will turn `make ci` red. The `test-suites` lane gate (xtask/src/test_suites.rs) still declares the deleted `src/main.rs` as an empty suite, which it reports as DeclaredButAbsent. The newly created `checked-closure-consumer` library adds a doc-test suite that runs 0 tests and is not declared, which the gate reports as Undeclared. Both affect the default-workspace and all-features-workspace lanes.

The new FR-287-AC-1 test allows a package whenever its absolute manifest path contains `/tools/`. Under a checkout path that contains `/tools/`, every package is allowed and the check passes without testing anything. Documentation still gives commands that no longer work: `cargo run -- run|compile` in the config-version example, and `quire run|compile <request.json>` and `quire format` with four labels in the README, none of which quire-driver main accepts.

What passed: `cargo clippy --workspace --all-targets` is clean. `no_binary::only_developer_tool_packages_list_a_bin_target` passes. The `checked-closure-consumer` unit test passes. The xtask `ci_clean_build` tests pass (3).

The loss of test coverage from the deleted test files is recorded in the gap-analysis artifact for this ticket, not here.

## Scope

- .github/workflows/ci.yml (examined): only the `Parse example` step is removed.
- Makefile ci-clean-build (examined): only the `cargo run ... parse` line is removed.
- Cargo.toml (examined): `default-run` and `[[bin]] quire-spec` are removed, and nothing else changes.
- tests/fixtures/checked-closure-consumer/src/lib.rs (examined): it is now a library, and the TC-1263 test is kept.
- tests/it/no_binary.rs (examined): the FR-287-AC-1 test.
- xtask/src/ci_clean_build_tests.rs (examined): the expected recipe matches the Makefile.
- README.md (examined): the binary instructions are rewritten.
- Repo-wide grep for `quire-spec`, `CARGO_BIN_EXE_quire-spec` and `cargo run --` (examined). Results: xtask/src/test_suites.rs:208, examples/config-version/README.md:14-15, src/emit_checked_v2.rs:13,246, docs/native-error-codes.md:78, README.md:381, docs/images/figures.py and layers-*.svg. No xtask, script, Makefile target or workflow outside this diff runs the binary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | WORKSPACE_EMPTY still declares the deleted `src/main.rs` unit-test suite; the test-suites gate reports DeclaredButAbsent in both workspace lanes, so make ci fails | xtask/src/test_suites.rs:207-211 |
| FND-002 | high | checked-closure-consumer is now a library whose doc-test suite runs 0 tests (measured) and is not declared in WORKSPACE_EMPTY; the gate reports Undeclared, so make ci fails | xtask/src/test_suites.rs:186-272 |
| FND-003 | medium | The FR-287-AC-1 test allows any package whose absolute manifest path contains `/tools/`. A checkout under a `tools` directory makes every package pass, and the check fails wrongly on Windows path separators. Compare the manifest dir relative to `workspace_root` with the prefix `tools/` instead | tests/it/no_binary.rs:20-23 |
| FND-004 | medium | The config-version example still documents `cargo run --locked -- run` and `-- compile`, which now fail because the root package has no bin target | examples/config-version/README.md:14-15 |
| FND-005 | medium | The README documents `quire run <run.json>`, `quire compile <compile.json>` and `quire format` with four labels. quire-driver main `run`/`compile` take a source with `--limit`/`--engine` (quire-cli/src/cli.rs RunOptions::parse) and `format` takes a path only. After this PR no executable reads native-run/1 or native-compile/1 request files | README.md:230 |
| FND-006 | low | Stale `quire-spec` binary references remain in doc comments, docs and figures | src/emit_checked_v2.rs:13 |

## Coverage

All 15 changed files in the PR were read. Focused checks were run with the shared target and debug=0: `cargo test --test it no_binary` (pass), `cargo test -p checked-closure-consumer` (1 unit test passes; the doc-tests run 0), `cargo test -p xtask ci_clean_build` (3 pass) and `cargo clippy --workspace --all-targets` (clean). The full `make ci` gate was not run, as the brief requires. FND-001 and FND-002 were derived from `evaluate_declared` in xtask/src/test_suites.rs (lines 1082-1110) together with the measured empty doc-test suite.

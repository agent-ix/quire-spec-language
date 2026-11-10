---
id: SR-20261109172912000128
title: "QSL-1813 command implementation scoped semantic gap review"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1813; PR #2641; qsl-command; qsl-foundation/src/diagnostic.rs; qsl-replay/src/outcome.rs; root imports and boundary tests; FR-100; FR-287-AC-10/11; FR-001-AC-16"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
---

## Summary

Manual scoped semantic and trace review of the command owner APIs and real external-consumer tests. No scoped unowned behavior, stubs, vacuous assertions or inflated AC bindings found.

## Verdict

**PASS for the manual scoped semantic delta review only; no whole-repository computed-matrix assurance or full-gate PASS is claimed.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Evidence

The full changed code, producers and tests were read against the examined obligations. Frozen head: `64beae30adb80dedb2c26aa86bf50ace8a641dd9`. Ticket: QSL-1813. The review covers the new non-core qsl-command package, moved command/formal-source tree, public Foundation converter, borrowed replay refusal producer, root manifests/imports/boundary tests, the carrier wording, and custody repair. No CI workflow or qualified-core exemption changed.

The strict request decoder, native serializer and formal-source implementation are moved within this repository, not copied or wrapped at the old root. The root has only a dev dependency for its acceptance tests; the new crate depends inward on replay and other sanctioned core libraries. Filesystem intake remains outside the qualified core. Production dependency edges do not enable test-support.

prepare_run reuses the bounded reader, validates the existing closed request, retains admitted models and dependency sources, and returns owned function or clause input. PreparedFunctionRun keeps native source correspondence until either its input is consumed or the native runner executes it. The private boxed storage and boxed clause bound enum size without a semantic alternative or dummy call. The package name remains the authored identity label. Package/domain intake and the existing declared-edition preflight precede compilation; the preparation entry calls neither a compiler stage nor an evaluator/provider.

compile_outcome retains the actual result of spine::package and passes that result to the core from_package producer before consuming its staged value. compile uses the same pipeline and emitted bytes. RunError projects typed variants through existing core factories, retaining category, code, source cause/label, optional region, and real reached stage. Pre-stage intake and request-level cancellation/fault have no invented stage. Closed typed matches replace no known case with a valid default. The core from_run_refusal extraction preserves the former cancellation, fault, supplied-phase and catalog branches and lets from_run delegate to that single mapping.

One read_typed supplies the private native/canonical source-failure pair. The now-borrowed Foundation converter retains its previous seven-case code mapping, identity-only native cause, offered-byte location, byte-budget context and native total-span origin fallback; the original refusal is then consumed by CST, so a canonical absent region stays absent. Existing native envelopes use their original typed native diagnostic. Error source chains retain the underlying native diagnostic.

The external tests assert independently chosen codes/categories, source identity and path, nonempty packages/libraries, concrete argument values and accounting, and run the prepared values after deleting every source/request file. They compare a real independently composed package result and bytes, cover actual source/check/emit refusals and typed cancellation/fault, and exercise all native projection causes at ceilings below/above 1 MiB with located, absent and unrenderable regions. Native equality controls supplement rather than replace the literal field assertions. Changed root probes check normal/build core edges, actual failed retired imports with each expected diagnostic, and in-process stream silence. Existing inherited compile/formal-source/run/ConfigVersion tests remain intact apart from direct owner imports and fixture locations.

Custody checks independently cmp the restored SR108 at reviews/qsl-1813-wire-code-review.md against its original specification artifact and SR125 at reviews/qsl-1813-wire-implementation-code-review.md against its original implementation artifact. Both are byte-identical; no ID or finding content was rewritten.

Independent confined checks passed under the required flock/nice/ionice, Rust 1.98.1, jobs=2, RUST_TEST_THREADS=2 and this worktree's target: workspace formatting; all-target/all-feature Clippy for qsl-command, qsl-foundation and qsl-replay; qsl-command's 14 unit and 10 external tests. Logs: /tmp/qsl1813-command-code-review/check-{fmt,lint,package}.log. The author's preserved records additionally show 73 inherited acceptance tests and four quiet changed-boundary probes passing. No metadata subprocess was launched by this review. The source carrier document passed explicit-path quire validation.

Known baseline: the unchanged no_binary::only_developer_tool_packages_list_a_bin_target fails because qsl-probability-check declares a bin outside the FR-287 developer-tool locations. Its source originated in upstream #2617/QSL-1826, and neither that crate nor the guard is changed in #2641. Evidence: /tmp/qsl1813-command-focused-quiet-no-binary.log. The lead has routed and is separately fixing the baseline. This review does not waive it or claim a clean full gate; final affected aggregate and baseline integration remain with the lead after custody.

No applicable AssuranceProfile was found under spec/. No assurance exception was applied. Cargo-deny, broad workspace runtime, docs and final aggregate were not repeated in this confined reviewer pass; their checkpoint belongs to the lead. The new dependency surface uses existing workspace versions plus the ordinary new path package. The exact model identifier is unavailable in the harness metadata and is recorded honestly as unavailable.

## Coverage

Plan completion: not assessed

Manual scoped acceptance-to-test, reverse-ownership, stub and semantic review explicitly authorized by the dispatch. FR-287-AC-10, FR-287-AC-11, FR-001-AC-16 and the changed FR-100 carrier text are examined; the remaining requirements are context. TC-771 owner/preparation and failure-projection steps and TC-424 step 11 are inspected to ground those obligations. This does not newly qualify the entire TC-771 procedure, driver process behavior, or every platform.

Whole-repository computed matrix and runtime coverage were NOT RUN and are not claimed: the dispatch prohibits repeating earlier 521-second/295-second probes that produced no JSON. No repository-wide gap-analysis PASS, QSL-1813 completion or driver #141 qualification is asserted.

## Examined Units

```yaml
{
  "scope": [
    {
      "id": "FR-287-AC-10",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "examined",
      "excerpt": "An external Rust consumer with no dependency on `quire-spec-language` calls `qsl_command::command::compile_outcome` over AC-2's compile request and `prepare_run` over its bounded run request."
    },
    {
      "id": "FR-287-AC-11",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "examined",
      "excerpt": "Missing request files and malformed request JSON produce the command owner's outcome document with the original catalog code and category and `last_stage` null; source, check and emit refusals retain their owning typed cause, category, reached stage and available region. A source identity refusal retains its exact typed label with no fabricated locus. The command's existing native library result and refusal behavior remain unchanged."
    },
    {
      "id": "FR-001-AC-16",
      "path": "spec/functional/FR-001-read-exact-source.md",
      "role": "examined",
      "excerpt": "A caller outside Foundation borrows each `SourceReadCause` refusal through the public native projection, then consumes that same refusal through CST's projection."
    },
    {
      "id": "FR-100",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "`PreparedRun::Function(PreparedFunctionRun)` and\n`PreparedRun::Clause(Box<qsl_replay::spine::ClauseRunRequest>)`; the owned\nindirection bounds the enum's stack size without changing the clause request."
    },
    {
      "id": "FR-027",
      "path": "spec/functional/FR-027-export-compiled-native-package.md",
      "role": "context_only",
      "excerpt": "The command SHALL construct that document with\n`OutcomeDocument::from_package` over the actual typed package result,\nbefore discarding its staged value."
    },
    {
      "id": "FR-284",
      "path": "spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md",
      "role": "context_only",
      "excerpt": "The crates above the core are QSL's `qsl-analyze` (layer A), `qsl-inspect`\n(layer P), `qsl-jit` (layer X) and `qsl-command` (the FR-027 and FR-100\nrequest-file adapter), and the driver's `quire-plugin-host`,\n`quire-aot` and `quire-cli` (ADR-029 CB-3 item 3). Each may\ndepend on the core. No core crate depends on any of them."
    },
    {
      "id": "FR-287-AC-1",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "context_only",
      "excerpt": "`cargo metadata --no-deps --format-version 1` in the QSL repository lists a `bin` target only in the developer-tool packages (`xtask`, the packages whose manifest directory is under `tools/`, `qsl-string-edge` and `qsl-bench`); the root package `quire-spec-language` has no `bin` target and no `default_run`; and no workspace member outside those packages lists one of them as a normal or build dependency."
    }
  ],
  "bindings": [
    {
      "test_id": "compilation_retains_the_actual_core_document_and_the_existing_artifact",
      "ac_id": "FR-287-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "preparation_retains_owned_inputs_and_the_existing_run_outcomes",
      "ac_id": "FR-287-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "preparation_does_not_compile_the_source_and_retains_caller_byte_limits",
      "ac_id": "FR-287-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "intake_failures_have_their_original_codes_and_no_invented_stage",
      "ac_id": "FR-287-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "stage_refusals_preserve_the_reached_stage_and_available_location",
      "ac_id": "FR-287-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "source_intake_retains_both_native_and_canonical_projections_of_one_failure",
      "ac_id": "FR-287-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "borrowed_run_failure_projection_preserves_cancellation_and_fault_categories",
      "ac_id": "FR-287-AC-11",
      "trace": "correct"
    },
    {
      "test_id": "native_projection_borrows_all_typed_causes_and_preserves_the_original_for_cst",
      "ac_id": "FR-001-AC-16",
      "trace": "correct"
    },
    {
      "test_id": "offered_byte_coordinates_and_unrenderable_fallback_do_not_change_the_typed_region",
      "ac_id": "FR-001-AC-16",
      "trace": "correct"
    },
    {
      "test_id": "source_read_and_direct_native_projection_have_identical_fields",
      "ac_id": "FR-001-AC-16",
      "trace": "correct"
    },
    {
      "test_id": "command_intake_is_a_non_core_leaf_consumer",
      "ac_id": "FR-287-AC-10",
      "trace": "correct"
    },
    {
      "test_id": "retired_mapped_and_extraction_imports_fail_in_a_downstream_crate",
      "ac_id": "FR-287-AC-10",
      "trace": "correct"
    }
  ]
}
```

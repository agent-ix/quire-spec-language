---
id: SR-20261109172912000115
title: "QSL-1813 noncore command boundary spec-review/ears-conformance review"
type: SpecReview
analysis: ears-conformance
review_set: subset
scope: "agent-ix/quire-spec-language; QSL-1813; PR #2635; command-owner ADR/FR/TC changes"
relationships:
  - { target: ix://agent-ix/quire-spec-language/ADR-011, type: references }
  - { target: ix://agent-ix/quire-spec-language/ADR-029, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-027, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-100, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-284, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-287, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1262, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-771, type: references }
---
## Summary

Independent spec-review/ears-conformance review of frozen PR #2635. No in-scope defect found.

## Assurance Context

No applicable AssuranceProfile exists under spec/. The lead selected a subset of the base, EARS, integrity, dependency, scope-boundary and failure-domain methods for this ownership and API change. No enforced review workflow or human acceptance is claimed.

## Verdict

PASS for specification readiness after exact artifact custody. Implementation, driver acceptance, full matrix coverage and QSL-1813 completion remain unqualified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Analysis

The changed normative SHALL statements name the command/preparation/projection actor and concrete responses: shared compilation, bounded typed intake, no execution, retained diagnostic fields and one existing serializer. They describe ubiquitous constraints; no event/state ambiguity changes implementation. Engine grammar validation reports 28 unchanged advisories elsewhere and none on these edited FR statements. ADR rationale and test procedures are not EARS requirement statements.

## Verification and Limits

Frozen head a0e2e7c0d1fc0007493041df91f674d81bf6e68b verified. Full scoped specification validation exits 0 and reports 28 existing grammar advisories outside the changed statements; diff check passes. Manual review traced the existing command decoder, bounded intake, native result and core outcome constructors to establish feasible ownership, without qualifying unimplemented new APIs. No Cargo, Kani, full matrix probe, source write, push or merge was performed. The invalid proposed filesystem move into replay was not part of this PR and was not reviewed as a candidate. Evidence/risk calibration and Jev criterion-strength methods were not selected; no such assurance is claimed.

## Reviewed Units

```json
{
  "clean": true,
  "scope": [
    {
      "id": "ADR-011",
      "path": "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md",
      "role": "examined",
      "excerpt": "- FR-027 and FR-100's request-file handling belongs to the non-core\n  `qsl-command` crate. Its `command` and `formal_source` modules replace\n  the existing root modules, with every consumer repointed and no root\n  compatibility module or re-export. The driver CLI calls this crate for\n  bounded file intake and typed preparation, then uses the existing driver\n  execution and provider pipeline. No filesystem access moves into the\n  qualified core (FR-284).\n- Lifecycle surfaces (providers, plugins, AOT and JIT) call the\n  same stage APIs and never bypass \u00a73.\n- #29 (CLI parse and dispatch) and #133"
    },
    {
      "id": "ADR-011",
      "path": "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md",
      "role": "examined",
      "excerpt": "FR-027 and FR-100 request-file preparation and orchestration"
    },
    {
      "id": "ADR-029",
      "path": "spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md",
      "role": "examined",
      "excerpt": "The non-core crate `qsl-command` owns that existing request decoder,\nbounded filesystem intake, typed preparation and native-result projection.\nThe public module is `qsl_command::command`, and its source-coordinate\nadapter is `qsl_command::formal_source`; neither remains in the QSL root\npackage. The driver's `quire-cli` package depends on this owner directly,\nwithout adding a QSL root dependency to any driver package or moving file\naccess into `qsl-replay`. The core list and ambient-input prohibition in\nCB-2 and FR-284 are unchanged."
    },
    {
      "id": "FR-027",
      "path": "spec/functional/FR-027-export-compiled-native-package.md",
      "role": "examined",
      "excerpt": "The request-file owner is the non-core `qsl_command::command` module\n([ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)\n\u00a76.1). Its existing `compile(&std::path::Path)` returns the package bytes.\nFor the driver, `compile_outcome(&std::path::Path)` returns\n`Result<CompileOutcome, Box<RunError>>`, where `CompileOutcome` exposes\n`package_bytes() -> &[u8]` and `document() -> &qsl_replay::OutcomeDocument`.\nThe command SHALL construct that document with\n`OutcomeDocument::from_package` over the actual typed package result,\nbefore discarding its staged value. The existing `compil"
    },
    {
      "id": "FR-100",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "`prepare_run(path: &std::path::Path, limits: RunLimits)` returns\n`Result<PreparedRun, Box<RunError>>` from the same module. Its variants are\n`PreparedRun::Function(PreparedFunctionRun)` and\n`PreparedRun::Clause(qsl_replay::spine::ClauseRunRequest)`.\n`PreparedFunctionRun::into_input(self) -> FunctionRunInput` exposes these\nowned typed members:"
    },
    {
      "id": "FR-100",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "`qsl_foundation::Source`, with its offered labels, path and admitted bytes |"
    },
    {
      "id": "FR-100",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "The preparation operation SHALL reuse the existing closed request decoder\nand bounded intake, retaining relative-path resolution, source identity,\nselected-file and byte ceilings, domain packages, libraries and call\nvalues. It SHALL perform no compiler, evaluator, engine or provider call.\nThe existing in-process `run` operations SHALL share this preparation\nrather than keeping another request decoder or intake path."
    },
    {
      "id": "FR-100",
      "path": "spec/functional/FR-100-run-a-named-function-through-the-spine.md",
      "role": "examined",
      "excerpt": "The driver passes the prepared values to its existing execution and\nconfigured-provider pipeline. Engine selection, provider configuration,\nsignal handling, cancellation and object environment remain the driver's\nresponsibility. The preparation API adds no execution callback or backend\nselector. Completed and refused runs are rendered with the existing\nproducer-owned FR-286 outcome constructors, not by reading the native\nresult's JSON or rebuilding an outcome in the CLI."
    },
    {
      "id": "FR-284",
      "path": "spec/functional/FR-284-keep-the-qualified-core-separable-by-crate.md",
      "role": "examined",
      "excerpt": "The crates above the core are QSL's `qsl-analyze` (layer A), `qsl-inspect`\n(layer P), `qsl-jit` (layer X) and `qsl-command` (the FR-027 and FR-100\nrequest-file adapter), and the driver's `quire-plugin-host`,\n`quire-aot` and `quire-cli` (ADR-029 CB-3 item 3). Each may\ndepend on the core. No core crate depends on any of them."
    },
    {
      "id": "FR-287",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "examined",
      "excerpt": "The request-file owner is the non-core crate `qsl-command`, whose public\nmodules are `qsl_command::command` and `qsl_command::formal_source`.\nThese replace the existing root modules, with no compatibility re-export.\nThe driver's `quire-cli` package calls this owner directly for bounded\nrequest preparation and compilation, without depending on the QSL root\npackage. No core crate depends on `qsl-command`, and its filesystem intake\nstays outside the qualified core\n([FR-284](FR-284-keep-the-qualified-core-separable-by-crate.md))."
    },
    {
      "id": "FR-287",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "examined",
      "excerpt": "`RunError::outcome_document(operation: qsl_replay::Operation)` returns a\n`qsl_replay::OutcomeDocument` through the command owner's pure projection\nof its existing typed failure. It SHALL retain the original category,\ncatalog code, typed cause and available location. It SHALL retain the\nspine's reached stage for a stage refusal and `last_stage` null for a\npre-stage intake failure, cancellation or fault as FR-286 specifies.\nSource identity causes retain FR-001's typed `label` and optional region.\nThe projection uses the core outcome types and their one serializer;\nneither it nor the driver reads"
    },
    {
      "id": "FR-287-AC-7",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "examined",
      "excerpt": "Test (TC-1262) |"
    },
    {
      "id": "FR-287-AC-8",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "examined",
      "excerpt": "Test (TC-1262) |"
    },
    {
      "id": "FR-287-AC-9",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "examined",
      "excerpt": "Test (TC-1262) |"
    },
    {
      "id": "FR-287-AC-10",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "examined",
      "excerpt": "Test |"
    },
    {
      "id": "FR-287-AC-11",
      "path": "spec/functional/FR-287-reach-qsl-through-the-driver-cli.md",
      "role": "examined",
      "excerpt": "Test |"
    },
    {
      "id": "TC-1262",
      "path": "spec/test-cases/TC-1262-the-quire-frontend-parses-encodes-writes-and-exits.md",
      "role": "examined",
      "excerpt": "6. Run `compile` and `run` in machine mode over valid requests and a refused request; compare stdout with the producer-owned FR-286 outcome bytes, from a relative and an absolute request path in two working directories. Check that the CLI neither substitutes the native library products nor writes a refusal envelope in place of the document."
    },
    {
      "id": "TC-771",
      "path": "spec/test-cases/TC-771-qsl-builds-no-binary-and-the-driver-cli-refuses-unknown-arguments.md",
      "role": "examined",
      "excerpt": "3. From an external consumer of `qsl-command` with no QSL root dependency, call `compile_outcome` over step 2's compile request and compare its bytes with `compile` and its document with `OutcomeDocument::from_package` over the actual composed package result. Call `prepare_run` over the bounded run request, inspect its typed inputs, then pass those inputs through the existing spine and render the typed outcome. Compare the result and category with the existing command run. Include supplied domain packages and libraries, and request paths resolved relatively and absolutely.\n4. Through the comma"
    },
    {
      "id": "TC-771",
      "path": "spec/test-cases/TC-771-qsl-builds-no-binary-and-the-driver-cli-refuses-unknown-arguments.md",
      "role": "examined",
      "excerpt": "4. Through the command owner, refuse a missing request file, malformed request JSON, a blank source label and actual source/check/emit failures. Render each typed `RunError` through `outcome_document`; compare its code, cause, category, reached stage and available location with the original typed failure. Confirm the root has no `command` or `formal_source` definition or re-export and that the unchanged core direction and ambient-input gates pass."
    },
    {
      "id": "FR-286",
      "path": "spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md",
      "role": "context_only",
      "excerpt": "`quire-outcome/1` is the outcome document of every operation, `execute`\nincluded; it replaces FR-100's `spine-run-result/1` when the driver's verbs\nland (ADR-029 CB-1)."
    }
  ],
  "findings": []
}
```

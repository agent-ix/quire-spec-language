---
id: TC-771
title: "QSL builds no user binary, and command is a library operation that writes nothing"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: verifies
---
# TC-771: QSL builds no user binary, and command is a library operation that writes nothing

## Description

Verify QSL's side of the CLI boundary: the workspace's only executable targets are in the developer-tool packages (`xtask`, `tools/*`, `qsl-string-edge`, `qsl-bench`), and `command` returns typed outcomes without touching the process streams.

Scope: FR-287-AC-1, FR-287-AC-2, FR-287-AC-10, FR-287-AC-11.

## Test Procedure

1. Run `cargo metadata --no-deps --format-version 1` in the QSL repository. For each package, read its `bin` targets, its manifest directory relative to the repository root, its `default_run` and its dependencies.
2. With stdout and stderr captured, call `command` in process with FR-027's compile request over `tests/fixtures/spine-compile.native`, then with FR-100-AC-6's `call.accounting` `{"work_units": 0}` run request, and map each outcome's category through FR-285.
3. From an external consumer of `qsl-command` with no QSL root dependency, call `compile_outcome` over step 2's compile request and compare its bytes with `compile` and its document with `OutcomeDocument::from_package` over the actual composed package result. Call `prepare_run` over the bounded run request, inspect its typed inputs, then pass those inputs through the existing spine and render the typed outcome. Compare the result and category with the existing command run. Include supplied domain packages and libraries, and request paths resolved relatively and absolutely.
4. Through the command owner, refuse a missing request file, malformed request JSON, a blank source label and actual source/check/emit failures. Render each typed `RunError` through `outcome_document`; compare its code, cause, category, reached stage and available location with the original typed failure. Confirm the root has no `command` or `formal_source` definition or re-export and that the unchanged core direction and ambient-input gates pass.

Tag the tests `#[trace("TC-771", "<AC id>")]`.

## Expected Results

- Step 1: every `bin` target belongs to `xtask`, `qsl-string-edge`, `qsl-bench` or a package whose manifest directory starts with `tools/`; `quire-spec-language` has no `bin` target and a null `default_run`; no package outside those has a normal or build dependency (`kind` null or `build`) naming one of them.
- Step 2: the first outcome is success, exit 0; the second is incomplete, exit 22; the captured streams are empty.
- Step 3: the external consumer compiles; package bytes and producer-owned documents are equal. Preparation retains offered identity, path, bytes, package name, packages, libraries, call arguments and accounting without executing a compiler, evaluator, engine or provider. Executing its input yields the same completed result or exhausted limit and category as the existing command entry. Relative and absolute resolution of the same request yields the same machine document.
- Step 4: pre-stage intake refusals have `last_stage` null; stage refusals retain their original reached stage, cause and location. The blank-label refusal retains the exact typed label, with no fabricated region. Native library results and refusal envelopes remain unchanged; no core crate gains filesystem access or a dependency on `qsl-command`, and no root compatibility module remains.

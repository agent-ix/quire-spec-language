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

Verify QSL's side of the CLI boundary: the workspace's only executable targets are the developer tools in `xtask` and under `tools/`, and `command` returns typed outcomes without touching the process streams.

Scope: FR-287-AC-1, FR-287-AC-2.

## Test Procedure

1. Run `cargo metadata --no-deps --format-version 1` in the QSL repository. For each package, read its `bin` targets, its manifest directory relative to the repository root, its `default_run` and its dependencies.
2. With stdout and stderr captured, call `command` in process with FR-027's compile request over `tests/fixtures/spine-compile.native`, then with FR-100-AC-6's `call.accounting` `{"work_units": 0}` run request, and map each outcome's category through FR-285.

Tag the tests `#[trace("TC-771", "<AC id>")]`.

## Expected Results

- Step 1: every `bin` target belongs to `xtask` or to a package whose manifest directory starts with `tools/`; `quire-spec-language` has no `bin` target and a null `default_run`; no package outside `xtask` and `tools/` has a normal or build dependency (`kind` null or `build`) naming `xtask` or a `tools/` package.
- Step 2: the first outcome is success, exit 0; the second is incomplete, exit 22; the captured streams are empty.

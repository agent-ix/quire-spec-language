---
id: TC-771
title: "QSL builds no binary, and command is a library operation that writes nothing"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: verifies
---
# TC-771: QSL builds no binary, and command is a library operation that writes nothing

## Description

Verify QSL's side of the CLI boundary: the workspace builds no executable, and `command` returns typed outcomes without touching the process streams.

Scope: FR-287-AC-1, FR-287-AC-2.

## Test Procedure

1. Run `cargo build --workspace` in a clean QSL checkout and list the executable files in its target directory.
2. With stdout and stderr captured, call `command` in process with FR-027's compile request over `tests/fixtures/spine-compile.native`, then with FR-100-AC-6's `work_units` 0 run request, and map each outcome's category through FR-285.

Tag the tests `#[trace("TC-771", "<AC id>")]`.

## Expected Results

- Step 1: there are none.
- Step 2: the first outcome is success, exit 0; the second is incomplete, exit 22; the captured streams are empty.

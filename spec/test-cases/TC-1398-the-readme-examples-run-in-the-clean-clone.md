---
id: TC-1398
title: "The README examples run in the clean clone and give their documented outputs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-017
    type: verifies
---
# TC-1398: The README examples run in the clean clone and give their documented outputs

## Description

Check the outputs of the examples the clean-clone gate runs.

Scope: NFR-017-AC-2.

## Test Procedure

In the clone of a passing `make clean-clone-check` run at `main`:

1. List the directories `config_version_fixtures` wrote under
   `<tmp>/config-version`.
2. List the files `requests.sh` wrote in `healthy-parent`.
3. Read `<tmp>/package.json`, which `emit_checked_v2` wrote, with QSL's I2
   reader and the switch model as evidence.
4. Read the result of the walkthrough test and its per-case exit codes.

Tag the tests `#[trace("TC-1398", "NFR-017-AC-2")]`.

## Expected Results

- Step 1: `healthy-parent`, `violating-parent`, `dangling-parent`,
  `incomplete-population` and `cycle`.
- Step 2: `run.json` and `compile.json`.
- Step 3: `emit_checked_v2` exited 0 and the reader reads the package back.
- Step 4: the test passes with exit codes 0, 10, 20, 22 and 10.

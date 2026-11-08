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

Run `make clean-clone-check` at the head of `main` and read step 5 of its
report: the case directories `config_version_fixtures` wrote, the files
`requests.sh` wrote in `healthy-parent`, the exit of `emit_checked_v2` and
the I2 read-back of its output, and the walkthrough test's result and
per-case exit codes.

Tag the tests `#[trace("TC-1398", "NFR-017-AC-2")]`.

## Expected Results

- The directories are `healthy-parent`, `violating-parent`,
  `dangling-parent`, `incomplete-population` and `cycle`.
- `healthy-parent` holds `run.json` and `compile.json`.
- `emit_checked_v2` exited 0 and the reader reads the package back.
- The walkthrough test passes with exit codes 0, 10, 20, 22 and 10.

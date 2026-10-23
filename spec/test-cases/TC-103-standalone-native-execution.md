---
id: TC-103
title: "A 0-draft request is refused before runtime execution"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
## Description

Integration, P1: verify FR-100-AC-2's `0-draft` refusal before the run
command reads runtime inputs or invokes an operation.

## Test Procedure

Prepare native-run/1 requests whose program source declares `0-draft`,
including one with runtime selections and one with `call` and `libraries`.
Call `command` in process and capture the typed outcome and process streams.

## Expected Results

Every request refuses `unknown_edition` at stage `profile`, exit 20, with no
result document; the refusal names the program file and `0-draft`. No runtime
file is read and no execution occurs. Tag the test
`#[trace("TC-103", "FR-100-AC-2")]`.

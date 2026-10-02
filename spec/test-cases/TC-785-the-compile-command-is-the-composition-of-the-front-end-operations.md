---
id: TC-785
title: "The compile command is the composition of the front-end operations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
# TC-785: The compile command is the composition of the front-end operations

## Description

Verify that QSL `command`'s compile request handling writes what FR-278's operations produce, and exits by FR-285.

Scope: FR-027-AC-11.

## Test Procedure

1. For FR-027-AC-5's, AC-9's and AC-10's requests, run the command and compose `parse`, `select`, `check` and `package` over the same source, domain packages and libraries.
2. For each FR-027-AC-8 source, run the command and the composed operations.

Tag the tests `#[trace("TC-785", "<AC id>")]`.

## Expected Results

- Step 1: the command's bytes equal `package`'s output.
- Step 2: the refusing operation's stage and cause code equal the command's, and its FR-285 exit code equals the command's exit status.

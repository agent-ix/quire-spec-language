---
id: TC-430
title: "Run requests and their outputs carry the two source labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
---
# TC-430: Run requests and their outputs carry the two source labels

## Description

Verify that the native-run/1 request wire requires both FR-001 labels on a
`1-draft` program source, that `spine-run-result/1` renders them with the
source digest and no other identity member, and that a request missing a
label refuses as malformed. This catches a label defaulted by the command.

Scope: FR-026-AC-6.

## Test Procedure

1. Run a `1-draft` native-run/1 request over
   `tests/fixtures/spine-compile.native` whose program source names
   `authority` `agent-ix` and `identity` `p`, with `call`
   `{"function": "seven", "arguments": []}`.
2. Run it again without the program source's `authority`, then without its
   `identity`, then with a blank `authority`.

Tag the tests `#[trace("TC-430", "FR-026-AC-6")]`.

## Expected Results

- Step 1: exit 0; the result's `source` names `agent-ix`, `p` and the source
  digest, and no other identity member.
- Step 2: the first two refuse at the request stage with `invalid-request`,
  exit 20; the blank label refuses with `invalid_source_identity`, cause
  `blank-label`, `label` `authority`.

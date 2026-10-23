---
id: TC-430
title: "Run requests and their outputs carry the two source labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: verifies
---
# TC-430: Run requests and their outputs carry the two source labels

## Description

Verify that the native-run/1 request wire requires both FR-001 labels on a
`1-draft` program source, that `spine-run-result/1` renders them with the
source digest and no other identity member, and that a request missing a
label refuses as malformed. This catches a label defaulted by the command.
It also verifies that the request admits no program `extraction` member in
any build.

Scope: FR-026-AC-6, FR-031-AC-5.

## Test Procedure

1. Run a `1-draft` native-run/1 request over
   `tests/fixtures/spine-compile.native` whose program source names
   `authority` `agent-ix` and `identity` `p`, with `call`
   `{"function": "seven", "arguments": []}`.
2. Run it again without the program source's `authority`, then without its
   `identity`, then with a blank `authority`.
3. Run a `1-draft` native-run/1 request whose program carries an
   `extraction` body record naming `authority` `agent-ix`, `identity` `b`,
   `document` `B` and a `digest`, through the command built with no features
   and through the command built with `--all-features`.

Tag the tests of steps 1 and 2 `#[trace("TC-430", "FR-026-AC-6")]` and
step 3 `#[trace("TC-430", "FR-031-AC-5")]`.

## Expected Results

- Step 1: exit 0; the result's `source` names `agent-ix`, `p` and the source
  digest, and no other identity member.
- Step 2: the first two refuse at the request stage with `invalid-request`,
  exit 20; the blank label refuses with `invalid_source_identity`, cause
  `blank-label`, `label` `authority`.
- Step 3: each build refuses `invalid-request` at stage `request`, exit 20,
  with empty stdout.

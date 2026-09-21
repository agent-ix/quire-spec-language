---
id: TC-188
title: "The replay request refuses an unknown version or an out-of-set profile/capability identifier before recompilation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: verifies
---
# TC-188: The replay request refuses an unknown version or an out-of-set profile/capability identifier before recompilation

## Description

Verify that a replay request whose `contract_version` is not exactly
`quire.native-runtime/v1`, or whose `replay` property names a
capability-vocabulary or semantic-profile identifier outside its closed
set, refuses at decode with a structured cause and produces no partial
request, and that this refusal happens before any recompilation would be
attempted. This is decode-time, type-level rejection — distinct from the
execution-time refusals (arity mismatch, stale `package_id`, out-of-domain
value) that belong to #243's executor and are explicitly out of this
ticket's scope. A wrong implementation this test would catch: a decoder
that accepts an unrecognized profile identifier by falling back to a
"default" profile silently, which would let a request from an old client
compile against the wrong semantic profile instead of refusing outright.
Scope: FR-071-AC-4.

## Test Procedure

1. Construct a well-formed replay request wire payload and mutate its
   `contract_version` to an unrecognized string; decode it.
2. Construct a well-formed replay request wire payload and mutate its
   `replay.semantic_profile_selections` to name a profile identifier
   outside the closed set; decode it.
3. Construct a well-formed replay request wire payload and mutate its
   capability-vocabulary-scoped fields to name a kind outside the FR-290
   vocabulary; decode it.
4. Instrument or otherwise observe that no recompilation, package lookup,
   or byte-provision read is attempted before each of the three refusals.

## Expected Results

- Each of the three malformed payloads refuses at decode with a structured,
  typed cause, and no request value is returned.
- No recompilation, package lookup, or byte-provision access is observed
  before any of the three refusals.

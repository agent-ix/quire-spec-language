---
id: TC-1015
title: "Complete protocol forms and memory forms refuse without the model profile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-361
    type: verifies
---
# TC-1015: Complete protocol forms and memory forms refuse without the model profile

## Description

Verify that the FR-308 complete protocol forms and the FR-309 memory forms refuse without the model profile. Scope: FR-361-AC-4.

## Test Procedure

1. Compile TC-887's protocol with a `replicated` role, a bounded channel and a quorum `join` under the header profile `quire.value.complete/v1`.
2. Compile a protocol with a `parallel` memory clause, an attempt `ordering` and a `fence` under the same header.
3. Compile both under `quire.model.complete/v1`.

## Expected Results

1. Refused `unsupported_construct`/`declaration-form`, family `protocol-complete`, at the form's span.
2. Refused the same way, family `memory`.
3. Both build and check as FR-308 and FR-309 state.

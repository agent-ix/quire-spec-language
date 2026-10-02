---
id: TC-792
title: "StateModel causes raised under Value and ProtocolClause evaluation render with the state-model prefix"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-301
    type: verifies
---
# TC-792: StateModel causes raised under Value and ProtocolClause evaluation render with the state-model prefix

## Description

Verify that a model cause raised inside an enclosing family's evaluation keeps
`StateModel` attribution. Scope: FR-301-AC-1, FR-301-AC-2.

## Test Procedure

1. Compile a unit with a `Value` function whose body is `lookup<T>(p, r)`;
   run it through the spine run entry with a population in which `r` is
   absent and the lookup is `absent refused`.
2. Compile a unit with a state clause whose body calls a dispatched query
   operation whose effective precondition is `false` for the receiver; run
   the clause over a snapshot holding that receiver.

## Expected Results

1. `FamilyResult::Refused`; the cause's catalog code begins `state-model`,
   not `value`; category refusal.
2. `FamilyResult::Undefined` with cause `precondition-false`; catalog code
   begins `state-model`, not `protocol-clause`; category undefined.

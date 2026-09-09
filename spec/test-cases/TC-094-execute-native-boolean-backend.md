---
id: TC-094
title: "Compare generated Boolean truth and source activation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-009
    type: verifies
  - target: ix://agent-ix/quire-spec-language/IT-008
    type: references
---
# TC-094: Compare generated Boolean truth and source activation

## Description

Execute every step of [IT-008](../integration/IT-008-native-boolean-backend-parity.md)
on `(a implies b) implies (c implies (not a or not b and c))` and a second constant
clause, for all eight Boolean assignments.

## Test Procedure

Compile the actual emitted artifacts once, then execute each assignment with
separate measured coverage. Compare the existing coverage consumer's three
consequent counts with native source events and independently specified control
equations, including the implication nested in an antecedent.

## Expected Results

All eight truth results and per-source activation counts agree. Complete clause
identity survives both IR pins. Missing tools, unavailable coverage and failed
native validation fail the test rather than count as a passing or skipped case.

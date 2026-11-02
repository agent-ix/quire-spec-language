---
id: TC-1292
title: "Select reader-backed Union cases by exact member identity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-1292: Select reader-backed Union cases by exact member identity

## Description

Verify that reader-backed protocol evaluation preserves a Union's declaration,
active member and payload, and that case evaluation selects by member identity.
This exercises FR-390-AC-27.

## Test Procedure

Compile two distinct Union declarations that each have a member with the same
spelling and payload shape, plus one Union whose members have distinct payload
arities. A protocol `finish.constraint` evaluates a case over a supplied value
from each declaration and reads every payload position of the selected arm.
Emit and admit the checked package, then run each slot through both the
in-process and reader-backed views. Supply one value for each member, including
a nullary member, and vary the active member while keeping payload values
otherwise equal. Supply a value of the second Union to a slot typed for the
first Union.

## Expected Results

The projected value retains the exact Union declaration and active member
identity, with that member's payload in declared position order. Each case
evaluates only the arm for that exact member and binds only its payload; same
spelling or shape in the other Union never selects the arm. Nullary members
carry no payload. A value of the second declaration is refused as the wrong
value kind before evaluation. The reader-backed and in-process runs return the
same value, outcome and ordered charge log.

---
id: TC-183
title: "The witness envelope refuses reconstruction when any one O-25 member is missing"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: verifies
---
# TC-183: The witness envelope refuses reconstruction when any one O-25 member is missing

## Description

Verify that reconstructing the witness envelope from a packet missing any
one required ADR-013 O-25 member refuses, rather than substituting a
default or omitted value. A wrong implementation this test would catch: an
envelope builder that treats `trace_position` as `Option<TracePosition>`
with a silent `None` default when the family declares one, instead of
requiring it, so a family that should always carry a trace position can
silently lose it without a visible refusal. Scope: FR-070-AC-4.

## Test Procedure

1. Take one fully-populated O-25 packet as the baseline (from TC-182).
2. Remove exactly the `backend` member and attempt reconstruction.
3. Remove exactly the trace position, using a family known to always retain
   one (temporal or protocol), and attempt reconstruction.
4. Remove exactly one `RawSourceRef` digest from the package reference and
   attempt reconstruction.
5. Remove exactly the obligation identity and attempt reconstruction.

## Expected Results

- Each of steps 2-5 refuses reconstruction with a structured, typed cause
  naming the missing member.
- No step produces an envelope with a defaulted, empty, or otherwise
  synthesized value standing in for the missing member.

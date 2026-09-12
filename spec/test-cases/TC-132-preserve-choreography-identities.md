---
id: TC-132
title: "Preserve workflow, role and channel occurrence identities"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-048, type: verifies }
---
# TC-132: Preserve workflow, role and channel occurrence identities

## Description

Verify source-owned role, relationship, endpoint, channel and message nodes plus
the runtime occurrence-key requirements in the emitted protocol artifact.

## Test Procedure

Compile the reusable OrderFlow template against D's Producer interface 1.2.0 at
revision `6259d3a5b99088740df9bcc8e8d60f3720aaa603`, selecting one external
payment component plus shipment, channel and relationship exports. Read the
artifact through the public parser-free reader and inspect each static role slot,
original node handle, source locus, endpoint, payload and the runtime
occurrence-key dimensions for workflow identity and enclosing repeat ordinal.
Do not place concrete O1/O2 instances in the artifact.

Exercise delivery `[0,0]`, `[0,3]` and `[1,1]`, then independently refuse a
negative lower bound, lower greater than upper, and an upper bound outside the
admitted integer representation. Swap one static relationship, component,
endpoint, sender, receiver, role slot or authored node handle at a time. At the
downstream fixture boundary bind O1 and O2 to the same permitted payment
component and verify that their supplied workflow/node occurrence keys differ.

## Expected Results

The positive artifact retains one reusable static subject and the complete key
schema needed to distinguish O1/O2 despite a shared provider or equal local
values. The three valid delivery boundaries retain their exact values; each
invalid bound and static-identity substitution refuses its dependent record.
Concrete workflow instances, deliveries, attempts, effects and relationship
membership remain absent from static compilation and are supplied only at the
owned downstream boundary.

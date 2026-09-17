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

Compile the reusable OrderFlow template against an admitted domain package
([FR-056](../functional/FR-056-admit-domain-package-model-declarations.md)),
selecting one external payment component plus shipment, channel and relationship
declarations. Read the
artifact through the public parser-free reader and use the admitted-package
occurrence-schema projection to inspect each static role slot, original node
handle, source locus, endpoint, payload, workflow-instance binding and
outer-to-inner repeat ordinal domain. Do not place concrete O1/O2 instances or
ordinal values in the artifact.

Exercise delivery `[0,0]`, `[0,3]` and `[1,1]`, then independently refuse a
negative lower bound, lower greater than upper, and an upper bound outside the
admitted integer representation. Swap one static relationship, component,
endpoint, sender, receiver, role slot or authored node handle at a time. At the
downstream fixture boundary bind O1 and O2 to the same permitted payment
component and verify that their supplied workflow/node occurrence keys differ.
For a node nested under repeats, require exactly one caller-supplied ordinal in
each returned domain; refuse a missing, surplus or out-of-range ordinal.

## Expected Results

The positive artifact retains one reusable static subject and the complete key
schema needed to distinguish O1/O2 despite a shared provider or equal local
values. The three valid delivery boundaries retain their exact values; each
invalid bound and static-identity substitution refuses its dependent record.
Concrete workflow instances, deliveries, attempts, effects and relationship
membership remain absent from static compilation and are supplied only at the
owned downstream boundary.

---
id: US-006
title: "Extend a semantic family without silently breaking a seam"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-063
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
---
# US-006: Extend a semantic family without silently breaking a seam

## Story

**As a** QSL contributor adding a new semantic variant to a family
**I want** the build to fail immediately, at every place that must handle my
new variant, if I forget to add a required arm
**So that** I cannot ship a family extension that silently falls through an
unhandled case, and I cannot reintroduce a string comparison where a closed
type or a marked edge belongs.

## Context

Before this ticket, whether every dispatch site over a family's enums has an
arm for a given variant depends on the reviewer noticing a missing arm by
reading the diff; nothing forces a build failure. Likewise, whether a string
comparison selects behaviour outside a designated edge depends on the same
kind of manual reading. Both gaps let a small, easy-to-miss omission or
regression reach production code.

## Acceptance Examples (Illustrative)

### US-006-EX-1: A forgotten arm fails the build, not just review

- **Given** a family's checked-node enum gains a new variant.
- **When** a build runs under the seam-probe feature.
- **Then** the build fails to compile at every seam that must handle the new
  variant, and the failure locations are checked against a checked-in list
  so the build tool, not a reviewer's attention, is the source of truth.

### US-006-EX-2: An unmarked string comparison is reported

- **Given** a contributor adds a string comparison that selects behaviour,
  outside any function marked as a string edge.
- **When** the string-edge scan runs.
- **Then** the scan reports the comparison's file and line, and the lint
  gate fails.

## Priority and Risk (Informative)

Priority: High. A closed-set extension mechanism with no build-time
enforcement degrades over time into exactly the pattern it was meant to
replace: a central routine that silently tolerates an unhandled case, or a
string comparison that quietly becomes a second dispatch mechanism.

## Traceability (Informative)

- [FR-063](../functional/FR-063-exhaustive-family-extension-seam-probe.md)
- [FR-064](../functional/FR-064-restrict-string-dispatch-to-marked-edges.md)

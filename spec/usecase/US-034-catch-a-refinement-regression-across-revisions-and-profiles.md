---
id: US-034
title: "Catch a refinement regression between two revisions of a specification or two layered profiles"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-340
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-341
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-342
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-343
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-344
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-034: Catch a refinement regression between two revisions of a specification or two layered profiles

## Story

**As a** maintainer of a native Quire specification, or of the profiles a
specification is compiled under
**I want** a gate that runs the same cases against the prior and the
superseding revision of my specification, or against a parent profile and
its child, and compares how each side classifies every case
**So that** a superseding revision that refuses a case its prior revision
admitted, or a child profile that admits a case its parent refused, fails
the gate naming the case, while a case the gate could not decide is reported
as unresolved and never counted as holding.

## Context

QSpec capability rows V1-TOOL-011 ("a superseding specification edition
admits every result its prior edition admitted") and V1-TOOL-012 ("every
parent profile's refusal [is] its child profile's refusal under profile
layering") are corpus-differential test gates in QSL. ADR-017 §2 decides
them: two `xtask refinement` subcommands that run a corpus through the
spine and compare structured outcomes. They are behavioural checks over
what each side admits and refuses. They record no requirement record,
request no backend and settle no claim: a passing gate is evidence over its
corpus only.

## Acceptance Examples (Illustrative)

### US-034-EX-1: A tightened clause is a regression

- **Given** a pair whose superseding revision tightens a ConfigVersion state
  clause, and a case whose snapshot the prior revision admits.
- **When** the maintainer runs the spec-versioning gate.
- **Then** the gate exits 10 with a violation and names exactly that case by
  the two revisions' source references and the selection, both classes and
  the superseding run's codes; every other case holds.

### US-034-EX-2: A dropped clause is not a regression

- **Given** a pair whose superseding revision removes a clause the prior
  revision admitted a case of.
- **When** the gate runs.
- **Then** the case holds: dropping a constraint admits more.

### US-034-EX-3: A case the build cannot decide is unresolved

- **Given** a pair whose superseding revision uses a construct QSL has not
  yet implemented.
- **When** the gate runs.
- **Then** the case is unresolved (unsupported), the gate exits 21, and any
  regression elsewhere in the corpus is still listed.

### US-034-EX-4: A parent refusal must survive in the child

- **Given** a unit its parent profile refuses with a typed refusal and its
  child profile admits.
- **When** the profile-layering gate runs.
- **Then** the gate exits 10 naming the case, the parent and child edge and
  the parent's codes.

## Priority and Risk (Informative)

Priority: High. V1-TOOL-011 and V1-TOOL-012 are Required V1 capability
rows. Without the gates, a revision or profile change can narrow what a
specification admits with no signal.

## Traceability (Informative)

- [FR-340](../functional/FR-340-run-the-spec-versioning-refinement-gate-over-a-corpus.md)
- [FR-341](../functional/FR-341-classify-a-spine-compile-result-for-a-refinement-gate.md)
- [FR-342](../functional/FR-342-classify-a-clause-run-disposition-for-a-refinement-gate.md)
- [FR-343](../functional/FR-343-compare-prior-and-superseding-classes-per-case.md)
- [FR-344](../functional/FR-344-report-a-refinement-gate-verdict-and-exit.md)
- [FR-345](../functional/FR-345-run-the-profile-layering-refinement-gate.md)

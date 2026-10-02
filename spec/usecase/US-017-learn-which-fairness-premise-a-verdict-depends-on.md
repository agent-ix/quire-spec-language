---
id: US-017
title: "Learn which fairness premise a verdict depends on or cannot use"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-132
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-133
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-017: Learn which fairness premise a verdict depends on or cannot use

## Story

**As a** verification operator reading a liveness result
**I want** a refutation to tell me when a strong-fairness premise I did not
state would exclude its lasso, and a claim over a recorded trace to tell me
when its fairness premise cannot be checked there
**So that** I can tell a real defect from a missing premise, and I never
read a fairness claim over a trace as checked when it was not.

## Context

ADR-019 SV-6 adds a diagnostic hint to every refuted liveness claim whose
loop a strong constraint would exclude. ADR-019 SV-2 settles a clause with a
fairness constraint over a supplied trace `unsupported`, because a recorded
trace carries no enabledness.

## Acceptance Examples (Illustrative)

### US-017-EX-1: A refutation names the premise that would exclude it

- **Given** ADR-019 §6's mutex claim under weak `each` fairness of
  `acquire`, refuted.
- **When** the operator reads the terminal record.
- **Then** it carries a warning that strong `each` fairness of `acquire`
  would exclude the loop, naming process 1's acquire and loop position 0,
  and the verdict is still `refuted`.

### US-017-EX-2: Fairness over a recorded trace is not checked

- **Given** a recorded trace and a liveness clause with a weak fairness
  constraint.
- **When** the operator evaluates the clause over the trace.
- **Then** it settles `unsupported` with the missing-fairness-premise cause
  naming that constraint.

## Priority and Risk (Informative)

Priority: Medium. Both make a liveness result say what it rests on.

## Traceability (Informative)

- [FR-132](../functional/FR-132-settle-fairness-over-a-supplied-trace-as-a-missing-premise.md)
- [FR-133](../functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md)

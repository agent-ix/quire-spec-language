---
id: US-029
title: "Drive the Quire lifecycle through one CLI or the library and get one outcome and exit code"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-284
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-297
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-298
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-299
    type: exercises
---
# US-029: Drive the Quire lifecycle through one CLI or the library and get one outcome and exit code

## Story

**As a** specification author, a CI job, or a Rust program that embeds QSL
**I want** to parse, check, package, execute, analyze, monitor, prove, replay
and inspect a specification through one `quire` command or through typed
library calls, with limits I set and a way to cancel
**So that** every run ends in one typed outcome, one JSON document and one
exit code that mean the same thing whichever surface I used, and a
qualification audit can cut the check and prove path out as its own small
tool.

## Context

ADR-029 decides the lifecycle stages and their owners (LC-1), the one
request and outcome shape (LC-2), cancellation (LC-3) and caller limits
(LC-4). The user binary is the driver's `quire` (CB-1), a thin frontend over
library operations. The qualified core is the check path, the prove path and
the certificate checkers through which `analyze` enters it (CB-2), and it
stays separable by crate (CB-3). One total function maps an outcome's
category to an exit code (CB-4).

## Acceptance Examples (Illustrative)

### US-029-EX-1: The library and the CLI agree

- **Given** a `1-draft` source that declares a function `seven` returning 7.
- **When** the author runs `quire run` on it and a Rust program calls
  `check`, then `execute`, on the same source.
- **Then** both report the value 7, the CLI exits 0, and the CLI's JSON
  document equals the serialized library outcome.

### US-029-EX-2: Cancelling a long check

- **Given** a large source and a running `check`.
- **When** the caller cancels it.
- **Then** `check` stops at its next work charge, returns a cancelled
  failure with no package, and the CLI exits 22.

### US-029-EX-3: An analyze proof counts only after its certificate is checked

- **Given** a finite model and an invariant that holds.
- **When** the author runs `quire analyze`.
- **Then** the item settles `proved` only after the in-core certificate
  checker accepts the engine's certificate, and a certificate with one
  member altered settles `inconclusive`.

### US-029-EX-4: Monitoring a recorded trace

- **Given** a recorded finite trace and an `always` clause the trace breaks
  at position 2.
- **When** the author runs `quire monitor`.
- **Then** the clause reports a violation at position 2 with its witness,
  and the CLI exits 10.

### US-029-EX-5: Several items, one exit code

- **Given** a prove request with three items that settle `proved`,
  `incomplete` and `refuted`.
- **When** the run ends.
- **Then** the JSON document lists each item with its own result, and the
  process exits 22, the most severe code present.

## Priority and Risk (Informative)

Priority: High. Every later lifecycle feature reaches users through this
surface, and a qualification audit needs the check and prove path to stand
alone.

## Traceability (Informative)

FR-275 to FR-287, FR-297 to FR-299.

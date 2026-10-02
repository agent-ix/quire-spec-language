---
id: US-030
title: "Add providers, plugins, a cache and faster execution backends without changing results"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-289
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-291
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-292
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-293
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-294
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-296
    type: exercises
---
# US-030: Add providers, plugins, a cache and faster execution backends without changing results

## Story

**As a** user who installs a proof backend, a third party who ships one as a
plugin, or an author who wants repeated proof runs and function calls to be
fast
**I want** to add a provider, run a plugin, reuse cached proof results and
pick a compiled execution backend
**So that** I get the same verdicts I would get without them, a plugin's
unreplayable proof is labelled as trusted, and a cached result is never
stale.

## Context

ADR-029 decides that the registry is built from provider manifests and that
negotiation changes a result only through an item's candidate set (PV-1,
PV-2). First-party providers are compile-time Rust traits, and third-party
providers are executables that speak one wire (PL-1 to PL-7). The cache is
keyed by content identity (CA-1 to CA-5). Interpreter, AOT and JIT sit behind
one execution-backend seam and return the interpreter's outcomes (EB-1 to
EB-8).

## Acceptance Examples (Illustrative)

### US-030-EX-1: Installing a provider leaves checking alone

- **Given** one source compiled with no provider installed and with three.
- **When** the author compares the two packages.
- **Then** the bytes are equal and the check outcomes are the same.

### US-030-EX-2: A plugin proof is labelled trusted

- **Given** an installed plugin that advertises a claim kind and reports
  `proved` for an item.
- **When** the author renders the outcome.
- **Then** the item reads `proved`, names the plugin's backend identity, and
  carries the label `trusted`.

### US-030-EX-3: A crashing plugin fails only its own items

- **Given** a prove run with two items, one routed to a plugin that crashes
  and one routed to a compile-time provider.
- **When** the run ends.
- **Then** the plugin's item settles `failed` with a tool-failure cause and
  the other item keeps its own result.

### US-030-EX-4: A second identical proof run reads the cache

- **Given** a completed `prove` run whose records were stored.
- **When** the author runs the same request again.
- **Then** the records are read from the cache and equal the first run's,
  and changing one claim bound makes the next run compute afresh.

### US-030-EX-5: The JIT returns what the interpreter returns

- **Given** a checked function and arguments.
- **When** the author runs it with `--engine interpreter` and with
  `--engine jit`.
- **Then** both return the same value, or the same undefined, refused or
  incomplete outcome with the same cause.

## Priority and Risk (Informative)

Priority: High. Third-party backends, caching and compiled execution are
only useful if they never change a verdict.

## Traceability (Informative)

FR-288 to FR-296.

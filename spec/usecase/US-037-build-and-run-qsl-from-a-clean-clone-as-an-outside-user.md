---
id: US-037
title: "Build and run QSL from a clean clone or a release tag as an outside user"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/NFR-017
    type: exercises
  - target: ix://agent-ix/quire-spec-language/NFR-018
    type: exercises
---
# US-037: Build and run QSL from a clean clone or a release tag as an outside user

## Story

**As a** developer outside the project who has only a Rust toolchain
**I want** to clone QSL, or check out a release tag, build it with the locked
dependencies and run the examples the README shows
**So that** I can try the compiler without sibling checkouts, private
repositories or local patches, and get the same build the maintainers got.

## Context

The README's Usage section says a clone builds with only Rust installed.
NFR-017 checks that at a commit, from a clean clone with an empty Cargo home.
NFR-018 checks it at each release tag, whose locked git revisions can vanish
if a dependency repository rewrites its history.

## Acceptance Examples (Illustrative)

### US-037-EX-1: A clean clone builds and runs the walkthrough

- **Given** a machine with only Rust at the `rust-version` and no Cargo
  cache.
- **When** the developer clones QSL, runs `cargo build --locked --workspace`
  and the README's ConfigVersion commands.
- **Then** the build succeeds and the walkthrough cases exit 0, 10, 20, 22
  and 10 as the README's table states.

### US-037-EX-2: A release tag still builds

- **Given** the newest release tag.
- **When** the developer checks it out in a clean clone and builds it.
- **Then** the locked build succeeds.

## Priority and Risk (Informative)

Priority: High. A compiler nobody outside the project can build cannot be
evaluated.

## Traceability (Informative)

NFR-017, NFR-018.

---
id: FR-058
title: "Detect current-head cross-repository incompatibility before it reaches the exact-pin lane"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/IT-013
    type: references
---
# FR-058: Detect current-head cross-repository incompatibility before it reaches the exact-pin lane

## Description

QSL's own build (root `Cargo.toml`, root `Cargo.lock`) resolves
`quire-contract-ir`, `quire-contract-codegen` and every other cross-repository
dependency at one pinned git revision each (the exact-pin lane). That lane
never moves until a maintainer bumps a `rev`, so a change on another
repository's `main` branch is invisible to QSL until the bump happens, at
which point an incompatibility surfaces as a broken bump PR with no separate
signal of when the incompatibility was introduced upstream.

QSL SHALL provide a second, separate build lane -- the current-head lane --
that resolves quire-contract-ir, quire-contract-runtime and
quire-contract-codegen at each repository's current head instead of a pinned
revision, so an incompatibility is visible before it is pinned.

## Inputs

- The current head commit of each of quire-contract-ir, quire-contract-runtime
  and quire-contract-codegen (each repository's default branch at run time).
- QSL's own working tree, unpinned for these three dependencies.
- The intentionally incompatible fixture manifest (see Behavior).

## Outputs

- A resolved-revision log: for each of the four repositories (QSL itself,
  quire-contract-ir, quire-contract-runtime, quire-contract-codegen), the exact
  commit the run resolved, written once per run.
- Pass or fail for the run: pass when the lane's manifest resolves, the lane's
  workspace builds under `cargo build`/`cargo test`, and the lane's
  representative contract test passes; fail otherwise, with a diagnostic that
  names the failing repository and dependency edge, not only a generic build
  error.

## Behavior

### Separation from the exact-pin lane

The current-head lane SHALL live in its own manifest(s), outside the root
`[workspace]` `members` list, so that running it never changes which
revisions QSL's own root `Cargo.toml`/`Cargo.lock` resolve, and a failure in
the current-head lane SHALL NOT fail QSL's own `cargo build`/`cargo test`/`cargo
clippy` over the root workspace.

### What the lane resolves at head, and what it does not

The current-head lane SHALL resolve quire-contract-ir, quire-contract-runtime
and quire-contract-codegen at each repository's current default-branch head
rather than a pinned `rev`.

QSpec artifacts (the vendored trees `revendor`/`revendor-check` manage under
`QSPEC_CLONE`) are consumed unchanged, at whatever revision QSL's root
manifest already vendors. Sourcing QSpec artifacts at head is an open
question this requirement does not settle (see Status).

### Failure modes the lane SHALL detect

A current-head lane run SHALL fail, rather than silently pass or silently
fall back, when any of the following holds:

- **Schema/API incompatibility**: the workspace at head does not compile, or
  the lane's representative contract test (IT-013) fails, because a type,
  function signature or wire schema changed incompatibly on another
  repository's head.
- **Missing version support**: a dependency's `Cargo.toml` at head requires a
  Rust edition, feature or transitive dependency version the lane's toolchain
  does not provide.
- **Stale generated artifacts**: a generated artifact the lane's
  representative contract test compares against (for example a codegen
  bound-coverage schema constant) no longer matches what the head build
  produces.
- **Silent fallback to released deps**: none of the three dependencies is
  ever resolved from a published registry release; each is a git dependency
  with no version fallback, so a resolution failure SHALL surface as a lane
  failure, never as a quiet substitution of a different source.

### Intentionally incompatible fixture

The lane SHALL include a fixture manifest that deliberately points one
dependency at a revision or reference this requirement documents as
incompatible with the others, and running the lane against that fixture
manifest SHALL fail with a stable, documented diagnostic (a fixed marker
string that identifies which dependency and which kind of incompatibility),
not an arbitrary, unstable compiler backtrace.

### Local and CI invocation

The lane SHALL be runnable locally by one documented command (a Makefile
target). Where and how the lane runs in hosted CI is an open question this
requirement does not settle (see Status); AGENTS.md's existing
`workflow_dispatch`-only rule applies to any hosted workflow this or a later
ticket adds.

### Ownership and update procedure

The lane's own README (or a section of this requirement's implementation)
SHALL document: who owns the lane, what a maintainer does when it starts
failing (bisect the three repositories' heads against the last-known-good
resolved-revision log; open an issue against the repository whose head
introduced the break; the exact-pin lane's pinned revisions are the source of
truth for what QSL ships and are never advanced solely because the
current-head lane failed), and how to re-run it locally.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-058-AC-1 | The current-head lane's manifest(s) are not members of the root `[workspace]`; running the lane never changes the root `Cargo.lock`'s resolved revisions. | Test (TC-159) |
| FR-058-AC-2 | A lane run against real current heads of quire-contract-ir, quire-contract-runtime and quire-contract-codegen resolves and records one commit per repository (QSL, IR, RT, CG) in a log. | Test (TC-159) |
| FR-058-AC-3 | A lane run against the intentionally incompatible fixture manifest fails, and its diagnostic names the incompatible dependency and a stable marker string, not only a raw compiler error. | Test (TC-159) |
| FR-058-AC-4 | The lane is invocable by one documented local command; its documentation names an owner and an update procedure. | Test (TC-159) |

## Dependencies

- ADR-011 §7.1 T-12 (`ix://agent-ix/quire-spec-language/ADR-011`) names the
  current-head lane and the backend direction/API-surface/duplicate-revision
  checks (FR-059, FR-060, FR-061) as its enforcement mechanisms.
- [IT-013](../integration/IT-013-current-head-integration-lane.md) exercises
  this requirement end to end.
- Repository Makefile `revendor`/`revendor-check` targets and the `QSPEC_CLONE`
  convention, unchanged by this requirement.

## Status

Specified and implemented under
[#215](https://github.com/agent-ix/quire-spec-language/issues/215). Two design
questions this requirement does not settle: where hosted CI runs the lane, and
how QSpec artifacts would be sourced at head rather than at their currently
vendored revision. Both are reported to the issue rather than decided here.

---
id: TC-158
title: "Report a quire-ecosystem crate resolved to more than one source"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-061
    type: verifies
---
# TC-158: Report a quire-ecosystem crate resolved to more than one source

## Description

Verify that `arch-lint duplicate-revisions` recovers each `[[package]]`
stanza's name, version and source from `Cargo.lock` text, reports no finding
for a lockfile with one source per ecosystem repository (and ignores a
non-ecosystem crate resolved to two sources), reports exactly one finding
naming both sources for an ecosystem repository resolved to two distinct
sources whether or not its two packages share one crate name, treats a
path/workspace-member package (no `source` field) as its own distinct source,
and reports no duplicate against QSL's real root `Cargo.lock`. Scope: FR-061-AC-1 through FR-061-AC-4.

## Test Procedure

1. Parse a small fixture `Cargo.lock` text with several `[[package]]` stanzas,
   including quire-ecosystem and non-ecosystem names, some with a `source`
   field and some without.
2. Run the check on a fixture lockfile with exactly one source per ecosystem
   repository, and a non-ecosystem crate name (`serde`) resolved to two
   distinct sources.
3. Run the check on a fixture lockfile with one ecosystem repository (for
   example `quire-contract-ir`) resolved to two distinct `source` strings.
4. Run the check on a fixture lockfile with two `[[package]]` stanzas of the
   same ecosystem repository, one with a `source` field and one with none (a
   path dependency); separately, a fixture lockfile with two *different* crate
   names (`quire-contract-ir`, `quire-contract-model`) that both classify to
   the IR repository, at two distinct sources (#249 review R2).
5. Run `arch-lint duplicate-revisions --lockfile Cargo.lock` against QSL's
   real root `Cargo.lock`.

## Expected Results

- Step 1: name, version and source are recovered for every stanza, including
  stanzas with no `source` field.
- Step 2: no duplicate is reported; `serde`'s two sources are not reported
  because it is not a quire-ecosystem name.
- Step 3: exactly one finding is reported, naming the repository and both
  distinct sources.
- Step 4: the path-dependency stanza is treated as its own distinct source, so
  the pair is reported as two distinct sources (a duplicate finding), not
  silently merged with the git-sourced stanza. The two-different-crate-names
  fixture also reports exactly one finding, for the shared repository, naming
  both sources -- a different crate name is not, by itself, a different
  component.
- Step 5: against the real root `Cargo.lock`, the check reports no duplicate
  and `arch-lint duplicate-revisions --lockfile Cargo.lock` exits `0`; `make
  ci` runs it.

## Metadata

- Priority: P1
- Target Integration: `tools/arch-lint/duplicate_revisions.rs`
- Automation: Automated Rust unit tests; step 5 runs in `make ci`

## Dependencies

**Upstream:** [FR-061](../functional/FR-061-check-duplicate-ecosystem-revisions.md).
**Downstream:** none.

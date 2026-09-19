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
for a lockfile with one source per ecosystem crate name (and ignores a
non-ecosystem crate resolved to two sources), reports exactly one finding
naming both sources for an ecosystem crate name resolved to two distinct
sources, treats a path/workspace-member package (no `source` field) as its
own distinct source, and reports no duplicate against QSL's real `Cargo.lock`.
Scope: FR-061-AC-1 through FR-061-AC-4.

## Test Procedure

1. Parse a small fixture `Cargo.lock` text with several `[[package]]` stanzas,
   including quire-ecosystem and non-ecosystem names, some with a `source`
   field and some without.
2. Run the check on a fixture lockfile with exactly one source per ecosystem
   crate name, and a non-ecosystem crate name (`serde`) resolved to two
   distinct sources.
3. Run the check on a fixture lockfile with one ecosystem crate name (for
   example `quire-contract-model`) resolved to two distinct `source` strings.
4. Run the check on a fixture lockfile with two `[[package]]` stanzas of the
   same ecosystem crate name, one with a `source` field and one with none (a
   path dependency).
5. Run `arch-lint duplicate-revisions` against QSL's real `Cargo.lock`.

## Expected Results

- Step 1: name, version and source are recovered for every stanza, including
  stanzas with no `source` field.
- Step 2: no duplicate is reported; `serde`'s two sources are not reported
  because it is not a quire-ecosystem name.
- Step 3: exactly one finding is reported, naming the crate and both distinct
  sources.
- Step 4: the path-dependency stanza is treated as its own distinct source, so
  the pair is reported as two distinct sources (a duplicate finding), not
  silently merged with the git-sourced stanza.
- Step 5: no duplicate is reported. `quire-contract-model` (consumed by QSL as
  `quire-contract-ir`) and `quire-contract-ir` (consumed by QSL as
  `quire-contract-ir-historical`) are two distinct crate names, each with one
  source; this is not a duplicate of one name and the check does not report
  it as one.

## Metadata

- Priority: P1
- Target Integration: `tools/arch-lint/duplicate_revisions.rs`
- Automation: Automated Rust unit tests plus one manual real-data run (step 5)

## Dependencies

**Upstream:** [FR-061](../functional/FR-061-check-duplicate-ecosystem-revisions.md).
**Downstream:** none.

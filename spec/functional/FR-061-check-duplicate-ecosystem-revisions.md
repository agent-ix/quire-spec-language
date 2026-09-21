---
id: FR-061
title: "Check for a duplicate resolved revision of a quire-ecosystem crate"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
---
# FR-061: Check for a duplicate resolved revision of a quire-ecosystem crate

## Description

ADR-011 §7.1 requires that QSL's own `Cargo.lock` resolve exactly one revision
per quire-ecosystem *repository*. Two `[[package]]` stanzas with different
`source` strings that both come from the same ecosystem git repository mean
Cargo's resolver selected two different git revisions (or a mix of a git
revision and a registry release) of what is meant to be one dependency --
silently, since Cargo permits this when a dependency graph has, for example, a
normal-dependency pin and a separate dev-dependency pin (or a `-historical`
alias) that drifted apart. This holds whether or not the two stanzas share one
crate name: a repository that publishes more than one crate (for example
quire-contract-ir's own workspace member, published as `quire-contract-model`)
is still one component, and two of its packages at two different revisions is
the same drift as one crate name at two revisions (#249 review R2).

QSL SHALL provide a check over `Cargo.lock` that reports every
quire-ecosystem repository resolved to more than one distinct `source`.

## Inputs

- QSL's `Cargo.lock` text.

## Outputs

- For each ecosystem repository resolved to more than one distinct source: the
  repository and the list of distinct sources.
- A pass/fail result: pass exactly when no ecosystem repository has more than
  one distinct source.

## Behavior

The check SHALL read `Cargo.lock` as plain text, parsing only the `name`,
`version` and `source` fields of each `[[package]]` stanza. It SHALL NOT
depend on a general TOML parser; `Cargo.lock`'s `[[package]]` stanza shape is
a stable, Cargo-owned format.

The check SHALL classify a package's ecosystem *repository* primarily by its
`source` string (a package sourced from
`git+https://github.com/agent-ix/quire-contract-ir...` classifies as the IR
repository regardless of which crate name it publishes), falling back to the
package name only when no `source` is present (a path dependency or workspace
member). This classification SHALL be one shared function
(`graph::classify`), used by both this check and FR-059's crate-level edge
extraction (`metadata::edges_for_manifest`), so QSL holds one definition of an
ecosystem repository component, not two independently maintained ones (#249
review R2).

The check SHALL group ecosystem packages by repository and report a
repository whose distinct `source` values (after deduplication) number more
than one.

The check SHALL treat a package with no `source` field (a path dependency or a
workspace member) as its own distinct source value, and SHALL NOT merge it
with a git or registry source of the same repository.

This check flags two *different* ecosystem crate names resolving to the same
repository at different revisions exactly as it flags one crate name at two
revisions: `quire-contract-ir` (consumed at one revision) and
`quire-contract-model` (IR's own workspace member, consumed at a different
revision, for example through QSL's `quire-contract-ir-historical` dev-alias)
are the IR repository twice, and are reported (#249 review R2; see Status for
what this means for QSL's own real `Cargo.lock`).

### Scope: the current-head lane's own lock too

FR-058's current-head lane (`integration/current-head/`) has its own,
separate `Cargo.lock`, independent of QSL's root one. That lock is in scope
for this requirement too (#249 review R3): the Makefile SHALL provide a
target that runs this check over the lane's own lockfile, and the lane's own
dependency graph SHALL be arranged (via its `[patch]` table) so that lock
converges on one revision per ecosystem repository. This requirement does not
require QSL's own root lock to converge; see Status for that lock's real,
current outcome under the R2 repository rule.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-061-AC-1 | `Cargo.lock` parsing recovers each `[[package]]` stanza's name, version and source. | Test (TC-158) |
| FR-061-AC-2 | A lockfile with one source per ecosystem repository reports no duplicate; a non-ecosystem crate name (for example `serde`) resolved to two sources is not reported. | Test (TC-158) |
| FR-061-AC-3 | A lockfile with two packages classifying to the same ecosystem repository -- whether they share one crate name or not -- resolved to two distinct sources reports exactly one duplicate finding, naming both sources. | Test (TC-158) |
| FR-061-AC-4 | Run against QSL's real root `Cargo.lock`, the check reports the IR repository as a duplicate: `quire-contract-ir` (consumed directly) and `quire-contract-model` (IR's own workspace member, consumed through the `quire-contract-ir-historical` dev-alias) are the same repository at two different revisions. Run against the current-head lane's own `Cargo.lock` (`integration/current-head/Cargo.lock`), the check reports no duplicate. | Test (TC-158) |

## Dependencies

- ADR-011 §7.1 (`ix://agent-ix/quire-spec-language/ADR-011`).
- [FR-058](FR-058-detect-current-head-cross-repository-incompatibility.md)'s
  current-head lane owns the second lockfile this requirement's R3 scope
  extension checks.

## Status

Specified and implemented under
[#215](https://github.com/agent-ix/quire-spec-language/issues/215) as the
`arch-lint duplicate-revisions` subcommand
(`tools/arch-lint/duplicate_revisions.rs`), keyed on repository (via the
shared `graph::classify`) rather than crate name since #249 review R2.

Run against QSL's real root `Cargo.lock`, the check now reports the IR
repository as a duplicate (`quire-contract-ir` vs. `quire-contract-model`,
FR-061-AC-4): `arch-lint duplicate-revisions --lockfile Cargo.lock` exits 1.
This is a real, pre-existing double-pin the R2 repository rule newly detects;
this requirement does not remediate it (`make arch-lint`'s own comment records
this; remaining work tracked as #211/#213-adjacent architecture debt, not
resolved here).

Run against the current-head lane's own `Cargo.lock`
(`make arch-lint-duplicate-revisions-lane`), the check reports no duplicate:
the lane's `[patch]` table (`integration/current-head/Cargo.toml`) redirects
every QSL/IR/RT node in its graph to one head-tracked git or path source per
repository, so `cargo update` converges that lock to exactly one revision
each for quire-contract-codegen, quire-contract-ir, quire-contract-model,
quire-contract-runtime, quire-observation, quire-protocol and
quire-spec-language (#249 review R3).

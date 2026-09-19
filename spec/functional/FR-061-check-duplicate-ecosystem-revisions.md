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
per quire-ecosystem crate. Two `[[package]]` stanzas with the same crate name
and different `source` strings mean Cargo's resolver selected two different
git revisions (or a mix of a git revision and a registry release) of what is
meant to be one dependency -- silently, since Cargo permits this when a
dependency graph has, for example, a normal-dependency pin and a separate
dev-dependency pin (or a `-historical` alias) that drifted apart.

QSL SHALL provide a check over `Cargo.lock` that reports every
quire-ecosystem crate name resolved to more than one distinct `source`.

## Inputs

- QSL's `Cargo.lock` text.

## Outputs

- For each ecosystem crate name resolved to more than one distinct source: the
  name and the list of distinct sources.
- A pass/fail result: pass exactly when no ecosystem crate name has more than
  one distinct source.

## Behavior

The check SHALL read `Cargo.lock` as plain text, parsing only the `name`,
`version` and `source` fields of each `[[package]]` stanza. It SHALL NOT
depend on a general TOML parser; `Cargo.lock`'s `[[package]]` stanza shape is
a stable, Cargo-owned format.

The check SHALL classify a package as a quire-ecosystem crate when its name
starts with `quire-`, or is `quire_contract_model` (the crate name
quire-contract-ir's workspace member publishes under).

The check SHALL group ecosystem packages by name and report a name whose
distinct `source` values (after deduplication) number more than one.

The check SHALL treat a package with no `source` field (a path dependency or a
workspace member) as its own distinct source value, and SHALL NOT merge it
with a git or registry source of the same name.

This check intentionally does not flag two *different* ecosystem crate names
resolving to the same repository at different revisions (for example
`quire-contract-ir` at one revision and `quire-contract-ir-historical`, QSL's
own dev-dependency alias for its historical-compatibility tests, at another):
that is two distinct crate names by design, not a duplicate revision of one
name.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-061-AC-1 | `Cargo.lock` parsing recovers each `[[package]]` stanza's name, version and source. | Test (TC-158) |
| FR-061-AC-2 | A lockfile with one source per ecosystem crate name reports no duplicate; a non-ecosystem crate name (for example `serde`) resolved to two sources is not reported. | Test (TC-158) |
| FR-061-AC-3 | A lockfile with the same ecosystem crate name resolved to two distinct sources reports exactly one duplicate finding, naming both sources. | Test (TC-158) |
| FR-061-AC-4 | Run against QSL's real `Cargo.lock`, the check reports no duplicate: `quire-contract-model` (consumed as `quire-contract-ir`) and `quire-contract-ir` (consumed as `quire-contract-ir-historical`) are two distinct crate names, each with one source, not a duplicate of one name. | Test (TC-158) |

## Dependencies

- ADR-011 §7.1 (`ix://agent-ix/quire-spec-language/ADR-011`).

## Status

Specified and implemented under
[#215](https://github.com/agent-ix/quire-spec-language/issues/215) as the
`arch-lint duplicate-revisions` subcommand
(`tools/arch-lint/duplicate_revisions.rs`).

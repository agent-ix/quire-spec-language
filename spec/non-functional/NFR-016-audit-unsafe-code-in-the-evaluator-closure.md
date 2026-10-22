---
id: NFR-016
org: agent-ix
title: "Audit unsafe code in the evaluator closure against a third-party unsafe inventory"
type: NFR
quality_attribute: compliance
relationships:
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-400
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-403
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
---
# NFR-016: Audit unsafe code in the evaluator closure against a third-party unsafe inventory

## Statement

The `make audit-unsafe` gate in `agent-ix/quire-eval` shall fail when a
first-party crate in the evaluator closure contains unsafe code or does not
declare `unsafe_code` forbidden, or when a third-party crate in the closure
that contains unsafe code does not have exactly one entry in the
repository's unsafe inventory.

## Scope

This is slice S8 (IR-592) of
[AD-001](../assurance/AD-001-shared-evaluation-core-interface.md). It covers
the crates that link into the `no_std` evaluator: what the gate scans, what
counts as unsafe code, the first-party rule, the inventory of third-party
unsafe code and how an entry is accepted, the conditions that fail the gate,
and where the gate runs.

### The evaluator closure

The evaluator closure is the set of packages Cargo resolves as normal
dependencies of `quire-eval`, including `quire-eval` itself, in the
`agent-ix/quire-eval` workspace with that repository's `Cargo.lock`, for the
target `thumbv7em-none-eabi` and with the features `quire-eval`'s own
manifest selects. `quire-eval` has no features
([FR-401](../functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md))
and takes each dependency without `std`, so this is the closure that links
into the `no_std` evaluator
([FR-403](../functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md)).
It contains `quire-checked`
([FR-400](../functional/FR-400-separate-checked-terms-from-the-checker.md)),
the shared leaves `quire-semantic-value`, `quire-exact`, `quire-walk` and
`quire-canonical`, and every third-party crate those reach.

These are outside the closure:

- a proc-macro crate, and a package reached from `quire-eval` only through
  a proc-macro crate, because it runs in the compiler on the build host and
  links into no evaluator artifact;
- build dependencies and dev-dependencies;
- `quire-eval-corpus` and every other workspace member that `quire-eval`
  does not depend on;
- the QSL repository's crates: QSL's host builds unify `std` features into
  the shared leaves (FR-403), so they do not build this closure.

### First-party and third-party crates

A first-party crate is a closure package whose source is a member of the
`agent-ix/quire-eval` workspace or a git repository under
`https://github.com/agent-ix/`. Every other closure package is a third-party
crate; the repository's `deny.toml` admits no source besides crates.io and
those git repositories.

### Unsafe code

A package contains unsafe code when the scan reports at least one unsafe
function, expression, impl, trait or method in its source files, counting
both the files the scan's build compiles and the package's other source
files. The unit is the package. The gate makes no claim about which unsafe
code the evaluator's entry points reach: an unsound unsafe block anywhere in
a linked crate can break a guarantee the evaluator relies on, and counting
every source file keeps the result independent of the build host's target
and `cfg` choices.

### First-party rule

Every first-party crate in the closure SHALL contain no unsafe code and
SHALL declare `unsafe_code` at level `forbid`, either in its manifest's
`[lints.rust]` table (its own or a `workspace = true` table that inherits
it) or as `#![forbid(unsafe_code)]` at its crate root. The inventory admits
no first-party crate: unsafe code in a first-party crate fails the gate,
and its fix belongs in that crate's repository.

The declaration and the scan are both required. Cargo caps lints for a git
dependency, so a shared leaf's `forbid` is enforced only by that leaf's own
gate; the scan is the evidence in this closure that its source has no
unsafe code.

### The inventory

The inventory is the file `unsafe-inventory.toml` at the root of
`agent-ix/quire-eval`. It holds one `[[crate]]` table per third-party crate
in the closure that contains unsafe code, ordered by `name`, with exactly
these keys:

| Key | Value |
| --- | --- |
| `name` | The crate's package name. |
| `reached_through` | The first-party crates in the closure that reach the crate through third-party crates only, as a list ordered by name. |
| `purpose` | One or more sentences naming what the crate's unsafe code does. |
| `acceptance` | One or more sentences stating why the evaluator may link that code. |

An entry names a crate, never a version, a source URL, a checksum or a
digest. The resolved version lives in `Cargo.lock` only, so a version change
of an inventoried crate needs no inventory change, and a change that brings
a new unsafe-bearing crate into the closure, or adds unsafe code to a crate
that had none, fails until the same change adds its entry.

A QSL maintainer accepts an entry by approving the change that adds it; the
change's review is the acceptance record, and the entry carries no reviewer,
date or approval field.

### Gate

`make audit-unsafe` in `agent-ix/quire-eval` SHALL run
`cargo geiger -p quire-eval --target thumbv7em-none-eabi --output-format Json --locked`
and a checker, a `std` binary crate of the workspace outside the closure
that itself forbids unsafe code. The checker reads the scan report, the
closure from `cargo metadata --locked --filter-platform thumbv7em-none-eabi`
and the inventory, and prints one line per finding naming its kind and
crate. The target SHALL fail when any of these holds:

1. **`first-party-unsafe`**: a first-party crate in the closure contains
   unsafe code.
2. **`first-party-not-forbidden`**: a first-party crate in the closure
   declares `unsafe_code` at no level or at a level below `forbid`.
3. **`unlisted-unsafe`**: a third-party crate in the closure contains unsafe
   code and has no inventory entry. The line also names the crate's
   `reached_through` set.
4. **`stale-entry`**: an inventory entry names a crate that is not in the
   closure or contains no unsafe code.
5. **`reached-through-mismatch`**: an entry's `reached_through` differs from
   the set the closure gives.
6. **`malformed-entry`**: the inventory does not parse, an entry lacks a key
   or has a key outside the four, `purpose` or `acceptance` is empty, two
   entries share a `name`, or entries are not ordered by `name`.
7. **`unscanned`**: the scan reports a closure package it could not scan, or
   the scan or `cargo metadata` exits non-zero.
8. `cargo-geiger` is not installed. The target prints the install command
   and exits non-zero; no variable, flag or file skips the gate or a
   finding.

The target passes only when the checker reports no finding. `make ci` in
`agent-ix/quire-eval` SHALL list `audit-unsafe` among its prerequisites. That
repository's `make ci` is its CI gate; QSL's `make ci` runs no copy of it.

### Ownership

The gate is QSL-owned and runs in `agent-ix/quire-eval`, the QSL-owned
repository that holds the closure's root (FR-401). It reads the shared
leaves' manifests and sources as closure members and adds no gate to their
repositories. RT and CG decide the unsafe policy for their own targets.

## Rationale

`no_std` does not imply the absence of unsafe code: the closure's
big-integer, allocator-API, lazy-initialization, Unicode-normalization,
SHA-256 and float-formatting dependencies use unsafe code on any target. A
qualified-Rust process needs the list of those crates and a recorded reason
for each, and needs a change that adds one to stop until that reason
exists. Inventorying by crate name keeps the list free of version records,
which live in `Cargo.lock`. Failing on a stale entry keeps the inventory
equal to the closure, so a reader can take it as the complete list.

Running in `agent-ix/quire-eval` places the gate where the closure is
resolved: that repository's lockfile fixes the shared leaves' revisions, and
its `-p quire-eval` build selects their `no_std` features, which QSL's
workspace cannot reproduce.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Checker findings over the closure in `make audit-unsafe` | 0 | 0 | Test |
| Gate fixtures whose verdict differs from the expected finding | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-016-AC-1 | `make audit-unsafe` in `agent-ix/quire-eval` runs `cargo geiger -p quire-eval --target thumbv7em-none-eabi --output-format Json --locked` and the checker, and passes on the repository's main branch; `make ci` lists `audit-unsafe` as a prerequisite; and `unsafe-inventory.toml` holds an entry for each third-party crate the scan reports with unsafe code and for no other crate. | Test (TC-1130) |
| NFR-016-AC-2 | Every first-party crate in the closure declares `unsafe_code = "forbid"` in its manifest lints or `#![forbid(unsafe_code)]` at its crate root; the checker, given a fixture closure in which `quire-walk` declares neither, reports `first-party-not-forbidden quire-walk` and fails. | Test (TC-1131) |
| NFR-016-AC-3 | Given a fixture scan report in which `quire-checked` has one unsafe expression, the checker reports `first-party-unsafe quire-checked` and fails, and an inventory entry for `quire-checked` does not clear the finding. In a scratch copy of the repository, an `unsafe` block added to a `quire-eval` source file makes `make audit-unsafe` fail. | Test (TC-1131) |
| NFR-016-AC-4 | Given a fixture closure in which `quire-exact` reaches a third-party crate whose scan reports unsafe code and the inventory has no entry for it, the checker reports `unlisted-unsafe` naming the crate and `quire-exact` and fails; adding a well-formed entry with `reached_through = ["quire-exact"]` makes the same fixture pass. | Test (TC-1131) |
| NFR-016-AC-5 | Given an inventory entry for a crate that is not in the fixture closure, and separately for a closure crate whose scan reports no unsafe code, the checker reports `stale-entry` naming the crate and fails; given an entry whose `reached_through` lists `quire-canonical` for a crate reached only through `quire-exact`, it reports `reached-through-mismatch` and fails. | Test (TC-1131) |
| NFR-016-AC-6 | Given an entry with a `version` key, an entry with an empty `acceptance`, two entries with the same `name`, and two entries out of `name` order, the checker reports `malformed-entry` for each and fails. | Test (TC-1131) |
| NFR-016-AC-7 | Given a fixture in which unsafe code is reported only in a proc-macro crate and in a crate reached only through it, the checker reports no finding and requires no entry, and an entry for that proc-macro crate reports `stale-entry`; given a scan report that lists a closure package without metrics, the checker reports `unscanned` and fails. | Test (TC-1131) |
| NFR-016-AC-8 | With `cargo-geiger` absent from `PATH`, `make audit-unsafe` exits non-zero and prints the install command; the Makefile, the checker's command line and its environment offer no setting that skips the gate or a finding. | Test (TC-1130) and Inspection |

## Verification

TC-1130 runs the gate in `agent-ix/quire-eval`: the real closure, the real
inventory, the `make ci` wiring and the missing-tool failure. TC-1131 runs
the checker's own tests over fixture scan reports, closures and inventories,
one per finding kind, plus the scratch-copy `unsafe` block. Rust tests carry
`#[trace("TC-...", "NFR-016-AC-...")]` attributes (NFR-005).

## Dependencies

- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md) slice S8
  (IR-592).
- [FR-400](../functional/FR-400-separate-checked-terms-from-the-checker.md)
  and [FR-401](../functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md):
  `quire-checked` and `quire-eval`, their repository and dependency sets.
- [FR-403](../functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md):
  the `no_std` evaluator this closure links into.
- [NFR-015](NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md):
  the `thumbv7em-none-eabi` target.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 layers C and E and FB-05 (shared leaves).

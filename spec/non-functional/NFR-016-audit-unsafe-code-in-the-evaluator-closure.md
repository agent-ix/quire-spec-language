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
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: references
---
# NFR-016: Audit unsafe code in the evaluator closure against a third-party unsafe inventory

## Statement

The `make audit-unsafe` gate in `agent-ix/quire-eval` shall fail when a
first-party crate in the evaluator closure has unsafe code in its shipped
source or does not declare `unsafe_code` forbidden, or when a third-party
crate in the closure that contains unsafe code does not have exactly one
entry in the repository's unsafe inventory.

## Scope

This is the unsafe-code half of slice S8 (IR-592) of
[AD-001](../assurance/AD-001-shared-evaluation-core-interface.md). It covers
the crates that link into the `no_std` evaluator: how the closure is
resolved, what counts as unsafe code in a first-party and in a third-party
crate, the first-party rule, the inventory of third-party unsafe code and
how an entry is accepted, the conditions that fail the gate, and where the
gate runs. S8's dependency half (allowed sources, licences and advisories)
is `agent-ix/quire-eval`'s existing `make deny` gate over its `deny.toml`;
this requirement does not restate it.

### The evaluator closure

The evaluator closure is the set of packages Cargo resolves for `quire-eval`
alone, including `quire-eval` itself, as `cargo tree --locked -p quire-eval
-e normal,no-proc-macro --target thumbv7em-none-eabi` reports it in the
`agent-ix/quire-eval` workspace with that repository's `Cargo.lock`. That
resolution selects features for `quire-eval` alone, so a feature another
workspace member enables on a shared crate adds no package to the closure.
`quire-eval` has no `std` feature
([FR-401](../functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md))
and no feature that adds or removes the evaluator
([FR-403](../functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md)),
and it takes each dependency without `std`, so this is the closure that links
into the `no_std` evaluator. It contains `quire-checked`
([FR-400](../functional/FR-400-separate-checked-terms-from-the-checker.md)),
the shared leaves `quire-semantic-value`, `quire-exact`, `quire-walk` and
`quire-canonical`, and every third-party crate those reach.

These are outside the closure:

- a proc-macro crate, and a package reached from `quire-eval` only through
  a proc-macro crate, because it runs in the compiler on the build host and
  links into no evaluator artifact;
- build dependencies and dev-dependencies;
- `quire-eval-corpus`, the checker and every other workspace member that
  `quire-eval` does not depend on;
- the QSL repository's crates: QSL's host builds unify `std` features into
  the shared leaves (FR-403), so they do not build this closure.

### First-party and third-party crates

A first-party crate is a closure package whose source is a member of the
`agent-ix/quire-eval` workspace or a git repository under
`https://github.com/agent-ix/`. Every other closure package is a third-party
crate; the repository's `deny.toml` admits no source besides crates.io and
those git repositories.

### Unsafe code

The unsafe items are unsafe blocks, unsafe functions and methods, unsafe
impls and unsafe traits.

- **First-party crate.** The checker parses the crate's shipped source: the
  root file of each library, binary and build-script target in its manifest,
  and every module file those roots declare, whatever `cfg` guards the
  declaration. The crate has unsafe code when that source holds at least one
  unsafe item. Files reached only from test, bench or example targets
  (`tests/`, `benches/`, `examples/`) are not shipped source and are not
  counted.
- **Third-party crate.** The crate contains unsafe code when the scan
  reports at least one unsafe item for its package, counting both the files
  the scan's build compiles and the package's other source files.

The unit is the package. The gate makes no claim about which unsafe code the
evaluator's entry points reach: an unsound unsafe block anywhere in a linked
crate can break a guarantee the evaluator relies on. Counting every source
file of a third-party package, and every declared module of a first-party
crate, keeps the result independent of the build host's target and `cfg`
choices.

Code that a build script generates and the crate compiles in (an
`include!` of a file under `OUT_DIR`, which the scan lists among its used but
unscanned files) is outside the gate's claim. That code exists only in the
build directory of a particular build and is produced by the build script,
whose own source the count above covers. The gate neither counts nor fails
on it.

### First-party rule

Every first-party crate in the closure SHALL have no unsafe item in its
shipped source and SHALL declare `unsafe_code` at level `forbid` for its
library and binary targets, in one of these places:

- its manifest's `[lints.rust]` table, as `unsafe_code = "forbid"` or
  `unsafe_code = { level = "forbid", ... }`;
- its manifest's `[lints] workspace = true`, when the root manifest of the
  workspace the crate's source belongs to (its own checkout, for a git
  dependency) declares `unsafe_code` at level `forbid` in
  `[workspace.lints.rust]`;
- `#![forbid(unsafe_code)]` at the root file of each library and binary
  target.

Level `deny` does not satisfy the rule, because an `#[allow(unsafe_code)]`
in the source can lower it; `forbid` cannot be lowered. The inventory admits
no first-party crate: unsafe code in a first-party crate fails the gate, and
its fix belongs in that crate's repository.

The declaration and the scan are both required. Cargo caps lints for a git
dependency, so a shared leaf's `forbid` is enforced only by that leaf's own
gate; the scan is the evidence in this closure that its shipped source has
no unsafe code.

### The inventory

The inventory is the file `unsafe-inventory.toml` at the root of
`agent-ix/quire-eval`. It holds one `[[crate]]` table per third-party crate
in the closure that contains unsafe code, ordered by `name`, with exactly
these keys:

| Key | Value |
| --- | --- |
| `name` | The crate's package name. |
| `purpose` | One or more sentences naming what the crate's unsafe code does. |
| `acceptance` | One or more sentences stating why the evaluator may link that code. |

An entry names a crate, never a version, a source, a checksum, a digest or
the dependency path that reaches it. The resolved versions and the
dependency graph live in `Cargo.lock` only, so a version change of an
inventoried crate, or a change in which first-party crate reaches it, needs
no inventory change. A change that brings a new unsafe-bearing crate into
the closure, or adds unsafe code to a crate that had none, fails until the
same change adds its entry.

A QSL maintainer accepts an entry by approving the change that adds it; the
change's review is the acceptance record, and the entry carries no reviewer,
date or approval field.

### Gate

`make audit-unsafe` in `agent-ix/quire-eval` SHALL run the checker, a `std`
binary crate of the workspace outside the closure that itself forbids unsafe
code. The checker takes four inputs:

1. **The closure**, from the `cargo tree` resolution above.
2. **The scan report**, from `cargo-geiger` run on the package `quire-eval`
   through its own manifest given by absolute path (the repository root is a
   virtual manifest), for the target `thumbv7em-none-eabi`, with `--locked`
   and the JSON report format. The report's `packages`,
   `packages_without_metrics` and `used_but_not_scanned_files` keys are the
   parts the checker reads.
3. **The first-party sources**: for each first-party closure crate, its
   manifest, the root manifest of the workspace its source belongs to, and
   its shipped source files, located through `cargo metadata --locked`.
   `cargo metadata` locates files only; it does not decide closure
   membership.
4. **The inventory.**

The checker may run `cargo tree`, `cargo-geiger` and `cargo metadata`
itself. It prints one line per finding naming its kind and crate, and the
target SHALL fail when any of these holds:

1. **`first-party-unsafe`**: a first-party crate's shipped source holds an
   unsafe item. The line also names the file.
2. **`first-party-not-forbidden`**: a first-party crate declares
   `unsafe_code` at level `forbid` in none of the places the first-party
   rule lists.
3. **`unlisted-unsafe`**: a third-party crate in the closure contains unsafe
   code and has no inventory entry. The line also names the first-party
   crates that reach it through third-party crates only, read from the
   closure.
4. **`stale-entry`**: an inventory entry names a third-party crate that is
   not in the closure or contains no unsafe code.
5. **`malformed-entry`**: the inventory does not parse, an entry lacks a key
   or has a key outside the three (a `version` or `reached_through` key
   among them), `purpose` or `acceptance` is empty, two entries share a
   `name`, entries are not ordered by `name`, or an entry names a
   first-party crate.
6. **`unscanned`**: the scan lists a closure package under
   `packages_without_metrics`, a closure package is absent from the scan's
   `packages`, a first-party source file the shipped source declares cannot
   be read or parsed, or `cargo tree`, `cargo-geiger` or `cargo metadata`
   exits non-zero.
7. `cargo-geiger` is not installed. The target prints the install command
   and exits non-zero; no variable, flag or file skips the gate, a crate or
   a finding.

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
exists. Inventorying by crate name keeps the list free of version and graph
records, which live in `Cargo.lock`. Failing on a stale entry keeps the
inventory equal to the closure, so a reader can take it as the complete
list.

First-party crates are judged on shipped source because their tests may
need unsafe code that never links into the evaluator (`quire-canonical`'s
counting test allocator), and the scan's per-package counts cannot separate
those files; the checker's own parse of the shipped module tree can.
Third-party crates are judged on every file because the gate does not
review their layout.

Running in `agent-ix/quire-eval` places the gate where the closure is
resolved: that repository's lockfile fixes the shared leaves' revisions, and
the `-p quire-eval` resolution selects their `no_std` features, which QSL's
workspace cannot reproduce.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Checker findings over the closure in `make audit-unsafe` | 0 | 0 | Test |
| Gate fixtures whose verdict differs from the expected finding | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-016-AC-1 | `make audit-unsafe` in `agent-ix/quire-eval` runs the checker over the closure `cargo tree --locked -p quire-eval -e normal,no-proc-macro --target thumbv7em-none-eabi` reports and a `cargo-geiger` JSON scan of `quire-eval` through its absolute manifest path for that target with `--locked`, and passes on the repository's main branch; `make ci` lists `audit-unsafe` as a prerequisite; and `unsafe-inventory.toml` holds an entry for each third-party closure crate the scan reports with unsafe code and for no other crate. | Test (TC-1130) |
| NFR-016-AC-2 | Every first-party crate in the closure declares `unsafe_code` at level `forbid` in a place the first-party rule lists. Over fixture crates, the checker accepts a crate's own `[lints.rust]` forbid, its table form with a `priority`, `[lints] workspace = true` under a git-checkout workspace root whose `[workspace.lints.rust]` forbids it, and `workspace = true` under a root that does not, with `#![forbid(unsafe_code)]` at the crate root; it reports `first-party-not-forbidden` for a crate declaring `unsafe_code = "deny"` only and for a crate declaring nothing, and fails. | Test (TC-1131) |
| NFR-016-AC-3 | Over fixture crates that declare `forbid`, the checker reports `first-party-unsafe` naming the crate and file for an unsafe block in a module the library root declares, including a module declared under `#[cfg(any())]`, and reports nothing for an unsafe block in a file under `tests/`. In a scratch copy of `agent-ix/quire-eval`, adding `#[cfg(any())] mod probe;` to `quire-eval`'s library root with `probe.rs` holding `unsafe {}` makes `make audit-unsafe` print `first-party-unsafe quire-eval` and fail. | Test (TC-1131) |
| NFR-016-AC-4 | Given a fixture closure in which `quire-exact` reaches a third-party crate whose scan reports unsafe code and the inventory has no entry for it, the checker reports `unlisted-unsafe` naming the crate and `quire-exact` and fails; adding a well-formed entry for it makes the same fixture pass. | Test (TC-1131) |
| NFR-016-AC-5 | Given an inventory entry for a crate that is not in the fixture closure, and separately for a closure crate whose scan reports no unsafe code, the checker reports `stale-entry` naming the crate and fails. | Test (TC-1131) |
| NFR-016-AC-6 | Given an entry with a `version` key, an entry with a `reached_through` key, an entry with an empty `acceptance`, two entries with the same `name`, two entries out of `name` order, and an entry naming the first-party crate `quire-checked`, the checker reports `malformed-entry` for each and fails. | Test (TC-1131) |
| NFR-016-AC-7 | Given a fixture in which unsafe code is reported only in a proc-macro crate and in a crate reached only through it, and that crate is absent from the closure, the checker reports no finding and requires no entry, and an entry for that proc-macro crate reports `stale-entry`. Given a scan that lists a closure package under `packages_without_metrics`, and separately a scan whose `packages` lacks a closure package, the checker reports `unscanned` naming it and fails. A scan listing a build-script output file under `used_but_not_scanned_files` yields no finding. | Test (TC-1131) |
| NFR-016-AC-8 | With `cargo-geiger` absent from `PATH`, `make audit-unsafe` exits non-zero and prints the install command; the Makefile, the checker's command line and its environment offer no setting that skips the gate, a crate or a finding. | Test (TC-1130) and Inspection |

## Verification

TC-1130 runs the gate in `agent-ix/quire-eval`: the real closure, the real
inventory, the `make ci` wiring and the missing-tool failure. TC-1131 runs
the checker's own tests over fixtures, one per finding kind and declaration
form, plus the scratch-copy probe module. Rust tests carry
`#[trace("TC-...", "NFR-016-AC-...")]` attributes (NFR-005).

## Dependencies

- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md) slice S8
  (IR-592).
- [FR-400](../functional/FR-400-separate-checked-terms-from-the-checker.md)
  and [FR-401](../functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md):
  `quire-checked` and `quire-eval`, their repository and dependency sets.
- [FR-403](../functional/FR-403-implement-the-no-std-evaluator-in-quire-eval.md):
  the `no_std` evaluator in `quire-eval`, which the gate scans; the gate
  passes only once it exists.
- [NFR-015](NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md):
  the `thumbv7em-none-eabi` target.
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 layers C and E and FB-05 (shared leaves).

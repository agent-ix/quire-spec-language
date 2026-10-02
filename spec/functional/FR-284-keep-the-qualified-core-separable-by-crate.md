---
id: FR-284
title: "Keep the qualified core separable by crate"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: references
---
# FR-284: Keep the qualified core separable by crate

## Description

The qualified core is the check path (S0 to S4), the prove path (E5 to S8,
with replay through S6a) and the certificate checkers through which
`analyze` enters it (ADR-029 CB-2, FR-282). Its QSL crates are
`quire-exact`, `quire-semantic-value`, `quire-walk`, `qsl-foundation`, `qsl-cst`,
`qsl-source`, `qsl-forms`, `qsl-semantics`, `qsl-package`, `qsl-eval`,
`qsl-route` and `qsl-replay`. Outside QSL it also holds the driver library
`quire-driver` and the CG, IR and RT crates the prove path reaches, under
their own records.

The crates above the core are QSL's `qsl-analyze` (layer A), `qsl-inspect`
(layer P) and `qsl-jit` (layer X), and the driver's `quire-plugin-host`,
`quire-cache`, `quire-aot` and `quire-cli` (ADR-029 CB-3 item 3). Each may
depend on the core. No core crate depends on any of them.

Separability lets a qualified binary be a thin frontend over the core with
the verbs `check`, `compile`, `prove`, `replay`, `check-certificate` and
`version`, JSON output and FR-285's exit function, added with no core change
(ADR-029 CB-3 item 4).

## Inputs

The QSL workspace's dependency graph, as `cargo tree` reports it for each
core crate.

## Outputs

The direction check's report: pass, or each offending (core crate,
dependency) pair.

## Behavior

- No QSL core crate shall depend, directly or transitively through normal
  dependencies, on:
  - a QSL workspace crate outside the core set;
  - a crate above the core: `qsl-analyze`, `qsl-inspect`, `qsl-jit`, or the
    driver's `quire-driver`, `quire-plugin-host`, `quire-cache`, `quire-aot`
    or `quire-cli`;
  - a CG or RT crate;
  - an argument parser, a terminal or colour crate, a renderer other than
    the JSON serialization of its own outcome types, a plugin host, a cache,
    or an execution backend other than the S6a interpreter.
- A third-party or first-party library crate outside those categories is
  allowed, including the IR crates ADR-011 §6.1 sanctions
  (`quire-contract-model`). Proc-macro crates and build and dev dependencies
  link nothing into a core crate and are not walked.
- The direction check shall run over the `cargo tree` of each core crate,
  with ADR-011 T-12's tooling, and fail naming each offending pair.
- No QSL core crate shall read an environment variable, a configuration
  file, a search path, a clock or a global registry. A monotonic counter,
  such as an atomic token counter, is not a global registry.
- No QSL core crate shall write to stdout or stderr, or end the process.
- Each QSL core crate shall read the bytes it needs from its request, by
  digest where a digest names them (ADR-013 O-26).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-284-AC-1 | The direction check passes over the QSL workspace. With a test manifest that adds `qsl-analyze` to `qsl-eval`'s dependencies, and with one that adds an argument-parser crate to `qsl-replay`'s, it fails and names the pair `(qsl-eval, qsl-analyze)` and the pair `(qsl-replay, <that crate>)`. | Test (TC-767) |
| FR-284-AC-2 | Running FR-275-AC-1's chain and FR-098's replay of a fixture counterexample in a process whose environment holds arbitrary values for every variable the test generates, whose working directory is an empty temporary directory and whose home directory is unset, gives outcomes equal to those of a run in the test's own environment. | Test (TC-768) |
| FR-284-AC-3 | During AC-2's runs, the bytes written to the process's stdout and stderr by core crates are empty, and the process exits only when the test harness ends it. | Test (TC-768) |

## Status

The direction check and the ambient-input scan are implemented as `arch-lint
qualified-core` (make target `arch-lint-qualified-core`). The target is not
in `make ci` yet, because main has two known violations. Both are fixed in
filament-core-data:

- `agent-ix-extraction-frontend` always depends on `clap`, which only its
  binary uses, so `qsl-semantics` and every core crate above it reach an
  argument parser. Fix: put `clap` behind a feature in filament-core-data and
  update the pinned version in `qsl-semantics`.
- `qsl-semantics` `model::intake::lift_document` creates a `tempfile`
  scratch directory, because filament-core-data's `lift` writes its document
  only to an output path. Fix: a `lift` in filament-core-data that returns
  the document bytes.

The runtime half of FR-284-AC-2 and AC-3 (TC-768's child-process run) waits
on FR-275's typed lifecycle API.

## Dependencies

- ADR-029 CB-2, CB-3: the core and separability.
- ADR-011 T-12: the dependency-direction tooling.
- ADR-013 O-26: bytes by digest.
- [FR-059](FR-059-check-backend-dependency-direction.md), [FR-060](FR-060-check-qsl-api-surface-boundary.md): the existing direction and API-surface checks this one extends.
- [FR-282](FR-282-check-analyze-certificates-in-the-qualified-core.md): the certificate checkers.

## Overlap

The driver repository owns the qualified binary's verbs and the direction
check over `quire-driver`.

## References

- QSL-390 (ARCH-50): owner ruling 1 (separable qualified core) and RU-2,
  recorded on the ticket.

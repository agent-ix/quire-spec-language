---
id: FR-063
title: "Make semantic-family extension exhaustive at the core dispatch seams"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-006
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
---
# FR-063: Make semantic-family extension exhaustive at the core dispatch seams

## Description

ADR-012 §5.1 closes four enums inside the QSL crate that every family
extension touches: the family catalogue (`FamilyKind`), the parsed form
enum together with its leading-token kind enum, the checked node enum, and
each family's own `Cause` enum. QSL SHALL make adding a variant to any of
these four enums fail the build at every seam ADR-012 §5.1 lists for that
enum (S1 to S4), and SHALL provide a probe mechanism that demonstrates this
on every build of the full gate, not only by inspection of the source.

## Inputs

- The four closed enums (`FamilyKind`, the parsed form enum with its
  leading-token kind enum, the checked node enum, each family `Cause` enum)
  and every `match` site over them.
- The `seam-probe` cargo feature, enabled only by the probe build.
- A checked-in list of the seam functions the probe build's compiler errors
  are expected to name.

## Outputs

- A build under the `seam-probe` feature that fails to compile with exactly
  the checked-in set of rustc `E0004` (non-exhaustive match) locations.
- A non-zero `xtask seam-probe` exit and a diagnostic naming any location
  that appeared and is not in the checked-in list, or any checked-in location
  that did not appear.

## Behavior

### Probe variant per closed enum

Each of `FamilyKind`, the parsed form enum, the checked node enum and each
family `Cause` enum SHALL carry one additional variant, gated behind the
`seam-probe` cargo feature, that exists for no purpose other than exercising
exhaustiveness. No code outside the probe build SHALL construct or match on
a probe variant, and the `seam-probe` feature SHALL NOT be enabled by any
other feature or by default.

### `xtask seam-probe`

`xtask seam-probe` SHALL build the QSL crate with the `seam-probe` feature
enabled and collect every rustc `E0004` diagnostic location the build
reports. It SHALL compare that location set against a checked-in list of seam
functions and SHALL fail when the two sets differ in either direction: a
location present in the build but absent from the list, or a location in the
list that the build does not report.

`xtask seam-probe` SHALL run as part of the full gate.

### Which seams the probe covers

Because every S1 to S4 enum and every `match` site over it lives in the one
QSL crate (ADR-012 §1), one `seam-probe` build reports all of them. The
checked-in seam-function list SHALL include, at minimum, one entry for each
of:

- the `FamilyKind` prefix arm of `catalog_code()` and the stage-participation
  table that names which hook each family has at each stage, including the
  explicit `Relation` evaluation arm;
- the parser's leading-token-kind entry table and the check seam over the
  parsed form enum;
- the checked node enum's evaluator, v2 emitter and requirement-derivation
  matches;
- each family `Cause` enum's `catalog_code()`.

### No wildcard arm, and no `#[non_exhaustive]` enum

No `match` at an S1 to S4 seam SHALL carry a `_` arm or a catch-all arm. Every
module containing a listed seam SHALL deny
`clippy::wildcard_enum_match_arm` and
`clippy::match_wildcard_for_single_variants`, and the lint gate SHALL run
both lints over every such module.

None of `FamilyKind`, the parsed form enum, the checked node enum or any
family `Cause` enum SHALL carry `#[non_exhaustive]`. A `#[non_exhaustive]`
enum forces a cross-crate `match` over it to carry a `_` arm to compile at
all, which would let a seam accept a new variant through that `_` arm with
no `E0004` and no probe failure, defeating the seam entirely.

### A correct-looking implementation without a probe fails this requirement

A change that adds a match arm for every family without a probe mechanism
that runs in the full gate does not satisfy this requirement: the seam list
SHALL be demonstrated, on every full-gate run, by an actual failing build
under the `seam-probe` feature, not asserted by code review or by a comment.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-063-AC-1 | Building the QSL crate with `--features seam-probe` fails to compile with `E0004` at every seam function in the checked-in list, and at no other location. Removing one seam function's exhaustive-match arm from the checked-in list without removing the corresponding compiler error causes `xtask seam-probe` to fail with a diagnostic naming that location as unexpected-but-present. | Test (TC-161) |
| FR-063-AC-2 | Adding a match arm for the probe variant at one seam function, without adding it at the others, leaves `xtask seam-probe` reporting that seam's location as no longer present while every other checked-in location still appears; `xtask seam-probe` fails, naming the missing location. | Test (TC-161) |
| FR-063-AC-3 | Building the QSL crate with default features (no `seam-probe`) compiles cleanly with no probe variant reachable from any non-probe code path; a test asserts the feature is absent from `default` in `Cargo.toml` and from every other feature's dependency list. | Test (TC-161) |
| FR-063-AC-4 | `xtask seam-probe` runs to completion and exits non-zero when the checked-in list and the actual `E0004` location set differ in either direction (extra or missing), and exits zero only when the two sets are equal; a test with a deliberately wrong checked-in list (one entry removed) demonstrates the non-zero exit with a concrete example, not only an assertion that the tool "checks" the list. | Test (TC-161) |
| FR-063-AC-5 | The full gate invokes `xtask seam-probe`, and a test that stubs the gate's target list shows the gate fails when `xtask seam-probe` exits non-zero. | Test (TC-161) |
| FR-063-AC-6 | The checked-in seam-function list contains at least one entry for each of: the `FamilyKind` `catalog_code()` prefix arm; the stage-participation table, including the explicit `Relation` evaluation arm; the parser's leading-token-kind entry table and the check seam over the parsed form enum; the checked node enum's evaluator, v2 emitter and requirement-derivation matches; and each family `Cause` enum's `catalog_code()`. A checked-in list missing the entry for any one of these categories, run against a real build that has an `E0004` at that category's location, causes `xtask seam-probe` to fail, naming the missing category's location as expected-but-absent from the list. | Test (TC-161) |
| FR-063-AC-7 | A `match` at an S1 to S4 seam that carries a `_ => unsupported(...)` fallback arm produces no `E0004` for that seam under `--features seam-probe` and is therefore invisible to `xtask seam-probe` alone; `cargo clippy` over that seam's module fails on `clippy::wildcard_enum_match_arm` (or `clippy::match_wildcard_for_single_variants`, for the enum that trips it instead), so the lint gate, not the seam probe, is what catches this case. A test reintroduces such a fallback arm in a fixture module and asserts the clippy lint fires. A separate test inspects the definition of `FamilyKind`, the parsed form enum, the checked node enum and each family `Cause` enum and asserts none carries `#[non_exhaustive]`. | Test (TC-161) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §5.1
  (closed seams table, and the wildcard-arm/`#[non_exhaustive]` rules that
  make the failure certain), §5.3 (seam-probe evidence rule).
- [FR-062](FR-062-implement-checked-family-contract.md) defines the four
  closed enums this requirement's probe targets.
- [US-006](../usecase/US-006-extend-a-family-without-breaking-seams.md).

## Status

Specified under
[#214](https://github.com/agent-ix/quire-spec-language/issues/214). Not yet
implemented. ADR-012 §5.3 states the seams that cross repositories (S5-S9)
are probed where their enums are defined, by the owning repository; this
requirement covers only S1-S4, which are wholly inside the QSL crate.

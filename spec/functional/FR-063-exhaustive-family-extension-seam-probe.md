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
- The `seam_probe` build-configuration flag: a `--cfg` name set through
  `RUSTFLAGS`, not a Cargo feature (see this section's own correction note
  below), enabled only by the probe builds.
- The `seam_probe_downstream` build-configuration flag: a second `--cfg`
  name, set only by the probe builds of the root crate, `qsl-route` and
  `qsl-eval`, that gives each seam `match` in `qsl-semantics` its probe arm
  so that crate compiles in those builds.
- A checked-in list of the seam functions the probe builds' compiler errors
  are expected to name.

**Correction to merged spec.** This requirement originally specified
`seam-probe` as a Cargo feature. That is a design error, found and corrected
during #214's implementation, before any other ticket depended on it: Cargo
features are additive by contract (enabling any set of them must still
build), and this probe is designed to fail to compile whenever it is active
-- so a Cargo feature for it makes `cargo build --all-features` (and any
gate that runs it) fail permanently the moment the feature exists, with no
way to exclude it from `--all-features`'s activation set. A `RUSTFLAGS`-set
`--cfg`, outside the feature system entirely, is invisible to
`--all-features` and is the correct mechanism for a deliberately-failing
compile check. Every occurrence of "`seam-probe` cargo feature" /
`--features seam-probe` below is replaced by "`seam_probe` cfg" /
`RUSTFLAGS=--cfg seam_probe`.

## Outputs

- Four probe builds, each failing to compile, whose rustc `E0004`
  (non-exhaustive match) locations together are exactly the checked-in set.
- A non-zero `xtask seam-probe` exit and a diagnostic naming any location
  that appeared and is not in the checked-in list, or any checked-in location
  that did not appear.

## Behavior

### Probe variant per closed enum

Each of `FamilyKind`, the parsed form enum, the checked node enum and each
family `Cause` enum with at least one real variant SHALL carry one
additional variant, gated behind the `seam_probe` cfg, that exists for no
purpose other than exercising exhaustiveness against real production
`match`es already reachable outside the probe construct itself. A `Cause`
enum with zero real variants has no such production `match` to break, so its
probe variant would test only its own `catalog_code()` mapping -- the
construct testing itself, not a seam; a family in that state does not carry
a probe variant until it has a real cause. No code outside the probe builds
SHALL construct or match on a probe variant, and the `seam_probe` and `seam_probe_downstream` cfgs SHALL be set only by
`xtask seam-probe`'s own builds: by no Cargo feature, `build.rs`,
`.cargo/config.toml` or other build path, and not by default.

### `xtask seam-probe`

`xtask seam-probe` SHALL run four probe builds and collect every rustc
`E0004` diagnostic location each reports:

1. `qsl-semantics` with `RUSTFLAGS=--cfg seam_probe`;
2. the root crate (`quire-spec-language`) with `RUSTFLAGS=--cfg seam_probe
   --cfg seam_probe_downstream`;
3. `qsl-route` with `RUSTFLAGS=--cfg seam_probe --cfg
   seam_probe_downstream`. The root crate names `qsl-route` only as a dev
   dependency, so building the root crate's library never compiles it;
4. `qsl-eval` (layer 5, QSL-183) with `RUSTFLAGS=--cfg seam_probe --cfg
   seam_probe_downstream`. The root crate does not depend on `qsl-eval`, so
   building the root crate's library never compiles it either.

A crate that fails to compile stops every crate that depends on it, so one
build cannot reach both `qsl-semantics`' seams and the seams of the crates
above it that match over `qsl-semantics`' probe variants.
`seam_probe_downstream` exists only so the second, third and fourth builds
compile `qsl-semantics`: under it, each seam `match` in `qsl-semantics` has
an arm for its probe variant, and it gates nothing else. Every build SHALL
fail. `xtask seam-probe` SHALL compare
the union of their location sets against a checked-in list of seam
functions and SHALL fail when the two sets differ in either direction: a
location present in a build but absent from the list, or a location in the
list that no build reports.

`xtask seam-probe` SHALL run as part of the full gate.

### Which seams the probe covers

The four probe builds together report every S1 to S4 seam in the QSL
workspace crates (ADR-012 §1). The
checked-in seam-function list SHALL include, at minimum, one entry for each
of:

- the `FamilyKind` prefix arm of `catalog_code()`;
- the S6a seam's `match` over the S6a family kind (`evaluate_declaration`),
  which has no `Relation` arm (FR-090-AC-4);
- the parser's leading-token-kind entry table and the check seam over the
  parsed form enum;
- the checked node enum's evaluator, v2 emitter and requirement-derivation
  matches;
- each family `Cause` enum's `catalog_code()`.

**Correction to merged spec (stage-participation table deleted, PR #262
review, finding F7).** #214's first implementation pass built a
stage-participation table --
`stage_hooks(FamilyKind, Stage) -> HookStatus` in `qsl-semantics/src/family/mod.rs` -- as
its own checked-in S1 location, distinct from `catalog_code_prefix`'s prefix
arm. Review found that table's only non-test callers were three
`assert_eq!(stage_hooks(...), Implemented)` sites, each passing a literal,
compile-time-known `FamilyKind`/`Stage` pair into the same hand-written
`match` and asserting the result equalled the value that arm already
returns for those literals -- an assertion manufactured to give otherwise-
dead code a caller, not a real one. With those three call sites deleted
(correctly), `stage_hooks` itself has no real reader left: nothing in this
ticket's runtime asks "what hook status does family X have at stage Y" to
make an actual decision. It is deleted along with them, rather than kept
alive by more fabricated callers or `#[allow(dead_code)]`. The list above
therefore names no stage-participation table. The evaluation-stage entry is
the S6a seam's `match` over the S6a family kind (`evaluate_declaration`,
QSL-191), which has one arm per family that implements
`ReferenceEvaluation` and no `Relation` arm (FR-090-AC-4).

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

This requirement scopes the clippy denial to "every module containing a
listed seam" because that is ADR-012 §5.1's own scoping, so a seam this
requirement did not enumerate in a module it did not name is not covered by
that denial (a residual this requirement inherits from its authority rather
than exceeds). A crate-level `#![deny(clippy::wildcard_enum_match_arm)]` in
the QSL crate's `lib.rs` removes that module-by-module scoping in one line,
covering every module including one added after this requirement lands; the
implementing ticket may take that route, but does so as a deliberate choice
recorded in the implementation, not as an accidental consequence of
module-by-module denial lists falling out of sync with new seam modules.

### A correct-looking implementation without a probe fails this requirement

A change that adds a match arm for every family without a probe mechanism
that runs in the full gate does not satisfy this requirement: the seam list
SHALL be demonstrated, on every full-gate run, by an actual failing build
under `RUSTFLAGS=--cfg seam_probe`, not asserted by code review or by a
comment.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-063-AC-1 | Building `qsl-semantics` with `RUSTFLAGS=--cfg seam_probe`, and then the root crate, `qsl-route` and `qsl-eval` each with `RUSTFLAGS=--cfg seam_probe --cfg seam_probe_downstream`, each fails to compile with rustc error code `E0004` specifically, and the four builds together report every seam function in the checked-in list and no other location; `seam_probe_downstream` gates only the probe arms that let the second, third and fourth builds compile `qsl-semantics`; a build that fails for any other reason (a typo, an unrelated compile error, a misconfigured build) does not satisfy this criterion merely by failing -- the error code and the exact sites must match. Removing one seam function's exhaustive-match arm from the checked-in list without removing the corresponding compiler error causes `xtask seam-probe` to fail with a diagnostic naming that location as unexpected-but-present. | Test (TC-161) |
| FR-063-AC-2 | Adding a match arm for the probe variant at one seam function, without adding it at the others, leaves `xtask seam-probe` reporting that seam's location as no longer present while every other checked-in location still appears; `xtask seam-probe` fails, naming the missing location. | Test (TC-161) |
| FR-063-AC-3 | Building the root crate, `qsl-route` and `qsl-eval` with no `RUSTFLAGS=--cfg seam_probe` set (the three normal builds) compiles cleanly, with `xtask seam-probe` confirming both that each build succeeds and that it reports zero `E0004` locations -- a probe variant reachable from a non-probe code path would surface as an `E0004` in this same build, not merely as an absent Cargo feature. A test inspects the repository for anything that could set `seam_probe` outside `xtask seam-probe`'s own build invocation -- a `[features]` table entry, `build.rs`, `.cargo/config.toml`, or a `RUSTFLAGS`/`rustflags` setting in any `Makefile` target or CI workflow -- and asserts none exists; this is a grep-shaped check over those specific files, not a proof that no code path anywhere could set the cfg (a `build.rs` added later, for instance, would need this check re-run, not exempt it from the pattern it greps for). | Test (TC-161) |
| FR-063-AC-4 | `xtask seam-probe` runs to completion and exits non-zero when the checked-in list and the actual `E0004` location set differ in either direction (extra or missing), and exits zero only when the two sets are equal; a test with a deliberately wrong checked-in list (one entry removed) demonstrates the non-zero exit with a concrete example, not only an assertion that the tool "checks" the list. | Test (TC-161) |
| FR-063-AC-5 | The full gate invokes `xtask seam-probe`, and a test that stubs the gate's target list shows the gate fails when `xtask seam-probe` exits non-zero. | Test (TC-161) |
| FR-063-AC-6 | The checked-in seam-function list contains at least one entry for each of: the `FamilyKind` `catalog_code()` prefix arm; the S6a seam's `match` over the S6a family kind (`evaluate_declaration`); the parser's leading-token-kind entry table and the check seam over the parsed form enum; the checked node enum's evaluator, v2 emitter and requirement-derivation matches; and each family `Cause` enum's `catalog_code()`. A checked-in list missing the entry for any one of these categories, run against a real build that still has an `E0004` at that category's location (the source is unchanged), causes `xtask seam-probe` to fail, naming that category's location as present in the build but absent from the list (unexpected-but-present). | Test (TC-161) |
| FR-063-AC-7 | A `match` at an S1 to S4 seam that carries a `_ => unsupported(...)` fallback arm produces no `E0004` for that seam under `RUSTFLAGS=--cfg seam_probe` and is therefore invisible to `xtask seam-probe` alone; `cargo clippy` over that seam's module fails on `clippy::wildcard_enum_match_arm` (or `clippy::match_wildcard_for_single_variants`, for the enum that trips it instead), so the lint gate, not the seam probe, is what catches this case. A test reintroduces such a fallback arm in a fixture module and asserts the clippy lint fires. A separate test inspects the definition of `FamilyKind`, the parsed form enum, the checked node enum and each family `Cause` enum and asserts none carries `#[non_exhaustive]`. | Test (TC-161) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §5.1
  (closed seams table, and the wildcard-arm/`#[non_exhaustive]` rules that
  make the failure certain), §5.3 (seam-probe evidence rule).
- [FR-062](FR-062-implement-checked-family-contract.md) defines the four
  closed enums this requirement's probe targets.
- [US-006](../usecase/US-006-extend-a-family-without-breaking-seams.md).

## Status

Specified under
[#214](https://github.com/agent-ix/quire-spec-language/issues/214). ADR-012
§5.3 states the seams that cross repositories (S5-S9) are probed where
their enums are defined, by the owning repository; this requirement covers
only S1-S4, which are wholly inside the QSL crate.

**By Acceptance Criterion (PR #262 review, P3 accounting), with real trace
tags as they exist in the delivered code today:** none carries a
`#[trace(..., "FR-063-AC-N")]` tag.

- FR-063-AC-1 through AC-5 and AC-7 are exercised by `cargo xtask
  seam-probe`'s real, end-to-end behavior -- verified manually this review
  round (a genuine probe-build failure at exactly the checked-in location,
  a clean normal build, a non-zero exit on a deliberate mismatch) -- but
  none carries its own trace tag in a dedicated test, so all six are
  recorded unbacked rather than claimed. Owner: QSL-149 (AC-1, 2, 3, 4, 6, 7).
  AC-5's second half (the gate-stubbing test this criterion's own text
  requires) is tracked as a spec defect against this requirement's text,
  owned by QSL-155.
- FR-063-AC-6: unbacked (untagged, PR #262 review, coordinator round 3,
  finding 6; previously misrecorded as backed). It requires at least one
  checked-in entry for *each of* five categories; the checked-in list has
  the S1 `FamilyKind::catalog_code_prefix` prefix arm and the S6a seam
  `evaluate_declaration`, but not the S2, S3 and S4 categories. A criterion
  cannot be backed while its own Status text says it is not. The
  untagged tests in `xtask/src/seam_probe.rs` check that each checked-in
  location names a function holding a `match`
  (`checked_in_locations_name_functions_that_hold_a_match`) and exercise
  `enclosing_item_name`, the F14 line-to-item-name helper; neither is
  evidence for AC-6's category coverage. Owner:
  QSL-149, which owns this criterion alongside AC-1/2/3/4/7 (above) even
  though it stays unbacked regardless of who owns it: the S2/S3 categories
  land under QSL-143 and the S4 (family `Cause` `catalog_code()`) category
  under QSL-152, so QSL-149 tracks the criterion's own probe-mechanism
  coverage, not a promise that landing QSL-149 alone backs AC-6.

Zero of this requirement's seven Acceptance Criteria are backed by a
dedicated, trace-tagged test today.

---
id: TC-161
title: "The seam probe demonstrates exhaustiveness at every S1-S4 seam"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-063
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: verifies
---
# TC-161: The seam probe demonstrates exhaustiveness at every S1-S4 seam

## Description

Verify that the four probe builds -- `qsl-semantics` under
`RUSTFLAGS=--cfg seam_probe`, then the root crate, `qsl-route` and
`qsl-eval` each under `RUSTFLAGS=--cfg seam_probe --cfg
seam_probe_downstream` -- each fail to compile, together
with exactly the checked-in set of seam-function locations (and with rustc
error code `E0004` specifically at each), that the three normal builds (the
root crate, `qsl-route` and `qsl-eval`, no `seam_probe` set) succeed with none, that removing a checked-in entry or removing an
actual compiler error each causes `xtask seam-probe` to report a mismatch,
that nothing in the repository sets `seam_probe` outside the probe builds'
own invocations, that the tool's exit code reflects set equality, that the
full gate fails when the tool does, that the checked-in list covers every
category the implementation actually delivers, that a wildcard-arm escape or
a `#[non_exhaustive]` enum is caught by the lint gate rather than silently
passing the probe. Scope: FR-063-AC-1 through FR-063-AC-7.

**Mechanism correction.** This test case originally verified a `seam-probe`
Cargo feature; FR-063's own spec now documents why that was a design error
(see its Inputs section) and the mechanism is `RUSTFLAGS=--cfg seam_probe`
instead. Steps 1 and 5 below are reworded for that mechanism; step 5's
substance changed from "absent from every Cargo feature's dependency list"
(a criterion that becomes vacuously true under a `--cfg`, since there is no
Cargo feature to inspect) to a grep-shaped check over the specific files
that could set the cfg, stated as exactly that, not as an equivalent
restatement.

**S4 category coverage remains out of scope; S2 and S3 landed under
QSL-143.** FR-062-AC-8 and FR-063-AC-6's `Cause`-enum category (S4) is still
unbacked: no family this repository has migrated has a real typed refusal
cause, so there is no `Cause` enum to demonstrate a probe over (see ADR-012's
record of this deferral; owned by QSL-152). FR-063-AC-6's S2 category (the
check seam over the parsed form enum, `Typer::infer_form` matching
`qsl-forms::Expression`) and S3 category (the checked-node-enum's evaluator,
`Machine::apply` in `qsl-eval`, and its identity-lowering pass,
`Lowering::lower_node` in `qsl-semantics`, both matching
`qsl-semantics::check::ir::NodeKind`) are delivered by
[QSL-143](https://linear.app/agent-ix/issue/QSL-143): both enums carry the
same `#[cfg(seam_probe)]` probe-variant treatment as `FamilyKind`, with a
protective arm at every other production `match` site the variant would
otherwise break. The parser's leading-token-kind entry table
(`qsl-forms::dispatch::dispatch`, S2's other named location) stays out: it
lives in `qsl-forms`, a dependency of every one of the seam probe's four
fixed build targets but never itself one of them, so giving it a seam's
no-arm treatment would hide every other seam behind it in every probe
build (see `xtask::seam_probe::checked_in_locations`'s own doc). This test
case's checked-in list therefore covers the `FamilyKind`, S6a/S7, S2 and S3
categories -- 4 of the 5 categories FR-063-AC-6 names -- and step 8 below
exercises each of those four in turn; S4 stays untested here pending
QSL-152.

## Test Procedure

1. Run `cargo build -p qsl-semantics --lib` with `RUSTFLAGS=--cfg
   seam_probe`, then `cargo build -p quire-spec-language --lib`,
   `cargo build -p qsl-route --lib` and `cargo build -p qsl-eval --lib` with
   `RUSTFLAGS=--cfg seam_probe --cfg seam_probe_downstream`; confirm each
   fails, and collect the union of their `E0004` diagnostic locations
   (via `--message-format=json`, filtering `compiler-message` entries whose
   `code.code` is `"E0004"`, reading each primary span's `file_name` and
   `line_start`) -- not merely whether the build failed.
2. Compare the collected location set against the checked-in seam-function
   list; run `xtask seam-probe` and capture its exit code and report.
3. Remove one entry from the checked-in list (leaving the source unchanged)
   and re-run `xtask seam-probe`.
4. Add a match arm for the probe variant at exactly one seam function in the
   source (leaving the checked-in list unchanged) and re-run
   `xtask seam-probe`.
5. Build the root crate's, `qsl-route`'s and `qsl-eval`'s `--lib` targets
   with no `RUSTFLAGS=--cfg seam_probe` set (the three normal builds) and
   confirm each succeeds with zero `E0004` locations (`xtask seam-probe`
   performs these builds itself and fails if one does not succeed or reports
   any `E0004`). Separately, grep `Cargo.toml`'s
   `[features]` table, every `build.rs`, every `.cargo/config.toml` and
   every `Makefile` target and CI workflow step for `seam_probe`,
   `seam_probe_downstream` or `--cfg seam_probe`; confirm the only match is
   `xtask seam-probe`'s own build invocations. State plainly that this grep does not prove no other code
   path could ever set the cfg (a `build.rs` added later would need this
   check re-run, not be caught by it retroactively) -- it proves the
   specific, named set of places FR-063-AC-3 lists today are clean.
6. Construct a checked-in list missing one real entry and run
   `xtask seam-probe` against the real build; separately, run it against a
   correct list.
7. Confirm the real `Makefile`'s `ci:` target names `seam-probe` as a
   prerequisite and that the `seam-probe:` target's own recipe runs
   `cargo xtask seam-probe` (a grep-shaped check over the real file, not a
   stub of a Rust-level gate-target-list abstraction, which does not exist
   -- QSL-155's correction to this criterion's original text). Separately,
   on a minimal fixture `Makefile` of the same `ci:`/prerequisite shape,
   confirm a failed prerequisite fails the aggregate target and a
   succeeding one does not.
8. Inspect the checked-in seam-function list and confirm it names each of
   the four categories this ticket's tree delivers -- `FamilyKind`'s prefix
   arm (S1), the S6a/S7 matches, `Typer::infer_form` (S2) and
   `Machine::apply`/`Lowering::lower_node` (S3); remove the entry for one
   category at a time and re-run `xtask seam-probe` against the real build.
   S4 is not exercised by this step -- see this test case's own "S4
   category coverage" note.
9. In a fixture seam module, replace one seam's exhaustive `match` with one
   carrying a `_ => unsupported(...)` fallback arm. Build the fixture with
   `RUSTFLAGS=--cfg seam_probe` and collect its `E0004` locations;
   separately run `cargo clippy` over the fixture module.
10. Inspect the definitions of `FamilyKind`, the parsed form enum
    (`Expression`), the checked node enum (`NodeKind`) and the one
    cause-bearing type this repository has today (`WrongSnapshotCause`) for
    `#[non_exhaustive]`.

## Expected Results

- Step 1: the collected `E0004` location set matches the checked-in list
  -- the `FamilyKind` prefix arm (S1), the S6a/S7 matches, `Typer::
  infer_form` (S2) and `Machine::apply`/`Lowering::lower_node` (S3). The
  parser's leading-token-kind entry table (S2's other named location) and
  each family `Cause` enum's `catalog_code()` (S4) are not in this list;
  see this test case's "S4 category coverage" note.
- Step 2: `xtask seam-probe` exits zero and reports no mismatch when the sets
  are equal.
- Step 3: `xtask seam-probe` exits non-zero, naming the removed entry's
  corresponding location as unexpected-but-present.
- Step 4: `xtask seam-probe` exits non-zero, naming that seam's location as
  expected-but-missing.
- Step 5: all three normal builds compile cleanly with zero `E0004` locations, so
  the probe variant is unreachable from any non-probe path in the builds
  that would actually catch it if it were not; the grep finds `seam_probe`
  set nowhere but `xtask seam-probe`'s own build invocation.
- Step 6: the wrong list produces a non-zero exit naming the missing entry;
  the correct list produces a zero exit.
- Step 7: `ci:` names `seam-probe` as a prerequisite, whose recipe runs
  `cargo xtask seam-probe`; the fixture `Makefile` fails its aggregate
  target exactly when the prerequisite's recipe fails, and not otherwise.
- Step 8: removing any one of the four delivered categories causes `xtask
  seam-probe` to fail, naming that category's location as present in the
  build but absent from the list (unexpected-but-present, the same
  direction step 3 demonstrates).
- Step 9: the fixture build with the fallback arm produces no `E0004` at
  that seam (the fallback arm compiles, so `xtask seam-probe` alone would
  report the sets as equal and exit 0 against a list that omits it); the
  `cargo clippy` run over the fixture module fails on
  `clippy::wildcard_enum_match_arm` or `clippy::match_wildcard_for_single_variants`.
- Step 10: none of the four types carries `#[non_exhaustive]`.

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

Verify that the three probe builds -- `qsl-semantics` under
`RUSTFLAGS=--cfg seam_probe`, then the root crate and `qsl-route` each
under `RUSTFLAGS=--cfg seam_probe --cfg seam_probe_downstream` -- each fail
to compile, together
with exactly the checked-in set of seam-function locations (and with rustc
error code `E0004` specifically at each), that the two normal builds (the
root crate and `qsl-route`, no `seam_probe` set) succeed with none, that removing a checked-in entry or removing an
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

**S2/S3/S4 category coverage is out of #214's scope.** FR-062-AC-8 and
FR-063-AC-6's `Cause`-enum category (S4) is unbacked: no family this
repository has migrated has a real typed refusal cause, so there is no
`Cause` enum to demonstrate a probe over (see ADR-012's record of this
deferral). FR-063-AC-6's S2 and S3 categories (the parser/parsed-form-enum
seam and the checked-node-enum seams) are tracked by
[QSL-143](https://linear.app/agent-ix/issue/QSL-143), not delivered here:
S2 and S3 sit over pre-existing, crate-wide enums used by every `Value`
form, not just the two forms (function declaration and application) #214
migrates. This test case's checked-in list, as #214 delivers it, therefore
covers only the `FamilyKind` category (S1's two matches) -- 1 of the 5
categories FR-063-AC-6 names -- and steps 8 and 9 below are scoped to that
one category rather than exercised against categories this ticket does not
deliver.

## Test Procedure

1. Run `cargo build -p qsl-semantics --lib` with `RUSTFLAGS=--cfg
   seam_probe`, then `cargo build -p quire-spec-language --lib` and
   `cargo build -p qsl-route --lib` with
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
5. Build the root crate's and `qsl-route`'s `--lib` targets with no
   `RUSTFLAGS=--cfg seam_probe` set (the two normal builds) and confirm each
   succeeds with zero `E0004` locations (`xtask seam-probe` performs these
   builds itself and fails if either does not succeed or reports any
   `E0004`). Separately, grep `Cargo.toml`'s
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
7. Stub the full gate's target list to include `xtask seam-probe`, then
   simulate a non-zero exit from the tool and observe the gate's own exit
   code.
8. Inspect the checked-in seam-function list and confirm it names the one
   category #214 delivers (the `FamilyKind` prefix arm and the
   stage-participation table, both in `qsl-semantics/src/family/mod.rs`); remove the
   entry for that category and re-run `xtask seam-probe` against the real
   build. The other four categories FR-063-AC-6 names (the second S1 match
   is covered above; S2, S3 and S4) are not exercised by this step -- see
   this test case's own "S2/S3/S4 category coverage" note.
9. In a fixture seam module, replace one seam's exhaustive `match` with one
   carrying a `_ => unsupported(...)` fallback arm. Build the fixture with
   `RUSTFLAGS=--cfg seam_probe` and collect its `E0004` locations;
   separately run `cargo clippy` over the fixture module.
10. Inspect the definitions of `FamilyKind`, the parsed form enum and the
    checked node enum for `#[non_exhaustive]`.

## Expected Results

- Step 1: the collected `E0004` location set matches the checked-in list
  -- today, the `FamilyKind` prefix and stage-participation matches in
  `qsl-semantics/src/family/mod.rs` (ADR-012 §5.1's S1). The parser entry table and check
  seam (S2), the checked-node-enum evaluator/emitter/requirement-derivation
  matches (S3) and each family `Cause` enum's `catalog_code()` (S4) are not
  in this list; see this test case's "S2/S3/S4 category coverage" note.
- Step 2: `xtask seam-probe` exits zero and reports no mismatch when the sets
  are equal.
- Step 3: `xtask seam-probe` exits non-zero, naming the removed entry's
  corresponding location as unexpected-but-present.
- Step 4: `xtask seam-probe` exits non-zero, naming that seam's location as
  expected-but-missing.
- Step 5: both normal builds compile cleanly with zero `E0004` locations, so
  the probe variant is unreachable from any non-probe path in the builds
  that would actually catch it if it were not; the grep finds `seam_probe`
  set nowhere but `xtask seam-probe`'s own build invocation.
- Step 6: the wrong list produces a non-zero exit naming the missing entry;
  the correct list produces a zero exit.
- Step 7: the full gate exits non-zero when `xtask seam-probe` does.
- Step 8: removing the one category #214 delivers causes `xtask seam-probe`
  to fail, naming that category's location as present in the build but
  absent from the list (unexpected-but-present, the same direction step 3
  demonstrates).
- Step 9: the fixture build with the fallback arm produces no `E0004` at
  that seam (the fallback arm compiles, so `xtask seam-probe` alone would
  report the sets as equal and exit 0 against a list that omits it); the
  `cargo clippy` run over the fixture module fails on
  `clippy::wildcard_enum_match_arm` or `clippy::match_wildcard_for_single_variants`.
- Step 10: none of the three enums carries `#[non_exhaustive]`.

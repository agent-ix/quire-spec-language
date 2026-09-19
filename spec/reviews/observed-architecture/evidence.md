---
id: SR-462
title: "Evidence analysis of ADR-010 observed architecture baseline"
type: SpecReview
analysis: evidence
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-010
    type: reviews
---
# SR-462: Evidence analysis of ADR-010

## Summary

Round 2. Reviewed commit 432e615 on `task/206-observed-architecture`, against
round 1 at faa1731. ADR-010 is a descriptive record. Its one obligation, from
#206, is that every assertion has an evidence path and a revision. This round
checks each round-1 finding, and every citation that is new or changed in the
faa1731..432e615 diff. It does not re-check the whole record.

All three round-1 `high` findings are fixed. Of the 12 round-1 findings, 11 are
resolved and 1 is partially resolved (FND-008). None is unresolved.

The new and changed citations hold, with seven exceptions, all listed below.
That covers the code citations, the 12 negative-evidence cells, the §3.1 module
table and the GitHub facts. Every `absent:` cell returns nothing at its pinned
revision. The §3.1 module table was recounted in full with the stated method:
all 31 rows match on both lines and public items.

The two `medium` findings:

- The record says no CI job runs IT-010. In fact a manually dispatched CI run
  does run it, and it fails there.
- The §7.5 downstream-issue rule is stated as exhaustive, but it misses issues
  that mapped QSL bodies cite.

Verdict: ACCEPT WITH FINDINGS. No `high` finding is open. The two `medium` and
five `low` findings are corrections to wording, counts or citations. None needs
structural rework.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | The claim that no CI job runs IT-010 is false. §2.1 says "QSL CI is `workflow_dispatch` only and installs no cargo-kani (`QSL:.github/workflows/ci.yml:3-4,13-16`), so no CI job runs IT-010". The Summary counts row and OBS-002 say the same ("not run by CI", "no CI job runs it"). But the dispatched job runs `cargo test --locked --workspace --no-default-features` and `--all-features` (`QSL:.github/workflows/ci.yml:22,26`). `tests/configversion_backends.rs` has no `required-features` in `QSL:Cargo.toml` and no `#[ignore]`, so both runs compile and execute IT-010. With no cargo-kani installed, the IT-010 Kani tests panic at `QSL:tests/configversion_backends.rs:513` (`expect("cargo-kani 0.67.0 must be installed")`). Corrected: "QSL CI runs only on manual dispatch (`ci.yml:3-4`). A dispatched run executes IT-010 through `cargo test --workspace` (`ci.yml:22,26`) but installs no cargo-kani, so IT-010 fails there (`tests/configversion_backends.rs:513`). No CI run passes IT-010." Change the Summary counts row to "test-only, never passes in CI". | ADR-010 Summary counts, §2.1, §9.2 OBS-002 |
| FND-002 | medium | The §7.5 rule is stated as complete, and it is not. It says §7.5 holds "every downstream issue referenced in the body of an issue mapped in §7.1–§7.4", and the Summary counts row repeats this ("Every downstream issue cited by a mapped QSL issue body", 28). Mapped bodies at 2026-09-19 cite downstream issues that §7.5 leaves out: QSpec #63 (QSL #1, lines 6 and 131, "Blocked until agent-ix/quire-specification#63 Task-010 passes"), QSpec #104 (#155), QSpec #13 (#42), spec-objects-business #8 (#133) and quire-wasm #6 (#207). Either add them and update the count, or narrow the rule in words, for example "open issues in the QSpec, IR, RT, CG and FCD repositories cited by architecture-relevant bodies", so that the rule and the count agree. | ADR-010 Summary counts, §7.5 |
| FND-003 | low | This is the rest of round-1 FND-008. The §3.2 "IR types used inside QSL (use counts)" (`ValueType` 212, `SymbolName` 103, … `DeclarationEnvironment` 20) still has no reproducible command, and §4.4 reuses the 212. The counts depend on the method: `git grep -c -w ValueType de627b5 -- src ':!src/value' ':!src/model' ':!src/complete'` gives 219 lines, not 212. State the exact pattern and path set that gives 212. The §3.1 fan-in and fan-out method is now stated, and the other FND-008 items are cited. | ADR-010 §3.2, §4.4 |
| FND-004 | low | S2 says "model→value: 7 `use crate::value` items". The §3.1 method excludes `#[cfg(test)]` modules, and under it the count is 6: `QSL:model/checked_dispatch.rs:97`, `QSL:model/conformance.rs:90`, `QSL:model/domain_package.rs:14`, `QSL:model/key.rs:19`, `QSL:model/normalize.rs:208` and `QSL:model/population.rs:146`. The 7th, `QSL:model/checked_dispatch.rs:860`, is inside `mod tests` (`#[cfg(test)]` at :850). The value→model count holds: 8 items in 6 files. | ADR-010 §3.1 S2 |
| FND-005 | low | FR-290 is cited as a whole file for claims about specific lines, which breaks the `<prefix>:<path>:<line>` convention. The claims are "6 protocol claim kinds", "QSL's enum aligns to its 6" and "names quire-spec-language's Kani backend as a registrant". Corrected: six labels `QSpec:spec/objects/protocol/FR-290-protocol-claim-kind.md:33`, QSL `Capability` alignment `:34-37`, Kani backend registrant `:50`. This applies to §1.2 (Codegen row), DA-11, OBS-012 and OBS-013. | ADR-010 §1.2, §5 DA-11, §9.2 OBS-012, OBS-013 |
| FND-006 | low | The Context says open PRs are cited "at their heads on 2026-09-19", and issue bodies are cited at 2026-09-19T17:02Z. Two of the cited shas were not the heads at that time: QSL #204 `6eee1f3` (head 2b02528 from 16:46:39Z) and IR #139 `64982f1` (head 417ec86 from 16:49:19Z). Every cited claim holds at both shas. #204 touches the same four `src/model` files, and `src/kani/witness.rs` exists at both #139 heads. So the record stays reproducible, but the wording is inaccurate. Corrected: "at the named revisions", or give the time of the head snapshot. Also, QSL #204 merged at 17:04:35Z (ba9e33b), closing #173, #174 and #176 and changing the OBS-007 site. §8's PR-sensitive list already routes that change to #208. | ADR-010 Context, §8 |
| FND-007 | low | The §9.2 "Load" paragraph says #211's 33 items "fall into three groups". The three groups listed hold 21 items. Twelve items are in none of them: DA-09, DA-10, DA-12, DA-13, DA-15, OBS-006, OBS-021, OBS-025, OBS-026, OBS-027, OBS-035 and OBS-039. Either add a group for outcomes, refusals, accounting, provenance and witness, or say that the groups are examples. The count and owner tallies (18, 6 and 17 findings; 33, 18 and 8 items) were recomputed from the tables and hold. | ADR-010 §9.2 |

## Round 1 resolution

| Round-1 ID | Severity | Status at 432e615 | Reason |
|---|---|---|---|
| FND-001 | high | resolved | The checked-handoff rows now cite `checked_predicate.rs:139,156` and `temporal_subject.rs:177,193`. Both are `pub fn derive` and `pub fn read` at those lines. |
| FND-002 | high | resolved | §3.2 now has "IR root → IR model, normal, workspace path, `IR:Cargo.toml:21`" and a separate dev self-pin row for `:37`. §6.3 gives the kind "normal / dev". Both check out at 553b6d1. |
| FND-003 | high | resolved | DA-06 and OBS-019 now say "10 variants; its integers are fixed-width `i64` and it has no function variant", citing `runtime/input.rs:94,101-104`, and OBS-019 cites AD-016:238. |
| FND-004 | medium | resolved | C5 cites `InputRefusal` at `value/expression/mod.rs:105` (`pub enum InputRefusal`). |
| FND-005 | medium | resolved | The B8 caller sentence is reworded. `git grep` finds no `native::admit`/`emit` call in `src/` outside `protocol_artifact/native`. The example lines 1411, 1415 and 1566 are B8 calls, and both cited test files call B8. OBS-011 is updated too. |
| FND-006 | medium | resolved | S2 and S3 now cite `use` items (`value/composite.rs:31`, `model/domain_package.rs:14`, `temporal.rs:29`) and state what the counts count. The model→value count is off by one (new FND-004). |
| FND-007 | medium | resolved | Every presence cell listed in round 1 now has a line number, and every one was checked. Absence cells use the new `absent:` form, and all 12 return nothing. FR-290 cells were missed in round 1 and are raised as new FND-005. |
| FND-008 | low | partially resolved | Exit codes, `read_verified` and `RunSelection::Extracted` are now cited and hold. The §3.1 fan-in and fan-out method is stated. The §3.2 IR-type use counts still have no command (new FND-003). |
| FND-009 | low | resolved | Every PR #200 citation now carries `@13b6687`, and the Context lists all cited PR heads. At 13b6687 the diff adds `src/model/intake.rs` with 2104 lines, `intake.rs:109,710-722` holds, `Cargo.toml:37-38,42` holds, and `Cargo.lock:1077,1098` holds. |
| FND-010 | low | resolved | DA-10 now says 45-variant `Code`. |
| FND-011 | low | resolved | §1.1, A11 and OBS-002 cite `:948-951`, and no `:946` remains. |
| FND-012 | low | resolved | §6.2 and OBS-028 say "IR integration test `IR:tests/kani_replay.rs:240`". It is the only caller outside the `src/kani/mod.rs:31` re-export. |

Counts: 11 resolved, 1 partially resolved, 0 unresolved.

## Method

Each citation was read at its pinned sha with `git -C <clone> show <sha>:<path>`
and `git grep -n`. Nothing was checked out, built or committed. Clones and shas:

- QSL de627b5; PR heads 13b6687 (#200), 6eee1f3 (#204) and a43e951 (#228)
- IR 553b6d1 and a5154d3
- CG a4b2a73 and 5e2a6a9
- RT d97bc0b
- QSpec 3a79dce
- QI 40cff46

A citation held when the file exists at the sha, the symbol is within 5 lines
of the cited line, and the claim is true. GitHub facts came from read-only `gh`.

What was checked:

- **QSL, new or changed cells:**
  - `command/compilation.rs:26,36,94,101-103`
  - `tests/configversion_backends.rs:32-34,252,259,262,509-526,543-544,597,789,820-821,831,837,840,843-857,871,948-951`
  - `.github/workflows/ci.yml` (whole file) ✗ (FND-001)
  - `main.rs:24-25,113-115,138` · `command.rs:227-232` · `package.rs:233-246`
  - `protocol_artifact/checked_predicate.rs:139,156` · `protocol_artifact/temporal_subject.rs:177,193` · `protocol_artifact/v2/intake.rs:476`
  - `examples/protocol-handoff/producer.rs:1411,1415,1566` · `tests/native_protocol_emission.rs` · `tests/compiled_protocol_v2.rs`
  - `value/expression/mod.rs:105`
  - `simulation/explore.rs:59,97` · `simulation/sample.rs:83,95` · `simulation/trace.rs:41,53,97`
  - `wire_format.rs:27,29,31,37,39` · `lib.rs:13-44`
  - S1 entering edges: `package/reading.rs:10`, `parser.rs:3,4`, `syntax.rs:3`, `lexer.rs:3`, `checking.rs:14`, `formal_source.rs:7`, `package.rs:20`, `runtime/execution.rs:4`
  - S2 and S3 edges: `value/composite.rs:31` · `model/domain_package.rs:14` ✗ count (FND-004) · `temporal.rs:29,50`
  - `Cargo.toml:17,26` and the absence cells · `Cargo.lock` (resolves CG's IR to 04eb6f8)
  - `tests/fixtures/native-lowering/Cargo.toml:14` · `tests/native_backend.rs:281` · `package/view.rs:39,46` (STANDARD e897f81 is a QSpec commit)
  - `model/key.rs:23,26,157` · `value/model_query.rs:108,123` (the only production `from_bytes` caller) · `value/node.rs:18,22,49` · `model/checked_dispatch.rs:657` · `model/population.rs:376`
  - `checking/types.rs:12` · `runtime/input.rs:94,101-104` · `complete/diagnostic.rs:13` · `value/accounting.rs:2,143` · `model/accounting.rs:127` · `source.rs:94` · `resources/native-v1/VENDOR.json:20`
- **Negative evidence** (all 12 empty):
  - QSL `src/`: `CheckedPackageV2` · `kani` (also case-insensitive) · `native-run-result/2`
  - QSL `src/model`: `mod intake`
  - `crate::lowering` and `EXECUTABLE_PROJECTION` in the five X5 paths
  - QSL `Cargo.toml`: `quire-exact` · `quire-contract-runtime` · `filament`
  - RT `src`: `replay` (case-insensitive)
  - QI `Makefile`: `heads`
- **§3.1 module table:** all 31 rows recounted with the stated line and
  public-item method. All match.
- **PR citations:**
  - `PR #200@13b6687:src/model/intake.rs:109,710-722` · `Cargo.toml:37-38,42` · `Cargo.lock:1077,1098`
  - `src/model` diff stats for #228, #204 and #200 against de627b5, which back the single-writer row
- **IR:**
  - `Cargo.toml:21,24,37,38` · `IR@a5154d3:Cargo.toml:24`
  - `src/kani/replay.rs:3-6,13-20,41,55-67,80-97`
  - `src/predicate/admission.rs:91-92` (QSL f1700a9 types, also in `temporal/admission.rs:6`)
  - `tests/kani_replay.rs:240` · `tests/fixtures/checked-package/PROVENANCE:117-127`
- **CG:** `CG@5e2a6a9:src/oracle.rs:14,17` · `assurance/pins.json:26` ·
  `Cargo.toml:17,18,27,28`.
- **QSpec:** `FR-290-protocol-claim-kind.md` ✗ no line numbers (FND-005). The lines
  are 33, 34-37 and 50.
- **GitHub:**
  - The 9 PR head shas: 2 were already superseded at 17:02Z (FND-006).
  - The ARCH-01 comment 5743530928 (16:35:16Z): merge order #228 → #204 → #200, and the 10 §8 dispositions.
  - #205: the 7 ownership boundaries, the four Layer 1 design tickets, and "#185 sole capability registry/routing implementation owner".
  - #209: the "Required design" flow. #212: seven scenarios and the failure rule.
  - #185 dependencies. The "Woven in after" edges and declared prerequisites of #186–#198.
  - `updatedAt` for the 11 L1-D1 issues: all match.
  - §7.1 prerequisites for #213, #216, #217, #218, #222, #229, #230, #231 and #232.
  - A scan of all 68 open QSL bodies for downstream references ✗ (FND-002).
- **Derived data:**
  - Summary counts: 41 findings, 53 reference claims (14 + 7 + 32) and 31 modules.
  - §1.2 tally: 0 agree, 5 partial, 2 disagree.
  - §9.2 owner tally: 18, 6 and 17. Load: 33, 18 and 8 of 59.
  - §7.5 row count: 28.

## Round 2 resolution (author)

Recorded by the authoring agent; the round-2 verdict stands.

- FND-001 resolved: §2.1, OBS-002 and the Summary row state that IT-010 runs on
  every full `cargo test` gate and fails at `configversion_backends.rs:513`
  without cargo-kani.
- FND-002 resolved: §7.5 rule widened and table completed (34 rows).
- FND-003 partly resolved: §3.2 states the counting method and marks the counts
  approximate, with the plain-grep figure.
- FND-004 resolved: S2 says 6 non-test items and names the test-only seventh.
- FND-005 resolved: FR-290 cells cite `:33-34,49-51`.
- FND-006 resolved: Context records the advanced heads and the #204 merge
  (`ba9e33b`, 17:04:35Z).
- FND-007 resolved: the §9.2 load paragraph has four groups covering all 33
  #211 items.

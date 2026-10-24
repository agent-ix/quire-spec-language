---
id: SR-12075
title: "Spec review of quire-spec-language PR #935: ADR-033 one-source type phase refresh"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@dd59b828d94a0d921e66e75d74349350eafde509; spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md, spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is-bound-by-digest.md, spec/test-cases/TC-1027-no-label-only-match-and-no-composed-parser-remain.md"
review_set: subset
---
# Spec review of quire-spec-language PR #935

## Summary

Ticket: QSL-783 (slice S0). One pass of spec-review over `git diff
origin/main...HEAD` (4 files, +77/-53): base checklist plus the integrity
checks (internal consistency, AC to TC coverage, id and edge validity). Spec-only
PR, so no code review. Major issues only, at most five findings.

Oracles, read-only on the reviewed head: `qsl-forms/src/dispatch.rs`
(`LeadingTokenKind::Temporal` at :178, :382, :420; no `relation` or `hyper`
entry), `qsl-semantics/src/check/assemble.rs` (`UnimplementedTemporalClause`
:321 maps to `Code::UnsupportedConstruct` :412), `qsl-cst/src/grammar.rs`
(`relation` :1566, `hyper` :1597), `spec/functional/FR-052-*.md` (QSL's own
FR-052, AC-1 to AC-8), and a strict `git grep` of composed `profile X = ".."
version` lines (18 files under `src`/`tests`, 12 of them `tests/it`, 40 under
`artifacts/`). `quire validate` on the four files: no errors.

Examined:
- ADR-033 Context bullets "Two CheckedUnit types" and "The families" (examined)
- ADR-033 Option A+D bullets: Type path, Migration cost, Fixtures and goldens, One CheckedUnit (examined)
- ADR-033 Decision bullets: Moved and unmoved families, Profile selection, Refusal and exhaustion (examined)
- FR-362 Target design items 3, 5, 7, 8, 10 (examined)
- FR-362-AC-4, FR-362-AC-7 (examined); FR-362-AC-1 to AC-3, AC-5, AC-6 (context_only)
- TC-1026 steps 4 and 5 with expected results (examined)
- TC-1027 step 3 and expected results for steps 1 and 3 (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-362 item 10 and the ADR-033 "Refusal and exhaustion" bullet cite `(FR-052-AC-10)` bare. In this repo a bare FR id resolves to QSL's own FR-052 ("publish native temporal evaluation owner"), which has only AC-1 to AC-8, all retired. The citation dangles and points at the wrong requirement; the intended IR-owned criterion is not identified. Fix: qualify it the way the repo cites other IR requirements ("Contract IR FR-052-AC-10" or "IR FR-052-AC-10"), id only. | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:81; spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:239 |
| FND-002 | medium | FR-362-AC-4 keeps a second clause ("the report type holds one unit, so a two-unit report cannot be built") with verification "Test (TC-1026)", but TC-1026 step 4 and its expected result now only compare inventory rows. No TC step verifies the clause, and a "cannot be built" property is not a runtime test. Fix: drop the clause from AC-4 (FR-362 item 3 already states one unit by construction), or make it an Inspection step in TC-1027. | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:93 |
| FND-003 | medium | FR-362 item 10 adds a normative rule with no AC and no TC step: a unit the S3 type phase refuses is an error outcome with no partial inventory rows, while `TypeWorkExhausted` / `TypeAdmissionUnfinished` stay `Incomplete` with the retained partial rows and the typed cause. None of AC-1 to AC-7 covers either half, so an implementation that emits partial rows for a refused unit, or drops them on exhaustion, passes every FR-362 test. Fix: add an AC (and a TC-1026 step) for both cases. | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:75-84 |
| FND-004 | medium | The R6 wording reads as a second `CheckedUnit`. ADR-033 says "`qsl_inventory` and its `CheckedUnit` live in `qsl_replay` and read the spine's", and FR-362 item 3 says "`qsl_inventory` with its `CheckedUnit` lives in `qsl_replay`". "Its CheckedUnit ... read the spine's" names an inventory-owned type beside `spine::CheckedUnit`, which contradicts "Exactly one public CheckedUnit", "the inventory's own CheckedUnit is deleted", the rejected alternative "A second CheckedUnit beside the spine's", and TC-1027 step 1. Fix: "`qsl_inventory` lives in `qsl_replay` and reads `spine::CheckedUnit`". | spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:176-178; spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:49-51 |
| FND-005 | medium | The corrected profile-line measurement names `src/linking/composed/parser.rs`, which does not exist. The composed profile line in that position is in `src/parser.rs` (:406). The counts (18 files, twelve `tests/it`, 40 under `artifacts/`) match a strict grep. Fix: replace the path with `src/parser.rs`. | spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md:152 |

## Verdict

FIX. One blocking finding (FND-001, a dangling id that resolves to the wrong
requirement) and four mediums the brief counts as major (an AC clause with no
verifying step, a requirement rule with no AC, a two-CheckedUnit contradiction,
a wrong path in the measurement this PR set out to correct). All are small text
fixes.

Rulings verified present and consistent:
- R3: `relation`/`hyper` are the unmoved example in FR-362-AC-7, TC-1026 step 5
  and ADR-033; `temporal` is out of the unmoved list. Matches the code: no
  `relation`/`hyper` dispatch entry; `temporal` has one, and S3 refuses it as
  `unsupported_construct`.
- R4: `DefinitionLock::pinned` plus spine `LockEvidence`; "no caller definition
  inventory"; composed revision/digest matching "deleted, not ported". No stale
  "caller's definition inventory" text remains in the four files.
- R5: the two-unit refusal is gone from FR-362 item 5, AC-4's refusal clause and
  TC-1026 step 4; no `NotOneUnit` anywhere in `spec/`.
- R6: present, with the wording defect in FND-004.
- R7: the handoff snapshots and `native_*_emission` goldens are "not
  re-digested"; no "goldens updated".
- R8: TC-1027 step 1 closes with the label-only types; steps 2 and 3 close
  with the composed lane.
- Refused unit vs exhaustion stops: stated (FND-003 is about its missing AC; FND-001 about its citation).
- Row identity: only the whole `RawSourceRef` (authority, identity, digest)
  binds; the row encoding is left to the owning FR. Not frozen.
- No QSpec text copied. The source digest is a canonical content identity, not
  a pin.

Not filed (below the major bar): TC-1027 says "(tracked by QSL-830)", a
ticket-tracking note in a test case.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | Fixed | FR-362 item 10 and the ADR-033 "Refusal and exhaustion" bullet cite `IR FR-052-AC-10`, id only. |
| FND-002 | Fixed | Dropped the "two-unit report cannot be built" clause from FR-362-AC-4; item 3 states one unit by construction. |
| FND-003 | Fixed | Added FR-362-AC-8 and TC-1026 step 6 for the refused-versus-exhausted rule. |
| FND-004 | Fixed | ADR-033 and FR-362 item 3 now say `qsl_inventory` lives in `qsl_replay` and reads `spine::CheckedUnit`. |
| FND-005 | Fixed | ADR-033 measurement names `src/parser.rs` instead of `src/linking/composed/parser.rs`. |
| Below bar | Fixed | Removed the ticket note from TC-1027. |

---
id: SR-320
title: "Composed binding delivery gaps against FR-036 and TC-114"
type: SpecReview
analysis: gap-analysis
scope: "FR-036; TC-114; TC-115; TM-003 (spec/model-linking/tests.md); src/linking/composed/; tests/composed_{binding,definitions,definition_source,models,scopes}.rs; resources/native-v1/"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-003
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-114
    type: references
---

## Summary

Gap analysis of the composed static-binding slice on
`agent-a/composed-native-binding` (92313b7). The engine reports 332/340 matrix
rows backed with zero status lies, and the spec correctly keeps every FR-036 row
Planned. The binding stage genuinely implements the definition/model/scope
portions of TC-114, but neither TC-114 nor TC-115 is discharged: TC-115 has no
backing tagged test at all, IT-009 is untouched, and eleven producible typed
causes are unverified.

## Verdict

**FAIL** — TC-115 (FR-036-AC-5, FR-036-AC-6, FR-036-AC-8) is a matrix Test Case
with no backing tagged test, and FND-001/FND-004 are high. This is the expected
gate result for an intentionally partial slice of compiler #35, not a regression.

**Current verdict (re-verified at 9aa788a, 2026-09-10): FAIL, unchanged.**
The full-ticket gap is still open: TC-115 has no backing tagged test and IT-009
is untouched, so FND-001 stands and the skill's verdict rule mandates FAIL.
This is distinct from the correction status of FND-004 (scale/budget), which is
**resolved** at 9aa788a and re-verified by a gate-run test. FND-005 through
FND-008 are also resolved; FND-002, FND-003 and FND-009 remain as recorded.
Compiler #35/#40 remains open by the owner's explicit decision to deliver this
binding slice while TC-115, type/profile/runtime/family checking and D's IT-009
correspondence continue; nothing here claims those ACs.

**Current verdict (re-verified at 5381672, 2026-09-10): FAIL, unchanged.**
The full-ticket gap is exactly where it was: TC-115 has no backing tagged test,
IT-009 is untouched, and `spec/` is unchanged by this commit, so FND-001 stands
and the skill's verdict rule mandates FAIL. The SR-319 code corrections being
complete does not move this gate. One new gap is recorded as FND-011.

## Target selection

The `gap-analysis` skill's Step 1 (plan completion) **could not be executed**:
there is no plan bundle for compiler #35 or for the composed compiler at all.
`plan/` holds Plan-001 through Plan-009, none of which owns this work — the only
occurrence of "composed" in `plan/` is unrelated prose at
`plan/Plan-007-native-packages/tasks/Task-016-package-construction.md:66`, and
the one incomplete task in the tree (`Plan-008`, `Task-020`, `status: in_progress`,
"Qualify generated Boolean execution and deliver the PR") belongs to native
lowering. No substitute plan was adopted. Per the owner's direction, the owning
GitHub issue plus the reviewed FR-036 and TC-114 define the work, so Steps 2–3
were run against those instead and Step 1 is recorded as FND-002.

Step 4 (semantic review, intent↔test↔code) was **declined by the owner** and was
not run. Ordinary spec/code/test faithfulness was still checked and is reported
in SR-319.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-115 and FR-036-AC-5/6/8 have no backing tagged test; TC-114's ACs are not discharged because IT-009 is untouched | spec/model-linking/tests.md:107, spec/functional/FR-036-link-composed-native-packages.md:129, spec/integration/IT-009-composed-package-boundary.md |
| FND-002 | medium | No plan bundle exists for compiler #35, so the skill's plan-completion gate has no target | plan/ |
| FND-003 | medium | Five FR-036 ACs read as engine-"backed" while the matrix Status is Planned; backed measures tag binding, not AC discharge | spec/model-linking/tests.md:124, spec/model-linking/tests.md:130 |
| FND-004 | high | FR-036-AC-7 is tag-backed per stage, but nothing exercises the combined path's reference budget — the gap that let SR-319 FND-001 ship | tests/composed_binding.rs:208, tests/composed_definitions.rs:366, src/linking/composed/models.rs:990 |
| FND-005 | medium | Eleven producible typed causes under FR-036-AC-2 and AC-3 have no test, including the primary undefined-name diagnosis | src/linking/composed/scopes.rs:260, src/linking/composed/definitions.rs:84, src/linking/composed/models.rs:97 |
| FND-006 | medium | `resources/native-v1/` has no owning requirement, no co-located provenance/licensing record, and duplicates first-party files under `external/` | resources/native-v1/, src/linking/composed/definition_source.rs:4 |
| FND-007 | low | Six preserved requirement ids collide with local `spec/` ids; separation rests only on `quire` reading `<repo>/spec` | resources/native-v1/spec/functional/FR-036-retain-lexical-source-locations.md:2 |
| FND-008 | low | Three new submodules carry no `FR-` owning-requirement header, unlike their six siblings | src/linking/composed/scopes/values.rs:2, src/linking/composed/scopes/protocol.rs:2, src/linking/composed/scopes/protocol/flow.rs:2 |
| FND-009 | low | The coverage engine flags a copied `id` oracle helper shared by a new and an existing test module | tests/composed_scopes.rs:65, tests/composed_linking.rs:252 |
| FND-010 | low | Issue #40 artifact delivery has no surface anywhere in the tree and is neither started nor claimed here | - |
| FND-011 | medium | FR-036-AC-7 now has scale controls on References but none on Bindings, the dimension SR-319 FND-018 exhausts | tests/composed_binding.rs:143, tests/composed_models.rs:524, src/linking/composed/models.rs:870 |

## Detail

**FND-001.** `quire coverage` lists `spec/model-linking/tests.md:107` (TC-115 →
FR-036-AC-5, FR-036-AC-6, FR-036-AC-8) as unbacked, together with the three
matching `verification` rows in FR-036 itself (lines 129, 130, 132). No test in
the tree carries a `TC-115` tag. Separately, FR-036-AC-1, AC-3 and AC-4 name
"TC-114, IT-009" as their verification; IT-009 is Planned and no producer
integration exists, so even the tag-backed ACs are partially discharged at best.
The spec states this plainly (FR-036 Status; TC-114 "these cases do not discharge
all of TC-114 or IT-009"), so this is an honest open gap, not drift.

**FND-002.** Recorded as a limitation of this analysis rather than a defect. The
absence looks deliberate given `AGENTS.md` ("Keep necessary specification and
plan changes scoped; do not create a review or detailed log for every
increment"), but it does mean no artifact enumerates the remaining #35 tasks, so
"what is left" is derivable only from FR-036/TC-114 prose and the issue itself.

**FND-003.** The engine binds `FR-036-AC-1, AC-2, AC-3, AC-4, AC-7` through the
`#[trace]` tags on the 35 new tests and therefore counts them backed, while
`spec/model-linking/tests.md:124-131` keeps all eight rows `🚧 Planned` and the
prose says "Status records local runs, not engine-verified coverage". Zero
status lies were reported, so the two views are not in conflict — but `backed`
must not be promoted to verified coverage. A tagged test proves a tag resolves,
not that the criterion is met. This is an engine limitation to report, per the
review brief, and the spec's Planned status is the correct reading.

**FND-004.** FR-036-AC-7 requires that "an unaffordable next charge ... retains
unfinished dispositions" and that exact-limit expectations be "derivable from a
controlled input and traversal". Four tests cover it
(`composed_definitions.rs:366`, `composed_models.rs:504`,
`composed_scopes.rs:403`, `composed_binding.rs:208`), but each drives a single
stage or a three-declaration package. None asserts that a realistically sized
unit completes within the default budget. That omission is exactly how SR-319
FND-001 escaped: `resolve_declaration` rescans the whole expression arena once
per declaration, so ~50 declarations over a 40 000-node arena — legal under the
parser's 50 000-node ceiling — consumes the entire unraisable 2 000 000
reference budget on containment scans and reports `Unfinished` for valid input.
The missing control is a scale test, not a new stage.

**FND-005.** Untested producible causes: `definitions::Cause::{WrongEdition,
AmbiguousRule, MissingAlias}`; `models::ModelErrorKind::{MissingAlias,
AmbiguousExport, IncompleteCatalog}`; `models::ImportRefusal::AmbiguousSelection`;
`scopes::ScopeIssue::{MissingValue, ModelOperationUnavailable, DuplicateSymbol,
InvalidAwaitEvent, IncompatibleReference}`. FR-036-AC-2 requires "illegal
binder/capture scope ... produce located typed refusals", and the most basic of
those — a value name bound nowhere in the declaration — reaches
`ScopeIssue::MissingValue` (`values.rs:41-45`), which only `finish_names`
rewrites into the tested `OutOfScope`. Recorded here as an assurance gap rather
than a delivery blocker, consistent with `AGENTS.md` (2026-09-09) deferring
assurance completion to a later campaign.

**FND-006.** Reverse gap: 45 files and roughly 4 600 lines of preserved
normative content entered the tree with no local requirement owning them. FR-036
Status describes the registry, which is the closest thing to an owner. Beyond
traceability, the tree carries no `README`/`NOTICE`, `README.md` and
`LICENSE-DECISION.md` do not mention it, and the sole licensing notice sits in a
Rust doc comment — under a repo-wide AGPL-3.0-only `LICENSE`, which is the
inference `AGENTS.md` forbids. `resources/native-v1/external/quire-spec-language/`
is a byte-identical copy of this repo's own `src/diagnostic.rs` and
`docs/native-error-codes.md`, labelled with URLs into this same repository, with
no control detecting drift after the originals change. The needed control is a
co-located provenance record plus a drift check — not a checksum catalog, which
`CLAUDE.md` prohibits.

**FND-009.** The engine's single suspicion: the `id` helper in the new
`tests/composed_scopes.rs` is token-identical to the one in
`tests/composed_linking.rs`, flagged as `oracle-resembles-implementation`. Both
are two-line namespace lookups, so the practical risk is low, but it is the only
new-code suspicion and is recorded rather than dismissed silently.

**FND-010.** Nothing in the tree references issue #40. Consistent with the
brief: artifact delivery is a following part of the active goal and is not
claimed complete here. This review neither advances nor verifies it.

## Coverage

`quire coverage --scope /home/peter/dev/worktrees/quire-language-composed-binding --json`,
engine 0.46.0 (quire 0.31.0), run in this session.

| Measure | Value |
| --- | --- |
| Matrix rows backed | 332 / 340 (97.6%) |
| Unbacked rows | 6 — TC-115, FR-036-AC-5/6/8, plus pre-existing TC-010 and FR-017-AC-2 |
| Status lies | 0 |
| No-symbol rows | 2 — TC-010 (Manual) and FR-017-AC-2 (Inspection); both by design |
| Untracked test symbols | 20 — all in `src/package/encoding/tests.rs` and `tests/package_construction_cases/limits.rs`, all pre-existing |
| Unmatched tags | 3 — IT-004 in `tests/fixture_audit.rs`, pre-existing |
| Criteria property-extractable | 78 / 226 (34%) |
| Suspicions | 1 — FND-009 |

This change introduces no new untracked test symbol and no new unmatched tag:
all 35 new tests carry resolving `#[trace]` tags. The `DuplicateArchetype` and
`DuplicateInverseEdge` warnings on stderr are pre-existing module-manifest
conditions, not findings against this change.

Semantic review (Step 4) was skipped at the owner's request, so no
intent↔test↔code judgement is recorded here.

## Remaining work for compiler #35

Open in FR-036 and TC-114, and not claimed by this slice: expression and type
checking, profile/family admission, complete typed runtime-role derivation,
downstream request handling, D's canonical/native correspondence and
relationship export authority (explicit unsupported boundaries today), TC-115 in
full, and IT-009's real producer integration. Issue #40 artifact delivery is
untouched. Local gates are green and the historical package path is unaffected;
no result of this stage is a checked or executable package.

## Correction re-review (9aa788a, 2026-09-10)

`quire coverage --scope /home/peter/dev/worktrees/quire-language-binding-fixes
--json`, engine 0.46.0 (quire 0.31.0), re-run in this session against the
remediation commit. The rollup is byte-for-byte the same picture as at 92313b7.

| Measure | 92313b7 | 9aa788a |
| --- | --- | --- |
| Matrix rows backed | 332 / 340 | 332 / 340 |
| Unbacked rows | 6 | 6 — TC-115, FR-036-AC-5/6/8, plus pre-existing TC-010 and FR-017-AC-2 |
| Status lies | 0 | 0 |
| Untracked test symbols | 20 | 20 — all pre-existing, in `src/package/encoding/tests.rs` and `tests/package_construction_cases/limits.rs` |
| Unmatched tags | 3 | 3 — IT-004 in `tests/fixture_audit.rs`, pre-existing |
| Suspicions | 1 | 1 — the same `id` oracle helper (FND-009) |

The remediation adds 18 tests (401 → 419 with `--no-default-features`,
417 → 435 with `--all-features`; both suites pass, four pre-existing `#[ignore]`
lanes). Every new test carries a resolving `#[trace]` tag: the untracked-symbol
and unmatched-tag counts did not move.

### Disposition

| ID | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | open, unchanged | `quire coverage` still reports `spec/model-linking/tests.md:107` (TC-115 → FR-036-AC-5/6/8) and the three matching `verification` rows at `spec/functional/FR-036-link-composed-native-packages.md:129,130,132` as unbacked. No test carries a `TC-115` tag; `spec/integration/IT-009-composed-package-boundary.md` is untouched and no producer integration exists. `spec/` is unchanged by 9aa788a, so the matrix still records these rows Planned — an honest open gap, and the owner has explicitly permitted delivering the binding slice while it stands. Not softened, not claimed. |
| FND-002 | open as an analysis limitation, not a defect | Still no plan bundle for compiler #35; the owner has confirmed no new Plan bundle is required to reformat the existing issue/FR scope, so Step 1 stays unexecutable and FR-036 plus TC-114 remain the target of record. |
| FND-003 | open, unchanged | The engine still counts FR-036-AC-1/2/3/4/7 backed through `#[trace]` tags while `spec/model-linking/tests.md:124-131` keeps all eight rows Planned, with zero status lies. `backed` measures tag binding, not AC discharge; the spec's Planned status remains the correct reading. |
| FND-004 | **resolved** | The missing control was a scale test, and it now exists. `tests/composed_binding.rs:93-135` binds 100 declarations over a 39 900-node expression arena through the full `binding::bind` path, asserts `report.complete()` and `Disposition::NamesResolved` for every declaration, and caps `usage().references` below 200 000 — where the per-declaration whole-arena rescan needed 3 990 000 against an unraisable ceiling of 2 000 000. `tests/composed_binding.rs:139-187` does the same for the definition stage over 200 protocols and 10 200 controls (2 040 000 under the old scan). Both were run green in this session. This closes the FR-036-AC-7 hole that let SR-319 FND-001 ship: the combined path's reference budget is now exercised against a realistically sized unit. |
| FND-005 | **resolved** | All eleven producible typed causes now have tagged tests — see SR-319's disposition table for the per-cause references. The `ScopeIssue::MissingValue` case specifically uses a name bound nowhere in its declaration, so it fails if `finish_names`' rewrite to `OutOfScope` is made unconditional. |
| FND-006 | **resolved** | The reverse gap is closed at the requirement level and the provenance level: `resources/native-v1/README.md` names FR-036 as the owner of the resources' use, records the originating standard PR, states that the standard's document licence remains deferred and that this snapshot adds none, and defines `external/` as external to the *standard* repository while retaining the copied Rust file's own AGPL notice. `README.md:18` and `LICENSE-DECISION.md:30` link it. The drift half is answered by the product boundary rather than by a new control: these are deliberately selected historical rule bytes that must not be synchronized to later compiler diagnostics, and `tests/composed_definition_source.rs:270-278` compares the registry's embedded bytes against the retained resource files (`resource()` at `:128-135` reads `resources/native-v1/`), never against `src/diagnostic.rs`. Exact registered-byte admission plus that comparison satisfies the boundary; no SHA inventory or checksum catalog was added, per `CLAUDE.md`. |
| FND-007 | **resolved by documentation** | `resources/native-v1/README.md:4-6` states that standard-relative paths and document ids keep the standard repository's meaning and that compiler requirement tooling scans the local `spec/` tree separately. |
| FND-008 | **resolved** | `src/linking/composed/scopes/values.rs:2`, `scopes/protocol.rs:2` and `scopes/protocol/flow.rs:2` now carry `FR-036:` owning-requirement headers, as does the new `arena.rs:2`. |
| FND-009 | open, unchanged | The engine still flags the `id` helper shared by `tests/composed_scopes.rs:66` and `tests/composed_linking.rs:66` (reported against its use at `tests/composed_linking.rs:252`) at token similarity 1.00. Unremediated and still low: both are two-line namespace lookups. |
| FND-010 | open, unchanged | Nothing in the tree references issue #40; artifact delivery is neither started nor claimed here, and this re-review neither advances nor verifies it. |

### Remaining work for compiler #35

Unchanged from the original review, and explicitly permitted to remain open:
expression and type checking, profile/family admission, complete typed
runtime-role derivation, downstream request handling, D's canonical/native
correspondence, relationship export authority (an explicit unsupported boundary
today), TC-115 in full, and IT-009's real producer integration. Issue #40
artifact delivery is untouched. Local gates are green and the historical package
path is unaffected; no result of this stage is a checked or executable package.
The optional semantic review (Step 4) remains declined and was not run here.

## Correction re-review (5381672, 2026-09-10)

`quire coverage --scope /home/peter/dev/worktrees/quire-language-binding-fixes
--json`, engine 0.46.0 (quire 0.31.0), re-run in this session against the second
remediation commit. The rollup is unchanged from both earlier runs.

| Measure | 92313b7 | 9aa788a | 5381672 |
| --- | --- | --- | --- |
| Matrix rows backed | 332 / 340 | 332 / 340 | 332 / 340 |
| Unbacked rows | 6 | 6 | 6 — TC-115, FR-036-AC-5/6/8, plus pre-existing TC-010 and FR-017-AC-2 |
| Status lies | 0 | 0 | 0 |
| Untracked test symbols | 20 | 20 | 20 — all pre-existing, in `src/package/encoding/tests.rs` and `tests/package_construction_cases/limits.rs` |
| Unmatched tags | 3 | 3 | 3 — IT-004 in `tests/fixture_audit.rs`, pre-existing |
| Suspicions | 1 | 1 | 1 — the same `id` oracle helper (FND-009) |

The commit adds three tests (419 → 422 with `--no-default-features`,
435 → 438 with `--all-features`; both suites pass with 0 failed and the same
four pre-existing `#[ignore]` lanes, all gates executed in this session). Two
are `#[cfg(test)]` unit tests inside `src/linking/composed/arena.rs` and one is
an integration test; all three carry resolving `#[trace]` tags, so neither the
untracked-symbol nor the unmatched-tag count moved.

### Disposition at 5381672

| ID | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | open, unchanged | `quire coverage` still reports `spec/model-linking/tests.md:107` (TC-115 → FR-036-AC-5/6/8) and the three `verification` rows at `spec/functional/FR-036-link-composed-native-packages.md:129,130,132` as unbacked. No test carries a `TC-115` tag; `spec/integration/IT-009-composed-package-boundary.md` is untouched and no producer integration exists. `spec/` is unchanged by 5381672. The completed SR-319 corrections do not touch this gap, and it is not softened here. |
| FND-002 | closed as an analysis limitation | The owner has dispositioned the absent #35 plan bundle; creating one is not a task. Step 1 stays unexecutable and FR-036 plus TC-114 remain the target of record. |
| FND-003 | open, unchanged | The engine still counts FR-036-AC-1/2/3/4/7 backed through `#[trace]` tags while `spec/model-linking/tests.md:124-131` keeps all eight rows Planned, with zero status lies. `backed` measures tag binding, not AC discharge. |
| FND-004 | resolved, and strengthened | Beyond the two arena-scale tests at 9aa788a, `tests/composed_binding.rs:143-245` now exercises the combined path against a **real admitted** model — 1 500 records through `model_source::read` + `admit(ModelLimits::default())` — with 400 declarations of four typed parameters each, asserting per-occurrence resolution as well as the reference cap. FR-036-AC-7's "exact-limit expectations derivable from a controlled input" is exercised at scale on References. |
| FND-005 | resolved, unchanged | The eleven producible typed causes retain their tagged tests from 9aa788a. |
| FND-006, FND-007, FND-008 | resolved, unchanged | `resources/` and the module headers are untouched by this commit. |
| FND-009 | open, unchanged | The engine still flags the `id` helper shared by `tests/composed_scopes.rs` and `tests/composed_linking.rs:252` at token similarity 1.00. Unremediated and still low. |
| FND-010 | open, unchanged | Nothing in the tree references issue #40; artifact delivery is neither started nor claimed here. |

### New finding

**FND-011.** The FND-004 remedy covers one dimension. Three tests now pin
References at scale (`tests/composed_binding.rs:97`, `:143`, `:249`), and
`tests/composed_models.rs:524-570` pins exact Bindings for a **tiny** fixture,
but nothing exercises Bindings against a realistically sized supply. That is
exactly the hole SR-319 FND-018 falls through: the per-model index reservation
at `src/linking/composed/models.rs:870-900` scales with the supplied model set,
and 59 models of the size the new test itself admits exhaust the unraisable
262 144 Bindings ceiling with no test to notice. FR-036-AC-7 does not privilege
one dimension — "zero never disables a limit" and exact-limit expectations apply
across the accounting contract. The missing control is again a scale test, not
a new stage: bind a multi-model inventory and assert the Bindings budget the way
`large_admitted_model_resolves_repeated_nominal_parameters_at_defaults` asserts
the References budget.

### Remaining work for compiler #35 at 5381672

Unchanged, and explicitly permitted to remain open by the owner: expression and
type checking, profile/family admission, complete typed runtime-role derivation,
downstream request handling, D's canonical/native correspondence, relationship
export authority (an explicit unsupported boundary today), TC-115 in full, and
IT-009's real producer integration. Issue #40 artifact delivery is untouched.
Local gates are green and the historical package path is unaffected; no result
of this stage is a checked or executable package. The optional semantic review
(Step 4) remains declined and was not run here.

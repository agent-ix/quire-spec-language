---
id: SR-324
title: "Rational model profile delivery gaps against FR-041, FR-040 and TM-003"
type: SpecReview
analysis: gap-analysis
scope: "FR-041; FR-040; TC-120; TC-119; TM-003 (spec/model-linking/tests.md); spec/spec.md; src/native_model/; src/model_source/; src/checking/{types,variables}.rs; src/linking/native.rs; tests/native_model_profiles.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-041
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-003
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-120
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: references
---

## Summary

Gap analysis of the rational native model producer slice on
`agent-a/composed-value-checking` (40f6b43). The delivered FR-041 scope is
genuinely covered — all seven ACs and TC-120 are engine-backed by fourteen
tagged, passing public tests, with zero status lies. The FAIL is the tracked
half: TC-119 and all ten FR-040 ACs have no backing tagged test, which is the
expected result for work the branch deliberately left PLANNED.

## Verdict

**FAIL** — by the skill's own rule: TC-119 is a matrix Test Case with no backing
tagged test (FND-001). This is tracked later feature work under compiler
[#35](https://github.com/agent-ix/quire-spec-language/issues/35)/[#36](https://github.com/agent-ix/quire-spec-language/issues/36),
not a regression and not a defect of this delivery. Nothing here disputes the
FR-041 producer slice, which is complete against its own ACs.

## Target selection

The skill's Step 1 (plan completion) **could not be executed**: no plan bundle
owns this work. `plan/` holds Plan-001 through Plan-009, all of which predate
the composed compiler; none mentions FR-040, FR-041, rational scalars or model
profiles. Per the owner's direction no bundle is required and none was created
to satisfy the step. Steps 2–3 were run against FR-041, FR-040, TC-120, TC-119
and TM-003 instead, and Step 1 is recorded as FND-004.

Step 4 (semantic review, intent↔test↔code) was **declined by the owner** and was
not run. Ordinary spec/code/test faithfulness was still checked and is reported
in SR-323. The QUOIN base + failure-domain spec review of the composed-values
requirements is being performed separately under `spec/reviews/composed-values`
and is not duplicated here.

Coverage was reconciled with
`quire coverage --scope /home/peter/dev/worktrees/quire-language-composed-checking --json`
(quire 0.31.0, engine ca7362d4).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-119 and all ten FR-040 ACs have no backing tagged test; the composed value/definedness checker does not exist | spec/model-linking/tests.md:108, spec/functional/FR-040-check-composed-values.md:139, spec/functional/FR-040-check-composed-values.md:148 |
| FND-002 | medium | FR-041 is fully backed and passing, but four spec documents still describe TC-120 as planned and one states a now-false fact | spec/test-cases/TC-120-admit-rational-native-model-profile.md:78, spec/model-linking/tests.md:109, spec/functional/FR-041-admit-rational-native-model-profile.md:138, spec/spec.md:296 |
| FND-003 | medium | Rational typing was admitted into the shared historical-checker catalog with no owning AC; FR-041-AC-5 owns only the composed export path | src/checking/types.rs:305, spec/functional/FR-041-admit-rational-native-model-profile.md:43 |
| FND-004 | medium | No plan bundle exists for FR-040/FR-041, so the skill's plan-completion gate has no target | plan/ |
| FND-005 | low | `src/checking/variables.rs` is a new module with no owning requirement id, unlike all five siblings | src/checking/variables.rs:2, src/checking/types.rs:2 |
| FND-006 | low | `NativeModelProfile::try_from` and `UnknownNativeModelProfile` are public surface with no production caller and no owning AC | src/native_model.rs:37, tests/native_model_profiles.rs:149 |
| FND-007 | low | Pre-existing unbacked rows and unmatched tags carried into this branch unchanged | spec/model-linking/tests.md:107, tests/fixture_audit.rs:231, src/package/encoding/tests.rs:67 |

## Detail

**FND-001.** `quire coverage` reports TC-119 and FR-040-AC-1 through
FR-040-AC-10 as unbacked rows — eleven of the seventeen unbacked rows in the
repo. No test carries a `TC-119` or `FR-040-AC-*` tag anywhere in the tree. This
is accurate and declared: the branch states FR-040/TC-119 is intentionally
PLANNED, the matrix marks every FR-040 row `🚧 Planned`
(`spec/model-linking/tests.md:141-150`), and FR-041 scopes itself to producer
admission only. What is *not* delivered, stated concretely so the remaining work
is legible: no composed expression typing, no guarded definedness or proof
discharge over composed units, no rational arithmetic or ordering rules, no
runtime rational execution, no composed `CheckedPackage` and no composed
artifact. FR-040-AC-4's "rational normalization-before-bounds" has a real
upstream capability to build on — the pinned IR normalizes at the literal
boundary (`quire-contract-ir` 690bde7 `src/expression.rs:1682-1692`) and
implements rational arithmetic ranges — so the dependency is available and the
gap is entirely on this crate's side. Issues #35/#36/#39/#40 remain open; this
slice is their model producer prerequisite, not their completion.

**FND-002.** The delivered half is measurably done: FR-041-AC-1 through AC-7 and
TC-120 all report `backed: true`, fourteen tagged tests in
`tests/native_model_profiles.rs` exercise them through the public API, and both
test configurations pass (433/449, 4 pre-existing ignored lanes). Four documents
have not caught up:

- `spec/model-linking/tests.md:109` lists TC-120 as `🚧 Planned`, and the L2
  narrative at `:113-125` explains why TC-114/TC-115 remain Planned but says
  nothing about TC-119 or TC-120.
- `spec/model-linking/tests.md:151-157` marks all seven FR-041 AC rows
  `🚧 Planned`.
- `spec/functional/FR-041-...:138` still reads "TC-120 ... is planned".
- `spec/spec.md:296` reads "implementation verification pending".
- `spec/test-cases/TC-120-...:78` states "All controls remain planned until
  implemented and run." They are implemented and they ran; that sentence is now
  false, and TC-120's Description at `:13` likewise says "Planned public Rust
  controls".

This report does **not** assert those rows should read `✅ Passed` — engine
"backed" measures tag binding, not AC discharge, and the repo's convention
reserves `✅ Passed` for the assurance campaign. The defect is narrower and
objective: a factual claim in TC-120 is contradicted by the tree, and a reader
cannot currently distinguish "not built" (TC-119) from "built, tests passing,
not yet blessed" (TC-120), because the matrix spells both the same way with no
prose separating them. One corrected sentence in TC-120 and one paragraph in the
L2 narrative resolve it.

**FND-003.** Reverse gap (code with no owning requirement).
`src/checking/types.rs:303-310` removes the `ir::ValueType::Rational { .. } =>
None` refusal so a rational becomes `NativeType::Scalar`. FR-041-AC-5 owns this
for composed binding ("Composed `bind_models` accepts actual admitted `/2`
exports"), and `Catalog` is reached from there
(`src/linking/composed/models.rs:904`). But the same `Catalog` is built by the
historical checker (`src/checking.rs:326`), and for that consumer the change has
no owning AC at all: FR-041 explicitly disclaims rational expression typing
(`FR-041:43-45`), and FR-040 — which would own it — is unimplemented. The
behaviour is unreachable today only because `link_native` refuses `/2`
(`src/linking/native.rs:19`), so nothing verifies it and nothing specifies it.
SR-323 FND-001 records the runtime consequence (ordering type-checks; the proof
materializes the operand as Boolean). Either the refusal returns for the
historical consumer, or an FR-040 AC has to own it before TC-119 is written —
otherwise the first composed-checking test to touch a rational will be writing
the requirement backwards from the code.

**FND-004.** Same condition as SR-320 FND-002 and unchanged by this branch. The
only in-progress task in `plan/` (`Plan-008`, `Task-020`) belongs to native
lowering. Recorded so the skipped step is visible, not as a request to create a
bundle.

**FND-005.** `src/checking/variables.rs:2` reads "Shared contextual native-type
variables; callers own syntax and work limits." Every sibling in the module
cites its owner — `bindings.rs:2`, `constraints.rs:2`, `proof.rs:2`,
`types.rs:2` all say `FR-016`, and `inputs.rs:2` says `FR-016-AC-9`. The
extraction is the correct preparation for FR-040 and the algorithm is unchanged,
but as written the file is the one place in `src/checking/` a traceability sweep
cannot attribute. Same omission pattern as SR-319 FND-014.

**FND-006.** `NativeModelProfile::try_from` and `UnknownNativeModelProfile`
(`src/native_model.rs:32-46`) are `pub` with no caller in `src/`. FR-041 requires
that "Unknown profile/format selections refuse", and the *format* half of that is
owned and exercised through `model_source::read`
(`src/model_source.rs:256-260`, tested at
`tests/native_model_profiles.rs:155-164`). The profile half has only the direct
constructor assertion at `tests/native_model_profiles.rs:149-154` and no
production consumer, so it is API surface ahead of a requirement. SR-323 FND-008
records the idiom half of this (`TryFrom` where the crate uses `FromStr`).

**FND-007.** Carried through unchanged and listed so they are not attributed to
this branch: TC-115 / FR-036-AC-5,6,8 unbacked (SR-320 FND-001); TC-010 and
FR-017-AC-2 unbacked with `Manual`/`Inspection` test types, correctly counted as
`no_symbol_rows`; twenty `NFR-007-M-*` untracked symbols in
`src/package/encoding/tests.rs` and `tests/package_construction_cases/limits.rs`;
three `IT-004` unmatched tags in `tests/fixture_audit.rs`; the single
`oracle-resembles-implementation` suspicion in `tests/composed_linking.rs:252`;
and the four `#[ignore]`d named lanes. None touches FR-040 or FR-041.

## Coverage

| Measure | Value |
| --- | --- |
| Matrix rows backed (repo-wide) | 340 / 359 |
| Status lies | 0 |
| Unbacked rows | 17 — 11 of them FR-040/TC-119 |
| `no_symbol_rows` (Manual/Inspection) | 2 |
| FR-041 acceptance criteria backed | 7 / 7 |
| TC-120 backed | yes |
| FR-040 acceptance criteria backed | 0 / 10 |
| TC-119 backed | no |
| New tagged tests added | 14, all in `tests/native_model_profiles.rs` |
| Untracked symbols introduced by this branch | 0 |
| Test suite | 433 passed (`--no-default-features`), 449 passed (`--all-features`), 0 failed, 4 pre-existing ignored lanes |

Semantic review (Step 4) skipped at the owner's direction; the backed/unbacked
rollup above is tag reconciliation, not a judgement that each test validates its
criterion's intent.

## Separating real defects from tracked later work

**Real gaps in delivered scope** — act on these before merge or shortly after:
FND-003 (unowned, unverified rational admission on the historical checker path,
with SR-323 FND-001), FND-002 (a false statement in TC-120 and a matrix that
cannot distinguish unbuilt from unblessed), FND-005 and FND-006 (small
traceability and surface debt).

**Tracked later feature/assurance work** — not defects of this branch:
FND-001 (FR-040/TC-119 — the composed value and definedness checker itself),
FND-004 (no plan bundle, by owner direction), FND-007 (pre-existing rows,
lanes and suspicions), and the assurance campaign that would move any
`🚧 Planned` row to `✅ Passed`. `AGENTS.md` (2026-09-09) defers assurance
completion explicitly, so none of these blocks engineering delivery of the
producer prerequisite.

## Remaining work before FR-040 can be discharged

1. Decide the owner of rational typing on the historical checker path — restore
   the refusal or write the FR-040 AC (FND-003).
2. Author TC-119 against the ten FR-040 ACs, including the rational
   normalization-before-bounds, exact-division and nonzero-guard contract that
   the pinned IR already supports.
3. Correct TC-120's Expected Results and give TC-119/TC-120 a narrative in the
   L2 section of TM-003 (FND-002).
4. Add the owning `FR-` id to `src/checking/variables.rs` before it acquires its
   second consumer (FND-005).

## Correction disposition (a4344d0)

Re-reconciled at a4344d0 with
`quire coverage --scope /home/peter/dev/worktrees/quire-language-composed-checking --json`
(quire 0.31.0, engine ca7362d4). Delivered scope remains the FR-041/TC-120
rational model producer and historical isolation; FR-040/TC-119 is subsequent-branch work.

| ID | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | unchanged — tracked later work | TC-119 and FR-040-AC-1..10 are still 11 of the 17 unbacked rows; no FR-040 code on this branch |
| FND-002 | fixed | TC-120:13 and :78 now state implemented, locally passing controls; FR-041:138; spec.md:296; new L2 paragraph at `spec/model-linking/tests.md:125-129` separates unbuilt (TC-119) from built-but-unblessed (TC-120). Matrix rows correctly stay `🚧 Planned` |
| FND-003 | fixed | historical catalog refusal restored (SR-323 FND-001); the admitted-rational path is again owned solely by FR-041-AC-5 through composed binding |
| FND-004 | unchanged — no bundle required by owner direction | plan/ |
| FND-005 | fixed | `src/checking/variables.rs:2` now cites FR-016/040 |
| FND-006 | residual low | `FromStr`/`TryFrom` still have no production caller; the correction added a second parse path rather than retiring surface (SR-323 FND-011) |
| FND-007 | unchanged — pre-existing | same three IT-004 unmatched tags, 20 untracked symbols, one suspicion |

| Measure (a4344d0) | Value |
| --- | --- |
| Matrix rows backed | 340 / 359 (unchanged) |
| Status lies | 0 |
| Unbacked rows | 17 — 11 FR-040/TC-119 |
| FR-041 ACs / TC-120 backed | 7 / 7; yes |
| Untracked symbols or unmatched tags added | 0 |
| Test suite | 440 (`--no-default-features`) / 456 (`--all-features`) passed, 0 failed, 4 pre-existing ignored lanes |

**Verdict: FAIL stands for the full ticket.** TC-119 is still a matrix Test Case
with no backing tagged test (FND-001), which is tracked FR-040 work on later
branches, not a defect of this producer PR. **Delivered FR-041/TC-120 producer
scope: PASS** — seven of seven ACs and TC-120 engine-backed, zero status lies,
and every delivered-scope gap (FND-002, FND-003, FND-005) closed, with FND-006
residual low. Nothing above converts the ticket-level FAIL into a PASS.

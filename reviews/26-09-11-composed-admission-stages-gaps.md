---
id: SR-376
title: "Gap analysis of TC-115 against FR-036-AC-5, AC-6 and AC-8"
type: SpecReview
analysis: gap-analysis
scope: "spec/functional/FR-036-link-composed-native-packages.md; spec/model-linking/tests.md; spec/test-cases/TC-115-preserve-composed-admission-stages.md; src/linking/composed/subject.rs; src/linking/composed/requests.rs; tests/composed_admission_stages.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-115
    type: references
---

## Summary

QUOIN `gap-analysis` over the TC-115 increment, with a fresh
`quire coverage --scope . --json` (quire 0.31.0, engine 0.46.0). TC-115 and its
three acceptance criteria are now backed by eleven tagged Rust controls in
`tests/composed_admission_stages.rs`. FR-036 reads 8/8 backed and
`spec/model-linking/tests.md` reads 42/42. The long-standing four-row FND-001
carried by roughly a dozen prior reviews (TC-115 with FR-036-AC-5/AC-6/AC-8) is
closed by this increment; the only unbacked rows left in the corpus are the two
method-exempt ones. The optional semantic review (step 4) was not run, matching
the owner's standing disposition on this repository.

## Verdict

**CONDITIONAL** — no matrix Test Case lacks a backing tagged test and no `high`
finding stands. The remaining findings are inherited corpus debt outside this
scope: twenty untracked `NFR-007-M-*` symbols and three `IT-004` tags matching
no matrix target, both unchanged in count by this increment.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | Prior FND-001 (TC-115, FR-036-AC-5/AC-6/AC-8 unbacked) is RESOLVED: `unbacked_rows` falls from six to two, and the two survivors are the declared method exemptions `TC-010` (Manual) and `FR-017-AC-2` (Inspection), which also appear in `no_symbol_rows` | spec/model-linking/tests.md:112 | correct-requirement-no-evidence |
| FND-002 | medium | Twenty untracked `NFR-007-M-*` evidence symbols remain; inherited, unchanged by this increment, and outside FR-036's scope | src/package/encoding/tests.rs:67 | correct-requirement-no-evidence |
| FND-003 | medium | Three `IT-004` tags match no matrix target; inherited, unchanged, and outside FR-036's scope | tests/fixture_audit.rs:231 | correct-requirement-no-evidence |
| FND-004 | low | No plan bundle owns FR-036 or TC-115, so this skill's step-1 artifact does not exist for this scope; the review targets the requirement set directly. `plan/` holds Plan-001..Plan-009, none naming FR-036 | plan/ | missing-requirement |

## Step 1 — plan completion

No plan bundle under `plan/` targets FR-036 or TC-115 (`grep -rln 'FR-036\|TC-115' plan/`
returns nothing across Plan-001 through Plan-009). Step 1 is therefore not
applicable and is recorded as FND-004 rather than silently passed.

## Step 2 — matrix verification

`quire coverage --scope . --json`:

| Metric | Value |
| --- | --- |
| `totals.backed` / `totals.total` | 378 / 383 |
| `unbacked_rows` | 2 — `TC-010` (spec/tests.md:45), `FR-017-AC-2` (spec/functional/FR-017-separate-qualification-stages.md:65) |
| `no_symbol_rows` | 2 — the same two, exempt by declared method (`Manual`, `Inspection`) |
| `status_lies` | 0 |
| `untracked_symbols` | 20 |
| `unmatched_tags` | 3 |
| FR-036 acceptance criteria | 8 / 8 backed |
| `spec/model-linking/tests.md` | 42 / 42 |
| `authoring.tag_rate` | 630 / 630 |

The matrix rows moved from `Planned` to `Passed` only for the three criteria
whose tests exist and run: TC-115, FR-036-AC-5, FR-036-AC-6, FR-036-AC-8.
TC-114's rows and its FR-036-AC-1/2/3/4/7 entries remain `Planned`, and IT-009
remains open; nothing in this increment claims them.

## Step 3 — underspecified code

Both new modules trace to behavior already declared in FR-036 rather than to new
requirements:

| Code | Owning requirement text |
| --- | --- |
| `linking::composed::subject::StaticSubject` | "Changing assessment inputs or selected backend does not change static meaning. A changed static definition, type owner or binding contract requires an explicit new selection." |
| `linking::composed::subject::BuildProvenance` | "Preserve full compile configuration provenance separately from static meaning, including resource-control changes." |
| `linking::composed::requests` | "The compiler SHALL retain each requested clause/capability pair when handing a linked subject to downstream processing." and "If a downstream checker or backend cannot admit a requested form, then the compiler SHALL retain its typed unsupported disposition without converting the package to complete success." |

The component set retained by `StaticSubject` is the package contract's own
declared list (Sources, Language, Profiles, Models, Native dependencies, Binding
requirements); no component was invented and no canonical digest, structural hash
or JSON fingerprint was introduced. No stub, `todo!`, placeholder return or
re-export-only module was added.

## Step 4 — semantic review

Not run. The intent↔test↔code pass is optional and gated on an explicit choice;
this repository's standing disposition is that it remains declined. Recorded here
rather than implied.

## Coverage

Gates run locally, serially, one Cargo job, in this worktree's own `./target`:
`fmt --check` exit 0; both strict Clippy configurations exit 0; minimal test lane
616 passed / 0 failed / 4 ignored; `quire-extraction` lane 632 passed / 0 failed
/ 4 ignored. The four ignored tests are pre-existing.

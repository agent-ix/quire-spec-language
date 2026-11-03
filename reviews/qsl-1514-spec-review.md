---
id: SR-2223001
title: "spec-review of FR-502/FR-504 progress rulings and TC-144 row 5 (quire-spec-language PR 2223)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-502-reject-a-regressing-watermark-and-a-conflicting-completeness-revision-on-the-spine.md; spec/functional/FR-504-carry-progress-on-event-position-and-fixed-sample-traces.md; spec/test-cases/TC-1296-reject-a-regressing-watermark-and-a-conflicting-completeness-revision.md; spec/test-cases/TC-1299-carry-progress-on-event-position-and-fixed-sample-traces.md; spec/test-cases/TC-144-checked-package-temporal-evaluation-owner.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-502
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-504
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: references
---

# Spec review of QSL-1514 progress rulings (quire-spec-language PR 2223)

## Summary

Ticket: QSL-1514. One spec-review pass over `git diff origin/main...HEAD` at head
83d236e91f15f50af1a8581635e2439dd37d95bf (FR-502 binding per occurrence and AC-12, FR-504 lower_boundary
placement, silent-sample anchoring and AC-9, TC-1296 row 16, TC-1299 row 18, TC-144 row 5). Sub-analyses
(EARS, integrity, dependency, criterion strength) were applied by hand over the diff; the Jev-backed
criterion-strength client was not used, so AC strength is a manual judgement.

Checked against main (54b387265): ruling 2 matches `TraceInput::Progressed { trace, progress, lower_boundary,
binding }` in qsl-replay/src/spine/clause/temporal.rs and `ProgressAssertion` (no boundary) in progress.rs.
Ruling 3 matches qsl-eval finite_window run.rs (`Activation::Each` iterates `trace.observed`, silent samples
excluded). Ruling 1 matches FR-501 "Canonical order" and FR-501-AC-19; trace A is the canonical order of
stamps 0,5,2 with level 0,2,1, and FR-501-AC-16/19 make the result identity order independent. Ruling 4 is not
implemented (LedgerBinding.clause is a NodeKey; expected, QSL-1503). QSpec: TemporalBinding.occurrence and
temporal_clause_names exist in checked-package-v2 schema; FR-094 says only "identified binding", so per
occurrence keys do not conflict; FR-088-AC-3 makes the ordinal source-position based, so the AC-12 rename claim
holds. FR-502-AC-12 arithmetic is sound: Fast 100; Slow 60 accepted with its own entry; Fast 60 refused Watermark
retained 100 (refusal leaves the ledger unchanged); Slow still 60. FR-504-AC-9 can fail (an anchoring evaluator
returns Completed(false)); both its cases are correct by hand. TC-144 and TC-1299 descriptions match row counts
and AC ranges (TC-1299 AC-1..9 over 18 rows, TC-1296 AC-1..12 over 16 rows). No new ids: AC-12, AC-9 and rows are
additions inside existing artifacts; none of the open PRs 2197, 2221, 2222 touch these files. `quire validate`
over the five files and over spec/**/*.md exits 0 (module duplicate-archetype warnings only).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-330 Inputs still gives `TraceInput::Progressed{trace, progress}` and says progress carries the lower boundary; FR-504 now cites FR-330 for the four-member shape, so the two requirements disagree on the signature | spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:53 | wrong-requirement |
| FND-002 | medium | FR-502-AC-12 (and TC-1296 row 16) do not pin the same package, profile and clock binding name for `Fast` and `Slow`; the key includes the clock name, which FR-337 holds per occurrence, so a reader giving them different clock names passes the AC under the old node-keyed ledger | spec/functional/FR-502-reject-a-regressing-watermark-and-a-conflicting-completeness-revision-on-the-spine.md:163 | correct-requirement-no-evidence |
| FND-003 | low | FR-504 says a `Timestamped` trace carries its lower boundary "in its window (FR-501)" and that the progress assertion is shared with Timestamped, but FR-501 Inputs lists the lower-boundary kind as part of the trace's progress and FR-330 lists `lower_boundary` as a top-level Timestamped member; the rationale misdescribes both | spec/functional/FR-504-carry-progress-on-event-position-and-fixed-sample-traces.md:52 | wrong-requirement |
| FND-004 | low | TC-144 row 5 says the Timestamped trace is "listed in stamp order 0, 5, 2", which is not stamp order (and is the out-of-order listing under test) | spec/test-cases/TC-144-checked-package-temporal-evaluation-owner.md:50 | wrong-requirement |

## Verdict

No high findings. All four rulings keep behavior and agree with main where main implements them. Fix FND-001 and
FND-002 in one round (both one-line edits); FND-003 and FND-004 are wording.

---
id: SR-810
title: "Base checklist review of QSL-323: FR-059-AC-6 and TC-156 step 6"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@4c51991de72f82db507d584499f87a5fff7ae0ac; spec/functional/FR-059-check-backend-dependency-direction.md, spec/test-cases/TC-156-check-backend-dependency-direction.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-059
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-156
    type: reviews
---
# SR-810: Base checklist review of QSL-323: FR-059-AC-6 and TC-156 step 6

## Summary

Ticket: QSL-323. PR agent-ix/quire-spec-language#534.

Checked ID formats, AC and TC quality and coverage for the changed FR-059-AC-6
and TC-156 Description and step 6. `quire validate` on the four changed files
exits 0. AC-6 and step 6 agree with each other and with ADR-011 OBS-029. AC-6
is testable: it fails if IR adds a QSL dependency. TC-156 still verifies
FR-059 and covers AC-1 to AC-7. No pins, SHAs or tracking were added; the PR
removes two inherited SHAs from touched rows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC-6 and step 6 now assert only absence on real data. The real-data run used to double as a positive control: it proved the classifier recognised a real QSL dependency in real `cargo metadata`. Step 6's "step 2's seeded IR → QSL edge is still reported" is a synthetic graph, not real metadata. A regression that stops classifying real git-sourced QSL crates (for example `qsl-*` package ids) would pass AC-6 silently. Fix: have step 6 also expect the real QSL `qsl-package` → `quire-contract-model` edge, classified QSL → IR, in the same run, so the real-data run shows at least one classified edge. | FR-059:97, TC-156:55-61 |

## Verdict

Approve with one low finding. The AC and test case are consistent and
testable.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The amended FR-059-AC-6 says "the same run's resolved edge set contains QSL's normal edge on the git-sourced `quire-contract-model` … a run whose edge set lacks it fails", and TC-156 step 6 fails the step on an edge set without it. `arch-lint direction` does neither: `run_direction` prints only the revisions and the FB-05/FB-11 violations, never the resolved edge set, and passes whenever both reports are empty (`tools/arch-lint/main.rs:152-180`). So the operator running step 6 cannot see whether the QSL → IR edge was classified, and a classifier regression still prints two PASS lines. FR-059's Status says the check is implemented, so the AC now claims behaviour the code lacks. Fix: either add the behaviour (print the classified edge set, or fail when `--qsl`'s resolved edges contain no QSL → IR edge) with a test, or reword AC-6 and step 6 to a check the current output supports, such as a unit test over `metadata::edges_for_manifest` on QSL's real `Cargo.toml` asserting the QSL → IR edge. | FR-059:97, TC-156:59-67, tools/arch-lint/main.rs:152 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 17b7269c |
| FND-002 | still-open | New this round: AC-6 and TC-156 step 6 require the run to show or fail on the QSL → IR edge; arch-lint direction prints only violations and passes when both reports are empty. |
| FND-002 | fixed | 5ce6f07e |

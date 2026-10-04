---
id: SR-1297
title: "Spec review of quire-spec-language PR #630: FR-264 depth removal and the 2^53 vectors"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@e28bc06358a790a8e27a74b6208956ede2ddfb19; spec/functional/FR-264-*.md, spec/test-cases/TC-739-*.md, spec/test-cases/TC-090-*.md, spec/native-packages/tests.md, spec/tests.md; context: spec/functional/FR-014-*.md, spec/test-cases/TC-035-*.md, spec/test-cases/TC-038-*.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-264
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: reviews
---
# Spec review of quire-spec-language PR #630

## Summary

Ticket: QSL-488. PR: quire-spec-language#630.

Checked and clean:
- FR-264 Behavior 2 now says "no depth limit or setting". AC-5 is testable and
  atomic. The Overlap paragraph names IR-495 as merged, which is true (Done).
- TC-739 scope, procedure, tags and expected results add step 3 for AC-5. The
  spec/tests.md row matches.
- TC-090 and spec/native-packages/tests.md move the revision vector to 2^53 and
  drop "without a float cast".

## Verdict

Request changes: one medium finding. The PR moved three tests off `u64::MAX` but
left the spec text that names `u64::MAX`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-014-AC-4 ("u64::MAX offset"), TC-038 ("at u64::MAX") and TC-035 ("including u64::MAX") still name values IR now refuses to construct (revision and byte offset at most 2^53). The tests `tc_035_retains_explicit_source_assignment` and `tc_038_refuses_false_constructor_valid_formal_coordinates` were moved to 2^53, so they no longer match the AC and TC text. Restate these as 2^53 (the largest IR admits). | spec/functional/FR-014-bind-native-formal-source.md:71; spec/test-cases/TC-038-refuse-inconsistent-formal-loci.md:19; spec/test-cases/TC-035-retain-formal-source-assignment.md:20; tests/it/formal_source.rs:59; tests/it/formal_source.rs:193 |

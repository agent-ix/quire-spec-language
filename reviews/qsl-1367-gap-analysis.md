---
id: SR-1948002
title: "Gap analysis of PR #1948 (QSL-1367): model state keys carry relationship extents"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1948 git diff origin/main...a8823ee74 (FR-120 'State, state key and anchor', FR-120-AC-4; qsl-eval/src/simulation/model/encode.rs, qsl-eval/tests/it/model_simulation.rs); ticket QSL-1367"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-120
  type: reviews
---
# SR-1948002: gap analysis of PR #1948 (QSL-1367)

## Summary

Ticket: QSL-1367. Plan completion: not assessed. The PR changes no spec. `quire matrix --scope . --format json` did not finish within 300 s over the whole repository, so I used a grep index of `#[trace]` tags over the diff instead. The PR adds key behavior (a `relationships` entry in the `semantic` map) that no FR-120 text or acceptance criterion describes. The new test carries no trace tag.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-120 "State, state key and anchor" | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:191-215 | examined |
| FR-120-AC-4 | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:725 | examined |
| extents_enc / semantic_enc | qsl-eval/src/simulation/model/encode.rs:729-803 | examined |
| states_that_differ_only_in_extents_have_different_keys | qsl-eval/tests/it/model_simulation.rs:1829-1890 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-120 "State, state key and anchor" defines the `semantic` map as population keys only. It says the empty members match "a state family whose only state is its populations". It does not mention the new `relationships` entry, its `{complete, instances}` value form, or the instance ordering. So the code now writes key bytes that the spec does not define, and no AC fixes them. FR-120 needs that text and an AC, for example "two states differing only in extents have different keys; a repeated instance of a repeatable relationship is kept". | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:191-210; qsl-eval/src/simulation/model/encode.rs:753-758 |
| FND-002 | low | The new test has no `#[trace]` tag, so the matrix does not count it toward any FR-120 criterion. It is the only untagged test among the 25 in model_simulation.rs. | qsl-eval/tests/it/model_simulation.rs:1834 |

## Verdict

**CONDITIONAL.** The code fixes the reported collision. The reverted-code run fails the new test, and the head passes it. The only reverse gap is spec text for the behavior the PR adds. No stub was found, and the test is not inflated: it builds a real delete-then-spawn path and compares real keys.

## Coverage

Plan completion: not assessed. The criteria-to-test check was done by hand over the diff, because the `quire matrix` run timed out after 300 s. Semantic review: run over the diff, comparing FR-120 text with encode.rs.

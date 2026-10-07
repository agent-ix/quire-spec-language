---
id: SR-1365
title: "Spec review of quire-spec-language PR #653: FR-256-AC-4 and TC-749 (QSL-645)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5aaaf81cd8778e1ffb83e9cf286888e736ffb538; spec/functional/FR-256-parse-source-at-any-nesting-depth.md, spec/test-cases/TC-749-root-native-parser-parses-deep-sources-on-a-small-stack.md, spec/spec.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-749
    type: reviews
---
# Spec review of quire-spec-language PR #653

## Summary

Ticket: QSL-645. Sub-analyses applied: integrity (table structure, status consistency) and soundness of the edited statement against the code.

`quire validate` passes on FR-256 and TC-749. spec/tests.md fails on the TC-202 row, which is the same on origin/main and not from this PR. `quire matrix --scope .` at the head lists FR-256-AC-1 to AC-3 only.

Examined: FR-256 Description, FR-256-AC-4, TC-749, the FR-256 row in spec.md, the TC-749 row in tests.md. Context only: FR-256 Behavior 1-4, FR-256-AC-1 to AC-3.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A blank line separates the FR-256-AC-4 row from the Acceptance Criteria table, so the row is not part of it. `quire matrix` lists FR-256-AC-1 to AC-3 and no AC-4, and the ten `#[trace("TC-749", "FR-256-AC-4")]` tags bind to a criterion that does not exist. Delete the blank line. | spec/functional/FR-256-parse-source-at-any-nesting-depth.md:55 |
| FND-002 | medium | The Description now puts the root native parser under FR-256 as a whole: "bounded only by S1's resource limits: source bytes, tokens, syntax nodes and parser work", plus Behavior 3 (`s1.input_bytes`/`s1.work_units`, `stage_limit_exceeded` naming the FR-255 setting). The root parser has none of that. Its `Limits` has only `source_bytes`, `tokens` and `nodes`, with no work limit, and it refuses with `resource_exhausted` and a `SyntaxLimit`, not `stage_limit_exceeded` with a setting. Either scope the root parser's clause to its own three limits and diagnostic, or record the mismatch as open work. | spec/functional/FR-256-parse-source-at-any-nesting-depth.md:21-25 |
| FND-003 | low | The FR-256 row in spec.md says "not yet implemented -- TC-722, TC-723, TC-749 planned", but TC-749's Status and its tests.md row say "Passed locally". Say that AC-4 is implemented (TC-749). | spec/spec.md:1230 |
| FND-004 | low | tests.md lists TC-749 as `Unit`, but its tests are integration tests in tests/it/deep_sources.rs that go through the public API. | spec/tests.md:516 |

## Verdict

AC-4 is testable and it says what QSL-645 asks for, with five shapes, both editions, 512 KiB and limits used as given. But it does not resolve as a criterion (high), and the widened Description claims behaviour the root parser does not have (medium). Changes requested.

## Dispositions

Round 1, reviewed at 9163ca383852b50150be0ab472efcbb5c6008002.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3430ab929: blank line removed; `quire matrix` now lists FR-256-AC-4 and FR-256-AC-5 as tagged |
| FND-002 | fixed | 1b9555987: Description scopes the root parser to its `source_bytes`, `tokens` and `nodes` and its diagnostic. The finding's word `resource_exhausted` was wrong: `qsl_foundation::diagnostic::resource_exhausted` is a constructor that emits `Code::StageLimitExceeded` carrying a `SyntaxLimit` (qsl-foundation/src/diagnostic.rs:427-442). The new text matches the code |
| FND-003 | fixed | 3430ab929 |
| FND-004 | fixed | 3430ab929 |

---
id: SR-20261103161230
title: "Gap analysis — QSL-1520 multi-source replay (FR-098 / FR-480)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2196 diff vs origin/main, against FR-098 (statement, AC-1, AC-7) and FR-480 (AC-7)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: reviews
---

## Summary

Ticket: QSL-1520. Planless gap analysis, scoped to the PR diff. Plan completion: not
assessed. Examined units:

- the FR-098 statement's package-reference rule ("each of its `dependencies` entries ... SHALL name that dependency's source units the same way ... A `sources` list or an entry that is empty, names a definition document or repeats one ... refuses before any recompile, `invalid_package`/`invalid-value` at that list's pointer")
- FR-098-AC-1
- FR-098-AC-7
- FR-480-AC-7

Each was checked against the code and the tests tagged to it.

## Verdict

**CONDITIONAL**: FR-480-AC-7 and the FR-098 statement are implemented and tagged. A
`replay_multi_source` test covers each clause: canonical and reversed order give equal
packages and results, and a repeated owner or a definition document refuses at
`/package/sources` (`invalid_package`/`invalid-value`). The same coverage exists for
dependency entries at `/package/dependencies/{i}/sources`. FR-098-AC-7's text now
contradicts both the code and the test tagged to it, and the cross-library owner case has
no test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-098-AC-7 still says "an entry naming two sources, or a definition document, refuses as a source reference that is not one source unit". The code now accepts a two-source entry: `canonical_dependency_sources` and `with_library_units` do, and `replay_accepts_canonical_and_reversed_dependency_unit_sources` asserts it. At 581b0cf the two-source clause has been deleted from `tc_444_dependency_entries_refuse_by_the_d4_rules` (traced to FR-098-AC-7), so no test traced to AC-7 asserts that clause. The FR-098 statement already allows multi-source entries, so AC-7 is stale and contradicts both the statement and the code. Fix: update FR-098-AC-7 in this PR to the statement's rule (a multi-source entry replays; an empty, repeated-owner or definition-document entry refuses `invalid_package`/`invalid-value` at `/package/dependencies/{i}/sources`), then run spec validation. | spec/functional/FR-098-execute-a-replay-request.md:614; qsl-replay/src/execute/tests.rs:1504-1600; tests/it/replay_multi_source.rs:486-500 |
| FND-002 | medium | No test covers one source unit shared by two dependency entries when the duplicate is not each entry's first unit, for example three libraries. FR-098-AC-7's "two entries whose sources share one authority and identity refuse `DependencyInput` (`conflicting-definition`)" is exercised only for single-source entries. That is the case the code-review FND-003 defect escapes. Fix: add a multi-source cross-entry shared-owner test tagged FR-098-AC-7. | qsl-replay/src/spine.rs:1984-2005; tests/it/replay_multi_source.rs |
| FND-003 | medium | No replay test pins temporal clock selections that target a dependency-library source, or protocol-refusal continuation, through the new `run_spine_units` path. The focused run of `protocol`/`temporal`/`tc_444` lib tests passes while both behaviours changed (code-review FND-001/FND-002). Fix: add regression tests with those fixes. | qsl-replay/src/execute.rs:1779-1854 |

## Author follow-up

FR-098-AC-7 now matches the multi-source behavior and its refusal cases. The
shared-owner scan has a three-library regression case, and replay tests cover
library-source temporal selections. See the code-review follow-up for the focused
gate results and the remaining workspace-gate status.

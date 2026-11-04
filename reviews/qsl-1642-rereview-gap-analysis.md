---
id: SR-20261104130240
title: "Gap analysis (PR #2294 re-review) — QSL-1642 timed population refusal"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR #2294 head dd4c417ee: FR-230 timed-predicate population statement and FR-230-AC-58, FR-391, FR-480-AC-17, TC-1334, qsl-semantics timed_model.rs and its integration test"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: references
---

## Summary

Ticket: QSL-1642. Planless, diff-scoped gap analysis. Plan completion: not assessed. Each new obligation was traced to code and to a trace-tagged test. FR-230-AC-58 is backed by `timed_predicates_refuse_named_populations_without_restricting_state_clauses` (`#[trace("TC-1334", "FR-230-AC-58")]`), which passed locally. FR-480-AC-17's existing test is unchanged and still passes. The production code covers every form the new FR-230 statement names; the acceptance criterion and test exercise only the quantifier-domain form.

## Verdict

**PASS with findings** — the statement is broader than its AC and test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-230 statement refuses a named population used by `allInstances`, `lookup`, a quantifier domain or a population form, and the code handles all of them (AllInstances, Lookup, Query, Count, Sum, Accumulate, Population). FR-230-AC-58, TC-1334 and the test cover only `forall`/`exists` over `Config::calls`. The `allInstances`, `lookup` and count/sum/accumulate branches of `named_population_read` are unasserted. Fix: extend AC-58 (or add an AC) and the test loop with one `allInstances`, one `lookup` and one `count`/`sum` predicate. | spec/functional/FR-230-check-time-declarations-clocks-and-clock-constraints.md:290-298; qsl-semantics/tests/it/timed_model.rs:295-298; qsl-semantics/src/check/timed_model.rs:4652-4659 |
| FND-002 | low | TC-685's scope line now claims FR-230-AC-7 to FR-230-AC-58, but TC-685 has no step for AC-58; TC-1334 is its verifier. Fix: leave TC-685's range at AC-57. | spec/test-cases/TC-685-s3-checks-time-declarations-and-clock-constraints.md:15 |

## Scope

- Examined: FR-230 new timed-predicate population statement and FR-230-AC-58.
- Examined: FR-391 new timed-predicate paragraph.
- Examined: FR-480-AC-17 clarification (type-name resolution only); its test `#[trace("FR-480-AC-17")]` at tests/it/timed_model.rs:1056 is unchanged.
- Examined: TC-1334 procedure and expected results against the test.
- Examined: `named_population_read` branches against the statement's list of forms.

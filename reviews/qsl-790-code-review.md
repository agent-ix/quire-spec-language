---
id: SR-11751
title: "QSL-790 PR 909 code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@ff2531182d1e523143275b0ef3d0a0ba1c896de5; Cargo.lock, qsl-eval/src/value/expression/clause_set.rs, qsl-eval/src/value/expression/mod.rs, qsl-replay/src/spine/clause.rs, qsl-replay/src/spine/clause/tests/clause_set.rs"
review_set: subset
---
# QSL-790 PR 909 code review

## Summary

Ticket: QSL-790. One pass of code-review with the rust-review lane over
`git diff origin/main...HEAD` (5 files). Only major bugs block; mediums and
lows are recorded in the Verdict.

Examined:
- `clause_set::evaluate` duplicate pre-check (examined)
- `input_refusal_code` mapping for `DuplicateClause` (examined)
- replay `check_clause_admitted` match arm (examined)
- test `a_set_naming_a_clause_twice_is_refused_naming_it` (examined)
- Cargo.lock: quire-semantic-value ad6e975 -> e622ca7, quire-exact 37aa172 -> 526f5bd (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS, no blocking findings.

- The duplicate check runs over the whole slice before `resolve_clause`, and
  before any meter is built (each clause's meter is built from `limits` in
  `run_group`). So a duplicate is refused before any evaluation or charge.
  The first repeat triggers, so two, three or more copies are all refused.
  Names are compared by `QualifiedName` equality. `resolve_clause` accepts
  only unqualified names, so no second spelling of the same clause can
  resolve. Names that differ only in case are distinct clauses, not
  duplicates.
- Code mapping: QSL `Code::InvalidRuntimeInput` matches QSV e622ca7
  `code()` = `invalid_runtime_input` (cause `duplicate-member`).
- Replay arm: truly unreachable. Replay production calls only
  `evaluate_clause` (a single name), which never returns `DuplicateClause`.
  `evaluate_clause_set` is called only by tests (qsl-eval tc235 and the
  qsl-replay clause_set tests). The arm is explicit, with no wildcard.
- Test: without the check, both copies resolve and are evaluated, and
  `evaluate` returns `Ok`, so the `matches!` fails. It asserts the typed
  variant and the name `UpdateAllowed`.
- Lock: the quire-exact range 37aa172..526f5bd is two commits (#50, #51)
  that touch only spec/ and reviews/ files. No `.rs` file changed, so nothing
  semantic moved. The QSV range is the one commit that adds the variant plus
  its `code`/`cause` arms.

Recorded, non-blocking:
- low: the replay arm files `DuplicateClause` under the fault reason
  `evaluate-clause-supplies-no-function-style-input`. That reason names the
  wrong invariant (single-clause evaluation, not function-style input).
- low: the trait doc on `evaluate_clause_set` (mod.rs:513-516) lists the
  unknown-clause and mismatch refusals but not the duplicate refusal.
- low: the test doc at clause_set.rs:281-282 still says the duplicate refusal
  "needs `InputRefusal::DuplicateClause`, which is not in this test". That is
  now stale.

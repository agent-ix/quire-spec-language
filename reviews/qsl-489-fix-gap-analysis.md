---
id: SR-1308
title: "Gap analysis of quire-spec-language PR #636: FR-285-AC-5 and FR-277-AC-3 against their tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6014d82773ad72641e9934b6d62c2963f7b870a7; git diff origin/main...HEAD (PR #636): FR-285-AC-3, FR-285-AC-5, FR-277-AC-1, FR-277-AC-3, TC-758 step 6, TC-769 step 5, spec/tests.md TC-758/TC-769 rows, against qsl-replay/src/spine/call/tests.rs and tests/it/{cli,parser,source_map,composed_namespace}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: reviews
---
# Gap analysis of quire-spec-language PR #636

## Summary

Ticket: QSL-489 (LC1, reopened). PR: quire-spec-language#636.

The driver lane's three requirements map to code and tests as follows:
1. Every stage limit refusal, `CompileRefusal::Limit` included, is incomplete
   with exit 22: `Code::is_incomplete`, tested by
   `a_stage_limit_refusal_of_run_is_incomplete_and_exits_22` (FR-285-AC-5;
   source bytes at 1 and `s3.nodes` at 1, each asserting code, category,
   exit 22, limits field, kind and bound) and the five updated integration
   tests.
2. `execute`'s `Incomplete` carries the FR-277 fields:
   `an_execute_accounting_limit_carries_its_kind_bound_counter_and_field`
   (FR-277-AC-3). The minimal passing bound comes from a search, so "one
   below" is measured, not assumed, and FR-277-AC-1's "setting the field to
   the counter succeeds" half holds by construction.
3. `CheckingLimits` re-export with `with_nodes`:
   `changing_s3_nodes_alone_keeps_the_other_checking_limits`.

No production code in the diff lacks an owning requirement.

## Verdict

Complete for the defect. Two low findings, both about traceability: one test
binding and the stale status cells.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `changing_s3_nodes_alone_keeps_the_other_checking_limits` is tagged `TC-769`/`FR-285-AC-5`, but it asserts neither the category nor exit 22, which is FR-285-AC-5's oracle. What it checks (the `s3.nodes` field alone changes and the refusal names `s3.nodes` at bound 1) is FR-277-AC-1's field-naming property for `CheckingLimits`. Retag it `TC-758`/`FR-277-AC-1`, or add the category and exit assertions. | qsl-replay/src/spine/call/tests.rs:1049-1074 |
| FND-002 | low | spec/tests.md adds FR-285-AC-5 and FR-277-AC-3 to the TC-769 and TC-758 rows, but the status cells still say "steps 2 to 4 pass locally" and list step 1 and step 2 work only. They do not record that TC-769 step 5 and TC-758 step 6 now pass. | spec/tests.md:986; spec/tests.md:989 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a04276c53700556ae1699ec78b947a01d0e33d5b |
| FND-002 | fixed | a04276c53700556ae1699ec78b947a01d0e33d5b |

---
id: SR-12272
title: "Gap analysis of quire-spec-language PR #1156: QSL-935 against FR-120, FR-107 and QSpec FR-151-AC-19 / TC-196 D10"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@707401ee7eecfcb844c89593d2d7d3639948b9e1; PR #1156: clause_set::evaluate/run_group, dispatch_link::view_clauses, checked_dispatch module doc, the new simulator and dispatch tests and the edited FR-107-AC-14 replay test; against QSL FR-120 (Contract clauses, AC-9, Remaining work), FR-107 (Clause sets, AC-13, AC-14), FR-302, and QSpec FR-151-AC-17 to AC-19 and TC-196 D09 to D11"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1156

## Summary

Ticket: QSL-935. This review is planless, so plan completion is not
assessed. The ticket's four items were read as data and checked against the
code. QSpec was read at its current main and is cited by id.

- Item 1, the simulator short-circuit: implemented in `clause_set.rs` and
  covered by FR-120-AC-9's test (both name orders, both source orders) and
  the FR-107-AC-14 replay test. The rule matches FR-120 "Contract clauses"
  and QSpec FR-151-AC-19.
- Item 2, `view_clauses` in ascending clause name: implemented and covered
  by the new dispatch test, which fails without the sort.
- Item 3, the `checked_dispatch.rs` module doc: reworded to the FR-151 rule.
- Item 4, a QSpec TC-196 D10 test on both paths: partly built. See FND-001.
- Production code without an owning requirement: none.

## Verdict

PASS with one medium gap, to fix in-PR. Nothing blocks.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Ticket item 4 asks for TC-196 D10 on both paths, with the undefined clause a `lookup` that is undefined with reason `absent-key`, and with the clauses split across two linked units in both link orders. The PR builds neither. Its undefined clause is a nested call whose own precondition is false (`self.ready()` and `self.other()`). Nothing covers the split-unit variant. Yet the PR replaces FR-120's Remaining-work entry with "None", while FR-120 still says the order "does not depend on source position, source unit or link order". In QSL a checked package is one unit (`CheckedPackage::link(graph)`), and `view_clauses` reads only `unit_clauses_for` of that unit (FR-302 links dispatch during unit compile), so the split-unit variant has no QSL form today. The fix is to state that in FR-120: the source-unit and link-order clause is the one-unit case, because QSL links clauses from one unit. Otherwise keep a Remaining-work entry naming the split variant. Also either use the `lookup ... absent undefined` body that QSL already supports (`named_population.rs`) in one of the two tests, or say in FR-120-AC-9 why the nested-call undefined form stands in for D10's. | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:372; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md:609; qsl-replay/tests/dispatch_link_compile.rs:1225-1276; qsl-semantics/src/check/dispatch_link.rs:315 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-120 no longer says "None" under Remaining work. It states the order rule is verified for a one-unit package and keeps a Remaining-work entry for the split-unit variant, which has no QSL form today. A second entry says the nested-call undefined form stands in for D10's `lookup` / `absent-key`, and why that does not change the ordering check. |

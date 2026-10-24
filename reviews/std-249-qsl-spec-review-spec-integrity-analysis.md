---
id: SR-14033
title: 'Spec review of PR #1191: FR-147 node-row argument count and types (STD-249), spec-integrity analysis'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@53cee90bd; git diff origin/main...53cee90bd (PR #1191, branch std-protocol-249, based on main, not stacked): spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md, spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md; ticket STD-249'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-147
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-552
  type: reviews
---

## Summary

Ticket: STD-249. PR: quire-spec-language#1191, head 53cee90bd, based on main (not stacked). Method: spec-review with the spec-integrity sub-analysis. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

The dispatch brief named QSL head 2acaea4; no object 2acaea4 exists in either clone after fetch. The PR's live head is 53cee90bd (one commit on main), and that is what was reviewed.

The PR replaces FR-147's Remaining-work text ("QSpec does not yet state the binder record's fields per node kind ...") with the rule that the argument list is checked against QSpec FR-432's binder record, with the same two refusals, and adds FR-147-AC-14, TC-552 step 14 and its expected result, and widens TC-552's scope to AC-14. The refusal codes, loci and retained fields agree with the QSpec half. No other QSL spec file still cites STD-249 as open work.

Gates at head: `make check-spec-validation` exit 0, with no warning on FR-147 or TC-552; `tools/check-index-completeness.sh` exit 0. The diff touches nothing under `docs/` and no test-parsed vector table.

What node rows do today (QSL, read at 53cee90bd): `node_target` (qsl-semantics/src/check/refinement.rs:2130-2207) resolves only a three-segment name `<alias>::<protocol>::<node>` naming an attempt, effect, event, commit, send, receive or finish node, and refuses any other spelling `missing_declaration`/`missing-name`. It then types each written argument with no expected type, over `self` as the concrete receiver plus the concrete operation's parameters, and checks no count, as FR-147's old Remaining-work text said. The operation-application path (same file, 2209-2290) already refuses a wrong count `operator-ineligible` at the row and types the receiver and arguments against the abstract types after reference lifting. The grammar (qsl-cst/src/grammar.rs:2116-2123, and QSpec shared-grammar `protocol-step-target`) admits a node target with no list or with one or more `step-argument`s; an empty `()` is not admitted.

FR-147-AC-14 and TC-552 step 14 are unused on main and on every other open QSL PR. The PR body states gates, defaults and remaining work only; it paraphrases no review finding.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-147 step-row statement | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:104-115 | examined |
| FR-147-AC-14 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:289 | examined |
| TC-552 step 14 | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:58-59 | examined |
| TC-552 expected step 14 | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:108-110 | examined |
| TC-552 scope | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:17-21 | examined |
| FR-148 visible-step statement | spec/functional/FR-148-decide-steps-against-an-abstract-protocol.md:77-81 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-147-AC-14 says `-> node Spec::Twice::a1()` refuses `ill_typed`/`operator-ineligible` at the node with `actual` 0, and TC-552 step 14's expected result says "the argument lists of length 3 and 0". The grammar admits no empty argument list: QSpec's `protocol-step-target` is `'node', qualified-name, [ '(', step-argument, { ',', step-argument }, ')' ]`, and qsl-cst/src/grammar.rs:2116-2123 builds the same production. `a1()` is therefore a parse refusal at S1 and never reaches the S3 count check, so no build can meet the AC without a grammar change neither half specifies. Fix: drop `a1()` (and "and 0") or replace it with a parsable short list such as a two-field row `a1(self, _)` (actual 2). | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:289 |
| FND-002 | low | The unchanged first bullet still says a right side is "an abstract protocol node with one expression per field of its binder record", while FR-147-AC-14 now admits `-> node Spec::Twice::a1` with no argument list, as QSpec FR-432 says. FR-148's visible-step bullet ("a step of that node whose binder record equals the row's expressions") also reads as if every field is given, while FR-432 leaves an open or absent field unconstrained. Fix: say "with an argument list per QSpec FR-432 (an open or absent field is unconstrained)" in FR-147, and "agrees with the row's expressions on every field the row gives" in FR-148. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:104-115 |

## Verdict

Not mergeable as written. FND-001 is high: the AC's `a1()` row cannot parse, so its stated S3 refusal with `actual` 0 is unbuildable. FND-002 is low (stale wording in FR-147's first bullet and in FR-148). The rest of AC-14 agrees with QSpec FR-432 and with the counts recomputed by hand.

## Dispositions

Re-checked at head ad0c475ce (fix commit 1c5fd5137, then a merge of origin/main). Gates at head ad0c475ce: `make check-spec-validation` exit 0 with no warning on FR-147, FR-148 or TC-552; `tools/check-index-completeness.sh` exit 0; the diff touches nothing under `docs/` and no test-parsed vector table. The head contains origin/main. FR-147-AC-14 and TC-552 step 14 are used by no other open QSL PR. The PR body paraphrases no review finding. The `a1()` row is gone; `a1(self, _)` is a parsable two-argument list over a one-field binder record (inc has no parameter and no result), so expected 1 and actual 2 is right, and `a1(self, 1, 2)` is 1 and 3. The receiver type named, the reference type of `Spec::Counter`, is what QSpec FR-432 now gives for an attempt whose operation name names `Spec::Counter`. FR-214's observation-label bullet does not contradict the FR-148 wording.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed text at the new head: \| FR-147-AC-14 \| In a build that holds `protocol-abstract-side`, over `Twice`, whose attempt `a1` applies `Spec::Counter::inc` (no parameter and no result, so its binder record is the receiver alone), the rows `-> node Spec::Twice::a1(self)`, `-> node Spec::Twice::a1(_)` and `-> node Spec::Twice::a1` are admitted; `-> node Spec::Twice::a1(self, 1, 2)` and `-> node Spec::Twice::a1(self, _)` each refuse `ill_typed`/`operator-ineligible` at the node retaining `expected` 1 and `actual` 3 and 2; `-> node Spec::Twice::a1(1)` refuses `ill_typed`/`type-mismatch` at the argument retaining `expected`... |
| FND-002 | fixed | Fixed text at the new head: - Step rows SHALL cover every concrete step class by FR-136. A right side SHALL be an abstract operation application, an abstract protocol node with an argument list per QSpec FR-432 (an open or absent field is unconstrained), `stutter` or `any`. / FR-148: and arguments (and the concrete result when bound); for a node, a step of that node whose binder record agrees with the row's expressions on every field the row gives. |

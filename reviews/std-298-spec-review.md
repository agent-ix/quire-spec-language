---
id: SR-15540
title: "spec review of STD-298 quire-eval leaf and vocabulary placement (QSL PR #1227)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2013cea1d67df19afbc6f1cd905bf5f6d33a0666; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md, spec/assurance/AD-001-shared-evaluation-core-interface.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: reviews
---
# SR-15540: spec review of STD-298 quire-eval leaf and vocabulary placement (QSL PR #1227)

## Summary

Ticket: STD-298. This review checks whether FR-401-AC-1 to AC-7 can be tested, and it judges the eight lead
defaults in the PR body. The PR body labels them "lead default, not an owner answer". The diff is spec-only, so
`code-review` and `gap-analysis` have no code to review and were skipped. SR-15542 checks the type placements
against `origin/main` source. SR-15541 checks consistency across documents and open PRs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: AC-3's negative paths `qsl_eval::value::expression::Evaluation` and `::CallFailure` cannot fail. `value::expression` is a private module (`qsl-eval/src/value/mod.rs:13` `mod expression;`), so those paths already fail to compile today. The live public paths are `qsl_eval::value::{Evaluation, CallFailure}` (`value/mod.rs:18` `pub use expression::{.. CallFailure, .. Evaluation, ..}`), and AC-3 does not list them. A re-export left at `qsl_eval::value::` would pass AC-3. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:231 |
| FND-002 | high | major: FR-401 contradicts itself on what the leaf may name after X-13. "The checked closure the leaf reads" says `quire-eval` SHALL NOT name `CheckedGraph` or `CheckedExpression`, with no time limit. AC-6 inspects the same rule, and ADR-011's new "E holds the evaluation interface" bullet repeats it. Yet the same section says that when X-13 lands, `quire-eval` MAY name C's checked terms. FR-400 moves `CheckedGraph` and `CheckedExpression` into C. Limit the SHALL NOT and AC-6 to the period before X-13, or name only the types that stay in QSL (`CheckedPackage`, `ObjectEnvironment` and the checker-side record). | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:150-189,234 |
| FND-003 | medium | minor: lead default 6 is not in the spec text. The default admits FR-151 synthesized dispatch candidates (`callable_by_name = false`) for host-body registration by identity. FR-401 says only "the functions of the package's checked graph and of its dependency closure". Two implementers could read that differently. The `callable_by_name` gate exists so that no public entry reaches a synthesized candidate (`qsl-eval/src/value/expression/mod.rs:583-590`, TC-196 D07). State the rule in FR-401, and add an AC-5 row that registers a synthesized candidate's `NodeKey`. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:180-185 |
| FND-004 | low | minor: AC-5 contains vacuous parts. Its last clause, "the same parameter types under another parameter name are `Accepted`", cannot be expressed: a registration supplies only `ValueType`s (Behavior item 2), so no parameter name reaches it. The fixture function `g(): Integer` is declared but no clause uses it. Separately, AC-1 says the leaf has no `quire-walk` dependency before X-13, but Behavior item 4 sets no such limit. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:80-83,229,233 |
| FND-005 | low | minor: Some sentences give status or reconcile ticket text instead of describing behaviour. Examples: "defined today", "already K", "No type named `EvaluationRefusal` or `CallPlan` exists in QSL; the `IeeeItemRequirement` and `IntegerDivisionConsumer` fields are absent" (FR-401 S1 bullet), and AD-001's "the IR-585 names `EvaluationRefusal` and `CallPlan` have no counterpart in QSL". These belong in the Linear ticket. Keep only the S1 item set. | spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md:29,211-215 |

## Verdict

AC-2, AC-4 and AC-7 are testable as written. AC-1 is testable but needs the alignment in FND-004. AC-3 needs the
public paths (FND-001). AC-6 conflicts with the post-X-13 MAY (FND-002). AC-5 is testable for its first three cases.

Judgement of the eight lead defaults (lead defaults, not owner answers):

1. `quire-eval` at layer E in `agent-ix/quire-eval`, beside `quire-checked`: sound. The leaf cannot sit in the QSL
   repository without closing an FB-11 cycle. Creating the repository is an owner action that blocks S0.
2. S0 carries the vocabulary moves: sound and necessary, because the leaf cannot import F or a QSL layer. It
   contradicts the text of FR-400 (#1225) and NFR-015 (#1223), which still say S1 (SR-15541 FND-001, FND-003).
3. The catalog vocabulary goes to SV: sound and implementable (SR-15542). F → SV closes no cycle.
4. The S6a result types go to E, not SV: sound. No shipped QSL code below layer 5 names them.
5. A signature is its parameter and result `ValueType`s: sound. It matches `CallableFunction`'s `parameters`
   and `result`.
6. Synthesized candidates are admitted by identity: not stated in the spec, and it needs an explicit owner decision
   against the `callable_by_name` gate (FND-003).
7. No `std` feature, and FR-063 probe variants travel with the moved types: sound. SV and `quire-eval` then carry a
   QSL test cfg (`seam_probe`).
8. Module names `catalog` and `supplied_stop`: sound.

Not mergeable until FND-001 and FND-002 and the majors in SR-15541 are fixed.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | AC-3 now lists `qsl_eval::value::Evaluation` and `qsl_eval::value::CallFailure` (the live `pub use` paths) and adds `qsl_semantics::family::{FamilyResult, EvalOutcome}`; X-14 forbids a re-export at `qsl_eval::value::{Evaluation, CallFailure}` (1c24c146). |
| FND-002 | fixed | FR-401 now says `quire-eval` MAY name C's checked terms and SHALL NOT name any QSL item (`CheckedPackage`, `ObjectEnvironment`, the checker-side record); AC-6 and ADR-011's E rule bullet match. |
| FND-003 | fixed | The rule is in the spec: the adapter admits only functions callable by name; a synthesized FR-151 candidate returns `CallableNotAdmitted`, and AC-5 has a row for it. The lead default was reversed and is labelled in the PR body. |
| FND-004 | fixed | AC-5 now uses `g` (in a dependency package), drops the parameter-name clause, and gives a fresh registry per case; AC-1 and Behavior item 4 now agree ("drawn from" C, SV, K, W and `thiserror`, each declared only when named). |
| FND-005 | fixed | "defined today", "already K" and the `EvaluationRefusal`/`CallPlan` reconciliation are gone from FR-401 and AD-001. |

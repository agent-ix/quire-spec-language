---
id: SR-17180
title: "QSL-1072 PR #1523 base spec review of FR-487, FR-315 and TC-1264"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md, spec/functional/FR-315-encode-the-smt-lib-transition-relation.md, spec/test-cases/TC-1264-hyper-atoms-and-match-conjuncts-are-lowered-and-evaluated.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1264
    type: reviews
---

## Summary

Ticket: QSL-1072. PR: quire-spec-language#1523 (head 56043f5ba, diff `origin/main...HEAD`, merge-base 0d1a9e580). Spec-only. This is the base review: consistency with merged FRs and whether the text can be implemented against the real code. Code read: qsl-semantics/src/check/hyper_clause.rs and check.rs/typing.rs as of origin/main 0d0c67b0b, which includes the merged #1521 (QSL-867). Also read qsl-semantics/src/check/temporal_clause/condition.rs, qsl-eval/src/hyper/seam.rs, joint.rs, qsl-eval/src/smt/monitor.rs, automaton/translate.rs, simulation/model/letters.rs and value/expression/mod.rs. Review set: base plus integrity, ears-conformance, dependency, evidence, failure-domain and gap-analysis, the subset the dispatching brief chose. criterion-strength was skipped: it needs a Jev client and none is installed. AC strength was instead judged by reading, under integrity (SR-17181).

Examined units: FR-487 (statement and Behavior), FR-487-AC-1 to FR-487-AC-8, FR-315 (Joint steps, Property, Product identity refusal order), FR-315-AC-43, TC-1264, all examined. Read for context only: FR-172 (Indexed atoms and step labels, Match, Body expressions), FR-172-AC-5, FR-315-AC-7, FR-315-AC-10, FR-315-AC-12, FR-315-AC-13, FR-107, FR-104, FR-337, FR-176-AC-1/AC-4/AC-5, and QSpec FR-395 "Step labels".

Ids: FR-487, TC-1264 and FR-315-AC-43 are unused on main and in every open PR, #1510 and #1483 included. #1521 is merged, not open. `quire validate` on the three files passes, and `quire properties` gives all nine new criteria `example` / `assertion`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Contradiction with merged FR-172. FR-172 "Indexed atoms and step labels" says S3 SHALL type `t.step.op` as the operation-or-`stutter` label of `t`'s model. FR-487 types the slot `String` and types each conjunct "with the value typer over its slots". Under String typing, conjuncts that FR-172 and main refuse would check: `a.step.op = "x"`, `size(a.step.op) > 0`, and `a.step.op = b.step.op` with `a` and `b` over different models. Main refuses these today because #1521's `HyperType::StepLabel` compares only with another label. Such conjuncts would also expose the RFC 8785 declaration-identity text as user-visible string data. Fix: keep FR-172's label type for typing, so a label compares only, by `=`/`!=`, with a label of the same model, a label constant or `stutter`, and make `String` the lowered and evaluated representation only. Or amend FR-172 in this PR. | FR-487:152-154, FR-487:164-165, FR-172:107-110 |
| FND-002 | medium | Q1. A model whose operations with a receiver are declared by several object types now refuses `unsupported_construct`/`expression-form`. FR-172 still says S3 SHALL type `t.step.receiver` "as its receiver key" with no such limit. Main accepts such clauses, though no merged test uses such a model. The narrowing is acceptable: QSpec FR-395 types the receiver `Option<Reference<T>>` with one `T`, and no union reference type exists. But FR-172's bullet must carry the limit, or the two merged requirements disagree. Amend FR-172 "Indexed atoms and step labels" in this PR and add the refusal to an FR-172 criterion. | FR-487:155-163, FR-172:107-110 |
| FND-003 | medium | Q3. FR-487 admits a declared-function call in a conjunct and says FR-172's state-read refusal does not apply. This agrees with FR-172's text and with merged FR-315-AC-10, whose conjunct is a call. But main's `check_match` (hyper_clause.rs, `ExprNode::Call` arm) refuses every call as `MatchReadsState`. No criterion states that a call conjunct checks, or that a call whose argument reads state is refused at that read. AC-5 and AC-8 only assume it. Add both cases to an AC, for example FR-487-AC-2. Also state that the arguments obey the same step-label-only rule, so a coder knows to narrow `check_match` and not just drop the arm. | FR-487:171-173, FR-315-AC-10 |
| FND-004 | medium | The lowering assumes FR-172-AC-5's label constants (`a.step.op = V::Vault::step` checks), and FR-487-AC-2 and TC-1264 step 2 use them. Main has no test tagged FR-172-AC-5. Its `check_match` refuses `ExprNode::Name`, which is how the qualified name parses, as a state read. So QSL-1072 cannot pass AC-2 without first implementing FR-172-AC-5. State that QSL-1072 delivers FR-172-AC-5, or make it a named prerequisite. Today a coder would find out only when AC-2 fails. | FR-487:166-170, FR-487-AC-2, FR-172-AC-5 |
| FND-005 | low | Q2. #1521 (QSL-867) is merged. Its `type_body` binds one root per object parameter (`HyperRoot::Object`) and trace variables as Boolean roots. It types step labels as `Boolean` placeholders with `HyperType` markers, uses `CheckingLimits::default()` and a private node counter, and discards the typed node. FR-487 needs per-(trace, parameter) slots, real label types, the package node ceiling, definedness and kept conditions. So the lowering must replace that pass, not run beside it. The Dependencies note "QSL-867 moves its typing onto the value typer, which this lowering runs" suggests the reverse. Say that the lowering replaces the QSL-867 roots and label placeholders, and keeps its value-typer refusals (FR-172-AC-6). | FR-487:344-347 |

## Verdict

Changes requested: FND-001 is a contradiction with merged FR-172. Fix it, and FND-002 to FND-004, in this PR before QSL-1072 is coded.

These parts are sound: the refusal contract (`Refused { conjunct, outcome }`, `MatchIncomplete { conjunct, incomplete }`, short-circuit in index order); the removal of every seam from `HyperQueryRequest`; the SMT reading of atoms through the checked condition and of other safety bodies through a `MonitorTable` constructor shared with `MonitorTable::of`; the `monitor-atoms`/`monitor-states` refusal order, which matches FR-315's temporal table; and the identity rule. Together they resolve all three #1484 rejection reasons (SR-17048 FND-001 and FND-002, and CG reaching the request through qsl-replay). AC-8's tuple arithmetic over FR-315-AC-7's joint-step order checks out: (1,1) is the first tuple past conjuncts 0, 1 and the new one. AC-3's and AC-43's vectors match FR-176-AC-1/AC-5 and `monitor.rs`'s m0 convention. FR-093, FR-107, FR-120, FR-173, FR-176, FR-262, FR-337 and FR-485 are not contradicted.

Sequencing: rebase this PR on main, since #1521 has landed with no spec conflict. Then code QSL-1072 on top of #1521, replacing its hyper roots per FND-005, and then QSL-1071.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The rewritten Dependencies bullet says the lowering replaces "main's `hyper_typing` `type_condition` pass, and the QSL-867 `type_body` route", then says "Those roots are one root per object parameter, `Boolean` placeholders with `HyperType` markers ..., a private node counter, and a typed node that is discarded". That describes only the reverted #1521 route. Main (after the #1525 revert) types with `hyper_typing::type_condition`, a forward `Ty` pass with real label types that builds no checked node and has no `HyperType` markers. A coder on main would look for markers that do not exist. Describe each base in one clause, or say "on the QSL-867 route". | FR-487:449-455 |

## Dispositions

Round 1, head bd9361749 (fix commit be4e0bad6), checked against origin/main f8cfd238d.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | "Typing at S3" keeps FR-172's operation-or-`stutter` label type. A label is compared only by `=`/`!=` with a label of the same model, a label constant or `stutter`, and every other use (ordering, `size`, call argument, text literal, label of another model) is refused `ill_typed`/`type-mismatch`. `String` is now the lowered form only. FR-487-AC-2 adds the three refusals. |
| FND-002 | fixed | FR-172 "Indexed atoms and step labels" now types the receiver `Option<Reference<T>>` and refuses a read over several declaring types. FR-172-AC-7 (TC-1264 step 12) covers it. |
| FND-003 | fixed | FR-172 "Match" and FR-487 "Calls" now check a declared call as a value call, with arguments under the `match` rules and a state-reading argument refused at the read. Covered by FR-172-AC-7 and FR-487-AC-11. |
| FND-004 | fixed | FR-487's Description and "Label constants" bullet state that this requirement delivers FR-172-AC-5 and that `check_match` refuses `ExprNode::Name` today. |
| FND-005 | fixed | Dependencies now says the lowering replaces the S3 hyper typing roots on either base, keeps FR-172-AC-6, and gives the sequencing. A residual inaccuracy is FND-006. |

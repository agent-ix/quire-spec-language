---
id: SR-12901
title: "Scope-boundary review of PR #992: S2 versus S3 and QSL versus QSpec for refinement step sources and the abstract side"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@91bf1ee1d; FR-310 Inputs, Outputs (StepSourceForm, AbstractSideForm), Behavior, Dependencies; FR-136 step classes; agent-ix/quire-specification@origin/main FR-375 header grammar, FR-432 step-class table, shared-grammar protocol-step-source"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-310
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---

## Summary

Ticket: STD-178. PR quire-spec-language#992. Checks two boundaries: what S2
may decide versus what S3 owns (name resolution), and which surface forms
QSL builds versus what QSpec's grammar actually spells.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `StepSourceForm` separates `Operation(QualifiedName)` from `Class { kind: None, name }`, but only name resolution can tell the two apart. QSpec's `protocol-step-source` is `[ 'fork' \| 'join' \| ... ], qualified-name \| 'memory', ident`. A source with no keyword is a qualified name that may name an operation, an attempt node, a template, an event, send, receive, timeout or finish node, or a fence (FR-432's table). FR-136 resolves the class at S3. FR-310 itself says "S2 SHALL build no form that resolves a name". What breaks: for `step Impl::P::a::S -> stutter;`, one S2 builds `Operation` and another builds `Class{None}`. The only other option is for S2 to resolve the name, which breaks FR-310. Fix: use one keyword-optional named variant (for example `Named { kind: Option<StepClassKeyword>, name }`) plus `Memory`, and let S3 classify it. | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md:61-65,103-105; spec/functional/FR-136-check-a-refinement-s-step-rows.md:48-54 |
| FND-002 | medium | QSL's abstract side `Protocol(alias, clause)` has no QSpec production. FR-310 keeps "a model alias and a protocol clause name" and the `Protocol(Spec, Twice)` form (AC-2), and says QSpec owns the grammar through FR-375 and FR-432. FR-375's header is `'abstract', ident`, which admits a model alias only. FR-432 admits an abstract protocol subject but never spells how a declaration selects one. FR-147's `abstract Spec::Twice` is therefore refused at `::` by a parser that follows QSpec. What breaks: QSL's S1 and QSpec's grammar disagree on whether `CasTwice` parses. Author's open issue 2: yes, QSpec needs to spell it. File a QSpec follow-up (ADR-020 QS-11 already asks for it), and until then mark the `Protocol` side in FR-310 as pending QSpec. | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md:32-37,45,114,125-126 |

## Verdict

Not mergeable until FND-001 is fixed. It is a small change to the form
shape. FND-002 can be deferred to a tracked QSpec ticket, provided FR-310
says that the `Protocol` abstract side waits on it.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | TC-889 was not updated when the form changed. Its Expected Results step 2 still expects "a `Class` source with a `Node` target", and FR-310 no longer has a `Class` variant. Nothing in the procedure builds AC-2's new `step Impl::P::a::S -> stutter;` row, which should give a `Named` source with no kind. An implementer who follows TC-889 cannot match its oracle. The `Named`-without-kind clause, which is the point of the FND-001 fix, goes untested. | spec/test-cases/TC-889-s2-builds-the-refinement-form.md:24,38-40; spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md:123 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | TC-889's step 2 expected result now says "a `memory` source builds `Memory`", but step 2's procedure builds no `step memory <kind>` row, so that expectation has no input. Either add a `step memory flush -> stutter;` row to the step 2 procedure or drop the clause. | spec/test-cases/TC-889-s2-builds-the-refinement-form.md:25-26,42-43 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | aea8326f7 replaces `Operation` and `Class` with one `Named { kind: Option<StepClassKeyword>, name }` plus `Memory`. It also adds the rule that S2 builds the keyword as written and does not decide whether an unkeyworded name is an operation, node or fence; S3 classifies it (FR-136). AC-2 adds the unkeyworded vector. The TC-889 side of the change is FND-003. |
| FND-002 | deferred | Tracked as Linear STD-182 (QSpec production for the abstract protocol subject, Backlog). aea8326f7 records in FR-310 Behavior and FR-147 Inputs that the `Protocol` side waits on that production. |
| FND-003 | fixed | 7b1aa3d34 adds `step Impl::P::a::S -> stutter;` to TC-889 step 2. The expected result is now a `Named` source of kind `fork` with a `Node` target, and `Named` with kind `None` and name `Impl::P::a::S` for the unkeyworded row. The stale `Class` wording is gone. |

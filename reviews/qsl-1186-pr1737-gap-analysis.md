---
id: SR-1737002
title: "gap analysis of PR #1737 (QSL-1186b): choice and repeat to V2"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1737 git diff origin/qsl-1186a-parallel...09b0d42e3; FR-466 choice/repeat rows, FR-464-AC-1/2/4, FR-218 (post-test, AC-32/33), FR-375 catalog keys"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1737002: gap analysis of PR #1737 (QSL-1186b)

## Summary

Ticket: QSL-1186. Head reviewed: 09b0d42e3.

What is covered:
- The FR-466 choice and repeat layouts are traced to FR-464-AC-1/2/4 by `a_choice_emits_...`, `a_repeat_emits_...` and `choices_and_repeats_nest_...`. These cover owner, visible, cases sorted by bytes, maximum, proof, guard, body and exhausted, plus the empty optionals and IR read-back.
- The three typed refusals are covered.

Gaps found by mutation and probe:
- **FND-001: the S3 fix has no S3-level or simulation regression test.** Reverting `.difference(&readable)` survives all 165 qsl-semantics protocol tests and qsl-eval. Only two qsl-package emission tests kill it, and only indirectly.
  - Nothing asserts that a guard reading an earlier receive gives post_test=false.
  - Nothing asserts that `max 0` is then admitted.
  - Nothing asserts that the simulation skips the body on a false entry guard.
  - The flipped fixtures (composed-choreography `Retry`, recorded.rs `Again`) assert neither reading.
- **FND-002: the `Choice => all` arm of `repeat_steps_through_repeat` is untested.** The mutant `all -> any` survives protocol_controls. With that mutant, `repeat Loop while { true } choice { case a: event; case b: repeat Inner while { true } {event} }` is emitted, and IR's reader refuses it with UnsupportedConstruct/DeclarationForm. The real code refuses RepeatStepsThroughRepeat.
- **FND-003: no positive FR-375 catalog test for a clause holding a choice or repeat.** The only catalog edit swaps the negative fixture. A probe shows such a clause now has keyed catalog rows (`Local::Ea`, `Local::E`, keyed=true), but no test pins it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The post_test fix has no direct test (entry-binder guard gives pre-test, `max 0` admitted, simulation skips the body); the reverted mutant survives qsl-semantics and qsl-eval | FR-218 |
| FND-002 | low | The `Choice` arm of repeat_steps_through_repeat is untested; the `all->any` mutant emits a package IR refuses | FR-464 Progress, FR-218-AC-32 |
| FND-003 | low | No test that a clause with a choice or repeat now keys its class catalog | FR-375-AC-23 |

## Verdict

Changes requested: FND-001. FND-002 and FND-003 are tests to add in this PR.

## Dispositions

Round 1, head 8be8961f2. The gates are those in SR-1737001's dispositions.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `graph.rs` `repeat_decisions_read_the_latest_body_output_and_classify_the_guard` adds a repeat whose guard reads an earlier attempt binder: it is `post_test == false`, and its `max 0` is admitted. `qsl-analyze` `repeat_guard::a_guard_over_an_entry_binder_is_pre_test_and_skips_the_body` covers an event `ready false` and a guard `{ ready }`: the body never runs, and `r1` keeps its initial value. Mutation check: I reverted `.difference(&readable)` to the plain `flow.output(body)`, and both tests fail (EXITM1=101, EXITM2=101). The mutant that survived before is now killed. |
| FND-002 | fixed | `decisions_without_a_node_form_are_not_written` has a `RepeatStepsThroughRepeat` case whose outer body is a `choice`. Case `a` steps (an event) and case `b` is a nested always-true repeat, so under `Choice => all` the outer body does not necessarily step. |
| FND-003 | fixed | `refinement_class_catalog::a_clause_holding_a_choice_and_a_repeat_keys_its_catalog` asserts that `class_catalog_keys()` is `Some` for a clause holding a choice and a pre-test repeat, with the events in the case and in the repeat body as classes with nodes. |

---
id: SR-11699
title: "Spec review — QSL-802 PR #895 state-machine contexts in snapshot documents"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@34e79e1e4cbe424d454f6ee8d7143c1c2ea3cf78; QSL-802; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md; spec/test-cases/TC-1040-a-clause-evaluation-reports-its-receiver-and-read-paths.md; spec/test-cases/TC-1045-a-snapshot-carries-the-state-machine-context-of-an-invocation.md"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-106, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-107, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1045, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1040, type: references }
---

## Summary

Ticket: QSL-802. Spec-only PR #895 at head 34e79e1e4, diff `origin/main...HEAD`. One pass: contradiction, completeness, vector soundness (check ordering, reachability, loci), EARS/AC consistency, and determinism, all checked against the merged FR-106 checks 1 to 11, FR-107, FR-098 and QSpec FR-208 (State-machine context anchors, AC-10, AC-13). Only major issues are blocking findings. Mediums and lows are one line each under Review Notes.

The ticket's outcome asks how a snapshot "carries the states/transitions" an invariant is evaluated against. The author reads that as the machine's context. That reading meets the outcome. QSpec FR-208 binds no current state. A machine's states and transitions are model declarations, and a transition's `from`/`to` are not related to the pre and post observations. The invariant's `self` is the context, its effective field values. QSL-793 is about the public effective view of model declarations, not observations. A snapshot member for states or transitions would bind nothing, so leaving it out is correct, and FR-106 says so explicitly.

## Verdict

FAIL — four blocking findings. Two admission checks that the PR did not extend (check 5) or extended inconsistently (check 9) make every `Context` admit in AC-41 to AC-47 unreachable as written. Two TC-1045 steps use machines with no clause or invocation, so they cannot produce their expected outcomes. One FR-107 SHALL bullet contradicts the new paragraph. Codes and causes are all in QSpec native-diagnostics, with no collision. The text has no roadmap language and no ticket ids. FR-098 needed no edit: an environment document is a current snapshot, and check 1.6 refuses `contexts` on it.

## Examined Scope

```yaml
scope:
  - {id: FR-106, path: spec/functional/FR-106-admit-snapshots-and-invocations.md, role: examined, excerpt: "Outputs (Context bullet), Document forms (contexts, invocation self), ClauseSelection Context, Behavior checks 1-11 incl. 6.10-6.12, 7, 8, 9, 11.6, value-count rule"}
  - {id: FR-106-AC-41, path: spec/functional/FR-106-admit-snapshots-and-invocations.md, role: examined, excerpt: "Context side pre/post over i1 admits one observation; Completed(true); reordering gives the same typed observation and digest."}
  - {id: FR-106-AC-42, path: spec/functional/FR-106-admit-snapshots-and-invocations.md, role: examined, excerpt: "post context {placed: true, cancelled: true} gives Completed(false); side pre ignores the post side and parameters."}
  - {id: FR-106-AC-43, path: spec/functional/FR-106-admit-snapshots-and-invocations.md, role: examined, excerpt: "no context -> invalid_model_binding/wrong-context; two -> duplicate-member; machine + specialization -> duplicate-member at check 9."}
  - {id: FR-106-AC-44, path: spec/functional/FR-106-admit-snapshots-and-invocations.md, role: examined, excerpt: "checks 6.10-6.12 one case per condition."}
  - {id: FR-106-AC-45, path: spec/functional/FR-106-admit-snapshots-and-invocations.md, role: examined, excerpt: "post-state range witness reaches an ExpressLifecycle context."}
  - {id: FR-106-AC-46, path: spec/functional/FR-106-admit-snapshots-and-invocations.md, role: examined, excerpt: "frame check compares contexts; self kind; selection form."}
  - {id: FR-106-AC-47, path: spec/functional/FR-106-admit-snapshots-and-invocations.md, role: examined, excerpt: "Owned context reference closure; observation.values; FR-098 environment doc with contexts refused."}
  - {id: FR-107, path: spec/functional/FR-107-evaluate-state-clauses-at-s6a.md, role: examined, excerpt: "The receiver path; Context-admitted observation paragraph."}
  - {id: TC-1045, path: spec/test-cases/TC-1045-a-snapshot-carries-the-state-machine-context-of-an-invocation.md, role: examined, excerpt: "steps 1-7 and expected results"}
  - {id: TC-1040, path: spec/test-cases/TC-1040-a-clause-evaluation-reports-its-receiver-and-read-paths.md, role: examined, excerpt: "step 6 Consistent admitted by Context side post"}
  - {id: FR-098, path: spec/functional/FR-098-execute-a-replay-request.md, role: context_only, excerpt: "environment document is a quire.state.snapshot/v1 current snapshot admitted by FR-106"}
  - {id: QSpec FR-208, path: "quire-specification spec/objects/foundation/FR-208", role: context_only, excerpt: "State-machine context anchors; AC-10; AC-13; no member binds the current state"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Check 5 (operation check) has no rule for a `Context`-selected invariant. It refuses when the invocation's `operation` "differs from the clause's", but an invariant has no operation. Read literally, every `Context` selection refuses `wrong_snapshot`/`wrong-invocation` (or the outcome is undefined) before checks 6 to 11, so the admits in AC-41, 42, 45, 46 and 47 and TC-1045 steps 1, 2 and 5 to 7 are unreachable. Check 5 needs a rule for `Context`: the invocation's `context` is the clause's machine or one of its specializations, and `operation` is any effective operation of that machine (QSpec FR-208: every invocation of every operation of the machine). | spec/functional/FR-106-admit-snapshots-and-invocations.md:523-531 |
| FND-002 | high | Check 9 contradicts itself. "If an invocation's `self` is `null`, then admission SHALL refuse `invalid_runtime_input`/`wrong-value-kind`" is unconditional, and it comes before the machine-receiver sentence that requires `self` to be `null`. Read literally, every state-machine invocation refuses at check 9, which contradicts AC-41 to AC-43 and AC-46 and the Document forms bullet on `self`. Scope the sentence to an object-receiver clause. | spec/functional/FR-106-admit-snapshots-and-invocations.md:691-695 |
| FND-003 | high | TC-1045 steps 5 and 7 (AC-45, AC-47) use machines that no clause or invocation reads. `Owned` has no clause and no operation, and `i1` is an invocation of `OrderLifecycle::place`. Checks 7 and 8 close only references in "the context the clause reads (check 9)", and any other machine's context is "validated by check 6 and then ignored". So `owner` naming the absent `p9` admits (or, with only an `Owned` context, refuses `wrong-context` at check 9), never `dangling_reference` or `Incomplete` as expected. Step 5's "a clause on `ExpressLifecycle`" is undeclared, has no invocation of an `ExpressLifecycle` operation, and gives no pre context (check 9 needs one of `ExpressLifecycle` in the pre snapshot too). Declare the clauses, operations, invocations and pre contexts these steps need. | spec/test-cases/TC-1045-a-snapshot-carries-the-state-machine-context-of-an-invocation.md:67-82 |
| FND-004 | high | The FR-107 receiver-path rule still says "The path's observation SHALL be the clause's own observation (`current` for an invariant, ...)". The new paragraph, and TC-1040 step 6's expected `(q1, invocation i1, receiver, [])`, give a `Context`-selected invariant the selected side's `pre` or `post` snapshot. The SHALL bullet and the paragraph contradict each other, and the paragraph says the rule "above" holds unchanged. Amend the bullet. | spec/functional/FR-107-evaluate-state-clauses-at-s6a.md:132-142 |

## Review Notes

Mediums and lows, recorded and not blocking:

- medium — Checks 7 and 8 use "the context the clause reads (check 9)", but check 9 runs after them. With two candidate contexts (a machine and its specialization), the spec does not say whether a dangling reference in one of them refuses at check 8 before check 9's `duplicate-member`.
- medium — AC-46's "a `Current` selection naming a `self` for a machine precondition" is already refused by check 2's first sentence (`Current` selects no precondition), so it tests nothing new. TC-1045 step 6 tests `PreCall` naming `self` instead, so AC and TC disagree on that case.
- medium — AC-41 and TC-1045 step 1 claim reordering the `contexts` entries keeps the same digest. RFC 8785 keeps array order, so with two or more entries the digest changes, and `i1`'s `pre` digest then refuses at check 1.3. With P1's single entry the claim is vacuous.
- medium — Check 11.6 does not say what happens when the pre and post contexts name different most-specific machines (`OrderLifecycle` to `ExpressLifecycle`). Objects refuse `FrameTypeChanged`.
- medium — AC-45's "A context field of a record, String, UUID, Timestamp or Text type follows the value rules" has no TC-1045 step.
- low — Check 6.10's first sentence (`contexts` on a `current` snapshot) is unreachable: check 1.6 already refuses `unknown-member` naming `contexts`, with the same code and cause, so the outcome is unaffected.
- low — Check 6.10 tests kind before missing and unknown members. Check 6.6 for relationships uses the reverse order.
- low — TC-1040 calls the post snapshot `q1` ({placed: true, cancelled: true}), which TC-1045 calls `Q2`. "TC-1045 lists the documents" should name the mapping.
- Checked and sound: the machine plus a specialization counts as two contexts (`duplicate-member`), consistent with QSpec FR-208 "exactly one context of the machine". `wrong-context` and `duplicate-member` are in QSpec native-diagnostics with no collision. Check 2 is updated consistently for an invariant selected by `Invocation`. FR-098 needs no edit.

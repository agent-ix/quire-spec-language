---
id: SR-12311
title: 'Gap analysis of PR #1198: FR-147 node-row binder-record checks (QSL-887)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@41fbf4af8; git diff origin/main...41fbf4af8 (PR #1198, branch qsl-887d-impl) against FR-147 "Step rows and observations", FR-147-AC-14, TC-552 step 14, ADR-027 ST-1 and QSpec FR-432 at agent-ix/quire-specification@97bfd65e; ticket QSL-887'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-147
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-552
  type: reviews
---

## Summary

Ticket: QSL-887. PR: quire-spec-language#1198, head 41fbf4af8. Method: quoin gap-analysis, planless. Plan completion: not assessed. Tooling: quoin 0.28.3, spec-artifacts-process@b82e42e.

Every FR-147-AC-14 row has a test tagged `TC-552`/`FR-147-AC-14` that asserts the right code, cause, locus and retained fields:

| AC-14 row | Test | Asserts |
| --- | --- | --- |
| `a1(self)`, `a1(_)`, `a1` admitted | `a_node_row_whose_arguments_fit_the_binder_record_checks` | one node step with one field; 0, 1, 1 open |
| `a1(self, 1, 2)` expected 1 actual 3; `a1(self, _)` expected 1 actual 2 | `a_node_row_with_the_wrong_argument_count_is_refused_at_the_node` | `ill_typed`/`operator-ineligible`, at `Spec::Twice::a1`, full cause equality |
| `a1(true)` expected Counter reference, actual `Boolean` | `a_node_row_argument_of_the_wrong_type_is_refused_at_the_argument` | `ill_typed`/`type-mismatch`, at `true`, position 0, `actual` `Boolean`, `expected` some reference (SR-12310 FND-002) |

Each test fails when its check is disabled (measured; see SR-12310).

The FR-147 statement the AC backs is wider than AC-14. It says "The expressions of a node right side SHALL be typed against the node's binder record, whose fields per node kind are QSpec FR-432's", and FR-147 resolves a node right side to an `attempt`, `effect`, `event`, `commit`, `send`, `receive` or `finish` node. QSpec FR-432 "The binder record of an abstract node" (added by STD-249, QSpec #330, merged 2026-10-21 20:37 -0700, before this PR's commit at 21:02 -0700) states the fields for every one of them:

> `effect`, `event`, `commit`, `finish` | when the node's binder type `T` is a record type, one field per field of `T`, in declaration order; otherwise exactly one field
> `send`, `receive` | as `event`, over the channel's `carries` type

The PR implements the `attempt` row only and writes into FR-147 that QSpec does not yet state the others (FND-001; the false spec text is SR-12312 FND-001). The data is in reach of the checker: binder types are already resolved for protocol nodes (`lowering::binder_type`, the `ProtocolAnchorCause::BinderType` refusals in protocol_controls.rs and protocol_clause/coverage.rs) and `CheckedProtocol::vias` gives each send/receive its channel.

Out of scope here, already declared by the PR body as remaining QSL-887 work: FR-147-AC-1/AC-3 (FR-148 decider), AC-9/AC-10 (S4 carriage), FR-136 protocol subject, FR-137-AC-3.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-147-AC-14 | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:299 | examined |
| FR-147 node typing statement | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:112-125 | examined |
| TC-552 step 14 | spec/test-cases/TC-552-s3-checks-a-refinement-whose-abstract-side-is-a-protocol.md:58-59 | examined |
| ADR-027 ST-1 | spec/decisions/ADR-027-protocol-transition-system-for-parallel.md:162 | context_only |
| QSpec FR-432 binder record section, AC-14, AC-15 | quire-specification spec/functional/protocol/FR-432-map-every-protocol-step-in-refinements.md:85-110, 238-239 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-147 SHALLs that node-row arguments are typed against the binder record of every node kind per QSpec FR-432, with the count and type refusals. QSpec FR-432 states the records of `effect`, `event`, `commit`, `finish` (fields of the binder type `T` when a record, else one field `T`) and `send`, `receive` (the same over the channel's `carries` type). The PR checks `attempt` only; `ProtocolClasses::record` returns `None` for the six others, so `node Spec::A::ev(true)` over a two-field event and `node Spec::A::eb(true, false)` over a one-field event are admitted where FR-432-AC-15 refuses them `operator-ineligible`, and any argument type is admitted. Implement the six kinds in this PR, with tests for the FR-432-AC-14/AC-15 event rows. | qsl-semantics/src/check/refinement/internal_list.rs:536-544 |

## Verdict

Not mergeable. FR-147-AC-14 is fully and strongly covered, and its checks are mutation-killed. FND-001 is high: the FR-147 requirement this AC belongs to is met for one of seven node kinds, and the deferral that excuses the other six rests on a statement about QSpec that is false at QSpec main.

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed: `ProtocolClasses::of` derives the record of effect, event, commit, finish (record fields or one field) and send, receive (over the channel's `carries`); tests for ev, eb, tx, rx and Done rows; the existing `Done(self)` row became `Done(true)` | 3a5180722 |

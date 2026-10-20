---
id: SR-12226
title: "Gap analysis of QSL-559 E15 PR 2 ProtocolSystem<Sc> constructs (PR #1106)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@aaea08dd918f98407927d81c7afee7af6808fc21; FR-205-AC-3, FR-206-AC-4, FR-206-AC-5, FR-210-AC-3, FR-215-AC-4 (TC-650, TC-651, TC-655, TC-660), ADR-027 §9 step 4; qsl-analyze/tests/it/{protocol_constructs.rs,protocol_system.rs}, qsl-eval/src/simulation/protocol/**"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-210
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: reviews
---
# SR-12226: Gap analysis of QSL-559 E15 PR 2 (PR #1106)

## Summary

Ticket: QSL-559. PR quire-spec-language#1106 at aaea08dd9, scoped to `git diff origin/main...HEAD`. This is a planless gap analysis. Each criterion in the brief was read in this repo's `spec/functional` and checked against its tagged test and the code it exercises. ADR-027 §9 step 4 was checked against what `refuse_unsupported` still refuses. Intent↔test↔code was checked by hand per AC, alongside SR-12225.

## Verdict

**No major gap.** Five criteria are tagged and fully covered, with two low test-strength nits. One medium: several constructs from §9 step 4 still have no owning PR. FND-001 of SR-12225 is a soundness defect with no test, and its regression test belongs with that fix.

Covered (tag, then the test that asserts it):

- FR-206-AC-4 (TC-651), `channel_premises_admit_exactly_their_queue_steps`. It covers `block` with no second send and a `full-queue` cause, `reject` enabled with the queue unchanged and the record `rejected`, `at-least-once` `duplicate` up to capacity and no `lose`, `at-most-once` `lose` and no `duplicate`, two receives from a bag of two distinct payloads, and one receive from a `fifo` head.
- FR-206-AC-5 (TC-651), `structural_moves_add_no_step`. It covers a `choice` over an earlier binder, `repeat max 2` that runs X, X, then E with counters 1 and 2, a false `check` resting in every later state, `timeout` at the waiting state, a match settling into `then`, and an unbounded `repeat` that keeps no counter and loops on one state.
- FR-210-AC-3 (TC-655), `a_message_orders_its_receive_after_its_send_and_nothing_else`. It asserts the three linear extensions, the same set with a second channel, and the crossed-channel case that has one extension.
- FR-205-AC-3 (TC-650), `a_repeated_fork_reuses_the_lowest_free_ordinal`. It covers ordinal 1 while `b` of instance 0 runs, ordinal 0 reused with equal paths, and both completion orders reaching one state.
- FR-215-AC-4 (TC-660), `a_protocol_model_check_is_deterministic`. It covers `RequiresBound` naming the `event` binder, Fill `Violated`/`Deadlock`, the repaired Fill `Holds` over ten states, and byte-equal reruns.

Not tagged, deliberately: `a_branch_reads_its_parents_binding_as_of_its_fork` carries only `TC-650` for ADR-027 PS-5. That is correct, since no AC owns PS-5's fork-time copy. FR-206-AC-3's predicate row stays untagged, as the PR states.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | ADR-027 §9 step 4 lists event nodes and channels with no carve-outs. Several constructs this PR leaves refused `not-yet-implemented` have no owning PR: `fence`, a symbolic channel capacity, `loss head|tail` on an `unordered` channel, `join predicate(f)` and protocol lasso replay (FR-217-AC-4). `effect` is assigned to PR 3. The other five have no owning PR, so their ACs can stay untagged when QSL-559 closes. Each needs an owner on the ticket, plus a spec ruling for `fence` (PS-3/FR-426) and for an unordered loss selector. | qsl-eval/src/simulation/protocol/program.rs:221-246, qsl-eval/src/simulation/protocol.rs:62-80 |
| FND-002 | low | FR-215-AC-4 says the `event`'s "record type has an unbounded `Int` field". The test uses the scalar `Integer` as the record type, and the asserted `DomainKey` path is `[]`. A record with an `Int` field would check that the refusal names the field path as well as the binder. | qsl-analyze/tests/it/protocol_system.rs:993-1020 |
| FND-003 | low | FR-206-AC-5's last sentence says the unbounded `repeat` "never enters `exhausted`". The test's `repeat` declares no `exhausted` control, so that half cannot fail. If the grammar admits `exhausted` without `max`, the fixture should declare one and assert it is never entered. If it does not, the AC clause is vacuous and should be dropped. | qsl-analyze/tests/it/protocol_constructs.rs:457-473 |

## Coverage

- Criteria examined: 5, all tagged. Their 5 bindings are correct. 2 are complete with a nit (FND-002, FND-003).
- Stubs and inflation: none. There is no `todo!`, placeholder return or tautological assertion in the diff.
- Reverse gap: none new. `channel_steps`, `settle`, `bind` and `reads_from` each have a production caller through `expand`/`normalize`.
- Untested soundness defect: SR-12225 FND-001, ordinal reuse under a running grandchild.
- Plan completion: not assessed

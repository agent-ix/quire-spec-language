---
id: SR-12252
title: "Gap analysis of QSL-559 E15 PR 3b ProtocolSystem<Sc>: replicated roles and activation on each (PR #1139)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@d9bb4ef97d03028400d44824d439c531ae556f5e; qsl-eval/src/simulation/protocol/**, qsl-semantics/src/check/{protocol_clause,protocol_slots,footprint.rs}, qsl-analyze/tests/it/{protocol_roles.rs,protocol_footprints.rs,weak_memory.rs,protocol_system.rs,protocol_compensation.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
---
# SR-12252: Gap analysis of QSL-559 E15 PR 3b (PR #1139)

## Summary

Ticket: QSL-559. PR quire-spec-language#1139, head d9bb4ef97. Plan completion: not assessed.

The criteria in scope were checked against the merged spec on origin/main: FR-208-AC-1 to AC-3, FR-209-AC-1 to AC-4, FR-205-AC-4, FR-211-AC-4 and AC-5, and FR-216-AC-5. The trace index is a grep of `#[trace(...)]` tags, because `quire matrix` had not finished within the session. Each binding was read against its test body. No test was run: the focused run was refused by the session's permission layer.

## Verdict

**CONDITIONAL**, with no major gap of its own. SR-12251 FND-001 is the merge blocker, and FND-001 below is the same work seen from the coverage side.

Bindings checked:

- `a_retired_role_instance_is_never_spawned_again` → FR-208-AC-1: correct. Its "after it, `spawn(r, w2)` is enabled" oracle is loose (FND-005).
- `spawn_order_is_no_part_of_the_key_and_each_live_instance_acts` → FR-208-AC-2: correct.
- `workflow_and_scope_lifetimes_retire_in_the_step_that_ends_them` → FR-208-AC-3: correct.
- `an_on_each_protocol_starts_with_no_instance` → FR-205-AC-4: correct for the `on each` half. The Fill half is `protocol_system.rs:598`, untagged ("Part of").
- `an_on_each_state_that_can_only_activate_is_a_deadlock` → FR-211-AC-4: correct.
- `an_instance_takes_the_smallest_free_ordinal_and_the_bound_is_no_deadlock` → FR-209-AC-3: correct. It reads `terminal_class`, not the deadlock-freedom item.
- `role_and_activation_steps_have_role_and_instance_footprints` → FR-216-AC-5: correct for the role and `pinst` parts.
- The two `NoRoleInstance` tests → FR-211-AC-5: incomplete (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The declared remainder FR-209-AC-1, AC-2 and AC-4 is not blocked. The reason the PR gives, that S4 refuses a temporal clause over an `on each` protocol, is wrong. `temporal_clause::is_emitted` refuses a clause whose own activation is `on each` (`form.activation`). A TP-1 claim `... on origin { always holds(k.v <= 1) }` over protocol `R` compiles, as `BOUNDED` and `NO_BOTH_ZERO` do over `on origin` protocols today. What remains is ordinary in-scope plumbing: `ModelCheckRequest.max_live_instances`, `Undecided(InstanceBoundReached{bound, states})`, its precedence, the terminal-record statement and obligation-identity independence. The default of 3 is also unasserted. This is the fix for SR-12251 FND-001. | qsl-semantics/src/check/temporal_clause.rs:1135-1137, spec/functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md |
| FND-002 | medium | FR-211-AC-5 is now tagged on two role-only tests. The repository's convention for a partial test is an untagged "Part of", as `protocol_system.rs:1038` and `protocol_compensation.rs:787` do. The tag makes the whole AC read as covered, yet these role clauses are untested: `NoRoleInstance` at an `attempt`, at a `send` and at a compensation attempt (the new `compensation.rs:498` branch), and "a `receive` of a send node with nothing queued gives `NoRoleInstance{r}` and not `EmptyQueue`". The receive test asserts the queued case only. Add those cases, or drop the tag back to "Part of". | qsl-analyze/tests/it/protocol_roles.rs:368-470, qsl-eval/src/simulation/protocol/compensation.rs:498-500 |
| FND-003 | low | For fixtures 16 and 17, `every_independent_pair_of_steps_commutes` accepts any pair with an `Event` as well as a `Spawn`/`Retire` pair. A run with no `spawn` or `retire` pair checked would pass, although the PR body says "Spawn/Retire/by-role/Activate pairs are among the independent pairs checked". Assert a `Spawn` or `Retire` pair separately. By reading, the fixture 18 `Activate` assertion is not vacuous: without `finish`'s `pinst` write, `activate` and `finish` are reported independent, and their two orders give ordinals 1 and 0. | qsl-analyze/tests/it/protocol_footprints.rs:856-878 |
| FND-004 | low | The four spec gaps in QSL-559 comment 0b94faf0 are real: the identity's `instance` member (TS-2, QSpec FR-426), `from pop` unresolved at S3, send/receive ownership absent from FR-218's outputs, and the derivation rule for guard and `until` reads in FT-1. None has an owning spec ticket. The `ActivationOnEachTemplate` refusal is not spec-blocked. Template frames would need the activation binder slot that QSpec FR-381 places. It is an honest `not-yet-implemented` that needs an owner on QSL-559. | qsl-eval/src/simulation/protocol/program.rs:355-363 |
| FND-005 | low | The FR-208-AC-1 test sets `respawned` for any state that holds a retired instance and has some `spawn` enabled. It never follows the `retire(r, c)` edge to check that `spawn(r, d)` is enabled from there. Its `p` (`r.v >= 1`) is not tied to "the instance has worked", because `Work` applies `inc` to every receiver whatever the actor. | qsl-analyze/tests/it/protocol_roles.rs:203-262 |

## Coverage

- Tagged in this PR: FR-208-AC-1, AC-2, AC-3; FR-209-AC-3; FR-205-AC-4 (`on each` half); FR-211-AC-4; FR-211-AC-5 (role part, FND-002); FR-216-AC-5 (role/`pinst` part).
- Untagged: FR-209-AC-1, AC-2, AC-4 (FND-001).
- Reverse gap: none. `CheckedProtocol::owners`, `slot_reads` and `with_max_live_instances` each serve an AC above.
- Stubs: none found.
- Semantic review: skipped as a separate pass. The bindings were read by hand.
- Plan completion: not assessed

## Dispositions

Round 1, head c83e872e8. The tests were not re-run, because the permission layer refused them in the review pass; each outcome was judged by reading the tests. The coder reports `qsl-analyze --test it` green (544 tests).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | c83e872e8 |
| FND-002 | fixed | c83e872e8 |
| FND-003 | fixed | c83e872e8 |
| FND-004 | deferred | STD-243 owns spec gaps 1 to 4 (and item 5, FR-209-AC-4's identity half). The `ActivationOnEachTemplate` refusal stays a QSL-559 remainder: the ticket stays open, and the refusal is an honest not-yet-implemented. |
| FND-005 | fixed | c83e872e8 |

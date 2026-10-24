---
id: SR-12314
title: "Gap analysis of QSL-559 PR #1202: replicated-role S3 population, send/receive region, universe field reads (STD-243)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@c3f550f1dbcd1bb67856715a02b015ee3033829c; qsl-semantics/src/check/{protocol_slots.rs,protocol_clause/mod.rs,refusal.rs}, qsl-eval/src/simulation/{model.rs,model/footprints.rs,protocol/program.rs,model/domain.rs}, qsl-analyze/tests/it/{protocol_footprints.rs,protocol_roles.rs}, qsl-semantics/src/check/protocol_slots/tests.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
---
# SR-12314: Gap analysis of QSL-559 PR #1202

## Summary

Ticket: QSL-559. PR quire-spec-language#1202, head c3f550f1d. Plan completion: not assessed.

The criteria in scope were checked against the STD-243 spec merged on origin/main (QSL #1187, 08330ebf4): FR-208-AC-4 and AC-5, FR-209 "The bound" and its Dependencies identity statement, FR-216-AC-6, FR-218-AC-28 and AC-29, and ADR-027 FT-1 "Locations a predicate reads". Each binding was read against its test body. Each new behaviour was also mutated, and the mutation was killed (SR-12313).

The coder named two gaps. Both were reproduced:

- A role predicate `until(lookup<M::Cell>(M::cells, r) absent refused = r)`, where `cells` is a population that the role's own `from cells` resolves, refuses at S3 with `MissingSegment { name: "M::cells" }`. Written unqualified, it refuses with `SlotClause { slot: Lifetime, cause: MissingName("cells") }`.
- `enumerable` returns `false` for every `ValueType::Composite`. So `program.rs` refuses every record-typed trigger with `UnenumerableRecord`, a bounded one included.

## Verdict

**PASS** for this PR. There is no major finding. The two gaps the coder reports are honest about what is unbuilt. They are code gaps, not spec defects:

- FR-218 bars model state in choice and repeat guards, `check`, `visible`, FIFO keys, activation and trigger guards, retry relations and recovery predicates. It does not bar it in a `retire` predicate. ADR-027 FT-1 gives the footprint of a population read in a retire predicate.
- FR-209 "Instances" gives one `activate(T)` per record of `T`.

Neither gap is recorded in QSL-559's remainder, which names only the liveness engine. Both need a `Remaining work` line, and FR-216-AC-6's tag overstates coverage until they land (FND-001, FND-002).

Bindings checked:

- `spawn_and_retire_identities_carry_the_protocol_instance` → FR-208-AC-4: correct.
- `a_send_and_a_receive_owned_by_a_replicated_role_are_taken_by_each_instance` → FR-208-AC-5: correct for the send and receive halves.
- `an_attempt_and_a_send_owned_by_a_role_with_no_instance_give_no_role_instance` → FR-208-AC-5: wrong for the empty-universe clause (FND-003).
- `a_replicated_roles_population_resolves_in_the_package_of_its_type` → FR-218-AC-28: correct.
- `send_and_receive_are_owned_by_their_channel_roles_and_count_in_the_region` → FR-218-AC-29: correct.
- `predicate_reads_are_the_fields_of_the_universe_and_a_role_read_orders_spawns` → FR-216-AC-6: correct for the field-read, `role(r)`, dependence and send/spawn clauses; partial overall (FND-001).
- `requests_differing_only_in_max_live_instances_share_one_obligation_identity` → TC-654 (FR-209 "The bound"): equality matches the spec, and the brief's "differ" is wrong. The test is structural (SR-12313 FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-216-AC-6 is tagged as fully covered, but two of its clauses are untested and cannot run today. The first is "a predicate that also reads `lookup<Job>(jobs, k)` holds the membership of the `Job` population": S3 has no named populations in a role predicate's scope (measured: `MissingSegment` at `M::cells`). The second is the guard `x.n > 0` over a record binder: record triggers refuse `UnenumerableRecord`, so the test uses an `Int` binder. This is a code gap in S3's slot scope and in the protocol program, not a spec defect. Record both as QSL-559 Remaining work. Per the repository's "Part of" convention, mark the test partial until they land. | qsl-analyze/tests/it/protocol_footprints.rs:989-1075, qsl-semantics/src/check/protocol_slots.rs |
| FND-002 | medium | FR-209 "Instances" gives one `activate(T)` per record of `T`, and its ACs use the record trigger `Tick`. `enumerable` treats every `Composite` as unbounded, so every record trigger, a bounded one included, refuses `UnenumerableRecord`. Every `on each` test therefore runs over the `Int` range `M::Level`. The gap predates this PR but is untracked. It blocks FR-209's record case and FR-216-AC-6's record-binder guard. Add it to QSL-559's remainder, or file it as its own ticket. | qsl-eval/src/simulation/model/domain.rs:15-36, qsl-eval/src/simulation/protocol/program.rs:363-372 |
| FND-003 | medium | The new `TC-653`/`FR-208-AC-5` tag sits on a test whose doc calls it "the empty-universe half". Its world is `retired_role_world`: universe `{c}`, with `c` retired at once. AC-5 states "with an empty universe a thread at the `send` waits with `NoRoleInstance{r}`". No test builds an empty universe, a path where no `spawn` ever exists and the population lists no instance. Add a `keys = []` world for the `send`, or retag. | qsl-analyze/tests/it/protocol_roles.rs:600-641 |

## Coverage

- Tagged in this PR: FR-208-AC-4; FR-208-AC-5 (FND-003); FR-216-AC-6 (FND-001); FR-218-AC-28; FR-218-AC-29; TC-654 (FR-209 "The bound").
- Reverse gap: none. `written_owner`, `role_population`, `universe_declaring` and the two new refusal variants each serve an AC above.
- Stubs: none found.
- Spec files changed: none, so no spec-review was run.
- Plan completion: not assessed

## Dispositions

Round 1, head 4d558634d. QSL-968 (Backlog) exists and names both FND-001 gaps.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 4d558634d. The test is tagged `TC-661` only, with a partial note that cites QSL-968. No test carries `FR-216-AC-6` any more, so the matrix shows it uncovered, which is honest. |
| FND-002 | deferred | QSL-968, item 1 (record triggers, `UnenumerableRecord`). |
| FND-003 | still-open | The retired-object test no longer carries the tag, but `FR-208-AC-5` is still tagged in full on `a_send_and_a_receive_owned_by_a_replicated_role_are_taken_by_each_instance` (protocol_roles.rs:1367). Its empty-universe clause has no test and is not in QSL-968. The claim that "an empty cell universe cannot be built because `over` needs a cell" holds only for Fill's single `cells` population. A second, empty, closed population of `Cell`, with `role r each M::Cell from <it>`, builds the case while `over` still binds `c` from `cells`. Add that world, or mark the test partial and add the clause to QSL-968. |

Round 2, head 2236f590d (fix commit 406c08a38, then a merge of main). Measured by mutation, then restored.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | 406c08a38. `a_send_from_a_role_over_an_empty_universe_waits_with_no_role_instance` builds a role over a new type `Idle`, whose closed population `idles` has an empty universe, and asserts that no `spawn` exists and that the `send` waits with `NoRoleInstance{r}`. The mutation made `no_role_instance` require a role that had held an instance. This test failed (1 failed, 22 passed) and the retired-object tests still passed, so the test exercises the empty-universe clause itself. FR-208-AC-5 is now tagged on two tests that together cover the send/receive halves and the empty-universe clause, so neither overclaims. |

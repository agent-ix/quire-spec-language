<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.4 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2026111306075602 method=gap-analysis lang=rust pr=quire-spec-language#2697 model=claude-opus-5-5 run=42c2157c-bfb4-4b50-a98f-bcc300009c3b -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | coverage | FR-575-AC-1 tag covers 4 of 8 rows; dynamic-role-population, compensation, timed-await, unassigned-local-choice, predicate join, outstanding untested |
| FND-002 | medium | test-intent | FR-573-AC-2 and AC-5 tags assert none of the FR-360 labels the ACs require |
| FND-003 | medium | coverage | FR-570-AC-1/4/5 backed only by flat choices; nested choice and multi-sender visible untested |

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-570-AC-1
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: For a projectable protocol the local graph's events and causal order equal the global event structure restricted to the role's events through the correspondence table, every local event maps to exactly one global event, and the local order is the induced order of any causal path (implements FR-174-AC-1).
- id: FR-570-AC-2
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: Every global communication projects to exactly one local send and one local receive, the correspondence table is total over the role's global events, and the send/receive pair is matched by message type and peer (implements FR-174-AC-4).
- id: FR-570-AC-3
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: A `parallel` region with `join all` projects to a local partial order with no enumerated interleaving, and two receives on one channel with equal evaluated FIFO key keep their global order as local receive order (PA-9).
- id: FR-570-AC-4
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: 'A choice branch merges for the role only by the PA-10 rule: owner, `visible`, an earlier outcome-carrying message, or distinct first-receive message types, or pairwise equal (direction, peer, message type) first events recursing on their continuations; a send against a receive, an empty first set against a non-empty one, and differing peers or send message types do not merge (implements FR-174-AC-5).'
- id: FR-570-AC-5
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: A choice the role cannot distinguish returns a counterexample naming the choice, owner, role, the two paths and the two actions, of smallest length with the PA-11 tie-break, and returns no `local_graph` and no substitute graph; for two failing choices the counterexample names the one the tie-break orders first (implements FR-174-AC-2).
- id: FR-570-AC-6
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: For `Seller` owning `ship | cancel` with `ship` sending `pick` to `Warehouse`, projection onto `Warehouse` returns the counterexample (`receive pick` against end); with `cancel` sending `cancelled` to `Warehouse`, or with an earlier message between `Warehouse` and `Seller` whose payload binder the choice's `visible` fact reads, it returns the witness; an earlier `decision` message of one type in both branches that no decision fact reads returns the counterexample (ADR-036 worked example).
- id: FR-570-AC-7
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: A request the engine cannot decide (FR-575) or whose search reaches `max_depth` returns a non-conclusive result with no graph, and the result carries no substitute for the missing graph (implements FR-174-AC-3).
- id: FR-573-AC-2
  path: spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md
  role: examined
  excerpt: A projection counterexample and each of the two realizability refutation forms settle `refuted`, `decisive-counterexample`, execution `completed`, truth `violated`, only after the counterexample replays through `ReplaySource::ProtocolAnalysis`.
- id: FR-573-AC-3
  path: spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md
  role: examined
  excerpt: A counterexample whose replay disagrees with the checked graph (a step that is not an admitted successor, or a walk that does not end in the stated divergence) settles `inconclusive`, never `refuted`.
- id: FR-573-AC-5
  path: spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md
  role: examined
  excerpt: A run stopped by `max_states`, `max_transitions` or the evaluation meter settles `failed`, execution `resource-incomplete`, truth `unavailable`, with no artifact and a record naming the limit and the value used.
- id: FR-574-AC-1
  path: spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md
  role: examined
  excerpt: Every `refuted`/`decisive-counterexample` result of either kind carries exactly one record; a `proved`, `inconclusive`, `unsupported` or `failed` result carries none.
- id: FR-574-AC-4
  path: spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md
  role: examined
  excerpt: For a projection counterexample whose divergent action is the third event of `path_a`, `trace_position` is `2`; for indistinguishable histories of lengths 3 and 5 it is `4`; for an environment defeat whose violation occurs at the fourth step of the first owner branch's defeating play it is `3`; two counterexamples that differ only in the step of divergence give different positions.
- id: FR-575-AC-1
  path: spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md
  role: examined
  excerpt: For each construct row other than `evaluator-unsupported`, a protocol using it yields `unsupported`, `unsupported_projection` with exactly that slug and the construct's checked node id, for projection and for realizability, and returns no local graph, strategy or counterexample (implements FR-174-AC-3).
- id: FR-575-AC-3
  path: spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md
  role: examined
  excerpt: A request with profile `bounded-static` over a protocol using only the profile's constructs is decided, and a request with any other profile word is refused `invalid_runtime_input`/`invalid-value` naming `profile` before any analysis.
findings:
- fnd: FND-001
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-575-AC-1
  related_ids:
  - FR-174-AC-3
  path: qsl-analyze/tests/it/protocol_projection.rs
  lines: 509-581
  excerpt: For each construct row other than `evaluator-unsupported`, a protocol using it yields `unsupported`, `unsupported_projection` with exactly that slug and the construct's checked node id, for projection and for realizability, and returns no local graph, strategy or counterexample (implements FR-174-AC-3).
  finding: The tagged test covers unsupported-join (any, quorum), repeat, unsupported-control (check only) and symbolic-queue-capacity. dynamic-role-population, compensation (template and compensate control), timed-await, unassigned-local-choice (protocol-owned choice), a predicate join and outstanding under join all are untested; the node check is skipped for symbolic capacity. evaluator-unsupported is slice 4 and not a gap.
- fnd: FND-002
  severity: medium
  confidence: high
  method: gap-analysis
  check: test-intent
  artifact_id: FR-573-AC-2
  related_ids:
  - FR-573-AC-5
  path: qsl-analyze/tests/it/protocol_projection.rs
  lines: 677-730
  excerpt: A projection counterexample and each of the two realizability refutation forms settle `refuted`, `decisive-counterexample`, execution `completed`, truth `violated`, only after the counterexample replays through `ReplaySource::ProtocolAnalysis`.
  finding: the_replay_arm_reproduces_... asserts only the replay arm's Category::Success; refuted, decisive-counterexample, completed and violated are never read. a_run_limit_fails_the_run_naming_the_limit (FR-573-AC-5) asserts Failed(limit) but not resource-incomplete or unavailable. Narrow the tags to what is shown, or assert the labels.
- fnd: FND-003
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-570-AC-4
  related_ids:
  - FR-570-AC-1
  - FR-570-AC-5
  path: qsl-analyze/tests/it/protocol_projection.rs
  lines: 385-463
  excerpt: 'A choice branch merges for the role only by the PA-10 rule: owner, `visible`, an earlier outcome-carrying message, or distinct first-receive message types, or pairwise equal (direction, peer, message type) first events recursing on their continuations; a send against a receive, an empty first set against a non-empty one, and differing peers or send message types do not merge (implements FR-174-AC-5).'
  finding: No test has a choice nested in a case or a visible set read from several senders, the two places the engine is unsound (SR-2026111306075601 FND-001, FND-002). The AC-1 graph-equality test uses one fixture with no choice, so a merged or owned choice's local graph is never compared with the global structure.
bindings:
- test_id: constructs_outside_the_profile_answer_unsupported_with_no_graph
  ac_id: FR-575-AC-1
  trace: correct
- test_id: the_replay_arm_reproduces_a_counterexample_and_refuses_an_edited_one
  ac_id: FR-573-AC-3
  trace: correct
- test_id: the_worked_example_without_a_distinguishing_message_has_a_counterexample
  ac_id: FR-570-AC-6
  trace: correct
- test_id: local_graphs_equal_the_global_structure_restricted_to_the_role
  ac_id: FR-570-AC-3
  trace: correct
- test_id: the_trace_position_is_the_divergent_actions_index_in_path_a
  ac_id: FR-574-AC-4
  trace: correct
```

+++

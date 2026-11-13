<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.4 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2026111306075601 method=code-review lang=rust pr=quire-spec-language#2697 model=claude-opus-5-5 run=42c2157c-bfb4-4b50-a98f-bcc300009c3b -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | soundness | Nested choice hides its communications from the outer merge: unsound Proved (probe reproduced) |
| FND-002 | high | soundness | Informed basis accepts a role that knows only one of several decision facts: unsound Proved (probe reproduced) |
| FND-003 | medium | code-bug | Merged choice leaves two concurrent local events and no alternative marker; local graph is not the role's behaviour |
| FND-004 | medium | code-bug | Witness edge build is cubic in kept events and unbounded by any run limit |
| FND-005 | medium | soundness | Whole branch pair skipped on any cut event; reported counterexample may not be the PA-11 minimum |
| FND-006 | low | other | ReplayResult::ProtocolAnalysis has no producer; facade refuses the source as WrongModelGraphEntry |
| FND-007 | low | other | Replay arm reuses engine classify/merge, so it cannot catch their faults |
| FND-008 | low | code-bug | shortest_path unwrap_or_default silently stores an empty path |
| FND-009 | low | other | PA-24 record does not travel on native-run-result/2 though SW-7 ties that to this producer |

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
- id: ADR-036-PA-7
  path: spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md
  role: context_only
  excerpt: 'Choice visibility: a non-owner role distinguishes a choice''s branches through `visible`, through an earlier received message that carries the outcome (FR-053-AC-1: the choice''s guards are a function of that message''s payload), or through a divergent first receive (PA-10).'
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: code-review
  check: soundness
  artifact_id: FR-570-AC-4
  related_ids:
  - FR-570-AC-5
  - FR-174-AC-5
  path: qsl-replay/src/protocol_analysis/structure.rs
  lines: 453-513
  excerpt: 'A choice branch merges for the role only by the PA-10 rule: owner, `visible`, an earlier outcome-carrying message, or distinct first-receive message types, or pairwise equal (direction, peer, message type) first events recursing on their continuations; a send against a receive, an empty first set against a non-empty one, and differing peers or send message types do not merge (implements FR-174-AC-5).'
  finding: 'case_actions keeps only events whose nearest enclosing case is the merged case, so a choice nested in a case hides its communications from the outer merge. Probe: Decide by Seller, cancel empty, ship = choice Inner {a: pick | b: pick2}; projection onto Warehouse returns Proved (outer MustMerge merged: [], inner merged) although Warehouse receives PickMsg in one outer branch and nothing in the other. Must be a counterexample.'
- fnd: FND-002
  severity: high
  confidence: high
  method: code-review
  check: soundness
  artifact_id: FR-570-AC-4
  related_ids:
  - FR-570-AC-6
  - FR-053-AC-1
  path: qsl-replay/src/protocol_analysis/structure.rs
  lines: 621-637
  excerpt: 'A choice branch merges for the role only by the PA-10 rule: owner, `visible`, an earlier outcome-carrying message, or distinct first-receive message types, or pairwise equal (direction, peer, message type) first events recursing on their continuations; a send against a receive, an empty first set against a non-empty one, and differing peers or send message types do not merge (implements FR-174-AC-5).'
  finding: 'classify returns Informed when any carrier receive the role took part in feeds the decision reads, without checking the decision depends only on what the role knows (PA-7: guards a function of that payload). Probe: visible (got.ok, got2.ok), got sent by Warehouse, got2 by Buyer, guards on got2.ok, ship sends pick to Warehouse: Proved with Informed{carrier: Got}. Require every decision fact to come from the role''s carriers or captures.'
- fnd: FND-003
  severity: medium
  confidence: medium
  method: code-review
  check: code-bug
  artifact_id: FR-570-AC-1
  related_ids:
  - FR-570-AC-4
  path: qsl-analyze/src/protocol_analysis/project.rs
  lines: 288-355
  excerpt: For a projectable protocol the local graph's events and causal order equal the global event structure restricted to the role's events through the correspondence table, every local event maps to exactly one global event, and the local order is the induced order of any causal path (implements FR-174-AC-1).
  finding: 'For a MustMerge choice both branches'' merged events stay as separate unordered local events (Warehouse: two concurrent Receive PickMsg, no edges), and no event marks alternatives for owned or external choices. The local graph reads as ''receive twice''; a consumer cannot rebuild the local type without merge_evidence and global ids. Collapse merged pairs to one local event or mark alternatives, and state the rule in FR-570.'
- fnd: FND-004
  severity: medium
  confidence: medium
  method: code-review
  check: code-bug
  artifact_id: FR-570-AC-1
  related_ids: []
  path: qsl-analyze/src/protocol_analysis/project.rs
  lines: 329-347
  excerpt: For a projectable protocol the local graph's events and causal order equal the global event structure restricted to the role's events through the correspondence table, every local event maps to exactly one global event, and the local order is the induced order of any causal path (implements FR-174-AC-1).
  finding: witness runs kept x kept x kept precedes checks (about 6.9e10 at the 4096-state default) after max_transitions has passed; shortest_path rescans all covers rows per frontier node. Derive local covering edges from the structure's covers/below, or charge the work to a run limit.
- fnd: FND-005
  severity: medium
  confidence: medium
  method: code-review
  check: soundness
  artifact_id: FR-570-AC-5
  related_ids:
  - FR-174-AC-2
  path: qsl-replay/src/protocol_analysis/structure.rs
  lines: 690-695
  excerpt: A choice the role cannot distinguish returns a counterexample naming the choice, owner, role, the two paths and the two actions, of smallest length with the PA-11 tie-break, and returns no `local_graph` and no substitute graph; for two failing choices the counterexample names the one the tie-break orders first (implements FR-174-AC-2).
  finding: merge returns Cut for the whole pair when any action of either case is beyond max_depth, even if they diverge within the horizon. A counterexample from another choice is then reported as the PA-11 minimum though the cut pair may hold a shorter one. Compare the prefix up to the horizon.
- fnd: FND-006
  severity: low
  confidence: high
  method: code-review
  check: other
  artifact_id: FR-570-AC-7
  related_ids: []
  path: qsl-replay/src/result.rs
  lines: 1102-1103
  excerpt: A request the engine cannot decide (FR-575) or whose search reaches `max_depth` returns a non-conclusive result with no graph, and the result carries no substitute for the missing graph (implements FR-174-AC-3).
  finding: ReplayResult::ProtocolAnalysis has no producer (replay_protocol_analysis returns ProtocolReplayResult), so the new arms (plugin.rs TerminalValue::Failed, UnsupportedReplayArm, measured bytes) are dead until slice 5; the replay facade refuses the new source as WrongModelGraphEntry. Wire it or defer the variant; rename the refusal.
- fnd: FND-007
  severity: low
  confidence: medium
  method: code-review
  check: other
  artifact_id: FR-570-AC-5
  related_ids:
  - FR-573-AC-3
  path: qsl-replay/src/protocol_analysis/artifact.rs
  lines: 255-303
  excerpt: A choice the role cannot distinguish returns a counterexample naming the choice, owner, role, the two paths and the two actions, of smallest length with the PA-11 tie-break, and returns no `local_graph` and no substitute graph; for two failing choices the counterexample names the one the tie-break orders first (implements FR-174-AC-2).
  finding: The PA-20 replay arm confirms the divergence with the engine's own classify and merge, so faults like FND-001/FND-002 are reproduced, not caught. Document it, or check stated actions against the cases' minimal remaining actions directly.
- fnd: FND-008
  severity: low
  confidence: medium
  method: code-review
  check: code-bug
  artifact_id: FR-570-AC-5
  related_ids: []
  path: qsl-analyze/src/protocol_analysis/project.rs
  lines: 152-156
  excerpt: A choice the role cannot distinguish returns a counterexample naming the choice, owner, role, the two paths and the two actions, of smallest length with the PA-11 tie-break, and returns no `local_graph` and no substitute graph; for two failing choices the counterexample names the one the tie-break orders first (implements FR-174-AC-2).
  finding: shortest_path(..).unwrap_or_default() stores an empty path for an action that does not follow the choice; the replay then disagrees and the run settles Inconclusive(ReplayDisagreed) instead of failing on an internal fault.
- fnd: FND-009
  severity: low
  confidence: medium
  method: code-review
  check: other
  artifact_id: FR-574-AC-1
  related_ids:
  - ADR-031-SW-7
  path: spec/decisions/ADR-031-state-forall-separating-witness.md
  lines: '94'
  excerpt: Every `refuted`/`decisive-counterexample` result of either kind carries exactly one record; a `proved`, `inconclusive`, `unsupported` or `failed` result carries none.
  finding: ADR-031 SW-7 says the native-run-result/2 reader extends to the protocol record with its producer; this PR is the producer but neither writer nor reader handles a protocol record. Extend it here or move that line to slice 5 (QSL-1897).
bindings: []
```

+++

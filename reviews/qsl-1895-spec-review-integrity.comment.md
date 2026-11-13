<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.4 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2026111306075603 method=spec-review/integrity lang=spec pr=quire-spec-language#2697 model=claude-opus-5-5 run=42c2157c-bfb4-4b50-a98f-bcc300009c3b -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | soundness | FR-570 'observing the outcome' needs only one read payload binder; weaker than PA-7 'guards a function of that payload' |
| FND-002 | medium | other | FR-570-AC-6 rewrite of the worked example (decision a) leaves ADR-036 PA-7/PA-10/worked example contradicting it; direction unstated |
| FND-003 | medium | soundness | Horizon bullet skips whole cut pairs, so PA-11 minimality does not hold under a cut |
| FND-004 | low | ambiguous | FR-570-AC-4 still lists `visible` as a separate basis |

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-570-choice
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: At a `choice` the engine shall classify the role as internal choice when it is the owner, as observing the outcome when it sent or received an earlier message whose payload binder the choice's decision facts read (the `visible` expressions, followed through the value nodes they reference; FR-053-AC-1; a checked package's `visible` holds expression nodes, so a role "lies in `visible`" exactly by taking part in such a message), as external choice when a peer sends a receive in every branch's first events and the message types differ across branches (PA-10).
- id: FR-570-path-a
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: 'A failure to merge shall return the counterexample of PA-11 and no graph: the choice, its owner, the role, two global paths to the first divergent action (path A being the path to an action when the other branch has ended, else the one whose node id sequence is canonically first, so the pair is ordered; the path to an ended branch is empty) and the two actions, of smallest length.'
- id: FR-570-global-events
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: A choice event, a send, a receive and the entry and exit of a `parallel` are the global events; the entry and exit of a `parallel` are local events of the role when at least two of its branches hold events of the role.
- id: FR-570-run-limits
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: Run limits `max_states` (events) and `max_transitions` (covering edges) default to 4096 and 262144; a run one of them stops is `failed`, `resource-incomplete`, and states the limit and its value.
- id: FR-570-horizon
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: 'An event deeper than `max_depth` (the longest chain of events ending at it) is beyond the horizon: a branch pair holding such an event is not compared, and a search that finds no counterexample with any event beyond the horizon is `inconclusive`.'
- id: FR-570-AC-4
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: 'A choice branch merges for the role only by the PA-10 rule: owner, `visible`, an earlier outcome-carrying message, or distinct first-receive message types, or pairwise equal (direction, peer, message type) first events recursing on their continuations; a send against a receive, an empty first set against a non-empty one, and differing peers or send message types do not merge (implements FR-174-AC-5).'
- id: FR-570-AC-6
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: For `Seller` owning `ship | cancel` with `ship` sending `pick` to `Warehouse`, projection onto `Warehouse` returns the counterexample (`receive pick` against end); with `cancel` sending `cancelled` to `Warehouse`, or with an earlier message between `Warehouse` and `Seller` whose payload binder the choice's `visible` fact reads, it returns the witness; an earlier `decision` message of one type in both branches that no decision fact reads returns the counterexample (ADR-036 worked example).
- id: FR-574-trace-position
  path: spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md
  role: examined
  excerpt: for a projection counterexample the index into `path_a` of the first divergent action, which is `path_a`'s last event, `path_a` being the path FR-570 orders first.
- id: FR-575-unsupported-control
  path: spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md
  role: examined
  excerpt: '`unsupported-control` | A `check`, `attempt`, `effect`, `event`, `commit` or `fence` control, which the profile''s list does not include.'
- id: FR-575-order
  path: spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md
  role: examined
  excerpt: When a protocol holds several such constructs the engine reports the first in the order roles, queues, compensations, then controls in layout order.
- id: ADR-036-PA-7
  path: spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md
  role: context_only
  excerpt: 'Choice visibility: a non-owner role distinguishes a choice''s branches through `visible`, through an earlier received message that carries the outcome (FR-053-AC-1: the choice''s guards are a function of that message''s payload), or through a divergent first receive (PA-10).'
- id: ADR-036-worked-example
  path: spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md
  role: context_only
  excerpt: 'A different repair is an earlier `Seller -> Warehouse: decision` before the choice whose payload the choice''s guards test (FR-053-AC-1); a `decision` message of one type in both branches does not distinguish them.'
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: spec-review/integrity
  check: soundness
  artifact_id: FR-570-choice
  related_ids:
  - ADR-036-PA-7
  - FR-053-AC-1
  - FR-570-AC-4
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  lines: 70-77
  excerpt: At a `choice` the engine shall classify the role as internal choice when it is the owner, as observing the outcome when it sent or received an earlier message whose payload binder the choice's decision facts read (the `visible` expressions, followed through the value nodes they reference; FR-053-AC-1; a checked package's `visible` holds expression nodes, so a role "lies in `visible`" exactly by taking part in such a message), as external choice when a peer sends a receive in every branch's first events and the message types differ across branches (PA-10).
  finding: The rule informs a role that sent or received any earlier message whose payload binder the decision facts read. PA-7 and FR-053-AC-1 need the guards to be a function of that payload. With visible (got.ok from Warehouse, got2.ok from Buyer) and guards on got2.ok, the FR text informs Warehouse though it cannot know the outcome. Require every decision fact the guards depend on to be known to the role.
- fnd: FND-002
  severity: medium
  confidence: high
  method: spec-review/integrity
  check: other
  artifact_id: FR-570-AC-6
  related_ids:
  - ADR-036-PA-7
  - ADR-036-worked-example
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  lines: '115'
  excerpt: For `Seller` owning `ship | cancel` with `ship` sending `pick` to `Warehouse`, projection onto `Warehouse` returns the counterexample (`receive pick` against end); with `cancel` sending `cancelled` to `Warehouse`, or with an earlier message between `Warehouse` and `Seller` whose payload binder the choice's `visible` fact reads, it returns the witness; an earlier `decision` message of one type in both branches that no decision fact reads returns the counterexample (ADR-036 worked example).
  finding: Decision (a) is grounded in QSpec FR-466 (visible holds the owner's expression nodes; a send binder is never eligible for the owner), so the ADR's visible = {Buyer} and Seller -> Warehouse decision repair cannot be written. But ADR-036 PA-7, PA-10 and the worked example were not amended, so the ADR now contradicts FR-570. Amend the ADR here; and AC-6 should say Warehouse sends and Seller receives, the only direction that carries the outcome.
- fnd: FND-003
  severity: medium
  confidence: medium
  method: spec-review/integrity
  check: soundness
  artifact_id: FR-570-horizon
  related_ids:
  - FR-570-AC-5
  - ADR-036-PA-11
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  lines: 97-100
  excerpt: 'An event deeper than `max_depth` (the longest chain of events ending at it) is beyond the horizon: a branch pair holding such an event is not compared, and a search that finds no counterexample with any event beyond the horizon is `inconclusive`.'
  finding: A branch pair with any event beyond max_depth is not compared, yet a counterexample from another choice is still reported as the PA-11 minimal, unique counterexample; the skipped pair can hold a shorter divergence inside the horizon. Compare cut pairs up to the horizon, or scope minimality to compared pairs.
- fnd: FND-004
  severity: low
  confidence: high
  method: spec-review/integrity
  check: ambiguous
  artifact_id: FR-570-AC-4
  related_ids:
  - FR-570-choice
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  lines: '113'
  excerpt: 'A choice branch merges for the role only by the PA-10 rule: owner, `visible`, an earlier outcome-carrying message, or distinct first-receive message types, or pairwise equal (direction, peer, message type) first events recursing on their continuations; a send against a receive, an empty first set against a non-empty one, and differing peers or send message types do not merge (implements FR-174-AC-5).'
  finding: AC-4 lists `visible` beside 'an earlier outcome-carrying message' as a merge basis, but after decision (a) the Behavior makes such a message the only way a non-owner is informed; `visible` names no separate basis.
bindings: []
```

+++

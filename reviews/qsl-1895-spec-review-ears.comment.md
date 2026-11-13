<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.29.4 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-2026111306075604 method=spec-review/ears-conformance lang=spec pr=quire-spec-language#2697 model=claude-opus-5-5 run=42c2157c-bfb4-4b50-a98f-bcc300009c3b -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | low | ambiguous | 'a peer sends a receive' is garbled |
| FND-002 | low | other | Rationale parenthetical inside the shall-statement; 'lies in visible' names nothing |

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-570-choice
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  role: examined
  excerpt: At a `choice` the engine shall classify the role as internal choice when it is the owner, as observing the outcome when it sent or received an earlier message whose payload binder the choice's decision facts read (the `visible` expressions, followed through the value nodes they reference; FR-053-AC-1; a checked package's `visible` holds expression nodes, so a role "lies in `visible`" exactly by taking part in such a message), as external choice when a peer sends a receive in every branch's first events and the message types differ across branches (PA-10).
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
findings:
- fnd: FND-001
  severity: low
  confidence: high
  method: spec-review/ears-conformance
  check: ambiguous
  artifact_id: FR-570-choice
  related_ids: []
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  lines: 76-77
  excerpt: At a `choice` the engine shall classify the role as internal choice when it is the owner, as observing the outcome when it sent or received an earlier message whose payload binder the choice's decision facts read (the `visible` expressions, followed through the value nodes they reference; FR-053-AC-1; a checked package's `visible` holds expression nodes, so a role "lies in `visible`" exactly by taking part in such a message), as external choice when a peer sends a receive in every branch's first events and the message types differ across branches (PA-10).
  finding: '''as external choice when a peer sends a receive in every branch''s first events'' is garbled; write ''when every branch''s first events hold a receive by the role from one peer, and the message types differ across branches''.'
- fnd: FND-002
  severity: low
  confidence: medium
  method: spec-review/ears-conformance
  check: other
  artifact_id: FR-570-choice
  related_ids: []
  path: spec/functional/FR-570-project-a-global-protocol-onto-one-role.md
  lines: 73-75
  excerpt: At a `choice` the engine shall classify the role as internal choice when it is the owner, as observing the outcome when it sent or received an earlier message whose payload binder the choice's decision facts read (the `visible` expressions, followed through the value nodes they reference; FR-053-AC-1; a checked package's `visible` holds expression nodes, so a role "lies in `visible`" exactly by taking part in such a message), as external choice when a peer sends a receive in every branch's first events and the message types differ across branches (PA-10).
  finding: The parenthetical justification inside the shall-statement is rationale, and 'lies in visible' no longer names anything. Move it to a note citing QSpec FR-466 'Decision facts'.
bindings: []
```

+++

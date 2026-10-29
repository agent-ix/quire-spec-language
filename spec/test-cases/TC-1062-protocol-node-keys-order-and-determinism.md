---
id: TC-1062
title: "Protocol node declarations, binders, order, keys and determinism"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: verifies
---
# TC-1062: Protocol node declarations, binders, order, keys and determinism

## Description

Verify the declaration paths, occurrences and binder levels of the protocol
nodes, recursion-group handling for the proposed cyclic-protocol case, that
every set and sorted list is canonical whatever order the source
declares it in, that only a sequence's children and the captures carry order,
that repeats collapse, that keys are the application-node preimage and stable
under reformatting, and that equal sources give equal bytes. Scope:
FR-383-AC-9 to FR-383-AC-16 and the approved Option A requirements in
FR-383-AC-24 and FR-383-AC-25.

Catches a declaration path that spells a branch label, equal-named role,
channel, template and control that collide, a binder numbered by source
position, a set written in source order, queues ordered by their channels,
cases not sorted by label, a repeated clause emitted twice, a key that moves
with whitespace or a second reference, a causal edge or a runtime subject
written into a node, and a derived causal relation that differs from the one
S3 compiled.

Fixture **W** is TC-1061's. Fixture **N**: a protocol `Names` with a role
`X`, a channel `X`, a template `X` and a control `X`. Fixture **C**: a
protocol `Flow` with a predicate-join function whose body references
`protocol Flow`, producing a cycle through the clause and control nodes; the
same package also contains an acyclic protocol `Plain`. Fixture **L** is a
100,000-node checked reference cycle, with otherwise identical variants whose
authored node order is permuted.

## Test Procedure

| # | Unit | Expected |
| --- | --- | --- |
| 1 | **N** | four nodes with four distinct `declaration.qualified_name` values: `Names, role, X`, `Names, channel, X`, `Names, compensate, X` and `Names, run, X`; no equal-name refusal |
| 2 | **W** | `Shipments`' branches `East` and `West` have declarations `OrderFlow, run, Main, Shipments, East` and `... West` (no `east` or `west` segment); a case body has no case segment; each declaring construct has one `declaration` occurrence at its region and a source-map row keyed (node id, `declaration`, 0); the queue has one `generated` occurrence and no `declaration`; a binder has one `expression` occurrence for each introducing construct, in source order |
| 3 | **W** | levels: `view` 0, `opened` 1, `order` 2, `sent` the entry count of `Requested`; a second construct that introduces a binder of equal name, level and type shares its node, with two occurrences; no binder level equals a source offset |
| 4 | **W** | a walk of the emitted JSON classifies every protocol body position as Leaf, Group, Tuple, Member or Body, and finds no nested application; every guard, constraint, key and initializer is a literal or a reference |
| 5 | **W** with the roles, channels, compensations, `visible` entries, `contracts`, cases, branches, joined labels and `related by` clauses each declared in a different order; with two channels whose order differs from their queue-key order | the same protocol bytes and `package_id` as **W** (for the second unit, the same as that unit with the channels swapped); every set is in strictly ascending key order; the queues are ordered by their own keys; cases and branches are sorted by label as UTF-8 bytes |
| 6 | **W** with two `sequence` children swapped; **W** with two captures swapped | the node key of the changed node and the clause key each change |
| 7 | **W** with `relationship A = M::R; relationship B = M::R;`, an equal `related by` written twice on a `send`, an equal `visible` entry written twice and `requires temporal OrderClosureBounded;` written twice | the same protocol bytes as **W** without the repeats |
| 8 | **W**; **W** reformatted; **W** with a comment; **W** with a second `protocol OrderFlow` reference in a user expression | every protocol node key equals the RFC 8785 bytes of its `quire.application-node/v1` preimage computed independently; the keys and the `package_id` are equal across the four units |
| 9 | **W** with a role renamed; a reference retargeted; one literal changed; the capacity changed to 5; the loss selector changed; a join policy changed; the loop-proof variant changed; `scheduling adversarial` added or removed | each changes the changed node's key and the clause key, and no unrelated node's key |
| 10 | **W** emitted twice; **W** in a unit with an unrelated protocol declared before it and one after it | the same protocol nodes, bytes and `package_id` for **W** each time |
| 11 | **W** | no node holds a causal edge, relationship alias, runtime subject, instance, ordinal, delivery or clock observation or an `internal`/`external` member; the (source port, target port) pairs QSpec FR-464's rules derive from the emitted nodes equal the compiled edge relation of the checked clause, back edges of the `repeat` included |
| 12 | **C** | the complete cyclic reference component shares one `recursion_group`; each member carries the FR-322 `{size, ordinal}` coordinate and in-component edges use `group_reference`; QSL's I2 reader admits the package. `Plain` has no group, `recursion` `null`, and unchanged acyclic key inputs |
| 13 | **L** | SCC discovery and membership assignment complete iteratively in O(V + E); the emitted package reads back successfully within the caller's supplied shared `s3.work_units` budget, without native stack overflow; the implementation hashes the completed group once and does not merge groups quadratically |
| 14 | **L** with zero work units; **L** with exactly the measured required work units; **L** with one fewer unit | zero and one-short refuse with the existing typed limit refusal naming `s3.work_units`; exact budget succeeds; no independent recursion depth, group-size, node-count or group-count cap is applied |
| 15 | the two authored-order variants of **L** | the group identity, member ordinals, node bytes and package identity are identical; no global lock or serialized package-emission section is used |

## Expected Results

- Each row asserts the stated typed facts and the byte equality or inequality it
  names, never message text.
- Row 8's independent key is computed by the test from the preimage fields,
  not read from the emitter.

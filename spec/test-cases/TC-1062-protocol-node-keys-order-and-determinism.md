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
nodes, that every set and sorted list is canonical whatever order the source
declares it in, that only a sequence's children and the captures carry order,
that repeats collapse, that keys are the application-node digest and stable
under reformatting, and that equal sources give equal bytes. Scope:
FR-383-AC-9 to FR-383-AC-16.

Catches a declaration path that spells a branch label, equal-named role,
channel, template and control that collide, a binder numbered by source
position, a set written in source order, queues ordered by their channels,
cases not sorted by label, a repeated clause emitted twice, a key that moves
with whitespace or a second reference, a causal edge or a runtime subject
written into a node, and a derived causal relation that differs from the one
S3 compiled.

Fixture **W** is TC-1061's. Fixture **N**: a protocol `Names` with a role
`X`, a channel `X`, a template `X` and a control `X`.

## Test Procedure

| # | Unit | Expected |
| --- | --- | --- |
| 1 | **N** | four nodes with four distinct `declaration.qualified_name` values: `Names, role, X`, `Names, channel, X`, `Names, compensate, X` and `Names, run, X`; no equal-name refusal |
| 2 | **W** | `Shipments`' branches `East` and `West` have declarations `OrderFlow, run, Main, Shipments, East` and `... West` (no `east` or `west` segment); a case body has no case segment; each declaring construct has one `declaration` occurrence at its region and a source-map row keyed (node id, `declaration`, 0); the queue has one `generated` occurrence and no `declaration`; a binder has one `expression` occurrence for each introducing construct, in source order |
| 3 | **W** | levels: `view` 0, `opened` 1, `order` 2, `sent` the entry count of `Requested`; a second construct that introduces a binder of equal name, level and type shares its node, with two occurrences; no binder level equals a source offset |
| 4 | **W** | a walk of the emitted JSON classifies every protocol body position as Leaf, Group, Tuple, Member or Body, and finds no nested application; every guard, constraint, key and initializer is a literal or a reference |
| 5 | **W** with the roles, channels, compensations, `visible` entries, `contracts`, cases, branches, joined labels and `related by` clauses each declared in a different order; with two channels whose order differs from their queue-digest order | the same protocol bytes and `package_id` as **W** (for the second unit, the same as that unit with the channels swapped); every set is in strictly ascending digest order; the queues are ordered by their own digests; cases and branches are sorted by label as UTF-8 bytes |
| 6 | **W** with two `sequence` children swapped; **W** with two captures swapped | the node key of the changed node, the clause key and the `package_id` each change |
| 7 | **W** with `relationship A = M::R; relationship B = M::R;`, an equal `related by` written twice on a `send`, an equal `visible` entry written twice and `requires temporal OrderClosureBounded;` written twice | the same protocol bytes as **W** without the repeats |
| 8 | **W**; **W** reformatted; **W** with a comment; **W** with a second `protocol OrderFlow` reference in a user expression | every protocol node key equals the SHA-256 of the RFC 8785 bytes of its `quire.application-node/v1` preimage computed independently; the keys and the `package_id` are equal across the four units |
| 9 | **W** with a role renamed; a reference retargeted; one literal changed; the capacity changed to 5; the loss selector changed; a join policy changed; the loop-proof variant changed; `scheduling adversarial` added or removed | each changes the changed node's key, the clause key and the `package_id`, and no unrelated node's key |
| 10 | **W** emitted twice; **W** in a unit with an unrelated protocol declared before it and one after it | the same protocol nodes, bytes and `package_id` for **W** each time |
| 11 | **W** | no node holds a causal edge, relationship alias, runtime subject, instance, ordinal, delivery or clock observation or an `internal`/`external` member; the (source port, target port) pairs QSpec FR-464's rules derive from the emitted nodes equal the compiled edge relation of the checked clause, back edges of the `repeat` included |

## Expected Results

- Each row asserts the stated typed facts and the byte equality or inequality it
  names, never message text.
- Row 8's independent digest is computed by the test from the preimage fields,
  not read from the emitter.

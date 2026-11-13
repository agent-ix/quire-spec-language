---
id: TC-1410
title: "Projection onto a role returns a sound local graph or a minimal counterexample"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-570
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-174
    type: references
---
# TC-1410: Projection onto a role returns a sound local graph or a minimal counterexample

## Description

Verify the projection result's graph, correspondence, choice and merge rule,
counterexample and non-conclusive paths over the `bounded-static` profile.

Scope: FR-570-AC-1 to FR-570-AC-8; QSpec FR-174-AC-1 to AC-5.

## Test Procedure

1. Project each role of a three-role protocol with a `parallel ... join all`
   region and two FIFO receives on one channel and key; compare each local
   graph with the global event structure through the correspondence table.
2. Count local sends and receives against global communications for every
   role of step 1.
3. Project the ADR-036 worked example onto `Warehouse` in its three
   variants (pick against nothing; `cancelled` message; earlier `decision`
   message whose payload the guards test) and in the variant with a `decision`
   message of one type in both branches.
4. Project a protocol with two failing choices onto one role.
5. Project a protocol whose `first` sets are a send against a receive, an
   empty set against a non-empty one, and two sends with differing message
   types or peers.
6. Project onto `Warehouse` a protocol whose `ship` branch holds a nested
   choice of two receives and whose `cancel` branch holds one equal receive,
   then the same with `cancel` empty.
7. Project with `max_depth` smaller than the protocol needs, and a protocol
   with a construct outside the profile.

Tag the tests `#[trace("TC-1410", "FR-570-AC-n")]`.

## Expected Results

- Step 1: events and causal order equal the global structure restricted to
  the role through the table; no interleaving is enumerated; the two receives
  keep their global order.
- Step 2: one send and one receive per communication; the table is total.
- Step 3: counterexample (`receive pick` against end, owner `Seller`); then
  witness; witness; counterexample.
- Step 4: the counterexample names the choice the tie-break orders first, of
  smallest length.
- Step 5: none merges; each returns a counterexample and no graph.
- Step 6: the witness, then the counterexample.
- Step 7: a non-conclusive result with no graph in both cases.

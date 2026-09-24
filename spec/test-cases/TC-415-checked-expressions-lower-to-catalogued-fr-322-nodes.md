---
id: TC-415
title: "Each checked Value expression lowers to its FR-322 node with its catalogued operation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
---
# TC-415: Each checked Value expression lowers to its FR-322 node with its catalogued operation

## Description

Verify the FR-093 lowering: one node per checked expression, shared by equal
content, with each source occurrence recorded; each application's operator,
semantic form, operation identity, member, mode, laws, leaves and arguments
per the lowering table; FR-149 classification of conversions; binder levels;
the text-leaf list of a structural comparison, recursive composites
included; and the refusal of an operation whose law the lock evidence does
not supply.

This catches a lowering that inlines operands (losing occurrences), a
`result_type` inferred rather than taken from the checked type, a wrong
catalog identity (for example `numeric.convert` for a narrowing), a law
invented from a constant, a `NodeKind` added without a lowering arm, a leaf
walk that runs a recursive composite to the depth limit, and an optional
field's leaf path without `inner`.

Scope: FR-093-AC-1 to FR-093-AC-6, FR-093-AC-8, FR-093-AC-10,
FR-093-AC-11, FR-093-AC-14, FR-093-AC-15, FR-093-CON-1.

## Test Procedure

Every fixture unit starts with the complete-V1 header and one profile
selection whose alias is `v`, and is checked under owner (`a`, `u`).

1. Check `function t using v(): Boolean pure { if true then true else true }`.
   List the checked graph's nodes, the conditional's arguments and the
   literal's occurrences.
2. Check `both`, `nb` and `h` (FR-092-AC-4, FR-065-AC-8). Read the keys of
   the nodes of `a and b`, `both(a, true)` and `let y = a in y`, and whether
   any node has a `Local` read as its own body.
3. For each row of FR-093's application table, check a fixture function
   whose body holds that checked node. Read its node's `operator`,
   `semantic_form`, `operation`, `result_type` and argument shape, for every
   row except `Attribute`, `AllInstances`, `Lookup`, `Dispatch` and `Pre`.
   Lower the checked `Attribute`, `AllInstances`, `Lookup` and `Dispatch`
   nodes of FR-093-AC-8's fixtures: functions over `p: Population<M::Order>[3]`
   and `r: Reference<M::Order>`, and a clause function holding a dispatched
   call (QSpec FR-151, QSpec TC-196 D06). Read the same members.
4. Check `c1`, `c2`, `c3` and `fm` of FR-093-AC-4, and read their body
   nodes.
5. Check `q` of FR-093-AC-5 and read the levels of `x`, `y` and `z` and the
   `forall` argument list.
6. Check `function te using v(p: Text[0, 8; nfc], r: Text[0, 8; nfc]): Boolean pure { p = r }`
   with lock evidence that supplies the text-profile definition, and again
   with lock evidence that does not.
7. Scan the lowering `match` over `NodeKind` for a `_` arm.
8. With lock evidence that selects the text definition FR-093's Recursive
   text-leaf vectors name, check `Node`, `A` and `B`, and `eq`, `has`, `eqa`
   and `eqo` of those vectors. Read the preimage bytes and keys of the
   `a = b`, `contains(s, b)`, `x = y` and `a = b` nodes, and of the type,
   group and parameter nodes they name. Check again with `B` declared before
   `A`.
9. Check structural equality over `R`, `S`, `W`, `Tree2` and `Two` of
   FR-093-AC-11, and structural equality and `contains` over FR-092's
   `List`, and read each operation's `leaves`. Check `eq` of step 8 with
   lock evidence that supplies no text-profile definition, and again with a
   node limit that admits every node of its package but not also its two
   leaves.
10. On a spawned thread with a 2 MiB stack, check a function body of 127
    nested `a and (…)` and one of 128 at the default limits. For each of
    these forms, check the deepest nesting the default limits admit and one
    level deeper: `a and (…)`, `(…) and a`, `if a then (…) else false`,
    `if (…) then a else a`, `let bN = a in (…)`, `not (…)`, `g(…)`,
    `(x < x) = (…)`, a guard `if ((…) and x < 1) then a else a`,
    `x + (…)`, `(…) + x`, `-(…)`, `h(x * (…))` narrowed to `Int[0, 9]`, a
    record literal nested in its optional field, an unguarded
    `value(….next)` chain, nested `forall`, `sum<Total>(vN in s: …)`,
    `count<Total>(vN in s: … = 0)`, `fold<Total>(accN, vN in s: …,
    identity: 0)`, `map(vN in …: vN)`, `filter(vN in …: true)`,
    `contains(bs, …)`, `convert<Int[0, 9]>(…)`, a sequence literal
    `gs(sequence[…])`, a tuple literal `tv(T(…))`, and over the FR-094
    `acme/orders` model a `lookup<M::Order>(p, …) absent undefined` chain,
    `deref(…).total` and the clause `….size() >= 0` over that chain,
    `size(allInstances<M::Order>(p)) + (…)`, the clause
    `r.scaled(r.scaled(…)) >= 0` nesting dispatch arguments, and
    `Order.scaled`'s precondition `n + (…) >= 0`, which the dispatch bridge
    renames into `Sub.scaled`'s effective precondition. Check each form nested 1,000
    deep at the default limits, at the maximum depth with node, input-byte
    and work limits unlimited, and at a depth limit of 16. Check a
    postcondition `pre(…)` over 1,000 nested `a and (…)`, and one over
    1,000 nested `let`s around `pre(a)`.
11. With the alias `Total = Integer` and parameters `a: Boolean`,
    `x: Int[0, 9]`, `s: Sequence<Int[0, 9]>[0, 5]` and
    `o: Option<Int[0, 9]>`, read `v` after each of `let v = x in v`,
    `count<Total>(v in s: v < 5)`, `sum<Total>(v in s: v)`,
    `forall(v in s: v < 5)` and
    `fold<Total>(acc, v in s: acc + v, identity: 0)`, and read `acc` after
    that `fold`. Check each of those forms beside a copy of itself. Lower
    `let v = x in ((let w = x in w) + (let u = x in v + u))`. Check
    `if a then value(o) else 0`, `if a then 0 else value(o)` and
    `if present(o) then value(o) else 0`.

Tag the tests `#[trace("FR-093-AC-n", "TC-415")]` with the AC each backs.

## Expected Results

- Step 1: one literal node and one conditional node, as FR-093-AC-1 states,
  with three literal occurrences, ordinals 0 to 2.
- Step 2: keys E1, E2 and E3; no node for a `Local` read.
- Step 3: each node matches its row, and its `result_type` is the type node
  of the checked node's `value_type`.
- Step 4: `c1` has no convert node; `c2` gives `quire.op.numeric.narrow`
  naming `Int[0, 9]`; `c3` gives `quire.op.numeric.convert`; `fm`'s body
  is one `quire.op.collection.flat_map` node with no map node.
- Step 5: levels 1, 2 and 1; the `forall` arguments are as FR-093-AC-5 gives.
- Step 6: the first carries law `text_profile` with the supplied
  `DefinitionRef` and mode `nfc`; the second refuses with
  `missing_declaration`/`missing-selection` naming role `text_profile` and
  yields no node.
- Step 7: no `_` arm.
- Step 8: every check succeeds; the four nodes key to E14 to E17 with the
  vectors' preimage bytes, the others to T13, T14, G16 to G21, S4, S5 and
  P10 to P16; declaring `B` first gives the same keys.
- Step 9: `R` and `S` each give one leaf at `field:t`, `inner`, and `W` one
  at `field:t`; `Tree2` and `Two` give the lists FR-093-AC-11 states;
  `List` gives `[]` for both operations; `eq` refuses with
  `missing_declaration`/`missing-selection` naming role `text_profile`, and
  under the node limit with `stage_limit_exceeded`/`node-count-exceeded`
  at that limit, each time yielding no node.
- Step 10: 127 levels check and 128 stop with
  `stage_limit_exceeded`/`nesting-depth-exceeded`, bound 128.
  Each form checks at its deepest admitted nesting, except the unguarded
  `value` chain, which refuses there as an unproved presence, and stops with
  `nesting-depth-exceeded` one level deeper. Every 1,000-deep form refuses
  with `nesting-depth-exceeded` at the depth limit in force. The first `pre` refuses as a forbidden
  pre-read and the second on the depth limit. No check aborts.
- Step 11: each read after its binder's body refuses with
  `missing_declaration`/`missing-name` naming the name read. Each form
  beside a copy of itself checks. `v` lowers at level 4, `w` and `u` at
  level 5, and `v + u` reads `v`'s and `u`'s parameter nodes. The two
  unguarded `value(o)` bodies each refuse with
  `undefined_expression`/`unproved-presence` alone; the guarded one checks.

## Status

Implemented on the QSL-156 slice A4b branch, pending merge. The tests back
steps 1 to 7 except step 4's `fm`: the A4b `flatMap` test flat-maps a flat
`s` over itself. Step 6's first half needs the QSpec
`complete-value-lock.json` accessor (ADR-011 §2.4). Steps 8 and 9 are
specified under QSL-212 and unbacked: on the A4b branch the leaf walk runs
`Node` to the depth limit and walks an optional field without `inner`.
The tests back steps 10 and 11 (QSL-228): before it, a debug build
aborted at 20 nested `a and (…)`.
Remaining work: QSL-156 A4b.

The expected `stage_limit_exceeded` outcome is ADR-013 §7 slice S-5b's
(QSL-160, FR-096). Until S-5b lands, the tests observe the same limit as
`ResourceExhausted`, and the code and outcome assertions move with S-5b.

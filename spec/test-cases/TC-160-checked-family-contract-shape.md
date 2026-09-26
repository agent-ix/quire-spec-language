---
id: TC-160
title: "Every family implements the six-part checked contract with no bypass"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-160: Every family implements the six-part checked contract with no bypass

## Description

Verify that the shared checked-family contract's three separately-omittable
parts (checked input, requirements, evaluate) each fail to compile
when omitted, that identity is content-addressed and occurrence keys are
distinct from identity (verifying identity and provenance behaviorally,
since neither is a separate trait item to omit), that the typing context has
no side door, that requirements are a pure function of a checked node, that
a reached limit is distinct from both a refusal and `Incomplete`, that
`Incomplete` is returned only by `evaluate`, never by `check` or the S4 v2
emitter, that a nesting-depth limit is
the proximate cause of a deeply-nested form's refusal (not the form's
absolute size or the host's available stack), that the S4 v2 emitter is
all-or-nothing over the nodes a node names, and that every evaluation hook reads only checked input.
The `Relation` half of FR-062-AC-6 (no `Relation` hook and no `Relation`
S6a input) is verified by TC-385.
Scope: FR-062-AC-1 through FR-062-AC-7, FR-062-AC-9 and FR-062-AC-13, and
FR-057-AC-10's value-function row.

## Test Procedure

1. Compile a minimal family implementation against the contract that in turn
   omits, one at a time: the checked-input (typing context) parameter type
   on `check`; the `requirements` method; the `evaluate` hook (for a family
   other than `Relation`). Confirm each omission fails to
   compile. Identity and provenance are not tested this way: they are
   properties of the `Checked` node type and the package's source map, not
   separate trait items, and are instead exercised by steps 2 and 3 below.
2. Parse two structurally identical forms into the same package and check
   both; also arrange for the same node to occur twice in the source (for
   example, a repeated identical sub-expression). Read the identity of each
   checked node and the occurrence keys the source map assigns.
3. Reorder the two source occurrences from step 2 and re-check; compare
   identities and ordinals before and after.
4. Construct two typing contexts from the same resolved declarations. In one,
   mutate only the meter, diagnostic sink and scope stack while checking a
   form; check the same form through the other, unmutated, context.
   Compare the two checked outputs and the mutated context's own observable
   state.
5. Call the pure requirements function on the checked function
   declaration `b and c` over Boolean parameters, and on the one whose body
   is `x + y` over `Int[0, 9]` parameters. Call it twice on the same checked
   declaration.
6. Configure a work budget small enough that checking a form reaches it;
   check the form and inspect the returned outcome's variant, confirming it
   is a `Limit` outcome, not `Incomplete` and not a refusal. Separately, run
   a family's `evaluate` hook on a checked node under a meter small enough to
   exhaust mid-evaluation and confirm it returns `Incomplete`. Run `check`
   and `qsl_package::emit_checked` across the same fixture set and confirm
   neither returns `Incomplete`.
7. Construct a fixture nested to depth D (for example, D levels of nested
   function application). Check it with the nesting-depth limit configured
   to D-1. Check the identical fixture again with the limit configured to
   D, one greater and nothing else changed.
8. Check a package holding `q(a: Length): Boolean { a * a == a * a }`,
   whose `metre^2` compound unit node names the `metre` unit node lowering
   does not build, and `t`, which names no omitted node. Emit it with
   `emit_checked` and read the bytes back through QSL's I2 read. Emit the
   same package again through `emit_package` with a region conversion that
   places no occurrence. The fixture's omission relies on lowering not
   building the `metre` unit node; when lowering builds it, this step
   switches to another form the emitter omits.
9. Check every family's evaluation hook for display-string reads. A
   "display string" is rendered text (`Display` or `Debug` output,
   diagnostic text, source spelling); a declared name carried on a checked
   node is checked input. (a) Scan the shipped dependencies and the
   evaluator's source for a path to source text or a call that renders text
   or reads a string-shaped accessor of the checked package. (b) Rename
   every declared name in one package and evaluate it before and after,
   completed and meter-stopped.
10. Check each unit RR-1 to RR-17 of FR-062 "Requirement records of a
    value function" and read `CheckedGraph::requirements`, and for RR-5
    also `CheckedPackage::graph().requirements()`. Check RR-5 a second
    time. For RR-15, also check the unit with its two `let` operands
    swapped. Also check `if flag then 0 else if n >= 0 then n + 1 else 0`,
    `n < 0 or n - 1 >= 0` and `n > 0 implies n - 1 >= 0` over
    `n: Integer, flag: Boolean`; `size(filter(v in s: v > 0)) + 1` over
    `s: Sequence<Integer>[0, 5]`; and a `fold`, a `reduce` and a `flatMap`
    whose step reads its binders.
11. Over RR-8's lowered graph and hand-built occurrence maps, key: a claim
    site whose location holds its application's `expression` occurrence; a
    site whose application has only a `generated` occurrence there; a site
    at a location holding no occurrence; no claim, beside a scalar
    application occurrence in a function body; and two sites of the one `+`
    node at two locations, in both claim orders.

## Expected Results

- Step 1: each of the three omissions fails to compile; no partial
  implementation is accepted.
- Steps 2 and 3: the two structurally identical forms mint one shared
  identity; the two occurrences of one node get distinct occurrence keys
  (identity, role, ordinal) that differ only in ordinal after reordering, and
  the shared identity is unchanged by the reorder.
- Step 4: the two checked outputs are identical; only the meter, diagnostic
  sink and scope stack mutations are observable, and the unmutated context
  shows none of them.
- Step 5: `b and c` yields no `Requirements` value; `x + y` yields exactly
  one, `value-validity`, at its `+` application, equal across both calls.
- Step 6: the reached-limit check returns a `Limit` outcome naming the
  work-budget kind, distinct in type from a checked node, a refusal and
  `Incomplete`; the exhausted `evaluate` call returns `Incomplete`; `check`
  and `emit_checked` return `Incomplete` in no observed case.
- Step 7: with the limit at D-1, `check` returns a `Limit` outcome naming
  the nesting-depth limit; with the limit at D on the identical fixture,
  `check` does not return that outcome. Varying only the limit by one flips
  the result, showing the limit value, not the fixture's absolute size, is
  the proximate cause.
- Step 8: the compound unit node is omitted with `NamesAbsentNode`; `q`'s
  declaration node and every node on its path to the compound unit are
  omitted with `NamesOmittedNode`; `t` and its body are written; the I2
  read is Verified and exports `t` and not `q`. The second emission returns
  `EmitRefusal::UnlocatedOccurrence` and no bytes.
- Step 9: (a) the scan finds no such path or call, and flags each forbidden
  form in a synthetic violating source; (b) the outcome, loss count and
  metered work are unchanged by the renaming, showing no display string was
  read to decide the result.
- Step 10: each unit's map holds exactly the records FR-062's fixture
  table lists for it and no other. Every record is `value-validity` and
  keyed by its application node's `expression` occurrence at its own site,
  with the listed extent, result bound (as its type node) and path
  condition (each guard as its node's occurrence key and required
  outcome). In particular: RR-2 has no record at the narrow; RR-10 has no
  record; RR-12's `+` is `Unbounded` at `n` with guard `n >= 0 and n < 10`
  true; RR-13's `/` has guard `y != 0` true; RR-14's roots are keyed by
  the binder `v`, not by `s`; RR-17's one record is at the body's
  occurrence, not the measure's. Where one node has two records (RR-5,
  RR-15, RR-16), the keys differ only in ordinal in source order, and each
  record carries its own occurrence's extent, result bound and guards: in
  RR-15 the `Bounded` extent is at the `t * 2` occurrence inside the
  `x + 1` binding and the `Unbounded` one inside the `n + 1` binding, in
  both orders of the operands; in RR-16 `Int[0, 10]` is at ordinal 0 and
  `Int[0, 20]` at ordinal 1. RR-5's second check gives an equal map, and
  `CheckedPackage::graph().requirements()` equals the S3 map. In the
  guard units, `n >= 0` has guard `flag` false and `n + 1` has guards
  `flag` false then `n >= 0` true; each application right of `or` has
  guard `n < 0` false, and right of `implies` guard `n > 0` true. The
  `size` unit's `>` is unbounded at `v`, and its `+` only at `s`'s
  element. The `fold` and `reduce` steps are unbounded at both the
  accumulator and the element, and the `flatMap` step at its binders.
- Step 11: the paired site keys to its occurrence; the `generated`-only
  site, the site with no occurrence and the unclaimed application each
  return `KeyFault::UnkeyableRequirements` rather than an omission from the
  map; the two sites of one node key to that node's two occurrences by
  location, differing only in ordinal, whichever order the claims come
  in.

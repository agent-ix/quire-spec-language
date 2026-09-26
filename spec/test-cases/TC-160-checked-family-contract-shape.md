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

Verify that the shared checked-family contract's four separately-omittable
parts (checked input, requirements, package, evaluate) each fail to compile
when omitted, that identity is content-addressed and occurrence keys are
distinct from identity (verifying identity and provenance behaviorally,
since neither is a separate trait item to omit), that the typing context has
no side door, that requirements are a pure function of a checked node, that
a reached limit is distinct from both a refusal and `Incomplete`, that
`Incomplete` is returned only by `evaluate`, that a nesting-depth limit is
the proximate cause of a deeply-nested form's refusal (not the form's
absolute size or the host's available stack), that `package` is
all-or-nothing, and that every evaluation hook reads only checked input.
The `Relation` half of FR-062-AC-6 (no `Relation` hook and no `Relation`
S6a input) is verified by TC-385.
Scope: FR-062-AC-1 through FR-062-AC-7, FR-062-AC-9 and FR-062-AC-13, and
FR-057-AC-10's value-function row.

## Test Procedure

1. Compile a minimal family implementation against the contract that in turn
   omits, one at a time: the checked-input (typing context) parameter type
   on `check`; the `requirements` method; the `package` hook; the `evaluate`
   hook (for a family other than `Relation`). Confirm each omission fails to
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
   and `package` across the same fixture set and confirm neither ever
   returns `Incomplete`.
7. Construct a fixture nested to depth D (for example, D levels of nested
   function application). Check it with the nesting-depth limit configured
   to D-1. Check the identical fixture again with the limit configured to
   D, one greater and nothing else changed.
8. Given a checked item requiring more than one v2 node, inject a fault
   partway through `package`'s emission for that item (after the first node
   is written, before the last); read whatever v2 bytes resulted.
9. Instrument every family's evaluation hook with a test double that
   panics if a CST, token or display string is touched, then invoke each on
   a checked node.
10. Check each of these units and read `CheckedGraph::requirements` (and
    `CheckedPackage::graph().requirements()` for one of them); parameters are
    `Int[0, 9]` unless stated: bodies `-z`; `x + y`; `x = y`;
    `(x + 1) * (x + 1)`; `n + 1` over `n: Integer`;
    `let t = x + 1 in t * 2`; `1 + 1`; `b and c` over Boolean parameters.
    Check the `(x + 1) * (x + 1)` unit a second time.
11. Over a hand-built `OccurrenceMap` (this crate has no real clause syntax
    yet, FR-088-CON-1, so this is exercised the same way `check::identity`'s
    own clause-identity mechanism test is): key two distinct identities,
    each with one recorded occurrence and one requested `Requirements`; key
    one identity with a requested `Requirements` and no recorded occurrence;
    key an index whose `Requirements` is requested but whose identity is
    `None`; key one identity shared by two indices, each with its own
    requested `Requirements`, over two recorded occurrences of that
    identity.

## Expected Results

- Step 1: each of the four omissions fails to compile; no partial
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
  and `package` return `Incomplete` in no observed case.
- Step 7: with the limit at D-1, `check` returns a `Limit` outcome naming
  the nesting-depth limit; with the limit at D on the identical fixture,
  `check` does not return that outcome. Varying only the limit by one flips
  the result, showing the limit value, not the fixture's absolute size, is
  the proximate cause.
- Step 8: the result is either a refusal with no v2 bytes for the item, or a
  complete v2 node set for the item; no reading finds a partial node set (for
  example, a declaration node with no body).
- Step 9: every family's evaluation hook completes without the test double
  panicking, showing no CST, token or display string was read.
- Step 10: every record is `value-validity` and keyed by an application
  node's `expression` occurrence. `-z`: one record at the
  `quire.op.integer.negate` node, `Bounded`. `x + y`: one at
  `quire.op.integer.add`, `Bounded`. `x = y`: one at `quire.op.integer.eq`,
  `Bounded`. `(x + 1) * (x + 1)`: three records, two at the one `+` node
  with ordinals 0 and 1 and one at the `*` node, and the second check gives
  an equal map. `n + 1`: one record, `Unbounded` with exactly one `Integer`
  domain keyed by `n`'s parameter node and the empty path. `let t = x + 1
  in t * 2`: the `*` record is `Bounded`. `1 + 1`: one `Bounded` record.
  `b and c`: no record. `CheckedPackage::graph().requirements()` equals
  the S3 map.
- Step 11: the two distinct identities each key to their own occurrence key;
  the identity with no recorded occurrence, and the `None` identity, each
  return `KeyFault::UnkeyableRequirements` rather than an omission from the
  map; the one shared identity's two indices key to two distinct occurrence
  keys, one per recorded occurrence.

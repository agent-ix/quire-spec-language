---
id: TC-160
title: "Every family implements the six-part checked contract with no bypass"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
---
# TC-160: Every family implements the six-part checked contract with no bypass

## Description

Verify that the shared checked-family contract exposes exactly its six parts
(identity, provenance, checked input, requirements, structured outcome, stage
hooks), that identity is content-addressed and occurrence keys are distinct
from identity, that the typing context has no side door, that requirements
are a pure function of a checked node, that a work-budget limit is distinct
from a refusal, and that the `Relation` family's evaluation hook returns a
named non-native-evaluability refusal rather than a panic or a silent
success. Scope: FR-062-AC-1 through FR-062-AC-6.

## Test Procedure

1. Compile a minimal family implementation that omits one of the six
   contract parts and confirm the build fails; repeat once per part.
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
5. Call the pure requirements function on a checked node from a claim form
   with no FR-057 kind, and on one from a claim form with a kind. Call it
   twice on the same checked node.
6. Configure a work budget small enough that checking a form exhausts it;
   check the form and inspect the returned outcome's variant. Separately,
   check a form with an actual semantic error and inspect the refusal's
   `catalog_code()` result.
7. Invoke the `Relation` family's evaluation hook on a checked `Relation`
   node built only from checked input. Instrument every other family's
   evaluation hook with a test double that panics if a CST, token or display
   string is touched, then invoke each on a checked node.

## Expected Results

- Step 1: each omission fails to compile; no partial implementation is
  accepted.
- Steps 2 and 3: the two structurally identical forms mint one shared
  identity; the two occurrences of one node get distinct occurrence keys
  (identity, role, ordinal) that differ only in ordinal after reordering, and
  the shared identity is unchanged by the reorder.
- Step 4: the two checked outputs are identical; only the meter, diagnostic
  sink and scope stack mutations are observable, and the unmutated context
  shows none of them.
- Step 5: the no-kind claim form yields no `Requirements` value; the
  kind-bearing claim form yields exactly one, equal across both calls.
- Step 6: the exhausted-budget check returns a limit outcome naming the
  work-budget kind, distinct in type from both a checked node and a refusal;
  the semantic-error check returns a refusal whose `catalog_code()` result is
  a stable catalog code, produced by an exhaustive mapping with no fallback
  arm (confirmed by inspecting that the mapping's match has no `_` arm).
- Step 7: the `Relation` invocation returns a named refusal stating
  non-native evaluability, never a panic and never a successful evaluated
  result; every other family's evaluation hook completes without the test
  double panicking, showing no CST, token or display string was read.

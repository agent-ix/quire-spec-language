---
id: TC-225
title: "A dispatch family deeper than the bound refuses instead of reporting a false unique winner"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: verifies
---
# TC-225: A dispatch family deeper than the bound refuses instead of reporting a false unique winner

## Description

Verify that a redefinition-family walk exceeding the bound returns a typed
incomplete result naming the bound, with neither a linked table nor an
ambiguity result reported, and that a family at exactly the bound links
successfully. Scope: FR-083-AC-4.

Catches an implementation that stops enumerating the family at the bound and
reports whatever partial family it has gathered so far as if it were
complete — which could easily produce a spuriously "unique" winner (the
walk simply never reached the competing redefiner that would have made it
ambiguous), passing any test that checks only "some candidate was selected."

## Test Procedure

1. Declare a redefinition family whose `redefines` chain is one link longer
   than the linker's depth bound.
2. Declare the family so that, if the walk continued past the bound, it
   would discover a second undominated candidate for one subtype (making
   that subtype genuinely ambiguous).
3. Link the dispatch table for the family.
4. Declare a second family at exactly the bound's depth with an
   unambiguous, correctly resolving unique winner, and link it.

## Expected Results

Step 3 returns the typed incomplete result naming the bound, not a linked
table and not an ambiguity result. Step 4 links successfully with the
correct unique winner. A mutant that truncates the walk at the bound and
reports the truncated family as resolved produces a linked table in step 3
naming a "unique" winner that is only unique because the walk never found
the competing candidate, failing the result-type assertion.

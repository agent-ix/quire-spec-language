---
id: TC-225
title: "A dispatch family walk that exhausts its edge limit stops incomplete instead of reporting a false unique winner"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: verifies
---
# TC-225: A dispatch family walk that exhausts its edge limit stops incomplete instead of reporting a false unique winner

## Description

Verify that a redefinition-family walk whose `family_steps` charge is denied
stops `Incomplete` naming the limit, its value and the count reached, with
neither a linked table nor an ambiguity result, that the same family links
once the limit fits, and that a deep family links with no depth refusal.
Scope: FR-083-AC-4.

Catches an implementation that stops enumerating the family at the limit and
reports whatever partial family it has gathered so far as if it were
complete — which could produce a spuriously "unique" winner (the walk never
reached the competing redefiner that would have made it ambiguous), passing
any test that checks only "some candidate was selected."

## Test Procedure

1. Declare a redefinition family whose walks follow `n` `redefines` edges in
   total, where the edge `n` leads to a second undominated candidate for one
   subtype (making that subtype genuinely ambiguous).
2. Link the dispatch table with `family_steps` at `n − 1`.
3. Link it again with `family_steps` at `n`.
4. On a thread with a 512 KiB stack, declare a 10,000-long `redefines` chain
   with one unique winner and link it with `family_steps` raised to fit it.

Tag the tests `#[trace("TC-225", "FR-083-AC-4")]`.

## Expected Results

Step 2 stops `Incomplete` naming `family_steps`, bound `n − 1` and count
`n`, with no linked table and no ambiguity result. Step 3 reports the
ambiguity. Step 4 links with the correct unique winner, and no outcome names
a depth. A mutant that truncates the walk at the limit and reports the
truncated family as resolved produces a linked table in step 2 naming a
"unique" winner, failing the result-type assertion.

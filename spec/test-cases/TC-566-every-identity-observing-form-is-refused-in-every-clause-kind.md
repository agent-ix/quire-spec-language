---
id: TC-566
title: "Every identity-observing form is refused in every clause kind, and transparent forms check"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-150
    type: verifies
---
# TC-566: Every identity-observing form is refused in every clause kind, and transparent forms check

## Description

Verify the refusal in invariants, temporal atoms, `terminal when` predicates and state constraints, the source-order report of several refusals, the per-fold cause in a unit that reads an annotated and an unannotated population, that a fold in a function body is FR-145's refusal alone, and that transparent uses of the population still check.

Scope: FR-150-AC-3, FR-150-AC-4 and FR-150-AC-6 to FR-150-AC-8.

## Test Procedure

Fixtures: The annotated ConfigVersion unit of TC-565.

1. Add, one at a time: an invariant comparing `self` to a reference literal for key `a`; a temporal atom converting `c` to text; a `terminal when` predicate with `reduce` over a `Bag` of references; a state constraint with a `fold` whose step is outside FR-145's catalog (it chooses an element). Then add two of them together.
2. Add the invariant `forall p in config_history: p.parent = self implies p != self`, the atom `holds(c.parent = a.parent)` and a `fold<Tally>(acc, x in s: acc + 1, identity: 0)` over a `Set<Int[0, 3]>` field `s`, with `type Tally = Integer;`.
3. Add a population `drafts` of type `Draft` without the annotation, fields
   `peers: Set<Reference<ConfigVersion>>` and `drafts: Set<Reference<Draft>>`,
   and two post clauses, the first folding `self.peers` and the second folding
   `self.drafts`, each with a step that chooses an element.
4. Add a function `pick(s: Set<Reference<ConfigVersion>>): Reference<ConfigVersion>`
   whose body folds `s` with a step that chooses an element, then functions
   whose bodies compare a reference parameter to a reference literal for key
   `a`, convert a reference parameter to text, and read
   `convert<Sequence<Reference<ConfigVersion>>[0, 8]>(s)`; separately, add a
   post clause reading `count`, `sum(p in self.peers: p.versionNumber)`,
   `exists`, `forall`, `contains` and `size` over `self.peers` and `filter`
   and `map` of it into a `Set`.
5. Add an operation precondition comparing `self` to a reference literal for
   key `a`; separately, add a refinement of the unit whose concrete model holds
   `config_history`, with a field row that converts `self.parent` to text.

Tag the tests `#[trace("TC-566", "FR-150-AC-n")]`.

## Expected Results

- Step 1: each refuses at its node's span with the node kind and `config_history`; the pair reports two refusals in source order.
- Step 2: the unit checks.
- Step 3: two refusals in source order, `ill_typed`/`identity-observing-form` naming `config_history` at the first `fold` and `ill_typed`/`operator-ineligible` at the second, each reported once (FR-150-AC-6).
- Step 4: the function's `fold` refuses `ill_typed`/`operator-ineligible` with no `identity-observing-form`; the literal, text-conversion and `Sequence`-conversion functions check; the post clause of transparent forms checks (FR-150-AC-7).
- Step 5: each refuses `ill_typed`/`identity-observing-form` at its node, naming the node kind and `config_history`, with no checked package (FR-150-AC-8).

---
id: TC-1334
title: "Admit named populations in timed model predicates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230-AC-58
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-230-AC-59
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-230-AC-60
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-230-AC-61
    type: verifies
---
# TC-1334: Admit named populations in timed model predicates

## Description

Verify that the data predicate of a `time invariant when` and of an
`urgent when` is a population position (FR-230, FR-391): it names the timed
model's one population, checks, evaluates at each discrete state over that
state's population, and emits the shared population reference, while the
time invariant body and the state-object rule stay as FR-230 states them.

## Test Procedure

1. Create a dense timed model (model alias `Config`, `time dense unit ms`)
   whose domain package declares object type `Call` (`inflight: Boolean`,
   `waiting: Boolean`, `n: Int[0, 2]`, clock `x`), the one population `calls`,
   and an operation `reply` with a precondition.
2. Check each FR-230-AC-58 predicate (`forall` and `exists` domains,
   `allInstances`, `lookup` by `self`, `count`, `sum`, and the two-read
   conjunction) in a `time invariant` or `urgent when` declaration, and the
   refusal variants: `exists(c in Config::nothing : true)`,
   `exists(j in Other::jobs : true)` over a second model alias `Other`,
   `Config::calls = Config::calls`, and
   the body `forall(c in Config::calls : c.x <= 3 ms)`.
3. Build the two-object discrete state of FR-230-AC-59 and, with each
   declaration alone in its model, read the admissible delays and urgency;
   repeat with `c2.waiting` false, and at a state whose `calls` holds no
   object for `urgent Idle`, `AnyInflight` and `AllInflight`.
4. Emit the FR-230-AC-60 unit, count its `relation`/`population` nodes and
   follow each predicate's references; run the I2 read-back.
5. Check a time invariant and an `urgent when` in models whose populations
   cover no object type, two object types, and `Call` by two populations.

## Expected Results

1. Every FR-230-AC-58 predicate checks to a `TimedModel` with the population
   operand `Population<Call>`, the domain and `allInstances` result
   `Set<Reference<Call>>`, reads at `current`, and `calls` as the predicate's
   only population domain. `Config::nothing` and `Other::jobs` refuse
   `missing_declaration`/`missing-name` at `nothing` and at `jobs`, `Config::calls =
   Config::calls` refuses `ill_typed`/`operator-ineligible` at the first
   `Config::calls`, and the population-quantified body refuses
   `invalid_timed_model`/`time-invariant-shape`, each with no `TimedModel`.
   Self-only predicates and a population-reading precondition still check.
2. `HasCalls` bounds no clock (a delay of 5 is admissible); `AnyInflight`
   bounds both clocks (a delay of 1 is admissible, `1 + 1/1000` is not);
   `CallsExist` makes the state urgent (no positive delay), and does not
   with `c2.waiting` false (a delay of 5 is admissible); `Known` holds of
   `c1` and of `c2`. With no object in `calls`, `Idle` makes the state urgent
   and neither `AnyInflight` nor `AllInflight` bounds a clock, so with either
   alone the state is quiescent. Each truth equals the
   `allInstances<Config::Call>(Config::calls)` spelling's.
3. The package holds one population reference node for `calls`, referenced
   by `HasCalls`, `CallsExist`, `Known` and the precondition; `CallsExist`
   writes no `self` `parameter` node and `Known` writes one; I2 reads back the
   same `TimedModel`.
4. No object type: `missing_declaration`/`missing-name` at the declaration.
   Two object types, or two populations of `Call`:
   `ambiguous_declaration`/`ambiguous-name` at the declaration. No
   `TimedModel` in any of them.

---
id: TC-1264
title: "Hyper atoms and match conjuncts are lowered to checked nodes and evaluated by the one clause evaluator"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: verifies
---
# TC-1264: Hyper atoms and match conjuncts are lowered to checked nodes and evaluated by the one clause evaluator

## Description

Verify that S3 lowers each `holds` atom and `match` conjunct of a hyper
clause to a checked condition over its slots. Verify that FR-107's
evaluator runs these conditions, that the clause seams name the conjunct
that decides a reading, and that `encode_hyper_query` reads the conditions
with no caller seam, an `always` body by its atom and any other safety body
by its tabulated monitor.

Scope: FR-487-AC-1 to FR-487-AC-8 and FR-315-AC-43.

## Test Procedure

1. Check ADR-023 §1's `NonInterference` and `SavesPower` in a unit that
   declares the vault model `V` and the device models `S` and `N`. Read
   each clause's `atom_slots()`, each atom's `condition()`, and each
   conjunct's `slots()` and `condition()`. Check both clauses a second time
   and compare.
2. Check `NonInterference` with `match { a.step.op = V::Vault::step and
   b.step.op = stutter }`, with `match { a.step.receiver =
   b.step.receiver }`, and over a model whose operations with a receiver are
   declared by two object types, with a conjunct reading `a.step.receiver`.
3. Over ADR-023 §8's secure vault at instance `v`, call
   `evaluate_hyper_holds` for `NonInterference`'s atom with components
   `(0, 0)` and `(1, 0)`, then `(0, 1)` and `(0, 0)`. Call it for FR-176-AC-5's
   atom with `(1, 0)` and `(0, 0)`, with a zero-budget meter, and with a
   `slots` vector of length 1. Repeat the first call, keeping both charge
   logs.
4. Call `CheckedStepMatch::universal` for `NonInterference`'s `match` over
   `(step(0), step(0))`, `(stutter, stutter)`, `(step(0), step(1))` and
   `(step(0), stutter)`, recording the meter's charge log for the last.
5. Add FR-176-AC-4's conjunct as conjunct 2, then in turn a conjunct 2
   that calls a declared function which exhausts the meter, and a conjunct 2
   that is refused by its evaluation. Call `universal` for each over
   `(step(0), step(0))` and over `(step(0), step(1))`.
6. Run EN-1 (FR-176) with `CheckedHyperLetters` and `CheckedStepMatch` over
   ADR-023 §8's leaky and secure vaults, and over the secure vault with step
   5's FR-176-AC-4 conjunct.
7. Encode `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for
   `NonInterference` at instance `v`. Build the request in a crate that names
   `HyperQueryRequest` through `qsl-replay`, setting each of its public
   members.
8. Encode FR-315-AC-10's request with the function-calling conjunct, then
   the same request with `present(a.step.Vault::step.i) implies
   value(a.step.Vault::step.i) = 1` placed directly before that conjunct.
9. Encode `InductionBase{depth: 1}` and `InductionStep{depth: 1}`, each
   twice, for `NonInterference` with the body `always (holds(v.h @ a =
   v.h @ b) implies always holds(v.l @ a = v.l @ b))`, then the base query
   with `encoding.max_monitor_states` 1.

Tag the tests `#[trace("TC-1264", "FR-487-AC-n")]`, and step 9's
`#[trace("TC-1264", "FR-315-AC-43")]`.

## Expected Results

- Step 1: `NonInterference` has slots `[(a, v), (b, v)]` and its atom is the
  equality of `l` of slot 0 and `l` of slot 1, slot count 2, with no node
  for `@`. `SavesPower` has slots `[(s, d), (n, e)]`. `NonInterference`'s
  conjunct slots are `[Operation a, Operation b]`, both `String`, and two
  `Option<Int[0, 1]>` argument slots. The second check gives equal
  conditions, slot lists and `HyperIdentity` values.
- Step 2: one conjunct compares its `String` slot with the literal operation
  text of `step`, and the other with the literal `stutter`. The receiver
  slots are `Option<Reference<Vault>>`. The two-type model refuses
  `unsupported_construct`/`expression-form` at the receiver read.
- Step 3: `Completed(true)`, then `Completed(false)`; `Undefined` with
  `division-by-zero`; `Incomplete` with
  `resource_exhausted`/`insufficient-next-charge`; `CallFailure::Fault` with
  no charge; and the repeated call gives an equal evaluation and an equal
  charge log.
- Step 4: `Holds`, `Holds`, `Fails` and `Fails`. The last charge log holds
  conjunct 0's charges only.
- Step 5: over `(step(0), step(0))` the three give
  `Undefined { conjunct: 2, cause: division-by-zero }`,
  `Err(MatchIncomplete { conjunct: 2, .. })` and
  `Refused { conjunct: 2, .. }`. Over `(step(0), step(1))` each gives
  `Fails` and evaluates no node of conjunct 2.
- Step 6: the leaky product has 10 reachable states and returns `Violated`
  with the §8.1 `Lockstep` counterexample. The secure product has 8 states
  and returns `Holds{Exhaustive}`. The added conjunct gives FR-176-AC-4's
  `Undefined` counterexample.
- Step 7: the step query asserts `(= s0.a.v.l s0.b.v.l)` at position 0 and
  the violation at position 1 over `s1.a.v.l` and `s1.b.v.l`. The base query
  has FR-315-AC-7's three joint disjuncts. `HyperQueryRequest` has no
  `StepMatch` or `HyperLetters` member.
- Step 8: `match-conjunct` naming the calling conjunct's index in
  `mu_universal()`, its site `hyper <c> match <k>` and the tuple
  `(step(0), step(0))`; then the same code and conjunct with the tuple
  `(step(1), step(1))`. No script is written in either case.
- Step 9: the declarations, range assertions, monitor implications,
  violations `(= m<p> r)` and step-query distinctness that FR-315-AC-43
  lists. Each script is byte-equal to its repeat. The limited request
  refuses `too-large` naming `monitor-states`, with no script.

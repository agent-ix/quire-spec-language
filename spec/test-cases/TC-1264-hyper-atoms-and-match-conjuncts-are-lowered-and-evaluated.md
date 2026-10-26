---
id: TC-1264
title: "Hyper atoms and match conjuncts are lowered to checked nodes and evaluated by the one clause evaluator"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: verifies
---
# TC-1264: Hyper atoms and match conjuncts are lowered to checked nodes and evaluated by the one clause evaluator

## Description

Verify that S3 lowers each `holds` atom and `match` conjunct of a hyper
clause to a checked condition over its slots. The atom reads are recorded by
component, labels keep FR-172's types, and calls and receivers are checked
as FR-172 now states. Then verify the evaluation and encoding:

- FR-107's evaluator runs the conditions, absent keys included.
- The clause seams carry atom stops and name the conjunct that decides a
  reading.
- `encode_hyper_query` reads the conditions with no caller seam: an `always`
  body by its atom, and any other safety body by its tabulated monitor.

Scope: FR-487-AC-1 to FR-487-AC-12, FR-315-AC-43, FR-315-AC-50, FR-172-AC-7 and FR-172-AC-8. The test
for FR-172-AC-5's label constants, which QSL-1072 delivers, stays in TC-597.

The model `G` and the clause `Linked` are those defined in FR-487's
Acceptance Criteria.

## Test Procedure

1. Check ADR-023 §1's `NonInterference` and `SavesPower` in a unit that
   declares the vault model `V` and the device models `S` and `N`. Read each
   clause's `atom_slots()`, each atom's `condition()` and `component_reads()`,
   and each conjunct's `slots()` and `condition()`. Check both clauses a
   second time and compare.
2. Check `NonInterference` with `match { a.step.op = V::Vault::step and
   b.step.op = stutter }`; with `match { a.step.receiver = b.step.receiver }`;
   with `match { a.step.op = "x" }`; and with `match { size(a.step.op) > 0 }`.
   Check `SavesPower` with `match { s.step.op = n.step.op }`.
3. Over ADR-023 §8's secure vault at instance `v`, call
   `evaluate_hyper_holds` for `NonInterference`'s atom:
   1. with components `(0, 0)` and `(1, 0)`, then `(0, 1)` and `(0, 0)`;
   2. for FR-176-AC-5's atom, with `(1, 0)` and `(0, 0)`;
   3. with a zero-budget meter;
   4. with a `slots` vector of length 1;
   5. with the first call repeated, keeping both charge logs.
4. Call `CheckedStepMatch::universal` for `NonInterference`'s `match` over
   `(step(0), step(0))`, `(stutter, stutter)`, `(step(0), step(1))` and
   `(step(0), stutter)`, keeping each charge log.
5. Add, in turn, each of these as conjunct 2:
   - FR-176-AC-4's conjunct;
   - a conjunct calling a declared function that exhausts the meter;
   - a conjunct comparing `quire::value::ieee::sqrt` under `Float64[exact]`
     of a constant that is not a perfect square with a constant.

   For each, call `universal` over `(step(0), step(0))` and over
   `(step(0), step(1))`, keeping the charge logs.
6. Run EN-1 (FR-176) with `CheckedHyperLetters` and `CheckedStepMatch` over
   ADR-023 §8's leaky and secure vaults. Repeat over the secure vault with
   step 5's FR-176-AC-4 conjunct.
7. Encode `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for
   `NonInterference` at instance `v`. Build the request in a crate that names
   `HyperQueryRequest` through `qsl-replay`, setting each of its public
   members.
8. Encode FR-315-AC-10's request, whose conjunct calls a function. Then
   encode the same request with `present(a.step.Vault::step.i) implies
   value(a.step.Vault::step.i) = 1` placed directly before the calling
   conjunct.
9. Encode `InductionBase{depth: 1}` and `InductionStep{depth: 1}`, each
   twice, for `NonInterference` with the body `always (holds(v.h @ a =
   v.h @ b) implies always holds(v.l @ a = v.l @ b))`. Then encode the base
   query with `encoding.max_monitor_states` 1.
10. Check `Linked` with `B` = `(let r = v.next in present(r) and
    deref(value(r)).l = 1) @ a` and read its `component_reads()`.
    Evaluate it in three cases:
    - over component `a` at `n1.next = n2`, `n2.l = 1` and component `b` at
      `n1.next` absent, `n2.l = 0`;
    - with `@ b`, over the same components;
    - with component `b` holding no `n1` and slot (`b`, `v`) bound
      `HyperSlot::Absent`.

    Over the third case, also evaluate `v.l @ b = 0` and `v.l @ a = 0`.
11. Check `Linked` with `B` = `present(v.p @ a) and value(v.p @ b) = 1`, and
    with `B` = `present(v.p @ a) and value(v.p @ a) = 1`.
12. Check each of these over a model whose operations with a receiver are
    declared by two object types:
    - a conjunct reading `a.step.receiver`;
    - `NonInterference` with `f(a.step.Vault::step.i)`, `f` being as in
      FR-487-AC-11;
    - `NonInterference` with `f(v.l @ a)`.

    Evaluate the `f(a.step.Vault::step.i)` conjunct over `(step(1), step(1))`
    and `(stutter, stutter)`.
13. Call `CheckedHyperLetters::atom` for `holds(S)` of FR-487-AC-12's body,
    and run EN-1 over the secure vault with that body. Call it for
    `NonInterference`'s atom with a component state that its system cannot
    observe.
14. Check `Linked` with `B` = `deref(value(v.next @ a)).l = 1`; with
    `B` = `(let r = v.next in present(r) and deref(value(r)).l = 1) @ a`;
    and with `B` = `present(v.next @ a) and present(v.next)`.
15. Tabulate the TP-3 item of FR-315-AC-45 with `MonitorTable::of` and with
    the constructor over its `CheckedFormula` and profile, and tabulate
    `NonInterference`'s body `always (holds(v.h @ a = v.h @ b) implies always
    holds(v.l @ a = v.l @ b))` with the constructor. Encode FR-315-AC-43's and
    FR-315-AC-45's queries, and compare them with golden bytes recorded from
    `main` before the refactor.

Tag the tests `#[trace("TC-1264", "FR-487-AC-n")]`. Tag step 9's test
`#[trace("TC-1264", "FR-315-AC-43")]` and step 12's checking tests
`#[trace("TC-1264", "FR-172-AC-7")]`. Tag step 14's tests
`#[trace("TC-1264", "FR-172-AC-8")]`. Tag step 15's test
`#[trace("TC-1264", "FR-315-AC-50")]`.

## Expected Results

- **Step 1:**
  - `NonInterference` has slots `[(a, v), (b, v)]`. Its atom is the equality
    of `l` of slot 0 and `l` of slot 1, with slot count 2, no node for `@`,
    and the two reads recorded at components 0 and 1.
  - `SavesPower` has slots `[(s, d), (n, e)]`.
  - `NonInterference`'s conjunct slots are `[Operation a, Operation b]`, each
    lowered as a `String`, and two `Option<Int[0, 1]>` argument slots.
  - The second check gives equal conditions, slot lists, component records
    and `HyperIdentity` values.
- **Step 2:** one conjunct compares its slot with the literal operation text
  of `step`, and the other compares its slot with the literal `stutter`. The
  receiver slots are `Option<Reference<Vault>>`. The `"x"`, `size` and
  cross-model label conjuncts each refuse `ill_typed`/`type-mismatch` at the
  label.
- **Step 3:** in order:
  1. `Completed(true)`, then `Completed(false)`;
  2. `Undefined` with `division-by-zero`;
  3. `Incomplete` with `resource_exhausted`/`insufficient-next-charge`;
  4. `CallFailure::Fault`, with no charge;
  5. an equal evaluation and an equal charge log.
- **Step 4:** `Holds`, `Holds`, `Fails` and `Fails`. The third charge log
  holds the charges of conjuncts 0 and 1, and the fourth holds conjunct 0's
  charges only.
- **Step 5:** over `(step(0), step(0))`, the three give
  `Undefined { conjunct: 2, cause: division-by-zero }`,
  `Err(MatchIncomplete { conjunct: 2, .. })` and `Refused { conjunct: 2,
  outcome }` with the kernel `Refused` `ieee_not_exact` outcome. Over
  `(step(0), step(1))`, each gives `Fails`, with no charge of conjunct 2 in
  its log.
- **Step 6:** the leaky product has 10 reachable states and returns
  `Violated` with the §8.1 `Lockstep` counterexample. The secure product has
  8 states and returns `Holds{Exhaustive}`. The added conjunct gives
  FR-176-AC-4's `Undefined` counterexample.
- **Step 7:** the step query asserts `(= s0.a.v.l s0.b.v.l)` at position 0
  and the violation at position 1 over `s1.a.v.l` and `s1.b.v.l`. The base
  query has FR-315-AC-7's three joint disjuncts. `HyperQueryRequest` has no
  `StepMatch` or `HyperLetters` member.
- **Step 8:** `match-conjunct` naming the calling conjunct's index in
  `mu_universal()`, its site `hyper <c> match <k>` and the tuple
  `(step(0), step(0))`. The second request gives the same code and conjunct
  with the tuple `(step(1), step(1))`. Neither writes a script.
- **Step 9:** the declarations, range assertions, monitor implications,
  violations `(= m<p> r)` and step-query distinctness that FR-315-AC-43
  lists. Each script is byte-equal to its repeat. The limited request
  refuses `too-large` naming `monitor-states`, with no script.
- **Step 10:** both reads are recorded at component 0. The three cases give
  `Completed(true)`, `Completed(false)` and `Completed(true)`. In the third
  case, `v.l @ b = 0` gives `ClauseLetters`' absent-key reading and
  `v.l @ a = 0` reads component `a`.
- **Step 11:** the first is refused `undefined_expression`/`unproved-presence`
  at `value(v.p @ b)`, and the second checks.
- **Step 12:**
  - The receiver read refuses `unsupported_construct`/`expression-form` at
    the read.
  - `f(a.step.Vault::step.i)` checks with its conjunct in `mu_universal` and
    `slots()` `[Argument(a, step, i)]`, and evaluates to `Completed(true)`
    and then `Completed(false)`.
  - `f(v.l @ a)` refuses `unsupported_construct`/`expression-form` at
    `v.l @ a`.
- **Step 13:** `Err(AtomStop::Refused(outcome))` with the `ieee_not_exact`
  outcome, and EN-1 returns `ModelCheckRefusal::AtomRefused` with it. The
  unobservable state gives `Err(AtomStop::Unobservable(..))`.
- **Step 14:** the first is refused `ill_typed` at the `.l` read. The second
  checks, and `component_reads()` records component 0 for both reads. The
  third is refused `ill_typed` at the bare `v.next`.
- **Step 15:** the two tabulations of the TP-3 item have equal state counts,
  initial state, rejecting state and moves, and the hyper body's table has
  the same shape (two atoms, three states). The scripts of FR-315-AC-43 and
  FR-315-AC-45 are byte-equal to those golden bytes.

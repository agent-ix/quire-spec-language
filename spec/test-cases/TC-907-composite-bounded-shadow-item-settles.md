---
id: TC-907
title: "A composite equality-parity claim settles Diverged, Agrees or refused when falsified, and by rows V-1 to V-5 when verified"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-358
    type: verifies
---
# TC-907: A composite equality-parity claim settles Diverged, Agrees or refused when falsified, and by rows V-1 to V-5 when verified

## Description

Verify both entries of FR-358. Both recompile the original proving context,
tie the package, select the claimed equality node, derive the operand
domains and positions, refuse defective bounds and a mismatched O-09
identity ahead of any settlement, and carry the obligation identity on
every outcome, the full claim (the observation too). `replay_composite_parity` compares exact equality and the
pair count with the retained shadow, and never settles `Refuted`.
`settle_verified_shadow` applies the coverage rule and settles by rows V-1
to V-5 for every CG FR-028-AC-17 strength.

Scope: FR-358-AC-1 to FR-358-AC-8.

## Test Procedure

Use FR-358's unit and its names `E`, `N`, `B`, `f`, `g`, `h` and `k`. Build
each request from the unit's spine compile. Unless a step says otherwise,
each request's `obligation_identity` is FR-358 step 6's digest over that
call's harness bounds.

1. Falsified, row F-7, with `refinement: Exhausted`, at `E`, with operands
   `Q { x: 1, s: [2] }` and `Q { x: 1, s: [3] }` and `native` `Completed`
   (then `Refused`): replay with the shadow `{ equal: true }` at the exact
   pair count; with the right verdict and a count one lower; and with the
   exact outcome. At `N`, replay with an agreeing shadow.
2. Falsified, rows F-2 to F-6, each with an agreeing shadow:
   - F-2: at `E` with `native` `Incomplete`, then `ExecutionFault`, each
     with a `NativeCause`; repeat both with left `x: 12`, with request
     accounting limits too small to admit the operands, under limits too
     small for the exact evaluation and with `refinement: CeilingReached`.
   - F-3: at `N` with right operand `Q { x: 2, s: [] }`; at `E` with left
     `x: 12`, with `refinement: CeilingReached`.
   - F-4: at `E` with request accounting limits too small to admit the
     operands, with `refinement: Exhausted` and again with `CeilingReached`.
   - F-5: at `E` with limits too small for the exact evaluation, with
     `refinement: Exhausted` and again with `CeilingReached`.
   - F-6: at `E` with `refinement: CeilingReached`, admitted operands, a
     `Completed` native and enough limits.
3. Both entries, at `E`: call with an edited source; with an undeclared
   `selected_function`; with a `node` the package does not hold; with the
   node of `h`'s body while `selected_function` names `f`; with `operator`
   `NotEqual`; with an `occurrence` the node does not have in `f`'s body and
   with one of another function; and with an `obligation_identity` over
   `[0, 9]` on the first `x` while the harness bound is `[0, 8]`.
4. Verified, `success_checks` 4: at `E` with `Exhausted` and `B`; with
   `[-1, 10]` on the first `x`; at `N` with `Exhausted` and `B`'s left half;
   at `k` with `DeclaredDomain`s of cardinality 5 on each `s` and harness
   cardinality 5 and `[0, 9]` on each element; at `c` (FR-358-AC-4's enum
   unit) with `Exhausted` and `Variants { Blue, Green, Red }` on each
   operand.
5. Verified, `success_checks` 4: `NotExhausted` with `B`; `Exhausted` with
   cardinality 2 on one `s`; with `[0, 8]` on one `x`; with no bound on one
   `x`; with an empty list at `E`; at `h` with harness depth 3; at `k` with
   harness cardinality 5 and no `DeclaredDomain`; at `c` with
   `Variants { Green, Red }` on `a` and every variant on `b`; at `t` (FR-358-AC-5's
   unit, with a `Text[0, 4]` leaf) with `Exhausted` and bounds covering each
   `x`.
6. Verified: `refinement: Disagreed` with 4 checks and `B`, and with 0 checks and an
   empty list; `CeilingReached` with 4 checks and with 0; `Exhausted` with 0
   checks and `B`.
7. Verified at `E`, with 4 checks and again with 0, each with `Exhausted`:
   `B` plus a bound keyed at path `[7]` of the first parameter; two bounds
   on one `x` key; an `IntegerRange` bound keyed at an `s`; and a request
   `DeclaredDomain` keyed at an `x`. At `c`: a `Variants` bound naming
   `Purple`; and a request `DeclaredDomain` over `a`.

8. Falsified, row F-1, with `refinement: Disagreed` at `E`: replay with a
   diverging shadow; with an agreeing shadow; with left `x: 12`; with
   `native` `ExecutionFault`; with limits too small for the exact
   evaluation; and with an edited source.

9. Occurrences, at `d(a: Q, b: Q): Boolean { (a = b) and (a = b) }`: each
   of the node's two occurrences, under its own identity, through both
   entries; occurrence 2's claim under occurrence 1's identity.
10. Identity, written independently: the canonical O-09 text for `f` over
    `B` and for `g`, hashed in the test, equals the request's identity; a
    composite literal operand's argument is `graph_child` over its own node
    id with the empty `bounds` domain; at `h`, the argument's text holds the
    depth the harness drew, for depth 3 and for depth 4.
11. Full claim: for each outcome of steps 2 and 3 and of rows V-1 to V-5,
    `report.claim()` equals the identity built from what was sent, and
    equals no identity built from a changed `shadow` pair count, `native`,
    `refinement`, operand or `success_checks`.
12. `ReplayLimits`: a request whose S1 limit is above the default
    `replay.input_bytes` refuses `LimitAboveReader` through both entries
    under the default and settles when the limit is raised.

Tag each test `#[trace("TC-907", "FR-358-AC-N")]` for the criterion it
backs.

The `k` cases of steps 4 and 5 have no
source form (the grammar takes only `K<T>[min, max]`), so the unbounded
`K<T>` position derivation and coverage are tested directly, over each of
`Sequence`, `Set`, `Bag` and `OrderedSet`. The `Text` leaf of step 5 uses a
rational leaf until text-profile selection exists (QSL-646).

## Expected Results

- Step 1: `Diverged` (`Failed`, internal failure) for the first two
  shadows, and `Agrees` (`Inconclusive(ScalarAgrees)`, holding a
  `CompositeEquality` claim and an `Equality` outcome) for the third and
  at `N`, where the exact verdict is the negation of `Equal`'s. No case
  settles `Refuted`.
- Step 2:
  - F-2: `GeneratedFault` holding the native outcome and its `NativeCause`
    (`Failed`), for every case, with no admission and no exact evaluation.
  - F-3: `RefusedInput` at index 1, then at index 0 twice (each
    `Inconclusive(ReplayRefused(invalid_runtime_input))`).
  - F-4: `Incomplete { stage: Admission }` naming the counter
    (`Incomplete(ResourceExhausted)`), for both cases.
  - F-5: `Incomplete { stage: ExactEvaluation }` naming QSL's counter
    (`Incomplete(ResourceExhausted)`), for both cases.
  - F-6: `Incomplete { stage: RefinementCeiling }`
    (`Incomplete(ResourceExhausted)`). The F-4, F-5 and F-6 reports differ in
    their stage.
- Step 3: in order, `PackageIdMismatch` (`content-mismatch`),
  `UnknownFunction` (`missing-name`), three `ScalarIdentity`
  (`content-mismatch`) refusals each with its own cause, two
  `ScalarIdentity::Occurrence` refusals, and `ScalarIdentity::Obligation`
  naming both digests. Each report carries the full claim.
- Step 4: `Proved { basis: Checks { success_checks: 4 }, certification: Certified }`, success, for each (row V-4).
- Step 5: `Tested`, success and never `Proved`, for each (row V-5).
- Step 6: `Failed` for both `refinement: Disagreed` calls (row V-1);
  `Incomplete(ResourceExhausted)` for both `CeilingReached` calls (row
  V-2); then `Proved { basis: Checks { success_checks: 0 }, certification: Certified }`, inconclusive, cause
  `kani_vacuous_proof` (row V-3).
- Step 7: each refuses with no settlement row, at 4 checks and at 0, naming
  the key.
- Step 8: `Disagreed` (`Failed`, never `Refuted`) for the first five cases.
  The edited source still refuses `PackageIdMismatch`.
- Step 9: `Agrees` for both occurrences through the falsified entry and
  `Proved` through the verified one; occurrence 2 under occurrence 1's
  identity refuses `ScalarIdentity::Obligation`.
- Step 10: every digest equals the hand-hashed text; two drawn depths give
  two digests.
- Step 11: equal for the sent claim and unequal for each changed member.
- Step 12: `LimitAboveReader` under the default, a settlement when raised.


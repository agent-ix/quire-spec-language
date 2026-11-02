---
id: TC-553
title: "Concrete steps are decided against an abstract protocol with internal closure and observation labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-148
    type: verifies
---
# TC-553: Concrete steps are decided against an abstract protocol with internal closure and observation labels

## Description

Verify the abstract protocol state sets with internal closure, visible and
internal concrete steps, refutations that replay, and QSpec FR-177's
divergence-freedom, refusal-set, terminal-success and environment-assumption
checks on the same product.

Scope: FR-148-AC-1 to FR-148-AC-14; FR-144-AC-9 to FR-144-AC-10.

## Test Procedure

Fixtures: `Twice`, `Once` and `CasTwice` (FR-147, FR-148); the lost-update
model (ADR-020 §8) with `peek` removed; `CasTwice` with `peek` kept.

1. Run `check_initial` and the steps `beginA`, `commitA`, then a second
   commit, over `CasTwice`.
2. Check and settle `CasTwice` over universe `counters = {c}`, without
   applying the model-subject simulation-certificate checker to its
   abstract-protocol safety result; the protocol step decisions and further
   checks have no core certificate checker.
3. Check the lost-update model against `Twice` and replay its
   counterexample.
4. Check `CasTwice` against `Once` and replay its counterexample.
5. Check `CasTwice` with `peek` kept and mapped `-> stutter`, with no
   fairness rows.
6. Check a one-thread protocol whose only enabled node maps to `inc`, with
   weak whole scheduler fairness, against a concrete `inc` model with the
   corresponding weak whole `assume`; check the assumption implication over
   every reachable paired lasso. Then directly require one admitted temporal
   clause carrying `fair weak whole inc`, plus an unrelated clause carrying
   fairness for another operation; verify only the directly required clause's
   checked fairness row enters authored `S`, with its clause `NodeKey` and
   exact checked `FairnessOperation` identity.
7. Change the protocol to adversarial scheduling, give it no directly
   required temporal clause, and remove its memory fairness source. Keep
   concrete weak whole `assume inc` and add
   an infinite stutter loop while `inc` remains enabled; check, settle and
   replay `ProtocolAssumptionNotEstablished{reason: strengthened}` with the
   exact concrete assumption identity and paired concrete/abstract prefix
   and loop. Keep an unrelated temporal clause with a fairness row in the
   package and verify it is excluded. Verify settlement is `inconclusive`,
   `unsettled`, not `refuted` or `proved`, and replay reproduces the
   predicates.
8. Use two concrete operation classes, `left` and `right`; have the abstract
   protocol directly require only an admitted temporal clause carrying `fair
   weak whole left`, and have the concrete model assume weak fairness of both
   classes. Keep a separate, unrequired temporal clause carrying `fair weak
   whole right`. Check a reachable lasso that takes `left` fairly while
   `right` remains continuously enabled but is never taken; verify the typed
   non-proof outcome carries the required clause's exact `NodeKey`, its
   checked left `FairnessOperation`, the right concrete identity and paired
   paths, while the unrequired right row does not enter `S`. Repeat with a
   product limit below the complete exploration and verify `incomplete`,
   never `proved`.
9. Use separate abstract protocol variants over the same finite single-
   operation domain. The first directly requires a temporal clause carrying
   `fair strong each inc` and compares it to weak whole concrete `assume inc`
   (which passes); the second directly requires a clause carrying `fair weak
   whole inc` and compares it to strong concrete fairness on a lasso where
   the operation is enabled infinitely often but not continuously and never
   taken (which returns the typed non-proof outcome and replays). Verify each
   exact source includes the required clause `NodeKey` and checked
   `FairnessOperation`.
10. Use an abstract protocol with a `tso` `parallel`,
    `scheduling adversarial` and no directly required temporal fairness
    clause, so its scheduler constraints are absent but its derived memory
    constraint `fair weak each flush(writer)` remains in `S`.
    Make `tick` the writer's repeated buffered store and keep concrete `inc`
    continuously enabled but untaken on the lasso. Check that the lasso takes
    a `flush(writer)` each cycle, satisfies memory fairness, violates
    concrete weak whole `assume inc`, and returns
    `ProtocolAssumptionNotEstablished{reason: strengthened}` with the exact
    memory and operation identities and paired paths. Replay confirms the
    flush fairness and violated `inc` constraint.
11. Exercise QSpec FR-379's refusal and terminal-success vectors on the same
    protocol product: a stable implementation refusing `ack` while every
    stable candidate enables it; a permitted refusal where one stable
    candidate refuses; both `concrete-only` and `abstract-only` terminal
    success mismatches; and replay with a wrong position, label and direction.
    Include an incoming step-rule failure at the same post-state and a model
    abstract subject to verify precedence and exclusion.
12. At a reachable paired state, make the source-checked enabledness of the
    concrete `assume` fairness target return `ContractUndetermined`, with no
    reachable paired lasso that satisfies `S` and violates `I`. Check and
    settle `ProtocolAssumptionNotEstablished{reason: relation-undetermined}`
    with the exact assumption identities, both finite prefixes to that state,
    and the typed undecided-predicate locus. Replay it, then change the locus
    and verify `ReplayParity`. Keep this case separate from step 8's
    product-limit `incomplete` result.
13. Compile a protocol with a `requires temporal` name that does not name a
    local temporal clause and verify FR-218-AC-16's S3 refusal. Then use an
    emitted protocol requirement key that names no emitted temporal node and
    verify FR-383-AC-17's S4 refusal. In both cases no FR-148 product runs and
    no `ProtocolAssumptionNotEstablished` result is produced. Verify a
    uniquely resolved required temporal clause with no fairness rows
    contributes no authored constraint; FR-148 selects admitted node keys,
    not names.
14. Use the directly required `fair weak whole inc` clause from step 6 and
    add a separate `ensure fair weak whole inc` guarantee. Choose a paired
    lasso that takes `inc` so the guarantee is satisfied. Compare the
    assumption source sets and result with the equivalent refinement without
    the `ensure`; verify the guarantee row is absent from both `I` and `S`.

Tag the tests `#[trace("TC-553", "FR-148-AC-n")]`.

## Expected Results

- Step 1: one state at `a1`, `value` 0; the state at `a2`, `value` 1; the
  finished state with `value` 2.
- Step 2: `proved`, `closed-scope`, `Uncertified`; FR-149 does not certify
  protocol-subject step decisions, and the further checks have no core
  certificate checker.
- Step 3: `refuted`, `NoAbstractMatch` at the commit that leaves `value`
  unchanged; `reproduced-with-evaluated-witness`.
- Step 4: `refuted`, `NoAbstractMatch{position: 4}` at the second commit;
  reproduced.
- Step 5: `refuted` by the divergence-freedom check, a lasso whose loop is
  one `peek` step at the initial state.
- Step 6: the abstract scheduler constraint implies the matching concrete
  weak whole assumption over all reachable paired lassos; the item passes.
  In the authored variant, only the directly required clause's exact checked
  operation row enters `S`, alongside its clause `NodeKey`; the unrelated
  temporal clause's fairness row is absent.
- Step 7: the empty specification assumption is true on the stutter loop and
  the concrete assumption is false; a typed `ProtocolAssumptionNotEstablished`
  witness is returned and replay reproduces it. The unrelated temporal row
  is excluded; the item is inconclusive, not refuted.
- Step 8: the required `left` row is the only authored specification
  constraint; the unrequired `right` row is excluded. The `right` concrete
  fairness constraint has a paired lasso despite the abstract `left`
  constraint; the typed result retains exact source identities. A limited
  exploration settles incomplete without a proved result.
- Step 9: with `StrongEach` required, its exact row implies weak whole for
  the finite class set. With `WeakWhole` required, it does not imply strong
  concrete fairness on the intermittent-enabledness lasso. Replay reproduces
  the latter paired typed non-proof witness, which remains inconclusive.
- Step 10: `S` contains the memory-origin flush constraint despite adversarial
  scheduling; the lasso satisfies that constraint, while the concrete `inc`
  assumption is violated. The typed inconclusive witness is replayed with the
  same paired path, identities and fairness predicates.
- Step 11: refusal reports `RefusalNotPermitted{position: 1, label: {step:
  "ack", binder: []}}`; the permitted refusal holds. Terminal failures are
  `TerminalSuccessMismatch{direction: concrete-only}` and
  `{direction: abstract-only}` at their prefix positions. Each QSpec kind
  replays under FR-379-AC-13; malformed positions, labels or directions
  settle `inconclusive`, `replay-parity`; the incoming step-rule failure
  takes precedence and a model abstract subject yields neither kind.
- Step 12: no strengthening lasso exists. The typed result is
  `ProtocolAssumptionNotEstablished{reason: relation-undetermined}` with the
  paired finite prefixes and exact `ContractUndetermined` locus; replay
  reproduces this as an inconclusive no-proof result, never `refuted` or
  `proved`. A changed locus settles `inconclusive`, `replay-parity`, while
  the separate product-limit case is `incomplete`.
- Step 13: the undeclared or alias-only temporal requirement refuses at S3,
  and the requirement key with no emitted temporal node refuses at S4. Neither
  reaches product evaluation or emits an assumption outcome. A uniquely
  resolved required clause with no fairness rows contributes no authored
  constraint; product lookup uses admitted node identities, never display
  names.
- Step 14: the selected lasso satisfies the `ensure` guarantee. The `ensure`
  row appears in neither `I` nor `S`, and adding or removing it leaves the
  assumption source sets and relation result unchanged.

---
id: TC-1180
title: "The quire-eval clause entry matches the std reference on every clause path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-406
    type: verifies
---
# TC-1180: The quire-eval clause entry matches the std reference on every clause path

## Description

Verify FR-406. `quire-eval`'s clause entry evaluates every node kind over
two observations, reports the FR-265 trail as events, and gives the std
reference's results, charges and reads on every clause path. The
`Value`-family entry is unchanged, and the clause entry adds at most 2% to
a `Value`-only `thumbv7em-none-eabi` binary. Scope: FR-406-AC-1 to
FR-406-AC-14.

Until FR-405's switch, each comparing step runs against the reference and
against the clause entry. After the switch it runs once on `quire-eval`
against the reference's recorded values.

## Test Procedure

1. In `agent-ix/quire-eval`, read `quire-eval`'s manifest and run
   `cargo tree -p quire-eval -e features -i quire-checked`. Search its source
   for `qsl_`. Run the `thumbv7em-none-eabi` gate build. Read which traits
   the closure-only environment implements.
2. Run FR-402's harness over the QSL-owned corpus at the clause-entry
   change's base and head, and diff the compared records. Begin the
   `Value`-family entry over `pre(allInstances<T>(p))` built by
   `check_postcondition_expression`.
3. Run FR-107-AC-1 to AC-5 and AC-13 to AC-20 over the FR-108 corpus
   through the clause entry, with the charge log. Run each corpus case ten
   times.
4. Run FR-107-AC-2 through the clause entry. Evaluate a postcondition
   reading `pre(self.versionNumber)` over observations with no pre side.
   Evaluate FR-405-AC-6's expression through
   `CheckedPackageEvaluation::evaluate` with a `ReadLog` attached. Run the
   `pre(...)` tests of `qsl-eval/tests/it/model_reference_queries.rs`.
5. Over a snapshot listing `P` `complete` with `m1`, `m2`, `m3`, two
   conforming to `T`, evaluate `allInstances<T>(M::P)`. Repeat with `P` not
   listed `complete`. Evaluate `lookup<T>(M::P, r)` with an absent `r`
   under `undefined` and under `refused`, and with a non-conforming `r`,
   once over `P` listed `complete` and once over `P` not listed
   `complete`, with the charge log.
6. Run FR-139's refinement-row tests and FR-138's history-update tests
   through the clause entry. Read a history field with no value. Run a
   `PopulationObjects` read ten times.
7. Evaluate a state clause reading `self.versionNumber`, and one reading
   `deref(value(self.parent)).versionNumber`, in the `Clause` regime and in
   the `Value` regime, with the charge log. Evaluate a state clause whose
   body is `allInstances<T>(p)` over a `Population` parameter.
8. With `qsl-eval`'s recorder, run FR-107-AC-7 to AC-12 and FR-265-AC-1 to
   AC-7 through the clause entry.
9. Run FR-268's separation-check tests through the clause-entry steps, with
   the charge log, counting Begins per step and recorders per check. Run
   the check for `LetAll` over `high`, and again with a meter that denies a
   charge of its `let` value.
10. Begin FR-265-AC-1's `AllBelow` over `high` with no recorder and with an
    accepting recorder. Begin it with a recorder that returns an
    `InternalFault` at the first `stopped` event.
11. Run the tests of each path FR-406-AC-11 lists through the clause entry.
12. Begin the body form over a node of an imported library graph that the
    closure's scope view holds. Begin
    the expression form over `x + 1` with `x: Int[0, 9]` given `10`, and
    given two values.
13. Search the QSL crates and `quire-eval` for the definitions of the four
    clause causes, `ModelQueryRefusal` and `ProtocolClauseFrameRefusal`.
    Compare the catalog codes and fields of steps 4 to 7's results with the
    reference's.
14. In `agent-ix/quire-eval`, at the clause-entry change's head, run the
    gate's probe builds (`staticlib`, `thumbv7em-none-eabi`,
    `opt-level = "s"`, fat LTO, one codegen unit, `panic = "abort"`) and
    read the `.text` plus `.rodata` sizes `llvm-size -A` reports. Build
    `probe-value` against the change's base. Set the budget one byte below
    `probe-value`'s head size and run the gate. Read the change's
    description.

## Expected Results

1. The manifest meets FR-403-AC-1 with no feature for the clause entry. No
   `construct` feature appears, and no `qsl_` path occurs. The gate build
   passes. The closure-only environment implements only FR-403's contract.
2. Zero divergences. Every compared record is equal at base and head. The
   `Value`-family Begin returns `CallFailure::Fault` with stage
   `quire-eval`, invariant `outside-value-family` and no charge.
3. Every criterion holds with the reference's outcomes, entries, counters
   and charges. `reaches` admits exactly FR-107-AC-3's five charges. The
   ten runs are equal.
4. `pre(self.versionNumber)` is 2 and `self.versionNumber` 3. The missing
   pre side gives `wrong_snapshot`/`wrong-anchor` at the read, with the
   reference's charges. The expression completes with `a` and `b` in
   ascending order, with the reference's charges and one `Membership` read
   observed `Pre`. Every `pre(...)` test passes with its expected values.
5. The two conforming members are returned in ascending reference-key
   order. The admitted charges equal the reference's: three
   `population.visit`, `collection.bound` sized 2, then the formation's
   charges, including a second `collection.bound` sized 2. The incomplete population gives
   `incomplete_population`/`incomplete-scope` with field `population`. The
   absent key gives the `state-model` undefined result under `undefined`
   and the `ModelQueryRefusal` code under `refused`. The non-conforming
   reference gives the type-mismatch code with no charge, over either
   population. Every result, location and
   charge sequence equals the reference's.
6. The tests pass with the reference's values and charges. The history
   read gives `missing_declaration`/`missing-name` with field `history`.
   The ten `PopulationObjects` runs return the members in ascending
   reference-key order, with equal results and charges.
7. In the `Clause` regime, `model.navigate` precedes each read, and
   `model.deref` precedes it for the `deref(...)` read. In the `Value`
   regime neither appears. The population-parameter clause gives
   `unknown_required_feature`/`unsupported-feature` at the node.
8. Receiver paths, read entries and their paths, decisions, stop reports,
   bases and witness records equal the reference's.
9. Each check gives the reference's `Separation` and charges, with one
   recorder per check. No Begin is made for the element step. `LetAll`
   holds. The denied `let` value gives `Separation::Stopped` at the domain
   step.
10. The two runs are equal in evaluation, counters, charges and reported
    reads. The faulting recorder ends the session with `CallFailure::Fault`
    carrying its fault, and no charge follows the event.
11. Every test passes with the reference's expected values.
12. The body form gives `CallFailure::Fault` with invariant
    `clause-root-outside-closure` and no charge. The expression form gives
    `CallFailure::Input` for `10` and `InputRefusal::Arity` for two values,
    each with no charge.
13. The four causes are defined only in `quire-eval`. `qsl-eval` defines
    `ModelQueryRefusal` and `ProtocolClauseFrameRefusal` and none of the
    four. Codes and fields equal the reference's.
14. The gate reports both probes' sizes and passes at the configured
    budget, which is 102% of `probe-value`'s base size. With the budget one
    byte below the head size, the gate fails. The description reports the
    base size, the budget and both head sizes.

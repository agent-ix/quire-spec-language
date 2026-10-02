---
id: TC-612
title: "An undefined predicate refutes a state-graph claim and replays"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-170
    type: verifies
---
# TC-612: An undefined predicate refutes a state-graph claim and replays

## Description

Verify that a predicate of a state-graph claim that evaluates undefined at
an explored node refutes the item with cause `UndefinedEvaluation`, in place
of a witness, and that replay reproduces the undefined value at that node.

Scope: FR-168-AC-5, FR-169-AC-6, FR-170-AC-5.

## Test Procedure

Fixture: ADR-022 §7.1's subject and `possible 2 / (2 - c.versionNumber) = 2`
for `c = a`.

1. Run the item through EN-1.
2. Replay its evidence, then the same evidence with the stem's last step
   removed, then with `where.locus` naming another expression of the claim
   and the same cause.
3. Settle the outcome with step 2's first replay result.
4. Run `always possible PDiv using v over (c: Config::ConfigVersion) from
   (c.versionNumber >= 1) { 2 / (2 - c.versionNumber) = 2 }` for `c = a`;
   replay its `Undefined` evidence, then the same evidence with
   `where.predicate` naming the `from` predicate `c.versionNumber >= 1`
   and the same cause.

Tag the tests `#[trace("TC-612", "<AC id>")]`.

## Expected Results

- Step 1: `Undefined` with stem `(0, 0) -upd(a)-> (1, 0) -upd(a)-> (2, 0)`,
  `where` `(2, 0)`, cause `division-by-zero`; not `Witnessed`.
- Step 2: `reproduced-with-evaluated-witness`; then `inconclusive`,
  `Verdicts`, twice.
- Step 3: `refuted`, `decisive-counterexample`, category violation, cause
  `UndefinedEvaluation`.
- Step 4: `Undefined` with stem `(0, 0) -upd(a)-> (1, 0) -upd(a)-> (2, 0)`
  and `where.predicate` the body; `reproduced-with-evaluated-witness`;
  then `inconclusive`, `Verdicts`.

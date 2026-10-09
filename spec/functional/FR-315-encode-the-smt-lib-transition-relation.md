---
id: FR-315
title: "Encode a model subject's transition relation as SMT-LIB"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
---
# FR-315: Encode a model subject's transition relation as SMT-LIB

## Description

QSL SHALL own the one SMT-LIB 2 encoding of a model subject's transition
relation and of a temporal item's proof queries, the bounded unrolling of
V-2 and the base and step cases of EN-3's `k`-induction, in `qsl-eval` (layer 5), exposed through the `qsl-replay` facade
as `encode_smt_query` (ADR-018 LA-2, LA-3, PC-6). CG's SMT backend calls it
to build the queries it sends to the solver, and FR-314's checker calls it
to rebuild the queries a certificate claims to refute, so the two compare
byte for byte under the canonical printing below.

## Use case

A verification operator gets an SMT proof labelled `certified`.
That label means the core checked the solver's refutation of a query the
core itself encoded, which works only when the backend that asked the
solver used the same encoding the core rebuilds.

## Inputs

- A model subject (FR-125): the checked package, its initial states, its
  universes and `ProofBound`s.
- A checked temporal item with its property form (FR-123), or the
  deadlock-freedom item (FR-124).
- A `SmtQueryKind`: `Unrolling { depth }`, `InductionBase { depth }` or
  `InductionStep { depth }`.

```rust
pub enum SmtQueryKind {
    Unrolling { depth: u64 },
    InductionBase { depth: u64 },
    InductionStep { depth: u64 },
}

pub fn encode_smt_query(
    subject: &ModelSubject,
    item: ModelCheckItem<'_>,
    kind: SmtQueryKind,
) -> Result<Vec<u8>, SmtEncodingRefusal>;
```

## Outputs

- The query's SMT-LIB 2 script in canonical printing.
- `SmtEncodingRefusal` naming the construct, with its locus, when the item
  or subject uses a construct the encoding has no form for.

## Behavior

- The encoding SHALL declare one state copy per position `0..=depth`, each
  field of each universe object as one constant of the field's SMT sort
  (`Int` for integer and enumeration fields with range assertions, `Bool`
  for Boolean fields), named `s<i>.<object>.<field>`; an optional field is
  a `Bool` presence constant `s<i>.<object>.<field>.present` and a value
  constant of its element's sort. Every term the encoding prints is linear,
  so each query is a `QF_LIA` script.
- The encoding SHALL assert the initial-state predicate on copy 0 for
  `Unrolling` and `InductionBase`, and on no copy for `InductionStep`.
- The encoding SHALL assert, for each step `i` to `i+1`, the disjunction
  over the subject's operations and argument vectors of the operation's
  precondition on copy `i`, its postcondition relating copies `i` and
  `i+1`, and its frame, with the terminal stutter as one more disjunct
  under infinite-trace (FR-120, FR-125).
- For a TP-3 item or a TP-2 `on each` item the encoding SHALL also declare
  one monitor-state constant `m<i>` per position, assert the initial
  monitor state on `m0` where the initial-state predicate is asserted, and
  assert each step of the safety monitor that `qsl-eval`'s
  property-automaton translation builds (ADR-018 EN-3, LA-2); the property
  at a position is then the monitor state there not rejecting. For a TP-1
  item the property at a position is its state predicate there.
- The encoding SHALL read undefinedness as a refutation, never as false or
  as a disabled step (ADR-018 UE-1, UE-4, UE-6): for every atom the claim
  reads at a position and every precondition guard a step reads, it SHALL
  build the atom's or guard's definedness condition (a divisor not zero, a
  reduction over a non-empty collection, a value inside its sort's range),
  and the negated property SHALL hold when a definedness condition fails
  at a position the query covers as well as when the property is
  violated there.
- The encoding SHALL assert the negated property: for `Unrolling` and
  `InductionBase`, a violation or a definedness failure at some position
  `0..=depth`; for `InductionStep`, the property and every definedness
  condition at positions `0..depth`, the states at those positions
  pairwise distinct (some field or monitor state differs between every two
  copies), and a violation or a definedness failure at `depth` (ADR-018
  EN-3).
- The encoding owns only these proof queries. EN-2's counterexample
  searches, including its lasso queries with a loop back-edge and fairness
  on the loop, are the SMT backend's own (ADR-018 DS-3); their refutations
  replay through FR-128 and need no certificate.
- The canonical printing SHALL be: `(set-logic QF_LIA)` first; then the
  `declare-const` commands in ascending (position, object, field) order;
  then one `assert` per conjunct in the order above; then `(check-sat)`;
  terms printed with single spaces, no comments, integer literals in
  decimal with negatives as `(- n)`, and each command on its own line ending
  in a line feed.
- Equal inputs SHALL give byte-equal scripts.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-315-AC-1 | Over the `Counter` subject (universe `{c}`, initial value 0), `Unrolling{depth: 3}` for `always[0,3] holds(c.value <= 3)` declares `s0.c.value` to `s3.c.value` in order, asserts `(= s0.c.value 0)`, one transition assertion per step and the negated property, and ends with `(check-sat)`; encoding it twice gives byte-equal scripts. | Test (TC-900) |
| FR-315-AC-2 | `InductionStep{depth: 1}` for `always holds(c.value <= 3)` asserts no initial-state predicate, asserts the property at position 0 and its violation at position 1; `InductionBase{depth: 1}` asserts the initial-state predicate on copy 0. | Test (TC-900) |
| FR-315-AC-3 | An item over a field whose sort the encoding has no form for refuses `SmtEncodingRefusal` naming the field, with its locus, and writes no script. | Test (TC-900) |
| FR-315-AC-4 | Over a `Slot` subject (universe `{s}`, an optional `Int[0, 3]` field `held`, initially present with value 0; operation `drop` with frame `modifies [held]` that clears `held`, and operation `take` with precondition `pre Has { value(self.held) <= 3 }` that sets `held` to 1), `Unrolling{depth: 2}` for `always holds(value(s.held) <= 3)` makes the negated property hold when `(not s<i>.s.held.present)` holds at some position `i` in `0..=2`, since `value` of an absent field is undefined (`none-value`); the `take` disjunct's guard contributes the same definedness disjunct at the step that reads it. A solver therefore cannot refute the query once `drop` is reachable, and no `proved` follows for a claim UE-1 refutes. Every term of the script is linear. | Test (TC-900) |
| FR-315-AC-5 | `InductionStep{depth: 2}` for `always holds(c.value <= 3)` asserts that copies 0, 1 and 2 are pairwise distinct; for a TP-3 item it declares `m0` to `m2` and asserts the monitor steps, the property at a position being the monitor state there not rejecting. | Test (TC-900) |

## Dependencies

- ADR-018 V-2, V-3, PC-6, LA-2, LA-3, DS-2.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (the
  transition relation), [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (the subject), [FR-314](FR-314-check-an-smt-proof-certificate.md) (the
  checker that rebuilds the queries).

## References

- SMT-LIB 2 standard.
- Owning ticket: Linear QSL-366.

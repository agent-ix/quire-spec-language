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
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-173
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-182
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
byte for byte under the canonical printing below. The same encoding gives
EN-3's base and step queries over an HP-2 item's self-composition product
(ADR-023 HC-1, HC-7) through `encode_hyper_query`, which CG's SMT backend
calls for HP-2 `k`-induction.

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

### Refinement queries

For a refinement's per-step simulation obligations (QSpec FR-468, FR-146) QSL
SHALL expose `encode_refinement_query(request, kind) -> Result<Vec<u8>,
SmtEncodingRefusal>`, `request` FR-314's `RefinementCertificateRequest` (the
recompiled package, the refinement node and the concrete and abstract
subjects), with `kind` either `Initial` or `Step { row }`, `row` the zero-based position in the refinement node's `steps`
array. Both use this requirement's state-copy declarations, sorts, canonical
printing and refusals.

- The `Initial` query SHALL declare one concrete state copy `s0`, assert the
  concrete subject's initial-state predicate on it and every hypothesis (the
  invariant clauses whose context is an object type of the concrete model,
  FR-146) on it, and assert the negation of: `map(s0)` equals an abstract
  initial state by state key. Every mapping row, receiver and argument the
  query reads SHALL also carry its definedness condition, and the negation
  SHALL hold when one fails (UE-1, UE-4, UE-6).
- The `Step { row }` query SHALL declare concrete copies `s0` and `s1`, assert
  every hypothesis on `s0`, the row's operation precondition on `s0`, its frame
  and postcondition relating `s0` and `s1`, and the negation of the row's rule
  over (`map(s0)`, `map(s1)`): for `stutter`, the mapped state is unchanged by
  state key; for an abstract operation, that abstract transition's
  precondition, frame and postcondition, encoded as this requirement encodes a
  transition, hold over the mapped pair with the row's receiver and arguments
  evaluated at `s0`; a row with a `_` argument leaves that argument open, which
  the negation would have to quantify universally, and the `QF_LIA` scripts
  carry no quantifier, so the call refuses `SmtEncodingRefusal` naming the row
  and the argument; for `any`, no query is encoded and the call refuses
  `SmtEncodingRefusal`, because a declaration with an `any` row has no per-step
  query (FR-146).
- A mapping row, receiver or argument outside the `QF_LIA` fragment this
  requirement prints SHALL refuse `SmtEncodingRefusal` naming it, with its
  locus, and write no script.

### Hyper self-composition queries

For EN-3's `k`-induction of an HP-2 item with a safety body (ADR-023 HC-7,
DS-3) QSL SHALL expose `encode_hyper_query(request, kind)` through the same
facade. Its queries encode FR-176's self-composition product (ADR-023 HC-1)
as one copy of the transition relation per trace variable, the copies
stepping together on the joint steps `μ_U` matches, with this requirement's
sorts, definedness reading, canonical printing and refusals.

```rust
pub struct HyperQueryRequest<'a> {
    pub clause: &'a CheckedHyperClause,   // FR-172
    pub subjects: &'a [ModelSubject],     // one per alias, in `clause.aliases` order
    pub instance: &'a [ParameterBinding], // each object parameter bound to a universe key
}

pub fn encode_hyper_query(
    request: &HyperQueryRequest<'_>,
    kind: SmtQueryKind,
) -> Result<Vec<u8>, SmtEncodingRefusal>;
```

The request carries the clause, the subjects and one instance of FR-176's
HP-2 `ModelCheckRequest`, so CG passes what negotiation routed to it and one
query covers one instance. `depth` counts joint steps, the length ADR-023
HV-3 reads.

- **Copies.** The query SHALL declare, for each position `0..=depth` and
  each trace variable `t` in quantifier order, one copy of `t`'s subject:
  each field of each universe object as `s<i>.<t>.<object>.<field>`, with
  this requirement's sorts, range assertions and presence constants. Two
  variables over the same alias get two copies of the one subject. A body
  read through a monitor (below) also declares `m<i>` per position. The
  declarations SHALL be ordered by position, then trace variable in
  quantifier order, then object and field as for one subject, with `m<i>`
  after the copies at position `i`.
- **Initial states.** `InductionBase` SHALL assert each trace variable's
  subject's initial-state predicate on that variable's copy 0, one
  assertion per variable in quantifier order, so the initial joint states
  are every tuple of initial states (HC-1), and the initial monitor state on
  `m0`. `InductionStep` SHALL assert no initial-state predicate.
- **Joint steps.** For each step `i` to `i+1` the query SHALL assert one
  disjunction over tuples of component disjuncts, a tuple holding one
  disjunct of this requirement's step relation per trace variable (an
  operation with its receiver and argument vector, or the terminal stutter
  under infinite-trace) in quantifier order, the tuples ordered
  lexicographically by each component's disjunct order. Each disjunct is
  the conjunction of its component disjuncts, each over its variable's
  copies `i` and `i+1`. A tuple SHALL be a disjunct exactly when `μ_U`,
  evaluated through the one clause evaluator (FR-107) over the tuple's step
  labels (HS-5) under the request's instance, is true or undefined. `μ_U`
  reads no state, so the script prints no term for it. An empty `μ_U`
  admits every tuple.
- **Undefined match.** A tuple on which `μ_U` evaluates undefined SHALL be a
  definedness failure at position `i+1` when its component disjuncts hold
  between copies `i` and `i+1` (ADR-023 HV-8, ADR-018 UE-1).
- **Property.** The body's property at a joint position SHALL be read as a
  temporal item's is above: for a body that is `always` over a state
  predicate, the predicate there; otherwise the deterministic bad-prefix
  monitor that `qsl-eval`'s property-automaton translation builds for the
  body (ADR-018 SM-6) not rejecting at `m<i>`, with its steps asserted as
  for a TP-3 item. An indexed atom `e @ t` SHALL read `t`'s copy at the
  position (ADR-023 HM-4), and carries its definedness condition as any
  atom does.
- **Negated property.** The query SHALL assert the negated property as for
  a temporal item's `InductionBase` and `InductionStep`, over joint
  positions and with the undefined-match failures among the definedness
  conditions; for `InductionStep` the joint states at positions `0..depth`
  SHALL be pairwise distinct, some field of some copy or the monitor state
  differing between every two positions.
- **Fairness and copy-swap.** The query SHALL read no fairness set and
  SHALL apply no copy-swap canonicalisation (FR-174): it reads every joint
  state whole. Two clauses that differ only in their fairness sets give
  byte-equal queries. An unsatisfiable base and step at one depth show that
  no matched prefix reaches a violation or a definedness failure, which
  covers every fair matched tuple (ADR-023 HC-4).
- **Reading the results.** For one instance and one depth `k`:
  - `InductionBase{depth: k}` and `InductionStep{depth: k}` both
    unsatisfiable is the HP-2 outcome `Inductive{depth: k}`, method
    `k-induction`, which FR-182 settles V-3, `proved`, `decisive-witness`,
    `Uncertified`.
  - An unsatisfiable base and a satisfiable step is `InductionNotClosed{depth:
    k}`, V-6.
  - A satisfiable base is no outcome of these queries. Its model is a
    matched prefix of at most `k` joint steps that reaches a violation or an
    undefined letter or `μ_U`; it carries no lasso and need not extend to a
    fair matched tuple (HC-4), and FR-182 settles `refuted` only through
    FR-183's replay of a `HyperCounterexample`, which EN-2's searches build
    (ADR-023 HC-6, ADR-018 DS-3).
  - FR-314 rebuilds no hyper query: an HP-2 `Inductive` proof settles
    `Uncertified` whether or not a solver certificate comes with it
    (FR-182, ADR-023 HV-1).
  - The queries carry no reading of vacuity (ADR-023 HM-7): an
    unsatisfiable pair holds as well when no fair tuple matches `μ_U`.
- **Refusals.** `encode_hyper_query` SHALL refuse `SmtEncodingRefusal`,
  writing no script, naming the construct with its locus:
  - `hyper-form`, when FR-173 classifies the clause as other than HP-2;
  - `hyper-body`, when the body is outside the safety fragment (ADR-023
    HC-7), at the body;
  - `query-kind`, for `Unrolling`, since an HP-2 item has no V-2 verdict
    (FR-182);
  - `subject-binding`, when `subjects` does not hold exactly one subject
    per alias of `clause.aliases` with that alias's model, at the first
    alias that has none or the wrong one;
  - `match-conjunct`, when a `μ_U` conjunct evaluates refused or incomplete
    on some tuple, at the conjunct, naming the tuple;
  - and this requirement's field-sort and out-of-fragment refusals, per
    copy.
- Equal requests and kinds SHALL give byte-equal scripts.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-315-AC-1 | Over the `Counter` subject (universe `{c}`, initial value 0), `Unrolling{depth: 3}` for `always[0,3] holds(c.value <= 3)` declares `s0.c.value` to `s3.c.value` in order, asserts `(= s0.c.value 0)`, one transition assertion per step and the negated property, and ends with `(check-sat)`; encoding it twice gives byte-equal scripts. | Test (TC-900) |
| FR-315-AC-2 | `InductionStep{depth: 1}` for `always holds(c.value <= 3)` asserts no initial-state predicate, asserts the property at position 0 and its violation at position 1; `InductionBase{depth: 1}` asserts the initial-state predicate on copy 0. | Test (TC-900) |
| FR-315-AC-3 | An item over a field whose sort the encoding has no form for refuses `SmtEncodingRefusal` naming the field, with its locus, and writes no script. | Test (TC-900) |
| FR-315-AC-4 | Over a `Slot` subject (universe `{s}`, an optional `Int[0, 3]` field `held`, initially present with value 0; operation `drop` with frame `modifies [held]` that clears `held`, and operation `take` with precondition `pre Has { value(self.held) <= 3 }` that sets `held` to 1), `Unrolling{depth: 2}` for `always holds(value(s.held) <= 3)` makes the negated property hold when `(not s<i>.s.held.present)` holds at some position `i` in `0..=2`, since `value` of an absent field is undefined (`none-value`); the `take` disjunct's guard contributes the same definedness disjunct at the step that reads it. A solver therefore cannot refute the query once `drop` is reachable, and no `proved` follows for a claim UE-1 refutes. Every term of the script is linear. | Test (TC-900) |
| FR-315-AC-5 | `InductionStep{depth: 2}` for `always holds(c.value <= 3)` asserts that copies 0, 1 and 2 are pairwise distinct; for a TP-3 item it declares `m0` to `m2` and asserts the monitor steps, the property at a position being the monitor state there not rejecting. | Test (TC-900) |
| FR-315-AC-6 | Over `CasRefinesCounter`, the `Initial` query declares the concrete copy `s0`, asserts the initial-state predicate on it and the negation of `map(s0)` equalling an abstract initial state, and the `Step { row }` query for a `stutter` row declares `s0` and `s1`, asserts the operation's precondition on `s0` and its frame and postcondition over `s0`, `s1`, and the negation of the mapped state being unchanged; encoding either twice gives byte-equal scripts; an `any` row refuses `SmtEncodingRefusal`; a `Step` query for an abstract-operation row (`step Impl::Counter::commitA -> Spec::Counter::inc(self);`) asserts that operation's precondition, frame and postcondition over `map(s0)`, `map(s1)` with the row's receiver and arguments evaluated at `s0`, negated; a row with a `_` argument refuses `SmtEncodingRefusal` naming the row and the argument, shown over `CasRefinesCounterAdd`, which is `CasRefinesCounter` with `Spec::Counter` gaining the operation `add(n: Int[0, 1])` (frame `modifies [value]`, precondition `self.value + n <= 2`, postcondition `self.value = pre(self.value) + n`) and `Impl::Counter` gaining the operation `bump()` with an empty frame, like `peek`, and the row `step Impl::Counter::bump -> Spec::Counter::add(self, _);` (one receiver and one argument for the one abstract parameter, so the row types under FR-136; `bump` is the eighth step row and its position in the `steps` order is `r`); a mapping row outside `QF_LIA` refuses naming the row. | Test (TC-900) |
| FR-315-AC-7 | Over ADR-023 §8's secure vault and its §1 `NonInterference` clause for the instance `v`, `encode_hyper_query` with `InductionBase{depth: 1}` declares, in order, `s0.a.v.h`, `s0.a.v.l`, `s0.b.v.h`, `s0.b.v.l`, `s1.a.v.h`, `s1.a.v.l`, `s1.b.v.h`, `s1.b.v.l`, all `Int`, and no `m<i>`; asserts `a`'s initial-state predicate on copy 0 and then `b`'s; and asserts for step 0 to 1 one disjunction of exactly two joint disjuncts, `(step(0), step(0))` then `(step(1), step(1))`. With the `match` block removed the disjunction has four, in the order `(step(0), step(0))`, `(step(0), step(1))`, `(step(1), step(0))`, `(step(1), step(1))`. Encoding either twice gives byte-equal scripts, and every term is linear. | Test (TC-1120) |
| FR-315-AC-8 | For the same clause, `InductionStep{depth: 1}` asserts no initial-state predicate, the property `v.l @ a = v.l @ b` at position 0, the joint states at positions 0 and 1 distinct, and its violation at position 1; `InductionStep{depth: 2}` asserts positions 0, 1 and 2 pairwise distinct. The same clause with `fair { weak V::Vault::step }` on `a` gives byte-equal scripts to it for both kinds. | Test (TC-1120) |
| FR-315-AC-9 | `NonInterference` over the secure vault with FR-176-AC-4's added `match` conjunct, which is undefined on every tuple whose `a` step is `step(0)`, keeps `(step(0), step(0))` as a joint disjunct, and `InductionBase{depth: 1}` makes the negated property hold when that disjunct holds between copies 0 and 1. | Test (TC-1120) |
| FR-315-AC-10 | Each call refuses `SmtEncodingRefusal` and writes no script: an HP-3 clause (`forall trace a of V exists trace b of V`, FR-177) names `hyper-form`; FR-176-AC-3's clause `forall trace a of V { always eventually holds(v.l @ a = 0) }` names `hyper-body`; `Unrolling{depth: 1}` for `NonInterference` names `query-kind`; ADR-023 §1's `SavesPower` with one subject, for `S`, names `subject-binding` at alias `N`; `NonInterference` with a `match` conjunct that calls a function whose evaluation exhausts its meter names `match-conjunct` at that conjunct. | Test (TC-1120) |
| FR-315-AC-11 | Over the secure vault, a matched joint step sets `l` to the same input `i` in both copies, so `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for `NonInterference` are both unsatisfiable, the HP-2 outcome `Inductive{depth: 1}` that FR-182-AC-7 settles. Over a vault whose post clause is `self.l = self.h`, `InductionBase{depth: 1}` is satisfiable from the initial tuple `((0, 0), (1, 0))` with the joint step `(step(0), step(0))`, a matched bad prefix and not a counterexample, so no outcome follows from it. | Analysis (TC-1120) |

## Dependencies

- ADR-018 V-2, V-3, PC-6, LA-2, LA-3, DS-2.
- ADR-023 HC-1, HC-4, HC-6, HC-7, HM-4, HM-7, HS-5, HV-1, HV-3, HV-8, DS-3
  (the hyper queries).
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (the
  transition relation), [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (the subject), [FR-314](FR-314-check-an-smt-proof-certificate.md) (the
  checker that rebuilds the queries).
- [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md)
  (the checked hyper clause), [FR-173](FR-173-classify-hyper-clauses-into-forms.md)
  (its form), [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md)
  (the product the hyper queries encode), [FR-182](FR-182-settle-a-hyper-verdict.md)
  (the settlement of their outcomes).

## References

- SMT-LIB 2 standard.
- Owning ticket: Linear QSL-366; hyper self-composition queries: Linear
  STD-311.

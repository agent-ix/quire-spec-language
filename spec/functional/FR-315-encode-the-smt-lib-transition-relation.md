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
  - target: ix://agent-ix/quire-spec-language/FR-336
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
    pub clause: &'a CheckedHyperClause, // FR-172
    pub subjects: &'a [ModelSubject],   // one per alias, in `clause.aliases` order
    pub instance: &'a [(Name, Key)],    // FR-183's object-parameter binding
    pub meter: Meter,                   // FR-107's meter, charged by `μ_U`
}

pub struct HyperQuery {
    pub product: HyperProductIdentity, // the product and instance the script encodes
    pub kind: SmtQueryKind,
    pub script: Vec<u8>,               // the canonical SMT-LIB 2 script
}

pub fn encode_hyper_query(
    request: HyperQueryRequest<'_>,
    kind: SmtQueryKind,
) -> Result<HyperQuery, SmtEncodingRefusal>;
```

The request carries the clause and subjects of FR-176's HP-2 request and one
of the clause's instances, so one query covers one instance. `instance`
holds one `(parameter, key)` pair per member of `clause.parameters`, in
parameter order, each key drawn from FR-125's instancing of a hyper clause.
It is the same binding FR-183's `HyperCounterexample.instance` carries.
`meter` is the budget for evaluating `μ_U` while the query is encoded. The
caller sets it. The encoding evaluates `μ_U` once per tuple, in the
joint-step order below, and charges `meter` for each of those evaluations.
`μ_U` reads no state, so one evaluation serves every step `i`, and the
charge does not depend on `depth`. `depth` counts joint steps, the length ADR-023
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
- **Component disjuncts.** Each trace variable's component disjuncts SHALL
  be the disjuncts of this requirement's step relation for its subject: the
  operations with their receivers and argument vectors in that relation's
  order, then the terminal stutter last. Every `behaviours` clause is
  infinite-trace (ADR-023 HS-7), so the terminal stutter is always a
  component disjunct. It holds only at a terminal state, and its step label
  is `stutter` with every argument absent (HS-5).
- **Joint steps.** For each step `i` to `i+1` the query SHALL assert one
  disjunction over tuples of component disjuncts, one per trace variable in
  quantifier order. The tuples are ordered lexicographically by each
  component's disjunct order, the first variable varying slowest. Each
  joint disjunct is the conjunction of its component disjuncts, each over
  its variable's copies `i` and `i+1`. A tuple SHALL be a joint disjunct
  exactly when `μ_U`, evaluated through the one clause evaluator (FR-107)
  over the tuple's step labels (HS-5) under the request's instance, is true
  or undefined. `μ_U` reads no state, so the script prints no term for it.
  An empty `μ_U` admits every tuple.
- **Undefined match.** A tuple on which `μ_U` evaluates undefined SHALL be a
  definedness failure at position `i+1` when its component disjuncts hold
  between copies `i` and `i+1` (ADR-023 HV-8, ADR-018 UE-1).
- **Property.** The body's property at a joint position SHALL be read the
  way this requirement reads a temporal item's property. For a body that
  is `always` over a state predicate, the property is the predicate at that
  position. For any other body, it is the deterministic bad-prefix monitor
  that `qsl-eval`'s property-automaton translation builds for the body
  (ADR-018 SM-6) not rejecting at `m<i>`, with its steps asserted as for a
  TP-3 item. An indexed atom `e @ t` SHALL read `t`'s copy at the position
  (ADR-023 HM-4), and it carries its definedness condition as any atom
  does.
- **Negated property.** The query SHALL assert the negated property as for
  a temporal item's `InductionBase` and `InductionStep`, over joint
  positions and with the undefined-match failures among the definedness
  conditions. For `InductionStep`, the property and every definedness
  condition hold at positions `0..depth`, the joint states at positions
  `0..=depth` SHALL be pairwise distinct (some field of some copy, or the
  monitor state, differs between every two positions), and a violation or
  a definedness failure holds at `depth`.
- **Fairness and copy-swap.** The query SHALL read no fairness set and
  SHALL apply no copy-swap canonicalisation (FR-174): it reads every joint
  state whole. Two clauses that differ only in their fairness sets give
  byte-equal queries. An unsatisfiable base and step at one depth show that
  no matched prefix reaches a violation or a definedness failure, which
  covers every fair matched tuple (ADR-023 HC-4).
- **Reading the results.** For one instance and one depth `k`:
  - `InductionBase{depth: k}` and `InductionStep{depth: k}` both
    unsatisfiable is the HP-2 outcome `Inductive{depth: k}`, method
    `k-induction`. These queries carry no reading of vacuity (ADR-023
    HM-7): the pair is also unsatisfiable when no fair tuple matches `μ_U`.
    The pair alone is therefore not enough for V-3 while `μ_U` is
    non-empty. FR-182 settles `Inductive{depth: k}` as V-3 `proved`,
    `decisive-witness`, `Uncertified` only when `μ_U` is empty or an
    accepted `MatchWitness` (FR-183) for the same instance comes with it.
    Over a subject with no initial state it settles V-6 `NoInitialState`.
    Otherwise it settles V-6 `VacuousMatch`, whose meaning FR-182 states:
    no matched fair tuple has been established.
  - An unsatisfiable base and a satisfiable step is `InductionNotClosed{depth:
    k}`, V-6.
  - A satisfiable base gives no outcome by itself. Its model is a matched
    prefix of at most `k` joint steps that reaches a violation or a
    definedness failure.
    - When the failure is an undefined letter, guard or `μ_U`, the prefix
      is HV-8 refuting evidence that needs no lasso. Concretised into an
      `Undefined` `HyperCounterexample` (one finite path per variable,
      ending at the failing position), it settles `refuted` once FR-183
      replays it.
    - When the failure is a violation of the body, the prefix need not
      extend to a fair matched tuple (HC-4). The item settles `refuted`
      only through FR-183's replay of a `Lockstep` counterexample, which
      EN-2's lasso searches build (ADR-023 HC-6, ADR-018 DS-3).
    - When EN-2's search to depth `k` completes with no replayable lasso,
      the outcome is `BoundReached{depth: k}`, V-5 (ADR-023 HV-3).
  - FR-314 rebuilds no hyper query: an HP-2 `Inductive` proof settles
    `Uncertified` whether or not a solver certificate comes with it
    (FR-182, ADR-023 HV-1).
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
  - `match-conjunct`, when a `μ_U` conjunct evaluates refused on some tuple,
    or is incomplete because `meter` ran out during its one evaluation for
    some tuple, at the conjunct, naming the tuple;
  - and this requirement's field-sort and out-of-fragment refusals, per
    copy.
- Equal requests, with meters of equal budget, and equal kinds SHALL give
  byte-equal scripts or equal refusals.

#### Product identity

`HyperProductIdentity` SHALL be QSL's one public identity of the HP-2
product and instance a hyper query encodes. It is the value that passes
from `encode_hyper_query` through CG's SMT backend to FR-182's settlement,
so every SMT outcome names the exact item and instance it answers. It SHALL
be a type of the `qsl-replay` facade, at `qsl_replay::HyperProductIdentity`,
beside `HyperQueryRequest`, `HyperQuery`, `SmtQueryKind`,
`SmtEncodingRefusal` and FR-336's `SubjectIdentity`, which the facade also
holds. CG names all of them through its one QSL dependency (FR-059, FR-060
T12-A).

```rust
pub struct HyperProductIdentity { /* private members */ } // Clone, Debug, Eq, Hash

impl HyperProductIdentity {
    pub fn of(request: &HyperQueryRequest<'_>) -> Result<Self, SmtEncodingRefusal>;
    pub fn clause(&self) -> &OccurrenceKey;       // the clause's `hyperproperty` claim occurrence
    pub fn subjects(&self) -> &[SubjectIdentity]; // FR-336's projection, one per alias in `clause.aliases` order
    pub fn instance(&self) -> &[(Name, Key)];     // the request's instance, in parameter order
}
```

- **Members.** The identity SHALL hold exactly three members:
  - `clause`: the `claim` occurrence key (ADR-013 O-07) of the clause's
    `hyperproperty` node in the checked-package v2 emission of the
    subjects' checked package. Its node id binds every member that FR-172
    "Identity" and ADR-023 HV-7 name: the aliases in order, the quantifier
    prefix with each variable's kind, alias and fairness set, `μ`, the
    `align skip` list and the body.
  - `subjects`: FR-336's identity projection of each subject in
    `clause.aliases` order, which binds its checked package, initial
    states, universes, domain packages and proof bounds.
  - `instance`: the request's `instance`.
- **No digest of its own.** The identity SHALL be made only of these
  existing canonical identities. It SHALL NOT be or carry a digest over its
  members, and it SHALL have no digest domain. Two identities are equal
  exactly when their three members are equal.
- **QSL mints it.** Only `HyperProductIdentity::of` and `encode_hyper_query`
  SHALL make a `HyperProductIdentity`. The facade SHALL expose no
  constructor from members and no deserializer for it, so a caller cannot
  mint one (ADR-013 O-09: no re-mint of a clause identity). CG keeps the
  value it receives and returns it unchanged with the outcome.
- **What it leaves out.** The query kind, the depth, the meter and every run
  limit SHALL be no part of the identity: they choose a query of the
  product, or decide whether a run finishes, and `HyperQuery.kind` names the
  query. Two clauses that differ only in a fairness set give byte-equal
  scripts and different identities, because they are different items
  (FR-172-AC-4).
- **Encoding returns it.** `encode_hyper_query` SHALL compute the identity
  before it writes any script and return it as `HyperQuery.product`, equal
  to `HyperProductIdentity::of(&request)`. The base and step queries of one
  request therefore carry equal identities.
- **Refusals.** `HyperProductIdentity::of` and `encode_hyper_query` SHALL
  each refuse `SmtEncodingRefusal`, with no identity and no script:
  - `subject-binding`, as above;
  - `instance-binding`, when `instance` does not hold exactly one pair per
    member of `clause.parameters`, in parameter order, each key in that
    parameter's universe, at the first parameter that breaks this;
  - `product-identity`, when two subjects carry different checked
    packages, when the subjects' checked package has no checked-package v2
    emission, or when that emission has no `hyperproperty` node for the
    clause, at the clause.
  `HyperProductIdentity::of` SHALL evaluate no `μ_U` conjunct and charge
  nothing to `meter`. For one request it SHALL give the same refusal that
  `encode_hyper_query` gives when that refusal is one of these three.
- **Outcome handoff.** The HP-2 SMT outcome CG returns to FR-182
  (`Inductive{depth}` or `InductionNotClosed{depth}`) SHALL carry the
  `product` of the base and step `HyperQuery` values it answers, and those
  two SHALL carry equal identities and equal depths. FR-182 settles an
  outcome only for the item and instance its identity names.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-315-AC-1 | Over the `Counter` subject (universe `{c}`, initial value 0), `Unrolling{depth: 3}` for `always[0,3] holds(c.value <= 3)` declares `s0.c.value` to `s3.c.value` in order, asserts `(= s0.c.value 0)`, one transition assertion per step and the negated property, and ends with `(check-sat)`; encoding it twice gives byte-equal scripts. | Test (TC-900) |
| FR-315-AC-2 | `InductionStep{depth: 1}` for `always holds(c.value <= 3)` asserts no initial-state predicate, asserts the property at position 0 and its violation at position 1; `InductionBase{depth: 1}` asserts the initial-state predicate on copy 0. | Test (TC-900) |
| FR-315-AC-3 | An item over a field whose sort the encoding has no form for refuses `SmtEncodingRefusal` naming the field, with its locus, and writes no script. | Test (TC-900) |
| FR-315-AC-4 | Over a `Slot` subject (universe `{s}`, an optional `Int[0, 3]` field `held`, initially present with value 0; operation `drop` with frame `modifies [held]` that clears `held`, and operation `take` with precondition `pre Has { value(self.held) <= 3 }` that sets `held` to 1), `Unrolling{depth: 2}` for `always holds(value(s.held) <= 3)` makes the negated property hold when `(not s<i>.s.held.present)` holds at some position `i` in `0..=2`, since `value` of an absent field is undefined (`none-value`); the `take` disjunct's guard contributes the same definedness disjunct at the step that reads it. A solver therefore cannot refute the query once `drop` is reachable, and no `proved` follows for a claim UE-1 refutes. Every term of the script is linear. | Test (TC-900) |
| FR-315-AC-5 | `InductionStep{depth: 2}` for `always holds(c.value <= 3)` asserts that copies 0, 1 and 2 are pairwise distinct; for a TP-3 item it declares `m0` to `m2` and asserts the monitor steps, the property at a position being the monitor state there not rejecting. | Test (TC-900) |
| FR-315-AC-6 | Over `CasRefinesCounter`, the `Initial` query declares the concrete copy `s0`, asserts the initial-state predicate on it and the negation of `map(s0)` equalling an abstract initial state, and the `Step { row }` query for a `stutter` row declares `s0` and `s1`, asserts the operation's precondition on `s0` and its frame and postcondition over `s0`, `s1`, and the negation of the mapped state being unchanged; encoding either twice gives byte-equal scripts; an `any` row refuses `SmtEncodingRefusal`; a `Step` query for an abstract-operation row (`step Impl::Counter::commitA -> Spec::Counter::inc(self);`) asserts that operation's precondition, frame and postcondition over `map(s0)`, `map(s1)` with the row's receiver and arguments evaluated at `s0`, negated; a row with a `_` argument refuses `SmtEncodingRefusal` naming the row and the argument, shown over `CasRefinesCounterAdd`, which is `CasRefinesCounter` with `Spec::Counter` gaining the operation `add(n: Int[0, 1])` (frame `modifies [value]`, precondition `self.value + n <= 2`, postcondition `self.value = pre(self.value) + n`) and `Impl::Counter` gaining the operation `bump()` with an empty frame, like `peek`, and the row `step Impl::Counter::bump -> Spec::Counter::add(self, _);` (one receiver and one argument for the one abstract parameter, so the row types under FR-136; `bump` is the eighth step row and its position in the `steps` order is `r`); a mapping row outside `QF_LIA` refuses naming the row. | Test (TC-900) |
| FR-315-AC-7 | Over ADR-023 §8's secure vault and its §1 `NonInterference` clause for the instance `v`, `encode_hyper_query` with `InductionBase{depth: 1}` declares, in order, `s0.a.v.h`, `s0.a.v.l`, `s0.b.v.h`, `s0.b.v.l`, `s1.a.v.h`, `s1.a.v.l`, `s1.b.v.h`, `s1.b.v.l`, all `Int`, and no `m<i>`; asserts `a`'s initial-state predicate on copy 0 and then `b`'s; and asserts for step 0 to 1 one disjunction of exactly three joint disjuncts, `(step(0), step(0))`, `(step(1), step(1))`, then `(stutter, stutter)`. Each component's disjunct order is `step(0)`, `step(1)`, `stutter`, and `μ_U` holds on `(stutter, stutter)` because both operations are `stutter` and both arguments are absent. With the `match` block removed, the disjunction has all nine tuples in lexicographic order, `(step(0), step(0))` first and `(stutter, stutter)` last. Encoding either twice gives byte-equal scripts, and every term is linear. | Test (TC-1120) |
| FR-315-AC-8 | For the same clause, `InductionStep{depth: 1}` asserts no initial-state predicate, the property `v.l @ a = v.l @ b` at position 0, the joint states at positions 0 and 1 distinct, and its violation at position 1; `InductionStep{depth: 2}` asserts positions 0, 1 and 2 pairwise distinct. The same clause with `fair { weak V::Vault::step }` on `a` gives byte-equal scripts to it for both kinds. | Test (TC-1120) |
| FR-315-AC-9 | `NonInterference` over the secure vault with FR-176-AC-4's added `match` conjunct, which is undefined on every tuple whose `a` step is `step(0)`, keeps `(step(0), step(0))` as a joint disjunct, and `InductionBase{depth: 1}` makes the negated property hold when that disjunct holds between copies 0 and 1. | Test (TC-1120) |
| FR-315-AC-10 | Each call refuses `SmtEncodingRefusal` and writes no script: an HP-3 clause (`forall trace a of V exists trace b of V`, FR-177) names `hyper-form`; FR-176-AC-3's clause `forall trace a of V { always eventually holds(v.l @ a = 0) }` names `hyper-body`; `Unrolling{depth: 1}` for `NonInterference` names `query-kind`; ADR-023 §1's `SavesPower` with one subject, for `S`, names `subject-binding` at alias `N`; `NonInterference` with a `match` conjunct that calls a function whose evaluation exhausts the request's `meter` names `match-conjunct` at that conjunct and the tuple `(step(0), step(0))`, the first in the joint-step order. The same request with an equal budget gives the same refusal. | Test (TC-1120) |
| FR-315-AC-11 | Over the secure vault, a matched joint step sets `l` to the same input `i` in both copies, so `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for `NonInterference` are both unsatisfiable, the HP-2 outcome `Inductive{depth: 1}` that FR-182-AC-7 settles. Over a vault whose post clause is `self.l = self.h`, `InductionBase{depth: 1}` is satisfiable from the initial tuple `((0, 0), (1, 0))` with the joint step `(step(0), step(0))`. That model is a matched bad prefix, not a `HyperCounterexample`: it has no lasso, so no outcome follows from it alone, and the clause is refuted only through EN-2's lasso and FR-183's replay. | Analysis (TC-1120) |
| FR-315-AC-12 | Over ADR-023 §8's secure vault with the `vaults` universe `{v, w}`, `NonInterference`'s `InductionBase{depth: 1}`, `InductionStep{depth: 1}` and `InductionStep{depth: 2}` queries for instance `v` carry three equal `product` values, each equal to `HyperProductIdentity::of` over the same request. Its `clause()` is the `claim` occurrence of the clause's `hyperproperty` node in the vault package's checked-package v2 emission; its `subjects()` holds one member, equal to FR-336's projection of the vault subject; its `instance()` is the one pair binding the clause's parameter to `v`. The request for instance `w` gives an identity that differs only in `instance()`. The clause with `fair { weak V::Vault::step }` on `a` gives byte-equal scripts and an identity that differs only in `clause()`. A subject that differs only in a proof bound gives an identity that differs only in `subjects()`. Meters of different budgets give equal identities. | Test (TC-1190) |
| FR-315-AC-13 | Each of these calls refuses `SmtEncodingRefusal`, from both `HyperProductIdentity::of` and `encode_hyper_query`, with no identity and no script: `NonInterference` with an empty `instance` names `instance-binding` at the clause's parameter; with the key `x`, outside the universe `{v}`, it names `instance-binding` at that parameter; ADR-023 §1's `SavesPower` with its `S` and `N` subjects carrying two different checked packages names `product-identity` at the clause. `HyperProductIdentity::of` over `NonInterference` with a `match` conjunct whose evaluation exhausts the request's `meter` returns the identity and leaves the meter's consumed counters at zero. | Test (TC-1190) |
| FR-315-AC-14 | A crate whose only QSL dependency is `qsl-replay` names `qsl_replay::HyperProductIdentity`, `HyperQuery`, `HyperQueryRequest` and `SubjectIdentity`, calls `encode_hyper_query` and `HyperProductIdentity::of`, and compiles. In that crate a struct literal of `HyperProductIdentity`, or deserializing one, fails to compile. | Test (TC-1190) |

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
- [FR-336](FR-336-bound-a-model-subject-by-its-universes.md) (the subject
  identity projection), ADR-013 O-07 and O-09 (the clause occurrence and
  its no-re-mint rule), [FR-059](FR-059-check-backend-dependency-direction.md)
  and [FR-060](FR-060-check-qsl-api-surface-boundary.md) (CG names QSL only
  through `qsl-replay`).

## References

- SMT-LIB 2 standard.
- Owning ticket: Linear QSL-366; hyper self-composition queries: Linear
  STD-311; hyper product identity: Linear STD-336.

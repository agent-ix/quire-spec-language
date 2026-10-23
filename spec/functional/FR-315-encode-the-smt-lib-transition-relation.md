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
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-128
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
- A `ModelCheckItem` (FR-126): a checked temporal clause with its property
  form (FR-123), or the subject's deadlock-freedom item (FR-124).
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

The call takes no meter and no limit setting. The encoding evaluates no
clause: it prints terms over the subject's state copies, so FR-107's meter
has nothing to charge. Its size limits (below) are fixed constants of this
requirement, because FR-314's checker rebuilds the query from the recompiled
package, the subject, the item and the kind alone, and a request setting
would be one more input that a certificate would have to carry.

## Outputs

- The query's SMT-LIB 2 script in canonical printing.
- `SmtEncodingRefusal`, with no script, naming the construct and its locus,
  when the item or the subject uses a construct the encoding has no form
  for. Each refusal of `encode_smt_query` has one code. The encoding SHALL
  apply the checks in the order of this table, one row at a time, and SHALL
  refuse on the first row that applies:

| Order | Code | When | Locus |
| --- | --- | --- | --- |
| 1 | `item-form` | The item is a TP-4 liveness clause, whose refutations are EN-2's lassos; a TP-2 `on each` clause; a TP-2 `on origin` clause other than `always[0,u] holds(I)` or `eventually[0,u] holds(I)` over a state predicate `I`; a clause with no recorded property form; a clause whose activation is guarded by a `when` condition; or a clause whose property-automaton translation refuses, carrying that refusal | the clause |
| 2 | `profile` | The clause binds a profile other than infinite-trace and event-position false-extension, such as ADR-026's `model-time` | the clause |
| 3 | `query-kind` | `InductionBase` or `InductionStep` for a TP-2 `on origin` item, whose V-2 verdict needs no induction; `Unrolling{depth}` with `depth < u` for `eventually[0,u] holds(I)` (Finite behaviours) | the clause |
| 4 | `subject-admission` | FR-120's admission of the subject refuses, carrying that refusal | the subject |
| 5 | `no-initial-state` | The subject's `initial` list is empty (FR-125), so every query would be unsatisfiable | the subject |
| 6 | `object-lifecycle` | An operation's frame creates or deletes objects, or a universe object is not held by every initial state at one type, so the objects of a state copy are not fixed | the operation, or the object |
| 7 | `constant-name` | A name cannot be printed as an SMT-LIB symbol, or two constants would print the same name (two objects under one key, or a dotted key that prints as another object's field) | the name |
| 8 | `field-sort` | A field of a universe object has a sort with no SMT form (FR-315-AC-3) | the object and field |
| 9 | `parameter-domain` | An operation parameter, or an operation's declared result, has no finite domain under FR-120 or a type with no SMT form | the operation, and the parameter or `result` |
| 10 | `field-domain` | The query expands the candidates of an application (Finite behaviours, Deadlock-freedom, Undefined evaluation) and a field the application may write has no finite domain under FR-120, such as an `Integer` field | the object and field |
| 11 | `requires-bound` | A population that admits the clause's `over` type has no universe (FR-125 `RequiresBound`) | the population |
| 12 | `no-instance` | The clause has zero instances (FR-125 `NoInstance`), so every query would be unsatisfiable | the clause |
| 13 | `instance-type` | An instance's object does not conform to the `over` type in an initial state | the instance |
| 14 | `several-instances` | A TP-3 item has more than one instance: the encoding builds one monitor, for one instance | the clause |
| 15 | `too-large` | The `constants`, then `applications`, then `monitor-atoms`, then `monitor-states`, then `candidates` measure passes its limit (Size limits) | the measure, its value and its limit |
| 16 | `out-of-fragment` | An expression is outside the `QF_LIA` fragment the encoding prints, such as a product of two non-literal terms or a non-literal divisor | the expression's clause and node |
| 17 | `too-large` | The `script-bytes` measure passes its limit | the measure, its value and its limit |

Of several constructs that one row refuses, the encoding names the first in
the canonical order of the script: object, field and operation order for
the subject rows, instance order for the instance rows, and assertion order
for `out-of-fragment`.

The codes of `SmtEncodingRefusal` are exactly these: the codes of this
table; `refinement-any-row`, `refinement-open-argument` and
`out-of-fragment` for `encode_refinement_query` (Refinement queries); and
`hyper-form`, `hyper-body`, `query-kind`, `subject-binding`,
`instance-binding`, `product-identity`, `match-conjunct`, `field-sort`,
`parameter-domain`, `field-domain`, `out-of-fragment` and `too-large` for `encode_hyper_query` and
`HyperProductIdentity::of` (Hyper self-composition queries).
`instance-binding` means a malformed `instance` of a hyper request only;
the single-subject check of an instance's object is `instance-type`.

## Behavior

### State copies

- The encoding SHALL declare one state copy per position `0..=depth`, each
  field of each universe object as one constant of the field's SMT sort
  (`Int` for integer and enumeration fields with range assertions, `Bool`
  for Boolean fields), named `s<i>.<object>.<field>`; an optional field is
  a value constant of its element's sort and a `Bool` presence constant
  `s<i>.<object>.<field>.present`. Every term the encoding prints is linear,
  so each query is a `QF_LIA` script.
- For a TP-3 item the encoding SHALL also declare one monitor-state constant
  `m<i>` per position. For a TP-2 `on origin` item it SHALL declare one
  `Bool` constant `h<i>` per position, true when position `i` lies past the
  end of a finite behaviour (Finite behaviours).
- **Field equality.** Two copies `k` and `j` agree on a field when the
  field's value is the same in both. For a required field the encoding SHALL
  print this as `(= s<k>.<object>.<field> s<j>.<object>.<field>)`, the later
  copy first. For an optional field it SHALL print
  `(and (= P<k> P<j>) (or (not P<k>) (= V<k> V<j>)))`, with `P` the presence
  constant and `V` the value constant. An absent field's value constant is
  not part of the state, so two copies that hold one state agree on every
  field whatever their absent fields' value constants hold. The stutter,
  the frame and the distinctness assertions compare fields only by field
  equality.

### Step relation

- The encoding SHALL assert, for each step `i` to `i+1`, the disjunction of
  the subject's step disjuncts between copies `i` and `i+1`. An operation
  disjunct is one application: an operation applied to a receiver and an
  argument vector, conjoining the operation's precondition clauses on copy
  `i`, its postcondition clauses relating copies `i` and `i+1`, and its
  frame (FR-120, FR-125). The frame is field equality, copy `i+1` with copy
  `i`, for every field the frame does not let the application write.
- **Results.** For an operation that declares a result, FR-120 pairs each
  candidate with each value of the result's domain, and a candidate is a
  successor when some pair satisfies the postcondition. The encoding SHALL
  print the postcondition part of the disjunct as the disjunction, over the
  result values in the result type's value-domain order, of the conjunction
  of the postcondition clauses read with that value as a literal for
  `result`. It declares no constant for a result. For an operation with no
  result the postcondition part is the conjunction of its postcondition
  clauses.
- The disjuncts SHALL be ordered by receiver key, then operation name, then
  argument vector in FR-120's domain order (the first parameter most
  significant, each parameter's values in its domain order), with the
  stutter disjunct last. Within an operation disjunct the conjuncts SHALL be
  the precondition clauses in ascending name order, then the postcondition
  part, its clauses in ascending name order, then the frame's equalities in
  (object, field) order. Each clause is printed as its definedness condition
  conjoined with its value, so an undefined clause holds in no disjunct.
- **Proof-query stutter.** Under infinite-trace the step relation SHALL have
  one more disjunct, the proof-query stutter: the conjunction of field
  equality, copy `i+1` with copy `i`, for every field in declaration order.
  The stutter SHALL have no enabledness conjunct, so it holds between equal
  copies at every state, terminal or not. SM-4's terminal stutter is the
  stutter taken at a terminal state. Reading "terminal" exactly needs "no
  candidate post-state satisfies the postcondition", which the encoding
  states only by expanding the candidates (Finite behaviours), at a size
  that grows with the product of the written fields' domains; the stutter
  needs no expansion. It over-approximates the successor relation and adds
  no state: every state a path through it reaches is reachable without it.
  An unsatisfiable query is therefore unsatisfiable for the exact relation
  too, so a refutation of it stays a proof. Every copy has a continuation by
  the stutter, so no path is cut short before `depth`. The step case's
  pairwise distinctness excludes a stutter between two positions with equal
  monitor states.
- **What the stutter costs.** A TP-1 property and every TP-3 property
  without an interval or previous-position operator are stutter-invariant,
  so a stutter at a non-terminal state cannot make them fail where the exact
  relation does not. A TP-3 property with an interval operator is not
  stutter-invariant (ADR-018 IV-7). For such an item the stutter can give a
  satisfiable query where the exact relation gives none: over
  FR-315-AC-1's `Counter`, `always (holds(c.value = 0) implies
  eventually[0,1] holds(c.value = 1))` holds, yet the base query is
  satisfiable at every depth by stuttering at value 0, so its `k`-induction
  never closes and the item is settled by EN-1. The encoding accepts this
  loss of completeness and keeps soundness.
- **A model that stutters at a non-terminal state.** A satisfiable query is
  not a refutation by itself. A model whose stutter is taken at a copy that
  is not terminal is a path of the over-approximation only, and it is no
  counterexample candidate. CG's SMT backend SHALL find each stutter step
  of a model and send the model to FR-128's replay only when every stutter
  step is at a terminal state. CG decides terminality of a concrete state
  through the replay facade's transition enumeration (FR-128, CX-3), and
  reads a stutter at a state with an enabled transition as "no candidate",
  never as `ReplayParity` or `ReplayRefused`. A base or step query that has
  only such models settles as an undecided query does,
  `InductionNotClosed{depth}` (V-6). An `Unrolling` query with only such
  models gives no outcome at that depth. The item settles `refuted` only
  through FR-128's replay of a counterexample built from a model.

### Finite behaviours

Under event-position false-extension (TP-2 `on origin`) a behaviour that
reaches a terminal state is closed there (ADR-018 SM-4), and the positions
after it are read by the closed-boundary rule, every atomic predicate false
(QSpec FR-091).

- **Candidates.** The candidates of an application are the post-states
  FR-120 enumerates for it: every field the frame does not let it write
  keeps copy `i`'s value, and each field it may write takes each value of
  its domain, absent included for an optional field. The encoding SHALL
  order them by the written fields in (object, field) order, the first most
  significant, each field's values in FR-120's value-domain order (integers
  ascending, enumeration members in declaration order, `false` before
  `true`), absent first for an optional field. This printing order is the
  encoding's own: it is numeric where a key's byte order is not, and the
  disjunctions it orders are order-independent in meaning. A candidate is
  read as literals: each written field's constant of copy `i+1` is replaced
  by its value, an absent field's presence by `false`. For an operation that
  declares a result the encoding SHALL read each (candidate, result) pair,
  candidate-major, the results in value-domain order.
- An application is **enabled** at copy `i` exactly when its precondition
  clauses all hold on copy `i` and some candidate, with some result value for
  an operation that declares one, satisfies all of its postcondition
  clauses. The encoding SHALL print enabledness as
  `(and <precondition clauses> (or <pair 1> ... <pair n>))`, each pair the
  conjunction of the postcondition clauses read with that pair's literals.
  The candidates are finite, so this is the exact existential, with no
  quantifier. Copy `i` is **terminal** when no application is enabled there.
  The encoding SHALL print it as `(not (or E1 ... En))`, `E1` to `En` the
  applications' enabledness in step-disjunct order.
- The encoding SHALL assert `(not h0)` after the initial-state predicate,
  and SHALL print the step assertion for step `i` as exactly
  `(and (= h<i+1> (or h<i> T)) (or h<i+1> D))`, with `T` the print of copy
  `i` terminal and `D` the disjunction of the operation disjuncts between
  copies `i` and `i+1`. The step relation under false-extension has no
  stutter disjunct. A copy past the end (`h<i>` true) is constrained only by
  its range assertions.
- Every live copy (one with `h<i>` false) either has an operation successor
  or is terminal, so every behaviour, finite or not, has a model path to
  `depth`, and no trace that ends before `depth` drops out of the query.
- The property of `always[0,u] holds(I)` at a position `p` is `I` there for
  every instance, and is false at a position past the end. Its violation is
  `h<p>` or the negation of `I` for some instance, at some position `p` in
  `0..=min(depth, u)`. Below its horizon this query is a bounded search: a
  violation at `p <= depth` is a bad prefix whatever follows.
- The violation of `eventually[0,u] holds(I)` is, for some instance, `h<p>`
  or the negation of `I` at every position `p` in `0..=u`. `Unrolling{depth}`
  with `depth < u` SHALL refuse `query-kind`: it would read "`I` has not
  held yet" as a violation, so a satisfiable answer would be no counterexample,
  and ADR-018 V-2 reads the query only at `depth >= u`.

### Deadlock-freedom

- The deadlock-freedom item is the TP-1 invariant `always holds(not
  deadlocked)` (ADR-018 DL-3), under infinite-trace. The encoding SHALL read
  `deadlocked` at a copy as DL-7 states: the copy is terminal, by the
  candidate expansion of Finite behaviours, and DL-1's `P` is false there,
  `P` being false when the subject declares no `terminal when P`. `P` is read
  only at a terminal copy, and its definedness is a definedness condition
  there (UE-1). The item has no instance.

### Monitors

- For a TP-3 item the property is read through the deterministic bad-prefix
  monitor that `qsl-eval`'s property-automaton translation builds (ADR-018
  SM-6, EN-3, LA-2). The encoding SHALL number the monitor states in the
  order a breadth-first tabulation reaches them from the initial monitor
  state, the initial state being 0, over letters in ascending numeric order,
  bit `j` of a letter being the value of the translation's atom `j`. It
  SHALL assert `0 <= m<i> <= n-1` for `n` tabulated states, and one
  implication per tabulated move: at position `i > 0`, `m<i-1>` at the
  move's source and the letter of position `i` imply `m<i>` at its target;
  at position 0, for `Unrolling` and `InductionBase` only, the letter of
  position 0 implies `m0` at the target of the move from the initial state.
- `m<i>` is therefore the monitor state after reading the letter of
  position `i`, EN-1's product convention: `m0` is the initial monitor state
  stepped over the letter of position 0, not the initial monitor state
  itself. Under `InductionStep`, `m0` is constrained by its range assertion
  only, which over-approximates the states the step case starts from. The
  property at a position is `m<i>` differing from the rejecting state's
  number. A monitor whose rejecting state is not reached by the tabulation
  has no violation term.

### Instances

- A query SHALL cover every instance of the item together. The property at
  a position is the conjunction over the item's instances, in FR-125's
  canonical order, and its violation is the disjunction of the instances'
  violations in that order. The query attributes no failure to an instance:
  a model shows that some instance fails, and the counterexample built from
  it names its `over_binding` (ADR-018 CX-2), which replay checks. A TP-3
  item SHALL have exactly one instance (`several-instances`).

### Undefined evaluation

- The encoding SHALL read undefinedness as a refutation, never as false or
  as a disabled step (ADR-018 UE-1, UE-4, UE-6). For each atom the claim
  reads, it SHALL build the atom's definedness condition (a divisor not zero,
  a `value` of a present option, a reduction over a non-empty collection, a
  value inside its sort's range), reading `and`, `or`, `implies` and `if`
  under FR-006's short-circuit.
- A **definedness failure** at position `p` is any of these:
  - an atom the claim reads at `p` is undefined. Under false-extension the
    claim reads positions `0..=min(depth, u)` only (UE-1: a TP-2 `on origin`
    claim reads positions 0 to `h`), and only at a live copy, since a
    position past the end evaluates no atom;
  - an application's precondition is undefined on copy `p`, read as FR-120
    reads it: some clause, in ascending name order, is undefined while every
    clause before it holds (a false clause before it leaves the precondition
    false, with no failure);
  - an application is enabled by its precondition on copy `p` and has a
    candidate (with a result value, for an operation that declares one) on
    which no postcondition clause is false and some postcondition clause is
    undefined, FR-120's `ContractUndetermined` on a candidate. The encoding
    SHALL print it as `(and <precondition clauses> (or <pair 1> ...
    <pair n>))`, each pair that condition read with the pair's literals,
    the pairs in the candidate order of Finite behaviours;
    for an application none of whose postcondition clauses holds a
    construct with a definedness condition (a division or modulo, a `value`
    read, a reduction or a range coercion), it prints no term;
  - for the deadlock-freedom item, `P` is undefined at a terminal copy `p`.
- Under false-extension every precondition and postcondition failure at `p`
  SHALL be conjoined with `(not h<p>)`, printed as `(and (not h<p>) F)`: a
  copy past the end is no reachable state, and a failure there would make the
  query satisfiable with no counterexample.
- Each failure at `p` reads copy `p` and literals only. It needs no
  successor of copy `p`: a failing guard or postcondition need not leave a
  step that the transition assertion admits, and the path after `p` reaches
  `depth` by the stutter under infinite-trace, and by an operation step or
  the end under false-extension.

### Negated property

- The encoding SHALL assert the negated property: for `Unrolling` and
  `InductionBase`, a violation or a definedness failure at some position
  `0..=depth` (for a TP-2 item, violations and the claim's atom failures
  within the positions Finite behaviours and Undefined evaluation name);
  for `InductionStep`, the property and the absence of every definedness
  failure at each position `0..depth`, the copies at positions `0..=depth`
  pairwise distinct (some field, by field equality, or the monitor state
  differs between every two copies), and a violation or a definedness
  failure at `depth`, the guard and postcondition failures of the
  applications from copy `depth` included (ADR-018 EN-3). Base and step at
  one depth `k` thus cover every definedness failure at every reachable
  state: the base covers positions `0..=k`, and the step covers the first
  failing state of a shortest path, whatever its distance.
- The negated property is one disjunction. Its disjuncts SHALL be, for each
  position in ascending order: the violation there, per instance in
  instance order (for an `eventually[0,u]` item, its per-instance violations
  stand first, before every position); then the atoms' definedness failures,
  per instance in instance order and atom in clause-node order; then the
  precondition failures, then the postcondition failures, of the
  applications in step-disjunct order; then, for the deadlock-freedom item,
  `P`'s failure. A disjunct equal to an earlier one is
  printed once, at its first place.
- The encoding owns only these proof queries. EN-2's counterexample
  searches, including its lasso queries with a loop back-edge and fairness
  on the loop, are the SMT backend's own (ADR-018 DS-3); their refutations
  replay through FR-128 and need no certificate.

### Canonical printing

- The canonical printing SHALL be, in this order:
  1. `(set-logic QF_LIA)`;
  2. the `declare-const` commands, position by position in ascending order:
     copy `i`'s constants in ascending (object key, field name) order, an
     optional field's value constant before its presence constant, then
     `m<i>`, then `h<i>`;
  3. the range assertions, position by position: one per ranged value
     constant of copy `i` in declaration order, then `m<i>`'s;
  4. for `Unrolling` and `InductionBase`, the initial-state predicate on
     copy 0, the disjunction over the subject's initial states in their
     list order, each the conjunction of its field values in declaration
     order (an absent optional field as `(not P)` alone); then, for a TP-2
     item, `(not h0)`;
  5. one step assertion per step, in ascending order;
  6. the monitor implications, position by position, each position's in
     tabulation order (source state, then letter);
  7. for `InductionStep`, one distinctness assertion per pair of positions
     `(j, k)` with `j < k`, in ascending `(j, k)` order, printed as
     `(not (and Q1 ... Qn))`, `Q1` to `Qn` the field equalities of copy `k`
     with copy `j` in declaration order, then `(= m<k> m<j>)` for a TP-3
     item; then one assertion per position `p` in `0..depth`, in ascending
     order, printed as `(and (not d1) ... (not dn))`, `d1` to `dn` the
     disjuncts the negated property has at position `p` (Negated property),
     in that order, so that it states the property and the absence of every
     definedness failure there;
  8. one assertion of the negated property;
  9. `(check-sat)`.

  Terms are printed with single spaces, no comments, integer literals in
  decimal with negatives as `(- n)`, and each command on its own line ending
  in a line feed. A conjunction of one term prints as that term and of no
  term as `true`; a disjunction of one term prints as that term and of no
  term as `false`. The printer simplifies no other term: a literal read in
  place of a constant stays in the term it stands in.
- Equal inputs SHALL give byte-equal scripts or equal refusals.

### Size limits

The encoding SHALL refuse `too-large`, writing no script, naming the measure,
its value and its limit, when a measure passes its limit. It SHALL compute
each measure from the subject, the item and the kind, check the measures in
this order, and check each before it builds the terms the measure counts:

| Measure | Counts | Limit |
| --- | --- | --- |
| `constants` | `declare-const` commands: `depth + 1` times the constants of one position, `m<i>` and `h<i>` included | 1,048,576 (2^20) |
| `applications` | applications of one step: receivers times operations times argument vectors, summed over the operations | 65,536 (2^16) |
| `monitor-atoms` | atoms of a TP-3 item's monitor letter | 8 |
| `monitor-states` | monitor states the tabulation reaches | 64 |
| `candidates` | candidates of one application whose enabledness or postcondition failure the query expands | 65,536 (2^16) |
| `script-bytes` | bytes of the canonical script, counted as it is printed, stopping at the first byte past the limit | 268,435,456 (2^28) |

The table of Outputs places each measure's check among the other refusals.
`encode_hyper_query` carries the same measures and limits (Hyper
self-composition queries). The limits are not request settings, and a refusal is not a run limit: no
V-7 follows from it. CG's SMT backend reads a refusal as the backend having
no query for the item.

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
  carry no quantifier, so the call refuses `SmtEncodingRefusal`
  `refinement-open-argument` naming the row and the argument; for `any`, no
  query is encoded and the call refuses `SmtEncodingRefusal`
  `refinement-any-row` naming the row, because a declaration with an `any`
  row has no per-step query (FR-146).
- A mapping row, receiver or argument outside the `QF_LIA` fragment this
  requirement prints SHALL refuse `SmtEncodingRefusal` `out-of-fragment`
  naming it, with its locus, and write no script.

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
  order, then the proof-query stutter last. Every `behaviours` clause is
  infinite-trace (ADR-023 HS-7), so the stutter is always a component
  disjunct. It holds at every state, as for one subject, so it
  over-approximates each component's terminal stutter (SM-4), and its step
  label is `stutter` with every argument absent (HS-5).
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
  - and this requirement's `field-sort`, `parameter-domain`, `field-domain`
    and `out-of-fragment` refusals, per copy, and its `too-large` refusal,
    whose `constants` measure counts every copy's constants and whose
    `applications` measure counts the joint disjuncts of one step.
- Equal requests, with meters of equal budget, and equal kinds SHALL give
  byte-equal scripts or equal refusals.

#### Product identity

`HyperProductIdentity` SHALL be QSL's one public identity of the HP-2
product and instance a hyper query encodes. It is the in-process value that
passes from `encode_hyper_query` through CG's SMT backend to FR-182's
`settle_hyper_smt`, so every SMT outcome names the exact item and instance
it answers. It SHALL be a type of the `qsl-replay` facade, at
`qsl_replay::HyperProductIdentity`, beside `HyperQueryRequest`,
`HyperQuery`, `SmtQueryKind`, `SmtEncodingRefusal` and FR-336's
`SubjectIdentity`. CG depends on QSL only through `qsl-replay` (FR-059),
and it names these types and calls `encode_hyper_query`,
`HyperProductIdentity::of` and `settle_hyper_smt` from its `replay` adapter
module, the one module FR-060's T12-A allows to call the facade. CG's SMT
backend reaches them through that module.

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
    subjects' checked package. Its node id binds every clause-level member
    that FR-172 "Identity" and ADR-023 HV-7 name: the aliases in order, the
    quantifier prefix with each variable's kind, alias and fairness set,
    `μ`, the `align skip` list and the body.
  - `subjects`: FR-336's `SubjectIdentity` of each subject in
    `clause.aliases` order, which binds its checked package, initial
    states, universes, domain packages and proof bounds.
  - `instance`: the request's `instance`, the one member FR-172 and HV-7
    name that a clause node id cannot bind.
- **No digest of its own.** The identity SHALL be made only of these
  existing canonical identities. It SHALL NOT be or carry a digest over its
  members, and it SHALL have no digest domain. Two identities are equal
  exactly when their three members are equal, and its `Hash` SHALL agree
  with that equality.
- **In-process only.** The identity SHALL have no wire form: it SHALL
  implement neither `Serialize` nor `Deserialize`. CG holds it in memory
  between the query and the settlement call and keeps no copy of it on a
  run record.
- **QSL mints it.** Only `HyperProductIdentity::of` and `encode_hyper_query`
  SHALL make a `HyperProductIdentity`. The facade SHALL expose no
  constructor from members, so a caller cannot mint one (ADR-013 O-09: no
  re-mint of a clause identity). CG keeps the value it receives and passes
  it unchanged to `settle_hyper_smt` with the outcome.
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
- **One checked package.** Every subject of the request is a subject of the
  checked package of the unit that declares the clause: ADR-023 HS-2
  compares two models through two aliases of that one unit, and each
  alias's subject differs only in its initial states, universes and proof
  bounds (ADR-018 SM-2, FR-125). A subject whose checked package is not the
  first subject's SHALL refuse `subject-binding` at its alias.
- **Refusals and their order.** `HyperProductIdentity::of` SHALL refuse
  `SmtEncodingRefusal`, with no identity, on the first of these that
  applies, in this order:
  1. `hyper-form`, as above: an identity is of an HP-2 product only;
  2. `subject-binding`, as above and for a subject over another checked
     package;
  3. `instance-binding`, when `instance` does not hold exactly one pair per
     member of `clause.parameters`, in parameter order, each key in that
     parameter's universe, at the first parameter that breaks this;
  4. `product-identity`, when the subjects' checked package has no
     checked-package v2 emission, so FR-336's `SubjectIdentity::of`
     refuses, or when that emission has no `hyperproperty` node for the
     clause, at the clause.

  `encode_hyper_query` SHALL apply these four checks first, in the same
  order, and then `hyper-body`, `query-kind`, `too-large` for `constants`,
  the `field-sort`, `parameter-domain` and `field-domain` refusals per copy,
  `match-conjunct`, `too-large` for `applications` and `candidates`,
  `out-of-fragment` per copy, and `too-large` for `script-bytes`, refusing on the
  first that applies with no script. For one request, the two calls
  therefore give the same refusal whenever `of` refuses.
  `HyperProductIdentity::of` SHALL evaluate no `μ_U` conjunct and charge
  nothing to `meter`.
- **Outcome handoff.** CG SHALL return every SMT result for an HP-2 item,
  EN-3's and EN-2's alike, to QSL as FR-182's `HyperSmtOutcome`, whose
  `product` is the `product` of the `HyperQuery` values it answers, or
  `HyperProductIdentity::of` over the same request when
  `encode_hyper_query` refused. The base and step queries of one induction
  result SHALL carry equal identities and equal depths. `settle_hyper_smt`
  settles an outcome only for the item and instance its identity names.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-315-AC-1 | Over the `Counter` subject of FR-124-AC-1 and FR-125-AC-1 (universe `{c}`, `value: Int[0, 3]`, initial value 0, operation `inc` with precondition `self.value < 3` and postcondition `self.value = pre(self.value) + 1`), under event-position false-extension, `Unrolling{depth: 3}` for the TP-2 `on origin` claim `always[0,3] holds(c.value <= 3)` declares `s0.c.value`, `h0`, `s1.c.value`, `h1`, `s2.c.value`, `h2`, `s3.c.value`, `h3` in that order, asserts the four range assertions, `(= s0.c.value 0)`, `(not h0)`, one step assertion per step and the negated property, and ends with `(check-sat)`; encoding it twice gives byte-equal scripts. | Test (TC-900) |
| FR-315-AC-2 | `InductionStep{depth: 1}` for `always holds(c.value <= 3)` asserts no initial-state predicate, asserts the property at position 0 and its violation at position 1; `InductionBase{depth: 1}` asserts the initial-state predicate on copy 0. | Test (TC-900) |
| FR-315-AC-3 | An item over a field whose sort the encoding has no form for refuses `SmtEncodingRefusal` naming the field, with its locus, and writes no script. | Test (TC-900) |
| FR-315-AC-4 | Over the `Tally` subject (universe `{t}`, fields `tally: Int[0, 3]` and `next: Int[0, 3]`, initially 0 and 1; operation `bump` with frame `modifies [tally, next]`, precondition `pre G { self.tally < 3 and self.next = self.tally + 1 }`, and postconditions `post B { self.tally - pre(self.tally) = 1 }` and `post C { self.tally < 3 and self.next = self.tally + 1 }`), under infinite-trace, `Unrolling{depth: 2}` for `always holds(t.tally < 3 and t.next = t.tally + 1)` builds the definedness condition of the range coercion of `t.tally + 1` into `Int[0, 3]` under the `and`'s short-circuit: at each position `i` in `0..=2`, a disjunct of the negated property holds when `s<i>.t.tally < 3` holds and `s<i>.t.tally + 1` lies outside `[0, 3]`. `G`'s coercion contributes the same condition over copy `i` as `bump`'s precondition failure at every position `0..=2`, position 2 included, and `C`'s coercion contributes `bump`'s postcondition failure at each position, expanded over the 16 candidates of (`tally`, `next`). `B` holds no partial construct and contributes no failure term. Every term of the script is linear. | Test (TC-900) |
| FR-315-AC-5 | `InductionStep{depth: 2}` for `always holds(c.value <= 3)` asserts that copies 0, 1 and 2 are pairwise distinct; for a TP-3 item it declares `m0` to `m2` and asserts the monitor steps, the property at a position being the monitor state there not rejecting. | Test (TC-900) |
| FR-315-AC-6 | Over `CasRefinesCounter`, the `Initial` query declares the concrete copy `s0`, asserts the initial-state predicate on it and the negation of `map(s0)` equalling an abstract initial state, and the `Step { row }` query for a `stutter` row declares `s0` and `s1`, asserts the operation's precondition on `s0` and its frame and postcondition over `s0`, `s1`, and the negation of the mapped state being unchanged; encoding either twice gives byte-equal scripts; an `any` row refuses `SmtEncodingRefusal` `refinement-any-row`; a `Step` query for an abstract-operation row (`step Impl::Counter::commitA -> Spec::Counter::inc(self);`) asserts that operation's precondition, frame and postcondition over `map(s0)`, `map(s1)` with the row's receiver and arguments evaluated at `s0`, negated; a row with a `_` argument refuses `SmtEncodingRefusal` `refinement-open-argument` naming the row and the argument, shown over `CasRefinesCounterAdd`, which is `CasRefinesCounter` with `Spec::Counter` gaining the operation `add(n: Int[0, 1])` (frame `modifies [value]`, precondition `self.value + n <= 2`, postcondition `self.value = pre(self.value) + n`) and `Impl::Counter` gaining the operation `bump()` with an empty frame, like `peek`, and the row `step Impl::Counter::bump -> Spec::Counter::add(self, _);` (one receiver and one argument for the one abstract parameter, so the row types under FR-136; `bump` is the eighth step row and its position in the `steps` order is `r`); a mapping row outside `QF_LIA` refuses `out-of-fragment` naming the row. | Test (TC-900) |
| FR-315-AC-7 | Over ADR-023 §8's secure vault and its §1 `NonInterference` clause for the instance `v`, `encode_hyper_query` with `InductionBase{depth: 1}` declares, in order, `s0.a.v.h`, `s0.a.v.l`, `s0.b.v.h`, `s0.b.v.l`, `s1.a.v.h`, `s1.a.v.l`, `s1.b.v.h`, `s1.b.v.l`, all `Int`, and no `m<i>`; asserts `a`'s initial-state predicate on copy 0 and then `b`'s; and asserts for step 0 to 1 one disjunction of exactly three joint disjuncts, `(step(0), step(0))`, `(step(1), step(1))`, then `(stutter, stutter)`. Each component's disjunct order is `step(0)`, `step(1)`, `stutter`, and `μ_U` holds on `(stutter, stutter)` because both operations are `stutter` and both arguments are absent. With the `match` block removed, the disjunction has all nine tuples in lexicographic order, `(step(0), step(0))` first and `(stutter, stutter)` last. Encoding either twice gives byte-equal scripts, and every term is linear. | Test (TC-1120) |
| FR-315-AC-8 | For the same clause, `InductionStep{depth: 1}` asserts no initial-state predicate, the property `v.l @ a = v.l @ b` at position 0, the joint states at positions 0 and 1 distinct, and its violation at position 1; `InductionStep{depth: 2}` asserts positions 0, 1 and 2 pairwise distinct. The same clause with `fair { weak V::Vault::step }` on `a` gives byte-equal scripts to it for both kinds. | Test (TC-1120) |
| FR-315-AC-9 | `NonInterference` over the secure vault with FR-176-AC-4's added `match` conjunct, which is undefined on every tuple whose `a` step is `step(0)`, keeps `(step(0), step(0))` as a joint disjunct, and `InductionBase{depth: 1}` makes the negated property hold when that disjunct holds between copies 0 and 1. | Test (TC-1120) |
| FR-315-AC-10 | Each call refuses `SmtEncodingRefusal` and writes no script: an HP-3 clause (`forall trace a of V exists trace b of V`, FR-177) names `hyper-form`; FR-176-AC-3's clause `forall trace a of V { always eventually holds(v.l @ a = 0) }` names `hyper-body`; `Unrolling{depth: 1}` for `NonInterference` names `query-kind`; ADR-023 §1's `SavesPower` with one subject, for `S`, names `subject-binding` at alias `N`; `NonInterference` with a `match` conjunct that calls a function whose evaluation exhausts the request's `meter` names `match-conjunct` at that conjunct and the tuple `(step(0), step(0))`, the first in the joint-step order. The same request with an equal budget gives the same refusal. | Test (TC-1120) |
| FR-315-AC-11 | Over the secure vault, a matched joint step sets `l` to the same input `i` in both copies, so `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for `NonInterference` are both unsatisfiable, the HP-2 outcome `Inductive{depth: 1}` that FR-182-AC-7 settles. Over a vault whose post clause is `self.l = self.h`, `InductionBase{depth: 1}` is satisfiable from the initial tuple `((0, 0), (1, 0))` with the joint step `(step(0), step(0))`. That model is a matched bad prefix, not a `HyperCounterexample`: it has no lasso, so no outcome follows from it alone, and the clause is refuted only through EN-2's lasso and FR-183's replay. | Analysis (TC-1120) |
| FR-315-AC-12 | Over ADR-023 §8's secure vault with the `vaults` universe `{v, w}`, `NonInterference`'s `InductionBase{depth: 1}`, `InductionStep{depth: 1}` and `InductionStep{depth: 2}` queries for instance `v` carry three equal `product` values, each equal to `HyperProductIdentity::of` over the same request. Its `clause()` is the `claim` occurrence of the clause's `hyperproperty` node in the vault package's checked-package v2 emission; its `subjects()` holds one member, equal to FR-336's projection of the vault subject; its `instance()` is the one pair binding the clause's parameter to `v`. The request for instance `w` gives an identity that differs only in `instance()`. The clause with `fair { weak V::Vault::step }` on `a` gives byte-equal scripts and an identity that differs only in `clause()`. A subject that differs only in a proof bound gives an identity that differs only in `subjects()`. Meters of different budgets give equal identities. | Test (TC-1190) |
| FR-315-AC-13 | Each of these requests is refused `SmtEncodingRefusal` with the same code by both `HyperProductIdentity::of` and `encode_hyper_query`, with no identity and no script: `NonInterference` with an empty `instance` names `instance-binding` at the clause's parameter; with the key `x`, outside the universe `{v}`, it names `instance-binding` at that parameter; ADR-023 §1's `SavesPower` with its `N` subject over a checked package other than the `S` subject's names `subject-binding` at alias `N`; `NonInterference` over a subject whose checked package has no checked-package v2 emission names `product-identity` at the clause; the HP-3 clause of FR-315-AC-10 names `hyper-form`; FR-176-AC-3's liveness clause with an empty `instance` names `instance-binding`, not `hyper-body`. `HyperProductIdentity::of` over `NonInterference` with a `match` conjunct whose evaluation exhausts the request's `meter` returns the identity and leaves the meter's consumed counters at zero. | Test (TC-1190) |
| FR-315-AC-14 | A crate whose only QSL dependency is `qsl-replay` names `qsl_replay::HyperProductIdentity`, `HyperQuery`, `HyperQueryRequest`, `SubjectIdentity`, `HyperSmtOutcome` and `settle_hyper_smt` from its `replay` module, calls `encode_hyper_query`, `HyperProductIdentity::of` and `settle_hyper_smt` there, and compiles. In that crate a struct literal of `HyperProductIdentity`, serializing one and deserializing one each fail to compile. | Test (TC-1190) |
| FR-315-AC-15 | Over the `Tally` subject of FR-315-AC-4 under infinite-trace, each step assertion of `Unrolling{depth: 3}` for `always holds(t.tally < 2)` is the disjunction of exactly two disjuncts, `bump`'s and then the proof-query stutter. The stutter between copies `i` and `i+1` is the conjunction of `(= s<i+1>.t.next s<i>.t.next)` and `(= s<i+1>.t.tally s<i>.t.tally)` and holds no other conjunct: no precondition, enabledness or candidate term. | Test (TC-900) |
| FR-315-AC-16 | Read by a `QF_LIA` solver: over `Tally`, `Unrolling{depth: 3}` and `InductionBase{depth: 3}` for `always holds(t.tally < 2)` are satisfiable, the path `(0, 1)`, `(1, 2)`, `(2, 3)` reaching the violation at position 2 and depth 3 by the stutter at `(2, 3)`, where `G` holds and no candidate satisfies `C`. `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for `always holds(t.tally < 3)` are both unsatisfiable. For `Tally`'s deadlock-freedom item, with no `terminal` declaration, `InductionBase{depth: 2}` is satisfiable, `(2, 3)` being a deadlock at position 2, and `InductionBase{depth: 1}` is unsatisfiable; with DL-1's `terminal when P` for `P` the state predicate `t.tally = 2`, `InductionBase{depth: 3}` is unsatisfiable. | Analysis (TC-900) |
| FR-315-AC-17 | Over FR-315-AC-1's `Counter` under event-position false-extension, `Unrolling{depth: 5}` for `always[0,5] holds(c.value <= 3)` declares `h0` to `h5` and asserts `(not h0)`. Each step assertion `i` is printed as `(and (= h<i+1> (or h<i> (not E))) (or h<i+1> D))`, with no stutter disjunct: `D` is `inc`'s disjunct between copies `i` and `i+1`, and `E` is `inc`'s enabledness, `(and A (or B0 B1 B2 B3))`, `A` being `CanInc` on copy `i` and `Bv` being `Inc` read with `value` `v` as a literal, for the candidates 0, 1, 2 and 3 in that order. The violation at each position `p` in `0..=5` includes `h<p>`. `InductionBase{depth: 3}` for `always[0,3] holds(c.value <= 3)` refuses `query-kind`, and so does `Unrolling{depth: 2}` for `eventually[0,3] holds(c.value = 3)`. | Test (TC-900) |
| FR-315-AC-18 | Read by a `QF_LIA` solver, over FR-315-AC-1's `Counter` under event-position false-extension: `Unrolling{depth: 3}` for `always[0,3] holds(c.value <= 3)` is unsatisfiable, the V-2 query of FR-314-AC-1. `Unrolling{depth: 5}` for `always[0,5] holds(c.value <= 3)` is satisfiable, the behaviour ending at position 3 and positions 4 and 5 lying past its end (FR-125-AC-2), and so is `Unrolling{depth: 5}` for `always[0,5] holds(c.value <= 2)`. `Unrolling{depth: 3}` for `eventually[0,3] holds(c.value = 3)` is unsatisfiable, and `Unrolling{depth: 2}` for `eventually[0,2] holds(c.value = 3)` is satisfiable. | Analysis (TC-900) |
| FR-315-AC-19 | Over `Tally`, `InductionStep{depth: 1}` for `always holds(t.tally < 3)` has, among the negated property's disjuncts at position 1, `bump`'s precondition failure over copy 1 and its postcondition failure over copy 1 and the 16 candidates; each reads constants of copy 1 and literals only. The position-0 assertion before the negated property holds the property at position 0 and the negation of the same two failures over copy 0. | Test (TC-900) |
| FR-315-AC-20 | For `Tally`'s deadlock-freedom item, `InductionBase{depth: 2}` declares no `m<i>` and no `h<i>`, keeps the stutter disjunct in each step assertion, and reads the violation at position `i` as copy `i` terminal, by the expansion over `bump`'s 16 candidates. With `terminal when P` for `P` the state predicate `t.tally = 2`, the violation at position `i` is copy `i` terminal conjoined with the negation of `P` over copy `i`. | Test (TC-900) |
| FR-315-AC-21 | Each call refuses `SmtEncodingRefusal` with the code and locus given and writes no script: over `Counter`, the TP-4 claim `always eventually holds(c.value = 0)` names `item-form`; the TP-2 `on origin` claim `always[0,3] eventually[0,1] holds(c.value >= 1)` names `item-form`; `Counter` with FR-120-AC-3's operation `spawn()` (frame `creates Counter`) names `object-lifecycle` at `spawn`; `Counter` with an empty `initial` list names `no-initial-state`; `Counter` with an empty universe and an initial state that holds no object names `no-instance`; `Counter` with an operation `setTo(n: Integer)` names `parameter-domain` at `setTo` and `n`; `always holds(c.value * c.value <= 9)` names `out-of-fragment` at the product; FR-315-AC-23's TP-3 item over `Counter` with the universe `{c, d}` names `several-instances`; `Unrolling{depth: 1048576}` for `always holds(c.value <= 3)` names `too-large` with measure `constants`, value 1,048,577 and limit 1,048,576, before any constant is built. Each refusal is equal across two calls. | Test (TC-900) |
| FR-315-AC-22 | Over a `Counter` subject with the universe `{d, c}`, both objects initially 0, operations `inc` and `setTo(n: Int[0, 2])` (each with frame `modifies [value]`), `InductionStep{depth: 2}` for `always holds(x.value <= 2)` with `over (x: Counter)` declares `s0.c.value`, `s0.d.value`, `s1.c.value`, `s1.d.value`, `s2.c.value`, `s2.d.value` in that order, and then asserts, in order: six range assertions in declaration order; two step assertions; three distinctness assertions for positions `(0, 1)`, `(0, 2)` and `(1, 2)`; two assertions for positions 0 and 1; the negated property; and `(check-sat)`. Each step assertion's disjuncts are `c.inc`, `c.setTo(0)`, `c.setTo(1)`, `c.setTo(2)`, `d.inc`, `d.setTo(0)`, `d.setTo(1)`, `d.setTo(2)` and the stutter, in that order. The instances are `d` then `c`, FR-125's universe-list order (FR-125-AC-10), so the negated property's first two disjuncts are the violations of instance `d` and then instance `c` at position 2. | Test (TC-900) |
| FR-315-AC-23 | For the TP-3 item `always (holds(c.value = 3) implies always holds(c.value = 3))` with `over (c: Counter)` over FR-315-AC-1's `Counter` under infinite-trace, `Unrolling{depth: 2}` asserts `0 <= m<i> <= n-1` for each position, `n` the tabulated monitor states with the initial state numbered 0; at position 0 one implication per letter from the initial state, the letter of position 0 implying `m0` at that move's target; and at positions 1 and 2 one implication per tabulated move, `m<i-1>` at its source and the letter of position `i` implying `m<i>` at its target, all after the step assertions. `InductionStep{depth: 2}` asserts no implication at position 0, so `m0` is constrained by its range assertion only. | Test (TC-900) |
| FR-315-AC-24 | Over FR-315-AC-1's `Counter` with an added operation `isZero(): Boolean` (empty frame, postcondition `post Z { result = (self.value = 0) }`), each step assertion's `isZero` disjunct has the postcondition part `(or Z0 Z1)`, `Z0` being `Z` read with `result` the literal `false` and `Z1` with `true`, and the script declares no constant for the result. With `Counter` given a field `id: Integer` that `inc`'s frame also writes, the false-extension query of FR-315-AC-17, which expands `inc`'s candidates, refuses `field-domain` at `c.id`. | Test (TC-900) |
| FR-315-AC-25 | Over a `Holder` subject (universe `{s}`, an optional field `held: Int[0, 3]`, initially absent; operation `put` with frame `modifies [held]` and postcondition `present(self.held)`), under infinite-trace, `InductionStep{depth: 1}` for `always holds(present(s.held) or not present(s.held))` prints the stutter between copies 0 and 1 as `(and (= s1.s.held.present s0.s.held.present) (or (not s1.s.held.present) (= s1.s.held s0.s.held)))`, and its distinctness assertion as the negation of that field equality. Two copies with `held` absent therefore agree on `held` whatever their value constants hold. | Test (TC-900) |

## Dependencies

- ADR-018 V-2, V-3, PC-6, LA-2, LA-3, DS-2; SM-4 and SM-6 (the stutter,
  finite behaviours and monitors), UE-1, UE-4 and UE-6 (definedness), DL-1,
  DL-3 and DL-7 (deadlock-freedom), EN-3, CX-2 and CX-3.
- ADR-023 HC-1, HC-4, HC-6, HC-7, HM-4, HM-7, HS-5, HV-1, HV-3, HV-8, DS-3
  (the hyper queries).
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (the
  transition relation), [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (the subject), [FR-314](FR-314-check-an-smt-proof-certificate.md) (the
  checker that rebuilds the queries), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (the deadlock-freedom item), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (`ModelCheckItem`), [FR-128](FR-128-replay-a-model-counterexample.md) (the
  replay a satisfiable query's model goes through).
- [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md)
  (the checked hyper clause), [FR-173](FR-173-classify-hyper-clauses-into-forms.md)
  (its form), [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md)
  (the product the hyper queries encode), [FR-182](FR-182-settle-a-hyper-verdict.md)
  (the settlement of their outcomes).
- [FR-336](FR-336-bound-a-model-subject-by-its-universes.md) (the subject
  identity projection), ADR-013 O-07 and O-09 (the clause occurrence and
  its no-re-mint rule), [FR-059](FR-059-check-backend-dependency-direction.md)
  and [FR-060](FR-060-check-qsl-api-surface-boundary.md) (CG names QSL only
  through `qsl-replay`, from its `replay` adapter module).

## References

- SMT-LIB 2 standard.
- Owning ticket: Linear QSL-366; hyper self-composition queries: Linear
  STD-311; hyper product identity: Linear STD-336; proof-query stutter,
  finite behaviours, deadlock-freedom, refusals and size limits: Linear
  STD-338.

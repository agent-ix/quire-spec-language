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
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
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
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-487
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
calls for HP-2 `k`-induction. `encode_hyper_query`, `HyperQueryRequest`, `HyperQuery` and
`HyperProductIdentity` are defined in `qsl-replay`, because the identity holds FR-336's `SubjectIdentity`,
which `qsl-replay` defines; `qsl-eval` supplies the script entry that runs the checks after the identity
checks, and the refusal codes.

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
    limits: &SmtEncodingLimits,
) -> Result<Vec<u8>, SmtEncodingRefusal>;

/// The encoding's size limits (Size limits), each a request setting
/// `smt_encoding.<member>` (FR-255). `Default` gives every member its
/// published default.
pub struct SmtEncodingLimits {
    pub max_constants: u64,      // `constants`, default 1048576 (2^20)
    pub max_applications: u64,   // `applications`, default 65536 (2^16)
    pub max_monitor_atoms: u64,  // `monitor-atoms`, default 8
    pub max_monitor_states: u64, // `monitor-states`, default 64
    pub max_candidates: u64,     // `candidates`, default 65536 (2^16)
    pub max_script_bytes: u64,   // `script-bytes`, default 268435456 (2^28)
}
```

The call takes no meter. The encoding evaluates no clause: it prints terms
over the subject's state copies, so FR-107's meter has nothing to charge.
Its size limits are the request's `SmtEncodingLimits` (Size limits). The
limits decide only whether a script is printed, never its bytes: two calls
that both print a script print the same bytes whatever their limits. CG's
SMT backend encodes under the run's limits, and FR-314's checker rebuilds
the query under the same values, which the item's certificate request
records (FR-314 "Encoding limits"), so a query that fits only raised
limits is rebuilt as the solver received it.

## Outputs

- The query's SMT-LIB 2 script in canonical printing.
- `SmtEncodingRefusal`, with no script, naming the construct and its locus,
  when the item or the subject uses a construct the encoding has no form
  for. Each refusal of `encode_smt_query` has one code. The encoding SHALL
  apply the checks in the order of this table, one row at a time, and SHALL
  refuse on the first row that applies:

| Order | Code | When | Locus |
| --- | --- | --- | --- |
| 1 | `item-form` | The item is a TP-4 liveness clause, guarded or not, whose refutations are EN-2's lassos (Monitors); a clause with no recorded property form; or a clause whose property-automaton translation refuses, carrying that refusal | the clause |
| 2 | `profile` | The clause binds a profile other than infinite-trace and event-position false-extension, such as ADR-026's `model-time` | the clause |
| 3 | `query-kind` | `InductionBase` or `InductionStep` for a TP-2 `on origin` item, whose V-2 verdict needs no induction; `Unrolling{depth}` with `depth < u` for `eventually[0,u] holds(I)` (Finite behaviours); `Unrolling{depth}` with `depth < h` for an `on origin` monitor item, `h` its ADR-014 TR-4 horizon (Monitors) | the clause |
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
| 14 | `too-large` | The `constants`, then `applications`, then `monitor-atoms`, then `monitor-states`, then `candidates` measure passes its limit (Size limits) | the measure, its value and its limit |
| 15 | `out-of-fragment` | An expression is outside the `QF_LIA` fragment the encoding prints, such as a product of two non-literal terms or a non-literal divisor | the expression's clause and node |
| 16 | `too-large` | The `script-bytes` measure passes its limit | the measure, its value and its limit |

Of several constructs that one row refuses, the encoding names the first in
the canonical order of the script: object, field and operation order for
the subject rows, instance order for the instance rows, and assertion order
for `out-of-fragment`.

The codes of `SmtEncodingRefusal::Refused` are exactly these: the codes of this
table; `refinement-any-row`, `refinement-open-argument` and
`out-of-fragment` for `encode_refinement_query` (Refinement queries); and
`hyper-form`, `hyper-body`, `query-kind`, `subject-binding`,
`instance-binding`, `product-identity`, `match-conjunct`, `field-sort`,
`parameter-domain`, `field-domain`, `out-of-fragment` and `too-large` for `encode_hyper_query` and
`HyperProductIdentity::of` (Hyper self-composition queries).
`instance-binding` means a malformed `instance` of a hyper request only;
the single-subject check of an instance's object is `instance-type`.

**Internal faults.** No refusal code names a broken internal invariant,
because a well-formed request cannot break one. A well-formed request is a
model subject admitted by FR-120 and an item S3 checked in the subject's
checked package. Such a request guarantees these invariants:

- a temporal clause's `over` parameter is an object reference (QSpec
  FR-370, FR-125);
- a TP-1, TP-3 or deadlock-freedom item binds infinite-trace, and only a
  TP-2 item binds event-position false-extension (FR-123, ADR-014 TR-3,
  ADR-018 §1);
- a TP-1 or TP-3 item has at least one atom, and a TP-1 item's one state
  predicate is that atom;
- a `terminal when P` predicate is a state predicate over an object type of
  the subject (FR-124);
- every monitor state number is below the request's `max_monitor_states`
  (Size limits), which the `monitor-states` check has passed;
- a checked hyper clause of an HP-2 request has a checked condition and a
  component record for every `holds` atom of its body, as many slots as
  `atom_slots()` holds, and conjuncts that `CheckedStepMatch` reads without a
  `Fault` (FR-487).

The encoding's own build order guarantees one more: it reads a term (copy
`i` terminal, an enabledness, an application count) only after building
it.

`SmtEncodingRefusal` SHALL have two arms:

```rust
pub enum SmtEncodingRefusal {
    Refused { code: RefusalCode, locus: Locus }, // one code of the closed set above
    Fault(InternalFault),                        // FR-096: a broken invariant
}
```

When one of these invariants does not hold, the call SHALL return
`SmtEncodingRefusal::Fault` carrying FR-096's `InternalFault`
(`runtime_invariant`, cause `established-invariant-broken`), naming the
invariant, and no script. It SHALL NOT return `Refused` with a code such as
`item-form`. `encode_hyper_query`, `HyperProductIdentity::of` and
`encode_refinement_query` return the same two arms.

A fault is QSL's internal failure and settles like CG's `BackendFault`, as
`failed`, basis `unavailable`, `TerminalValue::Failed`, in the ADR-013 O-16
internal-failure category, never `inconclusive` or `unsupported`:

- **Single subject.** CG's SMT map settles an item whose `encode_smt_query`
  call returned `Fault` this way (ADR-018 DS-2). When FR-314's checker
  meets `Fault` while it rebuilds a query, `check_smt_proof` returns
  `SmtProofCheck::Fault`, and FR-127's settlement map settles the result
  this way.
- **HP-2.** FR-182's `settle_hyper_smt` settles an `EncodingRefused`
  outcome carrying `Fault` this way. When `HyperProductIdentity::of(item)`
  returns `Fault`, the outcome CG sends carries `product: Err` (Outcome
  handoff), and `settle_hyper_smt` writes that `failed` record and does not
  refuse `ItemIdentity`, because a fault is no evidence about the item's
  identity.

## Behavior

### State copies

- The encoding SHALL declare one state copy per position `0..=depth`, each
  field of each universe object as one constant of the field's SMT sort
  (`Int` for integer and enumeration fields with range assertions, `Bool`
  for Boolean fields), named `s<i>.<object>.<field>`; an optional field is
  a value constant of its element's sort and a `Bool` presence constant
  `s<i>.<object>.<field>.present`. Every term the encoding prints is linear,
  so each query is a `QF_LIA` script.
- For a monitor item (Monitors) the encoding SHALL also declare, per
  position, one `Int` monitor-state constant `m<i>.<n>` per instance, `n`
  the instance's zero-based index in FR-125's canonical instance order
  (Instances). For every TP-2 item, `on origin` or `on each`, it SHALL
  declare one `Bool` constant `h<i>` per position, true when position `i`
  lies past the end of a finite behaviour (Finite behaviours).
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
  result values in FR-120's result order (ascending typed canonical JCS
  bytes of the value), of the conjunction
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
  loss of completeness and keeps soundness. A guarded item's `when`
  condition is read at a stutter position on the repeated state, as its
  atoms are (FR-338 "Guarded `on each` activation"), so a stutter can add
  activations only at a state the path already holds.
- **Reading a satisfiable model.** A satisfiable query is not a refutation
  by itself, because its model may take the stutter where an operation is
  enabled, a path of the over-approximation only. CG's SMT backend SHALL
  read a satisfiable `Unrolling` or `InductionBase` model as follows; a
  satisfiable `InductionStep` model is a counterexample-to-induction and is
  never replayed.
  1. Except for a TP-2 model (rule 4b), CG cuts the model at its **failing position** `p`, the least position
     at which a disjunct of the negated property holds in the model. Steps
     after `p` are never read, so a stutter the solver takes after the
     failure cannot discard the model.
  2. A stutter before `p` that is followed, still before `p`, by an
     operation step is at a non-terminal state: the operation step leaves
     the same state, so its operation is enabled there. CG decides this from
     the model alone.
  3. When the failure at `p` is a property of the state alone (an unguarded
     TP-1 violation, a deadlock, or a definedness failure, a guarded item's
     undefined condition included), CG SHALL drop every
     stutter step before `p`. A stutter repeats its state, so the remaining
     operation steps are a real path from an initial state to the failing
     state, and that prefix is a counterexample (ADR-018 CX-1, UE-3).
  4. When the failure is a monitor violation of an infinite-trace item (a
     TP-3 item, or a guarded TP-1 or TP-3 item), which can depend on
     positions a stutter adds (IV-7), a model with a stutter of rule 2 is no
     counterexample candidate. A model with no stutter before `p` gives the
     prefix to `p`, its operation steps a real path whose monitor rejects
     at `p`: a bad prefix, which is CX-1's safety counterexample under
     infinite-trace. A model whose stutters before `p` form one
     tail, from a position `q` to `p`, is the lasso of CX-1 with the prefix
     to `q` and the `terminal-stutter` loop, which CG replays through
     `qsl_replay::replay_model_trace` (FR-128). That entry refuses a
     `terminal-stutter` loop on a state that is not terminal with
     `invalid_runtime_input`/`invalid-value` naming the `terminal-stutter`
     step, so it is the terminality check, and FR-315 adds no facade entry.
     CG SHALL read as "no candidate" only that refusal: `invalid-value`
     naming the lasso's `terminal-stutter` loop step. Every other result of
     replaying a model built by these rules SHALL settle as FR-127's
     settlement table settles that result, which this requirement does not
     restate, and never as "no candidate", `InductionNotClosed` or no
     outcome.
  4a. When failures of several kinds hold at the failing position `p`, a
     failure that is a property of the state alone (rule 3: a definedness
     failure, an unguarded TP-1 violation or a deadlock) SHALL be read
     before a monitor violation, so CG builds rule 3's prefix and needs no
     terminality check. Among state-only failures CG reads them in the
     negated property's disjunct order.
  4b. A TP-2 model, `on origin` or `on each`, under false-extension has no
     stutter: every step it takes before the end is an operation step, and
     its copies past the end (`h<i>` true) are not states. Rules 2 to 4 do
     not apply. Its path is the live copies from position 0. A definedness
     failure at a least live position `p`, a guarded item's undefined
     condition included, read first as in rule 4a, gives the prefix to `p`
     (UE-3). Otherwise let `p` be the failing position, the least position
     at which a violation disjunct holds (for a monitor item, some
     instance's monitor rejecting), and let `c` be the item's **cover
     position**: `u` for the direct shapes `always[0,u] holds(I)` and
     `eventually[0,u] holds(I)` (Finite behaviours), the formula's ADR-014
     TR-4 horizon `h` for any other `on origin` item, and `p + h` for an
     `on each` item, whose failing activation starts at a position `a <= p`
     and reads no position after `a + h`. The violation gives the prefix to
     `e`, uncut, `e` the least of `c` and the last live position within
     `0..=depth`. ADR-018 CX-1 requires a bounded-profile prefix to cover
     the horizon from the activation it reports or end at a terminal state,
     so that the evaluator decides it exactly: the prefix to `e` does when
     `e = c` or copy `e` is terminal. When neither holds, the model is a bad
     prefix that is no CX-1 counterexample, and the query gives no outcome
     at that depth (rule 5); an `Unrolling` at a depth that reaches `c`
     reads it.
  4c. CG SHALL attribute the counterexample to the instance of the failure
     that rules 3, 4, 4a and 4b read at the failing position `p`, and SHALL
     name that instance's `over_binding` (ADR-018 CX-2), which FR-128's
     replay checks. When a definedness failure is read (rule 4a, and the
     first case of rule 4b), the instance is that of the first definedness
     failure, in the negated property's disjunct order, that holds at `p`;
     it is never the instance of a violation that also holds there. When a
     violation is read, the instance is that of the first violation
     disjunct that holds at `p` in instance order; for a monitor item, the
     instance whose monitor `m<p>.<n>` rejects first in instance order. Each
     instance has its own monitor, so the reading needs no search over
     instances.
  5. After a model that is no candidate, CG SHALL settle from that query
     with no further model: a base query gives `InductionNotClosed{depth}`
     (V-6), and an `Unrolling` query gives no outcome at that depth. It
     need not ask for another model. This keeps each query to one solver
     answer and one replay. Every outcome stays sound, because a refutation
     still needs FR-128's replay. The only cost is completeness, for TP-3
     items that are not stutter-invariant, which EN-1 settles. A search that
     blocks the spurious path and asks again is an EN-2 counterexample
     search, the backend's own (ADR-018 DS-3).

### Finite behaviours

Under event-position false-extension (every TP-2 item, `on origin` or
`on each`) a behaviour that
reaches a terminal state is closed there (ADR-018 SM-4), and the positions
after it are read by the closed-boundary rule, every atomic predicate false
(QSpec FR-091). TP-2 is the only form under that profile. Under
false-extension every temporal operator carries an interval (ADR-014 TR-3:
S3 refuses one without, `MissingInterval`), FR-123 classifies a
false-extension clause as TP-2 before it tries TP-3, and TP-1, TP-3 and
deadlock-freedom are infinite-trace forms (ADR-018 §1). So the only
`always` under false-extension is `always[0,u]`, read past the end as
FR-125-AC-2 reads it. Two `on origin` shapes are printed directly, with no
monitor: `always[0,u] holds(I)` and `eventually[0,u] holds(I)` over a state
predicate `I` (the **direct shapes**, the last two bullets below). Every
other TP-2 item, any bounded formula FR-123 admits (nested intervals,
`until`, `release`, past operators and Boolean combinations among them) and
every `on each` item, is a monitor item: its property is read through its
bounded monitor, which reads a position past the end through the end letter
(Monitors). The candidates, enabledness, terminality and step assertion
below hold for every TP-2 item.

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
  candidate-major, the results in FR-120's result order.
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
- The property of the direct shape `always[0,u] holds(I)` at a position `p` is `I` there for
  every instance, and is false at a position past the end. Its violation is
  `h<p>` or the negation of `I` for some instance, at some position `p` in
  `0..=min(depth, u)`. Below its horizon this query is a bounded search: a
  violation at `p <= depth` is a bad prefix whatever follows, and an
  unsatisfiable answer at `depth < u` is no proof: CG reports it as no
  outcome at that depth, never as `BoundedComplete` (FR-314 rejects such a
  basis). The encoding
  SHALL print the violation at `p` as separate disjuncts of the negated
  property: `h<p>` first, then `(not I)` for each instance in instance
  order. It SHALL NOT print it as one disjunct `(or h<p> ...)`. A position
  `p > min(depth, u)` contributes no violation disjunct.
- The violation of the direct shape `eventually[0,u] holds(I)` is, for some instance, `h<p>`
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

- A **monitor item** is a TP-3 item; a TP-2 item other than the two direct
  shapes (Finite behaviours), which is every `on each` TP-2 item and every
  `on origin` TP-2 item over any other bounded formula; and every item whose
  `on each` activation has a `when` condition (a **guarded item**), TP-1,
  TP-2 or TP-3. An unguarded TP-1 item and the deadlock-freedom item are
  read directly, by their state predicate. A TP-4 item, guarded or not, has
  no proof query (`item-form`): its refutations are EN-2's lasso searches,
  the SMT backend's own (ADR-018 DS-3), which read a guarded clause's
  condition at each position through the letter seam, as FR-126 reads it,
  and replay through FR-128.
- The property of a monitor item is read through the deterministic monitor
  that `qsl-eval`'s property-automaton translation builds for it (ADR-018
  SM-6, EN-3, LA-2; FR-126): the bad-prefix monitor of a TP-3 item, the
  bounded monitor of a TP-2 item for its activation (`on origin` or
  `on each`), and, for a guarded item, that monitor under FR-338's guarded
  reading, which adds the root activation at a model position exactly where
  the condition reads true. The encoding reads the monitor through the
  translation alone and keeps no formula-specific encoding, so every bounded
  formula FR-123 admits and every activation have the one encoding of this
  section.
- **Letters.** A model letter has one bit per letter member. For a guarded
  item member 0 is the `when` condition, read before the clause's atoms as
  FR-338 reads it, and member `j + 1` is the translation's atom `j`; for
  any other monitor item member `j` is the translation's atom `j`. Bit `j`
  of a model letter is the value of member `j` at the position for the
  instance. Under event-position false-extension the monitor has one more
  letter, the **end letter**, which it reads at a position past the end of a
  behaviour: the closed-boundary letter of QSpec FR-091, every atomic
  predicate false and every constant unchanged, at which no condition is
  read and no activation starts, so it only advances the debt already live
  (FR-338). The end letter is numbered `2^a`, `a` the number of letter
  members, after every model letter. An infinite-trace monitor has no end
  letter: its stutter positions read the repeated state's model letter.
- **Tabulation.** The encoding SHALL number the monitor states in the
  order a breadth-first tabulation reaches them from the initial monitor
  state, the initial state being 0, over letters in ascending numeric
  order, the end letter last. The tabulation is one per item: every
  instance's monitor is the same automaton over its own letters.
- **Letter terms.** The term of a model letter `b` at position `i` for
  instance `n` SHALL be the conjunction, in member order, of member `j`'s
  value term over copy `i` read for that instance when bit `j` of `b` is 1
  and its negation `(not ...)` when it is 0. Under false-extension it SHALL
  have `(not h<i>)` as its first conjunct, and the end letter's term SHALL
  be `h<i>`. A member's value term is its value, as an atom of an unguarded
  item is printed; its definedness is read apart (Undefined evaluation).
- **Moves.** The encoding SHALL assert `(<= 0 m<i>.<n>)` and
  `(<= m<i>.<n> s-1)` as one range assertion `(and (<= 0 m<i>.<n>) (<=
  m<i>.<n> s-1))` for `s` tabulated states, and one implication per
  tabulated move and instance: at position `i > 0`, for the move from state
  `x` on letter `b` to state `y`, `(=> (and (= m<i-1>.<n> x) L) (= m<i>.<n>
  y))`, `L` the term of letter `b` at position `i` for instance `n`; at
  position 0, for `Unrolling` and `InductionBase` only, for the move from
  the initial state on letter `b` to `y`, `(=> L (= m0.<n> y))`.
- `m<i>.<n>` is therefore instance `n`'s monitor state after reading the
  letter of position `i`, EN-1's product convention: `m0.<n>` is the
  initial monitor state stepped over the letter of position 0, not the
  initial monitor state itself. Under `InductionStep`, `m0.<n>` is
  constrained by its range assertion only, which over-approximates the
  states the step case starts from. The property of instance `n` at a
  position is `m<i>.<n>` differing from the rejecting state's number, and
  its violation there is `(= m<i>.<n> r)`, `r` that number. A monitor whose
  rejecting state is not reached by the tabulation has no violation term.
- A monitor item reads every position `0..=depth`. A TP-2 `on origin`
  monitor item is settled by `Unrolling` at a depth `depth >= h`, `h` its
  horizon, only: an unsatisfiable answer there is V-2's bounded-complete
  proof and a satisfiable one a refutation by rule 4b. `Unrolling{depth}`
  with `depth < h` SHALL refuse `query-kind`, because the monitor of such an
  item may decide a violation only at a position past `depth`, so an
  unsatisfiable answer below `h` proves nothing (ADR-018 V-2 reads the query
  only at `depth >= h`). A TP-2 `on each` item, like a
  TP-3 item, by all three query kinds: it holds at every activating
  position, an unbounded safety property that EN-3's `k`-induction settles
  (ADR-018 §1, EN-3).
- **Steps past the end.** Under false-extension a copy past the end is no
  state, so two copies that are both past the end hold the same position
  of the behaviour exactly when their monitor states agree, whatever their
  field constants hold (Canonical printing, step 7).

### Instances

- A query SHALL cover every instance of the item together. The property at
  a position is the conjunction over the item's instances, in FR-125's
  canonical order, and its violation is the disjunction of the instances'
  violations in that order. A model shows that some instance fails, and the
  counterexample built from it names that instance's `over_binding`
  (ADR-018 CX-2, rule 4c), which replay checks.
- A monitor item SHALL have one monitor per instance, whatever the number
  of instances: the constants `m<i>.<n>` for every instance `n`, each with
  its range assertion and its moves over its own letters, and the property
  at a position the conjunction of the instances' monitors not rejecting.
  No instance's monitor reads another's letters, so a violation of
  instance `n` is `(= m<p>.<n> r)` alone, and the model names the failing
  instance by which monitor rejects.

### Undefined evaluation

- The encoding SHALL read undefinedness as a refutation, never as false or
  as a disabled step (ADR-018 UE-1, UE-4, UE-6). For each atom the claim
  reads, it SHALL build the atom's definedness condition (a divisor not zero,
  a `value` of a present option, a reduction over a non-empty collection, a
  value inside its sort's range), reading `and`, `or`, `implies` and `if`
  under FR-006's short-circuit.
- A **definedness failure** at position `p` is any of these:
  - an atom the claim reads at `p` is undefined. Under false-extension a
    TP-2 `on origin` claim reads positions `0..=min(depth, h)` only, `h`
    its horizon (`u` for a direct shape; UE-1: a TP-2 `on origin` claim
    reads positions 0 to `h`), a TP-2 `on each` claim every position
    `0..=depth`, and both only at a live copy, since a position past the end
    evaluates no atom;
  - for a guarded item, its `when` condition is undefined at `p` for an
    instance: the condition has no value at a position the monitor reads it
    (FR-126), which is every position `0..=depth` under infinite-trace and
    every live position under false-extension. The failure is located at
    the activation, as FR-126 locates it, and is read before the atoms'
    failures at `p`, as the condition is read before the atoms;
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
    the pairs in the candidate order of Finite behaviours. FR-120 evaluates
    a candidate's results in its result order and stops at the first one
    the postcondition accepts, so an undefined clause on a later result
    records no finding. For an operation that declares a result, the pair
    of a candidate's `k`-th result SHALL therefore be printed as
    `(and (not A1) ... (not Ak-1) Uk)`, `Aj` the conjunction of the
    postcondition clauses read with that candidate and its `j`-th result
    (the pair accepted) and `Uk` the failure condition read with the `k`-th;
    the first result's pair is `U1` alone. SMT then reports exactly the
    failures EN-1 records and replay reproduces;
    for an application none of whose postcondition clauses holds a
    construct with a definedness condition (a division or modulo, a `value`
    read, a reduction or a range coercion), it prints no term;
  - for the deadlock-freedom item, `P` is undefined at a terminal copy `p`.
- Under false-extension every condition, atom, precondition and
  postcondition failure at `p` SHALL be conjoined with `(not h<p>)`, printed as
  `(and (not h<p>) F)`, `F` the failure's term: a
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
  differs between every two copies; for a TP-2 item, two copies past the
  end differ only by a monitor state, and a live copy differs from one past
  the end), and a violation or a definedness
  failure at `depth`, the guard and postcondition failures of the
  applications from copy `depth` included (ADR-018 EN-3). Base and step at
  one depth `k` thus cover every definedness failure at every reachable
  state: the base covers positions `0..=k`, and the step covers the first
  failing state of a shortest path, whatever its distance.
- The negated property is one disjunction. Its disjuncts SHALL be, for each
  position in ascending order: the violation there, per instance in
  instance order (for the direct shape `eventually[0,u]`, its per-instance
  violations stand first, before every position; for a monitor item,
  `(= m<p>.<n> r)` per instance `n`); then the definedness failures of the
  claim, per instance in instance order, for each instance a guarded item's
  condition failure first and then its atoms' in clause-node order; then the
  precondition failures, then the postcondition failures, of the
  applications in step-disjunct order; then, for the deadlock-freedom item,
  `P`'s failure. A position with no violation and no failure contributes no
  disjunct. A failure kind that has no term at a position, such as an
  application with no partial construct in its postcondition, contributes
  none. The negated property with no disjunct at all prints `false` (the
  no-term rule of Canonical printing). With one disjunct it prints as that
  disjunct. A disjunct equal to an earlier one is
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
     the monitor constants `m<i>.<n>` in instance order, then `h<i>`;
  3. the range assertions, position by position: one per ranged value
     constant of copy `i` in declaration order, then each `m<i>.<n>`'s in
     instance order;
  4. for `Unrolling` and `InductionBase`, the initial-state predicate on
     copy 0, the disjunction over the subject's initial states in their
     list order, each the conjunction of its field values in declaration
     order (an absent optional field as `(not P)` alone); then, for a TP-2
     item, `(not h0)`;
  5. one step assertion per step, in ascending order;
  6. the monitor implications, position by position, within a position
     instance by instance in instance order, and each instance's in
     tabulation order (source state, then letter);
  7. for `InductionStep`, one distinctness assertion per pair of positions
     `(j, k)` with `j < k`, in ascending `(j, k)` order, printed as
     `(not (and Q1 ... Qn M1 ... Mm))`, `Q1` to `Qn` the field equalities of
     copy `k` with copy `j` in declaration order and `M1` to `Mm` the
     monitor equalities `(= m<k>.<n> m<j>.<n>)` in instance order (none for
     an item that is not a monitor item). For a TP-2 item it SHALL be printed
     as `(not (and (= h<k> h<j>) (or h<k> (and Q1 ... Qn)) M1 ... Mm))`: two
     copies past the end hold one position of the behaviour exactly when
     their monitor states agree (Monitors), so their field constants are
     not compared. Then one assertion per position `p` in `0..depth`, in ascending
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
its value, its limit and the setting that raises the limit, when a measure
passes its limit. Each limit is a member of the call's `SmtEncodingLimits`,
and the Limit column below is its published default. It SHALL compute
each measure from the subject, the item and the kind, check the measures in
this order, and check each before it builds the terms the measure counts:

| Measure | Counts | Limit |
| --- | --- | --- |
| `constants` | `declare-const` commands: `depth + 1` times the constants of one position, every `m<i>.<n>` and `h<i>` included | 1,048,576 (2^20) |
| `applications` | applications of one step: receivers times operations times argument vectors, summed over the operations | 65,536 (2^16) |
| `monitor-atoms` | members of a monitor item's model letter, a guarded item's condition included | 8 |
| `monitor-states` | monitor states the item's one tabulation reaches, the same for every instance | 64 |
| `candidates` | candidates of one application whose enabledness or postcondition failure the query expands | 65,536 (2^16) |
| `script-bytes` | bytes of the canonical script, counted as it is printed, stopping at the first byte past the limit | 268,435,456 (2^28) |

The table of Outputs places each measure's check among the other refusals.
`encode_hyper_query` carries the same measures and limits, read from its
request's `encoding` (Hyper self-composition queries).

Each limit is a request setting with one name at every entry point (FR-255
"Engine limit setting names", QSpec FR-461 Behavior 5): the library field of
`SmtEncodingLimits`, a replay request's `stage_limits` entry and the driver
CLI's `--limit <setting>=<n>` all raise it, and a model-check item
carries the values as FR-126's `ModelCheckRequest.smt_encoding` and, on a
provider request, as QSpec FR-368's `temporal.smt_encoding` member. Its
value as a request setting is a positive count. A direct library caller may
pass zero in `SmtEncodingLimits`; the first nonzero measure exceeds that
limit and is refused as `too-large`, with no script. The values change the result (a `too-large`
stop against a proof), so they are part of the request identity and the
cache key (QSpec FR-368, FR-306), and a result is reused only for a request
with equal values. A `too-large` refusal names the measure's limit and
setting from this table:

| Measure | Limit name | Setting | `ModelCheckLimit` |
| --- | --- | --- | --- |
| `constants` | `max_constants` | `smt_encoding.max_constants` | `SmtMaxConstants` |
| `applications` | `max_applications` | `smt_encoding.max_applications` | `SmtMaxApplications` |
| `monitor-atoms` | `max_monitor_atoms` | `smt_encoding.max_monitor_atoms` | `SmtMaxMonitorAtoms` |
| `monitor-states` | `max_monitor_states` | `smt_encoding.max_monitor_states` | `SmtMaxMonitorStates` |
| `candidates` | `max_candidates` | `smt_encoding.max_candidates` | `SmtMaxCandidates` |
| `script-bytes` | `max_script_bytes` | `smt_encoding.max_script_bytes` | `SmtMaxScriptBytes` |

```rust
pub enum ModelCheckLimit {
    // FR-126's members, FR-166's, FR-184's and FR-314's, plus the encoding
    // limits above and the solver run limits (Solver run limits):
    SmtMaxConstants, SmtMaxApplications, SmtMaxMonitorAtoms,
    SmtMaxMonitorStates, SmtMaxCandidates, SmtMaxScriptBytes,
    SmtMaxSolverSteps, SmtMaxSolverTime,
}
```

- **A reached limit settles V-7.** A `too-large` refusal is a run limit: the
  item's SMT method cannot go on under the request's limits. CG's SMT map
  SHALL settle an item whose next query the encoding refuses `too-large`,
  with no earlier query of the run having settled it, as V-7, `failed`,
  execution `resource-incomplete`, `Incomplete(LimitReached{limit, value,
  setting})`, naming the measure's `ModelCheckLimit`, the configured value
  and the setting (ADR-018 V-7, FR-127), and never as `unsupported`, V-5 or
  V-6. Raising the named setting to fit lets the same query encode. FR-182
  settles an HP-2 `EncodingRefused` outcome carrying `too-large` the same
  way.
- **The values used are recorded.** CG's SMT backend SHALL encode every
  query of an item under the one `SmtEncodingLimits` of the item's request
  (`ModelCheckRequest.smt_encoding`), the defaults for every member the
  request does not set. The item's terminal record SHALL state the values
  used, as ADR-018 V-7 requires of every limit, and the certificate request
  of an SMT proof SHALL record them as FR-338's
  `CertificateRequest.smt_encoding`, so FR-314's checker rebuilds each query under the values the
  backend encoded it under (FR-314 "Encoding limits").

### Solver run limits

CG's SMT backend SHALL run every query of an item's EN-2 and EN-3 methods
(this requirement's proof queries, EN-2's counterexample and lasso
searches, and an HP-2 item's base, step and lasso queries) under the
request's `SmtRunLimits`, which bound the SMT run itself. A model-check
item carries them as FR-126's `ModelCheckRequest.smt_run` and, on a
provider request, as QSpec FR-368's `temporal.limits.max_solver_steps` and
`max_solver_time`:

```rust
/// The SMT run's limits, each a request setting `smt.<member>` (FR-255).
/// `Default` gives every member its published default.
pub struct SmtRunLimits {
    pub max_solver_steps: u64, // ModelCheckLimit::SmtMaxSolverSteps, default 100000000 steps
    pub max_solver_time: u64,  // ModelCheckLimit::SmtMaxSolverTime, whole seconds, default 60
}
```

- `max_solver_steps` SHALL bound each query to that many solver steps, as
  the solver counts its own deterministic resource steps (cvc5's resource
  units under `--rlimit-per`, Z3's `rlimit`), and a report of it SHALL state
  the count as the solver counts it and SHALL NOT convert it to time. It is
  deterministic, so it is part of the request identity and the cache key
  (QSpec FR-306, FR-331).
- `max_solver_time` SHALL bound each query to that many whole seconds of
  wall-clock time, independent of the backend's own wall-clock limit and of
  QSpec FR-331's `deadline_ms`. It is a wall-clock setting: the request
  identity and the cache key exclude it, and a result it stops is stored by
  no cache (QSpec FR-306).
- Both are positive counts. Each has one setting name at every entry point:
  `smt.max_solver_steps` and `smt.max_solver_time` (FR-255), with the limit
  names `max_solver_steps` and `max_solver_time`.
- **A stop under either settles V-7.** When the solver stops a query because
  it reached one of these limits, CG SHALL report the stop as
  `LimitStopped{limit: ReachedLimit{limit, value}}` (FR-126's
  `ReachedLimit`), `limit` `SmtMaxSolverSteps` or `SmtMaxSolverTime` and
  `value` its configured bound, and it settles V-7, `failed`, execution `resource-incomplete`,
  `Incomplete(LimitReached{limit, value, setting})`, with `limit`
  `max_solver_steps` or `max_solver_time`, `value` the configured bound and
  `setting` `smt.max_solver_steps` or `smt.max_solver_time`, never
  `InductionNotClosed`, `BoundReached` or `TimedOut` (ADR-018 V-6, V-7).
  Raising the named setting lets the same query run further.
- **Stop order.** A solver stop of a single-subject item and of an HP-2
  item SHALL settle by the first of FR-182's stop rules that applies:
  cancellation (`Cancelled{source}`), then the backend's own wall-clock
  limit (`TimedOut`), then these two limits (`LimitReached`), then the
  solver's own undecided answer, an internal resource stop that no request
  limit names included (`InductionNotClosed{depth}`). A stop under
  `max_solver_time` is rule 3, not rule 2: the backend's own wall clock is
  a host bound the request does not name. When several apply, the first
  wins, so no stop has two causes. CG's SMT map settles a single-subject
  item's stop by these rules, as FR-182 settles an HP-2 item's.

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
    pub meter: &'a mut Meter,           // FR-107's meter, charged by `μ_U`
    pub encoding: SmtEncodingLimits,    // the request's encoding limits (Size limits)
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

The request has these five members and no other. It carries no evaluator
seam: no `StepMatch`, no `HyperLetters` and no other caller-supplied
reader of the body or of `μ`. The body's `holds` atoms and `μ_U`'s conjuncts
are read from the checked nodes that
[FR-487](FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md)
lowers, which `clause` holds. Two readers do this, and they differ in what
they produce:

- **`μ_U` is evaluated.** It reads step labels only (HS-5). The encoder
  evaluates it, once per tuple, through FR-487's `CheckedStepMatch`, which it
  builds as `CheckedStepMatch::new(package, clause)` over the one checked
  package of the request's subjects (see Product identity, "One checked
  package"), `package` being the first subject's. `CheckedStepMatch` reads no
  state and no instance, so the encoder passes one `ParameterBinding` per
  `clause.parameters` member, in parameter order, whose population is that
  parameter's population and whose key is the request's key, and the readings
  do not depend on them.
- **The body's atoms are not evaluated.** An atom is read symbolically. The
  encoder reads `clause.condition(atom)` and `clause.component_reads(atom)`
  through this requirement's expression reader (`Reader::boolean`), with
  atom slot (`t`, `p`) bound to the layout object of the key that `instance`
  holds for `p` in the layout of `t`'s subject, and each model read read in
  copy `s<i>.<r>`, `r` being the component that `component_reads` records for
  it. The result is a `QF_LIA` value term and a definedness term over the
  copies, not a reading at a concrete state. No state of any subject is
  evaluated for an atom, which is why the request needs, and has, no
  `HyperLetters`. HM-4's indexed atom `e @ t` is that read in `t`'s copy.

`meter` is the budget for evaluating `μ_U` while the query is encoded. The
caller sets it. The meter is borrowed mutably, so the caller reads what the
encoding consumed from the same `Meter` after the call, refusal or not. It is
one cumulative budget across every tuple and conjunct of the call, not a fresh
budget per tuple. `HyperProductIdentity::of` takes the request by shared
reference and never touches it. The encoding evaluates `μ_U` once per tuple, in the
joint-step order below, through `CheckedStepMatch::universal`, which charges
`meter` as FR-487 states: its conjuncts in index order, each once, stopping
at the first that is not `Completed(true)`. The encoder charges nothing
else.
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
  over the tuple's step labels (HS-5), is true or undefined. The encoder
  evaluates `μ_U`'s conjuncts as
  [FR-487](FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md)
  lowers and evaluates them (`CheckedStepMatch::universal`): `Holds` and
  `Undefined` keep the tuple, `Fails` drops it. The label of an operation
  disjunct is the `ModelTransition` that FR-120 gives the same application
  (operation, receiver and argument vector), and the label of the
  proof-query stutter is `StepLabel::Stutter`. `μ_U` reads no state, so the
  script prints no term for it, and every tuple is evaluated once whatever
  `depth` and `kind` are. An empty `μ_U` admits every tuple.
- **Undefined match.** A tuple on which `μ_U` evaluates undefined SHALL be a
  definedness failure at position `i+1` when its component disjuncts hold
  between copies `i` and `i+1` (ADR-023 HV-8, ADR-018 UE-1).
- **Property.** The body's property at a joint position SHALL be read the
  way this requirement reads a temporal item's property. For a body that
  is `always` over a state predicate, the property is the predicate at that
  position. For any other body, it is the deterministic bad-prefix monitor
  that `qsl-eval`'s property-automaton translation builds for the body
  (ADR-018 SM-6) not rejecting at `m<i>`, with its steps asserted as for a
  monitor item (Monitors), the query's one instance's monitor being
  `m<i>` itself. An indexed atom `e @ t` SHALL read `t`'s copy at the position
  (ADR-023 HM-4), and it carries its definedness condition as any atom
  does. The encoder reads each atom through its FR-487 checked condition,
  and builds and tabulates the monitor as FR-487 states: the tabulation is
  `MonitorTable`'s constructor over the body's `CheckedFormula` and the
  clause's profile, the one constructor that `MonitorTable::of` delegates to
  for an `on origin` temporal clause, so a hyper body and a temporal clause
  with the same formula give the same table. Every term the property, the
  letters and the joint steps print SHALL be linear (`QF_LIA`); an atom
  outside the fragment is refused `out-of-fragment` (Refusals).
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
  - `match-conjunct`, when `CheckedStepMatch::universal` returns `Refused`
    or `Err(MatchIncomplete)` on some tuple (a conjunct evaluates refused, or
    `meter` ran out during its one evaluation), at the conjunct it names (its
    index in `mu_universal()` and its site `hyper <c> match <k>`), naming
    the first such tuple in the joint-step order by its component disjuncts
    in quantifier order;
  - and this requirement's `field-sort`, `parameter-domain`, `field-domain`
    and `out-of-fragment` refusals, per copy, the last also for an atom
    outside the fragment, located at the atom's site `hyper <c> holds <n>`
    (FR-487) and naming the component copy it reads, and its `too-large` refusal,
    whose `constants` measure counts every copy's constants and whose
    `applications` measure counts the joint disjuncts of one step.
- **Faults.** A `MatchReading::Fault`; a `Refused` or `Undefined` reading
  or a `MatchIncomplete` whose `conjunct` is not below `mu_universal().len()`;
  a `holds` atom of the body for which `clause.condition(atom)` or
  `clause.component_reads(atom)` is `None`; a condition whose slot count
  differs from `clause.atom_slots().len()`; or a read of a condition that
  `component_reads` does not record SHALL return `SmtEncodingRefusal::Fault`
  naming that invariant (Internal faults), never a `Refused` code.
  `Undefined { conjunct, .. }` is not a refusal: the tuple is kept with the
  undefined-match definedness failure above.
- Equal requests, with meters of equal budget, and equal kinds SHALL give
  byte-equal scripts or equal refusals. The encoder owns its readers, so this
  depends on no caller behaviour. No AC drives a Faults arm: a well-formed
  request cannot break those invariants, as for FR-315-AC-29's single-subject
  fault, which needs a hand-altered checked clause inside the crate that defines the call.

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
    subjects' checked package (FR-488 emits the node and exposes the
    occurrence as `CheckedHyperClause::occurrence()`). Its node id binds
    every clause-level member
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
- **What it leaves out.** The query kind, the depth, the meter, the
  encoding limits and every run limit SHALL be no part of the identity: they choose a query of the
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
     package, or when `subjects` holds more subjects than `clause.aliases`
     (the refusal then names no alias). The checked graph keeps no map from
     an alias to its model, so the check is one subject per alias over one
     checked package;
  3. `instance-binding`, when `instance` does not hold exactly one pair per
     member of `clause.parameters`, in parameter order, each key in that
     parameter's universe, at the first parameter that breaks this. The
     parameter's universe is the key list of the populations of its alias's
     subject that admit the parameter's object type (FR-125 Instancing); when
     the subject does not admit, it is the keys of all of that subject's
     universes, so that a subject whose checked package has no emission is
     refused `product-identity` and not `instance-binding`;
  4. `product-identity`, when the subjects' checked package has no
     checked-package v2 emission, so FR-336's `SubjectIdentity::of`
     refuses, or when that emission has no `hyperproperty` node for the
     clause, or when the clause is one no package assembled
     (`CheckedHyperClause::assembled_occurrence()` is `None`, so
     `occurrence()` is the zero occurrence that names no node), at the
     clause.

  `encode_hyper_query` SHALL apply these four checks first, in the same
  order, and then `hyper-body`, `query-kind`, `too-large` for `constants`,
  the `field-sort`, `parameter-domain` and `field-domain` refusals per copy,
  `match-conjunct`, `too-large` for `applications`, `monitor-atoms`,
  `monitor-states` and `candidates`,
  `out-of-fragment` per copy, and `too-large` for `script-bytes`, refusing on the
  first that applies with no script. The `constants` measure counts declared
  constants and the `applications` measure counts joint disjuncts, so neither
  reads an atom's term and an out-of-fragment atom does not change them; that
  is why `out-of-fragment` comes after them. A fault of the Faults bullet is
  returned at the stage that first reads the broken invariant: `μ_U`'s at the
  `match-conjunct` stage, an atom's at the `out-of-fragment` stage, and it
  takes the place of any `Refused` code of that stage. For one request, the two calls
  therefore give the same refusal whenever `of` refuses.
  `HyperProductIdentity::of` SHALL evaluate no `μ_U` conjunct and charge
  nothing to `meter`.
- **Outcome handoff.** CG SHALL return every SMT result for an HP-2 item,
  EN-3's and EN-2's alike, to QSL as FR-182's `HyperSmtOutcome`, whose
  `product` is the `product` of the `HyperQuery` values it answers, or
  `HyperProductIdentity::of` over the same request when
  `encode_hyper_query` refused. `product` is
  `Result<HyperProductIdentity, InternalFault>`. When
  `HyperProductIdentity::of` over the request returns
  `SmtEncodingRefusal::Fault(f)` (and therefore `encode_hyper_query` does
  too, in its first four checks), CG has no identity to send. It SHALL then
  return `result: EncodingRefused { refusal: SmtEncodingRefusal::Fault(f) }`
  with `product: Err(f)`, and FR-182 settles it `failed`. In every other case
  `product` is `Ok`. The base and step queries of one induction
  result SHALL carry equal identities and equal depths. `settle_hyper_smt`
  settles an outcome only for the item and instance its identity names.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-315-AC-1 | Over the `Counter` subject of FR-124-AC-1 and FR-125-AC-1 (universe `{c}`, `value: Int[0, 3]`, initial value 0, operation `inc` with precondition `self.value < 3` and postcondition `self.value = pre(self.value) + 1`), under event-position false-extension, `Unrolling{depth: 3}` for the TP-2 `on origin` claim `always[0,3] holds(c.value <= 3)` declares `s0.c.value`, `h0`, `s1.c.value`, `h1`, `s2.c.value`, `h2`, `s3.c.value`, `h3` in that order, asserts the four range assertions, `(= s0.c.value 0)`, `(not h0)`, one step assertion per step and the negated property, and ends with `(check-sat)`; encoding it twice gives byte-equal scripts. | Test (TC-900) |
| FR-315-AC-2 | `InductionStep{depth: 1}` for `always holds(c.value <= 3)` asserts no initial-state predicate, asserts the property at position 0 and its violation at position 1; `InductionBase{depth: 1}` asserts the initial-state predicate on copy 0. | Test (TC-900) |
| FR-315-AC-3 | An item over a field whose sort the encoding has no form for refuses `SmtEncodingRefusal` naming the field, with its locus, and writes no script. | Test (TC-900) |
| FR-315-AC-4 | Over the `Tally` subject (universe `{t}`, fields `tally: Int[0, 3]` and `next: Int[0, 3]`, initially 0 and 1; operation `bump` with frame `modifies [tally, next]`, precondition `pre G { self.tally < 3 and self.next = self.tally + 1 }`, and postconditions `post B { self.tally - pre(self.tally) = 1 }` and `post C { self.tally < 3 and self.next = self.tally + 1 }`), under infinite-trace, `Unrolling{depth: 2}` for `always holds(t.tally < 3 and t.next = t.tally + 1)` builds the definedness condition of the range coercion of `t.tally + 1` into `Int[0, 3]` under the `and`'s short-circuit: at each position `i` in `0..=2`, a disjunct of the negated property holds when `s<i>.t.tally < 3` holds and `s<i>.t.tally + 1` lies outside `[0, 3]`. `G`'s coercion contributes the same condition over copy `i` as `bump`'s precondition failure at every position `0..=2`, position 2 included, and `C`'s coercion contributes `bump`'s postcondition failure at each position, expanded over the 16 candidates of (`tally`, `next`). `B` holds no partial construct and contributes no failure term. Every term of the script is linear. | Test (TC-900) |
| FR-315-AC-5 | `InductionStep{depth: 2}` for `always holds(c.value <= 3)` asserts that copies 0, 1 and 2 are pairwise distinct; for a TP-3 item it declares `m0.0` to `m2.0` and asserts the monitor steps, the property at a position being the monitor state there not rejecting. | Test (TC-900) |
| FR-315-AC-6 | Over `CasRefinesCounter`, the `Initial` query declares the concrete copy `s0`, asserts the initial-state predicate on it and the negation of `map(s0)` equalling an abstract initial state, and the `Step { row }` query for a `stutter` row declares `s0` and `s1`, asserts the operation's precondition on `s0` and its frame and postcondition over `s0`, `s1`, and the negation of the mapped state being unchanged; encoding either twice gives byte-equal scripts; an `any` row refuses `SmtEncodingRefusal` `refinement-any-row`; a `Step` query for an abstract-operation row (`step Impl::Counter::commitA -> Spec::Counter::inc(self);`) asserts that operation's precondition, frame and postcondition over `map(s0)`, `map(s1)` with the row's receiver and arguments evaluated at `s0`, negated; a row with a `_` argument refuses `SmtEncodingRefusal` `refinement-open-argument` naming the row and the argument, shown over `CasRefinesCounterAdd`, which is `CasRefinesCounter` with `Spec::Counter` gaining the operation `add(n: Int[0, 1])` (frame `modifies [value]`, precondition `self.value + n <= 2`, postcondition `self.value = pre(self.value) + n`) and `Impl::Counter` gaining the operation `bump()` with an empty frame, like `peek`, and the row `step Impl::Counter::bump -> Spec::Counter::add(self, _);` (one receiver and one argument for the one abstract parameter, so the row types under FR-136; `bump` is the eighth step row and its position in the `steps` order is `r`); a mapping row outside `QF_LIA` refuses `out-of-fragment` naming the row. | Test (TC-900) |
| FR-315-AC-7 | Over ADR-023 §8's secure vault and its §1 `NonInterference` clause for the instance `v`, `encode_hyper_query` with `InductionBase{depth: 1}` declares, in order, `s0.a.v.h`, `s0.a.v.l`, `s0.b.v.h`, `s0.b.v.l`, `s1.a.v.h`, `s1.a.v.l`, `s1.b.v.h`, `s1.b.v.l`, all `Int`, and no `m<i>`; asserts `a`'s initial-state predicate on copy 0 and then `b`'s; and asserts for step 0 to 1 one disjunction of exactly three joint disjuncts, `(step(0), step(0))`, `(step(1), step(1))`, then `(stutter, stutter)`. Each component's disjunct order is `step(0)`, `step(1)`, `stutter`, and `μ_U` holds on `(stutter, stutter)` because both operations are `stutter` and both arguments are absent. With the `match` block removed, the disjunction has all nine tuples in lexicographic order, `(step(0), step(0))` first and `(stutter, stutter)` last. Encoding either twice gives byte-equal scripts, and every term is linear. | Test (TC-1120) |
| FR-315-AC-8 | For the same clause, `InductionStep{depth: 1}` asserts no initial-state predicate, the property `v.l @ a = v.l @ b` at position 0, the joint states at positions 0 and 1 distinct, and its violation at position 1; `InductionStep{depth: 2}` asserts positions 0, 1 and 2 pairwise distinct. The same clause with `fair { weak V::Vault::step }` on `a` gives byte-equal scripts to it for both kinds. | Test (TC-1120) |
| FR-315-AC-9 | `NonInterference` over the secure vault with FR-176-AC-4's added `match` conjunct, which is undefined on every tuple whose `a` step is `step(0)`, keeps `(step(0), step(0))` as a joint disjunct, and `InductionBase{depth: 1}` makes the negated property hold when that disjunct holds between copies 0 and 1. | Test (TC-1120) |
| FR-315-AC-10 | Each call refuses `SmtEncodingRefusal` and writes no script: an HP-3 clause (`forall trace a of V exists trace b of V`, FR-177) names `hyper-form`; FR-176-AC-3's clause `forall trace a of V { always eventually holds(v.l @ a = 0) }` names `hyper-body`; `Unrolling{depth: 1}` for `NonInterference` names `query-kind`; ADR-023 §1's `SavesPower` with one subject, for `S`, names `subject-binding` at alias `N`; `NonInterference` with a `match` conjunct that calls a function whose evaluation exhausts the request's `meter` names `match-conjunct` at that conjunct and the tuple `(step(0), step(0))`, the first in the joint-step order. The same request with an equal budget gives the same refusal. | Test (TC-1120) |
| FR-315-AC-11 | Over the secure vault, a matched joint step sets `l` to the same input `i` in both copies, so `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for `NonInterference` are both unsatisfiable, the HP-2 outcome `Inductive{depth: 1}` that FR-182-AC-7 settles. Over a vault whose post clause is `self.l = self.h`, `InductionBase{depth: 1}` is satisfiable from the initial tuple `((0, 0), (1, 0))` with the joint step `(step(0), step(0))`. That model is a matched bad prefix, not a `HyperCounterexample`: it has no lasso, so no outcome follows from it alone, and the clause is refuted only through EN-2's lasso and FR-183's replay. | Analysis (TC-1120) |
| FR-315-AC-12 | Over ADR-023 §8's secure vault with the `vaults` universe `{v, w}`, `NonInterference`'s `InductionBase{depth: 1}`, `InductionStep{depth: 1}` and `InductionStep{depth: 2}` queries for instance `v` carry three equal `product` values, each equal to `HyperProductIdentity::of` over the same request. Its `clause()` is the `claim` occurrence of the clause's `hyperproperty` node in the vault package's checked-package v2 emission; its `subjects()` holds one member, equal to FR-336's projection of the vault subject; its `instance()` is the one pair binding the clause's parameter to `v`. The request for instance `w` gives an identity that differs only in `instance()`. The clause with `fair { weak V::Vault::step }` on `a` gives byte-equal scripts and an identity that differs in `clause()` and in the checked package of `subjects()`, because the clause is a node of the subject's checked package, and not in `instance()`. A subject that differs only in a proof bound gives an identity that differs only in `subjects()`. Meters of different budgets give equal identities. | Test (TC-1190) |
| FR-315-AC-13 | Each of these requests is refused `SmtEncodingRefusal` with the same code by both `HyperProductIdentity::of` and `encode_hyper_query`, with no identity and no script: `NonInterference` with an empty `instance` names `instance-binding` at the clause's parameter; with the key `x`, outside the universe `{v}`, it names `instance-binding` at that parameter; ADR-023 §1's `SavesPower` with its `N` subject over a checked package other than the `S` subject's names `subject-binding` at alias `N`; `NonInterference` over a subject whose checked package has no checked-package v2 emission names `product-identity` at the clause; the HP-3 clause of FR-315-AC-10 names `hyper-form`; FR-176-AC-3's liveness clause with an empty `instance` names `instance-binding`, not `hyper-body`. `HyperProductIdentity::of` over `NonInterference` with a `match` conjunct whose evaluation exhausts the request's `meter` returns the identity and leaves the meter's consumed counters at zero. | Test (TC-1190) |
| FR-315-AC-14 | A crate whose only QSL dependency is `qsl-replay` names `qsl_replay::HyperProductIdentity`, `HyperQuery`, `HyperQueryRequest`, `SubjectIdentity`, `HyperSmtOutcome` and `settle_hyper_smt` from its `replay` module, calls `encode_hyper_query`, `HyperProductIdentity::of` and `settle_hyper_smt` there, and compiles. In that crate a struct literal of `HyperProductIdentity`, serializing one and deserializing one each fail to compile. | Test (TC-1190) |
| FR-315-AC-15 | Over the `Tally` subject of FR-315-AC-4 under infinite-trace, each step assertion of `Unrolling{depth: 3}` for `always holds(t.tally < 2)` is the disjunction of exactly two disjuncts, `bump`'s and then the proof-query stutter. The stutter between copies `i` and `i+1` is the conjunction of `(= s<i+1>.t.next s<i>.t.next)` and `(= s<i+1>.t.tally s<i>.t.tally)` and holds no other conjunct: no precondition, enabledness or candidate term. | Test (TC-900) |
| FR-315-AC-16 | Read by a `QF_LIA` solver: over `Tally`, `Unrolling{depth: 3}` and `InductionBase{depth: 3}` for `always holds(t.tally < 2)` are satisfiable, the path `(0, 1)`, `(1, 2)`, `(2, 3)` reaching the violation at position 2 and depth 3 by the stutter at `(2, 3)`, where `G` holds and no candidate satisfies `C`. `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for `always holds(t.tally < 3)` are both unsatisfiable. For `Tally`'s deadlock-freedom item, with no `terminal` declaration, `InductionBase{depth: 2}` is satisfiable, `(2, 3)` being a deadlock at position 2, and `InductionBase{depth: 1}` is unsatisfiable; with DL-1's `terminal when P` for `P` the state predicate `t.tally = 2`, `InductionBase{depth: 3}` is unsatisfiable. | Analysis (TC-900) |
| FR-315-AC-17 | Over FR-315-AC-1's `Counter` under event-position false-extension, `Unrolling{depth: 5}` for `always[0,5] holds(c.value <= 3)` declares `h0` to `h5` and asserts `(not h0)`. Each step assertion `i` is printed as `(and (= h<i+1> (or h<i> (not E))) (or h<i+1> D))`, with no stutter disjunct: `D` is `inc`'s disjunct between copies `i` and `i+1`, and `E` is `inc`'s enabledness, `(and A (or B0 B1 B2 B3))`, `A` being `CanInc` on copy `i` and `Bv` being `Inc` read with `value` `v` as a literal, for the candidates 0, 1, 2 and 3 in that order. The violation at each position `p` in `0..=5` includes `h<p>`. `InductionBase{depth: 3}` for `always[0,3] holds(c.value <= 3)` refuses `query-kind`, and so does `Unrolling{depth: 2}` for `eventually[0,3] holds(c.value = 3)`. | Test (TC-900) |
| FR-315-AC-18 | Read by a `QF_LIA` solver, over FR-315-AC-1's `Counter` under event-position false-extension: `Unrolling{depth: 3}` for `always[0,3] holds(c.value <= 3)` is unsatisfiable, the V-2 query of FR-314-AC-1. `Unrolling{depth: 5}` for `always[0,5] holds(c.value <= 3)` is satisfiable, the behaviour ending at position 3 and positions 4 and 5 lying past its end (FR-125-AC-2), and so is `Unrolling{depth: 5}` for `always[0,5] holds(c.value <= 2)`. `Unrolling{depth: 3}` for `eventually[0,3] holds(c.value = 3)` is unsatisfiable, and `Unrolling{depth: 2}` for `eventually[0,2] holds(c.value = 3)` is satisfiable. | Analysis (TC-900) |
| FR-315-AC-19 | Over `Tally`, `InductionStep{depth: 1}` for `always holds(t.tally < 3)` has, among the negated property's disjuncts at position 1, `bump`'s precondition failure over copy 1 and its postcondition failure over copy 1 and the 16 candidates; each reads constants of copy 1 and literals only. The position-0 assertion before the negated property holds the property at position 0 and the negation of the same two failures over copy 0. | Test (TC-900) |
| FR-315-AC-20 | For `Tally`'s deadlock-freedom item, `InductionBase{depth: 2}` declares no `m<i>` and no `h<i>`, keeps the stutter disjunct in each step assertion, and reads the violation at position `i` as copy `i` terminal, by the expansion over `bump`'s 16 candidates. With `terminal when P` for `P` the state predicate `t.tally = 2`, the violation at position `i` is copy `i` terminal conjoined with the negation of `P` over copy `i`. | Test (TC-900) |
| FR-315-AC-21 | Each call refuses `SmtEncodingRefusal` with the code and locus given and writes no script: over `Counter`, the TP-4 claim `always eventually holds(c.value = 0)` names `item-form`; the TP-4 claim `always eventually holds(c.value = 0)` with a guarded `on each` activation `when c.value = 1` names `item-form`; `Counter` with FR-120-AC-3's operation `spawn()` (frame `creates Counter`) names `object-lifecycle` at `spawn`; `Counter` with an empty `initial` list names `no-initial-state`; `Counter` with an empty universe and an initial state that holds no object names `no-instance`; `Counter` with an operation `setTo(n: Integer)` names `parameter-domain` at `setTo` and `n`; `always holds(c.value * c.value <= 9)` names `out-of-fragment` at the product; `Unrolling{depth: 1048576}` for `always holds(c.value <= 3)` names `too-large` with measure `constants`, value 1,048,577 and limit 1,048,576, before any constant is built. Each refusal is equal across two calls. | Test (TC-900) |
| FR-315-AC-22 | Over a `Counter` subject with the universe `{d, c}`, both objects initially 0, operations `inc` and `setTo(n: Int[0, 2])` (each with frame `modifies [value]`), `InductionStep{depth: 2}` for `always holds(x.value <= 2)` with `over (x: Counter)` declares `s0.c.value`, `s0.d.value`, `s1.c.value`, `s1.d.value`, `s2.c.value`, `s2.d.value` in that order, and then asserts, in order: six range assertions in declaration order; two step assertions; three distinctness assertions for positions `(0, 1)`, `(0, 2)` and `(1, 2)`; two assertions for positions 0 and 1; the negated property; and `(check-sat)`. Each step assertion's disjuncts are `c.inc`, `c.setTo(0)`, `c.setTo(1)`, `c.setTo(2)`, `d.inc`, `d.setTo(0)`, `d.setTo(1)`, `d.setTo(2)` and the stutter, in that order. The instances are `d` then `c`, FR-125's universe-list order (FR-125-AC-10), so the negated property's first two disjuncts are the violations of instance `d` and then instance `c` at position 2. | Test (TC-900) |
| FR-315-AC-23 | For the TP-3 item `always (holds(c.value = 3) implies always holds(c.value = 3))` with `over (c: Counter)` over FR-315-AC-1's `Counter` under infinite-trace, `Unrolling{depth: 2}` asserts `(and (<= 0 m<i>.0) (<= m<i>.0 s-1))` for each position, `s` the tabulated monitor states with the initial state numbered 0; at position 0 one implication `(=> L (= m0.0 y))` per letter from the initial state; and at positions 1 and 2 one implication `(=> (and (= m<i-1>.0 x) L) (= m<i>.0 y))` per tabulated move, all after the step assertions. `InductionStep{depth: 2}` asserts no implication at position 0, so `m0.0` is constrained by its range assertion only. | Test (TC-900) |
| FR-315-AC-24 | Over FR-315-AC-1's `Counter` with an added operation `isZero(): Boolean` (empty frame, postcondition `post Z { result = (self.value = 0) }`), each step assertion's `isZero` disjunct has the postcondition part `(or Z0 Z1)`, `Z0` being `Z` read with `result` the literal `false` and `Z1` with `true`, and the script declares no constant for the result. With `Counter` given a field `id: Integer` that `inc`'s frame also writes, the false-extension query of FR-315-AC-17, which expands `inc`'s candidates, refuses `field-domain` at `c.id`. | Test (TC-900) |
| FR-315-AC-25 | Over a `Holder` subject (universe `{s}`, an optional field `held: Int[0, 3]`, initially absent; operation `put` with frame `modifies [held]` and postcondition `present(self.held)`), under infinite-trace, `InductionStep{depth: 1}` for `always holds(present(s.held) or not present(s.held))` prints the stutter between copies 0 and 1 as `(and (= s1.s.held.present s0.s.held.present) (or (not s1.s.held.present) (= s1.s.held s0.s.held)))`, and its distinctness assertion as the negation of that field equality. Two copies with `held` absent therefore agree on `held` whatever their value constants hold. | Test (TC-900) |
| FR-315-AC-26 | Over FR-315-AC-1's `Counter` under infinite-trace, a model of `InductionBase{depth: 3}` for `always holds(c.value <= 1)` that takes `inc` twice and then the stutter at value 2, where `inc` is still enabled, has failing position 2. CG reads it as the prefix `0, 1, 2`, a counterexample that FR-128 replays to `refuted`, and the stutter after position 2 is not read. For `always (holds(c.value = 0) implies eventually[0,1] holds(c.value = 1))`, a base model that stutters at value 0 and then fails is no candidate: rebuilt as the lasso with the `terminal-stutter` loop at value 0, it is refused by `replay_model_trace` with `invalid-value` naming the `terminal-stutter` loop step, and the base query settles `InductionNotClosed{depth}`, not `ReplayParity` or `ReplayRefused`. | Analysis (TC-900) |
| FR-315-AC-27 | Over FR-315-AC-4's `Tally` with an added operation `peek(): Boolean` (empty frame, postcondition `post P { result = (self.tally < 3 and self.next = self.tally + 1) }`), `bump` unchanged, the postcondition failure of `peek` at copy `i` is printed over its one candidate as `(or U1 (and (not A1) U2))`: `U1` and `A1` read with `result` `false` (first in FR-120's result order), `U2` with `true`. `peek`'s disjunct has the postcondition part `(or A1 A2)`. | Test (TC-900) |
| FR-315-AC-28 | Over FR-315-AC-1's `Counter` under event-position false-extension, a model of `Unrolling{depth: 5}` for `always[0,5] holds(c.value <= 3)` has live copies `0` to `3` and `h4`, `h5` true; CG reads it as the prefix `0, 1, 2, 3` ending at the terminal state 3, which FR-128 replays to `refuted`. A model of `Unrolling{depth: 2}` for `eventually[0,2] holds(c.value = 3)` gives the prefix `0, 1, 2`. Over the AC-26 base model for `always holds(c.value <= 1)` with a prefix step altered so that it is not enabled at its pre-state, replay refuses naming that step, and CG settles it as FR-127's table settles that refusal, V-6 `ReplayRefused` (FR-127-AC-3), not `InductionNotClosed`. A model of `Unrolling{depth: 2}` for `always[0,5] holds(c.value <= 1)` fails at the live, non-terminal position 2, below the horizon 5, so it is no CX-1 counterexample and the query gives no outcome at depth 2. | Analysis (TC-900) |
| FR-315-AC-29 | Inside `qsl-eval`, `encode_smt_query` called with FR-315-AC-2's TP-1 item `always holds(c.value <= 3)` whose checked clause has had its profile set to event-position false-extension after S3, and with FR-315-AC-23's TP-3 item altered the same way, returns `SmtEncodingRefusal::Fault` carrying `InternalFault` (`runtime_invariant`, `established-invariant-broken`) naming the invariant that a TP-1, TP-3 or deadlock-freedom item binds infinite-trace, with no script and no `Refused` code. No well-formed request carries such an item. | Test (TC-900) |
| FR-315-AC-30 | Settlement of a fault: an SMT `Inductive{depth: 1}` result for FR-315-AC-2's item whose FR-314 rebuild of the base query meets `SmtEncodingRefusal::Fault` gives `SmtProofCheck::Fault`, and FR-127 settles it `failed`, basis `unavailable`, `TerminalValue::Failed`, category failed. Over ADR-023 §8's secure vault, `settle_hyper_smt` for `NonInterference` at instance `v` with an `EncodingRefused` outcome carrying `Fault` from a later check of `encode_hyper_query`, whose `product` is `Ok` and equal to the item's identity, settles `failed`, `Failed`, not V-8. Inside `qsl-replay`'s own tests, with the request's checked clause altered after S3 so that `HyperProductIdentity::of` returns `Fault(f)`, CG's outcome is `EncodingRefused { refusal: Fault(f) }` with `product: Err(f)`, and `settle_hyper_smt` over that item writes a `failed`, `Failed` record and refuses no `ItemIdentity`. The same `Err` outcome settled for an item whose `of` returns an identity is refused `ProductMismatch` naming `clause`. | Test (TC-900) |
| FR-315-AC-31 | FR-315-AC-1's `Unrolling{depth: 3}` for `always[0,3] holds(c.value <= 3)` has, at each position `p` in `0..=3`, the violation disjuncts `h<p>` and then `(not (<= s<p>.c.value 3))`, followed by `inc`'s postcondition failure `(and (not h<p>) F)`, and no disjunct for a position past 3. `inc` has that failure term because `Inc`'s right side `pre(self.value) + 1` is an integer sum compared by `=` with the `Int[0, 3]` field `self.value`, so FR-104's equality typing (TC-1084) coerces it into `Int[0, 3]`, a range coercion with a definedness condition. The query has no atom-failure disjunct, because `c.value <= 3` holds no partial construct, and no precondition-failure disjunct, because `CanInc` (`self.value < 3`) holds none. | Test (TC-900) |
| FR-315-AC-32 | `SmtEncodingRefusal` has exactly the two arms `Refused { code, locus }`, its code one of the closed refusal set, and `Fault(InternalFault)`. No refusal code names a broken invariant. None of FR-315-AC-21's requests returns `Fault`. Inside `qsl-eval`, an encoding called with a TP-1 item whose checked clause has had its one atom removed after S3, which no well-formed request carries, returns `Fault` naming the atom invariant, not `Refused` with `item-form`. | Test (TC-900) |
| FR-315-AC-33 | Over FR-315-AC-1's `Counter` under event-position false-extension, `Unrolling{depth: 2}` for the TP-2 claim `eventually[0,1] holds(c.value = 3)` with activation `on each` declares `s0.c.value`, `m0.0`, `h0`, `s1.c.value`, `m1.0`, `h1`, `s2.c.value`, `m2.0`, `h2` in that order; asserts at each position the range assertion of `s<i>.c.value` and then `(and (<= 0 m<i>.0) (<= m<i>.0 s-1))`, `s` the tabulated monitor states; asserts `(= s0.c.value 0)`, `(not h0)` and two step assertions in FR-315-AC-17's false-extension form with no stutter disjunct; and then the monitor implications. The monitor has one letter member, the atom `A<i>` = `(= s<i>.c.value 3)`, so the position-0 implications are one per move from the initial state on the letters 0, 1 and the end letter 2, whose terms are `(and (not h0) (not A0))`, `(and (not h0) A0)` and `h0`; positions 1 and 2 have one implication per tabulated move. The negated property has, at each position `p` in `0..=2`, the violation `(= m<p>.0 r)`, `r` the rejecting state's number, followed by `inc`'s postcondition failure `(and (not h<p>) F)` (FR-315-AC-31). Encoding it twice gives byte-equal scripts. `InductionBase{depth: 1}` and `InductionStep{depth: 1}` for the same item are encoded, not refused `query-kind`; the step asserts no initial-state predicate, no `(not h0)` and no position-0 implication, and its one distinctness assertion is `(not (and (= h1 h0) (or h1 (= s1.c.value s0.c.value)) (= m1.0 m0.0)))`. | Test (TC-1201) |
| FR-315-AC-34 | Read by a `QF_LIA` solver over FR-315-AC-1's `Counter` under event-position false-extension, for `eventually[0,1] holds(c.value = 3)` `on each`: `Unrolling{depth: 2}` is satisfiable, the monitor rejecting first at position 1, where the activation at position 0 has read the values 0 and 1; CG reads it by rule 4b as the prefix `0, 1, 2` (`e` the least of the cover position `1 + 1` and the last live position 2), which FR-128 replays to `refuted`, the counterexample FR-126-AC-2 finds by EN-1. `Unrolling{depth: 1}` is satisfiable with the same rejection, and its prefix `0, 1` neither reaches the cover position 2 nor ends at a terminal state, so the query gives no outcome at depth 1. For `eventually[0,1] holds(c.value >= 1)` `on each`, `InductionStep{depth: 1}` is satisfiable, from a copy past the end whose monitor state holds an open activation, and `InductionBase{depth: 2}` and `InductionStep{depth: 2}` are both unsatisfiable, the outcome `Inductive{depth: 2}` (ADR-018 V-3). | Analysis (TC-1201) |
| FR-315-AC-35 | Over FR-315-AC-1's `Counter` under event-position false-extension, the TP-2 `on origin` claim `always[0,3] eventually[0,1] holds(c.value >= 1)`, whose horizon is 4, is a monitor item: `Unrolling{depth: 4}` declares `m<i>.0` and `h<i>` at each position 0 to 4, asserts the monitor implications of its one letter member `(>= s<i>.c.value 1)` with the end letter `h<i>`, and its violation at each position is `(= m<p>.0 r)`. Read by a `QF_LIA` solver it is unsatisfiable, the V-2 outcome `BoundedComplete{depth: 4}`. For `always[0,3] eventually[0,1] holds(c.value = 3)`, `Unrolling{depth: 4}` is satisfiable, the monitor rejecting at position 1; CG reads the prefix `0, 1, 2, 3`, ending at the terminal state 3 before the cover position 4, which FR-128 replays to `refuted`. `Unrolling{depth: 0}`, `Unrolling{depth: 1}` and `Unrolling{depth: 3}` for either claim refuse `query-kind`, below the horizon 4, so no unsatisfiable answer below the horizon can stand as a `BoundedComplete` proof of the refuted `= 3` claim; `InductionBase{depth: 4}` for either claim refuses `query-kind`. | Analysis (TC-1201) |
| FR-315-AC-36 | Over FR-315-AC-1's `Counter` under infinite-trace, the guarded TP-1 claim `always holds(c.value <= 1)` `on each` `when c.value >= 3` is a monitor item. `Unrolling{depth: 1}` declares `m0.0` and `m1.0` and no `h<i>`; its letter members are the condition `C<i>` = `(>= s<i>.c.value 3)` (bit 0) and then the atom `A<i>` = `(<= s<i>.c.value 1)` (bit 1), so letter 2's term at position `i` is `(and (not C<i>) A<i>)`; the monitor has no end letter, and each step assertion keeps the stutter disjunct. Over the same `Counter` under event-position false-extension, `Unrolling{depth: 2}` for `eventually[0,3] holds(c.value = 3)` `on each` `when c.value + 2 = c.value` has, at each position `p` in `0..=2`, after the violation `(= m<p>.0 r)`, the condition's definedness failure, the range coercion of the integer sum `c.value + 2` into `Int[0, 3]` that FR-104's equality typing makes (as in FR-315-AC-31): `s<p>.c.value + 2` outside `[0, 3]`, a linear term, conjoined as `(and (not h<p>) F)`; and then `inc`'s postcondition failure. The atom holds no partial construct, so the query has no atom-failure disjunct. The condition is false wherever it is defined, so the clause never activates, and it is undefined exactly at the values 2 and 3. | Test (TC-1201) |
| FR-315-AC-37 | Read by a `QF_LIA` solver: `Unrolling{depth: 3}` for AC-36's guarded `always holds(c.value <= 1)` `when c.value >= 3` is satisfiable, the monitor rejecting at position 3, where the activation starts at value 3; the model takes `inc` three times with no stutter, and CG reads the prefix `0, 1, 2, 3`, which FR-128 replays to `refuted`, FR-126-AC-16's counterexample. With the condition `c.value = 7`, `InductionStep{depth: 2}` is satisfiable, from two copies whose monitor state holds the activation, and `InductionBase{depth: 3}` and `InductionStep{depth: 3}` are both unsatisfiable, the outcome `Inductive{depth: 3}`. AC-36's false-extension query, whose condition `c.value + 2 = c.value` is defined and false at the values 0 and 1 and undefined at value 2, is satisfiable only by the condition failure at position 2; CG reads the prefix `0, 1, 2` (rule 4b, UE-3), which FR-128 replays to `refuted` with `kind: UndefinedEvaluation` at position 2, its site naming the activation (FR-126-AC-16). | Analysis (TC-1201) |
| FR-315-AC-38 | Over a `Counter` subject with the universe `{c, d}`, both objects initially 0, and the operation `inc` with frame `modifies self.value` (FR-154's receiver scope), under infinite-trace, the TP-3 claim `always (holds(x.value = 1) implies eventually[0,1] holds(x.value = 2))` with `over (x: Counter)` has the instances `c` (index 0) and `d` (index 1). `Unrolling{depth: 2}` declares, at each position `i`, `s<i>.c.value`, `s<i>.d.value`, `m<i>.0`, `m<i>.1` in that order; asserts the monitor implications at each position for instance 0, whose letters read `s<i>.c.value`, and then for instance 1, whose letters read `s<i>.d.value`, both from the item's one tabulation; and its negated property has, at each position `p`, `(= m<p>.0 r)` and then `(= m<p>.1 r)`. `InductionStep{depth: 1}`'s distinctness assertion is `(not (and (= s1.c.value s0.c.value) (= s1.d.value s0.d.value) (= m1.0 m0.0) (= m1.1 m0.1)))`. FR-315-AC-23's TP-3 item over this subject encodes with `m<i>.0` and `m<i>.1` and is not refused. | Test (TC-1201) |
| FR-315-AC-39 | Read by a `QF_LIA` solver: a model of AC-38's `Unrolling{depth: 2}` that applies `c.inc` and then `d.inc`, reaching `(1, 0)` and then `(1, 1)` as `(c, d)`, fails at position 2, where only instance 0's monitor rejects: `c` is 1 at position 1 and not 2 at position 2. CG names `over_binding` `c` (rule 4c) and reads the prefix of the two steps, which has no stutter, and FR-128 replays it to `refuted`. The model that applies `d.inc` and then `c.inc` names `over_binding` `d`. | Analysis (TC-1201) |
| FR-315-AC-40 | Over FR-315-AC-1's `Counter` under event-position false-extension, `Unrolling{depth: 3}` for `always[0,3] holds(c.value <= 3)`, whose script declares eight constants, encoded under `SmtEncodingLimits` with `max_constants` 8 gives a script byte-equal to the one the defaults give; with `max_constants` 7 it refuses `too-large` naming the measure `constants`, value 8, limit 7 and setting `smt_encoding.max_constants`, with no script; with `max_script_bytes` 1 it refuses `too-large` naming `script-bytes`, limit 1 and setting `smt_encoding.max_script_bytes`. FR-315-AC-23's TP-3 item with `max_monitor_states` 1 refuses `too-large` naming `monitor-states`, a value above 1, limit 1 and setting `smt_encoding.max_monitor_states`. FR-315-AC-21's `Unrolling{depth: 1048576}` under the defaults names the setting `smt_encoding.max_constants` with its measure, value and limit. ADR-023 §8's `NonInterference` `InductionBase{depth: 1}` for instance `v`, whose script declares eight constants (FR-315-AC-7), with `encoding.max_constants` 7 refuses `too-large` naming `constants`, value 8, limit 7 and `smt_encoding.max_constants`, and its `HyperProductIdentity::of` equals the identity under the default encoding limits. Each call twice gives equal results. | Test (TC-1250) |
| FR-315-AC-41 | CG's SMT map, read against these vectors: AC-40's `max_constants` 7 refusal, the item's next query with no earlier query having settled it, settles `failed`, execution `resource-incomplete`, `Incomplete(LimitReached{limit: SmtMaxConstants, value: 7, setting: smt_encoding.max_constants})`, never `unsupported`, V-5 or V-6, and its terminal record states the encoding limits used. For `always holds(c.value <= 3)` over `Counter`, an `InductionBase{depth: 1}` query that the solver stops at `max_solver_steps` 1 settles `Incomplete(LimitReached{max_solver_steps, 1, smt.max_solver_steps})`, and one stopped at `max_solver_time` 1 settles `Incomplete(LimitReached{max_solver_time, 1, smt.max_solver_time})`, each never `InductionNotClosed` or `TimedOut`; the same step-limit stop with the cancellation handle tripped settles `Incomplete(Cancelled{source})`, and with the backend's own wall-clock limit expired `Incomplete(TimedOut)`; a solver `unknown` with no limit reached settles `InductionNotClosed{depth: 1}`. Two requests that differ only in `max_solver_time` have equal request identity, and two that differ only in `max_solver_steps` differ. | Analysis (TC-1250) |
| FR-315-AC-42 | A `ModelCheckRequest` (FR-126) for `always holds(c.value <= 3)` over `Counter` carries `smt_encoding` and `smt_run`, each `Default` when the caller sets no `smt_encoding.*` or `smt.*` setting. Two requests that differ only in `smt_encoding.max_constants` (7 and 8) have different request identities and cache keys, and a result recorded for one is not reused for the other; two that differ only in `smt.max_solver_time` have equal ones. The `CertificateRequest` that the settlement builds for an SMT proof of that item carries `smt_encoding` equal to the run's `ModelCheckRequest.smt_encoding`. | Test (TC-1250) |
| FR-315-AC-43 | Over ADR-023 §8's secure vault at instance `v`, `NonInterference` with the body `always (holds(v.h @ a = v.h @ b) implies always holds(v.l @ a = v.l @ b))`, which is in the safety fragment but is not `always` over a state predicate, is read through a monitor. `InductionBase{depth: 1}` declares at each position `i` the copies `s<i>.a.v.h`, `s<i>.a.v.l`, `s<i>.b.v.h`, `s<i>.b.v.l` and then `m<i>`. It asserts `(and (<= 0 m<i>) (<= m<i> s-1))` at each position, `s` being the states of the FR-487 tabulation of the body with the initial state numbered 0. At position 0 it asserts one implication `(=> L (= m0 y))` per letter from the initial state, and at position 1 one implication `(=> (and (= m0 x) L) (= m1 y))` per tabulated move. Each `L` is built over the two atoms' terms `(= s<i>.a.v.h s<i>.b.v.h)` and `(= s<i>.a.v.l s<i>.b.v.l)`, bit `j` being the table's atom `j`. The violation at each position `p` is `(= m<p> r)`, `r` being the rejecting state's number. `InductionStep{depth: 1}` asserts no position-0 implication, and its distinctness assertion includes `(= m1 m0)`. Encoding either twice gives byte-equal scripts. With `encoding.max_monitor_states` 1, the request refuses `too-large` naming `monitor-states`, with no script. | Test (TC-1264) |

| FR-315-AC-44 | Over FR-315-AC-1's `Counter` with `Unrolling{depth: 3}` for `always[0,3] holds(c.value <= 3)`, whose canonical script declares 8 constants, direct calls with `max_constants` 8 and the default produce byte-equal scripts; limits 7 and 0 each refuse `too-large` with measure `constants`, value 8, the corresponding limit and setting `smt_encoding.max_constants`, and no script. With `max_script_bytes` 0, the query refuses `too-large` at value 1, limit 0 and setting `smt_encoding.max_script_bytes`; raising it to exactly the canonical script length succeeds, and lowering it by one refuses at the first byte past the limit. | Test (TC-1250) |
| FR-315-AC-45 | For FR-315-AC-1's `Counter`, direct calls with `max_applications` 1 and `max_candidates` 4 produce the default script; values 0 and 3 respectively refuse `too-large` with the measure's value 1 or 4, the supplied limit and corresponding `smt_encoding.*` setting. For the TP-3 item `always (holds(c.value = 3) implies always holds(c.value = 3))`, whose monitor letter has two atoms and whose tabulation reaches three states, limits `max_monitor_atoms` 2 and `max_monitor_states` 3 produce the default script; limits 1 and 2 respectively refuse `too-large` with values 2 and 3 and their corresponding setting names. | Test (TC-1250) |
| FR-315-AC-46 | `qsl_replay::HyperQueryRequest` is built by a struct literal with exactly the members `clause`, `subjects`, `instance`, `meter` and `encoding`, and has no member of type `StepMatch`, `HyperLetters` or any `dyn` reader. Over the secure vault, `encode_hyper_query` for `NonInterference` at instance `v` and `InductionBase{depth: 1}` leaves the `Meter` that the request borrows with the consumed counters that one fresh `Meter` of the same budget has after nine `CheckedStepMatch::universal` calls on it in sequence, one per tuple of FR-315-AC-7's nine in lexicographic order with the labels the encoder gives that tuple, and no other charge. `InductionBase{depth: 3}`, `InductionStep{depth: 1}` and the same request at instance `w` leave the same counters. The consumed counters of `HyperProductIdentity::of` for the request stay at zero. | Test (TC-1120) |
| FR-315-AC-47 | `NonInterference` with FR-487-AC-5's third conjunct 2, the `quire::value::ieee::sqrt` comparison under `Float64[exact]` that the kernel refuses `ieee_not_exact`, is refused `SmtEncodingRefusal` `match-conjunct` with no script by `InductionBase{depth: 1}` and `InductionStep{depth: 1}`, naming conjunct 2 of `mu_universal()`, its site `hyper <c> match 2` and the tuple `(step(0), step(0))`, the first tuple in joint-step order on which conjuncts 0 and 1 hold. The same clause with the exhausting-function conjunct of FR-315-AC-10 names the same tuple, and both give the same refusal when repeated with an equal budget. `HyperProductIdentity::of` over either returns its identity and charges nothing. | Test (TC-1120) |
| FR-315-AC-48 | `NonInterference` with FR-176-AC-5's body `always holds(1 mod (1 - v.h @ a) = 1 mod (1 - v.h @ b))`, whose divisors are not literals, is refused `SmtEncodingRefusal` `out-of-fragment` with no script by `InductionBase{depth: 1}` and `InductionStep{depth: 1}`, located at the atom's site `hyper <c> holds <n>`. The same clause with a `match` conjunct that exhausts the meter is refused `match-conjunct`, not `out-of-fragment`, the refusal order being as this requirement states. The scripts of FR-315-AC-7 and FR-315-AC-9 and the base script of FR-315-AC-43 are accepted by a `QF_LIA` solver. | Test (TC-1120) |
| FR-315-AC-49 | `HyperProductIdentity::of` and `encode_hyper_query` over `NonInterference` as S3 checked it, before any package assembled it (`assembled_occurrence()` is `None`), each refuse `SmtEncodingRefusal` `product-identity` at the clause, with no identity and no script, charging nothing to the meter. The same request over the assembled clause returns the identity of FR-315-AC-12. | Test (TC-1190) |
| FR-315-AC-50 | For the TP-3 item of FR-315-AC-45 and for `NonInterference` with the body `always (holds(v.h @ a = v.h @ b) implies always holds(v.l @ a = v.l @ b))`, the `MonitorTable` constructor over the body's `CheckedFormula` and profile gives tables with equal state counts, initial state, rejecting state and numbered moves, and `MonitorTable::of` over the TP-3 item gives that item's table through the constructor. FR-315-AC-45's and FR-315-AC-43's scripts are byte-equal to golden bytes recorded from `main` before `MonitorTable::of` delegates to the constructor, the golden files being added in the refactor's first commit. | Test (TC-1264) |

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
  replay a satisfiable query's model goes through), [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)
  (the internal fault).
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (the property forms and the bounded formulas a monitor item reads),
  [FR-338](FR-338-check-an-en-1-closure-certificate.md) (the monitor
  translation's activation, end-of-behaviour and guarded `on each` readings).
- [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md)
  (the checked hyper clause), [FR-173](FR-173-classify-hyper-clauses-into-forms.md)
  (its form), [FR-176](FR-176-check-a-universal-hyperproperty-by-self-composition.md)
  (the product the hyper queries encode),
  [FR-487](FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md)
  (the checked atoms and `match` conjuncts the hyper queries read),
  [FR-182](FR-182-settle-a-hyper-verdict.md) (the settlement of their
  outcomes).
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) (the
  settings of the encoding and solver run limits).
- [FR-336](FR-336-bound-a-model-subject-by-its-universes.md) (the subject
  identity projection), ADR-013 O-07 and O-09 (the clause occurrence and
  its no-re-mint rule), [FR-059](FR-059-check-backend-dependency-direction.md)
  and [FR-060](FR-060-check-qsl-api-surface-boundary.md) (CG names QSL only
  through `qsl-replay`, from its `replay` adapter module).

## References

- SMT-LIB 2 standard.
- The encoding limits as request settings and the solver run limits:
  Linear STD-408.
- Owning ticket: Linear QSL-366; hyper self-composition queries: Linear
  STD-311; hyper product identity: Linear STD-336; proof-query stutter,
  finite behaviours, deadlock-freedom, refusals and size limits: Linear
  STD-338; false-extension items, internal faults and violation order:
  Linear STD-340; monitor items (every bounded TP-2 formula, `on each`
  activations, guarded clauses) and per-instance monitors: Linear STD-407;
  the hyper queries' checked atoms, `match` conjuncts and body monitor:
  Linear QSL-1072; `encode_hyper_query` and its request: Linear QSL-1010
  and QSL-1083. The earlier seam-carrying draft (quire-spec-language #1484)
  was closed by SR-17048 to SR-17053.

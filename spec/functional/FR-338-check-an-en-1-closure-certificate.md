---
id: FR-338
title: "Check an EN-1 closure certificate"
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
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-225
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: references
  - target: ix://agent-ix/quire-specification/FR-370
    type: references
---
# FR-338: Check an EN-1 closure certificate

## Description

`qsl-replay`'s `check_closure` SHALL accept a `ClosureCertificate` that EN-1
returns with a safety proof exactly when the certificate's product states
contain every initial product state, are closed under expansion, and hold
no bad state, and its checker-owned non-progress check below finds no
admitted violating lasso, recomputing every successor with the core's own code
(ADR-018 PC-3, PC-5, LA-3). A proof whose certificate it accepts settles
`proved`, `Certified` (FR-127); one it rejects settles `inconclusive`,
`CertificateRejected`.

## Use case

A verification operator relies on a model-check proof of an invariant. The
engine that searched the state space lives outside the qualified core, so
the proof counts only after the core recomputes the closure the engine
claims and finds every successor inside it and no bad state.

## Inputs

- A `CertificateRequest`: the checked package recompiled by FR-098's rules,
  the model subject re-admitted by FR-120 from its byte provision and
  universes (FR-125), and the item: a TP-1, TP-2 or TP-3 clause, or the
  deadlock-freedom item (FR-124).
- Over a protocol subject with a weak `parallel` (FR-215, FR-221, FR-222),
  the request's memory bounds: `max_store_buffer: Option<u64>` and
  `max_messages: Option<u64>`, the values the run used, each `None`
  meaning 4, the default FR-225 publishes.
- Over a protocol subject with an `on each` protocol (FR-209), the
  request's instance bound: `max_live_instances: Option<u64>`, the value
  the run used, `None` meaning 3, the default ADR-027 AE-4 publishes.
- For an SMT proof checked by FR-314, the request's encoding limits:
  `smt_encoding: SmtEncodingLimits` (FR-315), the values of the run's
  `ModelCheckRequest.smt_encoding` (FR-126), under which CG's SMT backend
  encoded the item's queries. EN-1's own checks read no member of it.
- A `ClosureCertificate`.

For a protocol subject, the checker request SHALL carry both memory bounds:

```rust
pub struct ProtocolCertificateRequest<'a> {
    pub subject: ProtocolSubject<'a>,
    pub item: ModelCheckItem<'a>,
    pub max_store_buffer: Option<u64>, // None means 4 (FR-225)
    pub max_messages: Option<u64>,     // None means 4 (FR-225)
    pub max_live_instances: Option<u64>, // None means 3 (ADR-027 AE-4)
}
```

```rust
pub struct ProductStateRef {
    pub model_state: StateKey,             // quire.simulation.state-key/v1
    pub automaton_state: AutomatonStateKey, // the translation's canonical key
}
pub struct ClosureCertificate {
    pub states: Vec<ProductStateRef>,
}

pub enum CertificateRule {
    InitialMissing, SuccessorMissing, BadState, NotPartition, BackwardEdge,
    WitnessFails, QueryMismatch, ShapeMismatch,
    ProofStepInvalid, NotRefutation,
}
pub struct CertificateRejection { pub rule: CertificateRule, pub at: CertificateLocus }   // wire certificate-rejected{rule, at}
pub enum CertificateLocus {
    ProductState(ProductStateRef),
    Query { part: QueryPart },
    ProofStep { part: QueryPart, index: u64 },
}
pub enum QueryPart { Unrolling, Base, Step { row: Option<u64> } }   // FR-314

pub enum TemporalCertificateCheck {
    Accepted,
    Rejected(CertificateRejection),
    Stopped(IncompleteCause), // existing LimitReached{limit,value,setting} or Cancelled{source}
}

pub fn check_closure(
    request: &CertificateRequest<'_>,
    certificate: &ClosureCertificate,
    limits: &ModelCheckLimits, // FR-126; subject meter/expansion limits remain in request
    cancel: &Cancel,           // FR-276
) -> Result<TemporalCertificateCheck, ClosureRefusal>;

pub fn check_protocol_closure(
    request: &ProtocolCertificateRequest<'_>,
    certificate: &ClosureCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, ClosureRefusal>; // check_closure's check over a protocol subject
```

`ClosureRefusal` is the checker's refusal to check at all: the subject does
not admit (FR-106), a root is unbounded, the item has no translation or no
instance (FR-125), the subject has no initial state, a state or automaton
key has no canonical encoding, a protocol subject's protocol system does
not build (FR-205), a fairness constraint of the item names an operation the
subject does not admit (`Fairness`), or an internal invariant of the checker
broke (`Fault`). A refusal is never an acceptance.

`AutomatonStateKey` is the property-automaton translation's canonical
encoding of an automaton state (`qsl-eval`, ADR-018 LA-2), the same for
every order in which the states are materialized.

This requirement carries an `AutomatonStateKey` as the bytes of its
canonical encoding.

For TP-4, the engine may address a materialized state by `StateId`, which is
local to the `PropertyAutomaton` that issued it (ADR-018 §12). The checker and
the engine SHALL resolve that handle only through that same automaton. The
product state's `automaton_state` SHALL be the `AutomatonStateKey` obtained by
encoding the state facts under that item's `context` and the TP-4 alternatives
below; it SHALL NOT contain or derive from the numeric `StateId` or discovery
order. Reaching the same structural TP-4 state under the same context SHALL
produce identical key octets, regardless of which transition first
materialized it. The checker SHALL admit a carried key against the request's
translation context before using it for certificate membership.

FR-163 additionally owns the labelled-transition-tuple and missing-member
loci of its hyper checker; those failures are not all product-state failures.

### Rejection spellings

QSpec FR-331 carries a `CertificateRejection` as `certificate-rejected{rule,
at}`, and its Certification table owns every spelling: each locus form, its
`at` spelling, and the rules admitted at that form. QSL SHALL write a
rejection with exactly those spellings and SHALL add none of its own. A
`CertificateRule` variant is written as FR-331's kebab-case rule name, so
`InitialMissing` is `initial-missing` and `ProofStepInvalid` is
`proof-step-invalid`.

Each `CertificateLocus` alternative is one FR-331 locus form, and only the
checker named beside it returns that alternative:

| `CertificateLocus` | FR-331 locus form | Checker |
| --- | --- | --- |
| `Query { part }` | Query part | FR-314 |
| `ProofStep { part, index }` | Proof step | FR-314 |
| `ProductState(ProductStateRef)` | Product state | FR-338, FR-339 |
| `HyperProductState(ProductStateKey)` | Hyper product state | FR-163 |
| `HyperTransitionTuple` | Hyper transition tuple | FR-163 |
| `CertificateMember` | Certificate member | FR-163 (`components`), FR-201 (`ranking`, `policy`, `components`), FR-245 (`components`, `divergence`, `bounds`) |
| `ModelState(StateKey)` | State | FR-169; FR-245 (the initial discrete state) |
| `Entry { index }` | State-graph entry | FR-169 |
| `SimulationStep { position, edge, verdict }` | Simulation step | FR-149 |
| `ProbabilisticProductState(ProductKey)` | Probabilistic product state | FR-201 |
| `ZoneNode { node }` | Zone node | FR-245 |
| `ZoneEdge { edge }` | Zone edge | FR-245 |
| `ZoneComponent { component }` | Zone component | FR-245 |

A `QueryPart` is written `unrolling`, `base` or `step`, and a `Step` that
carries a row (FR-314's refinement certificate, zero-based) as `step/<r>`,
`r` the row's `U64String`, so `step/3` names the fourth step row and `step`
alone names the step query of an `Inductive` proof. A `ProofStep`'s `index`
is FR-314's command number as a `U64String`.

- Every checker that returns a `CertificateRejection` SHALL name a rule that
  FR-331's table lists at the locus form it names, at a locus form the
  table above assigns to that checker. A rejection outside that set is a
  defect of the checker, not of the certificate.
- Before it writes a `certificate-rejected` cause, the outcome writer
  (FR-286) SHALL check that FR-331's table lists the rule at the locus
  form, and that a `ModelState` outside the `quire.simulation.state-key/v1`
  domain carries `Malformed`. A pair the table does not list SHALL be QSL's internal fault,
  `InternalFault` (`runtime_invariant`, FR-096) with invariant
  `certificate-rejection-pair`, and SHALL NOT be written. No caller input
  carries the pair, so it is never an `invalid_runtime_input` refusal.
- QSL reads no `certificate-rejected` cause. FR-290's reader returns
  `Unsupported` for a plugin's, whether or not FR-331 lists its pair,
  because no in-core checker binds a plugin proof (FR-331-AC-26).

### Canonical translation state

The following is a new translation representation decision. It defines
the state that the translation owns, not an encoding of a graph's local
state number. The translation SHALL use the closed structural algebra
below for its canonical state; a private cache, address, graph index,
materialization counter or provider-selected encoding is not a member.

`AutomatonStateKey` SHALL be the RFC 8785 canonical UTF-8 octets of
`["quire.automaton-state-key/v1", context, state]`, through ADR-013's one
`quire-canonical` encoder. There is no BOM, whitespace, trailing newline
in this array. Equality SHALL compare complete octets; order SHALL be ascending unsigned
lexicographic octet order. FR-331's base64url is a transport of these
octets, not a second state identity.

`context` is `[root, profile, activation, binding]`. For a clause, `root`
is `[node,role,ordinal]`: `node` is FR-322's existing NodeId object,
carried exactly as the QSpec FR-322 node `id` member spells it;
`role` and `ordinal` are the checked clause's O-07 occurrence role and
ordinal encoded as a JSON string in the checked O-07 u64 domain. Its
lexical form is `"0"` or `[1-9][0-9]*`, with value at most
`"18446744073709551615"`; JSON numbers, signs and leading zeros refuse.
Thus checked ordinal 0 becomes `"0"`, never `0`; the u64 maximum is
lexically admissible only when that actual occurrence exists, while
`"18446744073709551616"` refuses. The checker resolves this triple in the
request's recompiled package through FR-337's clause node (the node is the
clause's emitted `temporal_clause` node, its `over` node the first
argument's parameter node), rederives its checked formula and rejects a
foreign occurrence. For the derived deadlock item, root is the closed
alternative `["deadlock-freedom"]`, selecting the fixed formula below.
`binding` is the ordered vector of `[parameter_node,bound_value]` for the
item's actual concrete over/argument binding, with NodeId as above.
For a parameter bound to an instance (an `over` parameter, FR-125
"Instancing"), `bound_value` is the closed array `["instance",key]`: `key`
is the JSON string whose value is the instance's key. An instance is
identified by its key alone (QSpec FR-361), so the context carries no
population, no object type and no `UniverseId`: a key that several
overlapping populations list gives one context, and two subjects that
admit the key with different types give the same context. For every other
parameter, `bound_value` is QSpec FR-181's complete typed canonical
value. Parameter order is the
checked item's binding order; a genuinely parameter-free item uses `[]`.
No absent/default binding is synthesized. The property state is separate
from the model state; its context does not copy the model subject. The certificate request still binds that
subject through O-09 and FR-125. `profile` is the exact admitted temporal profile identity;
`activation` is `"origin"` or `"each"` for a bounded monitor, and
`"infinite"` for an infinite-trace `on origin` property. An unguarded
infinite-trace TP-4 clause with `on each` uses `"each"`; a clause whose
`on each` activation has a `when` condition uses `"each-when"` for every property form
(TP-1 to TP-4; "Guarded `on each` activation" below). A deadlock-freedom item's existing derived
obligation identity selects its `always holds(not deadlocked)` formula.
No absent root/profile/activation is defaulted.

An occurrence reference `o` SHALL be an array of canonical unsigned decimal
strings giving the operand path from the checked temporal formula's root:
the root is `[]`, the first operand is `["0"]`, and the second operand's
first operand is `["1","0"]`. Child positions follow checked operator
operand order; they are not node-materialization indices. A shared checked
node at two operand paths has two occurrences. Atom evaluation uses that
occurrence's original checked expression and binding, not a display name.
Polarity `s` is `"positive"` or `"negative"`; it applies to the whole
subformula. Negation-normal-form dualization retains the original path
and changes polarity, so it cannot invent a source occurrence or alias
two occurrences. Constants have their checked truth; atomic negative
polarity complements their truth. An undefined atom never becomes a
Boolean state fact: FR-125's undefined letter rejects the product.

Every unsigned integer below SHALL be a decimal string, with `"0"` the
only zero, no sign or leading zero. Bounds retain the checked u64 domain;
a counter that needs `b+1` can represent `"18446744073709551616"` when
`b` is u64::MAX, so host overflow is not an alternative state. No language
bound smaller than the checked bound is introduced.

The closed alternatives for `state` are:

| Form | Canonical array and meaning |
| --- | --- |
| TP-2 undecided | `["bounded", clock, branches]`; `clock` is `["origin",remaining]` or `["each"]`. `remaining` is the remaining counted distance of the root's checked horizon. Each branch carries every still-open activation; it is not only the newest activation. |
| TP-2 decided | `["accept"]` or `["reject"]`, absorbing under EN-5; under EN-1 `on each`, an accepted activation is removed from the live branches and new activations continue, so accepting one activation is not global acceptance. |
| TP-1 / TP-3 / deadlock | `["safety", branches]`; these are the subset construction's viable elementary configurations of the formula itself after SM-6 trimming. The unique rejecting state is `["safety",[]]`. |
| TP-4, guarded `on each` | `["guarded-buchi", phase, branch]`, only under activation `"each-when"`; `phase` is `"waiting"` or `"active"` ("Guarded `on each` activation" below). |
| TP-4, unguarded `on each` | `["each-buchi", phase, branch]`, only under activation `"each"`; `phase` is `"waiting"` or `"active"`. The key represents the engine's activation-closed `always(guard implies phi)` form, with the unguarded condition fixed to `true`; its phase records whether the Büchi run has selected a violating activation. |
| TP-4, `on origin` | `["buchi", branch]`, only under activation `"infinite"`; one elementary configuration of the negated formula, with its acceptance membership. This is not a subset-determinized safety state. |

Every TP-4 key form SHALL have the infinite-trace profile. Timed profiles
use FR-234's timed relation and have no key in this state algebra.

For an unguarded TP-4 clause with `on each`, the initial key is
`["each-buchi","waiting",branch]`. At every defined letter, a waiting
state has both its waiting successor and an active successor obtained by
expanding the negated clause formula at that position; the active state
advances that selected violation and never selects a second activation.
Its acceptance sets are one per signed least-fixed-point occurrence of the
selected negated formula and the derived `["activation"]` set containing
exactly active states. This is the unguarded specialization of the
activation-closed `always(guard implies phi)` form: the true guard permits
selection at every infinite-trace position, and an infinitely waiting run
does not witness a violation.

`branches` is a set of elementary configurations; an elementary configuration
is exactly `[obligations,past,windows,acceptance]`. The four members are
present even when empty. They are closed typed sets, with the element
alternatives below; no `Value`, opaque residual bytes, unspecified tableau
member, extension map or local index is admitted.

An obligation is `[o,s]` for an unbounded/current subformula, or
`[o,s,lower,upper]` for an open interval obligation; `upper` is a finite
decimal string or `"open"`. Both bounds are remaining offsets from the
last letter read: before the lower shift completes, `lower` decreases only
by counted distance and saturates at zero; a finite `upper` decreases by
the same distance, while `"open"` stays open. The matching `windows`
entry is the canonical stored debt, not a duplicate obligation. A shift
entry is removed exactly when `lower` reaches zero, when the open operator
is expanded as its unbounded form. A bounded
`on each` activation is a conjunction of its still-open obligations,
and the entire branch holds the conjunction of all live activations.
Identical obligations are idempotent. An empty obligation conjunction is
true. `branches` is a disjunction of conjunctions; an empty disjunction
is false. After expansion, a conjunction containing both polarities of
the same current atom is removed; an obligation-free branch subsumes
every other branch with identical past/window/acceptance facts. Otherwise
only the subsumption rules below remove obligations. Semantically similar
but structurally different facts are not coalesced by a heuristic.

The canonical expansion of a conjunction SHALL use the following rules,
branching for disjunction and joining for conjunction. A negative operator
uses its De Morgan/temporal dual before this table, preserving occurrence
references. `F`, `G`, `U`, `R` mean eventually, always, until and release;
the table's `X` schedules the indicated obligations for the next model
position and is not an authored operator.

| Positive obligation | Expansion |
| --- | --- |
| constant / atom | Discharge when its checked truth meets polarity; otherwise remove the branch. |
| `and(A,B)` / `or(A,B)` | Require both operands / branch requiring either operand. |
| `F A` | Require `A` now, or carry the same `F A` to the next position. |
| `G A` | Require `A` now and carry the same `G A` to the next position. |
| `A U B` | Require `B` now, or require `A` now and carry this `U` to the next position. |
| `A R B` | Require `B` now and either require `A` now or carry this `R` to the next position. |
| finite interval with lower `l>0` | Carry without reading its operands until the lower distance is reached. Finite until/release retain FR-091's lower-bound convention, with no prefix operand debt. |
| open-upper `F[l,open] A` / `G[l,open] A`, `l>0` | Carry the shift without reading A before the lower distance. |
| `A U[l,open] B`, `l>0` | Carry the shift without reading A or B before the lower distance, then run unbounded until: A is required only from the lower distance to the witness, as for a finite interval. |
| `A R[l,open] B`, `l>0` | Carry the shift without reading A or B before the lower distance, then run unbounded release. This is the Boolean dual of the preceding until rule. |
| `F[0,u] A`, finite `u` | Require A now, or carry the same debt until the distance region closes. An undischarged F debt fails when a move would pass u. |
| `G[0,u] A`, finite `u` | Require A now and carry until the region closes; discharge at its closed upper boundary. |
| `A U[0,u] B`, finite `u` | Require B now, or require A now and carry until the region closes. An undischarged U debt fails when a move would pass u. |
| `A R[0,u] B`, finite `u` | Require B now and either require A now and discharge or carry. Discharge the surviving carry at the closed upper boundary, the dual of finite until. |
| interval `[0,open]` | The corresponding unbounded rule. |

Implication and equivalence use their checked Boolean definitions before
expansion. Each expansion step decreases Boolean structure or schedules
a temporal obligation to the next position; it SHALL NOT recursively expand
an unbounded carry at the same position. Operand obligations inherit their
own path, polarity and declared interval. Obligations created by interval
operators are represented by counters/offset sets below, never one graph
state for each syntactic `X^d` term.

`windows` SHALL contain one entry per signed future interval occurrence
with an open obligation, in one of these alternatives:

* `[o,s,"deadline",d]`: lower bound zero, no interval in an operand, an
  `F` or `U` obligation; retain only the earliest remaining deadline.
* `[o,s,"extent",d]`: the same common shape for `G` or `R`; retain only
  the furthest required remaining position.
* `[o,s,"offsets",pairs]`: every other finite interval shape, with the
  set of `[lower,upper]` remaining offsets of its open obligations.
* `[o,s,"shift",d]`: an `[a,open]` occurrence before its lower shift
  completes; at zero use its unbounded obligation and remove this entry.

The entry represents the interval obligations of that occurrence: it is
their canonical storage, not a second copy of them. Positive `F/U` use
deadline and positive `G/R` extent; negative polarity uses the dual's
kind. Offsets are measured from the last letter read, not pre-decremented for
a presumed next step. Before reading a successor letter, subtract that
edge's counted distance delta, saturating lower at zero. A finite debt
whose upper would become negative closes before that out-of-window letter
is read: an undischarged F/U fails, a surviving G/R discharges. A debt with
upper zero remains live across zero-distance successors, whose letters
must still be read. Fulfilled obligations disappear, and a new activation
adds its checked interval. Open-upper U/R shift entries retain the signed
operator occurrence, which selects the shift rule above.
Only the common lower-zero/no-nested-interval shapes use the one-counter
subsumption. General offset sets preserve distinct lower shifts and nested
obligations; neither earliest nor largest alone represents them. A state
with an entry in both obligations and windows for the same interval debt
is malformed. An empty offsets set has no entry.

`past` SHALL carry the complete previous-position memory for every past
occurrence in the checked formula's closure, including a past occurrence in
an operand of that occurrence. Memory remains present while a future carry
can later read that occurrence, even when no current activation reads it;
otherwise a delayed activation could lose earlier history. Memory is evaluated in operand dependency order
and updated once per position the monitor reads: every activation position
(a model position or an infinite-trace stutter position, "Guarded `on each`
activation" below) and every terminal false-extension position, whose atoms
read false. Its alternatives are:

| Past operator | Entry and interpretation |
| --- | --- |
| unbounded `once A` / `historically A` | `[o,"once",seen_true]` / `[o,"historically",seen_false]`, Boolean flags over all positions through the previous position. |
| unbounded `A since B` / `A triggered B` | `[o,"since",previous_truth]` / `[o,"triggered",previous_truth]`; current since is `B or (A and previous_truth)`, current triggered is `B and (A or previous_truth)`. At origin previous since is false and previous triggered true. |
| finite lower-zero `once[0,b] A` / `historically[0,b] A` | `[o,"last-true",age]` / `[o,"last-false",age]`; age is counted distance since the last true/false operand, saturated at `b+1`. Initialization reads the complete-history operand at the previous virtual position: age zero when it has the tracked truth and saturated age otherwise; constants remain constants there. |
| finite lower-zero `A since[0,b] B` / `A triggered[0,b] B` | `[o,"since-age",age]` / `[o,"triggered-age",age]`; since-age counts from the last B while A has held thereafter, resetting to saturated b+1 on a false A with no current B; triggered-age is the same counter for the dual `(not A) since[0,b] (not B)`, with its truth complemented. A current B resets since-age to zero even when A is false. |
| other finite past interval | `[o,"history",operands]`; one `[operand_path,true_offsets]` per operand, in operand order, retaining exactly the set of its true positions at offsets `1..b` from the next position. Current truth is evaluated by FR-092's past interval definition using offset zero's current operand truth and these offsets. Offsets shift by the counted delta and those beyond `b` disappear. This ordinary-position form applies when every step counts; protocol general windows use the finite distance-summary form below. |
| past `[a,open]` | `[o,"shift-history",operands,tail]`; each operand retains true offsets `1..a`; `tail` is the corresponding unbounded once/historically flag or since/triggered previous truth at the position before that retained window. Evaluate the shifted unbounded operator at offset `a`; update truth at each position and shift the window/tail only on counted distance. Protocol general windows use the distance-summary form below. |

For protocol subjects, general finite past windows and shifted open past
windows SHALL instead use `[o,"distance-history",buckets,tail]`. A bucket
is `[distance,transform]`; buckets are a duplicate-free set sorted by their
complete bytes, with distances 0..b for finite windows or 0..a for a shift.
`transform` is exactly `[false_result,true_result]`, two Booleans giving
the operator's update on the chronological positions at that distance,
for each possible incoming truth. The one-position update is `z or A`
for once, `z and A` for historically, `B or (A and z)` for since and
`B and (A or z)` for triggered. Append a same-distance letter by function
composition, not by retaining an unbounded list of positions. On a counted
step increase bucket distances by one and create a distance-zero bucket;
on an uncounted step compose into the existing zero bucket. To read a
finite interval, compose buckets in chronological order (largest distance
first) for distances a..b, starting false for once/since and true for
historically/triggered. This preserves both mixed truth and the order of
witness/prefix positions sharing a distance. For finite windows `tail` is
null. For an open shift `tail` is the Boolean result of the same recurrence
over all older positions; compose an evicted oldest bucket into it before
removal, then compose retained buckets at distance at least a to evaluate
the shifted operator. Before-origin buckets/tail follow the complete-history
rule below. Empty buckets are absent, not invented false letters. Each
bucket has only four possible Boolean transforms, so zero-distance cycles
do not grow this memory without bound. The common lower-zero age counters
remain the selected form where specified; their resets read every letter,
but increments/saturation occur only on counted steps.

The letter and distance updates SHALL be separate. An ordinary operation
subject has delta one on each step. Under ADR-027 PB-2/PB-3 every protocol
step supplies a letter and updates past truth, unbounded carries and `on each`
activation, while only attempt/cattempt, event, activate, send and receive
have delta one. Fork, join, finish, timeout, cend, duplicate, lose, spawn,
retire, fence and memory steps have delta zero. Delta zero leaves origin
horizon, future offsets/shifts and past ages unchanged; it does not suppress
atom evaluation. A terminal stutter has delta one. A finite horizon's final
distance region includes every zero-distance position before crossing;
closure on a crossing evaluates no out-of-window letter. On an ordinary
subject that region has one position and closes immediately after it.
Terminal bounded false-extension supplies counted false letters through the
remaining horizon and closes the last region; infinite-trace stutter supplies
the actual terminal letter. Digital timed deadlines are FR-196/FR-204's
separate timed relation, not this event-position distance algebra.

For historical missing positions before origin, atoms read false and
constants retain their checked truth; complete-history is not fabricated
by repeating position zero. Initial history offset sets contain exactly the
operand truths under that complete-history rule, not an unconditional empty
set. Thus `historically[0,1] holds(p)` is false at origin even when p is
currently true, while `historically[0,1] true` is true. A `history` entry is not a copied model trace:
it stores operand truth facts only. A future operator inside a past operand
requires the tableau's signed operand obligations at the historical position;
their guessed truth and carry obligations stay in the branch until discharged.
Such a guess SHALL NOT be read as an already evaluated atom or be dropped
when the history window shifts. The guessed polarity is checked through the
same expansion and Büchi acceptance as every future obligation. This keeps
past/future nesting faithful to SM-7/SM-8 without retaining model states.

`acceptance` is the set of `[o,s]` least-fixed-point occurrences whose
acceptance set this elementary state belongs to: unbounded `F`/`U` or
their signed dual, and open-upper `F`/`U[a,open]` or its signed dual on
an ordinary subject after the lower shift completes, plus finite `F`/`U`
on protocol subjects as below. An open-upper ordinary `F`/`U` has no Büchi membership while its
shift is pending; on the first letter at distance `a`, its unbounded
least-fixed-point obligation begins contributing an acceptance set. This
prevents a non-discharging open obligation from being accepted by omission.
This state
membership is explicit. The closure has one generalized Büchi acceptance set per such
signed occurrence. A branch belongs to it when that occurrence is not
carried from this position, or its current expansion discharged it through
its operand/right operand. A carried least-fixed-point obligation with no
discharge excludes that acceptance membership. Greatest-fixed-point `G/R`
requires no separate acceptance set. Finite deadlines require no Büchi
set on an ordinary subject. On a protocol subject finite F/U (including
a signed dual and a debt still before its lower shift) also have a
least-fixed-point acceptance set: a carry excludes membership and actual
discharge includes it. Counted progress makes finite debt eventually
close; zero-distance progress does not. A finite G/R is greatest-fixed-point
and needs no such set. Membership refers to occurrences, never local set numbers; the
component checker derives set order by their canonical bytes.

All sets above SHALL be sorted in ascending complete canonical element
octet order and duplicate-free. Sets of conjunctions apply only the stated
contradiction/idempotence/subsumption rules before sorting; an imported
duplicate, unsorted set or non-normal form is refused, not silently repaired.
Operand paths and vectors are ordered vectors and SHALL NOT be sorted.
Every path/polarity/window/past/acceptance member SHALL resolve in the
context-selected checked formula with the matching operator and bounds.

The canonical translator SHALL expand initial signed root obligations on
position zero. A safety monitor determinizes the elementary configurations
of the formula itself after removing configurations with no accepting
infinite continuation; a liveness automaton retains an elementary
configuration of the negated formula with generalized Büchi membership.
A bounded monitor determinizes the finite branches: at the closure of its
final counted-distance region any
unfulfilled least-fixed-point finite debt rejects and fulfilled branches
accept, with QSpec FR-091's closed boundary rules. `on each` adds the root
activation at every activation position (a model position or an
infinite-trace stutter position, never a terminal false-extension position)
and retains every prior live debt;
rejecting one activation rejects the monitor. Terminal false-extension
advances precisely the remaining bounded positions with atoms false and
constants unchanged; infinite-trace terminal stutter advances on the real
last model state's letter. No terminal/cache marker is needed because
profile, clock and all open debt/memory facts select those moves; marking
a state "closed" cannot bypass recomputation.

### Guarded `on each` activation

A clause whose `on each` activation has a `when` condition (FR-126) SHALL
use the state algebra above with the following rules. A TP-1, TP-2 or TP-3
clause adds no state alternative and no certificate member; a TP-4 clause
uses the `guarded-buchi` alternative ("Guarded TP-4 clause" below).

- The condition is the `guard` term of the clause node's activation
  argument (QSpec FR-370). The context's root names that clause node, whose
  content identity covers the guard term, so the root binds the condition,
  and the activation member `"each-when"` selects the guarded reading. A key
  carries neither the condition nor any reading of it.
- At every activation position the monitor reads, the translation SHALL read
  the condition through FR-125's letter seam for the item's binding, as part
  of that position's letter and before the clause's atoms. An infinite-trace
  terminal stutter position reads it on the real last model state's letter,
  as it reads the atoms. Where it reads true the position adds the root
  activation exactly as an unguarded `on each` position does (for TP-4, by
  the selection rule of "Guarded TP-4 clause" below); where it reads
  false the position adds none. Prior live debt and past memory advance in
  both cases.
- A guarded activation SHALL start only at an activation position: a
  position of the trace the clause reads other than a terminal
  false-extension position. A model position is an activation position, and
  so is every infinite-trace terminal stutter position, which reads the
  condition and the atoms on the real last model state's letter with the
  past memory advanced to that stutter position. A terminal false-extension
  position is not an activation position: it reads no condition and adds no
  activation, whatever the guard term, a constant `true` guard included; it
  only advances the debt already live.
- The condition's truth SHALL NOT be a state member. It is a function of the
  position's model state and the binding, so a product state's successors
  are fixed by its model state, its automaton state and that reading, and
  two positions that reach the same structural facts have the same key
  whatever the condition read at them.
- The state is `["safety", branches]` for TP-1 and TP-3 and
  `["bounded", ["each"], branches]` (or a decided form) for TP-2. With no
  live activation, `branches` holds the one obligation-free configuration
  carrying the current past memory, which is true and does not reject. Its
  `acceptance` member is the membership the rule above derives: since it
  carries no occurrence, it holds `[o,s]` for every signed least-fixed-point
  occurrence that has an acceptance set on this subject (unbounded `F`/`U`
  and their duals on every subject, finite `F`/`U` too on a protocol
  subject). Only for a formula with no past occurrence and no such
  occurrence, for example the TP-1 and TP-2 formulas of FR-338-AC-10 on the
  ordinary `Counter` subject, is the configuration `[[],[],[],[]]` and
  `branches` `[[[],[],[],[]]]`. `["safety",[]]` remains the unique rejecting
  safety state.
- In the protocol non-progress construction below, a `waiting`
  configuration of a guarded clause SHALL choose as the start of its
  negated-root activation only an activation position whose condition reads
  true.

The checker SHALL rebuild this guarded automaton from the recompiled
clause and SHALL re-read the condition at every reached product state
through the same seam, taking no condition reading from the certificate or
the engine. A misread condition changes no context, so it never by itself
causes `ShapeMismatch`; `ShapeMismatch` follows only a key whose context is
not admitted, such as one whose activation member is not `"each-when"`. A
certificate built from a misread condition is rejected `InitialMissing` when
a recomputed initial product state is absent from it, and otherwise
`SuccessorMissing` at the first recomputed successor absent from it, in the
checker's breadth-first expansion order. A certificate holding the complete
recomputed closure together with extra states built from a misread
condition is checked exactly as any certificate with unreached states: the
checker expands only reached states, so the extra states are never expanded
and the certificate is accepted when the reached closure passes every check.

A condition that reads undefined at a model state rejects with `BadState`,
as an undefined atom does (ADR-018 UE-1). No automaton state exists for a
position whose letter is undefined, so the locus is the reached product
state whose expansion read that model state's condition. For an initial
model state, which no product state precedes, the locus is the least
certificate state carrying that model state, or, when none carries it, the
least certificate state, ordered as for refused keys above. For an empty
certificate the rejection is `BadState` at `ProductState` with that initial
model state (the least such state by `model_state` octets when
several read undefined) and the item context's rejecting key,
`K_C(["safety",[]])` for TP-1 and TP-3 or `K_C(["reject"])` for TP-2: an
undefined letter is a violation (UE-1), so `BadState` is the rule that
fits, and the rejecting key is the one automaton state the checker derives
from the context alone without reading the undefined letter. Every other
outcome of the reading, an evaluation-meter stop included, follows the rule
this requirement states for an atom of the letter.

#### Guarded TP-4 clause

A TP-4 clause whose `on each` activation has a `when` condition SHALL use
a generalized Büchi automaton for the negation that selects one violating
activation: the clause is violated exactly when, from some activation
position (a model position or an infinite-trace stutter position, above)
whose condition reads true, the formula is false. Its state is
`["guarded-buchi", phase, branch]`:

- `phase` `"waiting"`: no activation has been selected. `branch` is the
  elementary configuration `[[],past,[],acceptance]`: no obligation and no
  window, the complete past memory from origin, and the membership the
  acceptance rule above derives for a configuration that carries no
  occurrence.
- `phase` `"active"`: one activation has been selected. `branch` is an
  elementary configuration of the negated formula, expanded and advanced by
  exactly the TP-4 rules above, with the past memory carried over from the
  waiting configuration that selected it.

The rules that tie the phases to the condition are:

- On reading a position's letter, a waiting configuration SHALL have the
  waiting successor that updates its past memory, and, when the condition
  reads true at that position, also every active successor obtained by
  expanding the negated root obligations at that position with that past
  memory. Where the condition reads false it has only the waiting
  successor.
- An active configuration SHALL NOT return to waiting or select a second
  activation; its successors do not depend on the condition. The condition
  is still read at every position, as part of the letter, so an undefined
  condition is found whatever the phase.
- The initial automaton states are the successors of the waiting
  configuration with origin past memory on position 0's letter, by the
  rule above.
- TP-4 is an infinite-trace form (ADR-018 TP-4), so a guarded TP-4 clause
  has no false-extension position. A terminal model state's stutter edge
  reads the condition on the real last model state's letter, as it reads
  the atoms, so a waiting configuration can select an activation on a
  stutter position, and an active one advances on it.
- The generalized Büchi acceptance sets SHALL be one per signed
  least-fixed-point occurrence `[o,s]` of the negated closure, with the
  membership rule above, and one activation set `["activation"]` whose
  members are exactly the active states. The activation set is derived from
  `phase` and is never a supplied member. The sets are ordered by the
  canonical bytes of their identities, so `["activation"]` is always the
  first, index 0. An infinitely waiting run is therefore never accepting.
- A waiting configuration has a move on every defined letter, so a guarded
  TP-4 clause's automaton is never without a move (FR-126 "Model-only
  pass").

The checker SHALL refuse, as an admission negative: a `guarded-buchi` state
under a context whose activation is not `"each-when"` or whose item is not
a TP-4 clause; an `each-buchi` state unless its activation is `"each"`
and its profile is infinite-trace; a phase other than `"waiting"` or
`"active"`; an origin `buchi` state under an `"each"` or `"each-when"`
context; and a waiting branch with an obligation or a window entry. A
well-formed shift window for a checked `F[a,open]`/ `U[a,open]` occurrence
is still refused in the waiting phase: no activation has been selected, so
there is no live interval debt.

The checker SHALL recompute every successor by these rules, re-reading the
condition at every reached product state through the letter seam and
taking no reading or phase choice from the certificate or the engine. A
certificate that omits the active successor a true condition produces is
rejected `SuccessorMissing` at it (`InitialMissing` when it is an initial
product state); one that pairs a waiting key with an obligation is refused
`ShapeMismatch` as an admission negative. An undefined condition rejects
`BadState` by the locus rule above. A TP-4 automaton has no rejecting state,
so for an empty certificate the locus key is the waiting configuration
before position 0, `K_C(["guarded-buchi","waiting",[[],P,[],A]])` with `P`
the origin past memory of the complete-history rule, with position 0's
letter not applied, and `A` its derived membership; when the condition is
undefined there for several bindings, `C` is the context of the first
binding in FR-125's canonical instance order: like the rejecting key of the safety and bounded forms, it is
derived from the context alone, without reading the undefined letter.
FR-339 checks the components over these states.

### Protocol non-progress acceptance and closure classification

For a protocol subject with any future interval, the TP-2/TP-3 source form
SHALL NOT imply that first-phase bad-prefix closure alone is a proof.
Counted bounds do not bound the number of zero-distance positions. This
is an additional acceptance obligation of a Closure certificate, not a
new fairness assumption, an unsupported-form refusal, or a projection
that deletes those positions. The original bounded/safety state keys
remain the certificate's states. The checker SHALL recompute the following
auxiliary finite construction from their actual initial states/edges and
the request's checked formula, without trusting a producer's cycle claim.

An auxiliary configuration is exactly `[phase,branch]`, where branch is
the closed elementary configuration above for the **negated** checked
formula. For origin or infinite activation phase is `"active"` throughout.
For each
activation phase is `"waiting"` or `"active"`: waiting updates the complete
past closure at every letter and may choose that position as the start of
one negated-root activation; active never returns to waiting or restarts.
Its past facts are retained from the authoritative origin. The negated-root
expansion, signed paths, Boolean choices, future/past windows, delta updates
and terminal profile closure use exactly the rules above. Discharging the
negated root keeps an obligation-free branch; a contradictory branch has
no successor. This construction selects a violating activation, rather than
requiring every activation to violate. It introduces no generic residual
or provider-defined state. Waiting is not an accepting run.

Derive one generalized Büchi set per signed least-fixed-point occurrence
in the negated closure, **including finite F/U and their pre-lower shifts**.
The membership rule above applies. Also derive one activation set consisting
exactly of active configurations, so an infinitely waiting run cannot
witness a violation. These auxiliary set identities are `[o,s]` and
`["activation"]`, sorted by complete canonical element bytes; the activation
set is present for both origin and each and is always met for origin.
It is derived from phase, never an extra Boolean supplied by the producer.
Auxiliary configurations compare by complete canonical bytes of this closed
array. They are checker work, not additional certificate members or a new
public AutomatonStateKey alternative.

The checker SHALL retain all reachable pairs of a certified product state
and auxiliary configuration by following every recomputed edge, including
counted edges on the stem. It SHALL then search the subgraph containing
only delta-zero edges for a nonempty infinite cycle that visits every
auxiliary acceptance set and satisfies **exactly** the request's resolved
fairness constraints. Apply ADR-018's SCC acceptance and ADR-019's strong
fairness refinement, deriving enabledness from the complete subject at the
paired state; no requirement that an SCC be bottom is imposed. A counted
exit does not disallow a zero-distance cycle an adversarial scheduler may
keep choosing. No progress fairness is added. Undefined letters retain
UE-1's first-phase handling.

A passing cycle is a genuine non-progress refutation: EN-1 SHALL return its
canonical stem/nonempty loop through CX-5 and replay it using PB-3 distances.
When checking an offered Closure certificate, the checker SHALL reject it
with existing `WitnessFails` at the canonical first product-state locus of
the first passing auxiliary component, ordered by complete product-state
then auxiliary bytes. This rule is an explicit extension of PC-3's checked
closure obligation; it adds no rejection variant or certificate field.
PC-4 for a TP-4 item still owns its ordinary component certificate; its
protocol negated automaton uses the same finite least-fixed-point membership
so that a pending finite F/U carry is not accepted merely by looping at one
distance. TP-1/deadlock items without future intervals retain pure bad-prefix
closure. A completed prefix at h counted steps is not a BoundedComplete
proof while an admitted unresolved non-progress branch remains.

Correctness follows by splitting an infinite finite-product run: either it
has infinitely many counted steps, when each fixed finite debt reaches its
upper region, or after a finite stem it has only zero-distance steps. In the
latter case the auxiliary acceptance keeps finite F/U strong (a witness must
actually discharge), while finite G/R can survive universally on that tail.
For an interval whose lower distance is still positive, no witness is
fabricated: F/U cannot discharge, while its Boolean dual can hold. Thus the
negated construction accepts exactly a violating activation on that tail.
Past memory, operand order, and all actual zero-distance letters remain in
the construction. Fairness filters only behaviors already excluded by the
request's existing premise.

This auxiliary construction SHALL be materialized lazily, with checked
state/edge/automaton/evaluation budgets already applicable to the request.
For one closure/component checker invocation, the state counter charges each
new canonical reached product state and each new paired auxiliary
configuration once; the transition counter charges each newly materialized
edge between those configurations once; the automaton counter charges each
new canonical original/auxiliary structural automaton state once. Reuse by
partition/component checking does not reset these counters or charge the
same fact twice. Before an allocation/charge that would exceed a configured
limit, stop at its actual consumed value, using the corresponding existing
MaxStates/MaxTransitions/MaxAutomatonStates and FR-255 setting name. Atom and
enabledness evaluation use the subject's existing FR-120 evaluation meter
and candidate limits, preserving their actual limit/value/setting on a stop.
The core checks the same Cancel handle at each charge (FR-276), and records
the actual FR-255 setting names used. Exhaustion returns the closed typed
`Stopped(IncompleteCause::LimitReached{limit,value,setting})`; cancellation
returns `Stopped(IncompleteCause::Cancelled{source})`, with its actual
Requested/Deadline source. These are the existing FR-127 stop payloads,
not CertificateRejection, WitnessFails or an out-of-band exception. The
settlement map consumes this third outcome as FR-127 specifies.
The core explores no model state outside the supplied closure, but auxiliary
configurations may multiply its states; certificate size alone is therefore
not a bound on this acceptance work. A stopped acceptance check cannot
accept the certificate or settle Certified. It follows the existing resource
stop outcome and produces no default acceptance or progress premise.

The translation SHALL materialize only states reached by the product.
Canonicalization visits only this state's retained structural facts,
sorting set members; it does not enumerate unreachable automaton states,
permutations of a finite graph or shortest reaching words. Its encoding
cost is linear in retained fact bytes plus set sorting. Logical pruning
of nonviable safety configurations remains SM-6's translation operation,
not an eager encoding requirement. Arithmetic is checked/mathematical;
FR-126's max_automaton_states and the existing identity.input_bytes budget
stop work explicitly and produce no truncated/default key. Bounds and
run budgets are not substituted for state facts.

### Canonical octet vectors

Let `C` be the rederived context of the checked formula named in each
vector. Write `K_C(S)` for the **exact UTF-8 JCS encoding** of
`["quire.automaton-state-key/v1",C,S]`; C is supplied by that checked
fixture, not an invented value. The following state bodies are exact
ASCII octets, with no invisible suffix. The full key embeds the body
after the encoded C and comma and ends with one additional `]`.

| Checked situation | Exact state-body octets / required relation |
| --- | --- |
| `always holds(false)` bad prefix, TP-1 | `["safety",[]]` |
| `always holds(true)`, TP-1, after a position | `["safety",[[[[[],"positive"]],[],[],[]]]]`; the retained root G obligation is carried. |
| TP-4 `eventually holds(p)` `on each`, before selection | `["each-buchi","waiting",[[],[],[],[]]]`; the context activation is `"each"`. |
| The same TP-4 clause after selecting a position where p is false | `["each-buchi","active",[[[[],"negative"]],[],[],[]]]`; it differs from the phase-less origin `buchi` key. |
| bounded `eventually[0,1] holds(p)`, on origin, p false at position zero | `["bounded",["origin","1"],[[[],[],[[[],"positive","deadline","1"]],[]]]]`; the stored distance is from the letter just read. |
| previous vector after p true at counted distance one | `["accept"]`; false gives `["reject"]` when that final distance region closes. A false fork before that counted step leaves clock/deadline 1 and does not reject. |
| pending common `eventually[0,3] p` debts with remaining deadlines 1 and 3 | Retain `[[],"positive","deadline","1"]`, regardless of their insertion order; extent 3 would change the future and is refused. |
| two distinct interval occurrences at `["0"]` and `["1"]` | Both windows remain even when their checked operands and counters are equal; changing either counter changes key bytes. |
| a general offset set `{[0,2],[1,3]}` | Pairs encode `[["0","2"],["1","3"]]`; reversed materialization normalizes to these bytes, while that reversed array supplied as canonical input refuses. |
| TP-4 `eventually holds(p)`, elementary negated G not-p carry | G is retained as `[[],"negative"]` with no least-fixed-point acceptance set for the dual G; this differs from a positive F carry lacking discharge. |
| guarded TP-4 `eventually holds(p)` `on each` with a `when` condition, before any activation | `["guarded-buchi","waiting",[[],[],[],[]]]`; the negated `G not p` has no least-fixed-point set, so the derived acceptance is empty. |
| the same clause after selecting an activation at a position where p is false | `["guarded-buchi","active",[[[[],"negative"]],[],[],[]]]`; the waiting key above remains reachable beside it. |

A full-key independent byte oracle uses TC-525 step 3's checked `Counter`
subject with no terminal declaration and ADR-018 DL-3's derived
`always holds(not deadlocked)` item. At its initial value zero the carried
G root has this **complete** UTF-8 key (one line, no newline in the key):

```json
["quire.automaton-state-key/v1",[["deadlock-freedom"],"quire.temporal.infinite-trace/v1","infinite",[]],["safety",[[[[[],"positive"]],[],[],[]]]]]
```

Every character is ASCII, so the displayed character sequence itself is
the independent octet oracle, without invoking the encoder under test.
The derived root has no clause ordinal and this item is genuinely
parameter-free; its binding is exactly `[]`. Replacing the profile with
`quire.temporal.event-position.false-extension/v1` changes the bytes and
refuses for this derived item. Changing only the activation to `origin`
also refuses. The clause-root lexical vector above independently checks
string ordinal zero/max versus number zero/overflow.

Additional semantic vectors: for `p until[1,*] q`, p false at origin and
q true at distance one accepts, as finite `p until[1,1] q` on the same
letters does. The dual open release accepts when its right operand holds
at every position from the lower distance on, regardless of its left
operand and of its right operand before the lower distance. For `eventually[0,1] p`, false origin, false
fork, true counted operation accepts; replacing the fork with a counted
operation closes the false final region before that later true letter.
At distance one, a false letter followed by an uncounted p-true letter
accepts; deleting the zero-distance carry is therefore an adverse change.
A protocol past bucket with p true then false at one distance gives once
transform `[true,true]` and historically `[false,false]`; reversing those
letters for a since recurrence can change its transform, while no distance
counter increments. These cases test truth updates independently of distance.

Non-progress adverse oracle: `eventually[1,1] true` on origin, at distance
zero with an infinite uncounted fork/join or memory cycle under an actual
adversarial/no-fairness premise, has no eligible witness and is violated.
The positive monitor key remains undecided with lower/upper/horizon 1;
its constant letter and a reachable counted exit do not change that fact.
The auxiliary negation is `always[1,1] false`, whose finite greatest-fixed-point
shift can persist on the zero-distance suffix, giving an accepting active
cycle with no least-fixed-point debt. A closure offered as a proof rejects
WitnessFails, and the reproduced refutation has a nonempty zero-counted loop.
If every cycle instead includes a counted edge, the same eventuality reaches
true at distance one and no such auxiliary cycle passes. An uncounted loop
at distance zero for `eventually[0,1] true` has already discharged at origin
and does not produce a false non-progress rejection. The dual
`always[1,1] false` on the first loop is true because no eligible position
exists; its auxiliary finite F shift fails acceptance. These paired oracles
prevent rejecting every pending clock indiscriminately. An authored/resolved
fairness constraint may exclude a particular starvation loop only if its
actual enabled/taken test fails; a counted exit alone is not that premise.

These are normative prospective vectors, not a runtime qualification claim.
Admission negatives include a foreign C, a node id used as an operand path,
one occurrence's debt placed at another, a u32 state index, an unknown tag,
missing members, extra members, unsorted/duplicate sets, `"01"`, negative
offsets, `lower>upper`, an offset outside the checked interval, wrong
counter strategy, unjustified accepting state, and a purported closed
bounded state that omits an older `on each` debt. Same facts reached in
different materialization orders SHALL produce identical full octets.

## Outputs

- `TemporalCertificateCheck::Accepted` only when every required check completes.
- `Rejected(CertificateRejection{rule, at})` naming an actual failed
  certificate rule and its product state locus.
- `Stopped(IncompleteCause)` carrying exactly the existing resource limit,
  actual charged value and FR-255 setting name, or cancellation source, before
  completion. A stop is neither acceptance nor bad-certificate rejection;
  FR-127 settles it `Incomplete` with those same fields.

## Behavior

- The checker SHALL rebuild the item's property automaton with
  `qsl-eval`'s translation, and SHALL read nothing from the engine but the
  certificate.
- If an `AutomatonStateKey` in a certificate state is admitted by no
  instance's translation context (a listed admission negative; an item's
  instances each have their own context), then the checker SHALL return
  `Rejected` with `ShapeMismatch` at `ProductState` carrying that state's
  refused key and model state.
- The checker SHALL NOT use a refused key for membership, expansion or
  acceptance, and one refused key SHALL reject the whole certificate however
  many other states are valid.
- When more than one state's key is refused, the checker SHALL report the
  least refused `ProductStateRef`, ordered by the `model_state`
  octets and then the `automaton_state` octets, ascending unsigned
  lexicographic, so the locus does not depend on the order of `states`.
- If the request's own clause root does not resolve in the recompiled
  package (FR-337's `clause-root-unresolved`), then the checker SHALL refuse
  the request without running the check, because that is a fault in the
  request, not in the certificate; FR-127 settles it `Failed`. A key whose
  context root differs from the request's is a foreign context and follows
  the refused-key rule above.

Note: a rejected certificate settles `inconclusive`, `CertificateRejected`
(FR-127 V-6), never `proved`.

- The checker SHALL reject with `InitialMissing` when an initial product
  state is not in the certificate.
- Starting from the initial product states, the checker SHALL expand each
  reached state through `ModelSystem`, the automaton and the terminal
  rules (FR-125), and SHALL reject with `SuccessorMissing` when a successor
  product state is not in the certificate.
- The checker SHALL reject with `BadState` when a reached state's monitor
  state rejects (a bounded-profile closure at a terminal state included),
  its letter (a guarded activation's condition included) is undefined, or its expansion records `ContractUndetermined`;
  for the deadlock-freedom item it SHALL also reject with `BadState` a
  deadlocked state (FR-124). A deadlocked state is bad for no other item:
  it reads by the terminal rules (ADR-018 DL-6).
- The checker SHALL expand no state outside the certificate, so its work is
  bounded by the certificate's model-state closure and the explicit
  auxiliary acceptance budgets above, and SHALL accept only after every
  reached state is expanded and the required non-progress check completes
  with no admitted violating lasso.

### Memory bounds

- Over a protocol subject with a weak `parallel`, the checker SHALL build
  each `Tso` component under the request's `max_store_buffer` and each `Ra`
  component under its `max_messages`, as the run built them (FR-225), and
  expand each reached state through the protocol system under them.
- The checker SHALL NOT refuse a request for its memory bounds and SHALL
  NOT compare them with the run's: they are method parameters outside the
  obligation identity (FR-225), and the closure check decides.
- `ProtocolCertificateRequest` SHALL carry `max_store_buffer` and
  `max_messages` independently, with the values used by the run; `None`
  means the corresponding FR-225 default of 4. The checker SHALL use
  `max_store_buffer` for `tso` and `max_messages` for `ra`.
- If a reached state is bound-limited under the request's bounds, an
  enabled store that the bound cuts (FR-225), then the checker SHALL reject
  with `SuccessorMissing` at that product state: the cut store's successor
  is a successor of the subject that the certificate does not show, and
  FR-225 settles V-1 only when no state was bound-limited.
- The checker SHALL check this rule at a reached state when it expands the
  state, after that state's `BadState` checks and before its successors'
  membership. Among reached states, the first in the checker's breadth-first
  expansion order that fails any rule is the one reported, as for
  `SuccessorMissing` above.
- Over an `sc` subject the checker SHALL read neither bound.

### Instance bound

- Over a protocol subject with an `on each` protocol, the checker SHALL
  expand each reached state through the protocol system under the request's
  `max_live_instances`, as the run did (FR-209): an `activate` step at a
  state with that many live instances is not expanded.
- The checker SHALL NOT refuse a request for its instance bound and SHALL
  NOT compare it with the run's: it is a method parameter outside the
  obligation identity (ADR-027 AE-4), and the closure check decides.
- If a reached state is instance-limited under the request's bound, an
  enabled `activate` step that the bound leaves unexpanded (FR-209), then
  the checker SHALL reject with `SuccessorMissing` at that product state:
  the unexpanded activation's successor is a successor of the subject that
  the certificate does not show, and FR-209 settles V-1 only when no state
  was instance-limited.
- The checker SHALL check this rule where it checks the memory-bound rule
  above, after a reached state's `BadState` checks and before its
  successors' membership, the first failing state in breadth-first
  expansion order being reported.
- Over a subject with no `on each` protocol the checker SHALL read no
  instance bound.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-338-AC-1 | Accepted vector: over ADR-018 §6's subject, FR-126-AC-2's TP-1 claim `always holds(c.versionNumber <= 1000)` returns `Holds` with a `ClosureCertificate` of its explored product states, and `check_closure` accepts it. | Test (TC-525) |
| FR-338-AC-2 | Rejected vectors: AC-1's certificate with the product state for model state `(1, 0)` removed is rejected `SuccessorMissing` at that state; with the initial product state removed, `InitialMissing` at it; over the `Counter` subject with no `terminal` declaration, a certificate holding every reachable state, offered for the deadlock-freedom item, is rejected `BadState` at value 3 (deadlocked), while the same certificate for `always holds(c.value <= 3)` is accepted, since the deadlock at 3 changes no authored claim (ADR-018 DL-6). | Test (TC-525) |
| FR-338-AC-3 | Over the `Counter` subject with `pre CanInc { 6 mod (2 - self.value) >= 0 }`, a certificate holding values 0 to 2 for `always holds(true)` is rejected `BadState` at value 2, whose expansion records `ContractUndetermined` for the undefined `pre CanInc`. | Test (TC-525) |
| FR-338-AC-4 | The canonical octet vectors have exactly the specified state bodies and full `K_C` octets; reversing subset/window insertion order leaves bytes equal, distinct occurrence paths and differing live facts leave bytes unequal, and no graph enumeration index enters the key. | Test |
| FR-338-AC-5 | Each listed admission negative refuses before the key is used for membership: a waiting `each-buchi` state carrying the well-formed shift window of its checked `eventually[1,*]` root, a `buchi` state under an unguarded `on each` context, an `each-buchi` state under an origin or `each-when` context, and a TP-4 key under a timed profile are refused; a same-shaped state under a foreign item, profile or activation is not admitted by the original context. | Test |
| FR-338-AC-6 | `eventually[0,1] p` on origin with p false then true advances to accept, false then false to reject; the final counted-distance region and terminal false-extension give the evaluator's same result; zero-distance protocol steps read letters without reducing debts, open-upper until/release retain IV-2 prefix semantics, and an ordinary open-upper `F/U[a,open]` gains its least-fixed-point acceptance set only after the lower shift completes; the non-progress lasso vectors distinguish starvation from actual counted progress without invented fairness, and on each retains an older unfulfilled activation after creating a new one; auxiliary exhaustion/cancellation returns typed Stopped with exact limit/value/setting/source and settles Incomplete rather than a certificate defect. | Test |
| FR-338-AC-7 | Canonical keys retain past memory, general offset sets, shifts and Büchi acceptance facts; changing any fact affecting a future move changes the key, while common IV-3 subsumption produces one identical counter key. u64::MAX bounds and b+1 saturation remain exact, and a reached resource budget yields no truncated key. | Analysis |
| FR-338-AC-8 | AC-1's certificate with one product state's key replaced in turn by each of three admission negatives (a foreign context, a non-canonical encoding, an unsorted set) is rejected `ShapeMismatch` at `ProductState` carrying that refused key and its model state, never `Accepted`, and settles `inconclusive`/`CertificateRejected`, not `proved`; the certificate with that state's valid key restored is accepted. With two states' keys refused, both orderings of `states` report the same, least refused `ProductStateRef`. A request whose clause root does not resolve is refused, not rejected, and settles `Failed`. | Test (TC-1080) |
| FR-338-AC-9 | Over a subject with populations `P1` and `P2` of `Counter`, each with the universe `["a"]` and `a` admitted, the claim `always holds(c.value <= 1000)` with `over (c: Counter)` has one instance, whose `context` is `[root, profile, "infinite", [[N, ["instance", "a"]]]]` with `N` the `over` parameter's NodeId; renaming `P1` or `P2` leaves the context's octets unchanged. Over populations `P1` with universe `["a"]` and `P2` with universe `["b"]`, the two instances' contexts differ exactly in the key, and a certificate state carrying `a`'s context is not admitted under `b`'s instance (a foreign context, AC-5). Each of these replacements of `["instance", "a"]` in a product state's key refuses `ShapeMismatch` at `ProductState`: FR-181's `{"type":"reference",...}` value of the key's object; the three-element `["instance", T, "a"]` with `T` the JSON string of `P1`'s declaration identity text; and `["instance", {"type":"text","value":"a"}]`. The context carries no type: over two subjects with population `P1` of `Counter` and universe `["a"]`, one whose initial state holds a `Counter` under `a` and one whose initial state holds a `Fast` (a subtype of `Counter`) under `a`, the instance's context octets are equal. | Test (TC-1164) |
| FR-338-AC-10 | Fixture: the `Counter` subject (no `terminal` declaration) and a checked package of its unit that carries the guarded clause's `temporal`/`temporal_clause` node in the shape S4 emits (FR-337-AC-6: activation aggregate `kind` `each`, `trigger` the `on each` parameter's node, `guard` the clause's Boolean guard term), built by the test so that its letters can supply readings no checked source guard gives, as in FR-126-AC-16; the same package supplies the engine's clause and stands as the request's recompiled package, and each `when` condition is read by the test's own letters as in FR-126-AC-16. `always holds(c.value <= 1)` under infinite-trace `on each` with the condition `c.value = 7` returns `Holds` whose certificate holds values 0 to 3, each with activation `"each-when"` and state body `["safety",[[[],[],[],[]]]]`, and `check_closure` accepts it, so FR-127 settles `proved`. `eventually[0,1] holds(c.value = 3)` under event-position false-extension `on each` with the condition `c.value >= 2` returns `Holds` whose certificate `check_closure` accepts, its value-0 and value-1 states having body `["bounded",["each"],[[[],[],[],[]]]]` and its value-2 state body `["bounded",["each"],[[[],[],[[[],"positive","deadline","1"]],[]]]]`; that certificate with the value-2 state's body replaced by `["bounded",["each"],[[[],[],[],[]]]]` (the condition misread as false) is rejected `SuccessorMissing` at the value-2 product state with the deadline body; the same certificate with the misread value-2 state added beside the deadline one is accepted; the accepted certificate with every key's activation replaced by `"each"` is rejected `ShapeMismatch` at the least refused `ProductStateRef` (AC-8's order). With the condition false at values 0 and 1 and undefined at value 2, the certificate holding exactly the value-0 and value-1 states, each with activation `"each-when"` and body `["bounded",["each"],[[[],[],[],[]]]]`, is rejected `BadState` at the value-1 product state, whose expansion reads the undefined condition at value 2. | Test (TC-525) |
| FR-338-AC-11 | **RETIRED (staleness check withdrawn).** The certificate carries no `package_id` and the checker compares no package identity. | Test (TC-1141), retired |
| FR-338-AC-12 | Rejection spellings. The outcome writer writes `CertificateRejected{SuccessorMissing, ProductState(s)}` with `rule` `successor-missing` and `at` the product-state form of `s`; `{QueryMismatch, Query{Unrolling}}` with `at` `"unrolling"`; `{ShapeMismatch, Query{Step{row: Some(3)}}}` with `at` `"step/3"`; `{NotRefutation, Query{Step{row: None}}}` with `at` `"step"`; `{ProofStepInvalid, ProofStep{Step{row: Some(4)}, 41}}` with `at` `{"part":"step/4","index":"41"}`; and `{RankMissing, ModelState(d)}` with `at` the state form of `d`. QSpec FR-331's schema admits each written cause. | Test (TC-1197) |
| FR-338-AC-13 | Unlisted pairs. Writing `CertificateRejected` with `BadState` at `Query{Base}`, `QueryMismatch` at `ProofStep{Unrolling, 0}`, `ProofStepInvalid` at `Query{Step{row: None}}` or `WitnessFails` at `ModelState(d)` each returns `InternalFault` (`runtime_invariant`) with invariant `certificate-rejection-pair` and writes no `certificate-rejected` cause; `NotRefutation` at `Query{Base}`, `NotRefutation` at `ProofStep{Base, 2}` and `Malformed` at `Query{Unrolling}` are written. A plugin result entry whose cause is `certificate-rejected` with `query-mismatch` at `unrolling`, and one with `bad-state` at `base`, each reads `Unsupported` (FR-290). | Test (TC-1197) |
| FR-338-AC-14 | Checker binding. Every rejection that the rejected vectors of FR-338-AC-2, AC-3 and AC-8, FR-314-AC-2 and AC-4, FR-169-AC-10, AC-11 and AC-14, FR-201-AC-6 and FR-245-AC-7 return names a `CertificateLocus` alternative that the Rejection spellings table assigns to the checker that returned it, and a rule FR-331 lists at that locus form. | Test (TC-1197) |
| FR-338-AC-16 | Over FR-339-AC-5's fixture (the clause `eventually holds(c.versionNumber = 2)` `on each` with the condition `c.versionNumber = 1`, activation `"each-when"`), binding `a`'s waiting state at `(0,0)` has body `["guarded-buchi","waiting",[[],[],[],[]]]` and its active state at `(1,0)` has body `["guarded-buchi","active",[[[[],"negative"]],[],[],[]]]`. Each of these is refused `ShapeMismatch` at `ProductState`: that active key rewritten as `["buchi",[[[[],"negative"]],[],[],[]]]`; that waiting key with `branch` `[[[[],"negative"]],[],[],[]]` under phase `"waiting"`; a `guarded-buchi` key under the same clause's context with activation `"infinite"`; the phase `"selected"`. With the condition undefined at version 0, the empty certificate is rejected `BadState` at `ProductState` with the initial model state `(0,0)` and binding `a`'s key `K_C(["guarded-buchi","waiting",[[],[],[],[]]])`, the waiting configuration before position 0's letter is read (binding `a`, the first in canonical instance order); and FR-339-AC-5's accepted certificate under that condition is rejected `BadState` at binding `a`'s waiting state at `(0,0)`, the least certificate state carrying that model state. | Test (TC-525) |
| FR-338-AC-21 | Memory bounds. Over FR-225-AC-2's `tso` program with `always holds(x.v <= 1)`, the run with `max_store_buffer` 3 settles V-1 with a certificate; the request carrying `max_store_buffer` 3 is accepted, and the same certificate under a request carrying 2 is rejected `SuccessorMissing` at the shallowest bound-limited product state, the one whose storing branch has buffered its stores to `x` and `y` while the loading branch has taken no step, with no refusal. The same program under `ra`, run with `max_messages` unset, settles V-1, and its certificate is accepted under a request with `max_messages` unset (4) and rejected `SuccessorMissing` at the initial product state under a request with 1, where the first store would add a second message. | Test (TC-1230) |
| FR-338-AC-22 | Instance bound. Over FR-209-AC-2's protocol with guard `{false}`, which has no `activate` step, the run settles V-1 with a certificate, and the certificate is accepted under a request with `max_live_instances` unset (3) and under one with 1. Over FR-209-AC-1's protocol (guard `{true}`), a certificate holding every product state the run with `max_live_instances` 2 explored, offered for `always holds(k.v <= 1)` under a request carrying 2, is rejected `SuccessorMissing` at the shallowest instance-limited product state, the one after two `activate(Tick)` steps with both instances at their start and `k.v` 0, with no refusal. | Test (TC-1230) |
| FR-338-AC-23 | TP-4 state identity. For an infinite-trace TP-4 clause, `PropertyAutomaton`'s run-local `StateId` is resolved through the automaton that issued it to a structural TP-4 state whose context-bound FR-338 key admits back to that state. Reaching one structural state through different first-discovery orders yields equal key octets even when the `StateId` values differ. | Test (TC-525) |
| FR-338-AC-24 | TP-4 product keys. Over a TP-4 clause whose reached automaton states carry at least two distinct keys, the closure certificate's automaton keys are exactly the keys an independent replay of the engine reaches under the clause's context; the checker accepts the complete certificate, and rejects `InitialMissing` or `SuccessorMissing` a certificate in which one state's key is replaced by another's, two states' keys are swapped, every key is rotated, or a product state is dropped. | Test (TC-525) |
| FR-338-AC-25 | TP-4 foreign context. A well-formed TP-4 key of another clause context in a product state is rejected `ShapeMismatch` at that `ProductState` before certificate membership is checked. | Test (TC-525) |

## Dependencies

- ADR-018 PC-3, PC-5, LA-2, LA-3.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the certificate's producer), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the settlement that runs the check), FR-225 (memory-bound values and defaults).

## References

- Owning ticket: Linear QSL-366.

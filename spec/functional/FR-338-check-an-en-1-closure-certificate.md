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
- A `ClosureCertificate`.

```rust
pub struct ProductStateRef {
    pub model_state: DigestRecord,          // quire.simulation.state-key/v1
    pub automaton_state: AutomatonStateKey, // the translation's canonical key
}
pub struct ClosureCertificate { pub states: Vec<ProductStateRef> }

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
pub enum QueryPart { Unrolling, Base, Step }   // FR-314

pub fn check_closure(request: &CertificateRequest<'_>, certificate: &ClosureCertificate)
    -> Result<(), CertificateRejection>;
```

`AutomatonStateKey` is the property-automaton translation's canonical
encoding of an automaton state (`qsl-eval`, ADR-018 LA-2), the same for
every order in which the states are materialized.

### Canonical translation state

The following is a new translation representation decision. It defines
the state that the translation owns, not an encoding of a graph's local
state number. The translation SHALL use the closed structural algebra
below for its canonical state; a private cache, address, graph index,
materialization counter or provider-selected encoding is not a member.

`AutomatonStateKey` SHALL be the RFC 8785 canonical UTF-8 octets of
`["quire.automaton-state-key/v1", context, state]`, through ADR-013's one
`quire-canonical` encoder. There is no BOM, whitespace, trailing newline,
hash or digest of this array. This discriminator identifies an octet
encoding, not a new member of QSpec FR-201's digest-domain vocabulary.
Equality SHALL compare complete octets; order SHALL be ascending unsigned
lexicographic octet order. FR-331's base64url is a transport of these
octets, not a second state identity.

`context` is `[root, profile, activation, binding]`. For a clause, `root`
is `[node,role,ordinal]`: `node` is FR-322's existing NodeId object,
exactly `{"domain":"quire.checked-semantic-node/v1","digest":hex}`;
`role` and `ordinal` are the checked clause's O-07 occurrence role and
ordinal encoded as a JSON string in the checked O-07 u64 domain. Its
lexical form is `"0"` or `[1-9][0-9]*`, with value at most
`"18446744073709551615"`; JSON numbers, signs and leading zeros refuse.
Thus checked ordinal 0 becomes `"0"`, never `0`; the u64 maximum is
lexically admissible only when that actual occurrence exists, while
`"18446744073709551616"` refuses. The checker resolves this triple in the
request's recompiled package, rederives its checked formula and rejects a
foreign occurrence. For the derived deadlock item, root is the closed
alternative `["deadlock-freedom"]`, selecting the fixed formula below.
`binding` is the ordered vector of `[parameter_node,typed_value]` for the
item's actual concrete over/argument binding, with NodeId as above and
QSpec FR-181's complete typed canonical value. Parameter order is the
checked item's binding order; a genuinely parameter-free item uses `[]`.
No absent/default binding is synthesized. The property state is separate
from the model state; its context does not copy the model subject or hash
the existing obligation again. The certificate request still binds that
subject through O-09 and FR-125. `profile` is the exact admitted temporal profile identity;
`activation` is `"origin"` or `"each"` for a bounded monitor and
`"infinite"` otherwise. A deadlock-freedom item's existing derived
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
| TP-4 | `["buchi", branch]`; one elementary configuration of the negated formula, with its acceptance membership. This is not a subset-determinized safety state. |

`branches` is a set of elementary configurations; an elementary configuration
is exactly `[obligations,past,windows,acceptance]`. The four members are
present even when empty. They are closed typed sets, with the element
alternatives below; no `Value`, opaque residual bytes, unspecified tableau
member, extension map or local index is admitted.

An obligation is `[o,s]` for an unbounded/current subformula, or
`[o,s,lower,upper]` for an open interval obligation; `upper` is a finite
decimal string or `"open"`. Bounds are remaining offsets. A bounded
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
| `A U[l,open] B`, `l>0` | Require A at every prefix position before the lower distance, and carry the shift. This is IV-2's prefix conjunction followed by unbounded until. |
| `A R[l,open] B`, `l>0` | Require A now and discharge, or carry the shift. This is the Boolean dual of the preceding until rule; B is not required in the prefix. |
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
operator occurrence, which selects the prefix rule above; a shift counter
alone never erases prefix debt.
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
and updated once per model position. Its alternatives are:

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
their signed dual, plus finite `F`/`U` on protocol subjects as below. This state
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
activation at every model position and retains every prior live debt;
rejecting one activation rejects the monitor. Terminal false-extension
advances precisely the remaining bounded positions with atoms false and
constants unchanged; infinite-trace terminal stutter advances on the real
last model state's letter. No terminal/cache marker is needed because
profile, clock and all open debt/memory facts select those moves; marking
a state "closed" cannot bypass recomputation.

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
fixture, not an invented digest. The following state bodies are exact
ASCII octets, with no invisible suffix. The full key embeds the body
after the encoded C and comma and ends with one additional `]`.

| Checked situation | Exact state-body octets / required relation |
| --- | --- |
| `always holds(false)` bad prefix, TP-1 | `["safety",[]]` |
| `always holds(true)`, TP-1, after a position | `["safety",[[[[[],"positive"]],[],[],[]]]]`; the retained root G obligation is carried. |
| bounded `eventually[0,1] holds(p)`, on origin, p false at position zero | `["bounded",["origin","1"],[[[],[],[[[],"positive","deadline","1"]],[]]]]`; the stored distance is from the letter just read. |
| previous vector after p true at counted distance one | `["accept"]`; false gives `["reject"]` when that final distance region closes. A false fork before that counted step leaves clock/deadline 1 and does not reject. |
| pending common `eventually[0,3] p` debts with remaining deadlines 1 and 3 | Retain `[[],"positive","deadline","1"]`, regardless of their insertion order; extent 3 would change the future and is refused. |
| two distinct interval occurrences at `["0"]` and `["1"]` | Both windows remain even when their checked operands and counters are equal; changing either counter changes key bytes. |
| a general offset set `{[0,2],[1,3]}` | Pairs encode `[["0","2"],["1","3"]]`; reversed materialization normalizes to these bytes, while that reversed array supplied as canonical input refuses. |
| TP-4 `eventually holds(p)`, elementary negated G not-p carry | G is retained as `[[],"negative"]` with no least-fixed-point acceptance set for the dual G; this differs from a positive F carry lacking discharge. |

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
q true at distance one rejects immediately at origin; finite
`p until[1,1] q` on the same letters accepts. The dual open release
accepts when its left operand holds in the pre-shift prefix, regardless
of its right operand there. For `eventually[0,1] p`, false origin, false
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

- `Ok(())` when the certificate is accepted.
- `CertificateRejection{rule, at}` naming the first rule that failed
  and the product state it failed at.

## Behavior

- The checker SHALL rebuild the item's property automaton with
  `qsl-eval`'s translation, and SHALL read nothing from the engine but the
  certificate.
- The checker SHALL reject with `InitialMissing` when an initial product
  state is not in the certificate.
- Starting from the initial product states, the checker SHALL expand each
  reached state through `ModelSystem`, the automaton and the terminal
  rules (FR-125), and SHALL reject with `SuccessorMissing` when a successor
  product state is not in the certificate.
- The checker SHALL reject with `BadState` when a reached state's monitor
  state rejects (a bounded-profile closure at a terminal state included),
  its letter is undefined, or its expansion records `ContractUndetermined`;
  for the deadlock-freedom item it SHALL also reject with `BadState` a
  deadlocked state (FR-124). A deadlocked state is bad for no other item:
  it reads by the terminal rules (ADR-018 DL-6).
- The checker SHALL expand no state outside the certificate, so its work is
  bounded by the certificate's model-state closure and the explicit
  auxiliary acceptance budgets above, and SHALL accept only after every
  reached state is expanded and the required non-progress check completes
  with no admitted violating lasso.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-338-AC-1 | Accepted vector: over ADR-018 §6's subject, FR-126-AC-2's TP-1 claim `always holds(c.versionNumber <= 1000)` returns `Holds` with a `ClosureCertificate` of its explored product states, and `check_closure` accepts it. | Test (TC-525) |
| FR-338-AC-2 | Rejected vectors: AC-1's certificate with the product state for model state `(1, 0)` removed is rejected `SuccessorMissing` at that state; with the initial product state removed, `InitialMissing` at it; over the `Counter` subject with no `terminal` member, a certificate holding every reachable state, offered for the deadlock-freedom item, is rejected `BadState` at value 3 (deadlocked), while the same certificate for `always holds(c.value <= 3)` is accepted, since the deadlock at 3 changes no authored claim (ADR-018 DL-6). | Test (TC-525) |
| FR-338-AC-3 | Over FR-120-AC-9's `test/tallies` subject, a certificate holding `t1` for `always holds(true)` is rejected `BadState` at `t1`, whose expansion records `ContractUndetermined` for the undefined `pre Low`. | Test (TC-525) |
| FR-338-AC-4 | The canonical octet vectors have exactly the specified state bodies and full `K_C` octets; reversing subset/window insertion order leaves bytes equal, distinct occurrence paths and differing live facts leave bytes unequal, and no graph enumeration index enters the key. | Test |
| FR-338-AC-5 | Each listed admission negative refuses before the key is used for membership; the same state under a different item/profile/activation is not admitted by the original context. | Test |
| FR-338-AC-6 | `eventually[0,1] p` on origin with p false then true advances to accept, false then false to reject; the final counted-distance region and terminal false-extension give the evaluator's same result; zero-distance protocol steps read letters without reducing debts, open-upper until/release retain IV-2 prefix semantics; the non-progress lasso vectors distinguish starvation from actual counted progress without invented fairness, and on each retains an older unfulfilled activation after creating a new one. | Test |
| FR-338-AC-7 | Canonical keys retain past memory, general offset sets, shifts and Büchi acceptance facts; changing any fact affecting a future move changes the key, while common IV-3 subsumption produces one identical counter key. u64::MAX bounds and b+1 saturation remain exact, and a reached resource budget yields no truncated key. | Analysis |

## Dependencies

- ADR-018 PC-3, PC-5, LA-2, LA-3.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md),
  [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the certificate's producer), [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the settlement that runs the check).

## References

- Owning ticket: Linear QSL-366.

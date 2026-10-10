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
no bad state, recomputing every successor with the core's own code
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
canonical decimal ordinal. The checker resolves this triple in the
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
| TP-2 undecided | `["bounded", clock, branches]`; `clock` is `["origin",remaining]` or `["each"]`. `remaining` is the number of future positions still read by the root's checked horizon. Each branch carries every still-open activation; it is not only the newest activation. |
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
| interval with lower `l>0` | Carry the same signed obligation with `l-1`; decrement finite upper, retain `open`. Nothing is required of an operand before the shift, as IV-3's `X^a` expansion specifies. |
| `F[0,u] A`, finite `u` | Require `A` now, or, if `u>0`, carry `[0,u-1]`. At `u=0` the second branch is absent. |
| `G[0,u] A`, finite `u` | Require `A` now; if `u>0`, also carry `[0,u-1]`. |
| `A U[0,u] B`, finite `u` | Require `B` now, or, if `u>0`, require `A` now and carry `[0,u-1]`. |
| `A R[0,u] B`, finite `u` | Require `B` now and either require `A` now or, if `u>0`, carry `[0,u-1]`; at zero require `B` only, the dual of finite until. |
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
kind. After a model step, offsets decrement as the table states, fulfilled
obligations disappear, and a new activation adds its checked interval.
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
| finite lower-zero `once[0,b] A` / `historically[0,b] A` | `[o,"last-true",age]` / `[o,"last-false",age]`; age is positions since the last true/false operand, saturated at `b+1`. Initialization reads the complete-history operand at the previous virtual position: age zero when it has the tracked truth and saturated age otherwise; constants remain constants there. |
| finite lower-zero `A since[0,b] B` / `A triggered[0,b] B` | `[o,"since-age",age]` / `[o,"triggered-age",age]`; since-age counts from the last B while A has held thereafter, resetting to saturated b+1 on a false A with no current B; triggered-age is the same counter for the dual `(not A) since[0,b] (not B)`, with its truth complemented. A current B resets since-age to zero even when A is false. |
| other finite past interval | `[o,"history",operands]`; one `[operand_path,true_offsets]` per operand, in operand order, retaining exactly the set of its true positions at offsets `1..b` from the next position. Current truth is evaluated by FR-092's past interval definition using offset zero's current operand truth and these offsets. Offsets shift and those beyond `b` disappear after each position. |
| past `[a,open]` | `[o,"shift-history",operands,tail]`; each operand retains true offsets `1..a`; `tail` is the corresponding unbounded once/historically flag or since/triggered previous truth at the position before that retained window. Evaluate the shifted unbounded operator at offset `a`; update window and tail once per position. |

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

`acceptance` is the set of `[o,s]` unbounded least-fixed-point obligations
(`F`/`U`, or their signed dual) whose acceptance set this elementary state
belongs to. The closure has one generalized Büchi acceptance set per such
signed occurrence. A branch belongs to it when that occurrence is not
carried from this position, or its current expansion discharged it through
its operand/right operand. A carried least-fixed-point obligation with no
discharge excludes that acceptance membership. Greatest-fixed-point `G/R`
requires no separate acceptance set. Finite deadlines require no Büchi
set. Membership refers to occurrences, never local set numbers; the
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
A bounded monitor determinizes the finite branches: at its horizon any
unfulfilled least-fixed-point finite debt rejects and fulfilled branches
accept, with QSpec FR-091's closed boundary rules. `on each` adds the root
activation at every model position and retains every prior live debt;
rejecting one activation rejects the monitor. Terminal false-extension
advances precisely the remaining bounded positions with atoms false and
constants unchanged; infinite-trace terminal stutter advances on the real
last model state's letter. No terminal/cache marker is needed because
profile, clock and all open debt/memory facts select those moves; marking
a state "closed" cannot bypass recomputation.

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
| bounded `eventually[0,1] holds(p)`, on origin, p false at position zero | `["bounded",["origin","1"],[[[],[],[[[],"positive","deadline","0"]],[]]]]`; a deadline-zero obligation is tested at the next position. |
| previous vector after p true at position one | `["accept"]`; false at that position gives `["reject"]`. |
| pending common `eventually[0,3] p` debts with remaining deadlines 1 and 3 | Retain `[[],"positive","deadline","1"]`, regardless of their insertion order; extent 3 would change the future and is refused. |
| two distinct interval occurrences at `["0"]` and `["1"]` | Both windows remain even when their checked operands and counters are equal; changing either counter changes key bytes. |
| a general offset set `{[0,2],[1,3]}` | Pairs encode `[["0","2"],["1","3"]]`; reversed materialization normalizes to these bytes, while that reversed array supplied as canonical input refuses. |
| TP-4 `eventually holds(p)`, elementary negated G not-p carry | G is retained as `[[],"negative"]` with no least-fixed-point acceptance set for the dual G; this differs from a positive F carry lacking discharge. |

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
  bounded by the certificate's size, and SHALL accept when every reached
  state is expanded.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-338-AC-1 | Accepted vector: over ADR-018 §6's subject, FR-126-AC-2's TP-1 claim `always holds(c.versionNumber <= 1000)` returns `Holds` with a `ClosureCertificate` of its explored product states, and `check_closure` accepts it. | Test (TC-525) |
| FR-338-AC-2 | Rejected vectors: AC-1's certificate with the product state for model state `(1, 0)` removed is rejected `SuccessorMissing` at that state; with the initial product state removed, `InitialMissing` at it; over the `Counter` subject with no `terminal` member, a certificate holding every reachable state, offered for the deadlock-freedom item, is rejected `BadState` at value 3 (deadlocked), while the same certificate for `always holds(c.value <= 3)` is accepted, since the deadlock at 3 changes no authored claim (ADR-018 DL-6). | Test (TC-525) |
| FR-338-AC-3 | Over FR-120-AC-9's `test/tallies` subject, a certificate holding `t1` for `always holds(true)` is rejected `BadState` at `t1`, whose expansion records `ContractUndetermined` for the undefined `pre Low`. | Test (TC-525) |
| FR-338-AC-4 | The canonical octet vectors have exactly the specified state bodies and full `K_C` octets; reversing subset/window insertion order leaves bytes equal, distinct occurrence paths and differing live facts leave bytes unequal, and no graph enumeration index enters the key. | Test |
| FR-338-AC-5 | Each listed admission negative refuses before the key is used for membership; the same state under a different item/profile/activation is not admitted by the original context. | Test |
| FR-338-AC-6 | `eventually[0,1] p` on origin with p false then true advances to accept, false then false to reject; terminal false-extension gives the evaluator's same result, and on each retains an older unfulfilled activation after creating a new one. | Test |
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

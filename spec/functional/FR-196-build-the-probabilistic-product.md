---
id: FR-196
title: "Build the probabilistic product"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-023
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-187
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: depends_on
---
# FR-196: Build the probabilistic product

## Description

QSL's `exact_probabilistic` module in layer A, crate `qsl-analyze`
(ADR-028 SP-2), SHALL build, for a probabilistic
claim, the finite product on which EN-5 computes (ADR-028 §2, SCH-2): the
MDP over FR-187's actions and draws, or under a workload the DTMC over
FR-187's step probabilities, in product with FR-126's deterministic monitor
for a bounded event and with an accumulator for a bounded measure. It SHALL
reuse `model_check`'s product `TransitionSystem`, monitor translations, edge
retention and SCC decomposition, and SHALL retain every product edge with
its action, draw and exact probability.

## Use case

EN-5 decides `P95` at `5 ms`. It builds the product of `Service` with the
event's monitor and a `duration` accumulator that saturates above `5 ms`,
every edge carrying its exact step probability, and finds every path
decided within 11 positions.

## Inputs

```rust
pub struct ExactProbRequest<'a> {
    pub subject: ModelSubject<'a>,                // FR-125
    pub claim: &'a CheckedProbabilisticClaim,     // FR-195
    pub limits: ExactProbLimits,                  // FR-203
}

// EN-5's entry, in `qsl-analyze` (ADR-028 SP-2)
pub fn check_exact_probabilistic(
    request: ExactProbRequest<'_>,
    cancel: &Cancel,                              // FR-276
) -> Result<ExactOutcome, ExactProbRefusal>;

// In `qsl-replay`, read by the settlement map (FR-203)
pub enum ExactOutcome {
    Holds { entries: Vec<ExactEntry>, certificate: ProbabilityCertificate },   // FR-201
    Violated { counterexample: ProbabilisticCounterexample },                  // FR-202
    Straddles { lower: Rational, upper: Rational },                            // PrecisionBudget
    Stopped { limit: ExactProbLimit, value: u64, setting: SettingName },       // LimitReached
}
pub enum ExactProbRefusal { Admission(AdmissionFailure), NotMarkov(NotMarkov), ZeroWeightComponent, Timed(TimedRefusal) } // TimedRefusal: FR-204

```

## Outputs

```rust
pub struct ProbProduct {
    pub states: Vec<ProductKey>,                  // canonical discovery order
    pub kind: ProductKind,                        // Dtmc or Mdp
    pub edges: Vec<ProbEdge>,
    pub initial: Vec<(u32, Option<Binding>, ProductStateId)>,
        pub decided: Vec<(ProductStateId, MonitorVerdict)>, // MonitorVerdict { Accept, Reject }: the monitor's decided states
}
// ProductKey lives in `qsl-replay` with the certificate types, so the checker reads it
pub struct ProductKey { pub model: StateKey, pub monitor: Option<u32>, pub accumulator: Option<Accumulator>, pub pending: Option<(ScheduledIdentity, Vec<Value>)> }
pub struct ProbEdge { pub from: ProductStateId, pub choice: ProductAction, pub action: Option<ScheduledIdentity>, pub draw: Option<Vec<Value>>, pub to: ProductStateId, pub probability: Rational, pub rewards: Vec<(Identifier, Value)> }
```

`ProductAction` is the following new closed semantic action type, owned with
ProductKey in qsl-replay. An edge's `action` is its operation projection;
None alone SHALL NOT identify a delay or observation choice.

```rust
pub enum ProductAction {
    Identity(ScheduledIdentity), // FR-187 exact operation/receiver/non-random arguments
    PostState(ProductKey),       // actual SCH-2 resolved complete target key
    TerminalStutter,             // existing untimed terminal stutter
    UnitDelay,                  // ordinary running delay, no predicate letter
    EnterIdleDelay,             // positive delay committing to TS-5 idle tail
    IdleDelay,                  // positive delay already in idle tail
    IdleObserve,                // clock-valued observation, no additional elapsed time
}
```

Canonical semantic action bodies use exactly `["identity",schedule]`,
`["post-state",product_key]`, `["terminal-stutter"]`, `["delay"]`,
`["enter-idle-delay"]`, `["idle-delay"]` and `["idle-observe"]` respectively,
where schedule uses the full existing schedule form below and product_key
is a parsed full canonical key, not a local index/digest/string wrapper.
Unknown/extra/missing members refuse. Action equality/order uses complete
JCS UTF-8 bytes. These are explicit new semantic choices, not a claim that
QSpec FR-413/412 transport already accepts these spellings. No fake operation
NodeId, receiver or random draw is assigned to a timed tail choice.

UnitDelay is enabled only in running mode and stays running. EnterIdleDelay
is enabled only in a TS-5 quiescent running state, advances every clock and
deadline one digital unit and commits to idle-delayed. IdleDelay is enabled
in idle-ready or idle-delayed, advances one digital unit and yields
idle-delayed. IdleObserve is enabled only in idle-delayed, emits its current
clock-valued stutter letter, increments no clock/reward and yields idle-ready.
Each delay has elapsed reward one digital unit (1/scale in the model unit);
Observe has elapsed reward zero. All delay guards/invariants still rederive.
Identity/PostState follow the actual discrete draw/reset relation and emit
a letter only when the post-state is resolved, never at a pending intermediate.
TerminalStutter retains its existing untimed profile meaning and is not a
synonym for IdleObserve. FR-202 owns choice entries/path replay, FR-201 owns
certificate policy entries, and FR-204 enumerates these actual digital actions.

### Canonical probability product key

The `monitor: Option<u32>` above describes graph-local storage only.
The following is a new normative representation decision. The producer
and checker SHALL use the complete canonical representation below for
product identity in certificate values, ranking, policy, component bias,
undefined-evaluation evidence and certificate rejection loci. A model
state digest alone or a digest of a partial tuple SHALL NOT replace it.

`probabilityProductKey` SHALL be the RFC 8785 UTF-8 JCS octets of
`["quire.probability-product-key/v1",model,monitor,accumulator,pending]`,
through ADR-013/FR-259's one encoder. These are complete octets, not a
new digest domain. Equality SHALL compare complete canonical octets;
ordering SHALL be ascending unsigned lexicographic octet order. All
array members are required. JSON null denotes actual None and SHALL NOT
be replaced with an initial/default/zero value. This defines the semantic
representation; FR-331 owns its transport and each certificate field's
binding to it.

`model` is `["quire.simulation.state-key/v1",model_form]`, with
`model_form` the full FR-101/QSpec FR-181 typed simulation-state form,
including semantic state, control, queues, roles, observations, bounds
and memory. Exploration compares its full octets, not a hash. The
existing model digest in this same domain may continue to name a model
state in traces; a certificate's complete product key contains the full
canonical model form. A strict consumer SHALL recompute any separately
supplied model digest and check its domain before using it to resolve
that form.

For FR-204 the **new closed timed alternative** is
`model = ["quire.simulation.state-key/v1",["digital",discrete,unit,scale,clocks,deadline,tail]]`.
`discrete` is exactly the preceding complete simulation-state form for
non-clock data/control/queues/roles/observations/bounds/memory. Clock fields
are absent from that data form, rather than represented by fabricated kernel
Values: ADR-026 CK-4 prohibits clock values in the value kernel. `unit` is
the model's actual CK-1 time UnitId in the closed unit form below. `scale`
is a positive canonical decimal integer: FR-204's least common denominator
of all model/claim time constants after exact conversion to that unit
(including resets and admitted discrete delay supports). One digital unit
is 1/scale of that unit. A different multiple is not an alternate encoding.

`clocks` is the complete finite CK-2 universe of clock entries
`[clock_identity,value,cap]`. `clock_identity` is
`[declaring_type_node,field_identifier,receiver]`: the checked declaring
object type's existing NodeId, its exact declared field Identifier, and
that object's complete FR-181 reference triple. The field Identifier is
resolved in that type; no new clock-node digest or display-name alias is
invented. Entries are sorted by ascending canonical clock-identity bytes,
with exactly one entry per actual object/clock field. Clock identity is
independent of object allocation/field traversal order; symmetry moves the
receiver and its clock together (ADR-026 §15). `value` and `cap` are canonical
nonnegative integer strings, `0<=value<=cap`. The checker rederives cap as
one plus that clock's largest scaled compared constant; a clock with no
comparison has cap one. A reset sets `min(scaled_reset,cap)`. A delay of k
sets `min(value+k,cap)`; unit-delay edges use k=1. Cap is present and checked,
not provider chosen. Non-clock frames and clock resets both follow TS-2.
Missing/extra/duplicate clocks, wrong receivers/types/fields/caps, alternate
scale/unit, noncanonical strings and values beyond cap refuse.

`deadline` is null exactly for the admitted timed Reach/expected-elapsed
forms without a deadline. A TA-4 deadline event uses exactly
`["deadline",D,t]`: D is the exact scaled closed bound, t the extra
non-resetting clock in `0..D+1`, initially zero. Its cap is derived D+1;
it is not an ordinary object clock and cannot be placed in `clocks`.
The time unit, scale, complete clock identities/values/caps and actual
optional deadline and tail control all participate in full identity.
`tail` is exactly `"running"`, `"idle-ready"` or `"idle-delayed"`.
Initially it is running. The two idle tags record TS-5's irreversible choice
to end discrete operations and whether a positive delay has occurred since
the last observation stutter. They are control facts, not kernel clock Values,
a provider terminal flag or a bounded-profile closure marker. Idle-ready
admits delay but no immediate stutter; idle-delayed admits another delay or
one observation stutter, which returns to idle-ready. No idle state admits
a model operation/reset. Ordinary untimed model forms cannot be substituted
for this tagged form. Contextual admission still rederives the checked
claim and its constants; this is no global equality across unrelated requests.

`monitor` is null only for Reach, ExpectedReward and LongRunFraction.
For bounded forms its closed alternatives are:

| Event/measure | Canonical monitor |
| --- | --- |
| bounded event-position formula | `["formula",automaton_key]`, where `automaton_key` is the parsed complete FR-338 canonical key array, not its local u32 index, digest or transport base64url string. Its context selects this checked formula, profile, origin activation and actual binding. |
| timed deadline event | `["deadline",kind,phase]`, where kind is `"eventually"` or `"until"` and phase is `"pending"`, `"accept"` or `"reject"`; D and the exact elapsed clock are retained in the model member above, never inferred from a position horizon. |
| accumulate / steps comparison or quantile | `["measure",remaining,phase]`, with remaining future model positions in `0..h` and phase one of `"inactive"`, `"active"`, `"complete"`, `"censored"`, `"unactivated"`. The last three phases are decided and absorbing. |
| fraction comparison or mean of fraction | `["fraction",remaining,phase]`, with phase `"collecting"` or `"complete"`; remaining is the number of included positions whose contribution is still unread, initially `h+1`, including position zero. Completion has remaining `"0"`. |

Every undecided event-position monitor SHALL retain its exact remaining
horizon/debts. A decided FR-338 `["accept"]`/`["reject"]` is absorbing and
has no remaining counter; it SHALL NOT be wrapped in a fabricated horizon.
Measure alternatives retain their explicit remaining/control through
completion, and fraction completion retains zero as specified. A timed
deadline monitor instead uses the model's exact deadline clock. Quantiles use
the same measure control for both non-strict `M<=c` and strict `M<c`
transforms; the checked claim selects the comparison. A model state's
current predicate truth is re-evaluated, never copied from a provider
index. The control phase is not inferred from a reward magnitude alone:
a completed zero measure differs from an active zero measure.

A timed deadline SHALL be initialized by reading the initial discrete
position at t=0. Eventually accepts iff P holds; otherwise it is pending.
Until accepts iff B holds, rejects iff both B and A are false, and otherwise
is pending. At each actual discrete post-step with t<=D, apply those same
rules to its post-state predicates; discrete steps and resets do not
advance or reset t. Equal-timestamp discrete steps remain distinct letters.
Ordinary delay moves advance clocks/deadline but are not new predicate positions
(ADR-026 TS-3), so they leave a pending phase unchanged while t<=D. The
first delay crossing D sets phase reject before any subsequent discrete
letter; an admitted workload race edge of k delay units then a discrete step applies that
crossing before reading its endpoint. Success exactly at D accepts, while
success after D cannot revive rejection. Accept/reject are absorbing
product control, with no pending action/accumulator synthesized.

At t=D a pending state SHALL NOT reject merely because its clock reached D:
all admissible discrete steps and TS-5 idle observation stutters at that
timestamp remain available. If no admissible such letter can meet the event
before the horizon passes, TA-6 replay/closure must re-enumerate guards,
urgency, time invariants and idle control, then close pending as reject.
A time-lock follows ADR-026 TD-2's separate item rather than inventing a
delay edge or a false-extension letter. TA-5 zero-delay cycle refusal remains
required. These timed moves do not use FR-338's counted event-position counters.
Unsupported TT-2 shapes still refuse; this alternative admits no additional
timed formula or new timed scheduler premise.

A quiescent state SHALL retain ADR-026 TS-5's timed idle-tail alternative,
even if the behavior chooses to end discrete operations there while later
operations could otherwise be enabled. The first positive unit delay on
that chosen tail changes running to idle-delayed. Subsequent tail delays
advance every clock and the deadline exactly as ordinary delay does, and
keep idle-delayed. At a chosen observation instant, an idle-delayed state
emits one terminal-stutter letter from its **current clock valuation** and
unchanged non-clock state, then becomes idle-ready. That observation has
no reset or extra elapsed increment: the preceding positive delay was
already applied. Each subsequent stutter requires another positive delay.
The clock atoms permitted by DF-6 are evaluated at every such letter.
Choosing when to observe retains every TS-5 positive-delay choice on the
admitted digital grid; it does not force a letter after each delay unit.
After entering idle mode no ordinary discrete operation may resume.

An admitted infinite digital behavior either has infinitely many ordinary
discrete observation letters and time diverges, or has finitely many such
letters and then follows the irreversible idle-tail construction. A
running delay-only suffix is not a substitute for the TS-5 extension.
A digital idle-tail run is admitted exactly when it emits infinitely many
such observation stutters; between them it takes a positive number of delay
units. This represents TS-5's time-divergent repeated positive-delay stutters,
not an additional authored fairness constraint. A delay-only infinite path
with no more observation letters is not that TS-5 behavior and cannot be
used as its witness. Conversely, arbitrarily long **finite** waits before
the next stutter remain admitted; no observation period or progress bound
is imposed. The core checks this existing behavior admissibility, including
when consuming a lasso/policy/component, rather than dropping the tail or
silently making infinite skipped observations a counterexample. At a truly
quiescent terminal state, continuation uses these tails; there is no
bounded-profile last-discrete-step closure or atomic false extension.
Until/eventually control reads the tail stutters with the same inclusive-D
rules as a discrete letter. Delay crossing still rejects before a later
stutter, so a tail observation exactly at D can accept while one after D
cannot. Time unit, capped clocks, deadline and idle phase remain in every
key, including a decided control state.

For timed Reach/expected elapsed, target predicates are read at the initial
observation and at resolved discrete/IdleObserve **observation edges**, not
at a pure delay state's updated clocks. An analysis target sink may be used
internally for a successful observation edge, without adding a monitor or
accumulator default to the public key. Its predecessor edge carries the
actual elapsed reward and observation; the checker rederives it. This avoids
mistaking x>=1 at an unobserved idle-delayed node for reaching the target.
FR-198/ADR-028's admissibility-aware optimization and FR-202's exact induced
chain evidence use that same relation.

At model position zero, a measure is inactive unless A holds; when A
holds it is active, or complete with magnitude zero when B also holds.
Thereafter an active measure adds the reward of the model step entering
the new position, then completes at the first B. A step before A
activates contributes nothing. The first activation is retained, not
restarted by a later A. At the horizon, an active measure without B is
censored and an inactive measure is unactivated. Censored means greater
than every finite threshold, not an arbitrarily large finite reward.
These rules are QSpec FR-407's measure semantics, not a new activation
convention. At a terminal model state the bounded measure continues on
the specified zero-reward stutter steps through its horizon; a bounded
formula instead uses FR-126's profile closure.

For a weighted fraction, each included position's weight is the reward
of the step leaving it (FR-407). Its predicate is evaluated at that
source position. The contribution is added when that actual model step
is resolved, including the step leaving position h. The remaining count
decrements once per included contribution, not once per intermediate
scheduler node. At a terminal position that departing stutter has zero
reward. An unweighted fraction contributes one per included position,
including h; its numerator increments precisely when P holds. At
completion, denominator zero remains zero and the owning fraction
semantics decides its disposition; the key SHALL NOT fabricate denominator
one or hide an undefined quantity.

`accumulator` is null for an event with no measure and for the three
exact-only forms. Otherwise it is exactly one of these alternatives:

| Measure | Canonical accumulator |
| --- | --- |
| steps | `["steps",activated,amount]` |
| accumulated reward | `["reward",activated,reward,unit,amount]` |
| unweighted fraction | `["count-fraction",true_count,position_count]` |
| weighted fraction | `["weighted-fraction",weight,unit,true_sum,total_sum]` |

`activated` is an actual Boolean. False entails exact amount zero and
inactive/unactivated control; true entails active/complete/censored
control. It distinguishes a nonactivated path from an activated path of
zero reward. Fractions begin at position zero and have no conditional
activation, so their alternatives do not invent an activation flag.

Counts are canonical nonnegative arbitrary-precision decimal strings.
A finite amount or sum is the reduced exact nonnegative rational array
`[numerator,denominator]`, denominator positive, gcd one, zero exactly
`["0","1"]`. Steps use a nonnegative integer string instead of a
rational. Canonical decimal strings admit no leading zero/sign; no
floating-point number, rounding mode or host integer limit enters them.
An accumulated decimal reward is converted to its exact rational value
for arithmetic, while a random argument's typed decimal retains its
declared coefficient/scale as FR-181 requires. They are different roles.

`reward` or `weight` is `[model_node,identifier]`: the existing FR-185/
FR-187 model-scoped reward Identifier, paired with its checked declaring
model's FR-322 NodeId object. This does not invent a reward node id or
qualified-name alias. Each operation contributes that named reward, or
zero when it declares none. The checker resolves the exact identifier
through the checked model before accumulation. `unit` is null for a
plain numeric reward with no unit, otherwise the admitted UnitId represented as
`[domain,hex]`: domain `quire.checked-semantic-node/v1` for a declared
unit or `quire.value.compound-unit/v1` for a compound unit, and its exact
32-byte identity as 64 lowercase hexadecimal digits (ADR-013 T-6,
QSpec FR-142). Neither a display `ms` nor another equal-dimension unit
substitutes for the selected unit. Amounts use the checked claim's
normalized comparison unit for accumulate, and the checked weight's
unit for weighted fraction; conversion is the existing exact unit graph.
No new unit or digest domain is minted by this representation.
An explicitly dimensionless compound-unit quantity still retains its
actual compound UnitId; it is not a plain numeric reward with null unit.

For steps/reward used only by comparison or quantile, all active finite
amounts greater than c SHALL normalize to the **single** amount
`["above-bound"]`. Exact equality to c remains exact, preserving `<`
versus `<=`. Thus any above-bound semantic representative is encoded
identically, without selecting c+1 in a possibly dense rational/unit
domain. This supersedes ADR-028 PR-4's freedom to retain any arbitrary
above-bound magnitude. The AC-2 representative 6 ms at c=5 ms denotes
this tag; it does not require retaining the numeral six. A negative c
still uses the exact checked comparison; when activation occurs, zero
may already normalize above that c. Unactivated zero is retained as zero
because the conditional measure has not begun. A censored measure uses
amount `["censored"]`, distinct from finite overflow, and its monitor
phase must be censored. A complete finite-overflow measure retains
complete control plus `["above-bound"]`.

Both fraction sums SHALL remain exact, without saturation or ratio
reduction of the pair. The terminal ratio may reduce, but the retained
sums do not. Paths with sums (1,2) and (2,4) have equal current ratio,
yet a later true contribution of weight one produces 2/3 versus 3/5.
Coalescing these pairs before completion is unsound. This representation
also retains both sums at completion; no extra decided-state quotient
is introduced. Finite horizon and finite reward supports yield finitely
many possible exact sums, so truncation is unnecessary for finiteness.
Unweighted counts satisfy `0<=true_count<=position_count<=h+1`; weighted
sums satisfy `0<=true_sum<=total_sum`. Imported violations refuse.

`pending` is null at an ordinary model-position state, otherwise exactly
`["drawn",schedule,random]` for a discrete draw at the current model state.
A timed every-scheduler residual choice uses this same ordinary drawn
alternative at the **already delayed** digital source state. Under every
scheduler, delay distributions are unsupported; the admitted digital route
has separate unit-delay moves then enabled discrete draws. Under a workload,
a given-delay race is one edge with its exact probability, delay and discrete
endpoint; a draw with multiple post-states stops NotMarkov under the existing
admission. It is not a positive pending race. There is no drawn-timed tag
or retained unapplied winning delay alternative. Supplying one refuses.

`schedule` is
`[operation,receiver,arguments]`: operation is its existing WireNodeId
in FR-322's exact NodeId object shape; receiver is the FR-181 reference
triple object for its actual ObjectKey; arguments are its non-random
typed values in parameter declaration order. FR-187's ScheduledIdentity
requires an object receiver; no null/default receiver is added here.
`random` is the vector of typed values in random parameter declaration
order, including a genuinely drawn empty vector. Neither vector is a
set, sorted by value or combined with the other. The schedule omits
random arguments; the pending pair retains them separately.

All argument/draw leaves SHALL use QSpec FR-181's complete typed
canonical forms: integer strings; Boolean truth; float32/64 exact bits;
reduced rational; retained decimal coefficient/scale; exact text;
option with true None; set/bag canonical element order with bag
multiplicity; sequence/ordered-set own order; declared record fields
and tuple positions; enum/union declared qualified identity and member;
and reference universe/type/object-identity bytes. Quantities retain
the declared unit identity in that form. Existing field absence differs
from a typed null value. The leaf type is quire-exact Value with QSL's
checked type/identity resolution; FCD's retained SemanticValue JSON
object arrival order, duplicate members and number lexemes are not
canonical Value identity. No new leaf algebra or wire transition alias
is introduced.

An intermediate SHALL retain the source model, monitor and accumulator
unchanged after the draw and before scheduler selection. Its schedule
must be enabled there, its argument and random vector arities/types must
match the checked operation, and that draw must have multiple actual
post-states under SCH-2. The subsequent selected post-state advances
model-position control and accumulated reward exactly once. In a timed
ordinary drawn state the retained source clocks/deadline already include
preceding delay moves; resolution applies only the actual discrete data/reset
and predicate update, without repeating a delay or clock increment. No idle
tail has an ordinary drawn action. The checker
re-enumerates this relation; a provider cannot create a pending key
merely by supplying a well-formed tuple. Distinct receiver, operation,
non-random argument, random position/value or actual None/pending pair
changes complete identity, even at one model state.

Admission SHALL reject unknown/extra/missing members, non-JCS octets,
noncanonical numbers/rationals, incorrect model/unit/node domains,
monitor indices, wrong monitor/accumulator alternatives for the checked
claim, inconsistent activation/phase/counts, arbitrary saturation
magnitudes, reordered vectors, impossible pending states and foreign
formula contexts before they enter certificate membership. Complete
keys rederive from the checker-owned successor and terminal relation;
syntactic admission alone supplies no model state or authority. Budgets
stop explicitly under FR-203/FR-259 and produce no truncated key,
default monitor, default accumulator or fabricated pending data.

### Worked canonical vectors

Hold the actual rederived model form M, checked reward/model/unit identities
and FR-338 formula context fixed for each pair. `P(M,N,A,D)` is exactly
the JCS UTF-8 array defined above; this parameterized notation specifies
full key octets without guessing a semantic model digest. Component body
strings below are exact ASCII octets; embedding them uses these same
JSON values, not an additional JSON/base64 string layer.

| Case | Canonical body / outcome |
| --- | --- |
| unbounded Reach, ordinary state | monitor `null`, accumulator `null`, pending `null`; a supplied zero steps accumulator refuses. |
| steps before activation versus active zero | `["steps",false,"0"]` differs from `["steps",true,"0"]`; matching monitor phases are inactive versus active. |
| threshold 5, active amounts 5, 6, 7 | `["steps",true,"5"]` differs from `["steps",true,["above-bound"]]`; 6 and 7 produce that same overflow body. |
| horizon expires active without B | `["steps",true,["censored"]]` with `["measure","0","censored"]`; this differs from finite overflow at a completed B. |
| exact nonnegative arithmetic | 2/4 is emitted `["1","2"]`; an imported `["2","4"]`, `["0","2"]`, `["1","0"]`, negative numerator or `"01"` refuses. |
| weighted pairs (1,2) and (2,4) | Sums encode `["1","1"],["2","1"]` versus `["2","1"],["4","1"]`; full keys differ and a true weight-one step yields exact ratios 2/3 versus 3/5. |
| fraction horizon 1 | unweighted P true then false completes at `["count-fraction","1","2"]`; weighted positions with weights 2 then 1 complete at exact sums `["2","1"]` and `["3","1"]`. |
| pending draw with no random parameters | `["drawn",schedule,[]]` differs from null when that real residual nondeterministic draw is admitted. |
| two random Booleans | `[{"type":"boolean","value":true},{"type":"boolean","value":false}]` differs from the reversed vector; type and declared parameter positions are retained. |
| repeated model key | Changing only formula remaining horizon, activation, exact sum, phase or pending receiver changes P's octets; reversing graph discovery order changes none. |
| typed draw identity | Positive and negative IEEE zero bits differ; two NaN payloads differ; option None differs from a present value; a reference with another universe differs even with the same object identity bytes. |

Timed vectors use ADR-028 §15.5's actual checked Retx/Deadline context.
Let S be its complete non-clock state with delivered false, U its actual
ms UnitId, and I the resolved Msg::x clock identity for object m; these
three existing semantic leaves are rederived, never invented digests.
The **entire digital model body**, in its specified array order, is
`["digital",S,U,"1",[[I,"0","3"]],["deadline","4","0"],"running"]`
initially and
`["digital",S,U,"1",[[I,"1","3"]],["deadline","4","1"],"running"]`
after one unit delay. The independent expected numeric/string octets are
`"1"`, clock cap `"3"` and deadline D `"4"`; S,U,I must be substituted as
parsed canonical values, with no additional string/base64 layer. Cap 3 is
TA-3's largest comparison 2 plus one; undelivered reachable valuations
remain <=2 by the invariant. Arbitrary cap 2 is refused even though that
projection covers those reachable valuations.

At elapsed 2 after a losing send, the body is
`["digital",S,U,"1",[[I,"0","3"]],["deadline","4","2"],"running"]`
and the monitor remains `["deadline","eventually","pending"]`.
Two keys differing only in x=0 versus x=1, or t=2 versus t=3, differ;
reversing object-clock discovery order normalizes to identical bytes.
A successful send at t=4 has monitor `["deadline","eventually","accept"]`;
a losing send at t=4 stays pending while same-time admissible moves are
considered, then closes reject when no such success is possible. A unit
delay from t=4 yields t=5 and `["deadline","eventually","reject"]`, even
if a subsequent send succeeds. Two discrete steps at t=2 leave the deadline
clock 2; two unit delays with no discrete step advance it to 4 without
creating predicate letters. An until deadline with A false/B false rejects
at its discrete position even before D; B true at D accepts regardless of A.
A real every-scheduler residual discrete choice after two delay units
retains `["drawn",schedule,random]` at that already-delayed running model
state. Its resolution does not advance the deadline again. A workload race
with multiple post-states refuses NotMarkov; under every scheduler a declared
delay distribution refuses DelayDistribution. Neither refusal is a pending
positive, and `["drawn-timed",schedule,random,"2"]` is an unknown-tag negative.

Idle-tail adverse vector: a checked quiescent object has x=0, no operations,
no time invariant/urgency and claim `eventually[0,1] holds(x>=1)` under the
timed profile. Its x cap is 2 and D is 1. Write J for its actual clock identity
and T/V for its complete non-clock state/time unit. Initially the model body
is `["digital",T,V,"1",[[J,"0","2"]],["deadline","1","0"],"running"]`.
One positive tail delay gives
`["digital",T,V,"1",[[J,"1","2"]],["deadline","1","1"],"idle-delayed"]`
with monitor still pending. The terminal observation stutter at time1 emits
x>=1 true, gives tail idle-ready at the same x/deadline values and accepts.
Closing at the initial discrete position would incorrectly reject this
admitted behavior. A next observation delayed until time2 instead rejects
on crossing D before its true stutter. Both choices are admitted TS-5 tails;
this vector states their different trace results, not a uniform probability
or all-behaviors proof. A second stutter with no intervening positive delay,
a reset on the stutter, resumed operations after entering idle mode, omitted
idle control, and treating idle-delayed as an absorbing no-letter terminal
all refuse. An infinite tail delay cycle with no more observation stutters
fails TS-5 admissibility; arbitrarily long finite waits are preserved.
Tail action/policy oracles: at x=1/t=D=1, the same idle-delayed key admits
`["idle-observe"]` (zero elapsed reward, accepts x>=1) or `["idle-delay"]`
(elapsed reward1, crosses D and rejects). Replacing Observe by delay is a
semantic change, not an alias. At a quiescent running key `["delay"]` and
`["enter-idle-delay"]` have the same one-unit clock change but different
irreversible control; their action bodies and resulting keys differ.
Observe from idle-ready, a scheduled-operation alias for Observe, and a
missing/unknown semantic choice refuse. Replay compares the complete target
key, actual phase/clock/deadline, letter/no-letter and elapsed reward once.

Unbounded expected-elapsed oracle uses the same quiescent x>=1 target with
no deadline, actual None monitor/accumulator and cap2. Every finite first
observation wait N>=1 is admitted and has elapsed N. The minimum is1 and
the supremum is +infinity, although no deterministic memoryless policy may
witness an arbitrary finite wait at the capped same key. For threshold3,
choose EnterIdleDelay initially, IdleDelay at idle-delayed x1, and at capped
idle-delayed x2 choose IdleDelay with exact probability3/4 or IdleObserve
with1/4. Expected additional delay is3, so total expectation is5>3 and the
first observation occurs with probability1. After every observation, idle-ready chooses IdleDelay; the resulting
idle-delayed x2 uses that same 3/4 Delay versus 1/4 Observe distribution.
No history switch at the same key is required after the target. This is an almost-surely admitted
randomized memoryless witness with no authored fairness set. Its finite
support chain/certificate is checked in exact rationals; delay forever is
inadmissible and is not offered as an attaining infinity witness. Values
before target are5 initially,4 at idle-delayed x1,3 at capped idle-delayed x2,
and0 as the terminal contribution of a target observation edge; that
analysis sink is not a public ProductKey/certificate value row. The last
equality is3=3/4*(1+3)+1/4*0.
A certificate claiming maximum3 from deterministic policies alone fails.

Observation-edge alias adverse vector: a closed-clock object has x initially0,
f initiallyfalse and two one-shot operations, both requiring not f and setting
f true without reset: early with x<=1, late with x>=2. There are no invariants,
urgency or delay distributions; after f true neither operation is enabled,
so no ordinary zero-delay cycle exists. For Reach/expected elapsed until
x>=2, cap is3 and monitor/accumulator/deadline are actual None. Early at x1
then UnitDelay to x2 reaches the exact running key (f=true,x2) with the
quantity still unhit. Late at x2 reaches that **same** key through a goal
observation edge. The late edge has terminal contribution0 for elapsed;
the earlier pure-delay arrival has residual minimum1 (enter-idle positive
delay, then Observe). A certificate fixing the shared public key to0 merely
because its clock atom is true fails. No goal bit/local visit counter is
added to ProductKey: edge-goal evaluation and distinct objective termination
supply the checked meaning.

Adverse inputs include missing/reordered/duplicate clock identities, reset
of the deadline, wrong scale/cap, timed monitor local index, absent deadline
for Deadline, an event-position formula wrapper for timed Deadline and
acceptance justified only by a post-D predicate. Terminal closure and a
resource stop must preserve the same complete digital form.

These vectors prescribe prospective qualification, not implementation PASS.
All four components must be round-tripped by each consumer, including
rejection loci, with no model-only shortcut.

## Behavior

- Under a workload, the product SHALL be a DTMC whose edges are FR-187's
  `weighted_steps`; a `NotMarkov` from FR-187 SHALL stop the build with
  `Unsupported(NotMarkov{…})`.
- Under every scheduler, the product SHALL be an MDP whose actions at a
  state are FR-187's `actions`. A draw with one post-state SHALL be an edge
  with the draw's probability, summed over draws that reach the same
  post-state. A draw with several post-states SHALL lead, with its
  probability, to an intermediate state keyed by (state, action, drawn
  vector), whose actions are its post-states, each with probability 1
  (SCH-2). A draw with no post-state SHALL stop the build with
  `Unsupported(NotMarkov{…})`.
- For a bounded event, the product SHALL pair each model state with the
  state of FR-126's deterministic monitor for the formula, read at model
  states only, never at intermediate states, with FR-126's closure at
  terminal model states. Timed deadlines SHALL instead use the closed
  digital model and deadline control/advance/closure defined above, through
  FR-204. Accepting and rejecting monitor states SHALL be
  absorbing decided states.
- For a bounded measure, the product key SHALL retain the complete
  canonical monitor and accumulator above, with its actual activation,
  count/reward, unique above-bound/censored amount and horizon/control;
  a weighted fraction SHALL retain both exact sums through completion.
- For a `Reach`, `ExpectedReward` or `LongRunFraction` form, EN-5 SHALL use
  the MDP or DTMC itself as the product, with the predicates as state labels and
  rewards on edges.
- If an atom, a state predicate, a measure or a reward the claim reads
  evaluates `Undefined` at a product state the build creates, then
  EN-5 SHALL stop the build at the first such state in canonical breadth-first
  order and return a refutation whose evidence is an `Undefined` path to
  it, with `UndefinedEvaluation{where, cause}` (ADR-028 XV-8, ADR-018
  UE-1).
- The initial product states SHALL be each subject initial state, per `over`
  binding, paired with the monitor's successor on position 0.
- Exploration SHALL use FR-101's canonical breadth-first order, so the
  product, its state numbering and its edge order are functions of the
  subject, the claim and the limits. Reaching `max_states`,
  `max_transitions` or `max_automaton_states` SHALL stop the build and name
  the limit (FR-203).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-196-AC-1 | §15.1's product (ADR-024 §7.1 `NoFault`) is a DTMC whose undecided states hold the healthy model state at one position each, at most 1,001 of them, plus one accepting and one rejecting decided state. Every undecided state's outgoing probabilities sum to 1, and every path reaches a decided state within 1,001 positions. | Test (TC-631) |
| FR-196-AC-2 | §15.2's product for `P95` at `5 ms` carries the `duration` accumulator, which takes the values `0`, `1 ms`, `2 ms`, `3 ms`, `4 ms` and the saturated value `6 ms`; the activation flag is set from position 1. The edge for `attempt` with `d = 3 ms`, `outcome = Ok` carries `343/5000`. | Test (TC-631) |
| FR-196-AC-3 | §15.4's `Deliver` over every scheduler is an MDP with two actions at each live state, whose draws carry `9/10` and `1/10` (`send_a`) and `4/5` and `1/5` (`send_b`); under `Even` it is a DTMC with FR-187-AC-2's probabilities. | Test (TC-631) |
| FR-196-AC-4 | FR-187-AC-3's `Health` variant with two post-states per draw builds, over every scheduler, an intermediate state per draw with two probability-1 actions, and the monitor does not advance on it; under a workload the build stops with `Unsupported(NotMarkov)`; the variant with no post-state stops with `Unsupported(NotMarkov)` under both. Two builds of one request are equal. | Test (TC-631) |
| FR-196-AC-5 | Over a `Coin` model (object `c` with `v: Int[0, 1]`, initially 0; operation `flip(random b: Int[0, 1] ~ {0: 1, 1: 1})` with postcondition `self.v = b`; workload `Even` with weight `flip` 1) and the claim `probability >= 1/2 [ always[0,3] holds(1 / (1 - c.v) = 1) ]` under `Even`, evidence `exact`, the build stops at the product state with `v = 1` at position 1 and returns a refutation whose evidence is the `Undefined` path `flip` with `b = 1`, probability `1/2`, with `UndefinedEvaluation{where: 1, cause: division-by-zero}`. | Test (TC-641) |
| FR-196-AC-6 | Every worked vector produces the specified canonical component and full P octets; changing any present component changes identity where specified, and reversing discovery order changes none. Certificate values, ranks, policy, bias and rejection retain the same complete representation. | Test |
| FR-196-AC-7 | Steps/reward above-bound values normalize uniquely while equality, unactivation, active zero, completion and censoring remain distinct; both exact weighted sums remain distinct for pairs (1,2)/(2,4) and their subsequent ratios are 2/3 and 3/5. | Test |
| FR-196-AC-8 | Pending identity retains the actual operation node, receiver, non-random arguments and drawn vector in declared order; the empty drawn vector differs from None, and no monitor/accumulator advance occurs at the intermediate. | Test |
| FR-196-AC-9 | Every listed admission negative refuses before certificate membership; exact-only forms retain actual None, timed model keys retain all digital clocks and deadline facts, and a budget stop produces no default/truncated product key. | Test |

## Dependencies

- ADR-028 SP-2, PR-1 to PR-6, SCH-2; ADR-018 SM-3, SM-6, EN-1.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (product, monitors, edge retention),
  [FR-187](FR-187-give-model-transitions-step-probabilities-and-rewards.md),
  [FR-195](FR-195-check-exact-only-forms-and-route-exact-evidence.md).

## References

- QSpec half, which owns the probabilistic product's semantics: QSpec
  FR-406 and FR-411 (Linear STD-137).
- Owning ticket: Linear QSL-371.

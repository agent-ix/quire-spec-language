---
id: ADR-026
title: "Dense time: clocks, timed behaviours and the model-time clock binding"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-160
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-193
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-250
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-252
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
---
# ADR-026: Dense time: clocks, timed behaviours and the model-time clock binding

## Status

Proposed, 2026-10-01. Design draft. §1 records the owner's two rulings: QSL
specifies dense time, for robots and embedded real-time systems, and
model-claim intervals count time units through a clock binding. This record
designs the language half: clocks, constraints, timed semantics, time
divergence, the clock binding and the seam into ADR-018. The engine, hybrid
dynamics and the embedded real-time property set are open (§11). It adds no
requirement. The owning ticket and related work are listed under References.

"QSpec FR-nnn" names a quire-specification requirement; a bare FR id is a QSL
requirement. Item ids `RU-`, `CK-`, `TS-`, `TD-`, `CB-`, `DF-`, `TV-`, `CT-`,
`EG-`, `DC-`, `OV-` and `OQ-` are local to this record. Other artifacts cite
them as `ADR-026 TS-3`. ADR-018 items are cited as `ADR-018 SM-3`.

## Context

**Time in QSL today.** Every clock QSL admits is discrete and exact. QSpec
FR-090 fixes four temporal profiles: event-position false-extension and
infinite-trace count semantic-event positions and claim no elapsed time;
fixed-sample counts samples of an exact rational period; timestamped-event
finite-window counts integer ticks of a declared unit. QSpec FR-252 keys the
clock binding. QSpec FR-094 settles inclusive deadlines from a watermark, and
QSpec FR-160 carries clock uncertainty as exact closed intervals.

**Model subjects.** ADR-018 SM-3 reads a model behaviour as a sequence of
steps of FR-120's successor relation, one position per step. ADR-018 §1
admits event-position false-extension and infinite-trace over a model
subject, and an interval there counts steps. A deadline such as "within
5 ms" has no reading over a model.

**Dense time.** The timed-automata model has clocks valued in the
non-negative reals that advance together at rate 1, guards and location
invariants that compare clocks with constants, resets, and delays of any real
length between discrete steps (Alur and Dill). Reachability is decidable and
PSPACE-complete by the region construction; tools check it with zones held
as difference-bound matrices and extrapolated by the maximal constants
(Bengtsson and Yi; UPPAAL). A run is non-Zeno when its time diverges.
Integer-time verification is exact for automata whose constraints are all
closed (Henzinger, Manna and Pnueli), and strict constraints lose runs under
integer time (§9 example).

**Hybrid models.** QSpec FR-193 analyses continuous and hybrid models under a
selected sound solver contract and names no solver. Clocks are the hybrid
variables whose rate is 1 everywhere; other rates are hybrid dynamics (§11).

## Decision

### 1. Rulings

| ID | Ruling | Where it lands |
| --- | --- | --- |
| RU-1 | **Dense time is specified.** QSL supports robots and embedded real-time systems, so a model may declare real-valued clocks with strict and non-strict constraints, under timed-automata semantics | §2 to §8 |
| RU-2 | **Model-claim intervals count time units through a clock binding.** A deadline like "within 5 ms" is an interval of a claim bound to the model's time | §5; ADR-018 §1 amended; overlap items OV-1 and OV-2 |

### 2. Clocks and constraints

| ID | Rule |
| --- | --- |
| CK-1 | **Time declaration.** A state model declares its time source with one `time` member: `time dense unit u` for real-valued clocks, or `time tick O period p unit u` for digital time counted by occurrences of operation `O` (CB-2). `u` is a unit of QSpec FR-142's time dimension and `p` a positive exact rational quantity of it. A model with a `time` member is a **timed model**; its subject is a **timed subject** (TS-1). Spelling is illustrative; QSpec's shared grammar owns it (OV-7). |
| CK-2 | **Clock fields.** Under `time dense`, an object type may declare fields of type `Clock`. Each object of a universe has its own clock per clock field, so the number of clocks is the universe's objects times the clock fields, finite for a finite universe. A clock's value is a non-negative real in unit `u`; every clock is 0 in every initial state, and an FR-106 snapshot carries no clock value. A model's clocks are its only real-valued state. |
| CK-3 | **Reset.** An operation's postcondition may reset a clock of its frame to an exact non-negative rational constant: `self.x = 0` or `self.x = c`. A clock in the frame that is not reset keeps its value. Reset is the one write to a clock. |
| CK-4 | **Clock constraints.** An atomic clock constraint is `x ~ c` with `~` one of `<`, `<=`, `=`, `>=`, `>` and `c` an exact non-negative rational quantity of the time dimension, converted exactly to `u` (QSpec FR-142, FR-205). Strict and non-strict comparisons are both admitted. A clock is read only inside an atomic clock constraint: S3 refuses a clock read anywhere else, such as copying a clock into a data field or comparing two clocks, so clocks never reach the value kernel. |
| CK-5 | **Guards.** An operation's precondition is a Boolean combination of data predicates and atomic clock constraints. Its clock part is the guard. A disjunction in a guard is a union of convex constraints, which every engine handles by splitting. |
| CK-6 | **Time invariants.** A state model declares time invariants: `time invariant when P { x <= c }` or `{ x < c }`, where `P` is a data predicate over the state and the body is a conjunction of upper bounds on clocks. At a state where `P` holds, time may pass only while the body holds. A time invariant is the location invariant of timed automata; it is distinct from an `invariant` clause, which is a claim. |
| CK-7 | **Urgency.** `urgent when P`, with `P` a data predicate, forbids any positive delay at a state where `P` holds. `urgent O` forbids any positive delay while operation `O` is enabled; S3 admits it only for an operation whose guard reads no clock, so the set of admissible delays stays an interval (as UPPAAL requires of urgent channels). |
| CK-8 | **Atomic steps.** An operation is one atomic step (FR-120), so a multi-step atomic sequence is one operation. Timed automata use committed locations for that purpose; QSL needs no separate committed form. |
| CK-9 | **No caps.** The language fixes no bound on clock values, constants, clock count or delay length. Every limit on the analysis is a caller-set B-5 budget (EG-3). |

### 3. Timed semantics

| ID | Rule |
| --- | --- |
| TS-1 | **Timed subject.** A timed subject is an ADR-018 SM-2 model subject whose model has a `time` member. Its states are **timed states** `(s, v)`: `s` the discrete state (every non-clock field), `v` the clock valuation. |
| TS-2 | **Moves.** A *delay* by real `d >= 0` takes `(s, v)` to `(s, v + d)`. It is admissible when `d = 0`, or when `s` is not urgent (CK-7) and every time invariant whose `P` holds at `s` holds at `v + d`; time invariants are upper bounds, so they then hold at every point in between. A *discrete step* by transition identity `t` (ADR-018 FA-1) takes `(s, v)` to `(s', v')` when `t`'s data precondition holds at `s`, its guard holds at `v`, `s'` is a post-state of FR-120's successor rule, `v'` is `v` with `t`'s resets applied, and every time invariant whose `P` holds at `s'` holds at `v'`. |
| TS-3 | **Timed behaviours and positions.** A timed behaviour alternates delays and discrete steps from an initial state at time 0: `(s0, 0) -d1-> -t1-> (s1, v1) -d2-> -t2-> …`, one delay before each discrete step, possibly 0. Positions are ADR-018 SM-3's: position 0 is the initial state, position `i > 0` is the post-state of the `i`-th discrete step, with that step's operation anchor. Each position also carries its **time stamp** `τ_i = d1 + … + di`, non-decreasing, and its clock valuation. Delays are not positions. Steps with equal time stamps keep their step order, which is the admitted order QSpec FR-090-AC-4 requires. |
| TS-4 | **Time divergence.** A timed behaviour is admitted only when its time diverges: `τ_i` grows beyond every bound. Zeno behaviours, infinitely many steps in bounded time, are outside the admitted set, as in Alur and Dill's semantics. ADR-018 SM-5 reads "every admitted behaviour" as every time-divergent behaviour under every profile over a timed subject. |
| TS-5 | **Quiescent states and the idle tail.** A discrete state is **quiescent** when no time invariant applies at it and it is not urgent, so it admits unbounded delay. A behaviour may end its discrete steps at a quiescent state and delay forever. Under infinite-trace and the timed profile it is extended by ADR-018 SM-4's terminal stutter step, repeated forever, each after a positive delay of any length with time diverging; every such choice is an admitted behaviour, so a verdict holds for every observation instant on the tail. The stutter step is outside every fairness constraint (ADR-018 SM-4). Under a bounded profile an idle tail closes the execution at its last discrete step, as SM-4 closes a terminal state. Once a behaviour takes the stutter step it takes only stutter steps, so the stutter never inserts positions between discrete steps. |
| TS-6 | **Digital time.** Under `time tick O period p`, a timed behaviour is a behaviour of the untimed subject, each delay is 0, and the time stamp of position `i` is `p` times the number of `O` steps among steps 1 to `i`. The terminal stutter step counts as an `O` step, so a behaviour that ends at a terminal state diverges. TS-4 reads divergence as infinitely many `O` steps. |
| TS-7 | **Untimed reading.** An ADR-018 claim under event-position false-extension or infinite-trace over a timed subject reads TS-3's positions with the clock valuation as part of each position's state, over the time-divergent behaviours (TS-4) with idle tails (TS-5). ADR-018 TP-1 to TP-4, fairness, deadlocks and counterexamples apply with the changes of §4 and §7. |

### 4. Time divergence, time-locks, deadlocks and fairness

| ID | Rule |
| --- | --- |
| TD-1 | **Time-lock.** A time-lock is a reachable timed state from which no time-divergent behaviour exists. It is *local* when no positive delay is admissible and no discrete step is enabled at it, so time stops there; otherwise every continuation is Zeno. A time-lock silently removes behaviours from TS-4's admitted set, so every claim over a time-locked state holds vacuously. |
| TD-2 | **Time-lock-freedom item.** For each distinct timed subject among a request's items, the request writer (ADR-013 O-20) adds one derived **time-lock-freedom** item, alongside ADR-018 DL-3's deadlock-freedom item. It has no opt-out: no physical system stops time. Its obligation identity is the subject and the fixed item kind `time-lock-freedom` (ADR-013 O-09). It settles with ADR-018's verdict kinds. |
| TD-3 | **Time-lock counterexample.** A time-lock refutation is a finite prefix to the time-locked state, ending with the delay that reaches it, with `TemporalCounterexample.kind` `TimeLock` (ADR-018 DL-4 adds `kind`). Replay (CT-3) re-executes the prefix. For a local time-lock it then checks, at the last timed state, that no positive delay is admissible and no transition identity is enabled; agreement settles `refuted`, disagreement `inconclusive`, `ReplayParity`. For a non-local time-lock the prefix replays and the absence of a divergent continuation is a closed-scope fact of the engine's explored graph, recorded with the prefix (OQ-2). |
| TD-4 | **Deadlocks in a timed subject.** A timed state is terminal when no discrete step is enabled at it or after any admissible delay (ADR-018 SM-4, read over time). A terminal state at a quiescent discrete state reads by TS-5's idle tail; it is a deadlock when it is not intended (ADR-018 DL-1, DL-2), and the deadlock-freedom item reports it. A terminal state where delay is bounded is a local time-lock, reported by TD-2. ADR-018 DL-7's `deadlocked` predicate quantifies over admissible delays. |
| TD-5 | **Fairness over time.** ADR-018 FA-2 reads enabledness at a timed state. A behaviour satisfies weak fairness for a constraint when from no instant onward the constraint stays enabled at every instant, positions and the delays between them, while none of its transitions is taken. On a timed lasso (CT-2) the constraint is taken somewhere in the loop or disabled at some instant of the loop. Weak fairness and time divergence both filter behaviours; they are independent premises, and an accepting cycle refutes only when it is fair and its loop has positive total delay. |
| TD-6 | **Vacuity.** A proof over a timed subject whose initial state has no fair time-divergent behaviour settles `inconclusive`, `NoAdmittedBehaviour`, a new ADR-018 V-6 cause, so a premise that admits nothing never reads as proof. |

### 5. The model-time clock binding

| ID | Rule |
| --- | --- |
| CB-1 | **Bindings over a model subject.** A temporal claim over a model subject names one clock binding. `model-steps` is ADR-018's: the step sequence as the event-position sequence authority, and an interval counts steps. `model-time` binds the claim to the subject's time source (CK-1), and an interval counts time units. A `model-time` claim selects the timed profile (DF-1); a `model-time` binding over a model with no `time` member refuses at S3. |
| CB-2 | **Time sources.** A dense source gives each position the time stamp of TS-3. A digital source gives it the time stamp of TS-6. Both give non-decreasing exact rational time stamps in unit `u`, with the step order as admitted order. The evaluator and every engine read time stamps only, so one profile serves both. |
| CB-3 | **Binding identity.** The binding's QSpec FR-252 key is the timed profile identity, the clock identity (the model's qualified name and its time source kind), the unit, the period for a digital source, and the sequence authority (step order). Changing the source kind, unit or period changes the binding identity (QSpec FR-090-AC-3). |
| CB-4 | **Interval values.** An interval bound in a `model-time` claim is a time quantity, converted exactly to `u` (QSpec FR-142). The interval's QSpec FR-255 key carries the timed profile identity and the binding, so it never equals a step-counting interval. |

### 6. Temporal forms over dense time

| ID | Rule |
| --- | --- |
| DF-1 | **The timed profile.** A new QSpec FR-250 profile, illustratively `quire.temporal.timed/v1` (OV-1). One tick is an exact rational duration of the binding's unit. It reads positions in step order, each with its time stamp, over time-divergent infinite behaviours: no closure, as infinite-trace. It admits a timed subject through `model-time`, and a timed trace: a finite prefix or lasso of positions with exact rational time stamps, which is what replay reads (CT-3). It admits every unbounded operator of infinite-trace and the interval operators of DF-2. |
| DF-2 | **Timed intervals.** Every interval-capable operator (`eventually`, `always`, `until`, `release`, `once`, `historically`, `since`, `triggered`) admits an interval with exact rational time bounds `a <= b`, each end closed or open, written `[a, b]`, `(a, b]`, `[a, b)` or `(a, b)`; an interval with an open end has `a < b`. The upper bound is finite, as ADR-018 IV-1 refuses `[a,*]`. An interval with `a = b`, closed, is **punctual**. |
| DF-3 | **Pointwise meaning.** A formula is read at positions. `eventually_I p` holds at `i` when some `j >= i` has `τ_j − τ_i` in `I` and `p` at `j`; `p until_I q` when some such `j` has `q` and every `k` with `i <= k < j` has `p`; the past operators read `τ_i − τ_j` over `j <= i`; `always` and `release` are the duals. This is the reading of QSpec FR-090's timestamped-event profile with rational stamps and no window closure. Atoms read the discrete state and atomic clock constraints at the position's valuation. |
| DF-4 | **Forms with an every-behaviour verdict.** A model claim under the timed profile gets one of four forms, the timed counterparts of ADR-018 TP-1 to TP-4. **TT-1** timed invariant: `always holds(I)`. **TT-2** bounded window: activation `on origin` and only interval operators; its truth depends on the positions with time stamp at most the formula's time horizon, the sum of its upper bounds, which time divergence makes finitely many on each behaviour. **TT-3** timed safety: a formula in ADR-014 A-4's safety fragment with timed intervals. **TT-4** timed liveness: any other admitted formula, with its fairness. |
| DF-5 | **Decidability boundary.** MTL over dense time is undecidable (Alur and Henzinger); in the pointwise reading over infinite timed words it is undecidable (Ouaknine and Worrell), and over finite timed words decidable with non-primitive-recursive complexity. MITL, MTL with no punctual interval, is decidable and EXPSPACE-complete (Alur, Feder and Henzinger). Bounded MTL, every interval bounded, is decidable and EXPSPACE-complete (Bouyer, Markey, Ouaknine and Worrell). So TT-2 admits punctual intervals, and TT-1, TT-3 and TT-4 admit every formula with no punctual interval. A TT-1, TT-3 or TT-4 claim with a punctual interval settles `unsupported`, `PunctualInterval` (V-8), over a timed subject. Over a single timed trace every admitted formula evaluates (CT-3), punctual intervals included. |
| DF-6 | **Invariants between positions.** A data predicate is constant between positions, so TT-1 over data reads every instant. An atomic clock constraint in a claim is read at positions only; a bound that must hold at every instant is a time invariant (CK-6), which the semantics enforces. |
| DF-7 | **Interval operators and stuttering.** A timed interval measures time stamps, so it is invariant under inserting a step that repeats the discrete state at an equal time stamp. ADR-018 IV-7's position-counting caveat applies to `model-steps` intervals only. |

### 7. Verdicts and counterexamples

| ID | Rule |
| --- | --- |
| TV-1 | **Verdict kinds.** A timed item settles with ADR-018 V-1 to V-8 unchanged. The length of a counterexample is its number of discrete steps, so V-5 keeps its one definition. New causes: `NoAdmittedBehaviour` (V-6, TD-6) and `PunctualInterval` (V-8, DF-5). |
| CT-1 | **Step content.** ADR-018 CX-2's step content gains `delay`, an exact non-negative rational in the binding's unit (QSpec FR-205), the delay before the step. The post-state digest is the canonical identity digest of the discrete state and the exact clock valuation. A time-lock counterexample ends with a final delay (TD-3). |
| CT-2 | **Timed lasso.** A liveness counterexample is a lasso whose loop has positive total delay `D`. At the loop's end the discrete state equals the entry's; each clock the loop resets has the entry's value; each clock the loop never resets is, at entry, above the largest constant it is compared with in the model and the claim, so every constraint on it keeps its truth on every repetition. Repeating the loop with the same delays is then a time-divergent behaviour, and its time stamps grow by `D` per repetition. An idle tail (TS-5) is the lasso whose loop is one stutter step with a positive delay. |
| CT-3 | **Replay through `ModelSystem`.** `replay_model_trace` (ADR-018 CX-3) starts every clock at 0 and, for each step, checks the delay is admissible (TS-2), applies it, checks the transition identity is enabled at the delayed state, selects the successor by the post-state digest, applies the resets and checks the target's time invariants, all in exact rational arithmetic. A violated delay or guard refuses `invalid_runtime_input`/`invalid-value`; a digest with no matching successor refuses `stale_dependency`/`revision-mismatch`, as CX-3 states. It checks CT-2's loop closure and `D > 0`, checks fairness by TD-5, and evaluates the formula with SM-1 over the timed lasso: the evaluator unrolls the loop until the unrolled time span covers the formula's time reach, the timed counterpart of ADR-018 SM-8. Agreement settles `refuted`; disagreement `inconclusive`, `ReplayParity`. |
| CT-4 | **Rational delays exist.** Every non-empty zone or region of an automaton with rational constants contains a point with rational coordinates, so every refutation a symbolic engine finds has a counterexample with exact rational delays. An engine produces them from its symbolic path, as UPPAAL's concrete traces do, and a counterexample replays with no engine present (ADR-018 CX-4). |

### 8. Engine requirements and the seam into ADR-018

| ID | Rule |
| --- | --- |
| EG-1 | **Placement.** A timed engine is a backend placed by ADR-018 §3 negotiation, advertising (`temporal-satisfaction`, `unbounded`) for timed subjects. Which engine fills it is open (§11). |
| EG-2 | **What every timed engine does.** It decides TT-1 to TT-4, the deadlock-freedom item and the time-lock-freedom item over time-divergent behaviours; it gives V-1 to V-8 with ADR-018's meanings; and it returns CT-1 to CT-2 counterexamples that replay by CT-3. |
| EG-3 | **Budgets.** Every limit is a caller-set ADR-014 B-5 budget in the engine's `ModelCheckLimits`: states, transitions, symbolic states (zones, for a zone engine), automaton states, time, memory, cancellation. Reaching one settles V-7. Extrapolation by maximal constants is part of the semantics-preserving abstraction, not a cap. |
| EG-4 | **Digitization as optimisation.** When every clock constraint, time invariant and timed interval of a subject and claim is closed, an engine may explore integer-valued clocks after scaling every constant to an integer by the common denominator (Henzinger, Manna and Pnueli; Ouaknine and Worrell). It gives the same verdict for TT-1 over the discrete state, the deadlock-freedom item and location reachability, and its counterexamples have integer delays, which are valid dense delays. |
| EG-5 | **The seam.** Over ADR-018 this record adds a profile (DF-1), a subject kind (TS-1), positions with time stamps (TS-3), an admitted-behaviour filter (TS-4), the idle tail (TS-5), a derived item (TD-2), two causes (TV-1), a step member and lasso rule (CT-1, CT-2), and a replay arm (CT-3). ADR-018's verdict table, stage S6c, the product with a property automaton, SCC-based acceptance with fairness, and the evaluator as semantics (SM-1, SM-7) are unchanged. |

### 9. What discrete time still covers

| ID | Rule |
| --- | --- |
| DC-1 | **Steps.** `model-steps` intervals count operations, which states step-bounded properties of a protocol. |
| DC-2 | **Observed traces.** A live system's timestamps are integer ticks of a declared unit: the timestamped-event and fixed-sample profiles state its deadlines (QSpec FR-090, FR-094), and QSpec FR-160 states its clock uncertainty as exact intervals. |
| DC-3 | **Digital models.** A model whose time is a periodic tick (a scheduler tick, a sampled controller's period) declares `time tick O period p`, and its `model-time` claims count time units (TS-6, CB-2). |
| DC-4 | **Clock drift.** A local timer whose rate lies in `[1 − ρ, 1 + ρ]` reads `T` at a time in `[T/(1 + ρ), T/(1 − ρ)]`. A dense model states it exactly: a guard `x >= T/(1 + ρ)` on the firing operation and a time invariant `x <= T/(1 − ρ)` while armed, both rational. A clock whose rate itself varies is hybrid dynamics (§11). |

**Example: what integer time loses.** Clock `x` measures time since a
request and clock `y` time since a heartbeat. The heartbeat is admitted while
`x < 1`; a retry fires when `x > 1 and y < 1`. In dense time the heartbeat at
0.5 and the retry at 1.2 (`x = 1.2`, `y = 0.7`) form a run. In integer time
`x < 1` means `x = 0`, so `y = x` at every later state and "no retry" is
proved for the digital model; half ticks still prove it, and only ticks of a
third reach the retry. Dense time is the semantics, so the retry is found, and
EG-4 digitizes only when constraints are closed.

### 10. Worked example: a timeout racing a reply

A client sends a request and times out at `T`; the server replies at least
1 ms after the request. Spelling is illustrative, as in ADR-018 §6.

```text
model Rpc = "example/rpc" …;      // object Call { phase: Phase, inflight: Bool, late: Bool, x: Clock }
time dense unit ms;

operation Call::send
  pre  self.phase = Idle
  post self.phase = Waiting and self.inflight and not self.late and self.x = 0;
operation Call::reply
  pre  self.inflight and self.x >= 1 ms
  post not self.inflight and self.late = (pre(self.phase) = TimedOut) and
       self.phase = (if pre(self.phase) = Waiting then Replied else pre(self.phase));
operation Call::timeout
  pre  self.phase = Waiting and self.x >= T
  post self.phase = TimedOut;
operation Call::reset
  pre  not self.inflight and (self.phase = Replied or self.phase = TimedOut)
  post self.phase = Idle;

time invariant when self.inflight            { self.x <= 3 ms }   // server deadline
time invariant when self.phase = Waiting     { self.x <= T }      // client deadline

temporal NoLateReply using inf over (k: Rpc::Call) clock "model-steps" on origin {
  always holds(not k.late)
}
temporal Settles using timed over (k: Rpc::Call) clock "model-time" on origin {
  always (holds(k.phase = Waiting) implies eventually[0 ms, 3 ms] holds(k.phase != Waiting))
}
```

Subject: universe `{c}`, initial state `Idle` with `inflight` and `late`
false and `x = 0`. `Idle`, `Replied` and `TimedOut` with `inflight` false
are quiescent (TS-5).

**`T = 3 ms`: `NoLateReply` refuted.** At `x = 3` while `Waiting` and in
flight, both time invariants stop time, and `reply` and `timeout` are both
enabled. A counterexample:

| Position | Delay before | Step | Time stamp | State after (`phase`, `inflight`, `late`, `x`) |
| --- | --- | --- | --- | --- |
| 0 | | initialization | 0 | `Idle, false, false, 0` |
| 1 | 0 | `send` | 0 | `Waiting, true, false, 0` |
| 2 | 3 | `timeout` | 3 | `TimedOut, true, false, 3` |
| 3 | 0 | `reply` | 3 | `TimedOut, false, true, 3` |

The item settles `refuted`, V-4, `trace_position` 3. Replay (CT-3) checks
the delay of 3 against `x <= 3 ms` and `x <= T`, the guards `x >= 3` and
`x >= 1`, and the post-state digests, in exact arithmetic.

**`T = 4 ms`: `NoLateReply` proved.** In flight, `x <= 3` stops time at 3
while `reply` is enabled from 1, so `reply` comes at some `x` in `[1, 3]`
while `phase = Waiting`, which sets `Replied`; `timeout` needs
`x >= 4`, which no reachable timed state in flight has. The item settles
`proved`, V-1, over every real reply delay in `[1, 3]`. Every constraint is
closed, so an engine may digitize (EG-4).

**`Settles` under the timed profile.** The form is TT-3, with no punctual
interval. The only position with `phase = Waiting` is `send`'s; the next
position comes at most 3 ms later under either `T`, and its phase is
`Replied` or `TimedOut`. The item settles `proved`, V-1, for `T = 3 ms` and
`T = 4 ms`. With the strict upper bound `eventually[0 ms, 3 ms)` it settles
`refuted` with the prefix `send` at 0 then `reply` after delay 3: the reply
at exactly 3 ms falls outside `[0, 3)`, and the counterexample's delay is the
exact boundary value.

**Time-lock from a strict guard.** Change the server to `reply` with
`x > 3 ms` (strictly after its own deadline) and the timeout to `x > T`,
with `T = 3 ms`. At `Waiting`, `x = 3`, the time invariants admit no
positive delay, and `reply` and `timeout` both need `x > 3`. The
time-lock-freedom item (TD-2) settles `refuted` with the prefix `send` at 0
and a final delay of 3; replay checks that no positive delay is admissible and
no transition identity is enabled there (TD-3). The deadlock-freedom item
settles `proved`: the time-locked state is not quiescent, so TD-4 reports it
as a time-lock and not a deadlock. With non-strict guards (`x >= 3`) the item
settles `proved`.

**Deadlock freedom.** In the unmodified model every quiescent state has
`send` or `reset` enabled, and every non-quiescent state has `reply` or
`timeout` enabled at or before its time bound, so the deadlock-freedom and
time-lock-freedom items settle `proved`, V-1, under both values of `T`.

### 11. Open, pending research

These items are open pending the owning research ticket's findings (listed
under References). This record decides none of them.

| Item | Open question |
| --- | --- |
| Engine | A native zone engine in QSL layer 5 (difference-bound matrices, extrapolation, a non-Zeno check for liveness, as Tripakis, Yovine and Bouajjani give), or an external tool such as UPPAAL behind EG-1's backend interface, or both. EG-1 to EG-4 bind any choice. |
| Hybrid robot dynamics | Variables with rates other than 1, flows and resets for robot plants: whether QSL specifies hybrid automata, which decidable classes, and how bounded hybrid checking settles under ADR-018's verdict kinds and QSpec FR-193's provider contract. |
| Embedded real-time property set | Which properties embedded real-time users need beyond TT-1 to TT-4: for example schedulability, worst-case response time and jitter, and their forms. |

### 12. Interactions with the sibling records

- **ADR-019 (strong fairness).** Strong fairness reads enabledness over time
  as TD-5 reads weak fairness.
- **ADR-020 (refinement).** A timed concrete model refines an untimed abstract
  model through its step map; delays map to no abstract step.
- **ADR-021 (reductions).** Symmetry permutes objects with their clocks. Under
  POR-4 a delay writes every clock, so two transitions are independent only
  when neither reads a clock; reductions that exploit clock independence
  belong to the engine question (§11).
- **ADR-024 (statistical).** Its workload resolves discrete choice, and a
  timed subject's delays carry no distribution, so a probabilistic claim over
  a timed subject settles `unsupported` (OQ-3). Its `duration` reward and
  CT-1's delays are the same time-dimension quantity type.

### 13. Overlap items

| ID | Item | Owner |
| --- | --- | --- |
| OV-1 | **QSpec FR-090.** A fifth profile row for the timed profile (DF-1): one tick an exact rational duration of the binding's unit, positions in step order with time stamps, no closure, time-divergent behaviours, timed intervals with open or closed ends (DF-2); its admission of the unbounded operator grammar (FR-090-AC-7) | QSpec |
| OV-2 | **ADR-018 §1.** The model subject admits the timed profile through `model-time` (CB-1), amended in place with this record, together with SM-3's time stamps and SM-4's idle tail. ADR-018 QS-3 (profiles over a model subject and their clock binding) carries the same change to QSpec | QSL, amended here |
| OV-3 | QSpec FR-252: the time source component and the `model-steps` and `model-time` bindings (CB-3) | QSpec |
| OV-4 | QSpec FR-255: interval end openness and the timed profile identity in the key (CB-4, DF-2) | QSpec |
| OV-5 | QSpec FR-161 and FR-181: the timed subject, delay moves, the time-divergence premise, quiescent states, time-locks and the timed lasso (TS-1 to TS-6, TD-1, CT-2) | QSpec |
| OV-6 | QSpec FR-331 and the counterexample contract: the step `delay`, the final delay, the `TimeLock` kind, the time-lock-freedom item and the two new causes (CT-1, TD-2, TD-3, TV-1) | QSpec |
| OV-7 | The shared grammar: the `time` member, `Clock` fields, resets, clock constraints, time invariants, `urgent`, timed intervals and the `model-time` binding | QSpec |

## Consequences

- QSL states real-time and robot timing behaviour exactly: real-valued
  clocks, strict and non-strict constraints, urgency and time invariants.
- A model claim states a deadline in time units through `model-time`.
- Verdicts quantify over time-divergent behaviours, and the derived
  time-lock-freedom item reports every state where that set is empty.
- Every timed refutation carries exact rational delays and replays through
  `ModelSystem` with no engine present.
- ADR-018's verdict vocabulary, stage, engine interface and replay contract
  carry timed subjects with additive changes only.
- Discrete time remains for step counting, observed traces and digital
  models, and digitization is an engine optimisation for closed constraints.
- The engine, hybrid dynamics and the embedded real-time property set wait
  on research.

## Amendments made with this record

Each amended text carries an "Amended by ADR-026" note.

- ADR-018 §1: the model subject admits the timed profile through a
  `model-time` clock binding. §2 SM-3: positions over a timed subject carry
  time stamps. §2 SM-4: a timed subject reads terminal and quiescent states by
  TS-5 and TD-4.
- `spec/spec.md`: the index row for this record.

## Alternatives Considered

- **Discrete time only, with digitization.** Reversed by the owner (RU-1).
  Strict constraints over continuous time lose runs under integer time (§9).
- **Continuous (signal) semantics for timed formulas.** Not taken. Positions
  are ADR-018 SM-3's and QSpec's timestamped-event profile is pointwise, so
  the pointwise reading keeps one evaluator and one counterexample shape. The
  hybrid research may revisit it for plant signals.
- **Diagonal clock constraints (`x − y ~ c`).** Not taken. Zone extrapolation
  with diagonal constraints needs zone splitting to stay sound (Bouyer), and
  diagonal-free automata are equally expressive (Bérard, Diekert, Gastin and
  Petit).
- **A real-number type in the value kernel.** Not taken. Clocks are read only
  inside clock constraints (CK-4), so no real value reaches the kernel and
  every witness is exact rational.
- **Time divergence as a fairness constraint.** Not taken. Divergence is a
  property of delays and holds for every admitted behaviour (TS-4); fairness
  constraints name operations and are chosen per claim.
- **An opt-out for time-lock freedom.** Not taken. No intended behaviour stops
  time, unlike an intended terminal state (ADR-018 DL-1).
- **Steps as the only interval measure over models.** Replaced by RU-2.

## Open questions for the owner

| ID | Question |
| --- | --- |
| OQ-1 | RU-2 reads "intervals count time units" as the `model-time` binding beside ADR-018's `model-steps`, which sibling records use. Should `model-steps` intervals stay, or should every model-claim interval count time? |
| OQ-2 | A non-local time-lock's absence half is an engine fact over the explored graph (TD-3). Should its witness take the same shape as the AG EF witness of the possible-properties record when that lands? |
| OQ-3 | Probabilistic claims over timed subjects (ADR-024) settle `unsupported` here. Should stochastic delays (probabilistic timed automata) join the research scope? |

## References

- Owning ticket: Linear QSL-373. The owner's rulings of 2026-10-01 are on
  that ticket. Research on the engine, hybrid robot dynamics and the embedded
  real-time property set: Linear RES-54. Built on ADR-018 (Linear QSL-366).
  Sibling drafts read for interactions: ADR-019 (QSL-365), ADR-020 (QSL-367),
  ADR-021 (QSL-368), ADR-024 (QSL-371).
- R. Alur and D. L. Dill, "A theory of timed automata", Theoretical Computer
  Science, 1994.
- J. Bengtsson and W. Yi, "Timed automata: semantics, algorithms and tools",
  Lectures on Concurrency and Petri Nets, LNCS, 2004.
- K. G. Larsen, P. Pettersson and W. Yi, "UPPAAL in a nutshell",
  International Journal on Software Tools for Technology Transfer, 1997.
- T. A. Henzinger, Z. Manna and A. Pnueli, "What good are digital clocks?",
  ICALP, 1992.
- J. Ouaknine and J. Worrell, "Revisiting digitization, robustness, and
  decidability for timed automata", LICS, 2003.
- R. Alur and T. A. Henzinger, "Real-time logics: complexity and
  expressiveness", Information and Computation, 1993.
- R. Alur, T. Feder and T. A. Henzinger, "The benefits of relaxing
  punctuality", Journal of the ACM, 1996.
- J. Ouaknine and J. Worrell, "On the decidability of metric temporal logic",
  LICS, 2005; and "On metric temporal logic and faulty Turing machines",
  FoSSaCS, 2006.
- P. Bouyer, N. Markey, J. Ouaknine and J. Worrell, "The cost of
  punctuality", LICS, 2007.
- P. Bouyer, "Forward analysis of updatable timed automata", Formal Methods
  in System Design, 2004.
- B. Bérard, V. Diekert, P. Gastin and A. Petit, "Characterization of the
  expressive power of silent transitions in timed automata", Fundamenta
  Informaticae, 1998.
- S. Tripakis, S. Yovine and A. Bouajjani, "Checking timed Büchi automata
  emptiness efficiently", Formal Methods in System Design, 2005.
